"""Rank shared physical scale candidates by the A5 and D6 room spectral ratios."""
import argparse

import numpy as np

TRAIN = {81: [-31.6, -32.9], 86: [-26.3, -37.9]}
HELD_OUT = {83: [-26.3, -38.6]}


def fit(rows, note, distance, limit, width):
    length = .69 * (.048 / .69) ** ((note - 49) / 59)
    h = np.arange(1, 4)

    def weights(fraction, aperture):
        return abs(np.sin(np.pi * h * fraction) * np.sinc(h * aperture / (2 * length)))

    old = weights(.12 + .07 * (note - 21) / 87, min(.01, .025 * length))
    new = weights(min(limit, distance / length), width)
    relative = new / old
    return rows[rows[:, 0] == note, 4:] + 20 * np.log10(relative[1:] / relative[0])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('csv', help='stdout from the contact_probe example')
    args = parser.parse_args()
    data = np.loadtxt(args.csv, delimiter=',', skiprows=1)
    best = None
    for mass in [.45, .7, 1., 1.4, 1.8]:
        for stiffness in [.25, .5, 1., 2., 4., 8.]:
            rows = data[(data[:, 2] == mass) & (data[:, 3] == stiffness)]
            for distance_m in np.arange(.04, .101, .005):
                for limit in [.35, .40, .45, .47]:
                    for width_m in [.010, .020, .030]:
                        p = (distance_m, limit, width_m)
                        difference = np.concatenate([fit(rows, n, *p) - v for n, v in TRAIN.items()])
                        error = np.mean(abs(difference))
                        if best is None or error < best[0]:
                            best = (error, mass, stiffness, p, rows)
    error, mass, stiffness, p, rows = best
    held = np.concatenate([fit(rows, n, *p) - v for n, v in HELD_OUT.items()])
    print(f'mass {mass:g}×, stiffness {stiffness:g}×, {p[0]*1000:.0f} mm center, {p[1]:.2f}L limit, {p[2]*1000:.0f} mm aperture')
    print(f'A5 + D6 mean absolute H2/H3 error {error:.2f} dB; held-out B5 {np.mean(abs(held)):.2f} dB')


if __name__ == '__main__':
    main()
