"""Compare the default 80/0.75/da-adds and 80/0.19/da-adds renders with hardware."""
import argparse
import hashlib
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import find_peaks
from action import write_action

RATE = 44_100
KEYS = {46: "B♭2", 64: "E4", 78: "F♯5"}
# Search brackets are seconds in the original Aalto file; no key or pickup metadata.
STRIKES = {46: [20.17], 64: [35.35, 41.60], 78: [49.82, 56.16]}
# Two high-purity, distinct B♭2 strikes per selector in the four AMAZONA performances.
SWITCH_WINDOWS = {"CA": [5.77, 8.17], "CB": [5.81, 9.34], "DA": [5.88, 8.19], "DB": [5.76, 9.24]}
# Estimated acoustic key-up, inferred from departure of the held partials.
RELEASES = [5.885, 7.092, 8.271]


def read(path):
    data, rate = sf.read(path, always_2d=True)
    if rate != RATE:
        raise ValueError(f"Expected {RATE} Hz in {path}")
    return data.mean(axis=1)


def segment(x, start, duration):
    i = round(start * RATE)
    return x[i:i + round(duration * RATE)]


def level(x):
    return 10 * np.log10(max(1e-20, np.mean(x * x)))


def spectrum(x):
    n = max(32768, 1 << (len(x) - 1).bit_length())
    return np.fft.rfftfreq(n, 1 / RATE), abs(np.fft.rfft(x * np.hanning(len(x)), n))


def peak_hz(x, low, high):
    f, mag = spectrum(x)
    peaks, _ = find_peaks(mag)
    peaks = peaks[(f[peaks] > low) & (f[peaks] < high)]
    if not len(peaks):
        raise ValueError(f"No spectral peak between {low:g} and {high:g} Hz")
    i = peaks[np.argmax(mag[peaks])]
    a, b, c = np.log(mag[i - 1:i + 2])
    return f[i] + 0.5 * (a - c) / (a - 2 * b + c) * f[1]


def attack(x, start, midi):
    y = segment(x, start, 0.08)
    width = round(0.001 * RATE)
    energy = np.convolve(y * y, np.ones(width) / width, mode="valid")
    peak = np.percentile(energy, 95)
    onset = np.flatnonzero(energy >= peak * 0.01)[0]
    loud = np.flatnonzero(energy >= peak * 10 ** (-3 / 10))[0]
    t = start + onset / RATE
    f, m = spectrum(segment(x, t, 0.025))
    upper = 10 * np.log10(sum(m[f >= 2000] ** 2) / sum(m ** 2))
    nominal = 440 * 2 ** ((midi - 69) / 12)
    pitch = peak_hz(segment(x, t + 0.06, 0.13), 0.95 * nominal, 1.05 * nominal)
    return t, 1000 * (loud - onset) / RATE, upper, pitch


def partials(x, start):
    f, m = spectrum(segment(x, start, 0.09))
    f0 = 116.54
    amps = np.array([max(m[abs(f - n * f0) < 15]) for n in range(1, 11)])
    band = (f > 70) & (f < 1250)
    harmonic = abs(f - np.round(f / f0) * f0) < 20
    purity = sum(m[band & harmonic] ** 2) / sum(m[band] ** 2)
    return 20 * np.log10(amps / max(amps)), purity


def release(x, off, nominal):
    held = level(segment(x, off - 0.055, 0.040))
    times = np.arange(0, 0.12, 0.001)
    # 10-ms centred RMS, sustained crossing for at least 6 successive frames.
    quiet = [level(segment(x, off + t - 0.005, 0.010)) <= held - 20 for t in times]
    crossings = np.flatnonzero(np.convolve(quiet, np.ones(6), mode="valid") == 6)
    t20 = times[crossings[0]] * 1000 if len(crossings) else float("nan")
    pitch = peak_hz(segment(x, off + 0.034, 0.050), nominal * 0.68, nominal * 1.12)
    return t20, pitch


def audition(path, excerpts):
    gap = np.zeros(round(0.18 * RATE))
    pieces = []
    for audio in excerpts:
        audio = audio.copy()
        onset_fade = min(len(audio), round(0.001 * RATE))
        audio[:onset_fade] *= np.linspace(0, 1, onset_fade)
        fade = min(len(audio), round(0.008 * RATE))
        audio[-fade:] *= np.linspace(1, 0, fade)
        pieces.extend((audio, gap))
    reel = np.concatenate(pieces)
    sf.write(path, 0.86 * reel / max(abs(reel)), RATE, subtype="PCM_24")


def burst(x, centre):
    # Estimate the acoustic key-up from the loudest 10-ms RMS in a narrow event bracket.
    times = centre + np.arange(-0.015, 0.151, 0.001)
    levels = np.array([level(segment(x, t - 0.005, 0.010)) for t in times])
    peak = np.argmax(levels[:38])
    below = levels[peak:] <= levels[peak] - 10
    sustained = np.flatnonzero(np.convolve(below, np.ones(6), mode="valid") == 6)
    t10_ms = (times[peak+sustained[0]] - times[peak]) * 1000 if len(sustained) else float("nan")
    margin = levels[peak] - level(segment(x, centre + 0.25, 0.14))
    return t10_ms, margin


def jack_comparison(probe, singles, recordings):
    print("\nLoaded jack, all tabs on | attack rise ms | attack >2 kHz dB | short T20 ms | long burst T10 ms")
    attacks = []
    for (midi, name), hold in zip(KEYS.items(), [14.607, 5.303, 5.679]):
        jack = read(probe / str(midi) / "jack-CA.wav")
        onset, rise, high, pitch = attack(jack, 0.04, midi)
        t20, _ = release(read(probe / "short" / str(midi) / "jack-CA.wav"), 0.24, pitch)
        t10, _ = burst(read(probe / "long" / str(midi) / "jack-CA.wav"), 0.05 + hold)
        print(f"{name} | {rise:.2f} | {high:.1f} | {t20:.0f} | {t10:.0f}")
        recorded_onset = attack(singles, STRIKES[midi][0], midi)[0]
        if not attacks:
            trim = 10 ** ((level(segment(singles, recorded_onset, .1)) -
                           level(segment(jack, onset, .1))) / 20)
        attacks.extend((segment(singles, recorded_onset - .005, .45),
                        segment(jack, onset - .005, .45) * trim))
    audition(probe / "jack-attacks-hardware-then-model.wav", attacks)

    print("CA jack attack sensitivity | rise in ms for velocities 32/80/120 | hard minus soft level in first 25 ms, dB")
    for midi, name in KEYS.items():
        rises, levels = [], []
        for folder in [probe / "velocity-32", probe, probe / "velocity-120"]:
            jack = read(folder / str(midi) / "jack-CA.wav")
            onset, rise, _, _ = attack(jack, .04, midi)
            rises.append(f"{rise:.2f}")
            levels.append(level(segment(jack, onset, .025)))
        print(f"{name} | {' / '.join(rises)} | {levels[-1] - levels[0]:+.1f}")

    print("Loaded jack B♭2 | H1–10 median absolute error, DA adds/subtracts dB")
    switches = []
    for mode, times in SWITCH_WINDOWS.items():
        recorded = recordings[mode]
        measured = np.median([partials(recorded, t)[0] for t in times], axis=0)
        errors = []
        for position in (mode, {"DA": "DB", "DB": "DA"}.get(mode, mode)):
            jack = read(probe / "46" / f"jack-{position}.wav")
            levels, _ = partials(jack, .14)
            errors.append(np.median(abs(levels - measured)))
        print(f"{mode} | {errors[0]:.1f} / {errors[1]:.1f}")
        # DA subtracts for the dry performance comparison. Match gain once at CA.
        if not switches:
            trim = 10 ** ((level(segment(recorded, times[0], .09)) -
                           level(segment(jack, .14, .09))) / 20)
        switches.extend((segment(recorded, times[0], .09), segment(jack, .14, .09) * trim))
    audition(probe / "jack-switches-hardware-then-model.wav", switches)
    releases = []
    short_bass = read(probe / "short" / "46" / "jack-CA.wav")
    trim = 10 ** ((level(segment(recordings["CA"], RELEASES[0] - .055, .040)) -
                   level(segment(short_bass, .24 - .055, .040))) / 20)
    for off in RELEASES:
        releases.extend((segment(recordings["CA"], off - .18, .30),
                         segment(short_bass, .24 - .18, .30) * trim))
    audition(probe / "jack-releases-hardware-then-model.wav", releases)


def live_switches(probe):
    print("Running jack vs fixed position | difference relative to destination in dB, first 10 ms / 63–98 ms")
    sounds = []
    for midi, key in KEYS.items():
        directory = probe / str(midi)
        moving = read(directory / "jack-selector-sweep.wav")
        initial = read(directory / "jack-CA.wav")
        edge = round(0.24 * RATE)
        np.testing.assert_array_equal(moving[:edge], initial[:edge])
        for start, name in [(0.24, "CB"), (0.43, "DA"), (0.62, "DB")]:
            fixed = read(directory / f"jack-{name}.wav")
            difference = moving - fixed

            def residual(offset, duration):
                where = start + offset
                return level(segment(difference, where, duration)) - level(segment(fixed, where, duration))

            transient = residual(0, 0.010)
            settled = residual(0.063, 0.035)
            print(f"{key} {name} | {transient:.1f} / {settled:.1f}")
            if transient < -70 or settled > -65:
                raise ValueError(f"{key} {name} did not retain and settle the prior circuit state")
        sounds.append(moving)
    audition(probe / "jack-live-switches.wav", sounds)


def compare(ref, probe):
    manifest = Path(__file__).with_name("references.sha256")
    for line in manifest.read_text().splitlines():
        digest, name = line.split()
        if hashlib.sha256((ref / name).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Unexpected recording content in {name}")
    singles = read(ref / "samples_wholekeyboard.mp3")
    recordings = {s: read(ref / f"Hohner-Clavinet-D6_Pickup_{'DA-4' if s == 'DA' else s}.mp3")
                  for s in SWITCH_WINDOWS}
    model = {m: {s: read(probe / str(m) / f"{s}.wav") for s in SWITCH_WINDOWS} for m in KEYS}
    if any(len(y) != round(1.30 * RATE) for bank in model.values() for y in bank.values()):
        raise ValueError("Expected 0.75-second holds in the main render")
    for midi, bank in model.items():
        stems, rate = sf.read(probe / str(midi) / "pickups.wav", always_2d=True)
        if stems.shape != (len(bank["CA"]), 2) or rate != RATE:
            raise ValueError(f"Unexpected pickup stem format for {KEYS[midi]}")
        centre, bridge = stems.T
        for name, channel in zip(("centre", "bridge"), stems.T):
            mono, rate = sf.read(probe / str(midi) / f"{name}.wav")
            if rate != RATE or not np.array_equal(mono, channel):
                raise ValueError(f"{KEYS[midi]} {name} mono stem differs from the stereo pair")
        expected = {"CA": centre * 10/11, "CB": bridge * 10/11,
                    "DA": (centre + bridge) * 10/21, "DB": (centre - bridge) * 10/21}
        for mode, signal in expected.items():
            np.testing.assert_allclose(bank[mode], signal, atol=1e-7, rtol=1e-6,
                                       err_msg=f"{KEYS[midi]} {mode} disagrees with the raw stems")
    print("Attack | source/time | rise ms | energy above 2 kHz dB | held Hz")
    attacks = []
    for midi, note in KEYS.items():
        centre = model[midi]["CA"]
        for i, start in enumerate(STRIKES[midi]):
            t, rise, high, pitch = attack(singles, start, midi)
            print(f"{note} | hardware {t:.3f} | {rise:.2f} | {high:.1f} | {pitch:.2f}")
            if i == 0:
                attacks.append(segment(singles, t - 0.005, 0.45))
        t, rise, high, pitch = attack(centre, 0.04, midi)
        print(f"{note} | model {t:.3f} | {rise:.2f} | {high:.1f} | {pitch:.2f}")
        example = segment(centre, t - 0.005, 0.45)
        # Gain match each model to its reference over the first 100 ms after onset.
        example = example * 10 ** ((level(attacks[-1][:4630]) - level(example[:4630])) / 20)
        attacks.append(example)
    audition(probe / "attacks-hardware-then-model.wav", attacks)

    print("\nB♭2 switch | window purities | H5 vs neighbours, hardware/model dB | H7:H2 hardware/model dB | H1–10 median abs error default/reversed dB")
    for mode, starts in SWITCH_WINDOWS.items():
        pairs = [partials(recordings[mode], t) for t in starts]
        measured = np.median([p[0] for p in pairs], axis=0)
        predicted, _ = partials(model[46][mode], 0.14)
        alternate = {"DA": "DB", "DB": "DA"}.get(mode, mode)
        reversed_pair, _ = partials(model[46][alternate], 0.14)
        purities = ", ".join(f"{p[1]:.2f}" for p in pairs)
        def notch(h):
            return h[4] - (h[3] + h[5]) / 2
        errors = [np.median(abs(measured - h)) for h in (predicted, reversed_pair)]
        print(f"{mode} | {purities} | {notch(measured):.1f}/{notch(predicted):.1f} | "
              f"{measured[6]-measured[1]:.1f}/{predicted[6]-predicted[1]:.1f} | {errors[0]:.1f}/{errors[1]:.1f}")
    bass_ref, _ = partials(singles, 20.28)
    bass_model, _ = partials(model[46]["CA"], 0.14)
    print(f"Aalto B♭2 single, H5:H2 dB hardware/model "
          f"{bass_ref[4]-bass_ref[1]:.1f}/{bass_model[4]-bass_model[1]:.1f}")
    bridge_ref, _ = partials(read(ref / "D.wav"), 1.27)
    bridge_model, _ = partials(model[46]["CB"], 0.14)
    print(f"Independent Aalto B-pickup riff at 1.27 s, H7:H2 dB hardware/model "
          f"{bridge_ref[6]-bridge_ref[1]:.1f}/{bridge_model[6]-bridge_model[1]:.1f}")

    print("\nRelease | approximate off s | T20 ms | released Hz | semitones from held pitch")
    keyups = []
    short_bass = read(probe / "short" / "46" / "CA.wav")
    for off in RELEASES:
        t20, freq = release(recordings["CA"], off, 116.54)
        print(f"B♭2 hardware | {off:.3f} | {t20:.0f} | {freq:.1f} | {12*np.log2(freq/116.54):.2f}")
        recorded_note = segment(recordings["CA"], off - 0.18, 0.30)
        model_note = segment(short_bass, 0.24 - 0.18, 0.30)
        gain_db = level(segment(recordings["CA"], off - 0.055, 0.04)) - level(segment(short_bass, 0.185, 0.04))
        keyups.extend([recorded_note, model_note * 10 ** (gain_db / 20)])
    audition(probe / "releases-hardware-then-model.wav", keyups)
    for midi, note in KEYS.items():
        y = read(probe / "short" / str(midi) / "CA.wav")
        if len(y) != round(0.74 * RATE):
            raise ValueError("Expected 0.19-second holds in the short render")
        nominal = 440 * 2 ** ((midi - 69) / 12)
        held = peak_hz(segment(y, 0.08, 0.14), 0.95 * nominal, 1.05 * nominal)
        t20, freq = release(y, 0.24, nominal)
        print(f"{note} model | 0.240 | {t20:.0f} | {freq:.1f} | {12*np.log2(freq/held):.2f}")
    print("Long-hold key-up | reference time | hardware headroom dB | burst T10 ms hardware/model")
    for midi, event, hold in [(46, 34.808, 14.607), (64, 40.677, 5.303), (78, 55.526, 5.679)]:
        y = read(probe / "long" / str(midi) / "CA.wav")
        if len(y) != round(RATE * hold) + round(RATE * 0.55):
            raise ValueError(f"Unexpected long hold for {KEYS[midi]}")
        hardware_ms, margin = burst(singles, event)
        model_ms, _ = burst(y, 0.05 + hold)
        print(f"{KEYS[midi]} | {event:.3f} | {margin:.1f} | {hardware_ms:.0f}/{model_ms:.0f}")
    # Switch on a continuously sounding string. Preserve all electrical level differences.
    sweeps = []
    for bank in model.values():
        boundaries = [0, 0.24, 0.43, 0.62, len(bank["CA"]) / RATE]
        sweeps.append(np.concatenate([segment(bank[s], a, b-a) for s, a, b in
                                      zip(SWITCH_WINDOWS, boundaries, boundaries[1:])]))
    audition(probe / "pickup-switches.wav", sweeps)
    jack_comparison(probe, singles, recordings)
    live_switches(probe)
    write_action(probe)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ("references", "probe"):
        parser.add_argument(arg, type=Path)
    args = parser.parse_args()
    compare(args.references, args.probe)
