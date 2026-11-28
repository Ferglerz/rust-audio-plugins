"""Measure the physical probe, Hohner's three jack levels, and an independent AC deck."""
from pathlib import Path
import argparse
import hashlib
import subprocess
import tempfile

import numpy as np
from scipy.io import wavfile


def rms(audio):
    return np.mean(audio ** 2, axis=0) ** 0.5


def read(path):
    rate, data = wavfile.read(path)
    if data.ndim != 2 or data.shape[1] != 3 or not np.all(np.isfinite(data)):
        raise ValueError(f"Expected three finite voltage channels in {path}")
    return rate, data.astype(float)


def spice_response(loads):
    deck = Path(__file__).with_name("early-n.cir").read_text()
    for port, node, ohms in zip(("Low", "Mid", "High"), ("low", "mid", "high"), loads):
        deck = deck.replace(f"Rload{port} {node} 0 10meg", f"Rload{port} {node} 0 {ohms:g}")
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "panel.cir"
        path.write_text(deck)
        subprocess.run(["ngspice", "-b", str(path)], cwd=tmp, check=True, capture_output=True)
        result = np.loadtxt(Path(tmp) / "response.txt")
    return result[:, 0], result[:, 1::2] + 1j * result[:, 2::2]


def compare_panel(root, chord_rate, chord):
    rate, impulse = read(root / "panel-impulse.wav")
    gains = np.fft.rfft(impulse, axis=0)
    bins = np.fft.rfftfreq(len(impulse), 1.0 / rate)
    analogue_hz, spice = spice_response([10e6] * 3)
    hz = rate / np.pi * np.arctan(np.pi * analogue_hz / rate)
    digital = np.array([np.interp(hz, bins, signal) for signal in gains.T]).T
    ratio = digital / spice
    gain_error = np.max(abs(20 * np.log10(abs(ratio))), axis=0)
    phase_error = np.max(abs(np.rad2deg(np.angle(ratio))), axis=0)
    print("Circuit vs SPICE, worst 20 Hz–20 kHz gain dB", gain_error.round(4),
          "phase degrees", phase_error.round(4))
    if np.max(gain_error) > 0.03 or np.max(phase_error) > 0.2:
        raise ValueError("Electrical transfer differs from the analogue reference")
    # Compare a receiver plugged into only one jack. Other jacks are effectively open.
    count = 2 * len(chord)
    chord_hz = np.fft.rfftfreq(count, 1 / chord_rate)
    chord_fft = np.fft.rfft(chord, n=count, axis=0)
    loaded_peaks = []
    for i, ohms in enumerate((100e3, 200e3, 7e3)):
        loads = [1e13] * 3
        loads[i] = ohms
        _, loaded = spice_response(loads)
        change = np.interp(chord_hz, hz, loaded[:, i] / spice[:, i])
        out = np.fft.irfft(chord_fft[:, i] * change, n=count)
        loaded_peaks.append(max(abs(out[round(.05 * chord_rate):round(.25 * chord_rate)])) * 1000)
    print("Triad, peak mV with only one receiver connected (100k/200k/7k)", np.round(loaded_peaks, 1))


def component_slope(signal, rate, start, end):
    # Fixed-width complex projection at E4. The reference chord's nearby tones
    # are outside this window's principal lobe. Normalizing removes unknown gain.
    f = 440 * 2 ** ((64 - 69) / 12)
    n = round(0.15 * rate)
    window = np.hanning(n) * np.exp(-2j * np.pi * f * np.arange(n) / rate)
    times = np.arange(start, end + 0.001, 0.04)
    levels = [20 * np.log10(abs(window @ signal[round(t*rate):round(t*rate)+n]))
              for t in times]
    slope, offset = np.polyfit(times, levels, 1)
    scatter = rms(levels - (slope * times + offset))
    return slope, scatter


def main(root, sampled):
    audition = []
    for key in (45, 64, 81):
        rate, output = read(root / f"key-{key}.wav")
        trace = np.genfromtxt(root / f"key-{key}.csv", delimiter=",", names=True)
        release = np.flatnonzero(np.diff(trace["bonded"]) < 0)[0] + 1
        ons = 1000 * (trace["time_s"][release] - 0.05)
        tail = 20 * np.log10(rms(output[round(.85*rate):round(.89*rate), 2])
                              / rms(output[round(.74*rate):round(.78*rate), 2]))
        rate, repeats = read(root / f"repeats-{key}.wav")
        audition.extend((repeats[:, 2], np.zeros(rate // 6)))
        def rms_after(t):
            return rms(repeats[round((t+.016)*rate):round((t+.096)*rate), 2])
        dynamics = 20 * np.log10(rms_after(.780) / rms_after(.05))
        short = 20 * np.log10(rms_after(.402) / rms_after(.05))
        print(f"MIDI {key}: first detached trace {ons:.2f} ms, tip {trace['tip_m'][release]*1000:.3f} mm, "
              f"key-up tail {tail:.1f} dB, 22 ms reseat {short:.1f} dB, hard/soft {dynamics:+.1f} dB")
    rate, chord = read(root / "factory-triad.wav")
    wavfile.write(root / "pianet-n-audition.wav", rate,
                  np.concatenate([*audition, chord[:, 2]]).astype(np.float32))
    early = chord[round(.05*rate):round(.25*rate)]
    peak = abs(early).max(axis=0) * 1000
    print("Factory triad, peak mV", peak.round(1), "RMS mV", (rms(early)*1000).round(1),
          "peak ratios", (peak / peak[0]).round(2), "factory labels 100 300 600 mV")
    compare_panel(root, rate, chord)
    if sampled is not None:
        import soundfile as sf
        if hashlib.sha256(sampled.read_bytes()).hexdigest() != SAMPLED_HASH:
            raise ValueError("Unexpected OTO×NOMA Pianet N excerpt")
        y, sr = sf.read(sampled, always_2d=True)
        model_rate, model = read(root / "key-64.wav")
        print("E4 fundamental decay slope and fit RMS (dB/s, dB)")
        print("Sample-library chord", np.round(component_slope(y.mean(axis=1), sr, 2.85, 3.33), 2),
              "three-key model", np.round(component_slope(model[:, 2], model_rate, .12, .60), 2))


SAMPLED_HASH = "6f42c6c1e161b9001784c28bf35f29f896e9b4215eac47fda314801b12121d11"
if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("renders", type=Path)
    parser.add_argument("--sampled-performance", type=Path)
    args = parser.parse_args()
    main(args.renders, args.sampled_performance)
