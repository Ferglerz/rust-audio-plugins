# Vendored OpenWurli DSP

Source: https://github.com/hal0zer0/openwurli, `crates/openwurli-dsp` at
`c05403b9b60f95b4908ae19727682e81803a7351` (v0.9.1, upstream main checked
November 22, 2026). The standalone manifest expands upstream workspace metadata
and its unchanged melange dependency pin.

Local integration changes:

- The scalar modal rotation now calls the workspace’s `pleasant-dsp` primitive, also used by Stage 73. The operand order is preserved. The local DSP manifest resolves that crate by relative path.

- The shipping host compiles only the Fast preamp and power amplifier. The regression feature `runtime-models` compiles both upstream circuit pairs. `engine.rs` retains the
  shared voice bank, 100 ms live-input settling and 20 ms crossfade. Each chain
  owns its C-9 filter history; physical volume/trim coefficients are advanced
  once per base-rate sample and shared during transitions.
- `engine.rs` keeps explicit full-MIDI note-on/off/choke methods alongside upstream's
  clamped methods. It tracks editor audition voices independently of host MIDI
  and exposes held and pedal-sustained pitches for the keyboard display.
  The host can clear the key bank when changing pianos while keeping the Wurli circuit warm.
  The UI owns note-range selection. Buffer capacity survives
  sample-rate changes. Reed-bar trim smoothing resets with the other controls.
- `voice.rs` retains unity-neutral note-on Reed Decay, Hammer Hardness and Pickup
  Drive controls. Pickup Drive now scales upstream's physical swing and the
  corresponding air drag, rather than the removed parallel-plate displacement
  model. Upstream calibration and build-reed entry points remain available.
- `pickup.rs` accepts a global height trim in millimetres. Sounding voices follow the comb over 10 ms. Both
  the overlap lookup and the resting capacitance use the new finger position.
  The bass and treble orientations follow the existing pickup law.
- `tremolo.rs` scales the photocell time constants from ⅛× to 8× and caches the coefficients. `gen_tremolo.rs` retains reuse of an unchanged final
  transistor evaluation after early convergence. `dk_preamp/mod.rs`, `power_amp.rs` and `lib.rs` expose the
  circuit pairs for runtime selection. The generated preamp and power-amplifier
  BJT helpers also reuse their last pure evaluation on early exits that leave
  the internal junction voltages unchanged. All three helpers retain the final
  evaluation after a voltage update, including iteration exhaustion. Circuit
  equations, convergence thresholds and Jacobian calculations are unchanged.
  Their test-only `reuse_tests` modules share the frozen original helper in
  `../../tests/support/bjt_reuse_cases.rs`, loaded by the wrapper integration test.
  Preserve these small local changes and test hooks when regenerating the solvers.
  The field-solved pickup tables and learned weights are unmodified upstream source.

The v0.9.1 upstream note-control APIs and hammer-contact/damper calibration
entry points are retained. The engine combines that calibration with local
note-on character controls; the experimental upstream multipliers stay at unity
in the plugin. The wrapper handles wildcard note events, MIDI channel-mode
controls and choke using those APIs, without clamping extended choke pitches.
The hammer attack noise is removed exactly as upstream.

The v0.7 fork's SIMD, circuit-LUT and Heavy recovery modifications are removed:
those proofs were specific to the old physics/solver. The first fresh v0.9.1
reed-SIMD candidate missed the performance gate. A revised implementation now
keeps mode 0 scalar for air drag and pairs modes 1/2, 3/4 and 5/6 consistently
through damping, oscillation and renormalization. It retains f64 precision,
scalar exponential evaluations and the original contribution order. Exact-output
comparisons and fresh performance gates pass; `wide` 0.7.33 is a production
DSP dependency as well as a wrapper-test dependency. See
`../../docs/simd-investigation-2026-11-22` for the source snapshots and evidence;
`../../docs/optimizations-2026-11-22` retains the earlier rejected experiment.
The previous baseline was Ferglerz/openwurli revision
`1899523a3eb7a73a7f9c905c8c04ee04a06b415d`. Historical reports under `../../docs/`
remain evidence for that version only.

The wrapper owns the always-on MLP, disabled hiss, stable host parameter IDs,
linear output Volume, +14 dB alignment, and optional post-engine Extra Sag.
Old sessions load their settings but intentionally adopt the new sound model.
Rolling back requires restoring the pre-update wrapper, DSP and lockfile together
and reinstalling; no saved parameter schema is destructively migrated.

Verification lives in the wrapper crate, including independent upstream audio
fixtures in `../../tests/fixtures/upstream-0.9.1`. Do not regenerate those fixtures
from this locally adapted engine. Future refreshes should compare against the
pinned upstream first, port these extensions, and rerun the wrapper checks and
installer. The license remains upstream GPL-3.0-or-later.
