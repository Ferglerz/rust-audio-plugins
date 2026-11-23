#!/usr/bin/env python3
"""Build frozen, standalone source snapshots and compare/benchmark them offline."""
import csv
import hashlib
import json
import platform
import statistics
import subprocess
import sys
import tarfile
from pathlib import Path

HERE = Path(__file__).resolve().parent

def run(output):
    output.mkdir(parents=True, exist_ok=False)
    deps = []
    for variant in ("baseline", "reuse", "simd"):
        directory = output / variant
        directory.mkdir()
        with tarfile.open(HERE / f"{variant}.tar.gz") as archive:
            for member in archive.getmembers():
                destination = (directory / member.name).resolve()
                if not member.isfile() or not destination.is_relative_to(directory):
                    raise ValueError(f"Unexpected archive entry: {member.name}")
            archive.extractall(directory)
        manifest = directory / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('name = "openwurli-dsp"', f'name = "openwurli-{variant}"', 1))
        deps.append(f'{variant} = {{ package = "openwurli-{variant}", path = {json.dumps(str(directory))}, features = ["runtime-models"] }}')
    harness = output / "harness"
    (harness / "src").mkdir(parents=True)
    (harness / "src/main.rs").write_bytes((HERE / "probe.rs").read_bytes())
    (harness / "Cargo.toml").write_text('[package]\nname = "openwurli-optimization-probe"\nversion = "0.1.0"\nedition = "2024"\n[workspace]\n[dependencies]\n' + '\n'.join(deps) + '\n[profile.release]\nopt-level = 3\nlto = "thin"\n')
    metadata = {
        "platform": platform.platform(),
        "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
        "archives": {v: hashlib.sha256((HERE / f"{v}.tar.gz").read_bytes()).hexdigest() for v in ("baseline", "reuse", "simd")},
        "probe_sha256": hashlib.sha256((HERE / "probe.rs").read_bytes()).hexdigest(),
        "run_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    }
    (output / "provenance.json").write_text(json.dumps(metadata, indent=2) + '\n')
    subprocess.run(["cargo", "build", "--release", "--offline"], cwd=harness, check=True)
    binary = harness / "target/release/openwurli-optimization-probe"
    for command, filename in [("verify", "equivalence.json"), ("bench", "timings.csv")]:
        with (output / filename).open("w") as stream:
            subprocess.run([str(binary), command], stdout=stream, check=True)
    summarize(output)

def summarize(output):
    groups = {}
    with (output / "timings.csv").open() as stream:
        for row in csv.DictReader(stream):
            key = (row['kind'], row['mode'], int(row['voices']), row['release'])
            groups.setdefault(key, {}).setdefault(row['variant'], []).append(int(row['ns']))
    summary = []
    for key, variants in groups.items():
        medians = {v: statistics.median(values) for v, values in variants.items()}
        summary.append(dict(zip(['kind', 'mode', 'voices', 'release'], key), median_ns=medians,
            simd_reduction_pct=100 * (1 - medians['simd'] / medians['reuse']),
            reuse_reduction_pct=100 * (1 - medians['reuse'] / medians['baseline']) if 'baseline' in medians else None))
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary, indent=2))

if __name__ == '__main__':
    run(Path(sys.argv[1]).resolve())
