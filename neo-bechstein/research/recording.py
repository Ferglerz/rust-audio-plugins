"""Compare two opening dyads and the first repeat in John Hunt's HMV C.2567 transfer."""
import argparse
import hashlib
import subprocess
from pathlib import Path

import numpy as np
import soundfile as sf

# Onsets estimated on the transferred disc from the appearance of the new fundamentals.
# The first chord rings under the second, whose own remnants can carry into the repeat.
EVENTS = [(2.722, (65, 68), ()), (6.225, (66, 69), (65, 68, 73, 77)),
          (9.357, (65, 68), (66, 69, 72, 75))]
DIGEST = '902f572ea578073a3f3b5098de7662960155b40e95cefd194779cc63fb57bb00'

def nominal(note):
    return 440 * 2 ** ((note - 69) / 12)

def spectrum(samples, rate, start):
    window = samples[round(start*rate):round((start+.16)*rate)]
    size = 1 << (len(window)*8 - 1).bit_length()
    return (np.fft.rfftfreq(size, 1/rate),
            np.abs(np.fft.rfft(window*np.hanning(len(window)),size)))

def maximum(freq, size, expected, radius):
    indices = np.flatnonzero(abs(freq-expected) < radius)
    index = indices[np.argmax(size[indices])]
    return freq[index], size[index]

def describe(freq, size, note):
    f, amp = maximum(freq,size,nominal(note),nominal(note)*.027)
    partials = [maximum(freq,size,f*h,max(7.,f*h*.007))[1] for h in range(1,6)]
    return f, 20*np.log10(np.maximum(np.array(partials)/amp,10**(-38/20)))

def main(transfer, folder):
    if hashlib.sha256(transfer.read_bytes()).hexdigest() != DIGEST:
        raise ValueError('the recording does not match the HMV C.2567 Opus transfer')
    bits = subprocess.check_output(['ffmpeg','-v','error','-i',str(transfer),'-t','11',
                                    '-ar','22050','-ac','1','-f','f32le','-'])
    record=np.frombuffer(bits,dtype='<f4')
    print('onset,note,record_Hz,model_Hz,order,record_dBc,model_dBc,difference_dB')
    for onset, notes, previous in EVENTS:
        wav = 'midi-' + '+'.join(map(str,notes)) + '-v80.wav'
        model, fs = sf.read(folder/wav)
        rec_spectrum = spectrum(record,22050,onset+.05)
        model_spectrum = spectrum(model[:,2],fs,.02+.05)
        for note in notes:
            f, observed = describe(*rec_spectrum,note)
            fm, predicted = describe(*model_spectrum,note)
            # Mask overlaps with other keys close enough to enter the same Hann main lobe.
            others = [nominal(p)*(f/nominal(note))*k for p in (*notes,*previous)
                      if p!=note for k in range(1,9)]
            for h,(a,b) in enumerate(zip(observed,predicted),1):
                if h>1 and all(abs(h*f-other)>2/.16 for other in others):
                    print(f'{onset:.3f},{note},{f:.2f},{fm:.2f},{h},{a:.2f},{b:.2f},{b-a:.2f}')

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('transfer',type=Path)
    parser.add_argument('folder',type=Path)
    arguments=parser.parse_args()
    main(arguments.transfer,arguments.folder)
