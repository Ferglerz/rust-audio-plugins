#!/usr/bin/env python3
"""Regenerate the small golden fixture using the unmodified MIT NARA-WPE reference.

Reference: fgnt/nara_wpe @ 5fbfc3639274e5a70dffcf08e20615a38c372ebb
Install that checkout in a temporary Python environment with NumPy and click.
No Python package or reference implementation is shipped in the plugin.
"""
from pathlib import Path
import numpy as np
from nara_wpe.wpe import online_wpe_step

TAPS, DELAY, ALPHA, BINS, FRAMES = 10, 3, 0.99, 4, 240
rng = np.random.default_rng(591)
x = (rng.standard_normal((FRAMES, BINS)) + 1j * rng.standard_normal((FRAMES, BINS)))
x *= np.array([1.0, 0.1, 0.001, 1e-7])[None, :]
for t in range(4, FRAMES):
    x[t] += 0.65 * x[t - 4]
history = np.zeros((TAPS + DELAY + 1, BINS, 1), dtype=np.complex128)
p = np.tile(np.eye(TAPS, dtype=np.complex128), (BINS, 1, 1))
g = np.zeros((BINS, TAPS, 1), dtype=np.complex128)
result = np.empty((FRAMES, BINS, 4), dtype='<f8')
for t, current in enumerate(x):
    history[:-1] = history[1:]
    history[-1, :, 0] = current
    power = np.mean(np.abs(history[:, :, 0]) ** 2, axis=0)
    output, p, g = online_wpe_step(history, power, p, g, ALPHA, TAPS, DELAY)
    result[t] = np.stack([current.real, current.imag, output[:, 0].real, output[:, 0].imag], axis=-1)
path = Path(__file__).resolve().parents[1] / 'testdata' / 'wpe-reference.f64'
path.write_bytes(result.tobytes())
print(f'Wrote {FRAMES} frames × {BINS} complex bins to {path}')
