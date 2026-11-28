"""Measure releases in exposed Sparkletop notes in Grall's second harp clip.

Usage: python3 check_releases.py PROBE sparkle2.mp3 OUTPUT
The narrow windows end before the next attack. Timing is relative to -3 dB
because the recording has no synchronized key or pedal event.
"""

import csv
import subprocess
import sys
from pathlib import Path

import numpy as np
from measurements import FS, db, harmonic, load, window


def crossings(signal, note, release):
    frequency = 440 * 2 ** ((note - 69) / 12)
    t = np.arange(-0.055, 0.066, 0.001)
    half = 1.5 / frequency
    amplitude = [
        harmonic(window(signal, release + s - half, release + s + half), frequency)
        for s in t
    ]
    levels = db(np.array(amplitude) / np.median(amplitude[:30]))
    # Ignore a brief dip that recovers within 3 ms.
    sustained = np.maximum.reduce([levels[:-2], levels[1:-1], levels[2:]])
    times = []
    for loss in (3, 9, 15):
        indices = np.flatnonzero((t[:-2] >= -0.006) & (sustained <= -loss))
        if not len(indices):
            raise ValueError(f"MIDI {note} release does not reach -{loss} dB")
        times.append(1000 * t[indices[0]])
    return np.diff(times)


def main():
    probe, source, output = (Path(x).resolve() for x in sys.argv[1:])
    output.mkdir(exist_ok=True, parents=True)
    recording = load(source)
    rows = []
    for note, time, use in [
        (69, 12.29, "fit"),
        (70, 12.46, "held out"),
        (69, 13.19, "held out"),
        (74, 16.655, "held out"),
    ]:
        path = output / f"sparkletop-release-{note}.wav"
        subprocess.run(
            [
                probe,
                "note",
                path,
                str(FS),
                str(note),
                "0.69",
                "0.5",
                "0",
                "0",
                "sparkletop",
                "harp",
            ],
            check=True,
        )
        hardware = crossings(recording, note, time)
        model = crossings(load(path), note, 0.5)
        for interval, ref, sim in zip(("3-9 dB", "9-15 dB"), hardware, model):
            rows.append(
                {
                    "midi": note,
                    "source_seconds": time,
                    "use": use,
                    "interval": interval,
                    "reference_ms": ref,
                    "model_ms": sim,
                    "error_ms": sim - ref,
                }
            )
    with (output / "release-errors.csv").open("w") as f:
        writer = csv.DictWriter(f, fieldnames=rows[0])
        writer.writeheader()
        writer.writerows(rows)
    for row in rows:
        print(
            f"{row['use']} {row['midi']} {row['interval']}: {row['reference_ms']:.0f} ms recorded, {row['model_ms']:.0f} ms modeled"
        )


if __name__ == "__main__":
    main()
