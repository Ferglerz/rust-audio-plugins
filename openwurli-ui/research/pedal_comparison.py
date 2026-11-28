"""Measure a held E2–B2–E3 chord in the installed Electric Pianos VST3.

Usage: python3 pedal_comparison.py PLUGIN OUTPUT [--hardware UP.wav DOWN.wav]
For hardware use two dry, equal-gain takes following the timing in phrase().
"""

import argparse
from pathlib import Path

import numpy as np
from mido import Message
from pedalboard import VST3Plugin
from scipy.io import wavfile

RATE = 48000
CHORD = (40, 47, 52)


def phrase(notes, pedal):
    # Times are absolute seconds from the start of each render.
    events = [Message("note_on", note=n, velocity=96, time=0.05) for n in notes]
    if pedal:
        events.append(Message("control_change", control=64, value=127, time=0.35))
    # Lift the pedal while the chord is still fingered, then release the keys.
    events.append(Message("control_change", control=64, value=0, time=1.15))
    events.extend(Message("note_off", note=n, time=1.5) for n in notes)
    return events


def rms(x):
    return np.sqrt(np.mean(np.square(x, dtype=np.float64)))


def db(ratio):
    return 20 * np.log10(max(ratio, 1e-15))


def partial_report(a, b, rate):
    start, end = int(0.45 * rate), int(1.12 * rate)
    # A Hann-windowed projection is insensitive to starting phase. Printed
    # changes include all energy at each frequency, including spectral leakage.
    window = np.hanning(end - start)
    t = np.arange(end - start) / rate
    levels = []
    for note in (40, 41, 46, 47, 48, 51, 52, 53, 64):
        hz = 440 * 2 ** ((note - 69) / 12)
        kernel = np.exp(-2j * np.pi * hz * t) * window
        levels.append(
            (note, *(db(2 * abs(x[start:end] @ kernel) / window.sum()) for x in (a, b)))
        )
    print("MIDI   pedal up dBFS   pedal down dBFS")
    for note, up, down in levels:
        print(f"{note:4}   {up:13.2f}   {down:15.2f}")


def render_dry(plugin, selector, frame, notes, pedal):
    # Reset the same instance for each take, including its circuits and damper state.
    plugin._get_parameter("Piano").raw_value = selector
    plugin._get_parameter("Frame Ring").raw_value = float(frame)
    output = plugin(
        phrase(notes, pedal),
        duration=2.0,
        sample_rate=RATE,
        num_channels=2,
        buffer_size=128,
        reset=True,
    )
    assert np.isfinite(output).all() and np.array_equal(output[0], output[1])
    return output[0].astype(np.float64)


def main():
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument("plugin", type=Path)
    args.add_argument("output", type=Path)
    args.add_argument("--hardware", nargs=2, type=Path, metavar=("UP", "DOWN"))
    args = args.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    plugin = VST3Plugin(str(args.plugin))
    for name, selector, frame in (
        ("stage", 0.5, False),
        ("grand-stock", 1.0, False),
        ("grand-frame", 1.0, True),
    ):
        up = render_dry(plugin, selector, frame, CHORD, False)
        down = render_dry(plugin, selector, frame, CHORD, True)
        single_sum = sum(render_dry(plugin, selector, frame, [n], False) for n in CHORD)
        early = slice(int(0.05 * RATE), int(0.34 * RATE))
        held = slice(int(0.40 * RATE), int(1.15 * RATE))
        # VST3 CC automation may arrive at the start of the containing block.
        # End the null check before that block; score the settled held chord.
        assert rms(up[early]) > 1e-4 and np.array_equal(up[early], down[early])
        difference = down - up
        level = db(rms(difference[held]) / rms(up[held]))
        if name == "grand-stock":
            assert np.array_equal(up, down)
        else:
            assert -100 < level < -6
        pedal_level = (
            "identical samples"
            if np.array_equal(up, down)
            else f"{level:.2f} dB relative to pedal-up chord"
        )
        print(
            f"{name}: held pedal difference {pedal_level}; "
            f"pedal-up chord minus sum of singles {db(rms((up - single_sum)[held]) / rms(up[held])):.2f} dB"
        )
        if name != "grand-stock":
            before_lift = rms(difference[int(1.05 * RATE) : int(1.15 * RATE)])
            after_lift = rms(difference[int(1.21 * RATE) : int(1.23 * RATE)])
            print(
                f"Residual 60–80 ms after pedal lift: {db(after_lift / before_lift):.2f} dB relative to the preceding 100 ms"
            )
        partial_report(up, down, RATE)
        for suffix, audio in (("up", up), ("down", down), ("difference", difference)):
            wavfile.write(
                args.output / f"{name}-{suffix}.wav", RATE, audio.astype(np.float32)
            )

    plugin._get_parameter("Piano").raw_value = 0.5

    def stereo(peterson, intensity):
        plugin._get_parameter("Peterson Suitcase").raw_value = float(peterson)
        plugin._get_parameter("Vibrato Intensity").raw_value = intensity
        return plugin(
            phrase(CHORD, True),
            duration=2.0,
            sample_rate=RATE,
            num_channels=2,
            buffer_size=128,
            reset=True,
        )

    passive = stereo(False, 0.0)
    steady = stereo(True, 0.0)
    suitcase = stereo(True, 1.0)
    assert np.array_equal(steady[0], steady[1]) and np.array_equal(
        passive[0], passive[1]
    )
    assert rms(suitcase[0] - suitcase[1]) > 0.001 and rms(steady - passive) > 0.001
    clip = np.concatenate(
        (passive, np.zeros((2, round(0.25 * RATE))), suitcase), axis=1
    )
    wavfile.write(
        args.output / "stage-passive-then-peterson.wav", RATE, clip.T.astype(np.float32)
    )

    if args.hardware:
        pair = []
        for path in args.hardware:
            fs, signal = wavfile.read(path)
            if signal.dtype.kind == "i":
                signal = signal.astype(np.float64) / float(
                    1 << (signal.dtype.itemsize * 8 - 1)
                )
            elif signal.dtype.kind == "u":
                signal = (signal.astype(np.float64) - 128) / 128
            signal = signal if signal.ndim == 1 else signal[:, 0]
            pair.append((fs, signal))
        (fs, a), (other, b) = pair
        if fs != other or min(len(a), len(b)) < 1.5 * fs:
            raise ValueError("Use equal-rate, aligned takes covering the held chord.")
        print("Hardware, fixed-gain left/mono channel")
        partial_report(a, b, fs)


if __name__ == "__main__":
    main()
