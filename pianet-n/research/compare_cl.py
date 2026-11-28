"""Compare five C/L terminals to separate AC decks and measure knee/lamp behavior."""
import argparse
import hashlib
from pathlib import Path
import subprocess
import tempfile

import numpy as np
from scipy.io import wavfile


def sweep(root):
    data=np.loadtxt(root/'boards.csv',delimiter=',',skiprows=1)
    rate=192000
    fft=np.fft.rfft(data,axis=0)
    hertz=np.fft.rfftfreq(len(data),1/rate)
    for deck,columns in [('pianet-c-630228.cir',slice(0,2)),('pianet-l-e71.cir',slice(2,5))]:
        with tempfile.TemporaryDirectory() as temp:
            subprocess.run(['ngspice','-b',str(Path(__file__).with_name(deck).resolve())],
                           cwd=temp,capture_output=True,check=True)
            r=np.loadtxt(Path(temp)/'response.txt')
        hz=rate/np.pi*np.arctan(np.pi*r[:,0]/rate)
        ref=r[:,1::2]+1j*r[:,2::2]
        rust=np.asarray([np.interp(hz,hertz,s) for s in fft[:,columns].T]).T
        ratio=rust/ref
        gain=np.max(np.abs(20*np.log10(np.abs(ratio))),axis=0)
        phase=np.max(np.abs(np.angle(ratio,deg=True)),axis=0)
        print(deck,'max error dB',gain.round(4),'degrees',phase.round(4))
        if max(gain)>.05 or max(phase)>.25:
            raise ValueError('Circuit port transfer disagreement')


def measure(root):
    data=np.loadtxt(root/'controls.csv',delimiter=',',skiprows=1)
    ref=data[(data[:,0]==1)&(data[:,1]==0),3:]
    for knee,on in [(0,0),(.5,0),(1,0),(1,1)]:
        d=data[(data[:,0]==knee)&(data[:,1]==on),3:]
        ratio=np.sqrt(np.mean(d**2,axis=0)/np.mean(ref**2,axis=0))
        print('knee/vibrato',knee,on,'RMS dB',np.round(20*np.log10(np.maximum(1e-12,ratio)),2))
        if on:
            n=len(d);phase=np.arange(n)/48000
            # The RC astable cycle is approximately 6.01 Hz.
            amp=[abs(np.exp(-2j*np.pi*(440+f)*phase) @ d[:,0]) for f in [-6,0,6]]
            print('C lamp sidebands dBc',np.round(20*np.log10(np.array(amp)[[0,2]]/amp[1]),2))
    for note in [29,45,64,81,89]:
        sr,data=wavfile.read(root/f'key-{note}.wav')
        if not np.all(np.isfinite(data)):
            raise ValueError('Nonfinite terminal voltage')
        print('MIDI',note,'peak volts',np.max(abs(data),axis=0).round(4))


def recording(root, variant, path):
    from scipy.signal import resample_poly
    import soundfile as sf

    refs={
        'c':('bb918a9dc5d78cf895335419676d9e5fb4585667d9212bcd39dea145a78fb3f3',
             [(55,57.08),(57,53.20),(60,27.72),(62,49.64),(64,49.36),(67,57.04)]),
        'l':('577d2de8f1a82747430c1883c83890fa6e9be4b2c2ad2e6cd3cb8e1ca41bf72b',
             [(52,10.4),(57,51.24),(60,191.52),(62,21.80),(64,9.16),(67,103.32),
              (69,160.12),(72,2.56),(81,104.88)])
    }
    sha,passages=refs[variant]
    if hashlib.sha256(path.read_bytes()).hexdigest()!=sha:
        raise ValueError('Unexpected owner recording. Reference is the 48 kHz format 251 decode.')
    x,rate=sf.read(path,always_2d=True)
    assert rate==48000
    x=resample_poly(x.mean(axis=1),1,12)
    def slope(x,note,t):
        sr=4000;size=720
        window=np.hanning(size)*np.exp(-2j*np.pi*440*2**((note-69)/12)*np.arange(size)/sr)
        times=np.arange(17)*.04
        db=[20*np.log10(abs(window @ x[round((t+d)*sr):round((t+d)*sr)+size])) for d in times]
        slope,offset=np.polyfit(times,db,1)
        return slope,np.mean((np.array(db)-offset-times*slope)**2)**.5
    differences=[]
    for note,t in passages:
        _,model=wavfile.read(root/f'{variant}-held-{note}.wav')
        model=resample_poly(model.astype(float),1,12)
        reference,resid=slope(x,note,t)
        modeled,_=slope(model,note,.10)
        differences.append(abs(modeled-reference))
        print(f'{variant} {note} at {t:.2f}s: reference {reference:.2f} (fit RMS {resid:.2f}), model {modeled:.2f} dB/s')
    print(variant,'median absolute decay difference',round(np.median(differences),2),'dB/s')


if __name__=='__main__':
    opt=argparse.ArgumentParser(description=__doc__)
    opt.add_argument('renders',type=Path)
    opt.add_argument("--c-demo",type=Path)
    opt.add_argument("--l-demo",type=Path)
    args=opt.parse_args()
    sweep(args.renders)
    measure(args.renders)
    if args.c_demo: recording(args.renders,"c",args.c_demo)
    if args.l_demo: recording(args.renders,"l",args.l_demo)
