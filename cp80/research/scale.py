"""Extract stiff-string dispersion and partial losses from PP and FF training roots."""

import sys

import numpy as np
from measure import partials, recording, slope

# D3 is the paired-wound comparison key. Preserve complete unused pitches per region.
ROOTS = [27, 42, 50, 57, 63, 72, 80, 85, 91, 97, 102, 107]
for midi in ROOTS:
    nominal = 440 * 2 ** ((midi - 69) / 12)
    count = min(12, int(17000 / nominal))
    dispersion, losses = [], []
    for layer in ("PP", "FF"):
        x, fs = recording(sys.argv[1], midi, layer)
        f, a = partials(x, fs, midi, 0.10, 0.47, count)
        n = np.arange(1, count + 1)
        usable = (a / a.max() > 0.008) & (abs(f / (n * f[0]) - 1) < 0.08) & (n <= 8)
        if usable.sum() >= 3:
            b, k = np.polyfit(n[usable] ** 2, (f[usable] / n[usable]) ** 2, 1)
            if b > 0:
                dispersion.append(b / k)
        loss = [
            -slope(x, fs, h, [0.15, 0.35, 0.55, 0.75])
            if ai / a.max() > 0.006
            else np.nan
            for h, ai in zip(f[:6], a[:6])
        ]
        losses.append(loss)
    a = np.array(losses)
    avg = np.array(
        [np.mean(k[np.isfinite(k)]) if np.isfinite(k).any() else np.nan for k in a.T]
    )
    ok = np.flatnonzero(np.isfinite(avg))
    print(
        midi,
        round(np.median(dispersion), 7) if dispersion else None,
        np.round(np.clip(np.interp(np.arange(6), ok, avg[ok]), 0.6, 50), 2),
    )
