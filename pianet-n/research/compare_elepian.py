"""Compare the original EP-61C passive circuit and two physical instruments."""
import argparse
import csv
import hashlib
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import resample_poly
from compare_claviset import partials, pair

CASES = {
    "harp-tech": [(46,3.50), (48,382.48), (51,384.58), (53,384.88), (60,397.34),
                  (65,28.34), (67,26.36), (69,11.36), (70,9.74), (72,396.80)],
    "organ69": [(39,137.94), (41,161.04), (42,156.06), (44,159.96), (46,142.44),
                (49,136.86), (51,140.04), (53,120.06), (55,109.02), (58,145.50), (70,59.28)],
}
DIGESTS = {
    "harp-tech":"8931bf0c9de43cf167e38aa4885e0ce90650e0308e5235fe38760de6a172a1c1",
    "organ69":"ebb878133b3596e215b9908559bc2011780d2f8632fdf1fa6d567d59fa4bcfaa",
}


def panel_reference(hz, rate, load, tone, volume):
    # Map each digital frequency to its equivalent continuous frequency. Solve
    # two loaded wipers by complex impedance division independently of the MNA.
    s = 2j * rate * np.tan(np.pi * hz / rate)
    copper = 1 / (1/15000 - 2/100000) - 10000
    def parallel(a,b):
        return a*b/(a+b)
    bottom_tone = parallel(max(.001,100000*(1-tone)),1/(s*47e-9))
    tone_branch = max(.001,100000*tone) + bottom_tone
    wiper = parallel(parallel(max(.001,100000*volume),load),1/(s*470e-12))
    volume_branch = max(.001,100000*(1-volume)) + wiper
    both = parallel(volume_branch,tone_branch)
    return (both/(copper + s*.122 + 10000 + both)) * (wiper/volume_branch)


def electrical(root):
    worst = np.zeros(2)
    for load in [100000,1000000]:
        for tone in [0,50,100]:
            for vol in [50,100]:
                y, sr = sf.read(root/f"panel-{load}-{tone}-{vol}.wav")
                for f in np.geomspace(20,20000,17):
                    observed = y@np.exp(-2j*np.pi*f*np.arange(len(y))/sr)
                    expected = panel_reference(f,sr,load,tone/100,vol/100)
                    ratio = observed / expected
                    worst = np.maximum(worst,[abs(20*np.log10(abs(ratio))),abs(np.angle(ratio,deg=True))])
    print("12 passive sweeps, maximum magnitude and phase errors",worst)
    if max(worst) > .02:
        raise ValueError("Passive panel disagrees with the divider")
    for f in [110.,440.,3000.,7000.]:
        value = lambda tc: 20*np.log10(abs(panel_reference(f,88200,1e6,tc,1.)))
        print("Passive jack",f,"Hz, dark/center/bright dB",*[round(value(t),2) for t in [0,.5,1.]])


# Each start is a 0.72 s stretch with a monotonically decaying fundamental.
# Shop pitches 44, 60, 72 and 84 set the loss anchors. The technician's
# separate instrument was reserved for comparison.
DECAY = {
    "organ69": [(39,56.42),(44,40.5),(46,28.54),(48,61.14),(51,39.42),(53,28.74),
                (55,83.30),(58,33.90),(60,106.58),(64,106.10),(65,69.62),(67,60.82),
                (69,102.66),(70,150.66),(72,129.18),(76,64.90),(84,66.82)],
    "harp-tech": [(46,4.34),(48,2.34),(49,9.98),(51,15.54),(53,340.14),(55,338.46),
                  (58,342.82),(60,4.42),(64,338.46),(65,19.86),(67,7.02),(69,11.26),
                  (70,2.02),(72,2.50),(76,340.18)],
}


def decay(y, sr, n, start):
    f = 440 * 2**((n-69)/12)
    w = np.hanning(round(sr*.16))
    demod = w*np.exp(-2j*np.pi*np.arange(len(w))*f/sr)
    t = np.arange(15)*.04
    level = [20*np.log10(abs(demod @ y[round((start+i)*sr):round((start+i)*sr)+len(w)])) for i in t]
    return -np.polyfit(t,level,1)[0]


def recordings(root):
    rows=[]
    for label,cases in CASES.items():
        file=root/f"{label}.wav"
        ref, sr = sf.read(file)
        if sr!=22050 or ref.ndim!=1 or hashlib.sha256(file.read_bytes()).hexdigest() != DIGESTS[label]:
            raise ValueError(f"Expected documented 22,050 Hz mono capture for {label}")
        analysis = resample_poly(ref,1,2)
        electrode = 1 if label=="harp-tech" else 0
        errors=[]; listens=[]
        for note,t in cases:
            y,s = sf.read(root/f"note-{note}-80.wav")
            y=resample_poly(y[:,electrode],1,2)
            f, fit, h=partials(analysis,t,note)
            m, _, mh=partials(resample_poly(y,1,2),.26,note)
            if fit<.91:
                raise ValueError(f"Weak pitch evidence at {label} {t}: {fit}")
            errors.append(abs(np.maximum(-40.,h)-np.maximum(-40.,mh)))
            listens.append(pair(ref,t,y))
            for i,(a,b) in enumerate(zip(h,mh)):
                rows.append([label,note,t,round(f,3),round(m,3),round(fit,4),i+2,round(a,2),round(b,2)])
        print(label,len(cases),"median H2-H6 dB",round(np.median(errors),2),"by order",np.round(np.median(errors,axis=0),2))
        sf.write(root/f"{label}-then-model.wav",np.concatenate(listens),22050,subtype="PCM_24")
        losses=[]
        for n, start in DECAY[label]:
            m, rate = sf.read(root/f"note-{n}-80.wav")
            observed=decay(ref,sr,n,start)
            predicted=decay(m[:,electrode],rate,n,.40)
            group="anchor" if label=="organ69" and n in [44,60,72,84] else "reserved"
            losses.append((n,start,observed,predicted,group))
        with (root/f"{label}-decay.csv").open("w",newline="") as o:
            w=csv.writer(o);w.writerow(["note","time","hardware_dBper_s","model_dBper_s","use"]);w.writerows(losses)
        for group in ["anchor","reserved"]:
            e=[abs(a-b) for _,_,a,b,g in losses if g==group]
            if e: print(label,len(e),group,"median absolute decay-rate difference",round(np.median(e),2),"dB/s")
    y,_=sf.read(root/"harp-tech.wav");y=resample_poly(y,1,2)
    for t in [100.50,120.90]:
        f,fit,h=partials(y,t,65)
        print("F4 regulation on hardware",t,"s H2 and H3 dBc",np.round(h[:2],2))
    for tag in ["minus","stock","plus"]:
        y,_=sf.read(root/f"voicing-{tag}.wav")
        f,fit,h=partials(resample_poly(y[:,1],1,4),.21,65)
        print("F4 model offset",tag,"H2 and H3 dBc",np.round(h[:2],2))
    with (root/"reference.csv").open("w",newline="") as out:
        w=csv.writer(out);w.writerow(["source","note","time_s","recorded_Hz","model_Hz","fit","order","recorded_dBc","model_dBc"]);w.writerows(rows)


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root",type=Path)
    args=parser.parse_args()
    electrical(args.root)
    recordings(args.root)
