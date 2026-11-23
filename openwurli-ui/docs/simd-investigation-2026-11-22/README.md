# Reed SIMD follow-up

The original paired implementation used more render time during releases.
This investigation separates held notes, the damper-contact ramp, steady damping
and the complete release, then inspects the generated arm64 assembly.

## Mechanism

The scalar render loop is scalar in the inspected machine code; the compiler
had not already vectorized it. Both implementations make seven scalar `exp`
calls during the contact ramp. That ramp is faster in the original SIMD probe,
so those calls do not explain its release regression.

The regression appears during steady damping. The old implementation pairs
modes 0/1, 2/3 and 4/5, leaving mode 6 scalar. Mode 0 also receives a scalar,
amplitude-dependent air-drag update. Its envelope is therefore accessed as both
part of a vector and a scalar element in the same sample. The old assembly loads
and stores that pair at reed offset 336 in the damper step, then loads/stores it
again for natural decay and stores its low scalar lane for air drag. The scalar
implementation keeps these envelope values in scalar registers through the loop.

A local rearrangement that applied air drag before storing the pair did not fix
the problem. The useful change is consistent state grouping:

- Keep mode 0 scalar, including its damping, decay and air drag.
- Pair upper modes 1/2, 3/4 and 5/6 in oscillation, both damper stages and radius
  renormalization. Avoid switching the grouping between stages.
- Reduce contributions in original mode order, retain f64 precision, and keep
  the original scalar exponential evaluations. No approximate math or fused
  multiply-add replacement is introduced.

The revised assembly keeps the fundamental envelope in a scalar register and
writes it back at function exit, eliminating its per-sample scalar/vector memory
round trips. It still uses real two-lane arithmetic for the upper modes.

## Reproduction

Run from this directory with Rust, Python 3.9+ and cached Cargo dependencies:

```sh
python3 run-stages.py /private/tmp/openwurli-reed-stage-reproduction
python3 run.py /private/tmp/openwurli-reed-engine-reproduction
```

Choose fresh output directories. `run-stages.py` builds a standalone three-way
probe from the retained scalar, old SIMD and revised SIMD source files in
`../../tests/support/`. It records 21 alternating trials for each stage, input
hashes, compiler/platform information and the three exported render functions'
assembly. The stage probe uses one 440 Hz reed, fixed modal parameters and a
25 ms damper ramp. It isolates a mechanism rather than representing a whole
instrument workload.

`run.py` builds the frozen scalar and revised DSP source archives as dependencies
of a standalone binary. It does not run vendor-crate checks or tests. The scalar
snapshot already includes all accepted BJT evaluation reuse. The `reuse` and
`simd` variant labels in its CSV mean scalar and revised SIMD, respectively.
The archives retain wrapper-specific test hooks and are built as dependencies,
not independent test packages.

The full probe retains the preceding study's 48 kHz, 256-sample blocks, 24,064
measured frames, five alternating paired trials and actual learned-voicing reed
construction. It measures 1/6/32/64 held and releasing voices separately in both
circuits. Construction, settling and note-on are outside the timed interval.

## Exact output

The retained wrapper comparison now checks both SIMD implementations against the
frozen scalar implementation. The revised candidate matches all 36,637,440 f64
samples and damping/release bookkeeping across 64 normal keys, six extended
pitches, three velocities, four rates, two air-drag settings, initial phases,
repeated damping and irregular/empty buffers.

The full-engine comparison also matches scalar output bit for bit across both
circuits at 44.1/48/96 kHz, with 1/6/32/64 held/releasing voices: 1,155,072 f32
samples, zero mismatches, nonfinite samples or voice-output NaN guards.

These observations establish equivalence for the exercised cases on macOS arm64;
performance measurements are not cross-platform or DAW CPU-meter claims.


## Measured results and selection

**Adopt the revised implementation.** The isolated 32/64-voice bank uses 18.62%
and 19.33% less time for held notes, and 10.20% and 10.84% less for releasing
notes. All four cases clear the 5% gate.

The complete Fast engine uses 2.84% and 3.91% less time for held 32/64-voice
cases, and 0.71% and 2.08% less for their release cases. Heavy changes range from
0.53% to 1.55% less time in this run; the circuit dominates its cost. Across all
16 engine cases, the largest slowdown is 0.04%, below the 2% regression gate.
Small changes below about 1% should not be interpreted as demonstrated general
speedups. These are median render times, not a change in real-time DAW CPU
percentage points.

The stage probe provides a separate causal check. The old implementation uses
34.40% more time after the damper ramp; the revised version uses 8.86% less than
scalar there. During the ramp both SIMD implementations are faster than scalar.
Together with the assembly and unchanged `exp` call count, this isolates the
bad interaction between the scalar fundamental and vector envelope grouping.

`results/` contains all 240 paired timing records, 252 stage timing records,
complete exported-function assembly, source/compiler hashes and the exact-output
results. No builds or other verification commands from this investigation ran
during the full reed-bank or engine timing sections; the stage-probe compilation
ran during the untimed exact-output comparison phase. The final stage probe ran
after all engine timings completed.

The production Rust sources are byte-identical to the frozen `revised` source
snapshot and are hashed in `results/production-source-hashes.json`. The scalar
reference and initial rejected implementation remain available for regression
checks. Public controls, host parameter IDs and the circuit model are unchanged.

## Final verification

From the repository root:

```sh
cargo check -p openwurli-ui --all-targets --offline
cargo test -p openwurli-ui --release --offline --quiet
cargo clippy -p openwurli-ui --all-targets --offline
```

The tests compare the production reed and the initial experiment against the
scalar reference. The existing BJT-helper and independent upstream-audio tests
remain in the wrapper targets. The upstream fixture files are not regenerated.

All 34 release tests, compile checks and formatting checks pass. Crate-scoped
Clippy passes with the three existing Pleasant UI warnings and the existing
fixture-reader warning under `--all-targets`. The required installer rebuilt the
accepted source, verified the bundled executables, installed CLAP and VST3, and
verified both installed bundle signatures. The installer and Clippy logs and
installed executable hashes are retained in `results/`.
