"""Compare the two Clavinet boards at full travel and probe the small-signal assumption."""
import subprocess
import tempfile
from pathlib import Path

import numpy as np

from compare import HERE, spice


def distortion(directory, board, amplitude):
    # Select CA on each board. Leave the other pickup at zero transient excitation.
    lines = (HERE / f"{board}.cir").read_text().splitlines()
    for i, line in enumerate(lines):
        if line.startswith("Vcentre "):
            source = " ".join(line.split()[:3])
            lines[i] = f"{source} DC 0 SIN(0 {amplitude} 500)"
    deck = "\n".join(lines).replace(".end", """.control
set wr_singlescale
set numdgt=14
tran 5u 80m 40m 5u
wrdata level.dat v(jack)
quit
.endc
.end
""")
    (directory / "level.cir").write_text(deck)
    subprocess.run(["ngspice", "-b", "level.cir"], cwd=directory, check=True,
                   stdout=subprocess.DEVNULL)
    time, volts = np.loadtxt(directory / "level.dat").T
    # Last sixteen complete cycles, uniformly sampled from the adaptive solver.
    y = np.interp(0.048 + np.arange(6400) / 200000, time, volts)
    partial = abs(np.fft.rfft(y))[16 * np.arange(1, 11)] * 2 / len(y)
    return partial[0], 100 * np.linalg.norm(partial[1:]) / partial[0]


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as scratch:
        directory = Path(scratch)
        for board in ("d6", "e7"):
            for volume in (1.0, 0.5):
                f, h = spice(directory, "CA", 15, board, volume)
                print(f"{board} volume {volume:.1f}, gain dB at 100 / 500 / 1600 / 10000 Hz",
                      np.round(np.interp([100, 500, 1600, 10000], f, 20*np.log10(abs(h))), 2))
            for millivolts in (0.1, 0.4, 1, 4, 10):
                v, thd = distortion(directory, board, millivolts * 0.001)
                print(f"{board} 500 Hz | {millivolts:.1f} mVpk pickup | {v:.4f} Vpk jack H1 | {thd:.2f}% THD")
