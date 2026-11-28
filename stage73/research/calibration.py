"""Generate per-key physical inputs from Stølberg's published fits and the VV cutting chart.
Usage: python3 calibration.py RHODES_REPO Tine_Cutting_Chart_RevC16APR18.pdf JRHODES3D_REPO
The recording names
in Stølberg's archive are one octave lower than scientific pitch labels.
"""

import re
import sys
import zlib
from pathlib import Path

import numpy as np
from fit_di import refine
from fit_octave import octave_voicing
from scipy.integrate import solve_ivp

ROOT = Path(__file__).resolve().parents[1]
results = Path(sys.argv[1]) / "results"
fits = np.load(results / "step2_pickup.npz")
hammer = np.load(results / "step3_hammer.npz")
modes = np.load(results / "step4_modes.npz")
# Native MIDI 28..100 corresponds to tone bars 8..80.
columns = []
for stream in re.findall(
    rb"stream\r?\n(.*?)\r?\nendstream", Path(sys.argv[2]).read_bytes(), re.DOTALL
):
    try:
        content = zlib.decompress(stream)
    except zlib.error:
        continue
    bars = re.findall(rb"1 0 0 1 ([\d.]+) ([\d.]+) cm\s+0 0 m\s+([\d.]+) 0 l", content)
    bars = [b for b in bars if float(b[1]) > 300]  # omit logo strokes
    if len(bars) == 82:
        columns = sorted((float(x) > 300, -float(y), float(w)) for x, y, w in bars)
assert len(columns) == 82 and columns[2][2] == 432, "unexpected PDF revision or scale"
length = np.array([row[2] * 25.4 / 72 for row in columns[8:81]])
# Training/exclusion split fixed before auditory comparison. Bounds are hammer-tip transitions.
zones = [(28, 50), (51, 60), (61, 70), (71, 84), (85, 100)]
training = [
    28,
    34,
    40,
    46,
    50,
    51,
    54,
    57,
    60,
    61,
    64,
    67,
    69,
    71,
    76,
    80,
    84,
    85,
    88,
    91,
    96,
    100,
]
table = np.zeros((73, 9))
table[:, 0] = length
geometry_height = fits["p_o"] * 1000
geometry_gap = fits["p_d"] * 1000
soft_swing, hard_swing = fits["A0"][:, 0] * 1000, fits["A0"][:, -1] * 1000
# Keep the measured intermediate proportions when the two endpoint swings are
# refined at the loaded jack. Noisy layers can cross; keep the curve monotone.
fraction = np.log(fits["A0"] / fits["A0"][:, :1])
fraction = np.maximum.accumulate(np.clip(fraction / fraction[:, -1:], 0, 1), axis=1)
bass_fit, treble_fit, alignment = refine(results, fits)
# The Mark II sustain fit was evaluated with the original harp setup.
mark_ii_anchors = np.column_stack(
    (geometry_height, np.clip(geometry_gap, 1.59, 3.18), soft_swing, hard_swing)
)
mark_ii_anchors[[12, 60]] = [bass_fit, treble_fit]
mark_ii = np.zeros_like(mark_ii_anchors)
# Transfer the jack calibration across the keyboard. The same instrument-level
# gain and passive load apply to every note. Correcting only the two anchor
# keys would introduce a discontinuity in swing and voicing on adjacent keys.
for col, data in enumerate((geometry_height, geometry_gap, soft_swing, hard_swing)):
    ratio = np.array([bass_fit[col], treble_fit[col]]) / data[[12, 60]]
    data *= np.exp(np.interp(np.arange(73), [12, 60], np.log(ratio)))
geometry_gap = np.clip(geometry_gap, 0.508, 3.175)
swings = soft_swing[:, None] * (hard_swing / soft_swing)[:, None] ** fraction
# Voicing is adjusted per pickup and can cross a hammer-tip boundary. Keep one
# key in four and G3 out of this fit, including the dry B-flat 4 comparison.
notes = np.arange(28, 101)
keep = (notes % 4 != 2) & (notes != 55) & ~fits["flag"]
voicing = np.log(np.column_stack((geometry_height, geometry_gap, swings)))


def predict(note, measured, width):
    # Fit the local register trend in log units. Height, gap and swing come
    # from the same set of neighbours so large and small swings stay paired.
    offset = notes[measured] - note
    weights = np.exp(-0.25 * (offset / width) ** 2)
    basis = np.column_stack((np.ones(len(offset)), offset)) * weights[:, None]
    return np.linalg.lstsq(basis, voicing[measured] * weights[:, None], rcond=None)[0][
        0
    ]


def interpolation_error(width):
    errors = []
    for n in notes[keep]:
        estimated = predict(n, keep & (notes != n), width)
        errors.append(abs(estimated - voicing[n - 28]))
    return np.median(errors, axis=0).mean()


width = min((1, 2, 3, 4, 6, 8), key=interpolation_error)
for n in notes[~keep]:
    voicing[n - 28] = predict(n, keep, width)
table[:, 2:8] = np.exp(voicing)

# Relative elastic contact, normalized to unit approach speed and unit duration.
# Estimate one force projection per upper mode from the clear training partials.
# The fundamental fit's pulse width describes the whole attack. One tenth of
# that width for the elastic impact is a model assumption, not a measured time.
contact_fraction = 0.1
phase = np.linspace(0, 1, 257)
compression = solve_ivp(
    lambda _, y: [y[1], -18.57749636 * max(0, y[0]) ** 1.5],
    (0, 1),
    (0, 1),
    t_eval=phase,
    rtol=1e-10,
    atol=1e-12,
).y[0]
force = np.maximum(compression, 0) ** 1.5
cycles = (
    hammer["f0"][:, None]
    * hammer["tau0"][:, None]
    * contact_fraction
    * hammer["vels"] ** -hammer["beta"]
)


def response(ratio):
    return (
        abs(np.cos(2 * np.pi * ratio * cycles[:, :, None] * (phase - 0.5)) @ force)
        / force.sum()
    )


projection = []
for j, ratio in enumerate(modes["ratios_meas"][:3]):
    coupling = (modes["a_dyn"][:, :, j] * response(1) / response(ratio))[keep]
    projection.append(round(np.exp(np.nanmedian(np.log(coupling))), 8))
for lo, hi in zones:
    src = np.array(
        [n - 28 for n in training if lo <= n <= hi and not fits["flag"][n - 28]]
    )
    keys = np.arange(lo - 28, hi - 27)
    for j in range(4):
        mark_ii[keys, j] = np.exp(np.interp(keys, src, np.log(mark_ii_anchors[src, j])))
    for column, data in [(1, fits["sigma"]), (8, hammer["tau0"] * 1000 / length)]:
        table[keys, column] = np.exp(np.interp(keys, src, np.log(data[src])))
# Tim Kahn's isolated Mark II harp. Each pole is adjustable independently.
# An offset that matches one key can put its neighbor at a harmonic null.
mark_ii_pickup_trim_mm = {
    49: (0.3, 0.0), 50: (0.2, 0.4), 52: (1.0, 0.0), 53: (-0.6, -0.4),
    54: (-0.8, -0.4), 55: (-0.6, 0.0), 56: (-1.3, -0.8), 64: (1.0, 0.0),
    86: (-0.3, 0.0), 88: (-0.7, -0.4), 90: (-0.2, 0.0),
    91: (-0.8, -0.4), 92: (-0.5, 0.0),
}
for midi, trim in mark_ii_pickup_trim_mm.items():
    mark_ii[midi - 28, :2] += trim

late_voicing = octave_voicing(
    table.round(6), fits, results.parent, Path(sys.argv[3]), hammer["vels"]
)
with (ROOT / "src/calibration.rs").open("w") as out:
    out.write(
        "// Generated by research/calibration.py. Values use millimetres, seconds and milliseconds.\n"
    )
    out.write(
        "// Rows: cut length, modal decay (1/s), height, gap, p/mp/mf/f swing, contact ms/mm.\n"
    )
    out.write(f"pub(super) const JACK_GAIN: f64 = {alignment:.10f};\n")
    out.write(
        f"pub(super) const VELOCITIES: [f64; 4] = {hammer['vels'].round(8).tolist()};\n"
    )
    out.write(f"pub(super) const CONTACT_FRACTION: f64 = {contact_fraction};\n")
    out.write(
        f"pub(super) const UPPER_IMPULSE: [f64; 3] = {np.array(projection).tolist()};\n"
    )
    out.write("#[rustfmt::skip]\npub(super) const KEYS: [[f64; 9]; 73] = [\n")
    for row in table:
        out.write("    [" + ", ".join(f"{v:.6f}" for v in row) + "],\n")
    out.write("];\n")
    out.write("// Mark II harp setup: height, gap, soft swing and hard swing in mm.\n")
    out.write("#[rustfmt::skip]\npub(super) const MARK_II: [[f64; 4]; 73] = [\n")
    for row in mark_ii:
        out.write("    [" + ", ".join(f"{v:.6f}" for v in row) + "],\n")
    out.write("];\n")
    out.write("// Late Mark I resting height, gap and four tine swings (mm).\n")
    out.write("#[rustfmt::skip]\npub(super) const LATE_VOICING: [[f64; 6]; 73] = [\n")
    for row in late_voicing:
        out.write("    [" + ", ".join(f"{v:.6f}" for v in row) + "],\n")
    out.write("];\n")
