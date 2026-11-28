"""Compare the live engine against completely withheld native pitches and the offline solver."""

import argparse
import csv
import subprocess
import tempfile
from pathlib import Path

import numpy as np
import soundfile as sf
from circuit import NEUTRAL, calibrate, response
from compare import LAYERS, RATE
from measure import partials
from panel import Panel, loaded_bus
from scipy.signal import resample_poly

# Each listed pitch was excluded from the dispersion, loss and hammer fits.
HELD_OUT = {34, 35, 46, 53, 60, 65, 68, 76, 88}


def live(probe, midi, velocity, rate, path, hold=0.55):
    subprocess.run(
        [probe, "play", *map(str, (midi, velocity, rate, hold)), path], check=True
    )
    signal, fs = sf.read(path)
    if (
        fs != rate
        or not np.isfinite(signal).all()
        or not np.array_equal(signal[:, 0], signal[:, 1])
    ):
        raise ValueError(f"invalid live stereo output at {rate} Hz for MIDI {midi}")
    return signal[:, 0]


def table(output, filename, rows):
    with (output / filename).open("w", newline="") as file:
        w = csv.DictWriter(file, fieldnames=rows[0])
        w.writeheader()
        w.writerows(rows)


def compare_keys(args, path, pitches, filename):
    rows = []
    for sample in sorted(args.samples.glob("*.flac")):
        midi = int(sample.name[:3])
        if midi not in pitches:
            continue
        layer = sample.stem.split("-")[-1]
        ref, fs = sf.read(sample)
        assert fs == RATE
        model = live(args.probe, midi, LAYERS[layer] / 127, RATE, path)
        count = min(6, int(15500 / (440 * 2 ** ((midi - 69) / 12))))
        rf, ra = partials(ref, RATE, midi, 0.065, 0.24, count)
        mf, ma = partials(model, RATE, midi, 0.065, 0.24, count)
        eligible = ra[1:] / ra[0] > 0.01
        errors = 20 * np.log10((ma[1:] / ma[0]) / (ra[1:] / ra[0]))
        rows.append(
            {
                "midi": midi,
                "layer": layer,
                "partials_used": int(sum(eligible)),
                "median_partial_db": float(np.median(abs(errors[eligible])))
                if eligible.any()
                else None,
                "fundamental_cents": 1200 * np.log2(mf[0] / rf[0]),
            }
        )
    table(args.output, filename, rows)
    valid = [r for r in rows if r["median_partial_db"] is not None]
    worst = max(valid, key=lambda r: r["median_partial_db"])
    print(
        f"{filename}: {len(rows)} strikes on {len(pitches)} pitches, median relative partial error "
        f"{np.median([r['median_partial_db'] for r in valid]):.2f} dB; worst {worst}. "
        f"Median absolute fundamental error {np.median([abs(r['fundamental_cents']) for r in rows]):.2f} cents."
    )


def compare_solver(args, path):
    # Independent 384 kHz double-precision collision, linear jack and anti-alias resampling.
    hi = 96000
    hz, v = response(**NEUTRAL, **calibrate())
    panels = [
        Panel(hz, v[:, 1] * loaded_bus(hz, upper), 4 * hi) for upper in [False, True]
    ]
    rows = []
    for midi in [42, 63, 88, 103, 108]:
        subprocess.run([args.probe, *map(str, (midi, 1.0, hi, 0.28)), path], check=True)
        force, fs = sf.read(path)
        reference = 0.14 * panels[int(midi >= 103)].apply(force)
        for rate in (44100, 48000, 96000):
            model = live(args.probe, midi, 1.0, rate, path, 0.28)
            common = np.gcd(fs, rate)
            ref = resample_poly(reference, rate // common, fs // common)
            count = min(6, int(16000 / (440 * 2 ** ((midi - 69) / 12))))
            rf, ra = partials(ref, rate, midi, 0.02, 0.18, count)
            mf, ma = partials(model, rate, midi, 0.02, 0.18, count)
            eligible = ra / ra[0] > 0.01
            rows.append(
                {
                    "midi": midi,
                    "rate": rate,
                    "max_partial_db": max(
                        abs(20 * np.log10(ma[eligible] / ra[eligible]))
                    ),
                    "fundamental_cents": 1200 * np.log2(mf[0] / rf[0]),
                }
            )
    table(args.output, "reference-engine.csv", rows)
    worst = max(rows, key=lambda r: r["max_partial_db"])
    print(
        f"Live vs 384 kHz reference, median of per-strike worst partial errors "
        f"{np.median([r['max_partial_db'] for r in rows]):.2f} dB, worst {worst}"
    )


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("probe", type=Path)
    p.add_argument("samples", type=Path)
    p.add_argument("output", type=Path)
    args = p.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp, "probe.wav")
        compare_keys(args, path, HELD_OUT, "withheld-keys.csv")
        compare_keys(args, path, {82}, "bb5-layers.csv")
        compare_solver(args, path)
