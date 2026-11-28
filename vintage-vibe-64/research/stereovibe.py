"""Measure alternating channel envelopes in manufacturer and owner performances.

Supply the three stereo source files decoded at 24 kHz and the directory written
by the stereovibe_probe example. The analysis compares envelopes independently
of the notes being played. The output is a CSV and short local listening reels.
"""
import argparse
import csv
from pathlib import Path
import numpy as np
import soundfile as sf
from scipy.optimize import minimize_scalar
from scipy.signal import butter, resample_poly, sosfiltfilt

CASES = [
    ("ginty", "ginty", 5., 8., 1.835, "fit"),
    ("ginty", "ginty", 117., 8., 1.835, "later window"),
    ("intro", "intro-fast", 40., 7., 5.517, "owner"),
    ("intro", "intro-slow", 65., 5., 1.065, "owner"),
    ("sigma", "sigma-fast", 26., 9., 6.426, "early retrofit"),
    ("sigma", "sigma-slow", 47., 9., 1.563, "early retrofit"),
    ("sigma", "sigma-half", 68., 9., 0.911, "early retrofit"),
]


def read(path, start, duration):
    with sf.SoundFile(path) as f:
        rate = f.samplerate
        if f.channels != 2 or rate not in [24000, 48000]:
            raise ValueError("Expected 24 or 48 kHz stereo WAV")
        f.seek(round(start*rate))
        y = f.read(round(duration*rate))
    return resample_poly(y, 1, 2, axis=0) if rate == 48000 else y


def shape(y, hint):
    filter_ = butter(2, [100, 2200], fs=24000, btype="bandpass", output="sos")
    y = sosfiltfilt(filter_, y, axis=0)
    energy = (y.reshape(-1, 120, 2)**2).mean(axis=1)
    x = (energy[:, 0] - energy[:, 1])/np.maximum(energy.sum(axis=1), 1e-12)
    t = np.arange(len(x))*.005
    def projection(f):
        return np.mean((x-x.mean())*np.exp(-2j*np.pi*t*f))
    hz = minimize_scalar(lambda f: -abs(projection(f)), bounds=(hint-.095, hint+.095),
                         method="bounded").x
    phase = (np.mod(t*hz + np.angle(projection(hz))/(2*np.pi)+.5, 1)*64).astype(int)
    fold = np.array([np.median(x[phase == j]) for j in range(64)])
    spectrum = abs(np.fft.rfft(fold-fold.mean()))
    h3 = 20*np.log10(spectrum[3]/spectrum[1])
    bounds = np.percentile(x, [5., 95.])
    lr_db = 10*np.log10((1+bounds)/(1-bounds))
    return hz, h3, lr_db, fold


def listen_clip(y):
    y = y[:24000*2].copy()
    y *= min(.12/np.sqrt(np.mean(y**2)), .95/np.max(abs(y)))
    fade = np.linspace(0., 1., 240)[:, None]
    y[:240] *= fade
    y[-240:] *= fade[::-1]
    return y


def verify_input(root):
    # Independent complex-impedance solution of the proposed pickup R/L/C and
    # coupled input branches. Bilinear frequency map matches the nodal solver.
    errors = []
    with (root/"input-sweep.csv").open() as file:
        for row in csv.DictReader(file):
            mode, f = row["input"], float(row["frequency_hz"])
            conductance = (1/33000 + 1/390000 + 1/150000) if mode != "Fet" else 0.
            if mode != "Classic":
                conductance += 1e-6
            s = 2j*48000*np.tan(np.pi*f/48000)
            z_load = 1/conductance + 1/(s*.22e-6)
            send = 1/(1 + (2500 + s*.90) * (s*150e-12 + 1/z_load))
            gain = {"Classic": 1.8, "Fet": 2.3, "Parallel": 2.05}[mode]
            output = send*(1/conductance)/z_load*gain
            observed = [float(row["harp_send_db"]), float(row["dry_jack_db"])]
            errors.extend(np.abs(observed-20*np.log10(np.abs([send, output]))))
    print("input network", len(errors), "terminal checks, max error dB", max(errors))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    for name in ["ginty", "intro", "sigma"]:
        parser.add_argument("--"+name, type=Path, required=True)
    args = parser.parse_args()
    verify_input(args.root)
    reels = {}
    with (args.root/"recordings.csv").open("w", newline="") as csvfile:
        rows = csv.writer(csvfile)
        rows.writerow(["source", "use", "time", "owner_hz", "model_hz", "owner_H3_dBc",
                       "model_H3_dBc", "owner_LR_p5_dB", "owner_LR_p95_dB", "model_LR_p5_dB",
                       "model_LR_p95_dB", "folded_rms"])
        for source, setting, start, duration, hint, use in CASES:
            y = read(getattr(args, source), start, duration)
            h, h3, lr, shape_y = shape(y, hint)
            m, m3, mlr, shape_m = shape(read(args.root/f"{setting}-pilot.wav", .5, duration), hint)
            rms = np.mean((shape_m-shape_y)**2)**.5
            vals = [h, m, h3, m3, *lr, *mlr, rms]
            print(source, start, "hardware/model Hz, H3, L/R range and RMS", np.round(vals, 3))
            rows.writerow([source, use, start, *np.round(vals, 3)])
            reel = reels.setdefault(source, [])
            reel.extend([listen_clip(y), np.zeros((6000, 2)),
                         listen_clip(read(args.root/f"{setting}-chord.wav", .01, 2.)),
                         np.zeros((10000, 2))])
    for name, pairs in reels.items():
        sf.write(args.root/f"{name}-then-model.wav", np.concatenate(pairs), 24000, subtype="PCM_24")


if __name__ == "__main__":
    main()
