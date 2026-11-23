# OpenWurli v0.9.1 optimization verification

Historical first-pass results. The later [SIMD follow-up](../simd-investigation-2026-11-22/README.md)
accepted a revised implementation. The source and installed-binary hashes here
refer to the earlier scalar installation.

This package compares the uncommitted v0.9.1 integration present before this
work (`baseline`), that source with BJT evaluation reuse in all three generated
solvers (`reuse`), and the same source with paired f64 reed-mode SIMD (`simd`).
The baseline already included tremolo BJT reuse and the local depth-divider cache.
It is the wrapper's adapted engine, not an unmodified upstream build.

The upstream depth-divider change is separate:
[hal0zer0/openwurli#11](https://github.com/hal0zer0/openwurli/pull/11).

## Reproduce

The three archives contain the complete small DSP source package, including its
GPL license. `run.py` extracts them into standalone source packages and builds a
separate binary; it does not check or test a vendored crate. Wrapper-only test
hooks in the archives refer to support files in this repository; the reproduction
builds the snapshots as dependencies, not as independent Cargo test targets. Rust and Python 3.9+
are required, with the pinned Cargo dependencies already cached.

From this directory, choose a fresh output directory:

```sh
python3 run.py /private/tmp/openwurli-optimization-reproduction
```

`provenance.json` records platform, compiler and SHA-256 hashes for the frozen
archives, probe and runner. `equivalence.json`, `timings.csv` and `summary.json`
contain the comparisons and all five timings for each case. These are CLI wall
times on one macOS arm64 machine, not DAW CPU-meter readings or cross-platform
performance claims. No REAPER inspection or control was used.

## Coverage

The wrapper's `bjt_reuse` integration target compiles each self-contained generated
module and compares the private helper against the original unconditional-final-
evaluation implementation. Each helper covers 724 input/parameter combinations,
including both transistor polarities and model selections, nonfinite inputs,
zero/nonzero parasitics and deliberately singular Jacobians. Exit counters require
actual tolerance convergence, singular-Jacobian exits and iteration exhaustion.
The same tests also pass with `melange_precise_exp` enabled.

The reed comparison covers 36,637,440 f64 samples: 64 normal keys and six extended
pitches, three velocities, four sample rates, zero/nonzero air drag, zero/nonzero
initial phases, repeated damping with two scales, additive output, irregular/empty
buffers, silence detection and damping/release bookkeeping. Rendered samples and
bookkeeping match the frozen scalar source bit for bit.

The standalone full-engine comparison covers 48 held/releasing scenarios across
1/6/32/64 voices, both circuits and 44.1/48/96 kHz. Each candidate is compared with
the original baseline: 2,310,144 f32 sample comparisons, zero differing bits,
zero nonfinite samples and zero voice-output NaN guards. This is not an exhaustive
circuit stability assessment. The independent upstream audio fixtures remain
unchanged and also pass.

Benchmarks use 48 kHz, 256-sample blocks and 24,064 timed samples per case. They
exclude construction, circuit settling, note-on and the initial 4,096 held samples.
Release cases begin damping immediately before timing. Five trials alternate
variant order. Engine runs compare all three variants; isolated reeds compare
`reuse` and `simd`, so SIMD's contribution is measured separately. The same MIDI
pitches and velocity 0.8 are used throughout, with learned voicing enabled,
speaker character 0.65, tremolo depth 0.5 and noise disabled.

A short precise-math test compilation overlapped the first one-voice timing
cases. No other builds or test runs overlapped the later 32/64-voice acceptance
cases. Those early timings should not be used for a small performance claim.

The SIMD acceptance gate requires exact output, at least a 5% median isolated-reed
improvement at 32 and 64 voices for held and releasing notes, and no repeatable
whole-engine regression above 2%. The completed results and final selection follow.


## Results and selection

**Keep BJT reuse; retain the scalar production reed.** SIMD passed the exact-output
checks but failed the isolated-reed performance gate. Against the scalar reed
with the same BJT changes, median isolated render time was:

- 32 held voices: 0.17% less time; 64 held voices: 0.45% more time.
- 32 releasing voices: 30.56% more time; 64 releasing voices: 28.63% more time.

The paired full-engine timings also show regressions in some cases, so there is
no basis for shipping this candidate. The frozen SIMD source remains in the
archive and in `../../tests/support/simd_reed_v091.rs`; `wide` is a wrapper test
dependency only. The production reed is byte-identical to the pre-change scalar
source. All production Rust files match the verified `reuse` snapshot.

BJT reuse reduced median render time in the 32- and 64-voice held Heavy cases by
8.56% and 8.58%, respectively. This is limited evidence from this machine and
workload, not a general plugin speedup. There is no consistent Fast-mode gain;
its active circuit did not receive the new generated-preamp/power-amp changes.

All 320 timing records, per-case medians, exact-comparison totals and provenance
are in `results/`. Negative `simd_reduction_pct` denotes more render time. The
accepted production source hashes are in `results/production-source-hashes.json`.

## Local verification

From the repository root:

```sh
cargo check -p openwurli-ui --all-targets --offline
cargo test -p openwurli-ui --release --offline --quiet
cargo clippy -p openwurli-ui --all-targets --offline
```

The wrapper targets run the retained comparison tests without directly testing a
vendored package. Compile checks and all 33 release tests pass. Standard Clippy
passes with three existing Pleasant UI warnings and one existing fixture-reader
warning under `--all-targets`; `-D warnings` fails on the Pleasant UI warnings. Formatting checks cover the changed Rust files. The six
independent v0.9.1 fixture files retain their original SHA-256 hashes.

The BJT tests also pass with native precise exponential evaluation:

```sh
rustc --test --edition 2024 --cfg melange_precise_exp -O \
  openwurli-ui/tests/bjt_reuse.rs -o /private/tmp/openwurli-bjt-precise
/private/tmp/openwurli-bjt-precise --quiet
```

The required `openwurli-ui/scripts/install.sh` completed with
`CARGO_NET_OFFLINE=true`: it rebuilt from the accepted scalar/BJT-reuse source,
verified the bundled executables, installed both CLAP and VST3 bundles and
verified their signatures. The installer and final Clippy output are retained
in `results/install.txt` and `results/final-clippy.txt`.
