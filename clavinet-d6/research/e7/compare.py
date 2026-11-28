"""Compare three notes in Timothy Stedman's E7 chromatic run with a fixed model strike."""
import argparse
import hashlib
import subprocess
from pathlib import Path

import numpy as np
import soundfile as sf

RATE = 44100
# Start of a 75 ms window in the ascending scale, in seconds from the video start.
WINDOWS = [(46, 124.728), (64, 126.838), (78, 128.446)]
SOURCE_SHA256 = "8bcd5d54c72d3d9883411608af54b7dac442476da3d7b344a83175e446958a86"


def profile(audio, start, midi):
    count = round(0.075 * RATE)
    i = round(start * RATE)
    mag = abs(np.fft.rfft(audio[i:i + count] * np.hanning(count), 32768))
    f = np.fft.rfftfreq(32768, 1 / RATE)
    nominal = 440 * 2**((midi - 69) / 12)
    estimates = []
    for h in range(1, 7):
        choose = np.flatnonzero(abs(f - h * nominal) < max(17, 0.014 * h * nominal))
        k = choose[np.argmax(mag[choose])]
        a, b, c = np.log(mag[k-1:k+2] + 1e-18)
        peak = f[k] + 0.5 * f[1] * (a - c) / (a - 2*b + c)
        if abs(1200 * np.log2(peak / h / nominal)) < 30 and mag[k] > mag.max() * 0.05:
            estimates.append(peak / h)
    if len(estimates) < 2:
        raise ValueError(f"Insufficient resolved partials on MIDI {midi}")
    hz = np.median(estimates)
    # E4's fundamental overlaps the preceding key. Use the same H2-H8 test on every note.
    amps = np.array([mag[abs(f - h * hz) < max(12, 0.009 * h * hz)].max()
                     for h in range(2, 9)])
    band = (f > 65) & (f < 6200)
    harmonic = abs(f - np.rint(f / hz) * hz) < np.maximum(22, 0.008 * f)
    purity = sum(mag[band & harmonic]**2) / sum(mag[band]**2)
    return hz, 20 * np.log10(amps / amps.max()), purity


def compare(source, probe):
    if hashlib.sha256(source.read_bytes()).hexdigest() != SOURCE_SHA256:
        raise ValueError("Expected the published E7 demonstration, YouTube Opus format 251")
    pcm = subprocess.check_output(["ffmpeg", "-v", "error", "-i", str(source), "-ac", "1",
                                   "-ar", str(RATE), "-f", "f32le", "-"])
    hardware = np.frombuffer(pcm, dtype="<f4")
    print("MIDI | time | harmonic power | hardware / CA model Hz | H2–H8 median error CA CB DA DB, dB")
    for midi, start in WINDOWS:
        freq, ref, purity = profile(hardware, start, midi)
        errors = []
        for setting in ("CA", "CB", "DA", "DB"):
            model, rate = sf.read(probe / str(midi) / f"jack-{setting}.wav")
            if rate != RATE:
                raise ValueError("Expected the three-key 44.1 kHz fixture")
            pitch, simulated, _ = profile(model, 0.075, midi)
            if setting == "CA":
                ca_pitch = pitch
            errors.append(np.median(abs(ref - simulated)))
        print(f"{midi} | {start:.3f} | {purity:.0%} | {freq:.2f} / {ca_pitch:.2f} | " +
              " ".join(f"{e:.1f}" for e in errors))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("probe", type=Path)
    args = parser.parse_args()
    compare(args.source, args.probe)
