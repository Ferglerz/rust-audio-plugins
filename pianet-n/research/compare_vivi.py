"""Compare NMM C 1004, Emma Repairs’ separately identified samples and the model."""
import argparse
import csv
import hashlib
from io import BytesIO
from pathlib import Path
from zipfile import ZipFile
import numpy as np
import soundfile as sf
from scipy.signal import resample_poly
from compare_claviset import pair,partials
from compare_elepian import decay

MUSEUM=[(50,968.78),(52,943.04),(53,940.94),(54,953.18),(55,971.24),(56,948.86),(57,929.30),(59,932.30),
        (70,989.72),(71,967.70),(74,979.40),(76,983.54),(77,983.30),(79,982.28)]
TAKES={24:'c0',25:'csh0',26:'d0',27:'dsh0',28:'e0',29:'f0',30:'fsh0',31:'g0',32:'gsh0',
       36:'c1',40:'e1',41:'f1',45:'a1',48:'c2',50:'d2',53:'f2',55:'g2',57:'a2',59:'b2',60:'c3',
       62:'d3',64:'e3',65:'f3',67:'g3',69:'a3',71:'b3',72:'c4',74:'d4',76:'e4',77:'f4',79:'g4',81:'a4',
       84:'c5'}
MUSEUM_SHA='656f5e4fe15f4c680d37d4274052fdcb989affdc0ce8cc07db9b721a9838b193'
PACK_SHA='be2ada6b6c4f191d6258e6d9a7feb7ded7b31c6ebbfe3eda6dd33305b5ed5712'


def impulse(f,rate,load,v,e):
    s=2j*rate*np.tan(np.pi*f/rate);parallel=lambda *a:1/sum(1/i for i in a)
    bottom=parallel(max(.001,100e3*e),load,1/(s*470e-12));expr=max(.001,100e3*(1-e))+bottom
    wiper=parallel(max(.001,100e3*v),expr);ctrl=max(.001,100e3*(1-v))+wiper
    rail=parallel(ctrl,1/(s*33e-9));z=(9200+.72*s)/25
    return rail/(rail+z)*wiper/ctrl*bottom/expr/25


def electrical(root):
    worst=np.zeros(2)
    for load in [100000,1000000]:
        for vol in [.5,1]:
            for exp in [.5,1]:
                y,s=sf.read(root/f'terminal-{load}-{vol}-{exp}.wav')
                for f in np.geomspace(20,20000,17):
                    z=y@np.exp(-2j*np.pi*np.arange(len(y))*f/s)/impulse(f,s,load,vol,exp)
                    worst=np.maximum(worst,[abs(20*np.log10(abs(z))),abs(np.angle(z,deg=True))])
    assert max(worst)<.02;print('8 network sweeps, max dB / degrees:',worst)


def scaled(y,s):
    import math
    g=math.gcd(s,22050)
    return resample_poly(y,22050//g,s//g)


def rise(y):
    # First 180 ms, 6 ms RMS aperture, relative to the loudest aperture.
    n=round(.006*22050);rms=np.sqrt(np.convolve(y[:round(.18*22050)]**2,np.ones(n)/n,mode='valid'))
    t=[np.argmax(rms>=a*rms.max())/22050 for a in [.1,.9]]
    return (t[1]-t[0])*1000


def compare(root,zipfile):
    source_file=root/'museum.wav';assert hashlib.sha256(source_file.read_bytes()).hexdigest()==MUSEUM_SHA
    assert hashlib.sha256(zipfile.read_bytes()).hexdigest()==PACK_SHA
    ref,sr=sf.read(source_file);assert sr==22050
    with ZipFile(zipfile) as pack:
        files={Path(f).stem.lower():f for f in pack.namelist() if f.lower().endswith('.wav')}
        groups=[('museum',[(n,t,ref) for n,t in MUSEUM]),('restored',[])]
        for n,stem in TAKES.items():
            if stem not in files:continue
            y,s=sf.read(BytesIO(pack.read(files[stem])),always_2d=True)
            groups[1][1].append((n,.21,scaled(y.mean(axis=1),s)))
        for label,cases in groups:
            errors=[];rows=[];clips=[];envelopes=[];pitch=[]
            museum_down=resample_poly(cases[0][2],1,2) if label=='museum' else None
            for n,t,ref in cases:
                model,s=sf.read(root/f'note-{n}-80.wav');m=scaled(model[:,0],s)
                f,fit,h=partials(museum_down if museum_down is not None else resample_poly(ref,1,2),t,n)
                fp,mfit,mh=partials(resample_poly(m,1,2),.26,n)
                if fit<.80:continue
                count=min(len(h),len(mh));h,mh=h[:count],mh[:count]
                err=abs(np.maximum(-45.,h)-np.maximum(-45.,mh));errors.extend(err);pitch.append(n)
                rows.extend((n,t,f,fp,fit,mfit,k+2,a,b) for k,(a,b) in enumerate(zip(h,mh)))
                if label=='museum' or n in [24,36,48,60,72,84]:clips.append(pair(ref,t,m))
                if label=='restored':envelopes.append([n,rise(ref),rise(m[round(.05*22050):]),decay(ref,22050,n,.26),decay(m,22050,n,.31)])
            with (root/f'{label}-harmonics.csv').open('w',newline='') as o:
                w=csv.writer(o);w.writerow(['note','reference_s','ref_Hz','model_Hz','ref_pitch_fit','model_pitch_fit','order','ref_dBc','model_dBc']);w.writerows(rows)
            h2=[abs(a-b) for n,t,f,fp,fit,mfit,k,a,b in rows if k==2];print(label,len(pitch),'pitches, H2-H6 / H2 median error dB:',round(np.median(errors),2),round(np.median(h2),2));sf.write(root/f'{label}-then-model.wav',np.concatenate(clips),22050,subtype='PCM_24')
            audible=[abs(a-b) for _,_,_,_,_,_,_,a,b in rows if a>=-45];print('Reference partials above -45 dBc:',len(audible),'of',len(rows),', median error',round(np.median(audible),2),'dB')
            if envelopes:
                with (root/'restored-envelopes.csv').open('w',newline='') as o:
                    w=csv.writer(o);w.writerow(['midi','reference_rise_ms','model_rise_ms','reference_decay_dB_per_s','model_decay_dB_per_s']);w.writerows(envelopes)
                a=np.array(envelopes);print('restored, reference / model median rise ms',np.round(np.median(a[:,1:3],axis=0),2),'median absolute decay slope error',round(np.median(abs(a[:,3]-a[:,4])),2),'dB/s')

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('root',type=Path);p.add_argument('zipfile',type=Path);args=p.parse_args()
    electrical(args.root);compare(args.root,args.zipfile)
