"""Check the Cembalet against the Hohner drawings and trace physical action events."""
import argparse
import hashlib
from pathlib import Path
import subprocess
import tempfile

import numpy as np
from scipy.io import wavfile

RATE = 96000


def carrier_detector(f, pf):
    """Harmonic balance of the driven physical tank at carrier and 40 sidebands.

    Solve I = V/R + d(C(t)V)/dt + integral(V)/L. Rectify 1024 carrier phases
    per audio observation. The Rust receiver instead integrates two slowly varying
    states and evaluates the rectifier in closed form.
    """
    w = 2 * np.pi * 1.75e6
    c = 105e-12
    r = 28 / (w * c)
    l = 1 / (w * (w * c - 2**-0.5 / r))
    k = np.arange(-20, 21)
    freq = w + 2 * np.pi * f * k
    y = np.diag(1 / r + 1j * (freq * c - 1 / (freq * l)))
    # C(t) = C0 + delta*sin(2*pi*f*t); the derivative uses the row's frequency.
    for i, omega in enumerate(freq):
        if i:
            y[i, i-1] += omega * pf * 1e-12 / 2
        if i+1 < len(k):
            y[i, i+1] -= omega * pf * 1e-12 / 2
    stimulus = np.zeros_like(k, dtype=complex)
    stimulus[20] = 3 / r
    volt = np.linalg.solve(y, stimulus)
    t = np.arange(RATE // 10, RATE // 5) / RATE
    rf = np.exp(2j * np.pi * t[:, None] * f * k) @ volt
    # Integrate positive grid current over the carrier period at each time.
    rectified = np.zeros_like(t)
    for phase in np.arange(1024) * (2 * np.pi / 1024):
        rectified += np.maximum(0, np.real(rf * np.exp(1j * phase)) - .38) / 1024
    return t, rectified


def check_rf(root):
    rows = np.genfromtxt(root / 'rf.csv', delimiter=',', names=True)
    for f in np.unique(rows['tone_hz']):
        r = rows[rows['tone_hz'] == f]
        t, ref = carrier_detector(f, r['cap_mod_pF'][0])
        rust = r['detector_V'][RATE // 10:]
        result = []
        for harmonic in [1, 2]:
            basis = np.exp(-2j * np.pi * harmonic * f * t)
            ratio = (rust @ basis) / (ref @ basis)
            result.append((20 * np.log10(abs(ratio)), np.angle(ratio, deg=True)))
        print(f'RF {f:g} Hz H1/H2 deviations (dB, degrees)', np.round(result, 4).tolist())
        # H2 is near a null on the tuned flank. Bound H1 tightly, and
        # bound harmonic error in volts relative to the reference fundamental.
        fundamental = abs(ref @ np.exp(-2j * np.pi * f * t))
        max_error = max(abs((rust-ref) @ np.exp(-2j*np.pi*h*f*t)) / fundamental for h in [1,2])
        if abs(result[0][0]) > .1 or max_error > .02:
            raise ValueError('Receiver differs from resolved RF solution')


def board_sweep(loads):
    text = Path(__file__).with_name('cembalet-650720.cir').read_text()
    for name, node, old, load in zip(['Low','Mid','High'],['low','mid','high'],
                                     ['10meg','10meg','100k'],loads):
        text=text.replace(f'Rload{name} {node} 0 {old}', f'Rload{name} {node} 0 {load:g}')
    with tempfile.TemporaryDirectory() as tmp:
        src=Path(tmp)/'board.cir'
        src.write_text(text)
        subprocess.run(['ngspice','-b',str(src)],cwd=tmp,check=True,capture_output=True)
        spice=np.loadtxt(Path(tmp)/'response.txt')
    return spice[:,0],spice[:,1::2]+1j*spice[:,2::2]


def check_n(root):
    freq,response=board_sweep([10e6,10e6,100e3])
    impulse = np.loadtxt(root / '650720-impulse.csv', delimiter=',', skiprows=1)
    rate = 192000
    bins = np.fft.rfftfreq(len(impulse), 1 / rate)
    hz = rate / np.pi * np.arctan(np.pi * freq / rate)
    model = np.array([np.interp(hz, bins, v) for v in np.fft.rfft(impulse, axis=0).T]).T
    ratio = model / response
    gain = np.max(np.abs(20 * np.log10(abs(ratio))), axis=0)
    phase = np.max(np.abs(np.angle(ratio, deg=True)), axis=0)
    print('650720 20 Hz–20 kHz max ΔdB and degrees per jack',gain.round(4), phase.round(4))
    if max(gain) > .03 or max(phase) > .2:
        raise ValueError('650720 SPICE comparison failed')

    triad=np.loadtxt(root/'650720-triad.csv',delimiter=',',skiprows=1)
    count=4*len(triad)
    spectrum=np.fft.rfft(triad,n=count,axis=0)
    freqs=np.fft.rfftfreq(count,1/rate)
    peaks=[];levels=[]
    for i,load in enumerate([100e3,200e3,7e3]):
        loads=[1e13]*3
        loads[i]=load
        _,loaded=board_sweep(loads)
        ratio=np.interp(freqs,hz,loaded[:,i]/response[:,i])
        volts=np.fft.irfft(spectrum[:,i]*ratio,n=count)[:len(triad)]
        peaks.append(abs(volts).max())
        levels.append(rms(volts))
    print('Factory triad, individually loaded ports, peak and RMS mV',
          np.round(1000*np.array(peaks),1),np.round(1000*np.array(levels),1))


def rms(x):
    return np.sqrt(np.mean(x*x))


def action(root):
    traces=np.genfromtxt(root / 'mechanism.csv', delimiter=',', names=True)
    for note in [36,60,84]:
        t=traces[traces['midi']==note]
        escape=t[1:][np.diff(t['armed'])<0]
        rearm=t[1:][np.diff(t['armed'])>0]
        last=t[t['time_s']>=39800/96000]
        landing=last[last['damper_N']<0][0]['time_s']
        print(f'MIDI {note} active escapes',escape['key_fraction'].round(4),
              'escapes during partial depth/short return',len(escape)-2,
              'damper landing/rearm after final release ms',
              np.round(1000*(np.array([landing,rearm[-1]['time_s']])-39800/96000),2))
        for label in ['tube','650720']:
            def read(v):
                fs,x=wavfile.read(root / f'{label}-{note}-{v}.wav')
                assert fs==48000
                return x.astype(float)
            slow, fast = read(35), read(105)
            dyn=20*np.log10(rms(fast[960:4800])/rms(slow[960:4800]))
            tail=20*np.log10(rms(fast[20400:22320])/rms(fast[15600:17520]))
            print(f'  {label} fast/slow and late tail dB {dyn:+.2f} {tail:.2f}')
    for label in ['tube','650720']:
        _,x=wavfile.read(root / f'cembalet-{label}-audition.wav')
        if not np.all(np.isfinite(x)):
            raise ValueError('Nonfinite jack waveform')


def compare_recording(root, path):
    # The manufacturer identifies the performance as its recorded Cembalet.
    # Preserve four repeated bass phrases. Relative level removes recording gain.
    import soundfile as sf
    sha = '68f38135b408b5be1c2dff2902848920a1f06226dc3aefcd10299b7365e74760'
    if hashlib.sha256(path.read_bytes()).hexdigest()!=sha:
        raise ValueError('Expected Cinematique Instruments Just Playing demo')
    ref, sr = sf.read(path, always_2d=True)
    hz=440*2**((44-69)/12)
    def fit(x,rate,start):
        n=round(.20*rate)
        w=np.hanning(n)*np.exp(-2j*np.pi*hz*np.arange(n)/rate)
        ts=start+np.arange(13)*.08
        db=np.array([20*np.log10(abs(w@x[round(t*rate):round(t*rate)+n])) for t in ts])
        slope,intercept=np.polyfit(ts,db,1)
        return slope,rms(db-(ts*slope+intercept))
    matches=[fit(ref.mean(axis=1),sr,t) for t in [2.30,10.35,18.45,26.40]]
    print('Publisher demo G#2 slope (dB/s), fit residual (dB)',np.round(matches,2))
    for label in ['tube','650720']:
        rate,x=wavfile.read(root / f'{label}-held-44.wav')
        print(label,'isolated G#2',np.round(fit(x.astype(float),rate,.20),2))


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('renders',type=Path)
    parser.add_argument('--sampled-performance',type=Path)
    args=parser.parse_args()
    check_rf(args.renders)
    check_n(args.renders)
    action(args.renders)
    if args.sampled_performance:
        compare_recording(args.renders,args.sampled_performance)
