"""Compare the production MK8 FX board to the oversampled Rust return path.

Usage: python3 rhodes-mk8/research/fx.py BONNERS.webm SYNTH_SHED.webm WORK_DIR
Both command-line inputs are the original complete programs. See fx.md for links.
The table's rate/center/depth/initial phase were identified from two out of
each three STFT frames. Stereo-direction comparisons use the remaining frames.
"""
import subprocess
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import butter, correlate, find_peaks, istft, sosfilt, stft

FS = 24000
N = 1536
STEP = 960
probe = Path("target/release/examples/mk8_fx_probe")
bonners, owner = sys.argv[1:3]
work = Path(sys.argv[3])
work.mkdir(parents=True, exist_ok=True)


def excerpt(source, start, end):
    audio = subprocess.check_output([
        "ffmpeg", "-v", "error", "-ss", str(start), "-i", source, "-t", str(end-start),
        "-vn", "-ar", str(FS), "-ac", "2", "-f", "f32le", "-",
    ])
    return np.frombuffer(audio, dtype="<f4").reshape(-1, 2).astype(float)


def run(setting, mono, phase_seconds=0.):
    pad = round(FS * phase_seconds)
    sf.write(work/"mono.wav", np.pad(mono, (pad, 0)), FS, subtype="FLOAT")
    path = work/(setting+".wav")
    subprocess.run([probe, setting, work/"mono.wav", path], check=True)
    return sf.read(path)[0][pad:]


def transform(a):
    return stft(a, FS, nperseg=N, noverlap=N-STEP, axis=0)


def width(a):
    return 20*np.log10(np.std(np.diff(a,axis=1))/(2*np.std(a.mean(axis=1))))


def direction(ref, model, fmax):
    f, _, reference = transform(ref)
    _, _, rendered = transform(model)
    errors = []
    dry = []
    for i in range(3, reference.shape[-1]-1, 3):
        a, b = reference[:,:,i].T
        c, d = rendered[:,:,i].T
        power = abs(a)**2+abs(b)**2
        peaks = find_peaks(power, distance=3)[0]
        p = peaks[(f[peaks]>70)&(f[peaks]<fmax)&(power[peaks]>0.001*power.max())]
        if len(p)<3:
            continue
        w = power[p]**0.22
        # Squared sine of the angle between the two complex stereo vectors.
        # Per-bin input gain and phase cancel from this metric.
        score = abs(a[p]*d[p]-b[p]*c[p])**2/(power[p]*(abs(c[p])**2+abs(d[p])**2)+1e-18)
        errors.append(np.average(score,weights=w))
        dry.append(np.average(abs(a[p]-b[p])**2/(2*power[p]),weights=w))
    return np.mean(errors), np.mean(dry)


def modulation_peaks(a):
    f,t,z = stft(a, FS, nperseg=N, noverlap=N-480, axis=0)
    l,r = z[:,0],z[:,1]
    e = abs(l)**2+abs(r)**2
    w = (e/(e.mean(axis=1,keepdims=True)+1e-15)).clip(0,3)*(e>e.max()*.0003)
    score = 0
    for feat in [(abs(l)**2-abs(r)**2)/(e+1e-15), 2*(l*r.conj()).imag/(e+1e-15)]:
        mean = (w*feat).sum(1,keepdims=True)/(w.sum(1,keepdims=True)+1e-12)
        spectrum = abs(np.fft.rfft(w*(feat-mean),8192,axis=1))**2
        score += spectrum[(f>100)&(f<4000)].sum(0)
    fr = np.fft.rfftfreq(8192,480/FS)
    peaks = find_peaks(score,distance=25)[0]
    strongest = sorted(peaks[(fr[peaks]>.2)&(fr[peaks]<12)],key=lambda k:score[k],reverse=True)
    return fr[strongest[:2]]


def sweep(setting, source, start, end, rate, center, excursion, phase):
    ref = excerpt(source,start,end)
    f,t,z = transform(ref)
    arg = 2*np.pi*rate*t+phase
    if setting.startswith("chorus"):
        fmax=1600
        lag = [(center+excursion*np.sin(arg+angle))*.001 for angle in (0,np.pi)]
        H = np.stack([1+sign*.62*np.exp(-2j*np.pi*f[:,None]*delay)
                      for sign,delay in zip([-1,1],lag)],axis=1)
        # The variable bucket clock transports each packet through a complete
        # line; its instantaneous delay follows the clock near the transit midpoint.
        padding_angle=phase+np.pi*rate*center*.001
    else:
        fmax=5000
        pole = [center * 2**(excursion*np.sin(arg+angle)) for angle in (0,np.pi/2)]
        H = np.stack([1+((1j*f[:,None]-p)/(1j*f[:,None]+p))**4 for p in pole],axis=1)
        padding_angle=phase
    # Estimate the unknown mono excitation from both recorded channels.
    # Only this excitation uses all frames. Direction is measured independently
    # of source gain and phase, and only on held-out harmonic bins.
    source_spec = np.sum(H.conj()*z,axis=1)/(np.sum(abs(H)**2,axis=1)+0.01)
    _,mono=istft(source_spec,FS,nperseg=N,noverlap=N-STEP)
    mono=mono[:len(ref)]
    result=run(setting,mono*0.1/np.max(abs(mono)),(padding_angle%(2*np.pi))/(2*np.pi*rate))
    print(setting, "side/mid dB",np.round([width(ref),width(result)],2),
          "direction error (board / mono)",np.round(direction(ref,result,fmax),3),
          "stereo modulation Hz",np.round([modulation_peaks(ref),modulation_peaks(result)],2).tolist())


def repeat_lags(a, around):
    # Whitening makes each octave contribute and suppresses repeated note pitches.
    f,t,z=stft(a,FS,nperseg=240,noverlap=180,axis=0)
    z[(f<400)|(f>6000)]=0
    z/=np.sqrt(np.mean(abs(z)**2,axis=-1,keepdims=True)+1e-12)
    values=[]
    for spectrum,near in zip(z.transpose(1,0,2),around):
        _,x=istft(spectrum,FS,nperseg=240,noverlap=180)
        ac=correlate(x,x,mode="full",method="fft")[len(x)-1:]
        lo=round((near-0.0015)*FS)
        i=lo+np.argmax(abs(ac[lo:lo+round(.003*FS)]))
        values.append((round(1000*i/FS,2),round(ac[i]/ac[0],3)))
    return values


def band_repeats(a, around):
    values=[]
    for band in [[400,1500],[4500,9000]]:
        channels=sosfilt(butter(3,band,fs=FS,btype="bandpass",output="sos"),a,axis=0)
        for x,near in zip(channels.T,around):
            ac=correlate(x,x,mode="full",method="fft")[len(x)-1:]
            i=round(FS*near)
            values.append(np.min(ac[i-10:i+10])/ac[0])
    return np.round(values,3)


def dynamics(x):
    y=x[:len(x)//480*480].reshape(-1,480)
    rms=np.sqrt(np.mean(y*y,axis=1))
    audible=rms>max(rms)*.025
    rms=rms[audible]
    return np.array([20*np.log10(np.percentile(rms,95)/np.percentile(rms,10)),
                     np.median(20*np.log10(np.max(abs(y[audible]),axis=1)/rms))])


if __name__ == "__main__":
    subprocess.run(["cargo","build","--release","-q","-p","rhodes-mk8","--example","mk8_fx_probe"],check=True)
    sweep("phaser-medium",owner,439,445.8,1.801,420,.67,-.479)
    sweep("phaser-slow",owner,450,456.9,.659,420,1.53,-2.581)
    sweep("phaser-fast",owner,460,468.4,4.68,420,1.30,-.275)
    sweep("chorus",owner,419,431.5,3.253,5.155,1.085,-2.525)
    sweep("chorus-bonners",bonners,1638,1653,3.228,6.487,.694,.662)
    rng=np.random.default_rng(53)
    dry=sosfilt(butter(2,5500,fs=FS,output="sos"),rng.normal(0,.03,8*FS))
    for setting,start,end,near in [("delay-long",1467,1478,[.4002,.387]),
                                   ("delay-short",1591,1597,[.1844,.1712])]:
        live=excerpt(bonners,start,end)
        rendered=run(setting,dry)
        print(setting,"recorded/model echo (ms, correlation)",repeat_lags(live,near),repeat_lags(rendered,near))
        if setting=="delay-long":
            print("400–1500 / 4500–9000 Hz L/R repeat correlation",band_repeats(live,near),band_repeats(rendered,near))
    clean=excerpt(owner,5,24).mean(1)
    for name,x in [("owner clean",clean),("owner compressor",excerpt(owner,403,412).mean(1)),
                   ("Bonners compressor",excerpt(bonners,1670,1694).mean(1)),
                   ("model compressor, owner clean input",run("comp",.3*clean/max(abs(clean))).mean(1))]:
        print(name,"audible 20ms RMS spread, median crest dB",np.round(dynamics(x),2))
