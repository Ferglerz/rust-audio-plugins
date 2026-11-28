"""Common 48 kHz measurements for dry reference strikes."""

import numpy as np
import soundfile as sf
from scipy.optimize import minimize_scalar
from scipy.signal import find_peaks, resample_poly
from scipy.signal.windows import blackmanharris

FS = 48000


def load(path):
    wave, rate = sf.read(path)
    if wave.ndim == 2:
        wave = wave.mean(axis=1)
    return resample_poly(wave, FS, rate) if rate != FS else wave


def rms(x):
    return np.sqrt(np.mean(x * x))


def db(ratio):
    return 20 * np.log10(np.maximum(1e-12, ratio))


def window(x, start, stop):
    return x[round(start * FS) : round(stop * FS)]


def harmonic(x, f, h=1, taper=None):
    t = np.arange(len(x)) / FS
    w = np.hanning(len(x)) if taper is None else taper
    return 2 * abs(np.dot(w * x, np.exp(-2j * np.pi * t * f * h))) / w.sum()


def spectrum(samples):
    return abs(np.fft.rfft(np.hanning(len(samples)) * samples))


def bell_peak(x, fundamental):
    """Resolve a non-harmonic line in Gabrielli's first tine-mode band (7.1 ± 0.9)."""
    attack = window(x, 0.004, 0.184)
    taper = blackmanharris(len(attack))
    fft_size = 4 * len(attack)
    magnitude = abs(np.fft.rfft(attack * taper, fft_size))
    hz = np.fft.rfftfreq(fft_size, 1 / FS)
    band = (hz >= 6.2 * fundamental) & (hz <= min(8.0 * fundamental, 16000))
    if not band.any():
        return np.nan, np.nan
    # Blackman-Harris main-lobe half width. Reject harmonic leakage and peaks
    # indistinguishable from the local floor of the recording.
    distance = abs(hz - fundamental * np.rint(hz / fundamental))
    resolved = band & (distance > 4 * FS / len(attack))
    if not resolved.any():
        return np.nan, np.nan
    levels = db(magnitude / max(magnitude.max(), 1e-20))
    floor = max(-72, np.median(levels[resolved]) + 10)
    peaks, _ = find_peaks(levels, height=floor, prominence=8)
    peaks = peaks[resolved[peaks]]
    if not len(peaks):
        return np.nan, np.nan
    i = peaks[np.argmax(levels[peaks])]
    h1 = harmonic(attack, fundamental, taper=taper)
    return hz[i], db(2 * magnitude[i] / taper.sum() / max(h1, 1e-12))


def metrics(x, n):
    nominal = 440 * 2 ** ((n - 69) / 12)
    middle = window(x, 0.09, 0.30)
    f = minimize_scalar(
        lambda f: -harmonic(middle, f),
        bounds=(nominal * 0.994, nominal * 1.006),
        method="bounded",
    ).x
    attack = window(x, 0.004, 0.070)
    # Fundamental amplitude decay, measured over the same interval in both sources.
    t = np.array([0.14, 0.3, 0.46, 0.62])
    amps = [harmonic(window(x, s - 0.055, s + 0.055), f) for s in t]
    bell_hz, bell_level = bell_peak(x, f)
    h1 = harmonic(middle, f)
    h2 = harmonic(middle, f, 2)
    result = {
        "attack_rms_db": db(rms(attack)),
        "h2_dbfs": db(h2),
        "h2_h1_db": db(h2 / h1),
        "h3_h1_db": db(harmonic(middle, f, 3) / h1),
        "bell_peak_hz": bell_hz,
        "bell_peak_h1_db": bell_level,
        "bell_h1_db": db(harmonic(attack, nominal * 7.239) / harmonic(attack, f)),
        "fundamental_decay_db_s": np.polyfit(t, db(amps), 1)[0],
    }
    # Use at least six cycles to separate the partials on bass attacks.
    for phase, section in (
        ("attack", window(x, 0.004, max(0.070, 0.004 + 6 / f))),
        ("tail", window(x, 0.40, 0.62)),
    ):
        h1 = max(harmonic(section, f), 1e-12)
        for h in (2, 3):
            result[f"{phase}_h{h}_h1_db"] = db(harmonic(section, f, h) / h1)
    return result
