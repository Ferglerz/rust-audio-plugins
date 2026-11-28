"""Fit the wah opening on A2/G2 in the factory film. Reserve F-sharp2/E2."""
import argparse
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.optimize import least_squares

ONSETS={45:226.635,43:227.249,42:227.839,40:228.416}
TIMES=np.array([.025,.045,.065,.085,.110,.140,.170,.200])
BLOCK=1
ATTACK=.150
def lines(x,r,f0):
    # A 40 ms window trades low-note frequency resolution against time resolution.
    bins=np.fft.rfftfreq(32768,1/r)
    harmonics=f0*np.arange(1,1+int(5200/f0))
    peaks=[]; noise=[]
    for t in TIMES:
        a=x[round((t-.019)*r):round((t+.019)*r)]
        spectrum=abs(np.fft.rfft(a*np.hanning(len(a)),32768))
        def band(lo,hi):return max(spectrum[np.searchsorted(bins,lo):np.searchsorted(bins,hi)+1])
        peaks.append([band(f-max(7.,.007*f),f+max(7.,.007*f)) for f in harmonics])
        noise.append([band(f+.36*f0,f+.56*f0) for f in harmonics])
    return np.maximum(peaks,1e-12),np.array(noise),harmonics

def prepare(ref,r,strikes):
    result={}
    for note,onset in ONSETS.items():
        f0=440*2**((note-69)/12)
        synth,sr=sf.read(strikes/f"fulltone-{note}.wav");synth=synth[:,2]
        capture=ref[round(onset*r):]
        actual,noise,f=lines(capture,r,f0)
        dry,_,_=lines(synth,sr,f0)
        # The unknown recording gain and the strength of each strike are confounded.
        # Match the recorded fundamental only; the wah acts mainly on higher partials.
        relative_drive=np.median(actual[3:,0]/dry[3:,0])*sr/r
        signal=(20*np.log10(actual/dry))
        delta=signal-signal[-1]
        usable=(f>250)&(actual>1.5*noise)&(actual[-1]>1.5*noise[-1])
        rect=abs(synth[:int(.211*sr)//BLOCK*BLOCK]).reshape(-1,BLOCK).mean(axis=1)
        result[note]=(f,rect,sr/BLOCK,relative_drive,delta,usable)
    return result

def simulate(entry,params):
    f,rect,rate,drive,delta,usable=entry
    gain,q=np.exp(params)
    env=0.;envelope=[]
    a=1.-np.exp(-1/(ATTACK*rate))
    b=1.-np.exp(-1/(.082*rate))
    for val in rect:
        env+=(a if val>env else b)*(val-env)
        envelope.append(env)
    levels=np.interp(TIMES,np.arange(len(rect))/rate,envelope)
    cutoff=310+4640*(-np.expm1(-gain*drive*levels))
    axis=f[None,:]/cutoff[:,None]
    shape=-10*np.log10((1-axis*axis)**2+(axis/q)**2)
    err=delta-(shape-shape[-1])
    def center(arr):
        # Discard each window's broadband gain from varying attack/recording levels.
        clean=[]
        for row,ok in zip(arr[:-1],usable[:-1]):
            if ok.sum()>=3:clean.extend(row[ok]-np.median(row[ok]))
        return np.array(clean)
    return center(err),center(delta),cutoff

if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference",type=Path);parser.add_argument("strikes",type=Path)
    args=parser.parse_args()
    ref,r=sf.read(args.reference)
    entries=prepare(ref,r,args.strikes)
    norm=np.mean([entries[n][3] for n in (43,45)])
    for n,(f,rect,rate,drive,delta,usable) in list(entries.items()):
        entries[n]=(f,rect,rate,drive/norm,delta,usable)
    fit=least_squares(lambda x:np.concatenate([simulate(entries[n],x)[0] for n in (45,43)]),
        np.log([120,3.]),bounds=(np.log([12,1.]),np.log([500,9.])),loss="soft_l1",f_scale=3.)
    print("assumed attack",ATTACK,"s, fitted effective gain at comparison sensitivity and Q",np.exp(fit.x))
    for n in ONSETS:
        err,flat,freq=simulate(entries[n],fit.x)
        wet,sr=sf.read(args.strikes/f"wah-{n}.wav")
        dry,_=sf.read(args.strikes/f"fulltone-{n}.wav")
        f0=440*2**((n-69)/12)
        measured=20*np.log10(lines(wet[:,2],sr,f0)[0]/lines(dry[:,2],sr,f0)[0])
        rem=entries[n][4]-(measured-measured[-1])
        fixed=np.concatenate([v[use]-np.median(v[use]) for v,use in
                              zip(rem[:-1],entries[n][5][:-1]) if use.sum()>=3])
        print(n,"relative drive",round(entries[n][3],2),len(err),"bins, median time-relative shape residual",
              round(np.median(abs(err)),2),"dB; static",round(np.median(abs(flat)),2),
              "dB; fixed-strike Rust render",round(np.median(abs(fixed)),2),
              "dB; estimated frequency",freq.round().astype(int))
