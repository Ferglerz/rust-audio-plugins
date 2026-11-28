"""Estimate Vibanet EQ curves from repeated playing under the maker's on-screen setting labels."""
import argparse
import hashlib
import subprocess
from pathlib import Path

import numpy as np
from scipy.optimize import least_squares

# Times are the visible captions for the 2014 factory unit. Exclude transitions.
SECTIONS = {
    "low": [(162.50,165.25,0),(165.25,167.75,-7.5),(167.75,172.00,-15),
            (172.00,175.00,7.5),(175.00,178.25,15)],
    "mid": [(180.25,182.75,0),(182.75,185.25,-7.5),(185.25,187.50,-15),
            (187.50,189.25,7.5),(189.25,192.25,15)],
    "high":[(192.75,194.25,0),(194.25,195.50,-7.5),(195.50,198.50,-15),
            (198.50,200.25,7.5),(200.25,202.00,15)],
}
RATE=22050
WINDOW=.14
BINS=32768
F=np.fft.rfftfreq(BINS,1/RATE)

def harmonics(roots):
    hs=sorted((root*i,root) for root in roots for i in range(1,100) if root*i<9600)
    # A chord's near-coincident partials cannot be attributed to one string.
    return np.array([(f,root) for i,(f,root) in enumerate(hs)
                     if (i==0 or f-hs[i-1][0]>24) and (i+1==len(hs) or hs[i+1][0]-f>24)]).T

def section(y,start,stop,roots,hz):
    series=[]
    for t in np.arange(start+.13,stop-.12,.025):
        a=y[round((t-WINDOW/2)*RATE):round((t+WINDOW/2)*RATE)]
        m=abs(np.fft.rfft(a*np.hanning(len(a)),BINS))
        def band(f,width):
            return max(m[np.searchsorted(F,f-width):np.searchsorted(F,f+width)+1])
        def noise(f):
            return max(np.median([band(f+d,3) for d in [-30,-22,22,30]]),1e-9)
        quality=np.array([band(f,7)/noise(f) for f in roots])
        if min(quality)<3.2: continue
        signal=np.array([band(f,max(5.,.006*f)) for f in hz])
        floor=np.array([noise(f) for f in hz])
        series.append((t,signal,floor))
    if not series:
        raise ValueError(f"no shared pitches at {start}s")
    times, signals, floors=zip(*series)
    return np.median(signals,axis=0),np.median(floors,axis=0),len(times)

def shape(f,kind,fc,q,gain):
    # Analog prototypes expressed in normalized s=jf/fc. The bell has the
    # displayed centre gain, while the two shelves have the displayed end gain.
    s=1j*f/fc
    a=10**(gain/40)
    if kind=="mid":
        transfer=(s*s+s*a/q+1)/(s*s+s/(a*q)+1)
    elif kind=="low":
        transfer=a*(s*s+np.sqrt(a)*s/q+a)/(a*s*s+np.sqrt(a)*s/q+1)
    else:
        transfer=a*(a*s*s+np.sqrt(a)*s/q+1)/(s*s+np.sqrt(a)*s/q+a)
    return 20*np.log10(abs(transfer))

def run(y,kind,folder):
    roots=[110.] if kind=="low" else [220.,261.63,329.63]
    hz,owner=harmonics(roots)
    takes={g:section(y,t0,t1,roots,hz) for t0,t1,g in SECTIONS[kind]}
    s0,n0,_=takes[0]
    measured={}
    with (folder/(kind+"-measurements.csv")).open("w") as output:
        print("gain_dB,frequency_Hz,root_Hz,relative_dB,frames",file=output)
        for g,(signal,floor,frames) in takes.items():
            if g==0:continue
            use=(s0>2.5*n0)&(signal>2.5*floor)
            f,delta=hz[use],20*np.log10(signal[use]/s0[use])
            measured[g]=(f,delta,owner[use])
            for freq,d,root in zip(f,delta,owner[use]):print(f"{g},{freq:.1f},{root:.2f},{d:.3f},{frames}",file=output)
    def residual(p,g):
        f,d,root=measured[g]
        e=shape(f,kind,np.exp(p[0]),np.exp(p[1]),g)-d
        return center_by_note(e,root)
    fc0={"low":250,"mid":1100,"high":3000}[kind]
    fit=least_squares(lambda p:np.concatenate([residual(p,g) for g in [-15,15]]),
        np.log([fc0,.7]),bounds=(np.log([55,.28]),np.log([8500,2.])),loss="soft_l1",f_scale=2.)
    fc,q=np.exp(fit.x)
    print(f"{kind}  fc={fc:.0f}Hz Q={q:.3f}")
    for g,(f,d,root) in measured.items():
        err=residual(fit.x,g)
        print(f"  {g:+5.1f} dB  {len(f)} lines  {f.min():.0f}–{f.max():.0f}Hz"
              f"  median shape error {np.median(abs(err)):.2f}dB"
              f"  flat comparison {np.median(abs(center_by_note(d,root))):.2f}dB")

def center_by_note(x,root):
    # Each member of a manually played chord can change level separately.
    y=x.copy()
    for note in np.unique(root):
        use=root==note
        y[use]-=np.median(x[use])
    return y

if __name__=="__main__":
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("video",type=Path);p.add_argument("output",type=Path)
    args=p.parse_args()
    args.output.mkdir(parents=True,exist_ok=True)
    print("container SHA-256",hashlib.sha256(args.video.read_bytes()).hexdigest())
    raw=subprocess.check_output(["ffmpeg","-v","error","-i",str(args.video),"-map","0:a:0",
                                 "-t","204","-ar",str(RATE),"-ac","1","-f","f32le","-"])
    x=np.frombuffer(raw,dtype='<f4')
    for k in SECTIONS:run(x,k,args.output)
