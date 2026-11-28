"""Compare D6 and E7 board responses to the *same* pair of string pickup EMFs."""
import argparse
from pathlib import Path

import numpy as np
import soundfile as sf

def rms(x):
    return np.sqrt(np.mean(x * x))

def harmonics(x, fs, note):
    # Hann-weighted sinusoid amplitudes. Use the held portion of the first attack.
    t = np.arange(len(x)) / fs
    f = 440 * 2 ** ((note - 69) / 12)
    windowed = x * np.hanning(len(x))
    amps = abs(np.exp(-2j * np.pi * np.outer(np.arange(1, 11) * f, t)) @ windowed)
    return 20 * np.log10(np.maximum(amps / amps[0], 10 ** (-45 / 20)))

def compare(directory):
    listening = []
    for note in (46, 64, 78):
        def read(board, name):
            return sf.read(directory / board / str(note) / (name + ".wav"))
        for tap in ("centre", "bridge"):
            a, fs = read("d6", tap)
            b, bs = read("e7", tap)
            assert fs == bs and np.array_equal(a,b), (note, tap)
        for selector in ("CA", "CB", "DA", "DB"):
            a, fs = read("d6", "jack-" + selector)
            b, bs = read("e7", "jack-" + selector)
            assert fs == bs and a.shape == b.shape
            level = rms(b) / rms(a)
            polarity = 1 if (a @ b) >= 0 else -1
            residual = rms(a - polarity * b / level) / rms(a)
            sl = slice(round(.075 * fs), round(.225 * fs))
            change = harmonics(b[sl],fs,note) - harmonics(a[sl],fs,note)
            print(f"{note},{selector},{20*np.log10(level):.2f},{polarity},"
                  f"{20*np.log10(residual):.2f},{np.median(abs(change[1:])):.2f},"
                  f"{max(abs(change[1:])):.2f}")
            if selector == "CA":
                listening.extend((a, np.zeros(fs//8), b / level, np.zeros(fs//3)))
    excerpt = np.concatenate(listening)
    sf.write(directory / "CA-D6-then-E7-level-matched.wav", .85 * excerpt / max(abs(excerpt)), fs)
if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("directory",type=Path)
    print("midi,selector,E7_minus_D6_RMS_dB,polarity,matched_residual_dB,median_H2_H10_diff_dB,max_H2_H10_diff_dB")
    compare(p.parse_args().directory)
