"""Compare all 64 Rust jack transfer functions with ngspice using one open-circuit pickup impulse."""
import argparse
import hashlib
import subprocess
import tempfile
from pathlib import Path

import numpy as np
from scipy.interpolate import CubicSpline

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
RATE = 96000
SETTINGS = {
    "CA": (1.0, 0.37, 200, 1e15),
    "CB": (1.0, 0.37, 1e15, 200),
    "DA": (1.0, 0.37, 200, 200),
    "DB": (1.0, -0.37, 200, 200),
}


def spice(directory, setting, mask, board="d6", volume=0.73):
    centre, bridge, rc, rb = SETTINGS[setting]
    soft, med, treble, brilliant = [(mask >> i) & 1 for i in range(4)]
    deck = (HERE / f"{board}.cir").read_text().replace(
        ".param DUAL=0 B_THROW=0",
        f".param DUAL={int(setting[0] == 'D')} B_THROW={int(setting[1] == 'B')}",
    ).replace(
        ".param E1=1 E2=0 RC=200 RB=1e15",
        f".param E1={centre} E2={bridge} RC={rc} RB={rb}",
    ).replace(
        ".param SOFT=1 MEDIUM=1 TREBLE=1 BRILLIANT=1 VOL=1",
        f".param SOFT={soft} MEDIUM={med} TREBLE={treble} BRILLIANT={brilliant} VOL={volume}",
    )
    if board == "e7" and mask == 0:
        # At this very low signal level the 1 mΩ contacts exceed the actual
        # supply feedthrough. Use the same ideal short as the digital switch.
        lines = []
        for line in deck.splitlines():
            if line.startswith("Rtab"):
                continue
            if line.startswith(("R", "C", "L")):
                tokens = line.split()
                for i in (1, 2):
                    if tokens[i] in ("tones", "lo", "mid", "hi"):
                        tokens[i] = "0"
                line = " ".join(tokens)
            lines.append(line)
        deck = "\n".join(lines) + "\n"
    deck = deck.replace(".end\n", """.control
set wr_singlescale
set numdgt=14
ac dec 384 25 16000
wrdata response.dat v(jack)
ac lin 6801 2800 4500
wrdata notch.dat v(jack)
quit
.endc
.end
""")
    (directory / "sweep.cir").write_text(deck)
    subprocess.run(["ngspice", "-b", "sweep.cir"], cwd=directory, check=True,
                   stdout=subprocess.DEVNULL)
    sweep = np.vstack([np.loadtxt(directory / name) for name in ("response.dat", "notch.dat")])
    f, real, imag = sweep[sweep[:, 0].argsort()].T
    return f, real + 1j * imag


def main(board):
    worst_gain, worst_phase = (0, ""), (0, "")
    with tempfile.TemporaryDirectory() as scratch:
        directory = Path(scratch)
        subprocess.run(["cargo", "run", "--quiet", "--release", "-p", "clavinet-d6", "--example",
                        "d6_panel_impulses", "--", str(directory), board], check=True, cwd=REPO)
        for setting in SETTINGS:
            for mask in range(16):
                impulses = np.fromfile(directory / f"{setting}-{mask}.f64", dtype="<f8")
                frequency, target = spice(directory, setting, mask, board)
                if mask == 0 and board == "d6":
                    assert not impulses.any(), "all-off Rust transfer leaked"
                    assert np.max(abs(target)) < 0.003, "contact leakage at 1 mOhm"
                    continue
                # Sample the measured Rust response at the bilinear images of the AC frequencies.
                digital = RATE / np.pi * np.arctan(np.pi * frequency / RATE)
                h = CubicSpline(np.fft.rfftfreq(len(impulses), 1 / RATE),
                                np.fft.rfft(impulses))(digital)
                err = h / target
                for values, phase in [(np.abs(20*np.log10(abs(err))), False),
                                      (np.abs(np.angle(err, deg=True)), True)]:
                    i = np.argmax(values)
                    item = (values[i], f"{setting}/{mask} at {frequency[i]:.2f} Hz")
                    if phase:
                        worst_phase = max(worst_phase, item)
                    else:
                        worst_gain = max(worst_gain, item)
                if setting == "CA" and mask == 0:
                    print(f"CA/all-off peak supply feedthrough {20*np.log10(max(abs(target))):.2f} dBV/V")
                if setting == "CA" and mask in (1, 2, 4, 8, 12, 15):
                    if mask in (12, 15):
                        notch = np.flatnonzero((frequency >= 2800) & (frequency <= 4500))
                        i = notch[np.argmin(abs(target[notch]))]
                        print(f"CA/{mask:02d} upper-mid notch at {frequency[i]:.2f} Hz, "
                              f"gain {20*np.log10(abs(target[i])):.2f} dB")
                    probe = np.array([100, 500, 1600, 4000, 10000])
                    print(f"CA/{mask:02d} dB at {probe.tolist()} Hz:",
                          np.round(np.interp(probe, frequency, 20*np.log10(abs(target))), 2))
    print(f"Maximum magnitude difference {worst_gain[0]:.4f} dB ({worst_gain[1]})")
    print(f"Maximum phase difference {worst_phase[0]:.4f} deg ({worst_phase[1]})")
    if worst_gain[0] > 0.10 or worst_phase[0] > 1.0:
        raise ValueError("Rust response diverges from circuit AC sweep")


def compare_recordings(recordings):
    """Compare the change in partial levels between two performances of the same pitch."""
    import soundfile as sf

    notes = {
        "Soft": (64, [18.42, 18.87], 1),
        "Treble": (64, [12.63, 39.96], 4),
        "Medium-1": (46, [5.16, 20.34], 2),
        "Brilliant": (46, [6.81, 25.26], 8),
        "Soft_Medium_Brilliant": (46, [22.98, 23.34, 23.70], 11),
    }
    measured, predicted = {}, {}
    hashes = dict(line.split()[::-1] for line in (HERE / "tones.sha256").read_text().splitlines())
    with tempfile.TemporaryDirectory() as temp:
        for name, (midi, windows, mask) in notes.items():
            path = recordings / f"Hohner-Clavinet-D6_{name}.mp3"
            if hashlib.sha256(path.read_bytes()).hexdigest() != hashes[path.name]:
                raise ValueError(f"Unexpected recording content in {path.name}")
            samples, rate = sf.read(path, always_2d=True)
            f0 = 440 * 2 ** ((midi - 69) / 12)
            harmonics = f0 * np.arange(1, 11)
            partial_levels = []
            for start in windows:
                first = round(start * rate)
                x = samples[first:first + round(0.09 * rate)].mean(axis=1)
                freq = np.fft.rfftfreq(32768, 1 / rate)
                mag = np.abs(np.fft.rfft(x * np.hanning(len(x)), 32768))
                amps = [np.max(mag[abs(freq - hz) < 18]) for hz in harmonics]
                partial_levels.append(20 * np.log10(amps) - 20 * np.log10(amps[1]))
            measured[name] = np.array(partial_levels)
            freq, h = spice(Path(temp), "CA", mask)
            levels = np.interp(harmonics, freq, 20 * np.log10(abs(h)))
            predicted[name] = levels - levels[1]
    for bright, dark in [("Treble", "Soft"), ("Brilliant", "Medium-1"),
                         ("Soft_Medium_Brilliant", "Medium-1")]:
        hardware = np.median(measured[bright], axis=0) - np.median(measured[dark], axis=0)
        model = predicted[bright] - predicted[dark]
        print(f"{bright} over {dark} | H7:H2 real/model {hardware[6]:.1f}/{model[6]:.1f} dB | "
              f"H1–10 median absolute difference {np.median(abs(hardware-model)):.1f} dB")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--recordings", type=Path, help="folder with the five AMAZONA tone takes")
    parser.add_argument("--board", choices=["d6", "e7"], default="d6")
    args = parser.parse_args()
    main(args.board)
    if args.recordings:
        compare_recordings(args.recordings)
