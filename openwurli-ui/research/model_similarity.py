"""Measure the Electric Pianos VST3 offline. Requires Pedalboard, Mido and NumPy.

Run: python3 model_similarity.py PLUGIN.vst3 OUTPUT_DIRECTORY
"""
import argparse
import csv
import ctypes as c
from concurrent.futures import ProcessPoolExecutor, as_completed
from hashlib import sha256
from itertools import combinations
import json
from pathlib import Path
import wave

from mido import Message
import numpy as np
import pedalboard
from pedalboard_native import ExternalPluginReloadType

RATE = 48_000
BLOCK = 128
VELOCITIES = (40, 80, 115)
START, RELEASE, END = .04, .79, 1.29

STAGE = ("Sparkletop", "Early Mark I", "Mid Mark I", "Late Mark I", "Mark II", "Mark V")
GRANDS = ("CP-70", "CP-70B", "CP-80")
CLAVS = ("Clavinet D6", "Clavinet E7", "Vibanet", "Duo Clavinet")
REEDS = ("Pianet N", "Pianet C", "Pianet L", "Pianet T", "Cembalet", "Cembalet N",
         "Claviset 200", "Selmer Pianotron", "Duo Pianet")
TINES = (*STAGE, "Rhodes MK8 S", "Vintage Vibe 64", "Vintage Vibe StereoVibe")
METAL = ("OpenWurli", "Electra-Piano", "Elepian EP-61C", "Valente Electric Piano",
         "Vivi-Tone Clavier")
STRINGS = ("Storytone", "Neo-Bechstein", "Roadmaster 64", "Baldwin Electro-Piano", *GRANDS)
DUO = "Pianet/Clavinet Duo"

# Native compasses and adjacent keys at changes documented in the model sources.
# Each entry holds (compass, boundary keys, normalized VST3 selection).
MODELS = {
    "OpenWurli": ((33, 96), (), {}),
    **{name: ((28, 100), (43, 44, 57, 58, 63, 64, 68, 69, 80, 81, 92, 93),
              {"Piano": .5, "Era": i / 5}) for i, name in enumerate(STAGE)},
    **{name: (((21, 108) if i == 2 else (28, 100)),
              (42, 43, 62, 63, 64, 65, 89, 90, 91, 92, 93, 94, 102, 103),
              {"Piano": 1, "Grand Model": int(i < 2), "Original CP-70": int(i == 0)})
       for i, name in enumerate(GRANDS)},
    **{name: ((29, 88), (48, 49), {"Instrument Family": 1 / 3, "Clavinet Model": i})
       for i, name in enumerate(CLAVS[:2])},
    **{name: ((29, 89), (), {"Instrument Family": 2 / 3, "Pianet Revision": i / 2})
       for i, name in enumerate(REEDS[:3])},
    "Pianet T": ((29, 88), (), {"Instrument Family": 1}),
    **{name: ((36, 96), (), {"Instrument Family": 2 / 3, "Electrostatic Reed Model": (i + 1) / 2})
       for i, name in enumerate(REEDS[4:6])},
    **{name: ((36, 96), (), {"Claviset Family": (i + 1) / 2})
       for i, name in enumerate(REEDS[6:8])},
    **{name: ((29, 88), (48, 49), {"Instrument Family": 1 / 3, DUO: 1,
                                  **({"Duo Separate Recording Outputs": 1} if name != DUO else {})})
       for name in (DUO, "Duo Clavinet", "Duo Pianet")},
    **{name: (compass, boundaries, {name: 1}) for name, compass, boundaries in (
        ("Electra-Piano", (29, 100), (38, 39, 48, 49, 58, 59, 68, 69, 78, 79, 88, 89)),
        ("Storytone", (21, 108), ()), ("Neo-Bechstein", (21, 108), ()),
        ("Roadmaster 64", (33, 96), ()), ("Baldwin Electro-Piano", (21, 108), (48, 49, 52, 53, 56, 57)),
        ("Elepian EP-61C", (36, 96), ()), ("Vivi-Tone Clavier", (24, 96), ()),
        ("Valente Electric Piano", (36, 96), ()), ("Rhodes MK8 S", (28, 100), ()),
        ("Vibanet", (29, 88), ()), ("Vintage Vibe 64", (33, 96), ()),
        ("Vintage Vibe StereoVibe", (33, 96), ()),
    )},
}

GROUPS = {
    "Header defaults": ("OpenWurli", "Late Mark I", "CP-80"),
    "Stage 73 eras": STAGE,
    "Yamaha revisions": GRANDS,
    "Hohner menu": (*CLAVS[:2], DUO, *REEDS[:6], "Electra-Piano"),
    "Claviset menu": REEDS[6:8],
    "Electric Uprights menu": ("Storytone", "Vivi-Tone Clavier"),
    "Vintage Vibe menu": ("Vintage Vibe 64", "Vintage Vibe StereoVibe", "Vibanet"),
    "Tine and tonebar": TINES,
    "Tangent strings": CLAVS,
    "Plucked reeds": REEDS,
    "Other metal resonators": METAL,
    "Struck strings": STRINGS,
}

# Duplicate display names are numbered in VST3 parameter order.
DRY = {
    "OpenWurli": {"Tremolo Depth [1]": 0, "Extra Sag": 0},
    "Sparkletop": {"Tremolo [1]": 0},
    **{name: {"Tremolo [2]": 0, "Frame Ring": 0} for name in GRANDS},
    "Pianet N": {"Pianet N Vibrato": 0},
    "Pianet C": {"Pianet C Vibrato": 0},
    "Cembalet": {"Cembalet Vibrato": 0},
    "Cembalet N": {"Cembalet Vibrato": 0},
    "Claviset 200": {"Claviset Vibrato": 0},
    "Selmer Pianotron": {"Pianotron Vibrato Depth": 0},
    "Rhodes MK8 S": dict.fromkeys(("MK8 Vari-Pan", "MK8 Factory FX Board", "MK8 Compressor", "MK8 Chorus", "MK8 Phaser", "MK8 Delay"), 0),
    "Vintage Vibe StereoVibe": {"StereoVibe Tremolo": 0},
    "Vibanet": {"Vibanet Auto-Wah": 0},
}

SETTINGS = {
    "peterson": ({name: {"Peterson Suitcase": 1, "Vibrato Intensity": 0} for name in STAGE[1:]}, STAGE[1:]),
    "stage-tone-min": ({name: {"Stage Tone": 0} for name in STAGE[1:]}, STAGE[1:]),
    "pianet-local": ({"Pianet C": {"Early Pianet Local Output": 1},
                      "Pianet L": {"Early Pianet Local Output": 1}}, REEDS[:3]),
    "pianet-half-knee": ({"Pianet C": {"Early Pianet Knee": .5},
                          "Pianet L": {"Early Pianet Knee": .5}}, REEDS[1:3]),
    "clav-bridge": ({"Clavinet D6": {"D6 Pickup": 1 / 3}, "Clavinet E7": {"D6 Pickup": 1 / 3},
                     "Duo Clavinet": {"Duo String Pickup": 1 / 3}, "Vibanet": {"Vibanet Pickup": 0}}, CLAVS),
    "clav-paired": ({"Clavinet D6": {"D6 Pickup": 1}, "Clavinet E7": {"D6 Pickup": 1},
                     "Duo Clavinet": {"Duo String Pickup": 1}, "Vibanet": {"Vibanet Pickup": 1}}, CLAVS),
    "clav-half-volume": ({"Clavinet D6": {"D6 Panel Volume": .5}, "Clavinet E7": {"E7 Slider Volume": .5}}, CLAVS[:2]),
    "stereovibe-fet": ({"Vintage Vibe StereoVibe": {"StereoVibe Input": .5}}, TINES[-2:]),
    "stereovibe-parallel": ({"Vintage Vibe StereoVibe": {"StereoVibe Input": 1}}, TINES[-2:]),
    "stereovibe-tremolo": ({"Vintage Vibe StereoVibe": {"StereoVibe Tremolo": 1, "StereoVibe Depth": .8}}, TINES[-2:]),
    "cam-register": ({"Claviset 200": {"Claviset Register": 1}, "Selmer Pianotron": {"Pianotron Tone": 1}}, REEDS[6:8]),
    "cam-vibrato": ({"Claviset 200": {"Claviset Vibrato": 2 / 3},
                     "Selmer Pianotron": {"Pianotron Vibrato Depth": .8}}, REEDS[6:8]),
}

METRICS = ("attack_dB", "rise_ms", "harmonics_dB", "decay_dB", "release_dB",
           "velocity_span_dB", "stereo_width", "phrase_envelope_dB")


def db(value):
    return 20 * np.log10(np.maximum(value, 1e-10))


def rms(samples):
    return float(np.sqrt(np.mean(np.square(samples, dtype=np.float64))))


def window(samples, start, end):
    return samples[:, round(start * RATE):round(end * RATE)]


def width(samples):
    return rms(samples[0] - samples[1]) / max(rms(samples) * 2, 1e-10)


def harmonics(samples, note):
    size = 65536
    spectrum = np.abs(np.fft.rfft(samples * np.hanning(samples.shape[-1]), n=size))
    spectrum = np.sqrt(np.mean(spectrum ** 2, axis=0))
    expected = 440 * 2 ** ((note - 69) / 12)
    spacing = expected * size / RATE
    bins = np.arange(len(spectrum))
    fundamental = int(np.argmax(spectrum * (abs(bins - spacing) <= max(2, spacing * .047))))
    # The highest treble notes have one or more harmonics above Nyquist.
    peaks = [np.max(spectrum[abs(bins - fundamental * h) <= max(2, spacing * .16)])
             if fundamental * h < len(bins) else np.nan for h in range(1, 7)]
    return np.maximum(db(np.array(peaks[1:]) / max(peaks[0], 1e-10)), -60)


def strike_metrics(samples, note):
    attack = window(samples, START, START + .08)
    body = window(samples, START + .10, START + .32)
    early = rms(body)
    envelope = np.sqrt(np.mean(attack.reshape(2, -1, 48) ** 2, axis=(0, 2)))
    rise = np.argmax(envelope >= .9 * np.max(envelope))
    late = rms(window(samples, RELEASE - .14, RELEASE - .02))
    tail = rms(window(samples, RELEASE + .06, RELEASE + .18))
    return np.array([db(rms(attack) / max(early, 1e-10)), rise,
                     *harmonics(body, note), db(late) - db(early),
                     db(tail) - db(late), db(early), width(body), db(np.max(abs(samples)))])


def messages(note, velocity):
    return [Message("note_on", note=note, velocity=velocity, time=START),
            Message("note_off", note=note, time=RELEASE)]


def phrase():
    events = []
    for start, stop, notes, v in [
        (.04, .26, (48,), 55), (.27, .48, (60,), 83), (.49, .7, (60,), 110),
        (.76, 1.13, (48, 55, 64), 85), (1.27, 1.63, (55, 60, 67), 103),
    ]:
        for n in notes:
            events += [Message("note_on", note=n, velocity=v, time=start),
                       Message("note_off", note=n, time=stop)]
    for t, value in ((.72, 127), (1.75, 0)):
        events.append(Message("control_change", control=64, value=value, time=t))
    return sorted(events, key=lambda event: event.time)


def digest(samples):
    return sha256(samples.tobytes()).hexdigest()


F32P = c.POINTER(c.c_float)
class Midi(c.Structure):
    _fields_ = [("frame", c.c_int32), ("status", c.c_uint8), ("note", c.c_uint8), ("value", c.c_uint8)]

class NativeVST3:
    def __init__(self, library, path):
        dll = c.CDLL(str(library))
        for name, result_type, argument_types in (
                ("ep_open", c.c_void_p, [c.c_char_p]),
                ("ep_close", None, [c.c_void_p]),
                ("ep_parameter_count", c.c_int32, [c.c_void_p]),
                ("ep_parameter_info", c.c_double, [c.c_void_p, c.c_int32, c.POINTER(c.c_uint32), c.c_void_p]),
                ("ep_set", c.c_double, [c.c_void_p, c.c_uint32, c.c_double]),
                ("ep_render", None, [c.c_void_p, c.POINTER(Midi), c.c_size_t, F32P, F32P, c.c_size_t])):
            f = getattr(dll, name)
            f.restype, f.argtypes = result_type, argument_types
        self.dll = dll
        self.host = dll.ep_open(str(path).encode())
        self.parameters = []
        for n in range(dll.ep_parameter_count(self.host)):
            title = (c.c_uint16 * 128)()
            v3id = c.c_uint32()
            value = dll.ep_parameter_info(self.host, n, c.byref(v3id), title)
            name = bytes(title).decode("utf-16le").split("\0",1)[0]
            if not name.startswith("MIDI Ch. "):
                self.parameters.append((name, v3id.value, value))
    def set(self, parameter_id, value):
        return self.dll.ep_set(self.host, parameter_id, value)
    def render(self,events,duration):
        n = round(duration * RATE)
        out = np.zeros((2, n), np.float32)
        entries = []
        for event in events:
            status, note, value = event.bytes()
            entries.append(Midi(round(event.time * RATE), status, note, value))
        self.dll.ep_render(self.host, (Midi * len(entries))(*entries), len(entries),
                           out[0].ctypes.data_as(F32P), out[1].ctypes.data_as(F32P), n)
        return out
    def close(self):
        self.dll.ep_close(self.host)


class Audit:
    def __init__(self, bundle, output, native_path=None):
        self.bundle = bundle
        self.output = output
        # Pedalboard's high-level parameter scan resolves every value by
        # querying display strings. Use its native interface to retain distinct
        # parameters with identical titles and skip generated MIDI CC entries.
        host = pedalboard.VST3Plugin
        initialize = host.__set_initial_parameter_values__
        host.__set_initial_parameter_values__ = lambda self, values: None
        try:
            self.plugin = host(str(bundle), initialization_timeout=0)
        finally:
            host.__set_initial_parameter_values__ = initialize
        # Electric Pianos clears its VST3 state at a processing restart.
        # Use Pedalboard's in-place reset path without reloading the bundle.
        object.__setattr__(self.plugin, "_reload_type", ExternalPluginReloadType.ClearsAudioOnReset)
        parameters = self.live_parameters()
        self.parameter_names = {key: param.name for key, param in parameters.items()}
        self.defaults = self.snapshot(parameters)
        self.native = NativeVST3(native_path, bundle / "Contents/MacOS/Electric Pianos") if native_path else None
        if self.native:
            self.native_ids = named_parameters((name, identifier) for name, identifier, _ in self.native.parameters)
            if self.native_ids.keys() != self.defaults.keys():
                raise RuntimeError("The two VST3 hosts exposed different parameter sets")
        self.native_checks = {}
        self.pedalboard_fallbacks = set()
        self.configuration = None
        self.configurations = {}
        self.raw = {}
        self.phrases = {}
        self.reset_checks = {}

    def live_parameters(self):
        # Reacquire native objects after processing. A host reload invalidates
        # the old pointers, including those for duplicate display names.
        return named_parameters((param.name, param) for param in self.plugin._parameters
                                if not param.name.startswith("MIDI Ch. "))

    @staticmethod
    def snapshot(parameters):
        return {key: param.raw_value for key, param in parameters.items()}

    def configure(self, model, setting):
        if self.configuration == (model, setting):
            return
        parameters = self.live_parameters()
        values = dict(self.defaults)
        values.update(MODELS[model][2])
        if setting != "stock":
            values.update(DRY.get(model, {}))
        if setting in SETTINGS:
            values.update(SETTINGS[setting][0].get(model, {}))
        unknown = values.keys() - parameters.keys()
        if unknown:
            raise ValueError(f"VST3 is missing audit parameters: {sorted(unknown)}")
        for key, value in values.items():
            parameter = parameters[key]
            if abs(parameter.raw_value - value) > 1e-6:
                parameter.raw_value = value
            if abs(parameter.raw_value - value) > 1e-6:
                raise ValueError(f"{key} rejected normalized value {value}")
        self.configurations[f"{model} / {setting}"] = self.snapshot(parameters)
        if self.native:
            for key, value in values.items():
                actual = self.native.set(self.native_ids[key], value)
                if abs(actual - value) > 1e-6:
                    raise ValueError(f"Direct VST3 host rejected {key}={value}")
        self.configuration = (model, setting)

    def render(self, model, setting, events, duration, pedalboard_only=False):
        self.configure(model, setting)
        if self.native and not pedalboard_only and (model, setting) not in self.pedalboard_fallbacks:
            audio = self.native.render(events, duration)
        else:
            audio = self.plugin(events, sample_rate=RATE, num_channels=2, duration=duration,
                                buffer_size=BLOCK, reset=True)
        if model.startswith("Duo "):
            channel = int(model == "Duo Pianet")
            audio = np.repeat(audio[channel:channel + 1], 2, axis=0)
        if audio.shape != (2, round(duration * RATE)) or not np.isfinite(audio).all():
            raise RuntimeError(f"Invalid render for {model} / {setting}: {audio.shape}")
        if np.max(abs(audio)) < 1e-7:
            raise RuntimeError(f"Silent render for {model} / {setting}: {events}")
        return audio

    def capture(self, model, setting, notes):
        key = (model, setting)
        if key not in self.phrases:
            probe = messages(64, 80)
            before = self.render(model, setting, probe, END)
            audio = self.render(model, setting, phrase(), 2.25, pedalboard_only=True)
            after = self.render(model, setting, probe, END)
            delta = float(np.max(abs(before - after)))
            if self.native:
                reference = self.render(model, setting, probe, END, pedalboard_only=True)
                host_delta = float(np.max(abs(after - reference)))
                if host_delta > max(1e-7, 1e-5 * float(np.max(abs(reference)))):
                    print(f"Using Pedalboard for {model} / {setting}, host difference {host_delta:.7g}", flush=True)
                    self.pedalboard_fallbacks.add(key)
                    before = reference
                    after = self.render(model, setting, probe, END, pedalboard_only=True)
                    delta = float(np.max(abs(before - after)))
                self.native_checks[key] = host_delta
            self.reset_checks[key] = {"isolated_strike_sha256": digest(after), "max_sample_delta": delta}
            envelope = np.array([rms(x) for x in np.split(audio, 90, axis=1)])
            self.phrases[key] = (digest(audio), db(envelope / max(rms(audio), 1e-10)))
        for note in sorted(notes):
            for velocity in VELOCITIES:
                take = (*key, note, velocity)
                if take not in self.raw:
                    audio = self.render(*key, messages(note, velocity), END)
                    self.raw[take] = (digest(audio), strike_metrics(audio, note))

    def compare(self, setting, left, right, groups, notes):
        def take(model):
            return [self.raw[model, setting, n, v] for n in notes for v in VELOCITIES]

        a, b = take(left), take(right)
        av, bv = [np.array([take[1] for take in array]) for array in (a, b)]
        diff = abs(av - bv)
        spans = [np.diff(arr[:, 9].reshape(-1, 3)[:, [0, 2]], axis=1) for arr in (av, bv)]
        ap, bp = (self.phrases[model, setting] for model in (left, right))
        # Weight the phrase by activity. Silence in both takes carries no evidence.
        active = (ap[1] > -35) | (bp[1] > -35)
        values = np.array([
            np.median(diff[:, 0]), np.median(diff[:, 1]), np.nanmedian(diff[:, 2:7]),
            np.median(diff[:, 7]), np.median(diff[:, 8]), np.median(abs(spans[0] - spans[1])),
            np.median(diff[:, 10]), np.median(abs(np.maximum(ap[1][active], -60) -
                                                  np.maximum(bp[1][active], -60))),
        ])
        # Screening scales put different units on comparable axes. This is a
        # descriptive index, not an audibility threshold or hypothesis test.
        scales = [3, 12, 3, 4, 5, 4, .25, 4]
        return dict(setting=setting, model_a=left, model_b=right, groups="; ".join(groups),
                    notes=" ".join(map(str, notes)), strikes=len(a), exact_strikes=sum(
                        x[0] == y[0] for x, y in zip(a, b)), exact_phrase=ap[0] == bp[0],
                    stock_level_delta_dB=round(float(np.median(bv[:, 9] - av[:, 9])), 6),
                    score=round(float(np.mean(values / scales)), 6),
                    **{name: round(float(value), 6) for name, value in zip(METRICS, values)})

    def clips(self, comparisons):
        folder = self.output / "listening"
        folder.mkdir(exist_ok=True)
        manifest = []
        for index, row in enumerate(comparisons, 1):
            setting, a, b = (row[k] for k in ("setting", "model_a", "model_b"))
            notes = list(map(int, row["notes"].split()))
            sections = []
            for note in (notes[len(notes) // 5], notes[len(notes) // 2], notes[-1 - len(notes) // 5]):
                sections.append((f"MIDI {note}, velocity 80", messages(note, 80), END))
            sections.append(("Phrase", phrase(), 2.25))
            takes = []
            gains = []
            for label, events, duration in sections:
                for model in (a, b):
                    audio = self.render(model, setting, events, duration, pedalboard_only=duration != END)
                    reference = window(audio, .14, .36) if duration == END else window(audio, .04, 1.75)
                    gain = .07 / max(rms(reference), 1e-10)
                    takes.extend([audio * gain, np.zeros((2, RATE // 5))])
                    gains.append({"section": label, "model": model, "gain_dB": float(db(gain))})
            audio = np.concatenate(takes, axis=1)
            export_gain = min(1., .95 / np.max(abs(audio)))
            filename = f"{index:02d}-{setting}.wav"
            with wave.open(str(folder / filename), "wb") as wav:
                wav.setparams((2, 2, RATE, 0, "NONE", "not compressed"))
                wav.writeframes(np.rint(audio.T * (export_gain * 32767)).astype("<i2").tobytes())
            manifest.append(dict(file=filename, setting=setting, model_a=a, model_b=b,
                                 common_export_gain_dB=float(db(export_gain)), takes=gains))
        (folder / "index.json").write_text(json.dumps(manifest, indent=2) + "\n")


def named_parameters(entries):
    by_name = {}
    for name, entry in entries:
        by_name.setdefault(name, []).append(entry)
    return {(name if len(entries) == 1 else f"{name} [{i}]"): entry
            for name, entries in by_name.items() for i, entry in enumerate(entries, 1)}


def start_worker(bundle, output, native_path):
    global worker
    worker = Audit(bundle, output, native_path)


def capture_job(job):
    (model, setting), notes = job
    print(f"{setting}: {model} ({len(notes) * len(VELOCITIES)} strikes)", flush=True)
    worker.capture(model, setting, notes)
    key = (model, setting)
    results = {take: worker.raw.pop(take) for take in list(worker.raw)}
    return (results, key, worker.phrases.pop(key), worker.configurations.pop(f"{model} / {setting}"),
            worker.reset_checks.pop(key), worker.native_checks.pop(key, None), key in worker.pedalboard_fallbacks)


def pair_notes(left, right):
    a, ab, _ = MODELS[left]
    b, bb, _ = MODELS[right]
    low, high = max(a[0], b[0]), min(a[1], b[1])
    return sorted(set(np.rint(np.linspace(low, high, 8)).astype(int).tolist()) |
                  {note for note in (*ab, *bb) if low <= note <= high})


def write_csv(path, rows):
    with path.open("w", newline="") as out:
        writer = csv.DictWriter(out, fieldnames=rows[0])
        writer.writeheader()
        writer.writerows(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("plugin", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--native-vst3", type=Path, help="Optional direct VST3 probe library for the strike grid")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    pairs = {}
    for group, models in GROUPS.items():
        for pair in combinations(sorted(models), 2):
            pairs.setdefault(pair, []).append(group)
    jobs = [(setting, *pair, groups) for setting in ("stock", "dry") for pair, groups in pairs.items()]
    for setting, (_, models) in SETTINGS.items():
        jobs.extend((setting, a, b, pairs[a, b]) for a, b in combinations(sorted(models), 2))
    coverage = {}
    for setting, a, b, _ in jobs:
        notes = pair_notes(a, b)
        for model in (a, b):
            coverage.setdefault((model, setting), set()).update(notes)
    assert set(MODELS) == {model for model, _ in coverage}

    bundle = args.plugin.resolve()
    binary = bundle / "Contents/MacOS/Electric Pianos"
    binary_hash = sha256(binary.read_bytes()).hexdigest()
    audit = Audit(bundle, args.output, args.native_vst3)
    with ProcessPoolExecutor(max_workers=args.workers, initializer=start_worker,
                             initargs=(bundle, args.output, args.native_vst3)) as pool:
        pending = {pool.submit(capture_job, job): job[0] for job in coverage.items()}
        for future in as_completed(pending):
            try:
                strikes, key, phrase_result, settings, repeat, agreement, fallback = future.result()
            except Exception as error:
                print(f"Capture failed for {pending[future]}: {error}", flush=True)
                for job in pending:
                    job.cancel()
                raise
            if agreement is not None:
                audit.native_checks[key] = agreement
            if fallback:
                audit.pedalboard_fallbacks.add(key)
            print(f"Completed {key[1]} / {key[0]}", flush=True)
            audit.raw.update(strikes)
            audit.phrases[key] = phrase_result
            audit.reset_checks[key] = repeat
            audit.configurations[f"{key[0]} / {key[1]}"] = settings
    rows = [audit.compare(setting, a, b, groups, pair_notes(a, b)) for setting, a, b, groups in jobs]
    rows.sort(key=lambda row: (row["setting"], row["score"]))
    write_csv(args.output / "comparisons.csv", rows)
    strike_columns = ("attack_to_body_dB", "rise_ms", *(f"H{h}_dBc" for h in range(2, 7)),
                      "decay_dB", "release_dB", "body_dBFS", "stereo_width", "peak_dBFS")
    write_csv(args.output / "strikes.csv", [
        dict(model=model, setting=setting, midi=note, velocity=velocity, sha256=hash_value,
             **{name: (round(float(value), 6) if np.isfinite(value) else "")
                for name, value in zip(strike_columns, metrics)})
        for (model, setting, note, velocity), (hash_value, metrics) in audit.raw.items()
    ])
    closest = sorted([row for row in rows if row["setting"] == "stock"], key=lambda row: row["score"])[:12]
    tested_pairs = {(row["model_a"], row["model_b"]) for row in closest}
    controls = [row for row in rows if row["setting"] not in ("stock", "dry") and
                (row["model_a"], row["model_b"]) in tested_pairs]
    # Include a control that can collapse two outputs, the most revealing
    # alternate setting for each close pair, and StereoVibe's defining board.
    special = [row for row in rows if row["setting"] in ("peterson", "stereovibe-tremolo") and
               (row["exact_phrase"] or row["setting"] == "stereovibe-tremolo")]
    strongest = {}
    for row in sorted(controls, key=lambda row: row["score"], reverse=True):
        strongest.setdefault((row["model_a"], row["model_b"]), row)
    selected = {tuple(row[k] for k in ("setting", "model_a", "model_b")): row
                for row in closest + list(strongest.values()) + special}
    audit.clips(selected.values())
    if sha256(binary.read_bytes()).hexdigest() != binary_hash:
        raise RuntimeError("VST3 executable changed during capture")
    (args.output / "provenance.json").write_text(json.dumps(
        dict(bundle=str(args.plugin.resolve()), sha256=binary_hash,
             pedalboard=pedalboard.__version__, numpy=np.__version__, rate_hz=RATE,
             buffer_frames=BLOCK, velocities=VELOCITIES, pair_count=len(pairs), comparison_rows=len(rows),
             strike_count=len(audit.raw), modes=MODELS, groups=GROUPS, defaults=audit.defaults,
             host_parameter_names=audit.parameter_names,
             settings=audit.configurations, phrase_host="Pedalboard",
             strike_host=("Direct VST3" if args.native_vst3 else "Pedalboard"),
             reset_method=("VST3 setProcessing false/true per strike" if args.native_vst3 else
                           "Pedalboard ClearsAudioOnReset with reset=True"),
             host_validation_max_sample_delta={f"{k[0]} / {k[1]}": v for k, v in audit.native_checks.items()},
             pedalboard_fallbacks=sorted(f"{k[0]} / {k[1]}" for k in audit.pedalboard_fallbacks),
             repeat_checks={f"{k[0]} / {k[1]}": v for k, v in audit.reset_checks.items()},
             phrase_sha256={f"{k[0]} / {k[1]}": v[0] for k, v in audit.phrases.items()}),
        indent=2) + "\n")
    print(f"{len(pairs)} pairs, {len(rows)} comparisons, {len(audit.raw)} strikes")


if __name__ == "__main__":
    main()
