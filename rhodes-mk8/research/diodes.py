"""Compare the assumed anti-parallel drive against ngspice DC operating points."""
from pathlib import Path
import subprocess,tempfile, numpy as np
exe=Path('target/release/examples/mk8_probe').resolve()
rows=subprocess.check_output([exe,'diodes'],text=True).splitlines()
with tempfile.TemporaryDirectory() as folder:
    out=Path(folder)
    error=[]
    for row in rows:
        drive,model=map(float,row.split(','))
        circuit=f'''MK8 S diode-drive hypothesis
V1 drive 0 DC {drive}
R1 drive sense 4700
Dp sense 0 diode
Dn 0 sense diode
.model diode D(Is=2n N=1.70118)
.options temp=27 tnom=27
.control
op
wrdata dc.txt v(sense)
quit
.endc
.end
'''
        (out/'drive.cir').write_text(circuit)
        subprocess.run(['ngspice','-b','drive.cir'],cwd=out,check=True,stdout=subprocess.DEVNULL)
        ref=np.loadtxt(out/'dc.txt')[-1]
        error.append(1000*(model-ref))
    print(f'{len(error)} points, maximum error {max(map(abs,error)):.3f} mV')
