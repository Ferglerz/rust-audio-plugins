"""Measure Stage 73 era RMS and harmonic balance through an installed Electric Pianos VST3.

Usage: python3 stage_levels.py PLUGIN OUTPUT
"""

import argparse
import csv
import wave
from pathlib import Path

import numpy as np
from mido import Message
from pedalboard import VST3Plugin

RATE = 48000
KEYS = range(28, 101, 3)
VELOCITIES = (40, 64, 88, 112)


def render(plugin, events, duration):
    audio = plugin(
        events,
        duration=duration,
        sample_rate=RATE,
        num_channels=2,
        buffer_size=256,
        reset=True,
    )
    assert np.isfinite(audio).all() and np.max(abs(audio)) > 0
    return audio


def strike(events, notes, velocity, start, end):
    events.extend(
        Message("note_on", note=n, velocity=velocity, time=start) for n in notes
    )
    events.extend(Message("note_off", note=n, time=end) for n in notes)


def measurements(plugin):
    for velocity in VELOCITIES:
        events = []
        for i, n in enumerate(KEYS):
            start = 0.05 + i * 0.6
            strike(events, [n], velocity, start, start + 0.4)
        audio = render(plugin, events, 0.05 + len(KEYS) * 0.6)
        for i, n in enumerate(KEYS):
            start = round((0.05 + i * 0.6) * RATE)
            held = audio[:, start : start + round(0.4 * RATE)].astype(np.float64)
            power = np.mean(held**2)
            middle = held[0, round(0.09 * RATE) : round(0.30 * RATE)]
            window = middle * np.hanning(len(middle))
            hz = 440 * 2 ** ((n - 69) / 12)
            phase = 2j * np.pi * hz * np.arange(len(middle)) / RATE
            h1, h2, h3 = (abs(np.sum(window * np.exp(h * phase))) for h in (1, 2, 3))
            yield n, velocity, power, 20 * np.log10(h2 / h1), 20 * np.log10(h3 / h1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("plugin", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(exist_ok=True, parents=True)
    plugin = VST3Plugin(str(args.plugin))
    plugin._get_parameter("Piano").raw_value = 0.5
    plugin._get_parameter("Vibrato Intensity").raw_value = 0
    selector = plugin._get_parameter("Era")
    phrase = []
    strike(phrase, [40, 55, 59, 62], 64, 0.05, 0.55)
    strike(phrase, [45, 55, 61, 64], 100, 0.65, 1.05)
    strike(phrase, [88], 88, 1.15, 2.0)
    clips = []
    levels = []
    with (args.output / "stage-strikes.csv").open("w", newline="") as file:
        writer = csv.writer(file)
        writer.writerow(
            ("era", "peterson", "midi", "velocity", "rms_dbfs", "h2_h1_db", "h3_h1_db")
        )
        for peterson in (False, True):
            plugin._get_parameter("Peterson Suitcase").raw_value = float(peterson)
            for i in range(1 if peterson else 0, 6):
                selector.raw_value = i / 5
                era = selector.get_text_for_raw_value(selector.raw_value)
                power = []
                for n, v, p, h2, h3 in measurements(plugin):
                    writer.writerow((era, peterson, n, v, 10 * np.log10(p), h2, h3))
                    power.append(p)
                level = 10 * np.log10(np.mean(power))
                levels.append(level)
                print(
                    f"{era} ({'Peterson' if peterson else 'stock'}), {level:.3f} dBFS RMS",
                    flush=True,
                )
                if not peterson:
                    clips.append(render(plugin, phrase, 2.4))
    # One gain for the whole comparison preserves the installed playback balance.
    audio = np.concatenate(clips, axis=1)
    pcm = np.round(audio.T * (0.85 * 32767 / np.max(abs(audio)))).astype("<i2")
    with wave.open(str(args.output / "stage-era-comparison.wav"), "wb") as output:
        output.setparams((2, 2, RATE, 0, "NONE", "not compressed"))
        output.writeframes(pcm.tobytes())
    assert max(levels) - min(levels) < 0.15, (
        "Stage 73 playback RMS spread exceeds 0.15 dB"
    )


if __name__ == "__main__":
    main()
