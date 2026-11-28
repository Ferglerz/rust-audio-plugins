"""Fit effective hammer parameters at PP and FF. Leave both middle layers unused."""

import argparse
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import numpy as np
from circuit import NEUTRAL, calibrate, response
from compare import PITCHES, RATE, render
from measure import partials, recording
from panel import loaded_bus
from scipy.optimize import least_squares
from scipy.signal import resample_poly

# Interpolate harmonic levels across these recorded pitches. The physical solver
# can pass through deep cancellations if its masses and contact K alone are lerped.
TREBLE = (63, 72, 80, 85, 91, 97)


def fit_bb5(probe, samples, hz, transfer):
    # Only MP and FF are native training strikes. Reserve the recording at F.
    layers = ("PP", "MP", "F", "FF")
    windows = ((0.015, 0.075), (0.065, 0.24), (0.25, 0.40))

    def levels(wave, rate, midi, panel):
        rows = []
        for start, end in windows:
            f, amp = partials(wave, rate, midi, start, end, 6)
            db = 20 * np.log10(amp * (np.interp(f, hz, transfer) if panel else 1))
            rows.append(np.r_[db[0], db[1:] - db[0]])
        return np.array(rows)

    target = np.array([
        levels(*recording(samples, 82, layer), 82, False) for layer in ("MP", "FF")
    ])
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory, "force.wav")

        def strike(midi, layer, overrides=()):
            wave = render(probe, midi, layer, 0.402, path, overrides)
            return levels(resample_poly(wave, 1, 4), RATE, midi, True)

        # Recording gain varies between FLAC layers. Keep absolute note level
        # near the two adjacent keys, including a soft strike with no native FLAC.
        neighbors = np.array([[strike(n, l)[1, 0] for l in layers] for n in (81, 83)])
        center = neighbors.mean(axis=0)

        def residual(params):
            log_k, position, grams, linear = params
            overrides = (10**log_k, position, grams / 1000, linear * 10000)
            model = np.array([strike(82, layer, overrides) for layer in layers])
            measured = model[[1, 3]]
            error = np.maximum(measured[:, 1, 1:], -45) - np.maximum(target[:, 1, 1:], -45)
            middle = np.maximum(model[2, 1, 1:], -45) - np.maximum(target.mean(axis=0)[1, 1:], -45)
            h3 = measured[:, (0, 2), 2] - target[:, (0, 2), 2]
            fundamental = model[:, 1, 0] - center
            return np.r_[(error * [1.4, 4.0, 1.25, 0.9, 0.65]).ravel(),
                         middle * [0.65, 2.4, 0.85, 0.5, 0.4],
                         h3.ravel() * 1.3, fundamental * 0.45,
                         1.25 * np.maximum(abs(fundamental) - 4, 0)]

        # Basins from a coarse bounded contact search. The linear term gives
        # the felt finite stiffness at initial compression.
        fits = [
            least_squares(residual, start, bounds=([5, 0.05, 3, 0], [9, 0.245, 15, 35]),
                          diff_step=0.0001, max_nfev=42, loss="soft_l1", f_scale=4)
            for start in [(5.45, 0.1677, 12.33, 1.797), (5.98, 0.1792, 6.03, 2.105),
                          (5.77, 0.1872, 5.57, 1.87)]
        ]
    k, position, grams, linear = min(fits, key=lambda fit: fit.cost).x
    print(f"B-flat 5 K={10**k:.7g}, strike fraction={position:.8f}, "
          f"mass={grams:.5f} g, linear stiffness={linear * 10000:.3f} N/m")
    return 82, 10**k, position, grams / 1000, linear * 10000


def fit(midi, probe, samples, hz, transfer, interpolate=False):
    if midi == 82 and not interpolate:
        return fit_bb5(probe, samples, hz, transfer)
    layers = ("PP", "FF")
    targets = []
    count = min(8, int(15000 / (440 * 2 ** ((midi - 69) / 12))))

    def target(note, layer):
        wave, fs = recording(samples, note, layer)
        _, amp = partials(wave, fs, note, 0.065, 0.24, count)
        return 20 * np.log10(amp[1:] / amp[0])

    for layer in layers:
        if interpolate:
            lo = max(n for n in TREBLE if n < midi)
            hi = min(n for n in TREBLE if n > midi)
            u = (midi - lo) / (hi - lo)
            targets.append((1 - u) * target(lo, layer) + u * target(hi, layer))
        else:
            targets.append(target(midi, layer))
    if interpolate:
        # The intermediate target comes from PP/FF. No MP recording is read.
        layers = (*layers, "MP")
        targets.append((targets[0] + targets[1]) * 0.5)
    with tempfile.TemporaryDirectory() as temp:
        path = Path(temp, "fit.wav")

        def residual(params):
            k, position, mass = params
            error = []
            for layer, target in zip(layers, targets):
                force = render(
                    probe, midi, layer, 0.26, path, (10**k, position, mass / 1000)
                )
                freq, amp = partials(
                    resample_poly(force, 1, 4), RATE, midi, 0.065, 0.24, count
                )
                amp *= np.interp(freq, hz, transfer)
                model = 20 * np.log10(amp[1:] / amp[0])
                # Deep notches are poorly identified in a short FFT and vary with voicing.
                error.extend(
                    (np.maximum(model, -42) - np.maximum(target, -42))
                    / np.sqrt(np.arange(2, count + 1) / 2)
                )
            return error

        results = []
        for start in [(7.75, 0.135, 8.0), (7.1, 0.09, 7.0), (8.25, 0.18, 8.0)]:
            results.append(
                least_squares(
                    residual,
                    start,
                    bounds=([6.0, 0.055, 3.0], [9.5, 0.24, 16.0]),
                    diff_step=0.0015,
                    loss="soft_l1",
                    f_scale=3.0,
                    max_nfev=35,
                )
            )
        best = min(results, key=lambda r: r.cost)
        print(
            f"MIDI {midi} K={10 ** best.x[0]:.5g} position={best.x[1]:.5f} mass={best.x[2]:.4f} g weighted RMS={np.mean(best.fun**2) ** 0.5:.2f} dB",
            flush=True,
        )
        return (midi, 10 ** best.x[0], best.x[1], best.x[2] / 1000)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("probe", type=Path)
    parser.add_argument("samples", type=Path)
    parser.add_argument(
        "--scale", action="store_true", help="fit additional native training roots"
    )
    parser.add_argument("--note", type=int, help="fit one native reference pitch")
    parser.add_argument(
        "--treble-table",
        type=Path,
        help="write the interpolated contact table between E-flat 4 and C-sharp 7",
    )
    args = parser.parse_args()
    hz, v = response(**calibrate(), **NEUTRAL)
    transfer = abs(v[:, 1] * loaded_bus(hz, False))
    roots = (27, 57, 72, 80, 85, 91, 97, 102, 107) if args.scale else PITCHES
    if args.note is not None:
        roots = (args.note,)
    if args.treble_table:
        roots = [
            # These keys have separately calibrated contacts in scale.rs.
            n
            for n in range(TREBLE[0] + 1, TREBLE[-1])
            if n not in TREBLE and n not in (74, 82, 95)
        ]
    with ThreadPoolExecutor(max_workers=4) as pool:
        jobs = [
            pool.submit(
                fit, n, args.probe, args.samples, hz, transfer, bool(args.treble_table)
            )
            for n in roots
        ]
        rows = sorted(job.result() for job in jobs)
    if args.treble_table:
        body = "".join(
            f"    ({n}, [{k:.7e}, {pos:.7f}, {kg:.8f}]),\n" for n, k, pos, kg in rows
        )
        args.treble_table.write_text(
            "// Generated by research/fit.py --treble-table from neighboring PP/FF spectra.\n"
            "pub(super) const CONTACT: &[(u8, [f64; 3])] = &[\n" + body + "];\n"
        )
