"""Compare the measured Rust pickup impulse against the continuous ngspice deck."""
from pathlib import Path
import subprocess
import sys

import numpy as np
from scipy.io import wavfile

out = Path(sys.argv[1]).resolve()
deck = Path(__file__).with_name("harness.cir").resolve()
subprocess.run(["ngspice", "-b", str(deck)], cwd=out, check=True, capture_output=True)
frequency, real, imag = np.loadtxt(out / "wired-ac.txt", unpack=True)
rate, impulse = wavfile.read(out / "wired-pickup.wav")
# Interpolate the measured FFT at the exact SPICE frequencies, including the
# small low-frequency phase change between integer FFT bins.
spectrum = np.fft.rfft(impulse.astype(float), n=8 * len(impulse))
bins = np.fft.rfftfreq(8 * len(impulse), 1 / rate)
error = np.interp(frequency, bins, spectrum) / (real + 1j * imag)
level = max(abs(20 * np.log10(abs(error))))
phase = max(abs(np.angle(error, deg=True)))
print(f"20 Hz to 20 kHz, maximum error {level:.5f} dB and {phase:.5f} degrees")
assert level < 0.001 and phase < 0.01
