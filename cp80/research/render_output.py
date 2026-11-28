"""Render a single dry note at the patch send, jack or balanced line observation point."""

import argparse
import subprocess
import tempfile
from pathlib import Path

import soundfile as sf
from circuit import NEUTRAL, calibrate, response
from panel import Panel, loaded_bus
from scipy.signal import resample_poly


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("probe", type=Path)
    p.add_argument("node", choices=["patch", "jack", "line"])
    p.add_argument("midi", type=int, choices=range(21, 109), metavar="MIDI")
    p.add_argument("velocity", type=float)
    p.add_argument("hold", type=float)
    p.add_argument("output", type=Path)
    args = p.parse_args()
    rate = 48000
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory, "force.wav")
        subprocess.run(
            [args.probe, *map(str, (args.midi, args.velocity, rate, args.hold)), path],
            check=True,
        )
        force, integration_rate = sf.read(path)
    hz, outputs = response(**calibrate(), **NEUTRAL)
    transfer = outputs[:, ["patch", "jack", "line"].index(args.node)] * loaded_bus(
        hz, args.midi >= 103
    )
    observation = Panel(hz, transfer, integration_rate).apply(force)
    sf.write(
        args.output, 0.14 * resample_poly(observation, 1, 4), rate, subtype="FLOAT"
    )


if __name__ == "__main__":
    main()
