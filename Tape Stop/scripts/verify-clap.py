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


def verify(path):
    if path.is_dir():
        path = path / 'Contents/MacOS/Tape Stop'
    harness = abi.Harness(path, b'com.fergler.tape-stop')
    try:
        defaults = harness.save_state()
        assert defaults['params']['time_sync'] == {'bool': False}
        assert defaults['params']['tempo_sync'] == {'bool': True}
        for key in ['drop_division', 'return_division']:
            assert defaults['params'][key] == {'i32': 13}
        state = json.loads(json.dumps(defaults))
        settings = {'time_sync': True, 'tempo_sync': False, 'tempo': 87.0,
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
    finally:
        harness.close()
    print('Tape Stop CLAP passed: fine times, performance-control state roundtrip, legacy defaults, audio passthrough.')


if __name__ == '__main__':
    assert len(sys.argv) == 2, __doc__
    verify(Path(sys.argv[1]).expanduser().resolve())
