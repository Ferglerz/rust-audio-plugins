"""Compare native-pitch FLACs with the sample-free Rust renderer at a single gain."""

import argparse
import csv
import subprocess
import tempfile
from pathlib import Path

import numpy as np
import soundfile as sf
from circuit import NEUTRAL, calibrate
from measure import partials, recording, slope, spectrum
from panel import jack
from scipy.signal import find_peaks, resample_poly

RATE = 44100
LAYERS = {"PP": 25, "MP": 65, "F": 92, "FF": 116}
PITCHES = (42, 50, 63)


def render(probe, midi, layer, seconds, path, overrides=()):
    subprocess.run(
        [
            str(probe),
            *(["fit"] if overrides else []),
            str(midi),
            str(LAYERS[layer] / 127),
            str(RATE),
            str(seconds),
            str(path),
            *map(str, overrides),
        ],
        check=True,
    )
    wave, rate = sf.read(path)
    if rate != 4 * RATE or not np.isfinite(wave).all():
        raise ValueError("invalid force trace")
    return wave


def rms_db(x):
    return 10 * np.log10(max(1e-24, np.mean(x * x)))


def attack(x):
    return rms_db(x[: round(0.07 * RATE)])


def beat(x, midi):
    # Resolve two sustained partials only if separated by at least two Hann bins.
    start = round(0.15 * RATE)
    hz, amp = spectrum(x[start : start + round(3 * RATE)], RATE)
    resolution = 2 * RATE / min(3 * RATE, len(x) - start)
    root = 440 * 2 ** ((midi - 69) / 12)
    for n in (1, 2, 3, 4, 5, 6):
        k = np.flatnonzero((hz > n * root * 0.985) & (hz < n * root * 1.025))
        peaks, _ = find_peaks(amp[k], prominence=max(amp[k]) * 0.14)
        if len(peaks) >= 2:
            loud = peaks[np.argsort(amp[k[peaks]])[-2:]]
            gap = abs(np.diff(hz[k[loud]])[0])
            if gap >= resolution:
                return n, float(gap)
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("probe", type=Path)
    parser.add_argument("samples", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--live",
        action="store_true",
        help="render the stereo plugin engine for the A/B",
    )
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    panel = jack(dict(NEUTRAL, **calibrate()), RATE * 4)
    pairs = []
    with tempfile.TemporaryDirectory() as temp:
        for midi in PITCHES:
            for layer, velocity in LAYERS.items():
                actual, fs = recording(args.samples, midi, layer)
                if fs != RATE or actual.ndim != 1:
                    raise ValueError("expected the native mono 44.1 kHz Sullivan files")
                if args.live:
                    path = Path(temp, "live.wav")
                    subprocess.run(
                        [
                            args.probe,
                            "play",
                            str(midi),
                            str(velocity / 127),
                            str(RATE),
                            "3.25",
                            path,
                        ],
                        check=True,
                    )
                    model, _ = sf.read(path)
                    model = model[:, 0]
                else:
                    force = render(
                        args.probe, midi, layer, 3.25, Path(temp, "force.wav")
                    )
                    model = resample_poly(panel.apply(force), 1, 4)
                pairs.append((midi, layer, actual[: len(model)], model))
        audition = []
        for layer in ("PP", "MP", "FF"):
            for midi in (42, 43, 63):
                force = render(args.probe, midi, layer, 0.94, Path(temp, "force.wav"))
                audition.append(resample_poly(panel.apply(force), 1, 4))
    # The library's FLACs have almost full-scale peaks even for pp.
    # Hold this one gain fixed for all exported sources and validation pitches.
    model_gain_db = np.mean(
        [attack(a) - attack(m) for n, l, a, m in pairs if n == 42 and l in ("PP", "FF")]
    )
    gain = 10 ** (model_gain_db / 20)
    clips, rows, partial_rows = [], [], []
    for midi, layer, actual, model in pairs:
        model = gain * model
        af, aa = partials(actual, RATE, midi)
        mf, ma = partials(model, RATE, midi)
        dbrat = 20 * np.log10((ma / ma[0]) / (aa / aa[0]))
        cents = 1200 * np.log2(mf / af)
        early = [0.17, 0.35, 0.55, 0.75, 0.95]
        late = [1.2, 1.55, 1.9, 2.25, 2.6]
        for i in range(len(af)):
            partial_rows.append(
                {
                    "midi": midi,
                    "layer": layer,
                    "partial": i + 1,
                    "pitch_error_cents": cents[i],
                    "relative_level_error_db": dbrat[i],
                    "early_decay_error_db_s": slope(model, RATE, mf[i], early)
                    - slope(actual, RATE, af[i], early),
                    "late_decay_error_db_s": slope(model, RATE, mf[i], late)
                    - slope(actual, RATE, af[i], late),
                }
            )
        ab, mb = beat(actual, midi), beat(model, midi)
        rows.append(
            {
                "midi": midi,
                "layer": layer,
                "attack_error_db": attack(model) - attack(actual),
                "median_relative_partial_error_db": np.median(abs(dbrat[1:6])),
                "fundamental_error_cents": cents[0],
                "early_decay_error_db_s": partial_rows[-len(af)][
                    "early_decay_error_db_s"
                ],
                "reference_beat_partial": ab[0] if ab else "",
                "reference_beat_hz": ab[1] if ab else "",
                "model_beat_partial": mb[0] if mb else "",
                "model_beat_hz": mb[1] if mb else "",
            }
        )
        for x in (actual, model):
            excerpt = x[: round(1.3 * RATE)].copy()
            excerpt[-441:] *= np.linspace(1, 0, 441)
            clips.extend((excerpt, np.zeros(round(0.22 * RATE))))
    for name, data in [("strikes", rows), ("partials", partial_rows)]:
        with (args.output / f"{name}.csv").open("w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=data[0].keys())
            w.writeheader()
            w.writerows(data)
    ab = np.concatenate(clips)
    sf.write(
        args.output / "recording-then-model.wav",
        ab * 0.90 / max(abs(ab)),
        RATE,
        subtype="PCM_24",
    )
    play = np.concatenate(audition)
    sf.write(
        args.output / "prototype.wav",
        play * 0.90 / max(abs(play)),
        RATE,
        subtype="PCM_24",
    )
    (args.output / "recording-credits.txt").write_text(
        "Yamaha CP80 Electric Grand Piano by Greg Sullivan, SFZ conversion by kinwie.\n"
        "https://github.com/sfzinstruments/GregSullivan.E-Pianos/tree/8c3e581acda3594b553948ff0222d4f84a698376\n"
        "CC BY 3.0 https://creativecommons.org/licenses/by/3.0/\n"
        "Recording excerpts trimmed, faded, level-scaled and interleaved with synthesized audio.\n"
    )
    print(f"One model alignment {model_gain_db:.2f} dB from F#2 PP/FF")
    for name, subset in [
        ("PP/FF", [r for r in rows if r["layer"] in ("PP", "FF")]),
        ("held-out MP/F", [r for r in rows if r["layer"] in ("MP", "F")]),
    ]:
        print(
            name,
            {
                k: round(float(np.median([abs(r[k]) for r in subset])), 2)
                for k in (
                    "attack_error_db",
                    "median_relative_partial_error_db",
                    "fundamental_error_cents",
                    "early_decay_error_db_s",
                )
            },
        )


if __name__ == "__main__":
    main()
