"""Evaluate Stage 73 against two public instruments. Write local A/B audio and metrics.

Arguments: release probe, Stølberg checkout, jRhodes3d checkout, output directory.
Dependencies: numpy, scipy, soundfile. Recordings are development inputs.
"""

import csv
import subprocess
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from fit_octave import HARP_TRAINING
from measurements import FS, db, load, metrics, rms, spectrum, window
from scipy.signal import resample_poly

probe, matt, jeff, output = [Path(x).resolve() for x in sys.argv[1:]]
output.mkdir(parents=True, exist_ok=True)
notes = {}


def render(n, v, rate=FS, held=3.25, height=0.0, gap=0.0, tap="jack"):
    key = (n, v, rate, held, height, gap, tap)
    if key not in notes:
        path = output / ("model_" + "_".join(map(str, key)) + ".wav")
        subprocess.run(
            [
                probe,
                "note",
                str(path),
                str(rate),
                str(n),
                str(v),
                str(held),
                str(height),
                str(gap),
                "late_i",
                *(["harp"] if tap == "harp" else []),
            ],
            check=True,
        )
        notes[key] = sf.read(path)[0]
    return notes[key]


sources = {"Matt": {}, "Learman": {}}
for n, label in [(40, "E1"), (70, "A#3"), (88, "E5")]:
    for dyn, v in [("p", 0.37075), ("f", 1.0)]:
        sources["Matt"][n, dyn] = (
            load(matt / "examples" / f"{label}-{dyn}.wav"),
            render(n, v),
        )
# The SFZ maps layer 5 to the softest strikes. Layer 1 is the hardest.
for path in sorted((jeff / "jRhodes3d-mono").glob("*.flac")):
    n = int(path.name.split("_")[1])
    layer = int(path.stem.split("_")[-1])
    # The two intermediate layers are reserved for a separate evaluation.
    # Use the centres of their SFZ velocity ranges, as actual speeds are unknown.
    dyn, v = {
        1: ("f", 1.0),
        2: ("layer2", 104 / 127),
        3: ("mf", 0.69),
        4: ("layer4", 60 / 127),
        5: ("p", 0.37075),
    }[layer]
    sources["Learman"][n, dyn] = (load(path), render(n, v, tap="harp"))
rows = []
dynamics = []
gains = {}
for name, pairs in sources.items():
    # Level-align both dynamics together using only E2. No per-note normalization.
    gains[name] = np.exp(
        np.mean(
            [
                np.log(
                    rms(window(pairs[40, d][0], 0.02, 0.25))
                    / rms(window(pairs[40, d][1], 0.02, 0.25))
                )
                for d in ["p", "f"]
            ]
        )
    )
    by_strike = {}
    for (n, d), (reference, model) in pairs.items():
        a, b = metrics(reference, n), metrics(gains[name] * model, n)
        by_strike[n, d] = (a, b)
        rows.append(
            dict(
                source=name,
                tap="harp" if name == "Learman" else "jack",
                midi=n,
                dynamic=d,
                **{key: round(b[key] - a[key], 3) for key in a},
                reference_bell_hz=a["bell_peak_hz"],
                model_bell_hz=b["bell_peak_hz"],
            )
        )
    for n, soft in pairs:
        if soft != "p" or (n, "f") not in pairs:
            continue
        for key in ("attack_rms_db", "h2_h1_db", "h3_h1_db", "bell_peak_h1_db"):
            growth = [
                hard[key] - quiet[key]
                for hard, quiet in zip(by_strike[n, "f"], by_strike[n, "p"])
            ]
            dynamics.append(
                {
                    "source": name,
                    "midi": n,
                    "metric": key,
                    "reference": growth[0],
                    "model": growth[1],
                    "error": growth[1] - growth[0],
                }
            )
with (output / "audio-errors.csv").open("w") as file:
    writer = csv.DictWriter(file, fieldnames=rows[0].keys())
    writer.writeheader()
    writer.writerows(rows)
with (output / "dynamic-errors.csv").open("w") as file:
    writer = csv.DictWriter(file, fieldnames=dynamics[0].keys())
    writer.writeheader()
    writer.writerows(dynamics)
summary = []
for name in sources:
    selected = [
        r for r in rows if r["source"] == name and r["dynamic"] in ("p", "mf", "f")
    ]
    summary.append(
        f"{name}: {len(selected)} note/dynamic pairs; global comparison gain {db(gains[name]):+.2f} dB"
    )
    for key in [
        "attack_rms_db",
        "h2_dbfs",
        "h2_h1_db",
        "h3_h1_db",
        "fundamental_decay_db_s",
        "bell_peak_h1_db",
    ]:
        errors = [abs(r[key]) for r in selected if np.isfinite(r[key])]
        summary.append(
            f"  median absolute {key} error {np.median(errors) if errors else np.nan:.2f} ({len(errors)}/{len(selected)})"
        )
    for key in ("h2_h1_db", "h3_h1_db"):
        errors = [
            abs(r["error"])
            for r in dynamics
            if r["source"] == name and r["metric"] == key
        ]
        summary.append(
            f"  soft-to-hard {key} median absolute error {np.median(errors):.2f} ({len(errors)} keys)"
        )
    intermediate = [
        r for r in rows if r["source"] == name and r["dynamic"] in ("layer2", "layer4")
    ]
    if intermediate:
        for key in ("attack_rms_db", "h2_dbfs", "h2_h1_db", "h3_h1_db"):
            errors = [abs(r[key]) for r in intermediate]
            summary.append(
                f"  intermediate {key} median absolute error {np.median(errors):.2f} ({len(errors)} strikes)"
            )
held_out = [
    r
    for r in rows
    if r["source"] == "Learman"
    and r["midi"] not in HARP_TRAINING
    and r["dynamic"] in ("p", "mf", "f")
]
for key in ("h2_dbfs", "h2_h1_db"):
    errors = [abs(r[key]) for r in held_out]
    summary.append(
        f"Learman pitches outside the fit: median absolute {key} error {np.median(errors):.2f} ({len(errors)} strikes)"
    )
growth = [
    abs(r["error"])
    for r in dynamics
    if r["source"] == "Learman"
    and r["midi"] not in HARP_TRAINING
    and r["metric"] == "h2_h1_db"
]
summary.append(
    f"Learman pitches outside the fit: soft-to-hard octave growth error {np.median(growth):.2f} ({len(growth)} pairs)"
)

# Original then model, soft then hard at each pitch. Keep the instrument-level gain.
parts = []
for n, name in [(40, "Matt"), (55, "Learman"), (88, "Matt")]:
    for d in ["p", "f"]:
        a, b = sources[name][n, d]
        for piece in [a, b * gains[name]]:
            clip = piece[: int(1.15 * FS)].copy()
            clip[-960:] *= np.linspace(1, 0, 960)
            parts.extend([clip, np.zeros(round(0.15 * FS))])
audio = np.concatenate(parts)
sf.write(
    output / "three-note-comparison.wav",
    audio * (0.8 / max(abs(audio))),
    FS,
    subtype="PCM_24",
)

# Check the hard treble magnitude spectrum against 4x the production internal rate.
for fs in [44100, 48000, 96000]:
    primary = render(100, 1, fs, 0.65, 0, -1)
    higher = resample_poly(render(100, 1, fs * 4, 0.65, 0, -1), 1, 4)
    start, stop = round(0.015 * fs), round(0.20 * fs)

    a, b = spectrum(primary[start:stop]), spectrum(higher[start:stop])
    error = db(np.linalg.norm(a - b) / np.linalg.norm(b))
    summary.append(
        f"{fs} Hz hard E7: spectral difference versus oversampled reference {error:.1f} dB"
    )
    if error > -30:
        raise SystemExit(summary[-1])

# Fixed strike, manufacturer-style adjustments. G3 is held out of the per-key table fit.
height, _ = map(
    float, subprocess.check_output([probe, "geometry", "late_i", "55"]).split()
)
height_harmonics = []
for offset in [-height, 0.0]:
    signal = render(55, 0.60, height=offset)
    h = metrics(signal, 55)["h2_h1_db"]
    height_harmonics.append(h)
    summary.append(
        f"G3 height offset {offset:+.2f} mm: second harmonic relative to fundamental {h:+.1f} dB"
    )
if not height_harmonics[0] > 0 > height_harmonics[1]:
    raise SystemExit("Centering the G3 tine should make its octave dominant")
gap_levels = []
for gap in [-0.75, 0.75]:
    peak = [
        np.max(abs(window(render(55, vel, gap=gap), 0, 0.2))) for vel in [0.37075, 1.0]
    ]
    gap_levels.append(peak)
    summary.append(
        f"G3 gap offset {gap:+.2f} mm: soft-to-hard attack peak range {db(peak[1] / peak[0]):.2f} dB"
    )
close, wide = gap_levels
if not (close[0] > wide[0] and close[1] / close[0] > wide[1] / wide[0]):
    raise SystemExit(
        "Closing the G3 pickup gap should increase output and dynamic range"
    )
voicings = [
    render(55, 0.6, held=1.0, height=h, gap=g)
    for h, g in [(0, 0), (-height, 0), (0, -0.75), (0, 0.75)]
]
sweep = np.concatenate(voicings)
sf.write(output / "g3-voicing.wav", sweep * 0.8 / max(abs(sweep)), FS, subtype="PCM_24")
# Release measurements characterize the model when an isolated hardware release isn't available.
for n in [40, 55, 88]:
    x = render(n, 0.69, held=0.5)
    envelope = [
        rms(window(x, 0.5 + t / 1000, 0.51 + t / 1000)) for t in range(0, 350, 5)
    ]
    t20 = next((5 * i for i, r in enumerate(envelope) if r <= envelope[0] * 0.1), None)
    summary.append(f"Model MIDI {n} release: {t20} ms to -20 dB")
text = "\n".join(summary) + "\n"
(output / "audio-summary.txt").write_text(text)
print(text)
