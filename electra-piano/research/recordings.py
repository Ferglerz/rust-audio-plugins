"""Compare repeatable Electra-Piano bench notes to an owner's effects-off performance."""
import argparse
from pathlib import Path
import numpy as np
import soundfile as sf

# Coarse attack search, isolated early ring, unmasked ring-down end (seconds
# after the detected onset). All notes come from 84P3RGgeqvU, format 251.
TAKES = [(48, 296.98, .14, 1.20), (65, 875.29, .17, .93),
         (80, 663.84, .075, .69), (48, 241.84, .16, .98), (65, 149.58, .20, .70),
         (80, 664.71, .075, .55)]


def extract(path, start, duration, chan=None):
    with sf.SoundFile(path) as f:
        sr = f.samplerate
        f.seek(round(start * sr))
        y = f.read(round(duration * sr), always_2d=True)
        return (y.mean(axis=1) if chan is None else y[:, chan]), sr


def spectrum(y, sr, seconds, hz, width):
    n = round(sr * width)
    signal = y[round(seconds*sr):][:n]
    w = np.hanning(n)
    transform = abs(np.fft.rfft(signal*w, 32768))
    bins = np.fft.rfftfreq(32768, 1/sr)
    def peak(freq):
        candidates = np.flatnonzero(abs(bins-freq) < max(18., freq*0.022))
        i = candidates[np.argmax(transform[candidates])]
        left, center, right = np.log(np.maximum(transform[i-1:i+2], 1e-20))
        offset = 0.5*(left-right)/(left-2*center+right)
        return (i+offset)*sr/32768, np.exp(center - .25*(left-right)*offset)
    f0, a0 = peak(hz)
    h = np.array([peak(h*f0)[1] for h in [2,3]])
    return f0, 20*np.log10(h/a0), 20*np.log10(a0)


def note_onset(y, sr, f0):
    # The reference includes melody preceding some notes. Use a coherent
    # fundamental-only detector to avoid marking the preceding chord's attack.
    times = np.arange(0, .22, .001)
    length = round(.016*sr)
    oscillator = np.exp(-2j*np.pi*f0*np.arange(length)/sr)*np.hanning(length)
    a = np.array([abs(y[round(t*sr):][:length]@oscillator) for t in times])
    return times[np.argmax(a[12:]-a[:-12])] + .008


def metrics(y, sr, start, nominal, early, last):
    f, ratios, _ = spectrum(y,sr,start+.020,nominal,.065)
    t=np.arange(early,last-.065,.025)
    level=np.array([spectrum(y,sr,start+u,nominal,.065)[2] for u in t])
    slope=np.polyfit(t,level,1)[0]
    return f,ratios[0],ratios[1],slope


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('notes', type=Path)
    parser.add_argument('dry', type=Path, help='mono 48 kHz PCM of 84P3RGgeqvU')
    parser.add_argument('--grall', type=Path, help='mono 48 kHz PCM of cMarNNEoMes')
    args=parser.parse_args()
    print('note | onset s | source | pitch Hz | H2/H1 dB | H3/H1 dB | early dB/s')
    for note, search, early, last in TAKES:
        nominal = 440*2**((note-69)/12)
        y,sr=extract(args.dry,search-.10,last+.40)
        onset=note_onset(y,sr,nominal)
        print(note, f'{search-.10+onset:.3f}', 'hardware', *[f'{x:.2f}' for x in metrics(y,sr,onset,nominal,early,last)])
        for speed in [0.35,0.85,1.7]:
            y,sr=extract(args.notes / f'{note}-{speed}-hold.wav',0,1.6,3)
            print(note, '-', speed, *[f'{x:.2f}' for x in metrics(y,sr,0,nominal,early,last)])

    if args.grall:
        # The preceding chord rings into the attack. Report an isolated
        # 65 ms ring window and bracket its age against the rendered tail.
        for t in [52.39, 53.31, 55.54]:
            y,sr=extract(args.grall,t,.08)
            f,ratios,_=spectrum(y,sr,0,130.81,.065)
            print(f'Grall C3 ring at {t:.2f} s: {f:.2f} Hz, {ratios.round(2)} dB H2/H3')
        y,sr=extract(args.notes/'48-0.85-hold.wav',0,.25,3)
        for t in [.08,.15,.18]:
            f,ratios,_=spectrum(y,sr,t,130.81,.065)
            print(f'Model C3 ring at {t:.2f} s: {f:.2f} Hz, {ratios.round(2)} dB H2/H3')


if __name__=='__main__': main()
