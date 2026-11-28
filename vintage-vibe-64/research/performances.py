"""Measure selected windows in the maker's two Mikko Taylor performances.

Usage: performances.py PART1_48K_MONO.wav PART2_48K_MONO.wav REFERENCE_CAPTURE_DIR
Run vv64_probe references DIR first. Unknown impact speed is estimated from H3/H1;
H2/H1 is an independently read value in the same window. No recording is redistributed.
"""
from pathlib import Path
import sys
import numpy as np
import soundfile as sf

# Part, MIDI pitch, window start in the YouTube encode, approximate note age.
# 50 and 53 are sustained low voices in polyphony. 71 opens the second performance alone.
windows = [(1,50,94.20,1.096), (1,53,179.20,.895), (1,81,179.50,.121),
           (2,59,5.68,.069), (2,62,69.47,.139), (2,69,95.44,.038),
           (2,71,1.30,.081), (2,74,96.50,.020), (2,76,125.98,.106)]


def harmonics(path, start, note):
    with sf.SoundFile(path) as wav:
        rate = wav.samplerate
        wav.seek(round(rate*start))
        y = wav.read(round(.09*rate))
    f = np.fft.rfftfreq(131072, 1/rate)
    spectrum = abs(np.fft.rfft(y*np.hanning(len(y)),131072))
    fundamental = 440*2**((note-69)/12)
    peaks = []
    for h in range(1,4):
        ix = np.flatnonzero(abs(f-h*fundamental) < max(6,.013*h*fundamental))
        peaks.append(ix[np.argmax(spectrum[ix])])
    db = 20*np.log10(spectrum[peaks]/spectrum[peaks[0]])
    return f[peaks[0]], db[1], db[2]


def attack(path, start, stop):
    with sf.SoundFile(path) as wav:
        r=wav.samplerate
        wav.seek(round(start*r))
        x=wav.read(round((stop-start)*r))
    w=round(.005*r)
    a=np.sqrt(np.convolve(x*x,np.ones(w)/w,'same'))
    return (np.flatnonzero(a>.9*a.max())[0]-np.flatnonzero(a>.1*a.max())[0])*1000/r


def release(path, off):
    with sf.SoundFile(path) as wav:
        r=wav.samplerate
        wav.seek(round((off-.07)*r))
        x=wav.read(round(.19*r))
    w=round(.005*r)
    a=np.sqrt(np.convolve(x*x,np.ones(w)/w,'same'))
    a/=np.median(a[round(.02*r):round(.05*r)])
    cross=round(.045*r)+np.flatnonzero(a[round(.045*r):]<10**(-3/20))[0]
    fall=np.flatnonzero(a[cross:]<10**(-23/20))[0]
    return 1000*fall/r


if __name__=='__main__':
    print('part,note,start_s,recorded_hz,recorded_H2_dBc,recorded_H3_dBc,inferred_velocity,model_H2_dBc,model_H3_dBc')
    for part,note,t,age in windows:
        f,h2,h3=harmonics(sys.argv[part],t,note)
        candidates=[]
        for path in Path(sys.argv[3]).glob(f'n{note}-v*.wav'):
            _,m2,m3=harmonics(path,age,note)
            v=int(path.stem.split('-v')[1])
            candidates.append((abs(m3-h3),v,m2,m3))
        _,v,m2,m3=min(candidates)
        print(f'{part},{note},{t:.2f},{f:.2f},{h2:.2f},{h3:.2f},{v},{m2:.2f},{m3:.2f}')
    model=Path(sys.argv[3])/'n71-v24.wav'
    print('first B4 attack 10–90% and damper fall −3 to −23 dB (ms)')
    print('hardware',attack(sys.argv[2],1.18,1.39),release(sys.argv[2],1.49))
    print('model',attack(model,0.,.17),release(model,1.4))
    print('first bending attack spectral peak, Hz and dBc, 8–36 ms after onset')
    for note,start,velocity in [(71,1.226,24),(69,95.407,40),(74,96.485,48),(76,125.878,88)]:
        row=[]
        f0=440*2**((note-69)/12)
        for path,t in [(sys.argv[2],start),(Path(sys.argv[3])/f'n{note}-v{velocity}.wav',0.)]:
            with sf.SoundFile(path) as wav:
                sr=wav.samplerate
                wav.seek(round((t+.008)*sr))
                x=wav.read(round(.028*sr))
            f=np.fft.rfftfreq(65536,1/sr)
            a=abs(np.fft.rfft(x*np.hanning(len(x)),65536))
            ix=np.flatnonzero((f>6.25*f0)&(f<8.6*f0))
            top=ix[np.argmax(a[ix])]
            row.extend((round(f[top]),round(20*np.log10(a[top]/a[abs(f-f0)<18].max()),1)))
        print(note,*row)
