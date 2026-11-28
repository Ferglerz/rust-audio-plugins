"""Stress the physical driver and output grids with two recorded-from-model chord waveforms."""
from pathlib import Path
import tempfile
import subprocess
import numpy as np

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def compare(directory, rate, root, gain, master):
    samples = np.fromfile(directory/f'chord-{rate}-{root}-{gain:.2f}-{master}.bin',
                          dtype='<f8').reshape(-1,2)
    x, digital = samples.T
    osrate = rate*4
    taps = np.sinc(.24*np.arange(-48,49))*np.blackman(97)
    taps /= taps.sum()
    up = np.zeros(4*len(x))
    up[::4] = 4*x
    excitation = np.convolve(up,taps)[:len(up)]
    stimulus = 'Vsig e1 0 PWL(\n' + ''.join(f'+ {j/osrate:.10f} {v:.10g}\n'
                                               for j,v in enumerate(excitation)) + '+ )'
    raw = (HERE/'elv413.cir').read_text()
    raw = raw.replace('Vsig e1 0 dc 0 ac 1 sin(0 {AMP} 1k)',stimulus)
    raw = raw.replace('MASTER=.46 CSW=700p', 'MASTER=.46 CSW=350p' if master==9 else 'MASTER=1 CSW=700p')
    raw += '\n.control\nset wr_singlescale\nset numdgt=13\ntran 2u .18 0 2u\nwrdata chord.dat v(out) v(ga) v(p3)\nquit\n.endc\n.end\n'
    (directory/'chord.cir').write_text(raw)
    subprocess.run(['ngspice','-b','chord.cir'],cwd=directory,stdout=subprocess.DEVNULL,check=True)
    t,y,grid,driver = np.loadtxt(directory/'chord.dat').T
    high = np.interp(np.arange(len(up))/osrate,t,y)
    simulator = np.convolve(high,taps)[:len(up):4]
    pk = np.array([max(abs(digital)),max(abs(simulator))])
    error = np.linalg.norm(digital-simulator)/np.linalg.norm(simulator)
    print(f'{rate} root {root} input {max(abs(x)):.3f} Vpk, master {master}, '
          f'speaker Rust/SPICE {pk[0]:.3f}/{pk[1]:.3f} Vpk, RMS difference {100*error:.1f}%, '
          f'grid {max(abs(grid)):.1f} Vpk, driver plate range {120+min(driver):.1f}…{120+max(driver):.1f} V')
    assert abs(20*np.log10(pk[0]/pk[1])) < 1. and error < .40 and max(pk) < 10.


def main():
    with tempfile.TemporaryDirectory() as folder:
        d=Path(folder)
        subprocess.run(['cargo','run','--release','-q','-p','neo-bechstein','--example','elv413','--',folder,'stress'],cwd=ROOT,check=True)
        for gain,master in [(1.,9),(1.,12),(6.58,12)]:
            compare(d,44100,76,gain,master)


if __name__=='__main__':
    main()
