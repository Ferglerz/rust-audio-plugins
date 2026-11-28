"""Compare three Baldwin owner performances to the three-channel stock probe."""
import argparse
import csv
import hashlib
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import resample_poly

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "pianet-n" / "research"))
from compare_claviset import ANALYSIS, RATE, clip, pair, partials

# Times refer to each owner's video presentation. The E4 and D2 chord samples
# on the Tipps instrument are contextual checks, excluded from fit summaries.
CASES = {
    "tipps": [(54, 85.90, "fit"), (38, 97.42, "chord"), (64, 90.22, "chord")],
    "roberti": [(38, 48.12, "fit"), (69, 30.06, "fit"),
                (74, 44.34, "fit"), (76, 49.56, "holdout")],
    "martinez": [(43, 74.22, "holdout"), (53, 31.56, "holdout"),
                 (67, 51.06, "holdout")],
}
DIGEST = {
    "tipps": "398c55eb6812d8484bc7d1b1f211378c539e4c6d21f2fa65acecee64331a052c",
    "roberti": "f5d1e5f063a2d7b8463baae81ea6db3d2561342350a558ca061ffd083072f300",
    "martinez": "11f851c393e0bebe5c2933108670f9ac723e2ada6bafe2455f370f5225035723",
}
DECAYS = {
    "roberti": [(38, 48.18, 48.74), (64, 25.70, 26.26), (76, 49.56, 50.04)],
    "martinez": [(43, 48.68, 49.24)],
}
ATTACKS = {"tipps": [(54, 85.462)], "roberti": [(76, 49.460)]}


def stock(root, midi):
    audio, rate = sf.read(root / "stock" / f"{midi}.wav")
    if rate != 48000 or audio.ndim != 2 or audio.shape[1] != 3:
        raise ValueError("Expected 48 kHz bass, treble and jack volts")
    return resample_poly(audio[:, 2], 147, 320)


def string_partials(y, t, midi):
    """Fit f1 and string stiffness B; higher partials can be substantially sharp."""
    samples = clip(y, ANALYSIS, t, .16)
    window = np.hanning(len(samples))
    samples = samples - np.average(samples, weights=window)
    bins = np.fft.rfftfreq(65536, 1 / ANALYSIS)
    spec = abs(np.fft.rfft(samples * window, n=65536))
    nominal = 440 * 2**((midi-69)/12)
    f0 = np.linspace(nominal*2**(-35/1200), nominal*2**(35/1200), 181)
    # Find H1 separately; otherwise a much louder H2 can drag the fitted
    # fundamental several Hz off the real first mode at high string stiffness.
    f = f0[np.argmax(np.interp(f0, bins, spec))]
    b = np.linspace(0, .025, 2501)[:, None]
    orders = np.arange(1, 7)
    candidates = f * orders * np.sqrt((1 + b * orders**2) / (1 + b))
    score = np.interp(candidates[:, 1:], bins, spec, right=0.)
    frequencies = candidates[np.argmax((score**2).sum(axis=1))]
    basis = np.exp(2j*np.pi*np.arange(len(samples))[:, None]*frequencies/ANALYSIS)
    amplitude = 2 * (samples*window) @ basis.conj() / window.sum()
    fit = 1 - np.average((samples - np.real(basis @ amplitude))**2, weights=window) / np.average(samples**2, weights=window)
    return frequencies, fit, 20 * np.log10(np.maximum(abs(amplitude[1:]), 1e-15) / abs(amplitude[0]))


def level(y, t, frequency, duration):
    x = clip(y, RATE, t, duration)
    w = np.hanning(len(x))
    phase = np.exp(-2j*np.pi*frequency*np.arange(len(x))/RATE)
    return 2*abs((w*x) @ phase) / w.sum()


def decay(y, midi, start, end):
    hz = 440*2**((midi-69)/12)
    times = np.arange(start, end+.001, .04)
    levels = [20*np.log10(max(level(y, t, hz, .12), 1e-12)) for t in times]
    slope, intercept = np.polyfit(times-times[0], levels, 1)
    residual = np.sqrt(np.mean((levels-slope*(times-times[0])-intercept)**2))
    return -slope, residual


def rise(y, midi, start):
    """10–90% rise above the local pre-onset H1 amplitude; 1 ms time grid."""
    hz = 440*2**((midi-69)/12)
    times = np.arange(start-.024, start+.075, .001)
    duration = max(.012, 3/hz)
    envelope = np.array([level(y, t, hz, duration) for t in times])
    floor = np.median(envelope[:15])
    peak = np.argmax(envelope[times >= start]) + np.argmax(times >= start)
    envelope = (envelope-floor)/(envelope[peak]-floor)
    index = np.arange(len(times))
    hi = np.flatnonzero((index <= peak) & (envelope >= .9))[-1]
    lo = np.flatnonzero((index <= hi) & (envelope <= .1))[-1]
    hi = np.flatnonzero((index > lo) & (envelope >= .9))[0]
    return times[lo], times[hi]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path, help="probe directory containing stock/")
    for src in CASES:
        parser.add_argument("--" + src, type=Path, required=True, help="22.05 kHz mono hardware WAV")
    args = parser.parse_args()
    columns = ["source", "midi", "owner_center_s", "use", "owner_root_hz", "model_root_hz",
               "owner_harmonic_family_fit", "model_six_mode_fit", "partial",
               "owner_partial_hz", "model_partial_hz", "owner_dBc", "model_dBc"]
    with (args.root / "hardware-harmonics.csv").open("w", newline="") as file:
        rows = csv.writer(file)
        rows.writerow(columns)
        for source, cases in CASES.items():
            path = getattr(args, source)
            y, rate = sf.read(path)
            if rate != RATE or y.ndim != 1:
                raise ValueError(f"{path} must be 22050 Hz mono")
            if hashlib.sha256(path.read_bytes()).hexdigest() != DIGEST[source]:
                print(f"{source} decode differs from the documented capture")
            ref = resample_poly(y, 1, 2)
            errors, listen = [], []
            for midi, t, use in cases:
                model = stock(args.root, midi)
                _, fit, _ = partials(ref, t, midi)
                f, _, h = string_partials(ref, t, midi)
                mf, mfit, mh = string_partials(resample_poly(model, 1, 2), .26, midi)
                print(source, midi, t, f"fit {fit:.3f}",
                      "owner", np.round(h, 1), "stock", np.round(mh, 1))
                if use != "chord":
                    errors.append(abs(np.maximum(-40., h) - np.maximum(-40., mh)))
                for order, (a, b) in enumerate(zip(h, mh), 2):
                    rows.writerow([source, midi, t, use, round(f[0], 3), round(mf[0], 3),
                                   round(fit, 4), round(mfit, 4), order,
                                   round(f[order-1], 3), round(mf[order-1], 3),
                                   round(a, 2), round(b, 2)])
                listen.append(pair(y, t, model))
            print(source, "median error dB / H2-only error dB", round(np.median(errors), 2),
                  round(np.median(np.asarray(errors)[:, 0]), 2))
            sf.write(args.root / f"{source}-then-model.wav", np.concatenate(listen), RATE, subtype="PCM_24")
            for midi, a, b in DECAYS.get(source, []):
                print(source, midi, "held H1 loss dB/s, fit residual dB, model loss/residual",
                      np.round(decay(y, midi, a, b), 2), np.round(decay(stock(args.root, midi), midi, .15, .15+b-a), 2))
            for midi, start in ATTACKS.get(source, []):
                ref_a, ref_b = rise(y, midi, start)
                mod_a, mod_b = rise(stock(args.root, midi), midi, .05)
                print(source, midi, "attack interval, duration ms, model ms", round(ref_a, 3),
                      round(ref_b, 3), round(1000*(ref_b-ref_a)), round(1000*(mod_b-mod_a)))
            if source == "martinez":
                hz = 440*2**((67-69)/12)
                # Ensemble G4 clears here; footage cannot time the damper itself.
                for track, a, b in [(y, 122.84, 122.99), (stock(args.root, 67), .78, .93)]:
                    print("G4 decline dB", a, b, round(20*np.log10(level(track, a, hz, .04)/level(track, b, hz, .04)), 2))


if __name__ == "__main__":
    main()
