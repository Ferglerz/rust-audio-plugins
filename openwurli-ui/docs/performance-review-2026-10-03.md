# OpenWurli efficiency review — 2026-10-03

Historical baseline review. The investigation below did not change production DSP or install a plugin. Item 1 was subsequently implemented and verified; see [Heavy recovery shortcut results](heavy-recovery-shortcut-2026-10-03.md). Items 2–7 remain pending. The original probes build standalone release binaries and use temporary diagnostic source copies. No vendored tests or checks were run, and REAPER was not inspected or controlled.

The strongest new finding is in Heavy: the generated power amplifier performs expensive recovery work whose result the wrapper unconditionally discards. In the instrumented six-note case this consumed 27.5% of render time; in the 64-note case, 48.4%. These are measured cost shares, **not demonstrated optimization gains**. They make a much stronger first target than another general lookup-table pass.

For Fast, the common tremolo circuit and legacy preamp dominate ordinary chords. Exact damper tables offer a separate opportunity in both modes during releases. Existing reed SIMD is already useful; more voice work becomes more relevant at dense polyphony.

## Baseline and scope

The actual source under review is `openwurli-ui/vendor/openwurli-dsp`, based on Ferglerz/openwurli revision `b51f8dad3c5fb62070a20a11e20bf2886b8ff860`, plus its local full-MIDI APIs. The wrapper enables `runtime-models` and the DSP's default features. The workspace already contained uncommitted changes, including this vendor copy. Exact per-file SHA-256 hashes are in [provenance](performance-review-2026-10-03/provenance.json).

This is the fork's v0.7-based instrument. Changing to a newer upstream sound model would be a separate project. No upstream contribution is proposed. Future DSP work belongs in **Ferglerz/openwurli**, then the wrapper/vendor integration in our repositories.

Already implemented, and therefore excluded from new savings claims:

- Paired f64 reed-mode SIMD, cached jitter rotations and iterator mixing.
- Tremolo depth caching and skipping the inaudible LDR mapping at zero depth.
- Fast preamp kernel/current reuse.
- Heavy preamp shared inversions and exact Schur-product improvements.
- Lockstep BJT evaluation and reuse of already evaluated device results.

The older reports retain useful evidence but describe older source revisions, machines and workloads. Their percentages do not establish current M1 performance. The previous broad BJT/CdS table experiment failed whole-engine comparisons; the narrow amp tanh table passed a numerical screen but did not establish a CPU win.

## Fresh measurements

Apple M1 MacBook Air, 8 GB RAM, macOS 26.6.2, rustc 1.98.1. Release builds, 48 kHz, 256-sample blocks, 0.5013 seconds per trial, seven repetitions. Median render cost excludes construction, warmup and note-on. Velocity is 0.95, MLP off, hiss off, speaker character zero, volume 0.5; Heavy's native rail dynamics remain enabled. Each engine receives 0.6 seconds of warmup. These are CLI wall times, not DAW CPU-meter readings.

With tremolo depth 0.5:

- **Idle:** Fast 0.972 µs/sample; Heavy 9.306 µs/sample.
- **One note:** Fast 1.091 µs/sample; Heavy 15.494 µs/sample.
- **Six notes:** Fast 1.213 µs/sample; Heavy 22.565 µs/sample.
- **32 notes:** Fast 1.569 µs/sample; Heavy 20.272 µs/sample.
- **64 notes:** Fast 1.976 µs/sample; Heavy 51.582 µs/sample.

At 48 kHz the budget is 20.833 µs/sample. Six-note Fast uses about 5.8% of that budget; six-note Heavy uses 108.3%. Heavy's non-monotonic costs reflect different pitches/signals and nonlinear solver trajectories, so these rows are not a controlled per-voice scaling curve.

For the six-note case, p99 block time was 0.393 ms Fast and 30.060 ms Heavy; the block deadline is 5.333 ms. These statistics cover only 658 short-run blocks per case. Background scheduling and thermal behavior were not controlled. One brief harness build may overlap a late case in the original timing matrix. Use the medians to prioritize work, and repeat controlled paired measurements before publishing a speedup claim.

Separate component replays at depth 0.5 measured approximately:

- Tremolo: 0.565 µs per host sample, common to both modes.
- Fast preamp: 0.378 µs; Fast amp: 0.052 µs.
- Heavy preamp: 5.371 µs; Heavy amp: 11.435 µs.

These replays use their own startup histories, one-second input, dynamic dispatch for the preamp, and a slightly different velocity representation. They are **not an additive profile** of the engine rows. They establish where to investigate, not exact percentages of the full plugin.

The separate generated-preamp diagnostic reported zero preamp fallbacks in its cases. It isolated about 3.3 µs/host sample of matrix-update work at nonzero tremolo depth, compared with about 0.04 µs at depth zero. Main and shadow circuit solves together were about 2.0–2.3 µs/sample. Different tremolo depth also changes the signal, so subtracting complete-engine depth-zero/nonzero timings would not isolate this cost.

Raw data: [engine](performance-review-2026-10-03/engine.csv), [stages](performance-review-2026-10-03/stages.csv), [preamp](performance-review-2026-10-03/preamp-diag.csv), [lifetime counters](performance-review-2026-10-03/amp-lifetime.csv), [recovery timings](performance-review-2026-10-03/amp-recovery-time.csv).

## 1. Heavy: skip recovery whose output is guaranteed to be rejected

**Subsequent status:** implemented, with a retained full-recovery feature and exact-output proof. [Results and reproduction](heavy-recovery-shortcut-2026-10-03.md). The measurements and proposal below describe the original investigation.

**Priority: first. Strong source evidence and measured cost; exact-output candidate.**

`gen_power_amp.rs:9778` enters adaptive substeps after the primary Newton solve fails, then can enter a backward-Euler fallback. The recovery paths can report convergence, but they do not change `last_nr_iterations`, which remains the primary-failure sentinel. `power_amp.rs:531` rejects any sample with `last_nr_iterations >= MAX_ITER - 1`, restores the entire cached circuit state and rails, and returns `last_good`.

Consequently, those substep/fallback results cannot reach the instrument output or survive into its next sample. The full generic generated solver still needs them for callers that consume its recovery results.

A diagnostic-only copy added counters outside the resettable generated state. During 48,128 oversampled samples, after warmup:

- Six notes, depth 0.5: **14 adapter resets**, 13 backward-Euler fallback entries. Recovery alone took 143.9 ms of a 522.6 ms instrumented render: **27.5%**.
- 64 notes, depth 0.5: **55 adapter resets**, 50 backward-Euler fallback entries and five successful substep recoveries. Recovery took 641.8 ms of 1327.1 ms: **48.4%**.
- Six notes, depth zero: 12 adapter resets, including two successful substep recoveries subsequently rejected.

One reset can also reject primary convergence on its final permitted iteration because the adapter uses `>= MAX_ITER - 1`. Do not silently change that threshold in an exact optimization.

**Proposed implementation:** a separate wrapper-oriented generated entry point/policy that returns immediately when the unchanged primary solve has reached a result the adapter is certain to reject. Keep the complete generated solver callable. Preserve the existing guard, cached restoration, rail reset and held output. This removes discarded computation without lowering iteration limits or accepting less-converged audio.

**Required proof:** full output bits and post-reset state against the unmodified wrapper, including rate changes, stressed chords, mode transitions and recovery cases. Counters should explicitly distinguish primary failure, attempted recovery and wrapper rejection. The time fractions are a useful opportunity estimate; no shortcut was implemented or benchmarked here. Primary Newton work and ordinary processing remain.

**Related observability finding:** `diag_snapshot()` reads counters inside the state restored by `reset()`. A final zero count did hide failures in this investigation. Permanent lifetime counters belong outside that restored state. The diagnostic copy's rolling output hashes match the untouched engine in all ten Heavy cases; this is a checksum sanity check, not an exhaustive audio proof.

## 2. Both: cache immutable device math, especially in the tremolo solver

**Priority: next exact circuit work. Savings unmeasured.**

`gen_preamp.rs:4654` and `gen_power_amp.rs:8904` reconstruct `BjtArgs` every circuit sample. `bjt_lockstep.rs:150` computes parameter-only products and quotients. Existing work moved them out of the inner device iteration; moving them to device/configuration changes is the next step.

The shared `gen_tremolo.rs:2989` path still uses separate scalar BJT evaluations, including the parameter arithmetic. Tremolo was the largest measured stage for ordinary Fast chords. Its oscillator continues at zero depth to preserve subsequent automation, so disabling depth does not eliminate this opportunity.

**Proposed implementation:** cache the existing f64 expressions without replacing divisions by reciprocals; extend the proven device-evaluation approach to the tremolo model only where operations and behavior match. Check release assembly first because the compiler may already eliminate some constant work. Compare evaluating diode current and derivative together where it actually duplicates calculation; do not assume two inlined source calls mean two machine computations.

**Required proof:** device-current and Jacobian bits; full tremolo/engine trajectory across startup, reset, depth and response automation; invalidation on all device changes. Generated device fields are public, so a cache cannot assume only today's wrapper changes them. Keep a validated parameter key or explicit compatible update API. Never cache the voltage-dependent Jacobian across samples as though it were immutable.

## 3. Both: bake exact damper multipliers

**Priority: small, bounded exact candidate for release bursts.**

`reed.rs:264` calculates seven `exp()` values per active voice per sample during the 8–50 ms damper-contact ramp. Their inputs depend on MIDI note, mode, sample rate and release-sample index. They do not depend on audio, note velocity, jitter, reed-decay control or the current envelope.

Bake the original `inst_rate = damper_rate[i] * t / ramp; exp(-inst_rate)` values and continue multiplying the live envelopes in their original order. No interpolation, recurrence substitution or pre-rendered reed audio is necessary.

The isolated probe compared **10,358,516 envelope updates** across damped MIDI notes 0–91 at 44.1/48/88.2/96/192 kHz: **zero differing bits**. The isolated seven-mode ramp kernel measured about 18.2 ns/sample from native evaluation and 3.65 ns/sample using baked values, roughly 5× faster. This does not prove full-engine equivalence or a 5× instrument speedup.

At 48 kHz, a straightforward full bank costs 4,058,880 bytes for normal-range damped notes or 8,494,080 bytes including extended bass notes. At 192 kHz the extended bank is 33,976,320 bytes. Share banks per sample rate, generate outside the callback, and consider exact deduplication of capped rates. Preserve native fallback for unsupported/unprepared rates and exact repeated-note-off indexing. Notes 92 and above have no damper.

This revisits the earlier rejected recurrence with a different mechanism: storing exact native values avoids its cumulative rounding differences. CPU benefit is confined to active release ramps; held-note benchmarks cannot validate its benefit.

## 4. Heavy: share coefficient storage, not just the inversion

**Priority: exact structural candidate; medium implementation scope.**

`gen_preamp.rs:4182` now shares one inversion, but compares both matrix sets and copies eleven arrays into the shadow state on each accepted LDR update. Those arrays contain **10,904 bytes**. At 96 kHz internal rate, copying them on every sample represents about **1.05 GB/s of logical copy traffic**, before source reads and comparisons; this is not a measured DRAM-bandwidth figure.

Move identical circuit coefficients into one preamp-owned block and pass it to two independent main/shadow dynamic states. Keep nonmatching/failure behavior. This can preserve arithmetic and reduce copying; it does not remove the remaining inversion. Keep a separate compatibility route for standalone generated states.

Measure copy/comparison cost before committing to a broad state-layout refactor. Do not remove the shadow circuit merely because input is zero or assume it is interchangeable with the audio-driven state.

## 5. Heavy: replace full LDR reinversion with a rank-one update

**Priority: largest remaining preamp math opportunity; requires numerical validation.**

`gen_preamp.rs:4222` changes only `g_work[6][6]`, then requests a complete 13×13 inversion and derived products. At a fixed sample rate this is a rank-one change. The Fast preamp already uses a Sherman–Morrison construction (`dk_preamp_legacy.rs:653`), though its different circuit cannot simply be substituted for Heavy.

Use the Heavy model's exact matrix and source constants to derive the inverse and Schur products from a fixed reference matrix and current conductance. Prefer a fixed anchor over repeatedly updating the last inverse, to avoid accumulated update drift. Check denominators, conditioning, residuals and nonfinite inputs; retain full inversion as fallback/reference.

This preserves the circuit equations in real arithmetic but **changes floating-point evaluation**. It is not a bit-exact cache. Preserve the existing accumulated conductance stamp semantics, or explicitly classify a changed stamping calculation as another numerical change. Do not widen the LDR deadband or update it less often as part of this patch.

The measured matrix-update region is about 3.3 µs per host sample with tremolo enabled. That bounds the available benefit; a replacement has its own cost. Previous CdS/BJT table failures show why small function errors are insufficient proof for this feedback system. Gate on long renders, solver behavior, aliases, harmonics, transients and controls, as well as error residuals.

## 6. Both at high polyphony: improve the existing reed kernel

**Priority: after shared-circuit work for typical playing; medium scope.**

`reed.rs:256` already processes three f64x2 pairs plus one scalar mode. Each sample still loads/stores multiple arrays, branches for onset/damping, and schedules 16-sample jitter and 1024-sample renormalization events.

Inspect generated M1 assembly for actual spills before changing layout. Candidates include keeping mode vectors live over subspans bounded by those events and specializing held, onset and damper phases while retaining their exact update times. Cross-voice batching may help dense chords later, but ragged releases, steal fades, ordered RNG streams and ordered voice sums make it a larger change. Never fuse floating operations or reorder reductions under a bit-exact claim.

Do not delete inaudible modes solely because their amplitude is currently zero: state updates, RNG advancement, renormalization and nonfinite handling need an explicit equivalence argument. Keep the current threshold, decay and damper behavior; raising the voice-kill threshold is a sound-model change.

## 7. Both, especially Fast: bake the autonomous tremolo trajectory only as an optional study

**Priority: promising approximation after exact opportunities.**

The Twin-T oscillator in `tremolo.rs:348` receives zero input/no injections and feeds LED current into the CdS model. This makes its trajectory a better baking candidate than entire reeds: the reed must retain velocity-dependent onset, jitter, modal evolution and progressive damping.

Generate an oscillator trajectory from the retained full solver, keeping live LED/CdS/depth/response processing. A finite stored prefix with the exact terminal state can be exact over its stored horizon. A looped periodic table, interpolation, reduced solver rate or resampling changes the trajectory and must be labeled approximate. At 96 kHz, one minute of a single f64 stream alone is about 46 MB; endless exact playback is not a free small wavetable.

The entire measured tremolo stage is about 0.565 µs/host sample; even eliminating all of it would cap the six-note Fast gain around 47% against this baseline, and actual baking retains part of that work. Small LDR perturbations previously destabilized some native preamp trajectories, so approximation acceptance is a substantial gate. Preserve the complete live oscillator and native mapping permanently.

## Lower-priority work and rejected shortcuts

- **Note-on templates:** cache per-note constants and optional exact repeated-velocity results to reduce dense MIDI spikes. Hammer shaping and MLP run at note-on, so they do not explain sustained CPU. Hosts provide f32 velocity; rounding all velocities onto 127 levels would change the instrument. Measure event-inclusive p99 callbacks before prioritizing this.
- **Initialization snapshots:** share immutable settled states keyed by rate and configuration, especially tremolo startup. This targets load/reset cost, not steady-state rendering. Restore every dynamic field and preserve deterministic initialization.
- **Compiler settings:** benchmark a plugin-specific LTO/codegen profile, then native CPU targeting if distribution permits. Current workspace release uses Cargo defaults. Rust documents that default local ThinLTO differs from cross-crate LTO; neither guarantees faster DSP. Do not change the entire multi-plugin workspace profile based on one result. See [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html).
- **Whole Fast amp transfer table:** more plausible than tabulating only its inner tanh because that amp is memoryless. However, the measured stage is only about 0.052 µs/sample, so the whole-engine upside is small. Earlier tanh-only results did not justify shipping.
- **Idle suspension:** both circuits continue while no notes play. Sleeping can save substantial idle CPU, but freezing oscillator/rail/filter states changes the next note or automation response. It needs an explicit state policy and a preserved continuous reference. Returning zero whenever voice count is zero is not exact.
- **Avoid as default optimizations:** f32 circuit solvers, lower oversampling, fewer Newton iterations, looser tolerances, frozen rails, removed shadow cancellation, quantized controls, sampled reed replacements, unsafe indexing without measured bounds-check cost, GPU offload, and dropping nonfinite guards.

## Preserve the physical source and define acceptance

Keep three identifiable configurations: the current Fast/Heavy compatibility baseline; the complete generated circuit/reference path with all recovery logic; and any baked/optimized derivative. Record source/model version, sample rate, constants, generator/patch provenance, feature flags and table format in every derived asset. Never overwrite the only reference equations or require an external historical checkout merely to evaluate them.

Heavy selects a fuller circuit model, but its generated `fast_exp` defaults to a polynomial approximation; `--cfg melange_precise_exp` selects native exp. Preserve and document an explicit native-math reference build as well. Native exp increases numerical evaluation accuracy, but does not by itself prove greater physical accuracy or more stable solver behavior. This investigation did not build or validate that configuration.

For exact candidates, require local-platform bitwise full-engine comparison, preserved final states and explicit recovery behavior. For numerical candidates, keep the established residual gates as an initial screen (historically −120 dBFS peak / −100 dB relative RMS), plus spectral/alias/attack/decay checks and listening. Numerical residual gates alone do not prove perceptual or physical equivalence.

Expand coverage beyond the historical proof: Extended Notes 0–127 and its toggle/release behavior, Reed Decay through 20×, both modes, MLP on/off, control extremes and automation, sustained/released/stolen voices, long tails, sample-rate changes and the 100 ms preroll/20 ms mode crossfade. Include callback p99/max and note-event costs; average throughput alone misses the observed Heavy spikes. Do not claim a full Cartesian matrix unless it actually ran.

Recommended execution order: **1 → 2 → 3 → 4**, then re-profile before **5–7**. Treat persistent diagnostics and a frozen comparison baseline as part of item 1. Each item should be reviewed and verified separately. A solver-stability redesign could improve physical behavior, but it is distinct from removing work that today's wrapper already discards.

## Reproduce and limits

From the workspace root:

```sh
python3 openwurli-ui/docs/performance-review-2026-10-03/reproduce.py \
  --out /private/tmp/openwurli-review-repeat
```

The output directory must be new. This builds standalone release binaries offline using the current vendored DSP, runs them sequentially, copies the DSP into scratch for instrumentation, and applies [the diagnostic-only patch](performance-review-2026-10-03/diagnostic-only.patch). It does not edit production sources, run vendored tests or install a plugin. Cargo dependencies must already be cached. Compare source hashes with provenance before treating a rerun as the same baseline.

The main timing matrix is untouched DSP. The recovery timing is one instrumented pass; its denominator includes output capture and checksum work. The preamp diagnostic independently reconstructs its startup trajectory and is not an engine-state trace. The damper probe validates the isolated native formula/envelope multiplication with unit starting envelopes, not integrated voice playback or audio. None of this constitutes validation of a proposed optimization, a complete instrument conformance run, or DAW listening results.
