"""Compare the Rust panel against a literal 73-coil ngspice AC circuit."""

import io
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np

probe = Path(sys.argv[1]).resolve()
template = Path(__file__).with_name("stage.cir").read_text()
groups = []
for g, count in enumerate([4] + [3] * 23):
    lo = "0" if g == 0 else f"g{g}"
    hi = "harp" if g == 23 else f"g{g + 1}"
    for i in range(count):
        # One moving pickup in the bass group. 4 V / 4 gives 1 V open circuit.
        groups.append(f"X{g}_{i} {hi} {lo} pickup amplitude={4 if g == i == 0 else 0}")
worst = 0.0
for fs in [44100, 48000, 96000]:
    data = np.genfromtxt(
        io.StringIO(subprocess.check_output([probe, "sweep", str(fs)], text=True)),
        names=True,
        delimiter=",",
    )
    for vol in [0.1, 0.5, 1.0]:
        for bass in [0, 0.5, 1.0]:
            bottom = 10000 * vol**3
            deck = template.replace("@HARP@", "\n".join(groups))
            for name, value in [
                ("BASS", 50000 * (1 - bass) ** 3),
                ("UPPER", 10000 - bottom),
                ("LOWER", bottom),
                ("CABLE_PF", 300),
            ]:
                deck = deck.replace("@" + name + "@", str(max(value, 1e-6)))
            with tempfile.TemporaryDirectory() as wd:
                Path(wd, "stage.cir").write_text(deck)
                subprocess.run(
                    ["ngspice", "-b", "stage.cir"],
                    cwd=wd,
                    check=True,
                    stdout=subprocess.DEVNULL,
                )
                ac = np.loadtxt(Path(wd, "sweep.txt"))
            rows = data[(data["volume"] == vol) & (data["bass"] == bass)]
            analog = np.interp(np.log(rows["hz"]), np.log(ac[:, 0]), ac[:, 1])
            err = abs(rows["db"] - analog)
            worst = max(worst, err.max())
    print(f"{fs} Hz complete")
print(f"Maximum error across 27 sweeps: {worst:.4f} dB")
if worst > 0.5:
    raise SystemExit("0.5 dB limit exceeded")
