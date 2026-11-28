"""Inspect repeatable dominant pitches in SynthDan1's 1939 Storytone performance."""
import argparse
from pathlib import Path

import numpy as np
from scipy.io import wavfile


# Window starts in seconds, manually checked against neighboring spectra.
WINDOWS = [(64, 25.70), (64, 41.15), (81, 26.30), (81, 44.49),
           (83, 46.88), (83, 47.88), (86, 10.16), (86, 47.56)]


def partials(path, start, note, channel=0):
    rate, wav = wavfile.read(path, mmap=True)
    raw = wav[round(start * rate):round((start + .12) * rate)]
    y = raw if raw.ndim == 1 else raw[:, channel]
    n = 8 * len(y)
    spectrum = abs(np.fft.rfft(y * np.hanning(len(y)), n))
    hz = np.fft.rfftfreq(n, 1 / rate)
    nominal = 440 * 2 ** ((note - 69) / 12)

    def peak(center, width):
        where = np.flatnonzero(abs(hz - center) < width)
        k = where[np.argmax(spectrum[where])]
        return hz[k], spectrum[k]

    f0, fundamental = peak(nominal, nominal * .025)
    rel = []
    prominence = []
    for h in [2, 3]:
        # Stiffness can stretch a partial. A weak peak may be room noise,
        # another string, or an unrelated musical event inside the window.
        center = h * f0
        width = max(12, .012 * center)
        f, p = peak(center, width)
        annulus = (abs(hz - f) > width + 9) & (abs(hz - f) < width + 63)
        rel.append(20 * np.log10(p / fundamental))
        prominence.append(20 * np.log10(p / np.median(spectrum[annulus])))
    return f0, rel, prominence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('recording', type=Path)
    parser.add_argument('model', type=Path, help='output directory from storytone_probe notes')
    args = parser.parse_args()
    print('model pickup: note, onset to 1% of first 100 ms peak, first full 10 ms block 20 dB under pre-release RMS')
    for note in [21, 50, 64, 81, 96]:
        rate, wav = wavfile.read(args.model / f'{note}-0.60.wav', mmap=True)
        emf = wav[:, 0].astype(np.float64)
        onset = np.flatnonzero(abs(emf) > .01 * abs(emf[:rate//10]).max())[0]
        release = int(1.25 * rate)
        window = rate // 100
        before = np.mean(emf[release-4*window:release]**2)
        times = [n for n in range(release, len(emf) - window + 1, window)
                 if np.mean(emf[n:n+window]**2) < before / 100]
        print(f'{note} {1000*onset/rate:.2f} ms', f'{(times[0]-release)/rate*1000:.0f} ms' if times else '>240 ms')
    print('note, recording time, source, fundamental Hz, H2/H1 dB, H3/H1 dB, H2/H3 local prominence dB')
    for note, start in WINDOWS:
        for name, path, t, ch in [('room', args.recording, start, 0),
                                  ('aux', args.model / f'{note}-0.60.wav', .12, 3)]:
            f, rel, margin = partials(path, t, note, ch)
            print(f'{note:3} {start:5.2f} {name:4} {f:7.1f}  {rel[0]:5.1f} {rel[1]:5.1f}  {margin[0]:5.1f} {margin[1]:5.1f}')


if __name__ == '__main__':
    main()
