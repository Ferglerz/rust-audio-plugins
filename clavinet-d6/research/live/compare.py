"""Compare independently rendered live-rate stems and jack with the 352.8 kHz D6 solver."""
import argparse
import hashlib
from math import gcd
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import resample_poly

RATE = 44_100


def read(path):
    x, fs = sf.read(path, always_2d=True)
    if fs != RATE:
        d = gcd(RATE, fs)
        x = resample_poly(x, RATE // d, fs // d, axis=0)
    if not np.isfinite(x).all():
        raise ValueError(f"nonfinite output in {path}")
    return x


def cut(x, start, end):
    return x[round(start * RATE):round(end * RATE)]


def spec(x):
    size = max(32768, 1 << (len(x) - 1).bit_length())
    return abs(np.fft.rfft(x * np.hanning(len(x)), size)), np.fft.rfftfreq(size, 1 / RATE)


def peak(m, f, hz):
    window = np.flatnonzero(abs(f-hz) < max(hz * 0.05, 3))
    i = window[np.argmax(m[window])]
    a, b, c = np.log(m[i-1:i+2] + 1e-18)
    return f[i] + 0.5 * (a-c) / (a-2*b+c) * f[1]


def metrics(x, target):
    m, f = spec(cut(x, 0.12, 0.30))
    hz = peak(m, f, target)
    # The ten lowest harmonics, capped at 6 kHz for treble notes.
    partials = np.array([np.max(m[abs(f - n * hz) < max(8, n * hz * 0.018)])
                         for n in range(1, min(10, int(6000 / hz)) + 1)])
    partials = 20 * np.log10(partials / np.max(partials))
    attack = cut(x, 0.05, 0.075)
    attack_db = 10 * np.log10(np.mean(attack**2))
    held = np.mean(cut(x, 0.745, 0.785)**2)
    quiet = [np.mean(cut(x, 0.795+t, 0.805+t)**2) < 0.01*held for t in np.arange(0,0.2,0.001)]
    crossings = np.flatnonzero(np.convolve(quiet, np.ones(6), mode="valid") == 6)
    return hz, partials, attack_db, crossings[0] if len(crossings) else float("nan")


def main(live, reference):
    print("rate note tap | pitch error cents | median relative partial error dB | attack energy delta dB | T20 ref/live ms")
    for rate in (44100, 48000, 96000):
        for key in [29, 46, 51, 52, 64, 78, 88]:
            x = read(live / str(rate) / f"{key}.wav")
            nominal = 440*2**((key-69)/12)
            if key not in (46, 64, 78):
                hz, _, _, t20 = metrics(x[:,0], nominal)
                print(f"{rate} {key} centre | from nominal {1200*np.log2(hz/nominal):+.2f} cents, T20 {t20:.0f} ms")
                continue
            base = reference / str(key)
            ref = np.column_stack((read(base / "pickups.wav"), read(base/"jack-CA.wav")))
            for i, name in enumerate(["centre", "bridge", "jack"]):
                pitch, partials, attack, t20 = metrics(x[:,i], nominal)
                refpitch, refpartials, refattack, reft20 = metrics(ref[:,i], nominal)
                cents = 1200*np.log2(pitch/refpitch)
                delta = np.median(abs(partials-refpartials))
                print(f"{rate} {key} {name} | {cents:+.2f} | {delta:.2f} | {attack-refattack:+.2f} | {reft20:.0f}/{t20:.0f}")
                # Numerical reduction regressions at the observed taps of this fixed fixture.
                # Independent hardware comparisons below have different uncertainties.
                if not (abs(cents) <= 1.0 and delta <= 1.5 and
                        abs(attack-refattack) <= 1.0 and abs(t20-reft20) <= 5):
                    raise ValueError(f"Live reduction diverged at {rate} Hz, MIDI {key}, {name}")


def hardware(live, recordings):
    # Harmonic spacing identifies notes in these windows. The riff often has a
    # stronger second harmonic, which can misidentify the key by an octave.
    files = {"singles": "samples_wholekeyboard.mp3", "riff": "D.wav"}
    manifest = Path(__file__).parents[1] / "references.sha256"
    hashes = dict(line.split()[::-1] for line in manifest.read_text().splitlines())
    refs = {}
    for key, name in files.items():
        path = recordings / name
        if hashlib.sha256(path.read_bytes()).hexdigest() != hashes[name]:
            raise ValueError(f"Unexpected hardware recording in {name}")
        refs[key] = read(path)[:,0]
    print("Reserved notes | hardware file/time | held pitch hardware/model | H1–10 median error jack dB")
    for midi, name, start in [(34, "singles", 0.48), (36, "riff", 0.24), (39, "riff", 1.99),
                               (41, "riff", 1.47), (43, "riff", 1.62)]:
        ref = cut(refs[name], start, start + 0.09)
        model = cut(read(live / "48000" / f"{midi}.wav")[:,2], 0.14, 0.23)
        nominal = 440 * 2**((midi-69)/12)
        def profile(x):
            mag, f = spec(x)
            # Infer tuning from the first six partials with energy above -20 dB.
            # The median is resilient when a pickup null makes one partial ambiguous.
            peaks = []
            amps = []
            for h in range(1, 11):
                loc = np.flatnonzero(abs(f - h*nominal) < nominal*0.35)
                i = loc[np.argmax(mag[loc])]
                amps.append(mag[i])
                a, b, c = np.log(mag[i-1:i+2]+1e-18)
                peaks.append((f[i] + f[1]*0.5*(a-c)/(a-2*b+c))/h)
            amps = 20 * np.log10(np.array(amps)/max(amps))
            hz = np.median(np.array(peaks[:6])[amps[:6] > -20])
            return hz, amps
        href, pref = profile(ref)
        hmodel, pmodel = profile(model)
        print(f"{midi} | {name} {start:.2f} | {href:.2f}/{hmodel:.2f} Hz | "
              f"{np.median(abs(pref-pmodel)):.1f}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("live", type=Path)
    parser.add_argument("reference", type=Path)
    parser.add_argument("--recordings", type=Path, help="folder with the original Aalto hardware recordings")
    args = parser.parse_args()
    main(args.live, args.reference)
    if args.recordings:
        hardware(args.live, args.recordings)
