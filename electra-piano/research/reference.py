"""Compare discrete jack and power-stage responses to the original-side ngspice pickup netlist."""
from pathlib import Path
import subprocess
import tempfile

import numpy as np
from scipy.interpolate import CubicSpline

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]


def spice(directory, volume, pedal, drive):
    net = (HERE / '680209.cir').read_text()
    for index, count in enumerate([10, 10, 10, 10, 10, 10, 12]):
        lo = f'g{index}' if index else '0'
        hi = f'g{index + 1}'
        net += f"""
V{index} a{index} {lo} dc 0 ac {int(index == drive)}
Rw{index} a{index} winding{index} {8 * count}
Lw{index} winding{index} {hi} {0.0005 * count}
Rt{index} {lo} {hi} 100
"""
    tap = (100 ** volume - 1) / 99
    # An ideal voltage transformer and its reflected load with magnetization
    # and referred copper in the primary. Rust works entirely on the secondary.
    net += f"""
Rp8 g7 basic {5000 if pedal else 0.001}
Rp9a basic pri 750
Rp9b pri 0 4250
Rcu pri core {35 + 250/49}
Lprimary core 0 0.8
Esecondary secondary 0 core 0 7
Fref core 0 Esecondary -7
Rvol1 secondary jack {max((1-tap)*100000, 0.001)}
Rvol2 jack 0 {max(tap*100000, 0.001)}
Rexternal jack 0 1MEG
.control
set wr_singlescale
set numdgt=14
ac dec 120 35 14000
wrdata response.dat v(jack) v(spk)
quit
.endc
.end
"""
    (directory / 'sweep.cir').write_text(net)
    subprocess.run(['ngspice', '-b', 'sweep.cir'], cwd=directory,
                   stdout=subprocess.DEVNULL, check=True)
    f, a, b, c, d = np.loadtxt(directory / 'response.dat').T
    return f, np.stack([a+1j*b, c+1j*d])


def main():
    worst = np.zeros((2, 2))
    with tempfile.TemporaryDirectory() as folder:
        directory = Path(folder)
        subprocess.run(['cargo', 'run', '--quiet', '--release', '-p', 'electra-piano', '--',
                        'impulses', str(directory)], cwd=REPO, check=True)
        for volume in [0.25, 0.5, 1.]:
            for pedal in [False, True]:
                for drive in [0, 3, 6]:
                    file = directory / f'ir-{volume:g}-{str(pedal).lower()}-{drive}.bin'
                    impulse = np.fromfile(file, dtype='<f8').reshape(-1, 2)
                    f, wanted = spice(directory, volume, pedal, drive)
                    bins = np.fft.rfftfreq(len(impulse), 1/96000)
                    h = CubicSpline(bins, np.fft.rfft(impulse, axis=0))(
                        96000/np.pi * np.arctan(np.pi*f/96000))
                    ratio = h.T / wanted
                    delta = np.stack([abs(20*np.log10(abs(ratio))).max(axis=1),
                                      abs(np.angle(ratio, deg=True)).max(axis=1)], axis=1)
                    worst = np.maximum(worst, delta)
    print('Worst error across 18 settings, 35 Hz to 14 kHz')
    for label, row in zip(['auxiliary', 'speaker'], worst):
        print(f'{label}: {row[0]:.6f} dB / {row[1]:.6f} degrees')
    assert np.all(worst < [0.01, 0.02]), worst


if __name__ == '__main__':
    main()
