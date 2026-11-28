"""Render all six instruments at fixed MIDI, and compare matching signal taps.
Usage: python3 era_audio.py PROBE OUTPUT [REFERENCE_DIRECTORY]
See eras.md for the optional reference filenames and their provenance.
"""

import csv
import subprocess
import sys
from functools import lru_cache
from pathlib import Path

import numpy as np
import soundfile as sf
from measurements import FS, db, harmonic, load, metrics, rms, window

probe, output = (Path(arg).resolve() for arg in sys.argv[1:3])
output.mkdir(exist_ok=True, parents=True)
eras = ("sparkletop", "early_i", "mid_i", "late_i", "mark_ii", "mark_v")
velocities = (0.37075, 0.69, 1.0)
held = 0.85


@lru_cache(maxsize=32)
def strike(era, note, v, tap="jack", height=0):
    path = output / "strikes" / f"{era}-{note}-{v}-{tap}-{height}.wav"
    path.parent.mkdir(exist_ok=True)
    args = [
        probe,
        "note",
        path,
        str(FS),
        str(note),
        str(v),
        str(held),
        str(height),
        "0",
        era,
    ]
    if tap == "harp":
        args.append(tap)
    subprocess.run(args, check=True)
    return sf.read(path)[0]


def write_rows(name, rows):
    with (output / name).open("w") as f:
        writer = csv.DictWriter(f, fieldnames=rows[0])
        writer.writeheader()
        writer.writerows(rows)


def release_ms(x):
    hop = round(FS * 0.005)
    tail = x[round(FS * held) :]
    blocks = tail[: len(tail) // hop * hop].reshape(-1, hop)
    levels = np.sqrt(np.mean(blocks**2, axis=1))
    ref = rms(window(x, held - 0.06, held))
    # Time after which all following 5ms windows stay 20 dB below the held note.
    above = np.flatnonzero(levels > ref / 10)
    return (1 + above[-1]) * 5 if len(above) else 0


def render_comparisons():
    rows = []
    parts = []
    for era in eras:
        notes = []
        for n in (40, 55, 88):
            for v in velocities:
                x = strike(era, n, v)
                notes.append(np.r_[x, np.zeros(round(FS * 0.08))])
                measurement = metrics(x, n)
                rows.append(
                    dict(
                        era=era,
                        midi=n,
                        velocity=v,
                        release_20db_ms=release_ms(x),
                        **{k: round(a, 3) for k, a in measurement.items()},
                    )
                )
        parts.append(np.concatenate(notes))
    # One shared export gain protects PCM playback. Matched version uses a
    # single RMS gain per nine-strike take, preserving within-era dynamics.
    original = np.concatenate(parts)
    matched = np.concatenate([part / rms(part) for part in parts])
    for suffix, audio in (("fixed-gain", original), ("level-matched", matched)):
        sf.write(
            output / f"eras-{suffix}.wav",
            audio * (0.9 / max(abs(audio))),
            FS,
            subtype="PCM_24",
        )
    write_rows("era-strikes.csv", rows)


def reference_comparisons(refs):
    rows = []
    spark = load(refs / "sparkle4.mp3")
    # Upper triads of Grall's first and third chords, with no played octave
    # doubling of these notes. Compare settled partials, not polyphonic RMS.
    for start, notes in ((0.29, (62, 67, 71)), (1.46, (60, 65, 69))):
        y = window(spark, start, start + 0.18)
        for n in notes:
            f = 440 * 2 ** ((n - 69) / 12)
            reference = db(harmonic(y, f, 2) / harmonic(y, f))
            for v in velocities:
                x = window(strike("sparkletop", n, v, "harp"), 0.13, 0.31)
                model = db(harmonic(x, f, 2) / harmonic(x, f))
                rows.append(
                    {
                        "era": "sparkletop",
                        "source": "Grall sparkle4",
                        "midi": n,
                        "velocity": v,
                        "metric": "h2_h1_db",
                        "reference": reference,
                        "model": model,
                        "error": model - reference,
                    }
                )
    # Confirmed from recording pitches; file labels use a different octave
    # convention. No MIDI velocities or plastic-key identification supplied.
    anchors = ((65725, 52), (65726, 64), (65659, 88), (65660, 100))
    neighbors = ((65717, 49), (65723, 50), (65721, 51), (65728, 53),
                 (65727, 54), (65730, 55), (65729, 56), (65657, 86),
                 (65655, 87), (65662, 89), (65661, 90), (65664, 91), (65663, 92))
    available = [(asset, n) for asset, n in neighbors if (refs / f"{asset}.mp3").exists()]
    for asset, n in (*anchors, *available):
        x = load(refs / f"{asset}.mp3")
        start = np.flatnonzero(abs(x) > 0.015 * max(abs(x)))[0]
        obs = metrics(x[start:], n)
        for v in velocities:
            sim = metrics(strike("mark_ii", n, v, "harp"), n)
            for k in (
                "h2_h1_db",
                "h3_h1_db",
                "tail_h2_h1_db",
                "tail_h3_h1_db",
                "fundamental_decay_db_s",
                "bell_h1_db",
                "bell_peak_hz",
                "bell_peak_h1_db",
            ):
                # The inharmonic partial lies above the model's bandwidth on E7.
                if k.startswith("bell") and 440 * 2 ** ((n - 69) / 12) * 7.239 >= 16000:
                    continue
                rows.append(
                    {
                        "era": "mark_ii",
                        "source": f"Kahn {asset}",
                        "midi": n,
                        "velocity": v,
                        "metric": k,
                        "reference": obs[k],
                        "model": sim[k],
                        "error": sim[k] - obs[k],
                    }
                )
    write_rows("hardware-errors.csv", rows)
    # Check a single height adjustment against different notes from the same
    # instrument. Recording velocities are unknown, so hold test velocity fixed.
    checks = []
    for era, training in (("sparkletop", (62, 67, 71)), ("mark_ii", (52, 88))):
        anchors = [
            r
            for r in rows
            if r["era"] == era and r["velocity"] == 0.69 and r["metric"] == "h2_h1_db"
            and (era != "mark_ii" or r["midi"] in (52, 64, 88, 100))
        ]
        for height in (-1.25, -1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5):
            error = {"training": [], "held_out": []}
            for r in anchors:
                n = r["midi"]
                signal = strike(era, n, 0.69, "harp", height)
                if era == "sparkletop":
                    y = window(signal, 0.13, 0.31)
                    f = 440 * 2 ** ((n - 69) / 12)
                    ratio = db(harmonic(y, f, 2) / harmonic(y, f))
                else:
                    ratio = metrics(signal, n)["h2_h1_db"]
                error["training" if n in training else "held_out"].append(
                    abs(ratio - r["reference"])
                )
            checks.append(
                dict(
                    era=era,
                    height_mm=height,
                    **{k: np.median(v) for k, v in error.items()},
                )
            )
    write_rows("height-check.csv", checks)


if __name__ == "__main__":
    render_comparisons()
    if len(sys.argv) > 3:
        reference_comparisons(Path(sys.argv[3]))
