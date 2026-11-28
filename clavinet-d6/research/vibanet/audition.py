"""Write local listening pairs for the matched release and wah performances."""
import argparse
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import resample_poly
from math import gcd

from release import EVENTS, release
from wah import ONSETS

RATE=44100
def pair(a,ar,b,br):
    def play(x,r):
        g=gcd(r,RATE)
        y=resample_poly(x,RATE//g,r//g)
        y*=.16/max(np.sqrt(np.mean(y*y)),1e-8)
        y[:160]*=np.linspace(0,1,160)
        y[-320:]*=np.linspace(1,0,320)
        return y
    return np.concatenate([play(a,ar),np.zeros(round(.15*RATE)),play(b,br),np.zeros(round(.33*RATE))])

if __name__=="__main__":
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("reference",type=Path);p.add_argument("strikes",type=Path);p.add_argument("output",type=Path)
    args=p.parse_args();x,r=sf.read(args.reference)
    for kind in ["release","wah"]:
        out=[]
        if kind=="release":
            for i in [0,1,4,6,7]:
                mode,n,approx,gate=EVENTS[i]
                t=release(x,r,approx,n)-gate/1000
                y,sr=sf.read(args.strikes/f"release-{mode.title()}-{gate}-{n}.wav")
                duration=gate/1000+.14
                out.append(pair(x[round(t*r):round((t+duration)*r)],r,y[:round(duration*sr),2],sr))
        else:
            for n,t in ONSETS.items():
                y,sr=sf.read(args.strikes/f"wah-{n}.wav")
                out.append(pair(x[round(t*r):round((t+.38)*r)],r,y[:round(.38*sr),2],sr))
        joined=np.concatenate(out)
        sf.write(args.output/f"{kind}-factory-then-model.wav",.85*joined/max(abs(joined)),RATE)
