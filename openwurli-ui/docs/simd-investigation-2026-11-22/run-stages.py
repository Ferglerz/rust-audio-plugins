#!/usr/bin/env python3
"""Inspect the scalar/old/revised loops and time individual envelope stages."""
from pathlib import Path
import hashlib, json, platform, subprocess, sys
HERE = Path(__file__).resolve().parent
wrapper = HERE.parent.parent
output = Path(sys.argv[1]).resolve()
(output / "src").mkdir(parents=True, exist_ok=False)
source = (HERE / "stage-probe.rs").read_text().replace("__WRAPPER__", str(wrapper))
(output / "src/main.rs").write_text(source)
(output / "Cargo.toml").write_text('[package]\nname = "reed-stage-probe"\nversion = "0.1.0"\nedition = "2024"\n[workspace]\n[dependencies]\nwide = "=0.7.33"\n[profile.release]\nopt-level = 3\nlto = "thin"\n')
subprocess.run(["cargo", "rustc", "--release", "--offline", "--", "--emit=asm"], cwd=output, check=True)
with (output / "stages.csv").open("w") as stream:
    subprocess.run([str(output / "target/release/reed-stage-probe")], stdout=stream, check=True)
for path in (output / "target/release/deps").glob("*.s"):
    text = path.read_text()
    for name in ["scalar", "simd", "coherent"]:
        marker = "_" + name + "_render:"
        if marker in text:
            body = text[text.index(marker):].split(".cfi_endproc")[0]
            (output / (name + "-render.s")).write_text(body)
inputs = [HERE / "stage-probe.rs", Path(__file__), wrapper / "tests/support/scalar_reed_v091.rs", wrapper / "tests/support/simd_reed_v091.rs", wrapper / "tests/support/revised_simd_reed_v091.rs"]
metadata = {"platform": platform.platform(), "rustc": subprocess.check_output(["rustc", "-Vv"], text=True), "inputs": {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs}}
(output / "stage-provenance.json").write_text(json.dumps(metadata, indent=2) + "\n")
