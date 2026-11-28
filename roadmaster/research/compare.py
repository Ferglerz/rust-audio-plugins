"""Compare the three modeled courses and two obtainable recording windows."""
import argparse
from pathlib import Path
import numpy as np
from scipy.signal import find_peaks, resample_poly
import soundfile as sf

def rms(a):
    return np.sqrt(np.mean(a*a, axis=0))

def db(x):
    return 20*np.log10(np.maximum(1e-12,x))

def partials(signal, rate, fundamental, start, duration):
    x = signal[round(rate*start):round(rate*(start+duration))]
    spectrum = np.abs(np.fft.rfft(np.hanning(len(x))*x, 65536))
    hz = np.fft.rfftfreq(65536, 1/rate)
    peaks = find_peaks(spectrum)[0]
    near = peaks[np.abs(hz[peaks]-fundamental)<9]
    i = near[np.argmax(spectrum[near])]
    pitch = hz[i]
    bins = np.round(hz/pitch)
    harmonic = (bins>=1) & (np.abs(hz-bins*pitch)<12)
    band = (hz>70) & (hz<1550)
    concentration = (spectrum[harmonic & band]**2).sum() / (spectrum[band]**2).sum()
    partial = np.array([np.max(spectrum[np.abs(hz-n*pitch)<14]) for n in range(1,7)])
    return np.r_[pitch,concentration,db(partial/partial[0])]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root',type=Path)
    a = parser.parse_args()
    print('MIDI | sustained sensor and board peak (V, m/s) | foreign bars relative to struck | far gap change | sustain release energy change')
    for i,n in enumerate((48,60,84)):
        def read(s):
            x,r=sf.read(a.root/f'{n}-{s}.wav')
            assert x.shape==(r,4) and np.isfinite(x).all()
            return x,r
        p,r=read('pedal');s,_=read('stock');f,_=read('fixed');g,_=read('far')
        assert np.array_equal(p[:,3],g[:,3])
        assert np.max(np.abs(f[:,3]))==0 and np.max(np.abs(np.delete(f[:,:3],i,axis=1)))==0
        assert np.max(np.abs(np.delete(p[:,:3],i,axis=1)),axis=0).min()>1e-8
        gain=db(rms(g[:,i])/rms(p[:,i]))
        assert abs(gain+20.*np.log10(np.e)/3.5) < 0.0001
        tail=slice(round(.48*r),round(.85*r))
        release=db(rms(p[tail,i])/rms(s[tail,i]))
        print(f'{n} | {np.max(abs(p[:,i])):.6f}, {np.max(abs(p[:,3])):.6f} | '
              f'{db(rms(np.delete(p[:,:3],i,axis=1))/rms(p[:,i]))} | {gain:.2f} dB | {release:.2f} dB')
    if not (a.root/'luthier.wav').exists():
        return
    hardware,hardware_rate=sf.read(a.root/'luthier.wav')
    print('C3 | fundamental Hz | harmonic power fraction 70–1550 Hz | H1–H6 dB relative to H1')
    for t in (15.586,19.385):
        print(f'owner {t:.3f}',np.round(partials(hardware,hardware_rate,130.81,t,.130),2))
    model,rate=sf.read(a.root/'48-pedal.wav')
    for c,label in ((0,'bass sensor'),(3,'board surface')):
        print(label,np.round(partials(model[:,c],rate,130.81,.135,.130),2))

    # Listen to the same held windows used above, in order. Match levels only
    # for this listening file; the four-channel signal files retain SI units.
    takes = []
    for wave, fs, start in [(hardware,hardware_rate,15.586),(hardware,hardware_rate,19.385),
                            (model[:,0],rate,.135),(model[:,3],rate,.135)]:
        take = wave[round(fs*start):round(fs*(start+.130))]
        common=np.gcd(fs,48000)
        take=resample_poly(take,48000//common,fs//common)
        fade=np.linspace(0.,1.,96)
        take[:96]*=fade
        take[-96:]*=fade[::-1]
        takes.extend((take*(0.15/rms(take)),np.zeros(4800)))
    sf.write(a.root/'c3-owner-owner-sensor-board.wav',np.concatenate(takes[:-1]),48000,subtype='FLOAT')

    print('Sensor fraction | C3 H2 H3 H4 dB relative to H1')
    for fraction in (18,20,22,24,25,26,28,32):
        y,rate=sf.read(a.root/f'bass-position-{fraction}.wav')
        print(f'{fraction/100:.2f}',np.round(partials(y,rate,130.81,.085,.130)[3:6],2))

if __name__=='__main__':
    main()
