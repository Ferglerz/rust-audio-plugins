#!/usr/bin/env python3
"""Re-run the review probes outside the product tree. No installation or tests."""
import argparse
import json
import os
import shutil
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--out', type=Path, required=True, help='A new scratch output directory')
args = parser.parse_args()
data = Path(__file__).resolve().parent
source = data.parents[1] / 'vendor' / 'openwurli-dsp'
out = args.out.resolve()
out.mkdir(parents=True, exist_ok=False)
(out / 'src/bin').mkdir(parents=True)
for original, target in [('benchmark.rs', 'src/main.rs'), ('preamp-diagnostic.rs', 'src/bin/preamp_diag.rs'), ('damper-probe.rs', 'src/bin/damper.rs')]:
    shutil.copy2(data / original, out / target)
manifest = '''[package]
name="openwurli-review"
version="0.1.0"
edition="2024"
[workspace]
[dependencies]
openwurli-dsp={path=SOURCE,features=["runtime-models"]}
[profile.release]
debug=1
'''
(out / 'Cargo.toml').write_text(manifest.replace('SOURCE', json.dumps(str(source))))
shutil.copy2(data / 'Cargo.lock', out / 'Cargo.lock')
env = dict(os.environ, CARGO_TARGET_DIR=str(out / 'target'))

def run(command, output, cwd=out):
    print(' '.join(map(str, command)), flush=True)
    with (out / output).open('w') as log:
        subprocess.run(list(map(str, command)), cwd=cwd, env=env, stdout=log, check=True)

subprocess.run(['cargo', 'build', '--offline', '--release', '--manifest-path', str(out / 'Cargo.toml'), '--bins'], env=env, check=True)
run([out / 'target/release/openwurli-review', '7'], 'engine.csv')
run([out / 'target/release/openwurli-review', 'stages'], 'stages.csv')
run([out / 'target/release/preamp_diag'], 'preamp-diag.csv')
run([out / 'target/release/damper'], 'damper.txt')
shutil.copytree(source, out / 'instrumented-dsp')
subprocess.run(['patch', '-p1', '-i', str(data / 'diagnostic-only.patch')], cwd=out / 'instrumented-dsp', check=True)
(out / 'lens/src').mkdir(parents=True)
shutil.copy2(data / 'lifetime-diagnostic.rs', out / 'lens/src/main.rs')
(out / 'lens/Cargo.toml').write_text(manifest.replace('name="openwurli-review"', 'name="openwurli-review-lens"').replace('SOURCE', json.dumps(str(out / 'instrumented-dsp'))))
shutil.copy2(data / 'Cargo.lock', out / 'lens/Cargo.lock')
subprocess.run(['cargo', 'build', '--offline', '--release', '--manifest-path', str(out / 'lens/Cargo.toml')], env=env, check=True)
run([out / 'target/release/openwurli-review-lens'], 'amp-recovery-time.csv')
print(f'Results saved to {out}. Compare source hashes to provenance.json before treating them as the same baseline.')
