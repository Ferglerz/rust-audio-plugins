"""CP-70B adjustment sheet 006386 pp. 13–14. Run with ngspice, numpy and scipy."""
import sys
from pathlib import Path

import numpy as np
from scipy.io import wavfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import circuit

DECK = Path(__file__).with_name("tr.cir")


def response(key=None, **controls):
    network = None
    if key is not None:
        # Diagram pp. 14–15 uses physical key numbers; TR IN is at the high end.
        branches = ["Rmix1 bass middle 4.7k", "Rmix2 middle in 4.7k", "Rleak bass 0 1e13"]
        for k in range(1, 74):
            bus = "bass" if k <= 63 else "middle" if k <= 65 else "in"
            branches += [f"Vp{k} p{k} 0 DC 0 AC {int(key == k)}", f"Cp{k} p{k} {bus} 1n"]
        network = "\n".join(branches)
    return circuit.response(deck_file=DECK, pickup_network=network, **controls)


def check_rust(directory, trim):
    # Rust runs the complete pickup-to-jack circuit at 4x the 48 kHz host.
    # Check uncorrected audible frequencies as well as the trapezoidal mapping.
    ok = True
    for bus, key in enumerate((63, 64, 66), start=1):
        hz, taps = response(key, **trim)
        audible = (hz >= 40) & (hz <= 10_000)
        frequency, jack = hz[audible], taps[audible, 1]
        rate, samples = wavfile.read(directory / f"bus-{bus}.wav")
        if samples.ndim != 1 or rate != 192_000 or len(samples) != rate:
            raise ValueError(f"Unexpected Rust impulse format for bus {bus}")
        bins = np.fft.rfftfreq(len(samples), 1 / rate)
        spectrum = np.fft.rfft(samples.astype(np.float64))
        mapped = rate / np.pi * np.arctan(np.pi * frequency / rate)
        for label, frequencies, db_limit, deg_limit in [
            ("same frequency", frequency, 0.31, 0.5),
            ("trapezoidal mapping", mapped, 0.02, 0.05),
        ]:
            ratio = np.interp(frequencies, bins, spectrum) / jack
            db = max(abs(20 * np.log10(abs(ratio))))
            degrees = max(abs(np.angle(ratio, deg=True)))
            print(f"Bus {bus} to JO1, {label}: {db:.4f} dB, {degrees:.3f} degrees")
            ok &= db <= db_limit and degrees <= deg_limit
    return ok


def main(directory=None):
    trim = circuit.calibrate(response)
    print(f"VR12 travel {trim['trim']:.6f}, effective optical resistance {trim['cell']:.1f} ohms")
    cases = [
        ("PO", 500, 0, {}, -29, 0.1), ("JO1", 500, 1, {}, -31, 0.1),
        ("JO1", 50, 1, {}, -16.5, 1.5), ("BO1", 50, 2, {}, -18.5, 1.5),
        ("JO1 bass min", 50, 1, {"bass": 0}, -34.5, 1.5),
        ("JO1", 5000, 1, {}, -25, 1.5), ("BO1", 5000, 2, {}, -25.5, 1.5),
        ("JO1 treble min", 5000, 1, {"treble": 0}, -40, 1.5),
        ("JO1 patch cable", 500, 1, {"patchlink": 0.1}, -31, 1),
        ("BO1 volume min", 500, 2, {"volume": 0}, -30.5, 1.5),
    ]
    ok = True
    for name, hz, out, controls, target, tolerance in cases:
        value = circuit.level(response(**trim, **controls), hz, out)
        print(f"{name:17} {hz:4} Hz  {value:6.2f} dBm, target {target:5.1f} ±{tolerance:.1f}")
        ok &= abs(value - target) <= tolerance
    for name, controls in [("open patch", {"patchlink": 1e12}), ("muted jack", {"volume": 0})]:
        value = circuit.level(response(**trim, **controls), 500, 1)
        print(f"{name:17}  500 Hz  {value:6.2f} dBm, maximum -70")
        ok &= value <= -70
    for key in (63, 64, 66):
        hz, taps = response(key, **trim)
        i = np.argmin(abs(hz - 5000))
        print(f"Pickup {key} to patch at {hz[i]:.0f} Hz  {20*np.log10(abs(taps[i,0])):.2f} dB")
    return ok if directory is None else check_rust(directory, trim) and ok


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("render_directory", type=Path, nargs="?",
                        help="directory written by the cp70b_probe Rust example")
    raise SystemExit(0 if main(parser.parse_args().render_directory) else 1)
