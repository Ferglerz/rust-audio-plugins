"""Exercise the biased, tremolo-off CP-80 TR circuit against Yamaha service pp. 11–12."""

import subprocess
import tempfile
from pathlib import Path

import numpy as np
from scipy.optimize import brentq

DECK = Path(__file__).with_name("tr.cir")


NEUTRAL = {"bass": 0.30, "mid": 0.30, "treble": 0.50}


def response(pickup=None, *, deck_file=DECK, pickup_network=None, **settings):
    deck = deck_file.read_text()
    if pickup_network is not None:
        deck = deck.replace("Vin in 0 0 AC 1", pickup_network)
    if pickup is not None:
        # A voltage source in series with a ceramic capacitance for each key.
        branches = ["Cmix lower in 22n"]
        for key in range(1, 89):
            bus = "lower" if key <= 82 else "in"
            branches.append(f"V{key} k{key} 0 0 AC {int(key == pickup)}")
            branches.append(f"Cp{key} k{key} {bus} 1n")
        branches.extend(["Rleak1 lower 0 1e13", "Rleak2 in 0 1e13"])
        deck = deck.replace("Vin in 0 0 AC 1", "\n".join(branches))
    edits = "\n".join(f".param {key}={value:.12g}" for key, value in settings.items())
    deck = deck.replace(".model fet", edits + "\n.model fet")
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory, "tr.cir")
        path.write_text(deck)
        subprocess.run(
            ["ngspice", "-b", path],
            cwd=directory,
            check=True,
            stdout=subprocess.DEVNULL,
        )
        data = np.loadtxt(Path(directory, "response.txt"))
    return data[:, 0], data[:, 1::2] + 1j * data[:, 2::2]


def level(result, frequency, output):
    hz, voltage = result
    # Source is -32 dBm on Yamaha's meter. The voltage ratio cancels its reference.
    return -32 + np.interp(
        np.log(frequency), np.log(hz), 20 * np.log10(abs(voltage[:, output]))
    )


def calibrate(measure=response):
    trim = brentq(lambda v: level(measure(trim=v), 500, 0) + 29, 0.001, 0.999)
    cell = brentq(lambda v: level(measure(trim=trim, cell=v), 500, 1) + 31, 100, 1e8)
    return {"trim": trim, "cell": cell}


def checks():
    settings = calibrate()
    print(
        f"VR12 fraction {settings['trim']:.5f}, effective optical branch {settings['cell']:.0f} ohm"
    )
    neutral = NEUTRAL
    cases = [
        ("step 2 PO", {}, 500, 0, -29, 0.5),
        ("step 3 JO1", {}, 500, 1, -31, 0.1),
        ("step 4 JO1", {}, 50, 1, -17, 1.5),
        ("step 4 BO1", {}, 50, 2, -19, 1.5),
        ("step 5 bass min", {"bass": 0}, 50, 1, -34, 1.5),
        ("step 6 JO1", {}, 5000, 1, -23, 1.5),
        ("step 6 BO1", {}, 5000, 2, -23, 1.5),
        ("step 7 treble min", {"treble": 0}, 5000, 1, -35.5, 1.5),
        ("step 8 patched through", {"patchlink": 0.1}, 500, 1, -31, 1),
        ("step 9 line, volume min", {"volume": 0}, 500, 2, -31, 1.5),
        ("step 10 medium", neutral, 5000, 1, -29.5, 1.5),
        ("step 10 high", dict(neutral, br=2), 5000, 1, -24, 1.5),
        ("step 10 low", dict(neutral, br=0), 5000, 1, -41.5, 1.5),
    ]
    passed = True
    for name, controls, hz, node, target, tolerance in cases:
        measured = level(response(**settings, **controls), hz, node)
        good = abs(measured - target) <= tolerance
        print(
            f"{name:26} {measured:7.2f} dBm   target {target:5.1f} ±{tolerance}  {'pass' if good else 'MISS'}"
        )
        passed &= good
    mute = level(response(**settings, volume=0), 500, 1)
    print(f"step 9 muted jack {mute:.1f} dBm (maximum -70)")
    opened = level(response(**settings, patchlink=1e12), 500, 1)
    all_min = level(response(**settings, volume=0, bass=0, mid=0, treble=0), 500, 1)
    print(
        f"step 8 open patch {opened:.1f} dBm, step 9 all controls down {all_min:.1f} dBm"
    )
    return passed and max(mute, opened, all_min) <= -70


if __name__ == "__main__":
    raise SystemExit(0 if checks() else 1)
