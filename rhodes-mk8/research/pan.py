"""Measure stereo modulation independent of playing level from public prototype takes.
Usage: pan.py SQUARE.wav RAMP.wav AUDIO_RATE.wav CAPTURE_DIR
"""
import sys
from pathlib import Path
import numpy as np
import soundfile as sf
from scipy.signal import welch,find_peaks

def ratio(path):
    x,r=sf.read(path)
    size=round(.001*r);v=np.sqrt((x[:len(x)//size*size].reshape(-1,size,2)**2).mean(1))
    total=v.sum(1);b=total>.015*total.max()
    z=(v[:,0]-v[:,1])/(total+1e-12)
    return np.interp(np.arange(len(z)),np.flatnonzero(b),z[b]),r/size

for clp,model in zip(sys.argv[1:4],['square','ramp','audio']):
    for label,path in [('recording',clp),('model',Path(sys.argv[4])/f'{model}.wav')]:
        z,rate=ratio(path)
        f,p=welch(z,fs=rate,nperseg=min(16384,len(z)),nfft=32768)
        ix=find_peaks(p)[0];ix=ix[f[ix]>.15];i=ix[np.argmax(p[ix])]
        harmonics=[]
        for h in [2,3,5]:
            if h*f[i]>f[-1]:continue
            j=np.flatnonzero(abs(f-h*f[i])<max(.15,.04*f[i]));harmonics.append(round(10*np.log10(max(p[j].max(),1e-20)/p[i]),1))
        print(model,label,round(f[i],3),np.round(np.quantile(z,[.02,.98]),3),harmonics)
