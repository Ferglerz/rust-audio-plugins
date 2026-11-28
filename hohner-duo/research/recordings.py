"""Compare the three dated owner excerpts with the Duo's separately wired jacks.

Run after the duo_probe example, with the three named MP3s in the capture folder.
Writes per-window harmonic measurements and one hardware/model audition per branch.
"""
import argparse
import csv
import hashlib
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.optimize import minimize_scalar
from scipy.signal import resample_poly


FS = 44100
ANALYSIS_RATE = FS // 4
WINDOW = .16
FLOOR_DB = -40.
# Recording-only selection. Times identify the center of a 160 ms window. Multiple
# separated appearances expose performance variation without giving that pitch
# more weight in the aggregate than a note with a single accepted appearance.
RECORDINGS = [
    ("clavinet", 469901, "b67daeab3c58100b6676af69245d57cb94a0532304e11b22b2fad4f7ab30b10f", 0,
     {41: [.62, 4.66, 19.02], 58: [1.90, 23.54]}),
    ("pianet", 469902, "9ff746b8b06cf955aa295e42f594294189f3a707f159c1d8e8b604899eca0b8c", 1,
     {29: [9.50], 31: [9.78, 10.22], 33: [15.62, 17.62], 36: [.50], 39: [.74],
      40: [.98, 2.18], 41: [5.82], 45: [4.62, 7.06], 76: [16.70]}),
    ("mix", 469903, "bf7b76a97184d4728318dba8ad299754bdf03d8cd96bf3c60232b0f52a41b1ea", 2,
     {37: [3.38, 11.50, 15.62], 39: [4.90, 12.70], 41: [5.70, 9.78, 13.90]}),
]


def crop(audio, rate, center, seconds):
    size = round(seconds * rate)
    start = round(center * rate) - size // 2
    chunk = audio[start:start + size]
    if start < 0 or len(chunk) != size:
        raise ValueError(f"Window outside recording at {center:.3f} s")
    return chunk


def harmonics(audio, midi, center):
    """Windowed Fourier projections; estimate common f0 independently for each take."""
    y = crop(audio, ANALYSIS_RATE, center, WINDOW)
    weights = np.hanning(len(y))
    y = y - weights @ y / weights.sum()
    nominal = 440 * 2 ** ((midi - 69) / 12)
    orders = np.arange(1, 1 + min(24, int(4200 / nominal)))
    if len(orders) < 6:
        raise ValueError("H6 exceeds analysis band")
    fft = abs(np.fft.rfft(weights * y, n=32768))
    bins = np.fft.rfftfreq(32768, 1 / ANALYSIS_RATE)

    def objective(f):
        return -np.sum(np.interp(f * orders, bins, fft)**2)

    f = minimize_scalar(objective, bounds=(nominal * 2**(-35/1200),
                                           nominal * 2**(35/1200)), method="bounded").x
    phase = 2j * np.pi * f * np.arange(len(y)) / ANALYSIS_RATE
    basis = np.exp(phase[:, None] * orders)
    coeff = 2 * ((weights * y) @ basis.conj()) / weights.sum()
    residual = y - np.real(basis @ coeff)
    fit = 1 - (weights @ (residual**2)) / (weights @ (y**2))
    magnitude = abs(coeff)
    ratios = 20 * np.log10(np.maximum(1e-12, magnitude[1:6] / magnitude[0]))
    fundamental_db = 20 * np.log10(magnitude[0] / np.max(magnitude))
    return ratios, 1200 * np.log2(f / nominal), fit, fundamental_db


def audition_pair(owner, owner_time, model, model_time):
    # Match gains only in the measured window, without changing spectral measurements.
    pair = []
    for y, t in [(owner, owner_time), (model, model_time)]:
        x = crop(y, FS, t, .32).copy()
        active = crop(y, FS, t, WINDOW)
        x *= .1 / np.sqrt(np.mean(active**2))
        ramp = np.linspace(0, 1, round(.005 * FS))
        x[:len(ramp)] *= ramp
        x[-len(ramp):] *= ramp[::-1]
        pair.append(x)
    common = min(1., .95 / max(np.max(np.abs(x)) for x in pair))
    return np.concatenate([pair[0]*common, np.zeros(round(.16*FS)), pair[1]*common,
                           np.zeros(round(.36*FS))])


def compare(root):
    rows = []
    for label, track, digest, channel, passages in RECORDINGS:
        path = root / f"{track}.mp3"
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise ValueError(f"Reference identity mismatch for {track}")
        decoded, rate = sf.read(path, always_2d=True)
        if rate != FS:
            raise ValueError(f"Unexpected reference rate {rate}")
        owner = decoded.mean(axis=1)
        down = resample_poly(owner, 1, 4)
        clip = []
        errors = []
        repeat_spreads = []
        for note, times in passages.items():
            model = {}
            for velocity in ["soft", "medium", "hard"]:
                wave, sr = sf.read(root / f"{note}-{velocity}.wav", always_2d=True)
                if sr != FS or wave.shape[1] != 3:
                    raise ValueError("Expected the 44.1 kHz three-socket Duo probe")
                model[velocity] = wave[:, channel]
            primary = resample_poly(model["medium"], 1, 4)
            model_db, model_cents, _, _ = harmonics(primary, note, .21)
            variants = [harmonics(resample_poly(model[v], 1, 4), note, t)[0]
                        for v in model for t in (.17, .35, .55)]
            variants = np.maximum(FLOOR_DB, np.array(variants))
            obs = []
            for t in times:
                owner_db, cents, fit, fundamental = harmonics(down, note, t)
                if fit < .92 or fundamental < -16:
                    raise ValueError(f"Ambiguous {label} pitch {note} at {t:.2f}s: {fit:.3f}")
                obs.append(np.maximum(FLOOR_DB, owner_db))
                for i in range(5):
                    rows.append([label, track, note, t, round(cents, 3), round(model_cents, 3),
                                 round(fit, 4), i+2, round(owner_db[i], 3), round(model_db[i], 3),
                                 round(variants[:, i].min(), 3), round(variants[:, i].max(), 3)])
            hardware = np.median(obs, axis=0)
            error = np.abs(np.maximum(FLOOR_DB, model_db) - hardware)
            errors.append(error)
            if len(obs) > 1:
                repeat_spreads.extend(np.ptp(obs, axis=0))
            clip.append(audition_pair(owner, times[0], model["medium"], .21))
            print(f"{label:8s} MIDI {note:2d} H2–H6 dBc (floored):",
                  np.round(hardware,1), np.round(np.maximum(FLOOR_DB,model_db),1))
        e = np.array(errors)
        print(f"{label}: {len(passages)} pitches, {sum(map(len, passages.values()))} windows,"
              f" median absolute H2–H6 error {np.median(e):.2f} dB,"
              f" by harmonic {np.round(np.median(e,axis=0),2)},"
              f" repeated-window median range {np.median(repeat_spreads):.2f} dB")
        sf.write(root / f"duo-{label}-comparison.wav", np.concatenate(clip), FS, subtype="PCM_24")
    with (root / "recording-harmonics.csv").open("w", newline="") as out:
        writer = csv.writer(out)
        writer.writerow(["mode","recording","midi","hardware_center_s","hardware_cents",
                         "model_cents","hardware_harmonic_energy_fraction","harmonic",
                         "hardware_dBc","model_dBc","model_sensitivity_min_dBc_floored",
                         "model_sensitivity_max_dBc_floored"])
        writer.writerows(rows)

    # Play the same two routing demonstrations through the mixer and stereo sockets.
    takes = []
    for name in ("foot", "held-split"):
        signals, rate = sf.read(root / f"{name}.wav", always_2d=True)
        if rate != FS or signals.shape[1] != 3 or not np.isfinite(signals).all():
            raise ValueError(f"Invalid three-jack routing capture in {name}")
        takes.append(signals)
    phrase = np.concatenate((takes[0], np.zeros((FS // 4, 3)), takes[1]))
    phrase *= 0.85 / np.max(np.abs(phrase))
    sf.write(root / "duo-routing-stereo.wav", phrase[:, :2], FS, subtype="PCM_24")
    sf.write(root / "duo-routing-mix.wav", phrase[:, 2], FS, subtype="PCM_24")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture_dir", type=Path)
    compare(parser.parse_args().capture_dir)
