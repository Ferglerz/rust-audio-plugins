"""Measure the Rust circuit impulse against independent full-network ngspice AC sweeps."""

import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
import soundfile as sf
from circuit import calibrate, response


def check(probe):
    calibrated = calibrate()
    worst = 0.0
    phase = 0.0
    sweeps = 0
    with tempfile.TemporaryDirectory() as directory:
        for bass, mid, treble, volume in [
            (0.5, 0.5, 0.5, 1),
            (1, 1, 1, 1),
            (0.15, 0.27, 0.2, 0.55),
        ]:
            for br in [0, 1, 2]:
                for key in [22, 83]:
                    hz, v = response(
                        pickup=key,
                        **calibrated,
                        bass=bass ** (-np.log2(0.3)),
                        mid=mid ** (-np.log2(0.3)),
                        treble=treble,
                        volume=volume**3,
                        br=br,
                    )
                    select = (hz >= 40) & (hz <= 10000)
                    h = hz[select]
                    for fs in [44100, 48000, 96000]:
                        path = Path(directory, "impulse.wav")
                        subprocess.run(
                            [
                                probe,
                                "panel",
                                *map(
                                    str,
                                    (fs, bass, mid, treble, br, volume, int(key >= 83)),
                                ),
                                path,
                            ],
                            check=True,
                        )
                        x, rate = sf.read(path)
                        if not np.isfinite(x).all():
                            raise ValueError(
                                f"unstable circuit at {fs} Hz, controls {bass, mid, treble, br, volume}"
                            )
                        ft = np.fft.rfft(x)
                        measured = np.interp(h, np.fft.rfftfreq(len(x), 1 / rate), ft)
                        ratio = measured / v[select, 1]
                        worst = max(worst, max(abs(20 * np.log10(abs(ratio)))))
                        phase = max(phase, max(abs(np.angle(ratio, deg=True))))
                        sweeps += 1
    print(f"{sweeps} Rust sweeps: {worst:.4f} dB, {phase:.3f} degrees")
    if worst > 0.25:
        raise SystemExit("0.25 dB exceeded")


if __name__ == "__main__":
    check(sys.argv[1])
