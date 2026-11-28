"""Matched Stage 73 late Mark I harp and MK8 direct send, all tone controls neutral.
Usage: compare_stage.py MK8_CAPTURE_DIR SCRATCH_DIR. Build stage73 --example probe first.
"""
from pathlib import Path
import sys,subprocess
import numpy as np,soundfile as sf
from scipy.signal import welch

captures,path=map(Path,sys.argv[1:]);path.mkdir(exist_ok=True,parents=True)
for n in [36,60,84]:
    f0=440*2**((n-69)/12)
    for v in [48,80,112]:
        out=path/f'stage-{n}-{v}.wav'
        subprocess.run(['cargo','run','--release','--quiet','-p','stage73','--example','probe','--','note',str(out),'48000',str(n),str(v/127),'1.2','0','0','late_i','harp'],check=True)
        row=[]
        for wav in [out,captures/f'n{n}-v{v}.wav']:
            x,r=sf.read(wav,always_2d=True);y=x[:,0];z=y[1440:5760]
            f=np.fft.rfftfreq(65536,1/r);a=abs(np.fft.rfft(z*np.hanning(len(z)),65536))
            h=[a[abs(f-k*f0)<max(7,.015*f0)].max() for k in [1,2,3]]
            f,p=welch(y[:1680],fs=r,nperseg=1024,nfft=4096)
            row.extend([20*np.log10(h[1]/h[0]),20*np.log10(h[2]/h[0]),10*np.log10(p[f>1800].sum()/p.sum())])
        print(n,v,*np.round(row,1))
