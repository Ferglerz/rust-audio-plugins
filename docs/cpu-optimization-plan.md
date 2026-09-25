# CPU optimization plan (fidelity-first)

Review of the SIMD / LUT / wavetable / bounds-check / reciprocal proposal against
pinned OpenWurli DSP `6614ab9519471e956ecb0c82edc4fcf295f724dc`
(`Ferglerz/openwurli`, `codex/pleasant-controls`). Ranked by whether the change
can match the current sound with no audible compromise.

This wrapper now vendors `vendor/openwurli-dsp` so the exact-path work can
ship here (the fork is not writable from this agent). Land the same reed SIMD
+ mix-bus commit on `Ferglerz/openwurli` when possible, then switch back to a
git `rev`.

## What the proposal gets right

- Voices do not interact. Each of the 64 notes is an isolated 7-mode reed +
  pickup. They sum onto one electrostatic bus. There is no mechanical crosstalk
  or sympathetic resonance to skip.
- SIMD needs contiguous lanes. The engine is array-of-structs (`VoiceSlot` →
  `Voice` → `ModalReed.modes: [Mode; 7]`).
- Rust has no global `-ffast-math`. Constant divisions that LLVM will not fold
  must be written as multiplies if we care.
- Memoizing expensive **per-sample** transcendentals is the right instinct.
  Several of the cited functions are already off the audio thread.

## What the proposal gets wrong (against this tree)

| Claim | Actual code |
|---|---|
| Hammer Gaussian `exp()` is a CPU hog | `dwell_attenuation()` runs **once at note-on** (`hammer.rs`). Not in `render()`. |
| Pickup `1/(1-y)` needs a LUT | One add + one divide per sample (`pickup.rs`). A table + lerp is slower and less accurate. |
| Reed still computes `sin()` per mode per sample | `reed.rs` already uses quadrature rotation. Comment: "eliminates 7 transcendental calls per sample per voice — the dominant CPU cost in v0.1.x". |
| Power amp is a `tanh` soft-clip rail | Shipping default (`legacy-power-amp`) is a **Newton–Raphson closed-loop** that calls `exp` + `tanh` **up to 8 times per oversampled sample**. Not a single `tanh(x)`. |
| MNA is the thing to put in `f32x4` first | Shared chain is one preamp + one amp. SIMD-across-voices does not speed the solver. The solver is already `f64` because NR / Gummel–Poon `exp(Vbe/Vt)` is precision-sensitive. |
| Index loops force a bounds check every sample | Reed, pickup, and voice gain already use iterators. Remaining `for i in 0..len` is the mix bus in `engine.rs`. LLVM usually elides checks when `len` is the slice length. Measure before rewriting. |
| Pre-bake reed wavetables, keep the circuit | Reed is **not** a static oscillator. See "Do not ship as default" below. |

Default features this wrapper already uses: legacy 8-node preamp (no
`melange-preamp`) and behavioral power amp (`legacy-power-amp`). Hiss and Sag in
the UI are cheap post-engine effects, not the circuit solvers.

## Cost model (before writing SIMD)

Per base-rate sample at 44.1 kHz the engine does:

1. **Each active voice:** 7 quadrature rotates + decay; optional onset `cos` /
   `powf`; OU jitter every 16 samples; damper-ramp `exp` per mode while the felt
   is closing (8–50 ms); pickup RC + `y/(1-y)`; `tanh` only if `|y| ≥ 0.94`.
2. **Once, shared:** 2× upsample → preamp `dk_step` (two BJT `exp`s) →
   behavioral amp NR (`exp`+`tanh` × ≤8) → downsample → speaker biquads.

Voices scale with polyphony. The circuit does not. Profile with
`preamp-bench` / Instruments / `samply` on a 6-note ff chord and a 32-note hold
before choosing a SIMD width. Do not assume ARM NEON `f64x2` of two whole
voices is the highest yield.

---

## Ranked work

Order is **fidelity first**, then yield. "No audible compromise" means: same
algorithm, same `f64` rounding or a proven-inaudible substitute gated by the
existing peak / THD / bark / alias tests.

### 1. Measure the hot path — no sound change

**Where:** fork `tools/preamp-bench`, plus a release-mode chord render.

**Do:** break CPU into voice sum vs preamp vs amp vs speaker vs oversampler.
Record active-voice count. That ranking decides whether mode-SIMD or amp LUT
is next.

**Fidelity:** exact. This is a gate, not an optimization.

### 2. Intra-voice mode SIMD (`f64x2`) — highest exact yield

**Where:** `crates/openwurli-dsp/src/reed.rs`.

**Why this, not two voices:** the seven modes of **one** reed already share
control flow (one onset, one damper flag, one jitter tick). They differ only in
`(s, c, cos_inc, sin_inc, amplitude, envelope, …)`. That is a natural 2-wide
(or 4-wide on AVX) vector. Cross-voice SIMD fights per-note branches
(damper, onset, attack-noise done, pickup knee) and unused lanes.

**Layout:** SoA **inside** `ModalReed` (seven envelopes in a row, seven `s` in a
row). Keep `Voice` as AoS. Do not SoA the 64-slot engine until this pays off.

**Crate:** `wide` is fine (stable, used in Rust audio). Prefer it over
`std::simd` until that is stable on the 1.85 toolchain.

**Accept:** bit-identical vs scalar on the existing reed tests
(`test_jitter_deterministic_with_same_seed`, onset, decay, frequency). If
`wide` cannot match scalar `f64` ops, drop the crate and write NEON/`f64x2`
intrinsics that do.

**Do not:** `f32x4` modes. Modal decay and OU jitter are `f64` for a reason;
the metallic-coherence problem was already solved with 0.04% jitter in `f64`.

### 3. Mix-bus loop — exact, small

**Where:** `engine.rs` `render_voices_to_preamp_out`.

```text
voice.render(&mut voice_buf[..len]);
for (sum, v) in sum_buf[..len].iter_mut().zip(&voice_buf[..len]) {
    *sum += *v;
}
```

Same for the steal-fade loop. Optional `get_unchecked` only after a criterion
shows the zip form still checks.

**Fidelity:** bit-identical. Yield: small. Do it while touching the mix loop
anyway.

### 4. Damper-ramp `exp` — exact rewrite

**Where:** `reed.rs` damper-active branch.

Today each ramp sample does `envelope *= (-damper_rate * t / ramp).exp()` per
mode (7 `exp`s, 8–50 ms). Replace with a per-mode multiplier updated from
**precomputed** `exp(-damper_rate / ramp)` so the product is algebraically the
same sequence, or step the exponent in `f64` and `exp` once if the current
quadratic-in-`t` envelope is the intended felt model.

**Fidelity:** must match current envelopes within a few ulps on the damper
tests. If the closed form is not identical, keep the `exp` and skip.

**Yield:** only during note-off ramps. Still cheaper than a LUT and exact.

### 5. Reciprocal of true constants — exact, hunt with a criterion

**Where:** physical models + `melange` leftovers + this wrapper's `FastFinish`.

LLVM already turns `/ 2.0` and `/ 8.0` into multiplies. Hunt **runtime**
constant divisors that survive in release asm (`speaker` coeffs, `HEADROOM`,
`SPEAKER_LOAD_OHMS` if not folded).

Do **not** replace data-dependent divides:

- pickup `y / (1.0 - y)` and `(m_avg + beta)`
- NR `residual / jacobian`
- BJT `icc / qb`

Those are the model, not missing fast-math.

This wrapper: `*sample /= 1.0 + 1.2 * sag_envelope` is one divide by a varying
term. `* 1.0 / denom` is the same instruction.

### 6. Behavioral amp `exp`/`tanh` table — first *careful* approximation

**Where:** `power_amp.rs` `behavioral::forward_path`.

This is the transcendental the proposal pointed at, and it is on the **2×
oversampled** path. Up to 16 `exp` + 16 `tanh` per base sample, one shared amp.

**How to keep it inaudible:**

1. Table `exp(-v²/vt²)` and `tanh` over the clamped domain only.
2. Linear interpolation at minimum; cubic if THD moves.
3. Feed the **same** interpolated `tanh` into the analytic derivative
   `1 - tanh²` so NR stays consistent.
4. Gate with `power_amp` THD probe, `test_engine_peak_below_unity_at_vol_1`,
   `alias_audit_regression`, and a chord-ff A/B (peak, RMS, H2–H8 within
   ~0.1 dB).

If NR iteration count rises or the alias audit fails, revert. Do not put this
behind a "Low CPU" switch that still claims circuit identity.

### 7. BJT `exp(Vbe/Vt)` table — second careful approximation

**Where:** `dk_preamp_legacy.rs` `bjt()`.

Two `exp`s per device, two devices, once per oversampled sample. This **is**
the MNA nonlinearity. A dense interpolated LUT on `Vbe ∈ [-1, VBE_MAX]` is the
standard analog-model trick and can be inaudible.

**Gate:** existing legacy-vs-melange bounds (±0.18 dB gain, ±0.15 dB / 1/3 oct
harmonics, THD 0.79%, tremolo 6.10 dB). If any move, the table is too coarse.

Do this **after** measurement. If voices dominate a 12-note chord, mode SIMD
wins more than this.

### 8. Cross-voice `f64x2` / SoA engine — exact in theory, last among exact work

Bundle two **active** voices into `wide::f64x2` only after intra-voice SIMD
exists. Requires:

- compact active-voice list (skip `Free` slots first — that alone is a win)
- SoA of reed + pickup state
- masks for damper / onset / noise / pickup knee
- leftover odd voice

Pickup `soft_saturate` and attack noise make this messy. Yield is real only at
high polyphony. Do not start here.

### 9. Explicit Low CPU reed wavetable — audible by design

Pre-render 64 notes × velocity layers into memory, play through live pickup +
preamp + amp.

**Loses:** OU jitter, progressive damper, velocity onset shape, mid-note
character (reed decay / hammer / pickup drive are note-on today but a table
freezes them), MLP-at-note-on unless baked into layers, attack-noise coupling.

That is a second instrument, not an optimization of this one. If product wants
it, ship as a named mode with its own tests. Never as the default path and
never claimed as "same physics, half CPU."

---

## Do not do (as "no audible compromise")

1. **`f32x4` MNA / voices.** BJT `exp(Vbe/Vt)` and NR residuals are why the
   crate is `f64`. The 8-node solver already dropped Early effect to keep the
   kernel small; do not also drop mantissa.
2. **Pickup `1/(1-y)` LUT.** The divide is the cheap part. The bark **is** that
   function. Soft-saturate `tanh` only runs above `y = 0.94` (chord-ff grazing).
3. **Hammer Gaussian LUT.** Note-on only.
4. **Unsafe `get_unchecked` as a first step.** Iterators first; asm-proof
   second; unsafe last.
5. **GPU.** Agrees with the proposal. Buffer sizes and the serial NR solvers
   do not fill a GPU.

## Suggested sequence in the fork

1. **Done on this VM:** profile release chord + pad. See
   `docs/cpu-split-profile.md`. Re-run on the target host before LUTs.
2. **Done:** SoA + `f64x2` inside `ModalReed` (task 2) + mix zip (task 3).
   Locked by `reed::tests::test_render_checksum_c4` (`6843218719074185147`)
   in debug and release.
3. Damper closed form skipped. Repeated `exp(-rate * t / ramp)` is not
   bit-identical to a running multiplier.
4. Only if a profiler still shows the amp/preamp: tables 6 then 7, each
   behind the existing DSP gates.
5. When the fork can accept the commit, replace `vendor/openwurli-dsp` with
   a git `rev`. Re-run wrapper MIDI / finish tests.

## Wrapper-only leftovers

`FastFinish` hiss/sag are already cheap and already approximations. No SIMD,
no LUT. Leave them unless a profiler points at the stereo copy, in which case
keep the existing `copy_from_slice`.
