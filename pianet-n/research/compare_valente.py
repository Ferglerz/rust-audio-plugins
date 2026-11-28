"""Compare production Valente performances and the estimated passive network."""
import argparse
import csv
import hashlib
from pathlib import Path
import numpy as np
import soundfile as sf
from scipy.signal import resample_poly
from compare_claviset import pair, partials
from compare_elepian import decay

# One window per pitch/source, chosen on harmonic fit and independent of model.
CASES = {
    'doykin': [(37,47.50),(39,12.28),(44,41.62),(51,21.82),(63,1.42)],
    'fonseca': [(36,80.56),(38,84.82),(39,19.84),(40,85.78),(41,115.84),
                (43,54.34),(45,89.74),(48,78.22),(53,34.96),(55,57.52),
                (67,42.64),(69,51.82),(72,102.40),(74,62.56),(76,137.92)],
}
DECAY = {
    'doykin': [(44,41.10),(46,30.50),(49,23.26),(51,25.30),(58,66.98),(65,74.98),(70,2.34)],
    'fonseca': [(39,165.02),(41,114.70),(42,54.14),(46,142.50),(48,138.98),(49,138.06),
                (51,153.26),(53,99.42),(55,58.82),(58,12.26),(60,78.10),(64,73.82),
                (65,149.22),(67,100.70),(69,88.22),(70,157.46),(72,89.18),(76,112.86)],
}
DIGESTS = {'doykin':'8d25021e990ad1f6f8581ce6d242102655c035544b6e2c5c703158b903bbb75e',
           'fonseca':'4531e0ac4d3631fb2d92e4ee17db83bf9f1622928c7a0d60d3898e576c5337d9'}


def transfer(f, rate, load, volume, brilliance):
    s=2j*rate*np.tan(np.pi*f/rate)
    parallel=lambda *z: 1/sum(1/x for x in z)
    source=3400+s*.24
    cut=8200+100000*brilliance+1/(s*6.8e-9)
    top=parallel(max(.001,250000*(1-volume)),1/(s*470e-12))
    foot=parallel(max(.001,250000*volume),load,1/(s*470e-12))
    panel=parallel(cut,top+foot)
    return panel/(source+panel)*foot/(top+foot)


def electrical(root):
    worst=[0.,0.]
    for load in [100000,1000000]:
        for tone in [0,50,100]:
            for vol in [50,100]:
                y,s=sf.read(root/f'panel-{load}-{tone}-{vol}.wav')
                for f in np.geomspace(20,20000,17):
                    ratio=(y@np.exp(-2j*np.pi*np.arange(len(y))*f/s))/transfer(f,s,load,vol/100,tone/100)
                    worst=np.maximum(worst,[abs(20*np.log10(abs(ratio))),abs(np.angle(ratio,deg=True))])
    print('Terminal vs independent impedance, max magnitude dB and phase deg:',worst)
    assert max(worst)<.02
    for f in [110,440,3000,7000]:
        print(f,'Hz, dark/bright dB',*[round(20*np.log10(abs(transfer(f,88200,1e6,1,t))),2) for t in [0,1]])


def compare(root):
    for name,cases in CASES.items():
        file=root/f'{name}.wav';source,sr=sf.read(file)
        if sr!=22050 or source.ndim!=1 or hashlib.sha256(file.read_bytes()).hexdigest()!=DIGESTS[name]:
            raise ValueError(f'Unexpected audio for {name}')
        ref=resample_poly(source,1,2);rows=[];errs=[];samples=[]
        for n,t in cases:
            wave,rate=sf.read(root/f'note-{n}-80.wav'); model=resample_poly(wave[:,0],1,2)
            f,fit,h=partials(ref,t,n);m,_,m_h=partials(resample_poly(model,1,2),.26,n)
            assert fit>.90,(n,t,fit)
            errs.append(abs(np.maximum(-40.,h)-np.maximum(-40.,m_h)))
            samples.append(pair(source,t,model));
            for order,(a,b) in enumerate(zip(h,m_h),start=2):
                rows.append([n,t,f,m,fit,order,a,b])
        with (root/f'{name}-harmonics.csv').open('w',newline='') as out:
            w=csv.writer(out);w.writerow(['midi','time_s','reference_hz','model_hz','reference_fit','order','reference_dBc','model_dBc']);w.writerows(rows)
        sf.write(root/f'{name}-then-model.wav',np.concatenate(samples),22050,subtype='PCM_24')
        print(name,'H2–H6 median absolute error (40 dB floor)',round(np.median(errs),2),'by order',np.round(np.median(errs,axis=0),1))
        rows=[]
        for n,t in DECAY[name]:
            wave,rate=sf.read(root/f'note-{n}-80.wav'); measured=decay(source,sr,n,t); predicted=decay(wave[:,0],rate,n,.40)
            rows.append([n,t,measured,predicted]);
        with (root/f'{name}-decays.csv').open('w',newline='') as out:
            w=csv.writer(out);w.writerow(['midi','time_s','recorded_dBper_s','model_dBper_s']);w.writerows(rows)
        print(name,'decay median absolute error dB/s',round(np.median([abs(a-b) for _,_,a,b in rows]),2))


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('root',type=Path);args=parser.parse_args()
    electrical(args.root);compare(args.root)
