"""Compare factory prototype and independent production instrument to generated captures.

Usage: recordings.py FLAT.wav BONNERS.wav MODEL_DIR
Use 48 kHz source audio. Bonners is the full 2024-09-29 video, including its intro.
"""
import sys
from pathlib import Path
import numpy as np
import soundfile as sf


def spectrum(path,t,n,window=.09):
    with sf.SoundFile(path) as wav:
        sr=wav.samplerate
        wav.seek(round(t*sr))
        x=wav.read(round(window*sr),always_2d=True).mean(1)
    hz=440*2**((n-69)/12)
    f=np.fft.rfftfreq(65536,1/sr)
    mag=abs(np.fft.rfft(np.hanning(len(x))*x,65536))
    peaks=[]
    for i in range(1,8):
        band=np.flatnonzero(abs(f-i*hz)<max(7,hz*i*.012))
        peaks.append(band[np.argmax(mag[band])])
    return f[peaks[0]],20*np.log10(mag[peaks[1:]]/mag[peaks[0]])


if __name__=='__main__':
    flat,production,directory=sys.argv[1:]
    captures=Path(directory)
    print('prototype C2, observed / modeled H2 through H7 (dBc)')
    for t in [.08,18.31]:
        f,obs=spectrum(flat,t,36,.13)
        options=[]
        for p in captures.glob('n36-v*.wav'):
            v=int(p.stem.split('-v')[1])
            _,m=spectrum(p,.04,36,.13)
            options.append((np.sqrt(np.mean((obs-m)**2)),v,m))
        err,v,m=min(options,key=lambda x:x[0])
        print(t,round(f,2),v,np.round(obs,1),np.round(m,1),round(err,2))
    print('production notes: MIDI, t, F0, inferred velocity, H2/hardware/model, H3/hardware/model')
    # H2 selects the unknown impact velocity. H3 remains independent.
    for n,t,age in [(60,553.51,.04),(74,563.32,.055),(75,563.50,.04),(80,565.81,.05),
                    (79,566.00,.035),(52,574.70,.09),(62,581.10,.05)]:
        f,obs=spectrum(production,t,n,.07)
        fits=[]
        for p in captures.glob(f'n{n}-v*.wav'):
            _,m=spectrum(p,age,n,.07)
            fits.append((abs(m[0]-obs[0]),int(p.stem.split('-v')[1]),m))
        _,v,m=min(fits,key=lambda x:x[0])
        print(n,t,round(f,2),v,*np.round([obs[0],m[0],obs[1],m[1]],1))

    from scipy.signal import butter,sosfiltfilt
    print('D5 release, first 10 / 20 dB fall from −3 dB (ms)')
    for path,around in [(production,566.61),(captures/'n74-v80.wav',1.2)]:
        with sf.SoundFile(path) as wav:
            sr=wav.samplerate
            wav.seek(round((around-.16)*sr))
            x=wav.read(round(.44*sr),always_2d=True).mean(1)
        hz=440*2**((74-69)/12)
        band=sosfiltfilt(butter(3,[hz-55,hz+55],fs=sr,btype='bandpass',output='sos'),x)
        k=round(.005*sr)
        env=np.sqrt(np.convolve(band**2,np.ones(k)/k,'same'))
        plateau=np.median(env[round(.06*sr):round(.105*sr)])
        db=20*np.log10(env/plateau+1e-12)
        ix=round(.135*sr)+np.flatnonzero(db[round(.135*sr):]<-3)[0]
        print(*[round(1000*np.flatnonzero(db[ix:]<q)[0]/sr,2) for q in [-13,-23]])
