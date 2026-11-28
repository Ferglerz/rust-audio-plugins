"""Check B6 voicing against native strikes and compare surrounding keys with a prior build."""

import argparse
import tempfile
from pathlib import Path

import numpy as np
import soundfile as sf
from compare import LAYERS, RATE
from measure import partials, recording
from validate_keys import live, table

WINDOWS = [(0.016, 0.066), (0.065, 0.240), (0.150, 0.290), (0.250, 0.400)]
RATES = (44100, 48000, 96000)


def levels(x, rate, start, end):
    _, amps = partials(x, rate, 95, start, end, 2)
    return 20 * np.log10(amps)


def compare(args, path):
    rows, clips = [], []
    for layer in ("MP", "F", "FF"):
        reference, fs = recording(args.samples, 95, layer)
        for rate in RATES:
            signals = {"recording": (reference, fs)}
            velocity = LAYERS[layer] / 127
            if args.before:
                signals["before"] = (live(args.before, 95, velocity, rate, path), rate)
            signals["after"] = (live(args.probe, 95, velocity, rate, path), rate)
            for start, end in WINDOWS:
                row = {
                    "layer": layer,
                    "velocity": LAYERS[layer],
                    "rate": rate,
                    "start": start,
                    "end": end,
                }
                for label, (wave, sample_rate) in signals.items():
                    f1, f2 = levels(wave, sample_rate, start, end)
                    row[label] = f2 - f1
                    if rate == RATE and start == 0.065:
                        # Match the fundamental so a louder strike cannot mask the overtone.
                        clip = wave[: round(0.48 * RATE)].copy() * 10 ** (-f1 / 20)
                        clip[-441:] *= np.linspace(1, 0, 441)
                        clips.extend((clip, np.zeros(round(0.22 * RATE))))
                rows.append(row)
    table(args.output, "b6-levels.csv", rows)
    wave = np.concatenate(clips)
    sf.write(
        args.output / "b6-comparison.wav",
        0.9 * wave / max(abs(wave)),
        RATE,
        subtype="PCM_24",
    )
    names = (
        "recording, previous model, revised model"
        if args.before
        else "recording, model"
    )
    (args.output / "b6-credits.txt").write_text(
        f"Native B6, MP, F and FF. Each group plays {names}.\n"
        "Fundamentals aligned over 65–240 ms. Excerpts trimmed, faded and normalized together.\n"
        "CP80 recordings by Greg Sullivan. SFZ conversion by kinwie. CC BY 3.0.\n"
        "https://github.com/sfzinstruments/GregSullivan.E-Pianos/tree/8c3e581acda3594b553948ff0222d4f84a698376\n"
        "https://creativecommons.org/licenses/by/3.0/\n"
    )
    for row in rows:
        if row["start"] == 0.065:
            error = row["after"] - row["recording"]
            print(
                f"{row['layer']} at {row['rate']} Hz: H2/H1 {row['after']:.2f} dB, error {error:+.2f} dB"
            )
            if abs(error) > 1.25:
                raise AssertionError(
                    "B6 second partial exceeded the 1.25 dB voicing tolerance"
                )


def check_neighbors(args, path):
    checked = 0
    for rate in RATES:
        for note in (91, 92, 93, 94, 96, 97, 98):
            for velocity in LAYERS.values():
                before = live(args.before, note, velocity / 127, rate, path)
                after = live(args.probe, note, velocity / 127, rate, path)
                if not np.array_equal(before, after):
                    raise AssertionError(
                        f"Neighbor changed at MIDI {note}, velocity {velocity}, {rate} Hz"
                    )
                checked += 1
    print(f"{checked} neighboring renders matched sample for sample.")


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("probe", type=Path)
    p.add_argument("samples", type=Path)
    p.add_argument("output", type=Path)
    p.add_argument("--before", type=Path)
    args = p.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp, "strike.wav")
        compare(args, path)
        if args.before:
            check_neighbors(args, path)
