"""Measure directly recorded pitches. Frequencies and decay come from unlooped audio."""

from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import find_peaks


def recording(directory, midi, layer):
    (path,) = Path(directory).glob(f"{midi:03}-*-{layer}.flac")
    return sf.read(path)


def spectrum(x, rate):
    nfft = 1 << (len(x) * 8 - 1).bit_length()
    mag = np.abs(np.fft.rfft(x * np.hanning(len(x)), n=nfft))
    return np.fft.rfftfreq(nfft, 1 / rate), mag * (4 / len(x))


def partials(x, rate, midi, start=0.10, end=0.47, count=12):
    hz, amp = spectrum(x[int(start * rate) : int(end * rate)], rate)
    f0 = 440 * 2 ** ((midi - 69) / 12)
    peaks, _ = find_peaks(amp)
    measured = []
    for n in range(1, count + 1):
        expected = n * f0
        region = peaks[
            (hz[peaks] > expected - 0.27 * f0) & (hz[peaks] < expected + 0.47 * f0)
        ]
        i = region[np.argmax(amp[region])]
        left, center, right = np.log(np.maximum(amp[i - 1 : i + 2], 1e-20))
        offset = 0.5 * (left - right) / (left - 2 * center + right)
        measured.append([hz[i] + offset * (hz[1] - hz[0]), amp[i]])
    return np.asarray(measured).T


def slope(x, rate, hz, times, width=0.15):
    levels = []
    for t in times:
        y = x[round((t - width / 2) * rate) : round((t + width / 2) * rate)]
        w = np.hanning(len(y))
        v = (
            2
            * abs((y * w) @ np.exp(-2j * np.pi * hz * np.arange(len(y)) / rate))
            / w.sum()
        )
        levels.append(20 * np.log10(max(v, 1e-12)))
    return np.polyfit(times, levels, 1)[0]


if __name__ == "__main__":
    import sys

    for midi in [42, 50, 63]:
        for layer in ["PP", "FF"]:
            x, rate = recording(sys.argv[1], midi, layer)
            freq, amp = partials(x, rate, midi)
            n = np.arange(1, len(freq) + 1)
            b, a = np.polyfit(n[:8] ** 2, (freq[:8] / n[:8]) ** 2, 1)
            print(
                midi,
                layer,
                "f1",
                round(freq[0], 2),
                "B",
                round(b / a, 6),
                "levels",
                np.round(20 * np.log10(amp / amp[0]), 1),
                "loss",
                np.round(
                    [slope(x, rate, h, [0.15, 0.4, 0.65, 0.9]) for h in freq[:6]], 1
                ),
            )
