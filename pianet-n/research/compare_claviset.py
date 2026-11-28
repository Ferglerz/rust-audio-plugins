"""Measure the 200 and Selmer performances against the cam-reed examples.

Pass the example output directory containing owner-selmer.wav and owner-200.wav
(22,050 Hz mono decoded from the identified demonstrations).
"""
import argparse
import csv
import hashlib
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.optimize import minimize_scalar
from scipy.signal import butter, hilbert, periodogram, resample_poly, sosfiltfilt

RATE = 22050
ANALYSIS = RATE // 2
# Times in the owner recording. The model's fixed 160 ms analysis window is
# centered 210 ms after key-down. Stock Selmer tone is approximate. The 200
# register labels below are spoken before these passages in Christie's demo.
CASES = {
    "selmer": [
        (36, 136.32, ""), (40, 136.86, ""), (43, 137.16, ""),
        (48, 137.70, ""), (52, 138.06, ""), (55, 131.94, ""),
        (57, 146.82, ""), (60, 204.60, ""), (64, 238.74, ""),
        (67, 130.56, ""), (69, 160.62, ""), (72, 129.06, ""),
        (76, 140.52, ""),
    ],
    "200": [
        (36, 365.78, "Bass"), (48, 364.64, "Bass"),
        (55, 412.68, "Celesta"), (64, 409.56, "Celesta"), (73, 413.64, "Celesta"),
        (62, 424.98, "MusicBox"), (64, 421.68, "MusicBox"), (69, 424.62, "MusicBox"),
        (74, 424.38, "MusicBox"), (62, 328.58, "Xylophone"), (74, 434.06, "Xylophone"),
    ],
}
DIGESTS = {"200": "4650bf0cc213d85a747713def129a74b943e254716653217610d8294c31447bd",
           "selmer": "0abfa5980c23661cdc833f8aeb13d3a0a68d7265c15791499d6ca35a0c400446"}


def clip(y, rate, time, seconds):
    count = round(seconds * rate)
    start = round(rate * time) - count // 2
    return y[start:start + count]


def partials(y, center, midi):
    x = clip(y, ANALYSIS, center, .16)
    w = np.hanning(len(x))
    x = x - (w @ x) / w.sum()
    hz = 440 * 2**((midi-69)/12)
    orders = np.arange(1, min(24, int(4400/hz)) + 1)
    power = np.abs(np.fft.rfft(w*x, n=32768))**2
    bins = np.fft.rfftfreq(32768, 1/ANALYSIS)
    def cost(f):
        return -np.interp(f*orders, bins, power).sum()
    f = minimize_scalar(cost, method="bounded", bounds=(hz*2**(-35/1200), hz*2**(35/1200))).x
    basis = np.exp(2j*np.pi*f*np.arange(len(x))[:, None]*orders/ANALYSIS)
    amp = 2*(w*x) @ basis.conj()/w.sum()
    fit = 1-(w @ (x-np.real(basis@amp))**2)/(w @ x**2)
    h = 20*np.log10(np.maximum(abs(amp[1:6]), 1e-15)/abs(amp[0]))
    return f, fit, h


def slope(y, rate, midi, start, end):
    times = np.arange(start, end, .04)
    n = round(rate*.08)
    w = np.hanning(n)
    hz = 440*2**((midi-69)/12)
    basis = w*np.exp(-2j*np.pi*hz*np.arange(n)/rate)
    level = np.array([20*np.log10(abs(basis @ clip(y, rate, t, .08))) for t in times])
    return -np.polyfit(times, level, 1)[0]


def modulation(y, start, end):
    x = y[round(RATE*start):round(RATE*end)]
    band = sosfiltfilt(butter(3, [160, 1200], btype="bandpass", fs=RATE, output="sos"), x)
    envelope = sosfiltfilt(butter(3, 26, fs=RATE, output="sos"), abs(hilbert(band)))[::100]
    sr = RATE/100
    baseline = sosfiltfilt(butter(3, 2.5, fs=sr, output="sos"), envelope)
    f, p = periodogram(envelope / np.maximum(baseline, envelope.max()*.13),
                       fs=sr, window="hann", nfft=8192)
    which = (f>2.5) & (f<14.)
    return f[which][p[which].argmax()]


def pair(y, time, model):
    result = []
    for track, t in [(y, time), (model, .26)]:
        sub = clip(track, RATE, t, .32).copy()
        sub *= .10 / np.sqrt(np.mean(clip(track, RATE, t, .16)**2))
        ramp = np.linspace(0., 1., round(.005*RATE))
        sub[:len(ramp)] *= ramp
        sub[-len(ramp):] *= ramp[::-1]
        result.append(sub)
    gain = min(1., .94/max(np.max(abs(x)) for x in result))
    return np.concatenate([result[0]*gain, np.zeros(round(.16*RATE)), result[1]*gain,
                           np.zeros(round(.30*RATE))])


def main(root):
    rows = []
    for label, cases in CASES.items():
        reference = root / f"owner-{label}.wav"
        y, sr = sf.read(reference)
        if sr != RATE or y.ndim != 1:
            raise ValueError("Decode the source to 22,050 Hz mono")
        if hashlib.sha256(reference.read_bytes()).hexdigest() != DIGESTS[label]:
            print(f"{label} reference bytes differ from the documented Opus/FFmpeg capture")
        analysis = resample_poly(y,1,2)
        errors, listen = [], []
        for midi, t, register in cases:
            name = "-".join(str(x) for x in (label, register, midi) if x)
            model, sr = sf.read(root / f"{name}.wav")
            if sr != 44100:
                raise ValueError("Expected a 44.1 kHz probe")
            model = resample_poly(model, 1, 2)
            f, fit, h = partials(analysis, t, midi)
            mf, _, mh = partials(resample_poly(model, 1, 2), .26, midi)
            if fit < .91:
                raise ValueError(f"Weak recording pitch at {label} {t}: {fit}")
            for i, (a,b) in enumerate(zip(h,mh)):
                rows.append([label, register, midi, t, round(f,3), round(mf,3), round(fit,4),
                             i+2, round(a,2), round(b,2)])
            errors.append(abs(np.maximum(-40.,h)-np.maximum(-40.,mh)))
            listen.append(pair(y,t,model))
        sf.write(root/f"{label}-hardware-then-model.wav",np.concatenate(listen),RATE,subtype="PCM_24")
        print(label,"pitch/register observations",len(cases),"median H2-H6 error dB",round(np.median(errors),2),
              "by harmonic",np.round(np.median(errors,axis=0),1))
        if label=="selmer":
            for n,a,b in [(60,204.05,205.75),(64,149.10,150.15),(67,127.10,128.60),(69,159.08,159.76)]:
                m,s=sf.read(root/f"selmer-{n}.wav")
                print("Selmer H1 decay dB/s",n,round(slope(y,RATE,n,a,b),2),round(slope(m,s,n,.40,1.35),2))
        else:
            print("200 recorded modulation Hz", [round(modulation(y,a,b),2) for a,b in
                                                 [(459.5,462.3),(464.6,467.7),(468.8,470.9)]])
    with (root/"owner-harmonics.csv").open("w",newline="") as out:
        w=csv.writer(out)
        w.writerow(["instrument","register","midi","owner_center_s","owner_Hz","model_Hz",
                    "owner_fitted_energy","harmonic","owner_dBc","model_dBc"])
        w.writerows(rows)


if __name__ == "__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output",type=Path)
    main(parser.parse_args().output)
