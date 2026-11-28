"""Refine two anchor keys against Matt Blostein's dry recordings at the loaded jack.

Fit harmonic magnitudes in sustained windows. Keep one electrical alignment for both
keys and dynamics. Only use E2 and E6 here; leave Bb4 audio and the 1977 piano out.
"""

import numpy as np
from scipy.interpolate import RegularGridInterpolator
from scipy.io import wavfile
from scipy.optimize import least_squares


def field_table(gaps=None, radius=None):
    # Same physical pole surface as the renderer, with a separate interpolation grid.
    xy = (np.arange(24) + 0.5) / 3 - 4
    x, y = np.meshgrid(xy, xy)
    keep = x * x + y * y <= 16
    x, y = x[keep], y[keep]
    bevel = np.maximum(0, abs(x) - 1.3)
    area = np.where(bevel > 0, np.sqrt(2), 1) / 9
    if radius is None:
        radius = np.arange(0, 20.01, 0.05)
    if gaps is None:
        gaps = np.arange(1.5, 3.31, 0.05)
    dx = radius[:, None] - x
    slopes = []
    for gap in gaps:
        z = gap + bevel
        d2 = dx * dx + y * y + z * z
        b = area * z / d2**1.5
        slopes.append(6 * b.sum(1) * (b * dx / d2).sum(1))
    return RegularGridInterpolator(
        (gaps, radius), slopes, bounds_error=False, fill_value=0.0
    )


def jack_response(hz):
    jw = 2j * np.pi * hz
    load = 1 / (1 / 10000 + 1 / 1e6 + jw * 300e-12)
    return load / (load + (180 + jw * 0.075) * (0.25 + 23 / 3))


def refine(results, fits):
    field = field_table()
    angles = np.arange(512) * 2 * np.pi / 512
    band = np.arange(1, 9)
    observations = []
    start = []
    for note in [12, 60]:
        freq = fits["f0"][note]
        initial = [
            fits["p_o"][note] * 1000,
            fits["p_d"][note] * 1000,
            fits["A0"][note, 0] * 1000,
            fits["A0"][note, -1] * 1000,
        ]
        start.extend(initial)
        for j, dyn in enumerate(["p", "f"]):
            fs, wave = wavfile.read(
                results.parent / "examples" / f"{fits['notes'][note]}-{dyn}.wav"
            )
            for time in [0.09, 0.26, 0.48]:
                data = wave[int((time - 0.045) * fs) : int((time + 0.045) * fs)].astype(
                    float
                )
                win = np.hanning(len(data))
                t = np.arange(len(data)) / fs
                harmonics = (
                    abs((data * win) @ np.exp(-2j * np.pi * t[:, None] * (band * freq)))
                    * 2
                    / win.sum()
                )
                # Exclude partials indistinguishable from noise in this window.
                above = harmonics > max(0.00018, harmonics.max() * 0.006)
                observations.append((note, j, time, freq, harmonics, above))

    def residual(theta):
        errors = []
        for note, j, time, hz, measured, mask in observations:
            idx = 0 if note == 12 else 4
            height, gap, soft, hard = theta[idx : idx + 4]
            swing = (soft, hard)[j] * np.exp(-fits["sigma"][note] * time)
            x = height + swing * np.sin(angles)
            gradient = field((np.full_like(x, gap), abs(x))) * np.sign(x)
            source = gradient * swing * np.cos(angles) * (2 * np.pi * hz)
            upper = np.fft.rfft(source)[1:9] * 2 / len(source)
            response = jack_response(band * hz)
            predicted = abs(upper * response) * np.exp(theta[-1]) / 3
            errors.extend(
                20 * np.log10((predicted[mask] + 1e-7) / (measured[mask] + 1e-7))
            )
        # Priors limit parameter ambiguity in recordings without known pot positions.
        errors.extend(3 * np.log(theta[:-1] / np.array(start)))
        return errors

    fit = least_squares(
        residual,
        start + [np.log(0.000025)],
        bounds=(
            [0.1, 1.59, 0.005, 0.04] * 2 + [np.log(5e-6)],
            [4, 3.18, 7, 12] * 2 + [np.log(1e-4)],
        ),
        loss="soft_l1",
        f_scale=2,
        max_nfev=120,
    )
    return fit.x[:4], fit.x[4:8], np.exp(fit.x[-1])
