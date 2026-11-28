"""Held-out notes outside the three calibration keys; accepts the owner's effects-off PCM."""
from pathlib import Path
import argparse
import numpy as np
from recordings import extract, metrics, note_onset

# Coarse attack time, begin/end of isolated fundamental decay in seconds.
# No data from these seven keys entered scale() or the original three fits.
TAKES=[(36, 543.97, .14, .98), (36, 357.55, .15, .70), (43, 420.34, .15, .87),
       (60, 949.84, .07, .22), (70, 874.43, .12, .63),
       (74, 144.57, .09, .31), (89, 727.03, .11, .54),
       (90, 328.82, .08, .34)]


def resolved(y,rate,start,f0,harmonic):
    # Estimate local floor around the predicted partial in the same 65 ms window.
    n=round(rate*.065)
    offset=round((start+.020)*rate)
    transform=abs(np.fft.rfft(y[offset:offset+n]*np.hanning(n),32768))
    frequency=np.fft.rfftfreq(32768,1/rate)
    center=f0*harmonic
    dist=abs(frequency-center)
    width=min(0.4*f0, max(29., center*.035))
    signal=transform[dist<max(12.,center*.006)].max()
    noise=np.median(transform[(dist>width*.85)&(dist<width*1.15)])
    return signal>4*max(noise,1e-12)



def main():
    p=argparse.ArgumentParser()
    p.add_argument('captures', type=Path)
    p.add_argument('dry', type=Path)
    args=p.parse_args()
    for n,t,early,last in TAKES:
        nominal=440*2**((n-69)/12)
        hw,rate=extract(args.dry,t-.1,last+.4)
        onset=note_onset(hw,rate,nominal)
        model,_=extract(args.captures/f'key-{n}.wav',0,1.33,chan=0)
        m=np.array(metrics(model,rate,0,nominal,early,last))
        r=np.array(metrics(hw,rate,onset,nominal,early,last))
        fields=[f'{x:.2f}/{y:.2f}' for x,y in zip(r,m)]
        for i in [1,2]:
            if not resolved(hw,rate,onset,r[0],i+1):
                fields[i]=f'unresolved/{m[i]:.2f}'
        print(f'{n} {t-.1+onset:.3f} | '+' | '.join(fields))


if __name__=='__main__': main()
