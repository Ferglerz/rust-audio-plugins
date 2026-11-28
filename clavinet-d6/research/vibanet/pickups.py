"""Compare four physical pickup selections in the factory 2014 film and the Rust model."""
import argparse
import csv
from pathlib import Path

import numpy as np
import soundfile as sf

# Midpoints of 160 ms windows with >84% of energy in a single harmonic series.
# 34 and 41 are used to estimate pickup placement. 38 and 43 are reserved.
WINDOWS = {
    "lead": {34:97.53, 38:96.63, 41:94.68, 43:92.33, 45:89.63},
    "rhythm": {34:114.88, 38:113.88, 41:112.58, 43:111.63, 45:118.73},
    "fulltone": {34:131.98, 38:131.33, 41:131.23, 43:127.23, 45:134.68},
    "phase": {34:148.93, 38:147.43, 41:146.83, 43:144.13},
}
PARTIALS = np.arange(1, 17)
def lines(x, rate, note, centre):
    size = 65536
    a = x[round(rate*(centre-.08)):round(rate*(centre+.08))]
    mag = abs(np.fft.rfft(a*np.hanning(len(a)), size))
    bins = np.fft.rfftfreq(size, 1/rate)
    f = 440 * 2**((note-69)/12)
    def peak(lo, hi):
        return max(mag[np.searchsorted(bins,lo):np.searchsorted(bins,hi)+1])
    amp, noise = [], []
    for h in PARTIALS:
        w = max(5., h*f*.011)
        amp.append(peak(h*f-w,h*f+w))
        noise.append(max(peak(h*f-.45*f,h*f-.30*f),peak(h*f+.30*f,h*f+.45*f)))
    return 20*np.log10(np.maximum(amp,1e-12)), np.array(amp)>2*np.array(noise)

def read(reference, strikes):
    ref, rate = sf.read(reference)
    obs, models = {}, {}
    for mode, by_note in WINDOWS.items():
        for note, t in by_note.items():
            y, r = sf.read(strikes/f"{mode}-{note}.wav")
            obs[mode,note] = lines(ref,rate,note,t)
            models[mode,note] = lines(y[:,2],r,note,.16)[0]
    return obs, models

def compare(obs, models, output):
    with output.open("w") as fp:
        out=csv.writer(fp)
        out.writerow(["mode","midi","harmonic","hardware_dB","model_dB","hardware_SNR_above_6dB"])
        for (mode,note),(ref,ok) in obs.items():
            pred=models[mode,note]
            for h,a,b,keep in zip(PARTIALS,ref,pred,ok):
                out.writerow([mode,note,h,round(a-ref[0],2),round(b-pred[0],2),int(keep)])
    for mode in WINDOWS:
        residuals={}
        for n in WINDOWS[mode]:
            ref,ok=obs[mode,n]
            delta=ref-models[mode,n]
            if ok.sum()>=4:
                residuals[n]=abs(delta[ok]-np.median(delta[ok]))
        fmt=lambda keys:np.median(np.concatenate([residuals[n] for n in keys if n in residuals]))
        print(f"{mode:9} per-strike level matched H1–H16  fit {fmt([34,41]):.2f} dB   reserved {fmt([38,43,45]):.2f} dB")
    print("Lead minus rhythm, median per-strike gain removed")
    for n in WINDOWS["lead"]:
        a,oka=obs["lead",n]; b,okb=obs["rhythm",n]
        use=oka&okb
        if use.sum()<4:
            print(n,"too few resolved lines")
            continue
        raw=(a-b)[use]
        pred=(models["lead",n]-models["rhythm",n])[use]
        residual=raw-pred
        print(n,use.sum(),"usable lines",round(np.median(abs(raw-np.median(raw))),2),"dB flat;",round(np.median(abs(residual-np.median(residual))),2),"dB model")

if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference",type=Path,help="Factory film decoded to mono WAV")
    parser.add_argument("strikes",type=Path,help="Output of the vibanet Rust example")
    parser.add_argument("output",type=Path)
    args=parser.parse_args()
    compare(*read(args.reference,args.strikes),args.output)
