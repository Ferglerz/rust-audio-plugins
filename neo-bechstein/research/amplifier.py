"""Check ELV413 host-rate impulses against ngspice and the restored Vienna amplifier."""
from pathlib import Path
import subprocess
import tempfile

import numpy as np
from scipy.interpolate import CubicSpline

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SETTINGS = [(9, 1., .46), (9, 0., .46), (3, .5, .035), (12, 1., 1.)]


def run_spice(d, tap, swell, analysis, amplitude=.141421356237):
    raw = (HERE/'elv413.cir').read_text().replace(
        '.param MASTER=.46 CSW=700p AMP=.141421356237',
        f'.param MASTER={tap} CSW={(350+350*swell)*1e-12} AMP={amplitude}')
    (d/'test.cir').write_text(raw+'\n.control\nset wr_singlescale\nset numdgt=13\n'
                              +analysis+'\nquit\n.endc\n.end\n')
    subprocess.run(['ngspice','-b','test.cir'],cwd=d,stdout=subprocess.DEVNULL,check=True)
    return np.loadtxt(d/'out.dat').T


def sinc_gain(f, host):
    t=np.arange(-48,49)
    fir=np.sinc(2*.48*t/4)*np.blackman(len(t))
    return np.cos(2*np.pi*np.outer(f/(4*host),t)) @ (fir/fir.sum())


def harmonics(t,y):
    basis = [np.ones(len(t))]
    for n in range(1, 10):
        angle = 2*np.pi*1000*n*t
        basis.extend([np.cos(angle),np.sin(angle)])
    p = np.linalg.norm(np.linalg.lstsq(np.array(basis).T,y,rcond=None)[0][1:].reshape(-1,2),axis=1)
    return p[0]**2/12, 100*np.linalg.norm(p[1:])/p[0]


def main():
    with tempfile.TemporaryDirectory() as folder:
        d=Path(folder)
        subprocess.run(['cargo','run','--release','-q','-p','neo-bechstein','--example','elv413','--',folder],cwd=ROOT,check=True)
        hfreq,measured=np.loadtxt(HERE/'fig10.csv',delimiter=',',skiprows=1).T
        for i,(step,swell,tap) in enumerate(SETTINGS):
            f,re,im=run_spice(d,tap,swell,'ac dec 320 7 25000\nwrdata out.dat v(out)')
            z=re+1j*im
            if i==0:
                sampled=CubicSpline(f,z)(hfreq)
                relative=20*np.log10(abs(sampled/CubicSpline(f,z)(1000)))
                print('freq, measured dB, analog dB, delta dB')
                for a,b,c in zip(hfreq,measured,relative):print(f'{a:g} {b:.2f} {c:.2f} {c-b:+.2f}')
                rms = np.mean((relative-measured)**2)**.5
                print(f'Fig 10 RMS {rms:.2f} dB; largest miss {max(abs(relative-measured)):.2f} dB')
                assert rms<2.5
            for rate in [44100,48000,96000]:
                y=np.fromfile(d/f'ir-{rate}-{i}.bin',dtype='<f8')
                x=np.fft.rfftfreq(len(y),1/rate)
                # Remove the explicit double-FIR delay before interpolating the transform.
                hf=np.fft.rfft(y)*np.exp(2j*np.pi*x*24/rate)
                if i == 0:
                    measured_points = CubicSpline(x,hf)(hfreq)
                    delta = 20*np.log10(abs(measured_points / CubicSpline(x,hf)(1000))) - measured
                    print(f'{rate} Hz: direct host-vs-Fig10 RMS {np.mean(delta**2)**.5:.2f} dB, max {max(abs(delta)):.2f} dB')
                keep=(f>=15)&(f<=19000)
                mapped=(4*rate/np.pi)*np.arctan(np.pi*f[keep]/(4*rate))
                digital=CubicSpline(x,hf)(mapped)
                ratio=digital/(z[keep]*sinc_gain(mapped,rate)**2)
                err=max(abs(20*np.log10(abs(ratio))))
                phase=max(abs(np.angle(ratio,deg=True)))
                print(f'{rate} Hz, step {step} swell {swell:.1f} SPICE max {err:.4f} dB {phase:.4f} deg')
                assert err<.30 and phase<.7
        for level in [.005,.025,.05,.1,.2,.35,.5,1.,2.]:
            t,v=run_spice(d,.46,1.,'tran 3u .28 .23 3u\nwrdata out.dat v(out)',level*2**.5)
            y=np.fromfile(d/f'sine-{level:.3f}.bin',dtype='<f8')
            tm=np.arange(len(y))/48000.
            pw,thd=harmonics(tm[tm>=.24],y[tm>=.24])
            ref,refthd=harmonics(t,v)
            print(f'{level:.3f} Vrms -> Rust/SPICE {pw:.4f}/{ref:.4f} W, {thd:.3f}/{refthd:.3f}% THD, {max(abs(y[tm>=.24])):.3f}/{max(abs(v)):.3f} Vpeak')
            assert abs(10*np.log10(pw/ref))<.4 and abs(thd-refthd)<1.


if __name__=='__main__':
    main()
