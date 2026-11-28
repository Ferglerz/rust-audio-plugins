"""Compare Rust AC response and large-signal harmonic growth to the 1940 circuit deck."""
from pathlib import Path
import tempfile,subprocess
import numpy as np
from scipy.interpolate import CubicSpline

HERE=Path(__file__).resolve().parent
REPO=HERE.parents[1]
SETTINGS=[(0,0,.7),(0,1,.7),(.5,0,.7),(.5,.5,.7),(.5,1,.7),(1,0,.7),(1,1,1)]
LEVELS=[5e-5, .0002, .0005, .001, .002, .004,.00628,.008]
RATE=96000


def run(directory,manual,swell,optimum,analysis,amplitude=0.0001):
    deck=(HERE/'rca.cir').read_text().replace('.param OPT=.7 MAN=.7 SW=0 AMP=.0001',
           f'.param OPT={optimum} MAN={manual} SW={swell} AMP={amplitude}')
    (directory/'run.cir').write_text(deck+'\n.control\nset wr_singlescale\nset numdgt=13\n'+analysis+'\nquit\n.endc\n.end\n')
    subprocess.run(['ngspice','-b','run.cir'],cwd=directory,check=True,stdout=subprocess.DEVNULL)
    return np.loadtxt(directory/'out.dat')


def harmonics(t,y):
    # Each output is in volts peak; remove DC and allow either phase.
    basis=[np.ones(len(t))]
    for h in range(1,8):
        angle=2*np.pi*400*h*t
        basis += [np.cos(angle),np.sin(angle)]
    fit=np.linalg.lstsq(np.stack(basis,axis=1),y,rcond=None)[0][1:].reshape(-1,2)
    partials=np.linalg.norm(fit,axis=1)
    return partials[0],100*np.linalg.norm(partials[1:])/partials[0]


def main():
    worst=np.zeros(2)
    with tempfile.TemporaryDirectory() as tmp:
        d=Path(tmp)
        subprocess.run(['cargo','run','--release','-q','-p','storytone','--example','storytone_probe','--','impulses',str(d)],cwd=REPO,check=True)
        for i,(manual,swell,optimum) in enumerate(SETTINGS):
            f,re,im=run(d,manual,swell,optimum,'ac dec 120 27.5 14000\nwrdata out.dat v(spk)').T
            y=np.fromfile(d/f'transfer-{i}.bin',dtype='<f8')
            bins=np.fft.rfftfreq(len(y),1/RATE)
            z=CubicSpline(bins,np.fft.rfft(y))(RATE/np.pi*np.arctan(np.pi*f/RATE))
            ratio=z/(re+1j*im)
            errs=[abs(20*np.log10(abs(ratio))).max(),abs(np.angle(ratio,deg=True)).max()]
            print(f'{manual:.1f} {swell:.1f} {optimum:.1f} | {errs[0]:.5f} dB, {errs[1]:.5f} deg')
            worst=np.maximum(worst,errs)
        print('linear max',worst)
        assert np.all(worst<[.025,.08]),worst
        print('input mV pk | V1 Rust/SPICE | THD% Rust/SPICE')
        for i,level in enumerate(LEVELS):
            t,spice=run(d,1,1,1,'tran 5u .25 .20 5u\nwrdata out.dat v(spk)',level).T
            y=np.fromfile(d/f'sine-{i}.bin',dtype='<f8')
            tm=np.arange(len(y))/RATE
            raw=harmonics(tm[tm>=.21],y[tm>=.21])
            reference=harmonics(t[t>=.21],spice[t>=.21])
            print(f'{1000*level:5.2f} | {raw[0]:.4f}/{reference[0]:.4f} | {raw[1]:.4f}/{reference[1]:.4f}')
            assert abs(20*np.log10(raw[0]/reference[0]))<.04 and abs(raw[1]-reference[1])<.1
            if level == .00628:
                print(f'near rated output: {raw[0]**2/30:.2f} W at {raw[1]:.2f}% harmonic distortion')
if __name__=='__main__': main()
