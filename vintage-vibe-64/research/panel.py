"""Compare the proposed passive network with an independent ngspice AC solve."""
from pathlib import Path
import subprocess, tempfile
import numpy as np

exe=Path('target/release/examples/vv64_probe').resolve()
rows=subprocess.check_output([exe,'panel'],text=True)
settings={'open':(1,1,1),'lowcut':(1,0,1),'highcut':(1,1,0),'both':(.65,.3,.4)}
with tempfile.TemporaryDirectory() as tmp:
    path=Path(tmp)
    differences=[]
    for name,(vol,bass,treble) in settings.items():
        deck=f"""Vintage Vibe 64 passive hypothesis
V1 emf 0 AC 1
RW emf x 2.5k
Lharp x harp .90
Charp harp 0 150p
Rbass harp bus {1+500000*(1-bass)**2}
Cbass harp bus 2.2n
Rtreble bus treble {1+500000*treble**2}
Ctreble treble 0 22n
Rtop bus jack {1+250000*(1-vol**2.5)}
Rbottom jack 0 {1+250000*vol**2.5}
Rload jack 0 1Meg
Ccable jack 0 470p
.control
ac dec 1024 20 20000
wrdata ac.txt db(v(jack))
quit
.endc
.end
"""
        cir=path/'panel.cir'
        cir.write_text(deck)
        subprocess.run(['ngspice','-b',cir],cwd=path,check=True,stdout=subprocess.DEVNULL)
        hz,db=np.loadtxt(path/'ac.txt').T
        for row in rows.splitlines():
            setting,f,v=row.split(',')
            if setting==name:
                target=np.interp(np.log(float(f)),np.log(hz),db)
                differences.append(float(v)-target)
    print(f'{len(differences)} points, maximum error {max(map(abs,differences)):.4f} dB')
