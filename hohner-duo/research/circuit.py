"""Duo two-input/three-output AC regression against the contact-level transistor deck."""
import itertools
import subprocess
import tempfile
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
RATE = 96000
SIZE = 131072
# Test exact bins in the Rust response; ngspice sees their bilinear inverse.
BINS = np.array([42, 137, 500, 990, 2050, 4560, 5130, 7070, 12960, 19500])
FREQ = RATE / np.pi * np.tan(np.pi * BINS / SIZE)


def reference(folder, case, port):
    split, pickup, tones, two, three, foot, cv, pv = case
    clav, reed = [(60,60), (20,40), (40,20), (40,60)][split]
    deck = (HERE / "duo.cir").read_text()
    deck = deck.replace("DUAL=0 B_THROW=0 CBANK=60 PBANK=60",
                        f"DUAL={int(pickup >= 2)} B_THROW={pickup%2} CBANK={clav} PBANK={reed}")
    deck = deck.replace("MC=0 MP=0 CV=1 PV=1", f"MC={int(foot and not three)} MP={int(not foot and not two)} CV={cv} PV={pv}")
    deck = deck.replace("EC=1 EP=0", f"EC={1-port} EP={port}")
    # Apply ideal wires to the four tone shorts and two footswitch shorts. Finite
    # contact resistances obscure the tiny signal through a regulated supply.
    alias = {}
    bottom = "0"
    for i, top in reversed(list(enumerate(["tones","lo","mid","hi"]))):
        if not tones & (1 << i):
            alias[top] = bottom
        else:
            bottom = top
    for short, top, jack, v in [(foot and not three,"cpot","cjack",cv),(not foot and not two,"ppot","pjack",pv)]:
        if short:
            alias[top] = "0"
        if v == 1:
            alias[jack] = alias.get(top,top)
        elif v == 0:
            alias[jack] = "0"
    lines = []
    for line in deck.splitlines():
        if line.startswith(("Rtab","Rcut")):
            continue
        if line.startswith(("R","L","C","V")):
            parts = line.split()
            for i in (1,2):
                parts[i] = alias.get(parts[i],parts[i])
            if parts[1] == parts[2] and not line.startswith("L"):
                continue
            line = " ".join(parts)
        lines.append(line)
    signals = [alias.get(n,n) for n in ("cjack","pjack","mix")]
    read = " ".join("v("+n+")" if n != "0" else "(v(rail)*0)" for n in signals)
    orders = []
    for i,hz in enumerate(FREQ):
        orders.extend([f"ac lin 1 {hz:.14g} {hz:.14g}", f"wrdata p{i}.dat {read}"])
    control = "\n".join([".control","set wr_singlescale","set numdgt=14",*orders,"quit",".endc",".end"])
    (folder / "verify.cir").write_text("\n".join(lines).replace(".end",control) + "\n")
    subprocess.run(["ngspice","-b","verify.cir"], cwd=folder, check=True,stdout=subprocess.DEVNULL)
    a = np.stack([np.loadtxt(folder/f"p{i}.dat") for i in range(len(FREQ))])
    return a[:,1::2]+1j*a[:,2::2]


def compare():
    subprocess.run(["cargo","build","--quiet","--release","-p","hohner-duo","--example","duo_impulse"], cwd=ROOT,check=True)
    exe = ROOT/"target/release/examples/duo_impulse"
    cases = [(0,p,t,1,1,0,0.83,0.73) for p,t in itertools.product(range(4),range(16))]
    cases += [(s,0,15,a,b,f,1.,1.) for s,a,b,f in itertools.product(range(4),range(2),range(2),range(2))]
    cases += [(3,2,15,1,1,0,0.,1.),(3,2,15,1,1,0,1.,0.),(3,2,15,1,1,0,0.,0.)]
    max_db, max_phase, worst_null = 0., 0., 0.
    with tempfile.TemporaryDirectory() as temp:
        for case in cases:
            raw = subprocess.check_output([exe,*map(str,case)])
            data = np.frombuffer(raw,dtype="<f8").reshape(2,SIZE,3)
            for port in (0,1):
                model = np.fft.rfft(data[port],axis=0)[BINS]
                spice = reference(Path(temp),case,port)
                # Below 1 µV/V evaluate absolute error, including exact grounded jacks.
                tiny = abs(spice)<1e-6
                if np.any(tiny):
                    worst_null = max(worst_null,abs((model-spice)[tiny]).max())
                ratio = (model[~tiny]/spice[~tiny])
                if ratio.size:
                    db = np.max(abs(20*np.log10(abs(ratio))))
                    phase = np.max(abs(np.angle(ratio,deg=True)))
                    if db > .15 or phase > 1.5:
                        raise AssertionError(f"{case}, port {port}: {db:.4g} dB, {phase:.4g} deg")
                    max_db,max_phase = max(db,max_db),max(phase,max_phase)
    print(f"{len(cases)} control combinations · two independent drives · three jacks · {len(FREQ)} frequencies")
    print(f"Worst gain {max_db:.5f} dB, phase {max_phase:.5f}°, low-level absolute error {worst_null:.3g} V/V")
    assert worst_null < 1e-7


if __name__ == "__main__":
    compare()
