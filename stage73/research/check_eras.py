"""Compare the plugin's electrical paths to independent 73-coil ngspice decks.

Usage: python3 check_eras.py PROBE OUTPUT_DIRECTORY
"""

import csv
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
import soundfile as sf
from jordan_lfo import oscillator
from scipy.signal import find_peaks, hilbert

HERE = Path(__file__).resolve().parent
probe, output = (Path(arg).resolve() for arg in sys.argv[1:])
output.mkdir(parents=True, exist_ok=True)


def harp(original, key):
    groups = [7] + [6] * 11 if original else [4] + [3] * 23
    coils = []
    for i, size in enumerate(groups):
        lo = "0" if i == 0 else f"g{i}"
        hi = "harp" if i == len(groups) - 1 else f"g{i + 1}"
        for _ in range(size):
            index = len(coils)
            coils.append(f"X{index} {hi} {lo} pickup amplitude={int(index == key)}")
    return "\n".join(coils)


def electrical_deck(era, volume, tone, treble, source, cable_pf=300):
    jordan = era == "sparkletop"
    template = {"early_i": "early", "sparkletop": "jordan"}.get(era, "stage")
    deck = (HERE / f"{template}.cir").read_text().replace("@HARP@", source)
    pot = 100000 if jordan else 10000
    bottom = pot * volume**3
    tleft = 10000 * (1 - (1 - tone) ** 3)
    values = {
        "CABLE_PF": cable_pf,
        "LOWER": bottom,
        "UPPER": pot - bottom,
        "BASS": 50000 * (1 - tone) ** 3,
        "BLEFT": 100000 * (1 - tone),
        "BRIGHT": 100000 * tone,
        "TLEFT": 100000 * (1 - treble) if jordan else tleft,
        "TRIGHT": 100000 * treble if jordan else 10000 - tleft,
    }
    for name, value in values.items():
        deck = deck.replace(f"@{name}@", str(max(0.01, value)))
    return deck


def ac_reference(era, volume, tone, treble, key, wd, cable_pf=300):
    deck = electrical_deck(era, volume, tone, treble, harp(era == "sparkletop", key), cable_pf)
    (wd / "ac.cir").write_text(deck)
    subprocess.run(
        ["ngspice", "-b", "ac.cir"], cwd=wd, stdout=subprocess.DEVNULL, check=True
    )
    return np.loadtxt(wd / "sweep.txt").T


def jordan_overload(wd):
    # Use the original harness's Thévenin impedance. Source amplitudes are
    # open-circuit harp volts peak, independent of the plugin's digital gain.
    factor = 1 / 7 + 11 / 6
    with (output / "jordan-overload.csv").open("w") as f:
        writer = csv.writer(f)
        writer.writerow(
            ("hz", "harp_v_peak", "gain_db", "gain_change_db", "h2_dbc", "h3_dbc")
        )
        for hz in (80, 1000):
            for peak in (0.001, 0.01, 0.03, 0.1, 0.3, 1.0):
                source = f"Vharp src 0 sin(0 {peak} {hz})\nRharp src coil {180 * factor}\nLharp coil harp {0.075 * factor}"
                deck = electrical_deck("sparkletop", 0.8, 0.5, 0.5, source)
                circuit = deck.split(".control")[0]
                (wd / "overload.cir").write_text(
                    circuit
                    + f""".control
set wr_singlescale
tran {1 / (256 * hz)} {0.3 + 8 / hz} 0.3
wrdata transient.txt v(jack)
quit
.endc
.end
"""
                )
                subprocess.run(
                    ["ngspice", "-b", "overload.cir"],
                    cwd=wd,
                    stdout=subprocess.DEVNULL,
                    check=True,
                )
                t, y = np.loadtxt(wd / "transient.txt").T
                phase = np.arange(4 * 256) / 256
                sample = np.interp(0.3 + (4 + phase) / hz, t, y)
                harmonics = 2 * abs(np.fft.rfft(sample)[[4, 8, 12]]) / len(sample)
                gain = 20 * np.log10(harmonics[0] / peak)
                if peak == 0.001:
                    small_signal = gain
                writer.writerow(
                    (
                        hz,
                        peak,
                        gain,
                        gain - small_signal,
                        *20 * np.log10(harmonics[1:] / harmonics[0]),
                    )
                )


rows = []
with tempfile.TemporaryDirectory() as tmp:
    wd = Path(tmp)
    jordan_overload(wd)
    for era in ["early_i", "late_i", "sparkletop"]:
        settings = [(v, t, 0.5) for v in (0.1, 0.5, 1.0) for t in (0.0, 0.5, 1.0)]
        if era == "sparkletop":
            settings += [(1.0, b, t) for b in (0.0, 0.5, 1.0) for t in (0.0, 1.0)]
        loads = (300,) if era == "sparkletop" else (100, 300, 1000)
        for k, (v, t, tr, cable_pf) in enumerate(
            (v, t, tr, cap) for v, t, tr in settings for cap in loads
        ):
            key = (0, 4, 6, 7, 72)[k % 5]
            freq, analog = ac_reference(era, v, t, tr, key, wd, cable_pf)
            for rate in (44100, 48000, 96000):
                subprocess.run(
                    [
                        probe,
                        "panel",
                        wd / "imp.wav",
                        era,
                        str(rate),
                        str(v),
                        str(t),
                        str(tr),
                        str(key),
                        str(cable_pf),
                    ],
                    check=True,
                )
                x, fs = sf.read(wd / "imp.wav")
                bins = np.fft.rfftfreq(len(x), 1 / fs)
                digital = 20 * np.log10(np.maximum(abs(np.fft.rfft(x)), 1e-16))
                error = np.interp(freq, bins, digital) - analog
                i = np.argmax(abs(error))
                rows.append((era, rate, v, t, tr, key, cable_pf, freq[i], error[i]))
with (output / "circuit-errors.csv").open("w") as f:
    w = csv.writer(f)
    w.writerow(
        (
            "era",
            "rate",
            "volume",
            "tone",
            "treble",
            "key",
            "cable_pf",
            "worst_hz",
            "signed_error_db",
        )
    )
    w.writerows(rows)
for era in ("early_i", "late_i", "sparkletop"):
    subset = [abs(r[-1]) for r in rows if r[0] == era]
    print(f"{era} · {len(subset)} sweeps · maximum {max(subset):.3f} dB")
    if not np.isfinite(subset).all() or max(subset) > 0.5:
        raise SystemExit(f"{era} response exceeds the 0.5 dB tolerance")

# Endpoint differences on the same driven coil, with volume fully clockwise.
# Holding this branch fixed removes the change in source level across groups.
with tempfile.TemporaryDirectory() as tmp, (output / "tone-travel.csv").open("w") as f:
    writer = csv.writer(f)
    writer.writerow(("era", "control", "up_minus_down_80hz_db", "1khz_db", "5khz_db"))
    for era, knob in (
        ("early_i", "tone"),
        ("late_i", "bass"),
        ("sparkletop", "bass"),
        ("sparkletop", "treble"),
    ):
        levels = []
        for position in (0.0, 1.0):
            bass, treble = (0.5, position) if knob == "treble" else (position, 0.5)
            freq, gain = ac_reference(era, 1.0, bass, treble, 27, Path(tmp))
            levels.append(np.interp((80.0, 1000.0, 5000.0), freq, gain))
        writer.writerow((era, knob, *(levels[1] - levels[0])))

# Demodulate a 1 kHz carrier at the loaded Jordan output. Compare its rate to
# the independently simulated oscillator and its excursion to the depth knob.
rows = []
with tempfile.TemporaryDirectory() as tmp:
    wd = Path(tmp)
    for resistance in (102200, 12200, 2200):
        hz, _ = oscillator(resistance, wd)
        for depth in (0.0, 0.5, 1.0):
            subprocess.run(
                [probe, "tremolo", wd / "carrier.wav", "192000", str(hz), str(depth)],
                check=True,
            )
            audio, rate = sf.read(wd / "carrier.wav")
            envelope = abs(hilbert(audio))[rate // 5 : -rate // 5]
            # Use valley-to-valley timing. The oscillator's flat peak makes
            # high-amplitude crossings sensitive to tiny numerical ripple.
            observed_depth = 1 - np.min(envelope) / np.max(envelope)
            observed_hz = None
            if depth:
                valleys = find_peaks(
                    -envelope,
                    prominence=0.25 * np.ptp(envelope),
                    distance=rate * 0.8 / hz,
                )[0]
                if len(valleys) < 2:
                    raise SystemExit("Missing Jordan tremolo cycles")
                observed_hz = rate / np.mean(np.diff(valleys))
                if not np.isclose(observed_hz, hz, rtol=0.01):
                    raise SystemExit(
                        f"Jordan tremolo measured {observed_hz} Hz at {hz} Hz"
                    )
            if not np.isclose(observed_depth, depth, atol=0.02):
                raise SystemExit(
                    f"Jordan tremolo measured depth {observed_depth} at {depth}"
                )
            rows.append((resistance, hz, depth, observed_hz, observed_depth))
with (output / "tremolo.csv").open("w") as f:
    writer = csv.writer(f)
    writer.writerow(
        (
            "rate_resistance_ohms",
            "spice_hz",
            "set_depth",
            "observed_hz",
            "observed_depth",
        )
    )
    writer.writerows(rows)
print(
    f"Jordan tremolo · {len(rows)} time-domain checks · {rows[0][1]:.2f}–{rows[-1][1]:.2f} Hz"
)
