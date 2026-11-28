"""Runtime approximation for a fixed CP-80 panel setting. Compare with untouched SPICE bins."""

import numpy as np
from circuit import NEUTRAL, calibrate, response
from scipy.signal import lfilter


def loaded_bus(hz, upper):
    # 1 nF per pickup is an assumption. All 88 ceramic capacitances stay connected.
    ceramic = 1e-9
    mixing = 22e-9
    low = 82 * ceramic
    transfer = mixing / (mixing + low)
    total = 6 * ceramic + transfer * low
    gate = 2j * np.pi * hz * 470e3 * 100e-9
    return (
        ceramic
        / total
        * (1 if upper else transfer)
        * (1 + gate)
        / (1 + gate + 100e-9 / total)
    )


class Panel:
    def __init__(self, hz, voltage, rate):
        # Parallel first-order sections with real, stable preset poles. Fit both
        # quadratures on alternating AC sweep bins. Reject inaudibly large residues.
        self.poles = np.exp(-2 * np.pi * np.geomspace(1, rate * 0.42, 22) / rate)
        # The active RC feedback section can have a conjugate pole pair.
        self.pairs = []
        for freq in np.geomspace(4, 16000, 24):
            theta = 2 * np.pi * freq / rate
            radius = np.exp(-theta / 2)
            self.pairs.append((-2 * radius * np.cos(theta), radius**2))
        k = (np.arange(len(hz)) % 2 == 0) & (hz <= rate * 0.48)
        k_hz = np.r_[0, hz[k]]
        target = np.r_[0j, voltage[k]]
        basis = self.basis(k_hz, rate)
        weight = 1 / np.maximum(abs(target), np.max(abs(target)) * 0.007)
        # Prioritize the passband at the listening rate. Oversampling leaves a
        # transition region above it for the output decimator.
        weight *= 1 / (1 + (k_hz / 17000) ** 6)
        matrix = basis * weight[:, None]
        rhs = target * weight
        self.gains = np.linalg.lstsq(
            np.vstack([matrix.real, matrix.imag]), np.r_[rhs.real, rhs.imag], rcond=1e-9
        )[0]

    def basis(self, hz, rate):
        z = np.exp(2j * np.pi * hz[:, None] / rate)
        columns = [np.ones(len(hz)), (1 - self.poles) / (z - self.poles)]
        for a1, a2 in self.pairs:
            den = z * z + a1 * z + a2
            columns.extend([(1 + a1 + a2) / den, (1 - a2) * (z - 1) / den])
        return np.column_stack(columns)

    def apply(self, x):
        out = self.gains[0] * x
        for p, g in zip(self.poles, self.gains[1:]):
            out += lfilter([0, g * (1 - p)], [1, -p], x)
        pair_gains = self.gains[1 + len(self.poles) :].reshape(-1, 2)
        for (a1, a2), (low, band) in zip(self.pairs, pair_gains):
            b1 = band * (1 - a2)
            out += lfilter([0, b1, low * (1 + a1 + a2) - b1], [1, a1, a2], x)
        return out


def jack(settings, rate):
    hz, v = response(**settings)
    return Panel(hz, v[:, 1] * loaded_bus(hz, False), rate)


if __name__ == "__main__":
    calibrated = calibrate()
    worst = 0.0
    phase = 0.0
    for controls in [
        NEUTRAL,
        {"bass": 1, "mid": 1, "treble": 1},
        {"bass": 0.02, "mid": 0.05, "treble": 0.1},
    ]:
        for br in [0, 1, 2]:
            settings = dict(**controls, **calibrated, br=br)
            hz, v = response(**settings)
            for key in [22, 83]:
                _, physical = response(pickup=key, **settings)
                for rate in [44100, 48000, 96000]:
                    panel = Panel(hz, v[:, 1] * loaded_bus(hz, key >= 83), rate * 4)
                    k = (np.arange(len(hz)) % 2 == 1) & (hz >= 40) & (hz <= 10000)
                    ratio = (panel.basis(hz[k], rate * 4) @ panel.gains) / physical[
                        k, 1
                    ]
                    worst = max(worst, max(abs(20 * np.log10(abs(ratio)))))
                    phase = max(phase, max(abs(np.angle(ratio, deg=True))))
    print(f"54 sweeps, worst magnitude {worst:.4f} dB, phase {phase:.3f} degrees")
    if worst > 0.25:
        raise SystemExit("exceeded 0.25 dB")
