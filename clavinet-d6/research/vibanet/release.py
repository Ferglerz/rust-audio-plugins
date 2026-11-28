"""Compare release time in the dry, labelled pickup passages of the factory film.

The first 7.5% drop in measured fundamental marks opening of the clamp.
T12 uses broadband RMS and stops before the next note or the recording floor.
"""
import argparse
from pathlib import Path

import numpy as np
import soundfile as sf

# Approximate key-up from the audio. No MIDI or contact closure signal was recorded.
EVENTS = [
    ("lead",41,96.540,134), ("rhythm",41,113.840,134), ("rhythm",41,114.240,126),
    ("fulltone",41,129.710,122), ("fulltone",41,131.280,122),
    ("rhythm",46,116.050,114), ("fulltone",46,133.250,154), ("phase",43,145.750,118),
    ("fulltone",43,127.425,146),
]
def release(x, r, around, note):
    f = 440*2**((note-69)/12)
    bins = np.fft.rfftfreq(16384,1/r)
    use=(bins>f*.76)&(bins<f*1.10)
    for t in np.arange(around-.04,around+.041,.002):
        a=x[round(r*(t-.018)):round(r*(t+.018))]
        spectrum=abs(np.fft.rfft(a*np.hanning(len(a)),16384))
        if bins[use][spectrum[use].argmax()] < .925*f:
            return t
    raise ValueError(f"No clear pitch drop at {around}")

def decay(x,r,t):
    def energy(dt):
        a=x[round((t+dt-.010)*r):round((t+dt+.010)*r)]
        return np.mean(a*a)
    ref=np.median([energy(d) for d in np.arange(-.065,-.024,.005)])
    times=np.arange(.002,.161,.002)
    attenuation=-10*np.log10(np.maximum([energy(dt) for dt in times],1e-20)/ref)
    passed=np.where(np.convolve((attenuation>=12).astype(int),np.ones(5,dtype=int),"valid")>=5)[0]
    return (round(1000*times[passed[0]]) if len(passed) else None,
            np.interp([.04,.08,.12], times,attenuation))

if __name__=="__main__":
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("reference",type=Path);p.add_argument("strikes",type=Path)
    args=p.parse_args()
    reference,r=sf.read(args.reference)
    for mode,n,approx,gate in EVENTS:
        t=release(reference,r,approx,n)
        actual,hw=decay(reference,r,t)
        render,sr=sf.read(args.strikes/f"release-{mode.title()}-{gate}-{n}.wav")
        pred,model=decay(render[:,2],sr,gate/1000)
        print(f"{mode:9} {n}  release {t:7.3f}s  gate {gate}ms  T12 ref/model {actual}/{pred}ms"
              f"  40/80/120ms drop ref {np.round(hw,1)} dB  model {np.round(model,1)} dB")
