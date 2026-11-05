#!/usr/bin/env python3
"""Check installed Tape Stop parameter/state compatibility without a DAW.

Usage: python3 'Tape Stop/scripts/verify-clap.py' '/path/to/Tape Stop.clap'
The existing Chordboard harness supplies the common CLAP ABI and state streams.
"""
import importlib.util
import json
import math
from pathlib import Path
import sys

source = Path(__file__).resolve().parents[2] / 'chordboard/scripts/verify-clap.py'
spec = importlib.util.spec_from_file_location('pleasant_clap_harness', source)
abi = importlib.util.module_from_spec(spec)
spec.loader.exec_module(abi)


def run_midi(harness, events=(), frames=256):
    """Process silent audio so a triggered tape can be checked without a DAW."""
    C = abi.C
    captured = []
    inputs = abi.InEvents(None, abi.F(abi.U, abi.P)(lambda _: len(events)),
                          abi.F(abi.P, abi.P, abi.U)(lambda _, i: C.addressof(events[i])))
    def push(_, event):
        header = C.cast(event, C.POINTER(abi.Header)).contents
        captured.append(C.string_at(event, header.size))
        return True
    outputs = abi.OutEvents(None, abi.F(abi.B, abi.P, abi.P)(push))
    samples = [(C.c_float * frames)() for _ in range(2)]
    pointers = (C.POINTER(C.c_float) * 2)(*samples)
    audio = abi.Audio(pointers, None, 2, 0, 0)
    process = abi.Process(harness.time, frames, None, C.pointer(audio), C.pointer(audio),
                          1, 1, C.pointer(inputs), C.pointer(outputs))
    assert harness.plugin.process(harness.ptr, C.byref(process)) != 0
    harness.time += frames
    result = list(map(abi.decode, captured))
    assert all(0 <= e[1] < frames for e in result), 'out-of-block MIDI output'
    assert [e[1] for e in result] == sorted(e[1] for e in result), 'unordered MIDI output'
    return result


def verify_midi_tape(harness, defaults):
    state = json.loads(json.dumps(defaults))
    for key, value in {'auto_restart': False, 'midi_assign': 1, 'override_note': 0,
                       'time_sync': True, 'return_sec': 0.05}.items():
        abi.state_value(state, key, value)
    harness.load_state(state)
    source = [(0, 0x90, 0, 100), (1000, 0x90, 60, 100), (2000, 0x90, 64, 100),
              (3000, 0x80, 60, 0), (4000, 0x80, 64, 0), (6000, 0x80, 0, 0)]
    output = []
    for start in range(0, 40000, 256):
        events = [abi.midi(at - start, status, note, vel) for at, status, note, vel in source
                  if start <= at < start + 256]
        output.extend((e[0], e[1] + start, *e[2:]) for e in run_midi(harness, events))
    notes = [e for e in output if e[0] in ('on', 'off')]
    assert [(e[0], e[3]) for e in notes] == [('on', 0), ('on', 60), ('on', 64),
                                           ('off', 60), ('off', 64), ('off', 0)], notes
    assert all(actual[1] > incoming[0] for actual, incoming in zip(notes[1:], source[1:])), notes
    bends = [(e[4] << 7) | e[3] for e in output if e[0] == 'midi' and e[2] == 0xe0]
    assert min(bends) < 8192 and max(bends) > 8192 and bends[-1] == 8192, bends
    # Toggle off during an active brake: release output and discard delayed notes.
    harness.load_state(state)
    run_midi(harness, [abi.midi(0, 0x90, 0, 100), abi.midi(128, 0x90, 60, 100)])
    events = run_midi(harness, [harness.parameter('Affect MIDI', 0, 64),
                                abi.midi(100, 0x91, 67, 100)])
    assert any(e[0] == 'off' and e[1] == 64 and e[3] == 0 for e in events), events
    assert any(e[0] == 'on' and e[1] == 100 and e[3] == 67 for e in events), events


def verify(path):
    if path.is_dir():
        path = path / 'Contents/MacOS/Tape Stop'
    harness = abi.Harness(path, b'com.fergler.tape-stop')
    try:
        defaults = harness.save_state()
        assert defaults['params']['time_sync'] == {'bool': False}
        assert defaults['params']['tempo_sync'] == {'bool': True}
        assert defaults['params']['affect_midi'] == {'bool': True}
        for key in ['drop_division', 'return_division']:
            assert defaults['params'][key] == {'i32': 13}
        state = json.loads(json.dumps(defaults))
        settings = {'affect_midi': False, 'time_sync': True, 'tempo_sync': False, 'tempo': 87.0,
                    'drop_division': 8, 'return_division': 15,
                    'xy_x_axis': 1, 'xy_mode': 1,
                    'drop_time': 0.1374, 'return_sec': 0.9231}
        for key, value in settings.items():
            abi.state_value(state, key, value)
        harness.load_state(state)
        saved = harness.save_state()
        for key, expected in settings.items():
            actual = next(iter(saved['params'][key].values()))
            assert math.isclose(actual, expected, abs_tol=1e-6), (key, actual, expected)
        harness.load_state(saved)
        assert harness.save_state()['params'] == saved['params'], 'state roundtrip changed'
        # Load a legacy preset into the same previously synced instance.
        legacy = json.loads(json.dumps(defaults))
        for key in settings:
            if key not in ['drop_time', 'return_sec']:
                del legacy['params'][key]
        abi.state_value(legacy, 'drop_time', 0.1374)
        harness.load_state(legacy)
        restored = harness.save_state()['params']
        for key in settings:
            if key not in ['drop_time', 'return_sec']:
                assert restored[key] == defaults['params'][key], ('legacy default', key)
        assert math.isclose(restored['drop_time']['f32'], 0.1374, abs_tol=1e-6)
        harness.run()  # Untriggered audio remains passthrough after restoration.
        # Power-off preserves both MIDI timestamps and audio passthrough.
        harness.run([harness.parameter('Power', 0)])
        for enabled in [0, 1]:
            harness.run([harness.parameter('Affect MIDI', enabled)])
            events = harness.run([abi.midi(12, 0x92, 60, 100),
                                  abi.midi(24, 0xb2, 74, 80),
                                  abi.midi(80, 0x82, 60, 40)])
            assert [(e[0], e[1]) for e in events] == [('on', 12), ('midi', 24), ('off', 80)], events
            assert events[0][2:4] == (2, 60) and events[-1][2:4] == (2, 60), events
        verify_midi_tape(harness, defaults)
    finally:
        harness.close()
    print('Tape Stop CLAP passed: state, legacy defaults, audio/MIDI passthrough, tape timing, pitch, and toggle cleanup.')


if __name__ == '__main__':
    assert len(sys.argv) == 2, __doc__
    verify(Path(sys.argv[1]).expanduser().resolve())
