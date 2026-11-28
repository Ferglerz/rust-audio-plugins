"""Compare matching 48 kHz note events and a controlled shaft-length substitution.

Build stage73 --example probe and vintage-vibe-64 --example vv64_probe in release.
Run from the workspace root, with a scratch directory as the only argument.
The Stage 73 source tap includes its stock late Mark I pickup-group weight.
"""
from pathlib import Path
import sys, subprocess
import numpy as np
import soundfile as sf
from scipy.signal import welch

out=Path(sys.argv[1])
out.mkdir(exist_ok=True,parents=True)
subprocess.run(['target/release/examples/vv64_probe','comparison',out],check=True)
def energy(y,rate):
    # H2 relative to H1 in the 30–120 ms body and upper-band attack energy share.
    z=y[round(.03*rate):round(.12*rate)]
    freq=np.fft.rfftfreq(65536,1/rate)
    amp=abs(np.fft.rfft(z*np.hanning(len(z)),65536))
    bands=[amp[abs(freq-i*f0)<max(7.,f0*.015)].max() for i in [1,2]]
    f,p=welch(y[:round(.035*rate)],fs=rate,nperseg=1024,nfft=4096)
    return 20*np.log10(bands[1]/bands[0]),10*np.log10(p[f>1800].sum()/p.sum())
print('note,velocity,stage_source_H2,stage_jack_H2,vv_source_H2,vv_jack_H2,stage_source_hi,vv_source_hi,vv_jack_hi,vv_smooth_length_hi')
for n in (40,60,84):
    f0=440.*2**((n-69)/12)
    for v in (48,80,110):
        native=[]
        for port in ('harp','jack'):
            wav=out/f'stage-{n}-{v}-{port}.wav'
            cmd=['cargo','run','--release','--quiet','-p','stage73','--example','probe','--','note',str(wav),'48000',str(n),str(v/127),'1','0','0','late_i']
            subprocess.run(cmd+(['harp'] if port=='harp' else []),check=True)
            y,sr=sf.read(wav)
            native.append(energy(y,sr))
        model,sr=sf.read(out/f'vv-{n}-{v}.wav')
        vv=[energy(model[:,i],sr) for i in [0,2,3]]
        values=[native[0][0],native[1][0],vv[0][0],vv[1][0],native[0][1],vv[0][1],vv[1][1],vv[2][1]]
        print(f'{n},{v},'+','.join(f'{x:.2f}' for x in values))
