"""Compare the three installed Yamaha models on identical, isolated strikes.

Usage: python3 grand_revisions.py PLUGIN OUTPUT
Requires NumPy, Mido and Pedalboard. The table and audition keep the stock
plugin calibration. Harmonic and key-release ratios cancel that constant gain.
"""
import argparse
import csv
import wave
from pathlib import Path

import numpy as np
from mido import Message
from pedalboard import VST3Plugin

RATE = 48000
# Construction boundaries F#2/G2 and E4/F4, pickup branches and last key damper.
KEYS = [28, 42, 43, 64, 65, 69, 89, 90, 91, 92, 93, 94, 100]


def decibels(value):
    return 20 * np.log10(max(float(value), 1e-14))


def rms(x):
    return np.sqrt(np.mean(x * x))


def region(x, start, end):
    return x[round(RATE * start):round(RATE * end)]


def harmonic_pair(x, note):
    y = region(x, 0.12, 0.33)
    hz = 440 * 2 ** ((note - 69) / 12)
    # Follow the model's stretch and paired-string beating rather than bin centres.
    f = np.linspace(hz * 2 ** (-25 / 1200), hz * 2 ** (25 / 1200), 101)
    t = np.arange(len(y)) / RATE
    yw = y * np.hanning(len(y))
    fundamental = f[np.argmax(np.abs(np.exp(-2j * np.pi * f[:, None] * t) @ yw))]
    bands = [abs(np.sum(yw * np.exp(-2j * np.pi * h * fundamental * t))) for h in (1, 2, 3)]
    return [decibels(b / max(bands[0], 1e-14)) for b in bands[1:]]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('plugin', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    args.output.mkdir(exist_ok=True, parents=True)
    plugin = VST3Plugin(str(args.plugin))
    plugin._get_parameter('Piano').raw_value = 1
    clips = []
    with (args.output/'grand-revisions.csv').open('w', newline='') as out:
        writer = csv.writer(out)
        writer.writerow(['model', 'midi', 'velocity', 'peak_dBFS', 'attack_rms_dBFS',
                         'H2_over_H1_dB', 'H3_over_H1_dB', 'release_change_dB'])
        for model, grand, original in [('CP-70', 1, 1), ('CP-70B', 1, 0), ('CP-80', 0, 0)]:
            plugin._get_parameter('Grand Model').raw_value = grand
            plugin._get_parameter('Original CP-70').raw_value = original
            for note in KEYS:
                for velocity in (48, 100):
                    msgs = [Message('note_on', note=note, velocity=velocity, time=.04),
                            Message('note_off', note=note, time=.54)]
                    stereo = plugin(msgs, sample_rate=RATE, duration=.82, num_channels=2,
                                    buffer_size=128, reset=True)
                    x = stereo[0].astype(np.float64)
                    assert np.isfinite(stereo).all() and np.max(np.abs(x)) > 1e-6
                    attack = decibels(rms(region(x, .055, .13)))
                    release = decibels(rms(region(x, .69, .77))) - decibels(rms(region(x, .44, .52)))
                    writer.writerow([model, note, velocity, *[f'{v:.4f}' for v in
                                     (decibels(np.max(np.abs(x))), attack, *harmonic_pair(x, note), release)]])
                    if velocity == 100 and note in (42, 64, 93, 94):
                        clips.append(stereo.T)
            print(model, 'verified', 2 * len(KEYS), 'strikes', flush=True)
    all_clips = np.concatenate(clips)
    with wave.open(str(args.output/'grand-revisions.wav'), 'wb') as audio:
        audio.setparams((2, 2, RATE, 0, 'NONE', 'not compressed'))
        audio.writeframes(np.rint(np.clip(all_clips, -1, 1) * 32767).astype('<i2').tobytes())


if __name__ == '__main__':
    main()
