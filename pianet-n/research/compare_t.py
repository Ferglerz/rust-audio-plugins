"""Compare native Pianet T dry jack recordings with the physical model and passive load model."""
from pathlib import Path
import argparse
import hashlib

import numpy as np
import soundfile as sf

FIT = {29, 37, 45, 52, 61, 69, 77, 88}


def measures(x, sr, note):
    hz = 440 * 2 ** ((note - 69) / 12)
    segment = x[round(.12 * sr):round(.32 * sr)]
    size = 131072
    freq = np.fft.rfftfreq(size, 1 / sr)
    mag = abs(np.fft.rfft(segment * np.hanning(len(segment)), size))
    amps = [mag[abs(freq - n * hz) < max(6, n * hz * .008)].max() for n in (1, 2, 3)]
    # A later half-second window resolves single-cent shifts even for the lowest note.
    segment = x[round(.15 * sr):round(.65 * sr)]
    mag = abs(np.fft.rfft(segment * np.hanning(len(segment)), size))
    i = np.argmax(np.where(abs(freq-hz) < .04 * hz, mag, 0.))
    a, b, c = np.log(mag[i-1:i+2])
    fundamental = freq[i] + (a-c)/(a-2*b+c) * sr/(2*size)
    n = round(.1 * sr)
    basis = np.hanning(n) * np.exp(-2j*np.pi*fundamental*np.arange(n)/sr)
    times = np.arange(.20, 1.15, .10)
    levels = [20*np.log10(abs(basis @ x[round(t*sr):round(t*sr)+n])) for t in times]
    slope = np.polyfit(times, levels, 1)[0]
    return np.array([1200*np.log2(fundamental / hz), slope, *20*np.log10(np.array(amps[1:])/amps[0])])


def burst_ms(x, sr):
    width = round(.004*sr)
    energy = np.convolve(x*x, np.ones(width)/width, mode="valid")
    peak = energy.argmax()
    hop = round(sr*.0005)
    rest = energy[peak::hop] < energy[peak]*.10
    below = np.flatnonzero(np.convolve(rest, np.ones(6), mode="valid") == 6)
    return 1000*hop/sr*below[0] if len(below) else np.nan


def main(ref, root):
    for line in Path(__file__).with_name("t-hashes.txt").read_text().splitlines():
        digest, name = line.split()
        if hashlib.sha256((ref/name).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Unexpected recording {name}")
    print("Absolute residuals: pitch (cents), fundamental decay (dB/s), H2:H1 (dB), H3:H1 (dB)")
    measured = {}
    for layer in ("P", "FF"):
        residuals = {True: [], False: []}
        for wav in sorted(ref.glob(f"*_{layer}.flac")):
            note = int(wav.name.split("_")[0])
            prediction = root/f"{note}-{layer}.wav"
            if not prediction.exists():
                raise ValueError(f"Missing render {prediction}")
            x, sr = sf.read(wav); y, model_sr = sf.read(prediction)
            observed = measures(x,sr,note); modeled = measures(y,model_sr,note)
            residuals[note in FIT].append(abs(modeled-observed))
            measured[note,layer] = (observed,modeled)
        for fit, errors in residuals.items():
            print(f"{layer} {'anchor' if fit else 'reserved'} pitches n={len(errors)} median",
                  np.round(np.median(errors,axis=0),2))
    for key in [45,65,81]:
        observed, modeled = measured[key,'P']
        print(f"MIDI {key} P reference/model", np.round(observed,2),np.round(modeled,2))
        src=next(ref.glob(f"{key}_*_release.flac"))
        x, sr = sf.read(src)
        y, model_sr=sf.read(root/f"{key}-P.wav")
        print("  isolated release burst / modeled key-up burst, peak to -10 dB ms",
              round(burst_ms(x,sr),1),round(burst_ms(y[round(1.50*model_sr):round(1.65*model_sr)],model_sr),1))
    # Verify the rate-warped RLC transfer against the continuous impedance divider.
    # Coil and transformer values are working estimates; only the 2 nF is printed.
    for load in [10000,100000,1000000]:
        impulse, sr = sf.read(root/f"bank-{load//1000}k.wav")
        transfer = np.fft.rfft(impulse)
        f = np.array([44,110,440,1000,2500,5000,10000])
        omega = 2j*sr*np.tan(np.pi*f/sr)
        z = 1/(1/load + 2e-9*omega + 1/(5*16*omega))
        analog = 4*z/(z + 60*18*16 + 60*.0011*16*omega)
        ratio = transfer[f] / analog  # one-second impulse => FFT bins at integer Hz
        err = np.max(abs(20*np.log10(abs(ratio))))
        if err > .01: raise ValueError(f"Passive transfer residual {err} dB at {load:g} ohms")
        print(f"Load {load//1000} kΩ, gains at 110/440/5000 Hz in dB",
              np.round(20*np.log10(abs(transfer[[110,440,5000]])),1),f"worst solver delta {err:.5f} dB")


if __name__ == "__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sample_directory",type=Path)
    parser.add_argument("probe_directory",type=Path)
    args=parser.parse_args()
    main(args.sample_directory,args.probe_directory)
