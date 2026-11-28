"""Fit Late Mark I pickup height and gap while preserving its touch response."""

from pathlib import Path

import numpy as np
from fit_di import field_table, jack_response
from measurements import db, load, metrics
from scipy.optimize import least_squares

# Alternate pitches in the harp archive. Use only its outer layers for fitting.
HARP_TRAINING = (29, 40, 50, 59, 65, 76, 86, 96)
REGISTER_KNOTS = (40, 60, 88)
ANGLE = np.arange(256) * (2 * np.pi / 256)
SIN, COS = np.sin(ANGLE), np.cos(ANGLE)
TIMES = np.linspace(0.09, 0.30, 11)[1:-1]
WINDOW = np.hanning(11)[1:-1]


def octave_voicing(keys, fits, matt, learman, velocities):
    # Same grid and piecewise-linear gradient as the running instrument.
    radius = np.arange(385) / 16
    gaps = 0.5 + np.arange(65) / 16
    gradient = field_table(gaps, radius).values

    def pole(gap):
        position = (np.clip(gap, gaps[0], gaps[-1]) - gaps[0]) * 16
        i = min(int(position), len(gaps) - 2)
        return gradient[i] + (position - i) * (gradient[i + 1] - gradient[i])

    def harmonics(height, gap, amplitude):
        # The free fundamental dominates the 90–300 ms scoring window. Sum the
        # complex amplitudes before measuring dB so a polarity reversal cancels.
        x = height + amplitude[:, :, None] * SIN
        voltage = (
            np.interp(abs(x), radius, pole(gap))
            * np.sign(x)
            * amplitude[:, :, None]
            * COS
        )
        partials = np.fft.rfft(voltage, axis=-1)[:, :, 1:4]
        return db(abs(np.einsum("vtb,t->vb", partials, WINDOW)) / WINDOW.sum())

    def gap_sensitivity(height, gap, swings):
        # Preserve the existing effect of closing the gap on attack peak range.
        position = abs(height + swings[:, None] * SIN)
        speed = swings[:, None] * COS

        def growth(at_gap):
            peaks = np.max(
                abs(np.interp(position, radius, pole(at_gap)) * speed), axis=1
            )
            return db(peaks[-1] / peaks[0])

        return growth(gap - 0.75) - growth(gap + 0.75)

    swings = keys[:, 4:8]
    amplitudes = swings[:, :, None] * np.exp(-keys[:, 1, None, None] * TIMES)
    stock = np.array([harmonics(*row[2:4], a) for row, a in zip(keys, amplitudes)])

    def level_errors(note, paths, transfer):
        i = note - 28
        # Faraday voltage scales with tine speed and with the parallel pickup
        # group. The common field/gain factor cancels against E2 on each piano.
        weight = 0.25 if i < 4 else 1 / 3
        voltage = stock[i, [0, -1], :2] + db(fits["f0"][i] * weight) + db(abs(transfer))
        errors = []
        for path, (h1, h2) in zip(paths, voltage):
            if path is not None:
                recorded = metrics(load(path), note)
                balance = recorded["h2_h1_db"]
                errors.append([balance - (h2 - h1), recorded["h2_dbfs"] - balance - h1])
        return np.array(errors)

    centers = []
    growth = {}
    directory = Path(learman) / "jRhodes3d-mono"
    for note in HARP_TRAINING:
        soft = next(directory.glob(f"*_{note:03}_*_5.flac"))
        hard = next(directory.glob(f"*_{note:03}_*_1.flac"), None)
        differences = level_errors(note, (soft, hard), 1.0)
        centers.append(differences.mean(axis=0))
        if len(differences) == 2:
            # Allow a 1 dB growth adjustment from the second piano's nominal layers.
            growth[note] = np.clip(differences[1, 0] - differences[0, 0], -1, 1)

    # The two dry keys already anchor the instrument's swing and electrical
    # gain. Use their measured endpoints directly. The second piano shapes
    # the trend between them, with one observation per available pitch.
    dry_levels = []
    for note in (40, 88):
        i = note - 28
        paths = [
            Path(matt) / "examples" / f"{fits['notes'][i]}-{v}.wav" for v in ("p", "f")
        ]
        response = jack_response(fits["f0"][i] * np.array([1, 2]))
        soft, hard = level_errors(note, paths, response)
        dry_levels.append((soft + hard) / 2)
        growth[note] = hard[0] - soft[0]

    centers, dry_levels = np.array(centers), np.array(dry_levels)
    centers[:, 1] -= centers[HARP_TRAINING.index(40), 1]
    dry_levels[:, 1] -= dry_levels[0, 1]

    def trend(pitches, middle):
        return np.array(
            [
                np.interp(pitches, REGISTER_KNOTS, knots)
                for knots in np.vstack([dry_levels[0], middle, dry_levels[1]]).T
            ]
        ).T

    middle = least_squares(
        lambda x: (trend(HARP_TRAINING, x) - centers).ravel(),
        [0.0, 0.0],
        loss="soft_l1",
        f_scale=3,
    ).x
    position = (velocities - velocities[0]) / (velocities[-1] - velocities[0]) - 0.5

    def power(levels):
        return 10 * np.log10(np.sum(10 ** (levels / 10), axis=1))

    def fit_key(i):
        note = i + 28
        original = np.r_[keys[i, 2:4], 0.0]
        base = stock[i]
        endpoints = swings[i, [0, -1]]
        # G3 is the editor's reference for pickup travel. Other keys can lie
        # on either side of a harmonic null as the player adjusts the gap.
        sensitivity = gap_sensitivity(*original[:2], endpoints) if note == 55 else None
        # Equal endpoint adjustments in opposite log directions preserve the
        # geometric mean swing and the order of all four measured strengths.
        fraction = (
            np.log(swings[i] / endpoints[0]) / np.log(endpoints[1] / endpoints[0]) - 0.5
        )
        # Both endpoints get the same shift on pitches outside the fit.
        balance, fundamental = trend(note, middle)
        target = balance + growth.get(note, 0.0) * position

        def residual(setup, growth_weight):
            height, gap, tilt = setup
            scale = np.exp(tilt * fraction)
            levels = harmonics(height, gap, amplitudes[i] * scale[:, None])
            delta = levels - base
            octave = delta[:, 1] - delta[:, 0] - target
            travel = (
                min(
                    0,
                    gap_sensitivity(height, gap, endpoints * scale[[0, -1]])
                    - sensitivity
                    + 0.12,
                )
                if sensitivity is not None
                else 0
            )
            return np.r_[
                octave[[0, -1]],
                0.7 * octave[1:3],
                growth_weight * (octave[-1] - octave[0]),
                2 * (delta[:, 0] - fundamental),
                0.5 * (power(levels) - power(base)),
                0.25 * np.clip(delta[:, 2] - delta[:, 0], -24, 24),
                2 * (setup[:2] - original[:2]),
                12 * tilt,
                120 * travel,
            ]

        # Harmonic nulls split the search into distinct basins. Start on both
        # sides of the nulls, keep travel within 0.7 mm of the existing setup.
        bounds = (
            np.maximum(original - 0.7, [0.12, 0.508, -0.24]),
            np.minimum(original + 0.7, [4, 3.175, 0.24]),
        )
        grid = [
            np.array([h, g, 0.0])
            for h in np.linspace(bounds[0][0], bounds[1][0], 17)
            for g in np.linspace(bounds[0][1], bounds[1][1], 7)
        ]
        starts = []
        for seed in sorted(grid, key=lambda x: np.linalg.norm(residual(x, 2))):
            if all(np.linalg.norm(seed - other) > 0.25 for other in starts):
                starts.append(seed)
            if len(starts) == 4:
                break
        starts.append(original)

        # Find a basin by level first. Then hold the endpoint growth tightly
        # while adjusting within that basin.
        def solve(seed, weight):
            return least_squares(
                residual, seed, args=(weight,), bounds=bounds, max_nfev=70
            )

        solutions = [solve(solve(seed, 2).x, 32) for seed in starts]
        height, gap, tilt = min(solutions, key=lambda s: s.cost).x
        return np.r_[height, gap, swings[i] * np.exp(tilt * fraction)]

    return np.array([fit_key(i) for i in range(len(keys))])
