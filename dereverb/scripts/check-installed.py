#!/usr/bin/env python3
"""Minimal offline CLAP host: checks the installed processor, not a DAW session.

No third-party Python packages. Exercises plugin lifecycle, parameter flushing,
variable block sizes, real inference, resampling, and host latency reporting.
"""
import ctypes as c
import math
from pathlib import Path
import sys
import time

P = c.c_void_p
U = c.c_uint32
F = c.CFUNCTYPE


class Version(c.Structure):
    _fields_ = [("major", U), ("minor", U), ("revision", U)]


class Entry(c.Structure):
    _fields_ = [("version", Version), ("init", F(c.c_bool, c.c_char_p)),
                ("deinit", F(None)), ("factory", F(P, c.c_char_p))]


class Host(c.Structure):
    _fields_ = [("version", Version), ("data", P), ("name", c.c_char_p),
                ("vendor", c.c_char_p), ("url", c.c_char_p), ("version_text", c.c_char_p),
                ("extension", F(P, P, c.c_char_p)), ("restart", F(None, P)),
                ("process", F(None, P)), ("callback", F(None, P))]


class Plugin(c.Structure):
    _fields_ = [("descriptor", P), ("data", P), ("init", F(c.c_bool, P)),
                ("destroy", F(None, P)), ("activate", F(c.c_bool, P, c.c_double, U, U)),
                ("deactivate", F(None, P)), ("start", F(c.c_bool, P)), ("stop", F(None, P)),
                ("reset", F(None, P)), ("process", F(c.c_int32, P, P)),
                ("extension", F(P, P, c.c_char_p)), ("main", F(None, P))]


class Factory(c.Structure):
    _fields_ = [("count", F(U, P)), ("descriptor", F(P, P, U)),
                ("create", F(P, P, P, c.c_char_p))]


class EventHeader(c.Structure):
    _fields_ = [("size", U), ("time", U), ("space", c.c_uint16),
                ("type", c.c_uint16), ("flags", U)]


class ParamValue(c.Structure):
    _fields_ = [("header", EventHeader), ("id", U), ("cookie", P),
                ("note_id", c.c_int32), ("port", c.c_int16), ("channel", c.c_int16),
                ("key", c.c_int16), ("value", c.c_double)]


class Transport(c.Structure):
    _fields_ = [("header", EventHeader), ("flags", U),
                ("song_beats", c.c_int64), ("song_seconds", c.c_int64),
                ("tempo", c.c_double), ("tempo_inc", c.c_double),
                ("loop_start_beats", c.c_int64), ("loop_end_beats", c.c_int64),
                ("loop_start_seconds", c.c_int64), ("loop_end_seconds", c.c_int64),
                ("bar_start", c.c_int64), ("bar_number", c.c_int32),
                ("numerator", c.c_uint16), ("denominator", c.c_uint16)]


class State(c.Structure):
    _fields_ = [("save", F(c.c_bool, P, P)), ("load", F(c.c_bool, P, P))]


class Stream(c.Structure):
    _fields_ = [("data", P), ("transfer", F(c.c_int64, P, P, c.c_uint64))]


class ParamInfo(c.Structure):
    _fields_ = [("id", U), ("flags", U), ("cookie", P), ("name", c.c_char * 256),
                ("module", c.c_char * 1024), ("min", c.c_double), ("max", c.c_double),
                ("default", c.c_double)]


class Params(c.Structure):
    _fields_ = [("count", F(U, P)), ("info", F(c.c_bool, P, U, P)),
                ("value", P), ("to_text", P), ("from_text", P), ("flush", F(None, P, P, P))]


class InputEvents(c.Structure):
    _fields_ = [("data", P), ("size", F(U, P)), ("get", F(P, P, U))]


class OutputEvents(c.Structure):
    _fields_ = [("data", P), ("push", F(c.c_bool, P, P))]


class Audio(c.Structure):
    _fields_ = [("f32", c.POINTER(c.POINTER(c.c_float))), ("f64", P),
                ("channels", U), ("latency", U), ("constant", c.c_uint64)]


class Process(c.Structure):
    _fields_ = [("steady_time", c.c_int64), ("frames", U), ("transport", P),
                ("inputs", c.POINTER(Audio)), ("outputs", c.POINTER(Audio)),
                ("num_inputs", U), ("num_outputs", U),
                ("in_events", c.POINTER(InputEvents)), ("out_events", c.POINTER(OutputEvents))]


def main():
    bundle = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.home() / "Library/Audio/Plug-Ins/CLAP/Dereverb.clap"
    library = c.CDLL(str(bundle / "Contents/MacOS/Dereverb"))
    entry = Entry.in_dll(library, "clap_entry")
    assert entry.init(str(bundle).encode()), "CLAP entry initialization failed"
    host = Host(Version(1, 2, 0), None, b"Dereverb offline check", b"Fergler", b"", b"1",
                F(P, P, c.c_char_p)(lambda *_: None), *[F(None, P)(lambda *_: None) for _ in range(3)])
    factory_ptr = entry.factory(b"clap.plugin-factory")
    factory = c.cast(factory_ptr, c.POINTER(Factory)).contents
    assert factory.count(factory_ptr) == 1
    plugin_ptr = factory.create(factory_ptr, c.byref(host), b"com.fergler.dereverb")
    assert plugin_ptr, "CLAP factory did not create Dereverb"
    plugin = c.cast(plugin_ptr, c.POINTER(Plugin)).contents
    assert plugin.init(plugin_ptr), "CLAP plugin initialization failed"
    params = c.cast(plugin.extension(plugin_ptr, b"clap.params"), c.POINTER(Params)).contents
    parameters = {}
    for index in range(params.count(plugin_ptr)):
        parameter = ParamInfo()
        assert params.info(plugin_ptr, index, c.byref(parameter))
        parameters[parameter.name] = parameter
    info = parameters[b"Strength"]
    # NIH-plug exposes continuous CLAP values normalized to 0..1; text is percent.
    assert info.name == b"Strength" and (info.min, info.max, info.default) == (0, 1, 1)
    guard = parameters[b"Transient Guard"]
    assert (guard.min, guard.max, guard.default) == (0, 1, 1)
    quality = parameters[b"Quality"]
    assert quality.name == b"Quality" and (quality.min, quality.max, quality.default) == (0, 1, 0)
    assert not quality.flags & (1 << 5), "Quality must not be automatable"
    engine = parameters[b"Engine"]
    assert (engine.min, engine.max, engine.default) == (0, 2, 0)
    assert not engine.flags & (1 << 5), "Engine must not be automatable"
    no_events = InputEvents(None, F(U, P)(lambda _: 0), F(P, P, U)(lambda *_: None))
    out_events = OutputEvents(None, F(c.c_bool, P, P)(lambda *_: True))
    event = ParamValue(EventHeader(c.sizeof(ParamValue), 0, 0, 5, 0), info.id, info.cookie, -1, -1, -1, -1, 0.0)
    change = InputEvents(None, F(U, P)(lambda _: 1), F(P, P, U)(lambda *_: c.addressof(event)))
    params.flush(plugin_ptr, c.byref(change), c.byref(out_events))
    latency_ptr = plugin.extension(plugin_ptr, b"clap.latency")
    latency_fn = c.cast(latency_ptr, c.POINTER(F(U, P))).contents
    sources = [(c.c_float * 512)() for _ in range(2)]
    destinations = [(c.c_float * 512)() for _ in range(2)]
    source_ptrs = (c.POINTER(c.c_float) * 2)(*sources)
    destination_ptrs = (c.POINTER(c.c_float) * 2)(*destinations)
    inputs, outputs = Audio(source_ptrs, None, 2, 0, 0), Audio(destination_ptrs, None, 2, 0, 0)
    block = Process(0, 0, None, c.pointer(inputs), c.pointer(outputs), 1, 1, c.pointer(no_events), c.pointer(out_events))
    transport = Transport()
    transport.header = EventHeader(c.sizeof(Transport), 0, 0, 9, 0)
    transport.flags = 1 << 4  # Playing throughout every mode change.
    block.transport = c.addressof(transport)
    for rate in [44100, 48000, 96000, 192000]:
        assert plugin.activate(plugin_ptr, rate, 1, 512), f"activation failed at {rate}"
        assert plugin.start(plugin_ptr)
        latency = latency_fn(plugin_ptr)
        base = 2880
        expected = base if rate == 48000 else math.ceil(64 + (base + 64) * rate / 48000) + math.ceil(rate / 48000) + 2
        assert latency == expected, (rate, latency, expected)
        wet_levels = []
        steady_time = 0
        for algorithm, mode, name in [
            (0, 0, "Live"), (0, 1, "High Quality"), (1, 0, "NARA-WPE 10 ms"),
            (1, 1, "NARA-WPE 20 ms"), (2, 1, "DTLN Denoise"),
        ]:
            event.id, event.cookie, event.value = engine.id, engine.cookie, algorithm
            params.flush(plugin_ptr, c.byref(change), c.byref(out_events))
            event.id, event.cookie, event.value = quality.id, quality.cookie, mode
            params.flush(plugin_ptr, c.byref(change), c.byref(out_events))
            event.id, event.cookie = info.id, info.cookie
            position, calls = 0, 0
            squared_error, squared_signal, wet_energy, processing_time = 0.0, 0.0, 0.0, 0.0
            for amount in [100.0, 0.0]:
                event.value = amount / 100.0
                params.flush(plugin_ptr, c.byref(change), c.byref(out_events))
                # Render the newly selected wet mode during playback without a reset.
                if amount == 0:
                    plugin.reset(plugin_ptr)
                position = 0
                total = rate // 2 + latency
                while position < total:
                    count = min([1, 17, 128, 257, 511][calls % 5], total - position)
                    for i in range(count):
                        value = 0.3 * math.sin(math.tau * 997 * (position + i) / rate)
                        sources[0][i] = sources[1][i] = value
                    block.frames, block.steady_time = count, steady_time
                    start = time.perf_counter()
                    assert plugin.process(plugin_ptr, c.byref(block)) != 0, "CLAP_PROCESS_ERROR"
                    processing_time += time.perf_counter() - start
                    for i in range(count):
                        value = destinations[0][i]
                        assert math.isfinite(value) and abs(value) < 1.0
                        assert abs(value - destinations[1][i]) < 1e-5, "stereo channels diverged"
                        if position + i > latency + max(512, rate // 100):
                            original = 0.3 * math.sin(math.tau * 997 * (position + i - latency) / rate)
                            if amount == 0:
                                squared_error += (value - original) ** 2
                                squared_signal += original ** 2
                            else:
                                wet_energy += value ** 2
                    position += count
                    steady_time += count
                    calls += 1
            assert latency_fn(plugin_ptr) == latency, "latency changed during playback"
            assert wet_energy < squared_signal * 0.95, "Strength did not engage the selected engine"
            wet_levels.append(10 * math.log10(max(wet_energy / squared_signal, 1e-30)))
            error = math.sqrt(squared_error / squared_signal)
            assert error < 0.001, f"{rate} Hz reconstruction error: {error}"
            print(f"{name} {rate} Hz: latency {latency} samples ({latency * 1000 / rate:.2f} ms), aligned error {error:.2g}, processing RTF {processing_time / (2 * total / rate):.3f}")
            if rate == 48000:
                # Exercise the new curve through the installed host parameter API.
                event.value = 1
                params.flush(plugin_ptr, c.byref(change), c.byref(out_events))
                weights = [parameters[b"Weight: " + label] for label in
                           [b"quietest", b"quiet", b"middle", b"loud", b"loudest"]]
                for parameter in weights:
                    event.id, event.cookie, event.value = parameter.id, parameter.cookie, 0
                    params.flush(plugin_ptr, c.byref(change), c.byref(out_events))
                plugin.reset(plugin_ptr)
                worst = 0.0
                for position in range(0, 12000, 480):
                    for i in range(480):
                        sources[0][i] = sources[1][i] = 0.3 * math.sin(math.tau * 997 * (position + i) / rate)
                    block.frames, block.steady_time = 480, steady_time
                    assert plugin.process(plugin_ptr, c.byref(block)) != 0
                    steady_time += 480
                    for i in range(480):
                        original = 0.0 if position + i < latency else 0.3 * math.sin(math.tau * 997 * (position + i - latency) / rate)
                        worst = max(worst, abs(destinations[0][i] - original))
                assert worst < 3e-6, f"{name}: zero curve did not produce aligned dry audio: {worst}"
                for parameter in weights:
                    event.id, event.cookie, event.value = parameter.id, parameter.cookie, 1
                    params.flush(plugin_ptr, c.byref(change), c.byref(out_events))
        assert len({round(level, 1) for level in wet_levels[:4]}) == 4, (rate, wet_levels)
        plugin.stop(plugin_ptr)
        plugin.deactivate(plugin_ptr)
    curve_values = {b"Engine": 2, b"Transient Guard": 1, b"Level reference": 1, b"Weight: quietest": 0.05,
                    b"Weight: quiet": 0.2, b"Weight: middle": 0.4,
                    b"Weight: loud": 0.8, b"Weight: loudest": 1.0}
    for name, value in curve_values.items():
        parameter = parameters[name]
        event.id, event.cookie, event.value = parameter.id, parameter.cookie, value
        params.flush(plugin_ptr, c.byref(change), c.byref(out_events))
    state = c.cast(plugin.extension(plugin_ptr, b"clap.state"), c.POINTER(State)).contents
    saved = bytearray()
    def write(_, data, length):
        saved.extend(c.string_at(data, length))
        return length
    writer = Stream(None, F(c.c_int64, P, P, c.c_uint64)(write))
    assert state.save(plugin_ptr, c.byref(writer))
    for name in curve_values:
        parameter = parameters[name]
        event.id, event.cookie, event.value = parameter.id, parameter.cookie, 0
        params.flush(plugin_ptr, c.byref(change), c.byref(out_events))
    event.id, event.cookie, event.value = quality.id, quality.cookie, 0
    params.flush(plugin_ptr, c.byref(change), c.byref(out_events))
    offset = 0
    def read(_, data, length):
        nonlocal offset
        part = bytes(saved[offset:offset + length])
        c.memmove(data, part, len(part))
        offset += len(part)
        return len(part)
    reader = Stream(None, F(c.c_int64, P, P, c.c_uint64)(read))
    assert state.load(plugin_ptr, c.byref(reader))
    restored = c.c_double()
    get_value = F(c.c_bool, P, U, c.POINTER(c.c_double))(params.value)
    assert get_value(plugin_ptr, quality.id, c.byref(restored)) and restored.value == 1, "Quality preset state did not restore"
    for name, expected in curve_values.items():
        assert get_value(plugin_ptr, parameters[name].id, c.byref(restored))
        assert abs(restored.value - expected) < 1e-6, f"{name!r}: curve preset state did not restore"
    plugin.destroy(plugin_ptr)
    entry.deinit()
    print("Installed CLAP live mode switching, fixed latency, preset restore, variable blocks, DPDFNet, NARA-WPE, and DTLN verified.")


if __name__ == "__main__":
    main()
