"""Compare Baldwin, Roadmaster and CP-70B at their installed electrical outputs."""
import argparse
import csv
import itertools
import wave
from pathlib import Path

import numpy as np
from mido import Message
from pedalboard import VST3Plugin
from grand_revisions import RATE, decibels, region, rms


MODELS = {
    "Baldwin": {"Baldwin Electro-Piano": 1},
    "Roadmaster": {"Roadmaster 64": 1},
    "CP-70B": {"Piano": 1, "Grand Model": 1},
}
# Cross the estimated Baldwin rail split and the transition to plain wire.
NOTES = [36, 43, 44, 45, 48, 49, 54, 60, 64, 72, 84]


def partial_levels(audio, note):
    segment = region(audio, .13, .35)
    size = 131072
    spectrum = np.abs(np.fft.rfft(segment * np.hanning(len(segment)), n=size))
    expected = 440 * 2 ** ((note - 69) / 12)
    partials = []
    for harmonic in (1, 2, 3):
        # Allow unequal tuning, unisons and stiff-string partials within the band.
        center = expected * harmonic
        low = round((center - .12 * expected) * size / RATE)
        high = round((center + .18 * expected) * size / RATE)
        partials.append(decibels(spectrum[low:high + 1].max()))
    return np.subtract(partials[1:], partials[0])


def strikes(plugin, isolated):
    trials = list(itertools.product(NOTES, (48, 100)))
    if isolated:
        for note, velocity in trials:
            events = [Message("note_on", note=note, velocity=velocity, time=.04),
                      Message("note_off", note=note, time=.66)]
            take = plugin(events, sample_rate=RATE, duration=.92, num_channels=2,
                          buffer_size=128, reset=True)
            yield note, velocity, take[0].astype(np.float64)
        return
    events = []
    spacing = 1.75
    for i, (note, velocity) in enumerate(trials):
        start = i * spacing
        events.extend([
            Message("note_on", note=note, velocity=velocity, time=start + .04),
            Message("note_off", note=note, time=start + .66),
            Message("control_change", control=120, value=0, time=start + .96),
        ])
    take = plugin(events, sample_rate=RATE, duration=len(trials) * spacing,
                  num_channels=2, buffer_size=128, reset=True)[0].astype(np.float64)
    for i, (note, velocity) in enumerate(trials):
        audio = region(take, i * spacing, i * spacing + .92)
        assert np.max(abs(region(audio, 0, .025))) < 1e-5, "residual signal before next strike"
        yield note, velocity, audio


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("plugin", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--isolated", action="store_true",
                        help="reset the host for each note to check for residual effects of CC120")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    values = {}
    clips = {}
    with (args.output / "uprights.csv").open("w", newline="") as file:
        table = csv.writer(file)
        table.writerow(["model", "note", "velocity", "H2_over_H1_dB", "H3_over_H1_dB",
                        "release_change_dB", "peak_dBFS"])
        for model, selection in MODELS.items():
            plugin = VST3Plugin(str(args.plugin))
            for name, position in selection.items():
                plugin._get_parameter(name).raw_value = position
            rows = []
            for note, velocity, audio in strikes(plugin, args.isolated):
                assert np.isfinite(audio).all() and np.max(np.abs(audio)) > 1e-6
                h2, h3 = partial_levels(audio, note)
                release = decibels(rms(region(audio, .73, .83))) - decibels(rms(region(audio, .54, .64)))
                rows.append([h2, h3, release])
                table.writerow([model, note, velocity, h2, h3, release, decibels(max(abs(audio)))])
                if velocity == 100 and note in (43, 60, 84):
                    clips[note, model] = audio * (.09 / rms(region(audio, .10, .38)))
            values[model] = np.array(rows)
            file.flush()
    for a, b in itertools.combinations(MODELS, 2):
        print(a, b, "median absolute differences, H2/H1, H3/H1, release (dB)",
              np.round(np.median(abs(values[a] - values[b]), axis=0), 2))
    # Each pitch plays three times in model order. Match body RMS, then apply
    # a common final gain to preserve each attack and key-release envelope.
    auditions = [np.r_[clips[note, model], np.zeros(round(.12 * RATE))]
                 for note in (43, 60, 84) for model in MODELS]
    audio = np.concatenate(auditions)
    pcm = np.rint(audio * (.9 * 32767 / max(abs(audio)))).astype("<i2")
    with wave.open(str(args.output / "baldwin-roadmaster-cp70b.wav"), "wb") as out:
        out.setparams((1, 2, RATE, len(pcm), "NONE", "not compressed"))
        out.writeframes(pcm.tobytes())


if __name__ == "__main__":
    main()
