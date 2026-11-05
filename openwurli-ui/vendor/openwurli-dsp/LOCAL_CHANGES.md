# Vendored OpenWurli DSP

Source: https://github.com/hal0zer0/openwurli, `crates/openwurli-dsp` at
`531680b5b49eb4a734a8d8ef7a9c13cb83cf79f9` (v0.9.0, upstream main checked
November 5, 2026). The standalone manifest expands upstream workspace metadata
and its unchanged melange dependency pin.

Local integration changes:

- `runtime-models` compiles both upstream circuit pairs. `engine.rs` retains the
  shared voice bank, 100 ms live-input settling and 20 ms crossfade. Each chain
  owns its C-9 filter history; physical volume/trim coefficients are advanced
  once per base-rate sample and shared during transitions.
- `engine.rs` keeps explicit full-MIDI note-on/off methods alongside upstream's
  clamped methods. The UI owns note-range selection. Buffer capacity survives
  sample-rate changes. Reed-bar trim smoothing resets with the other controls.
- `voice.rs` retains unity-neutral note-on Reed Decay, Hammer Hardness and Pickup
  Drive controls. Pickup Drive now scales upstream's physical swing and the
  corresponding air drag, rather than the removed parallel-plate displacement
  model. Upstream calibration and build-reed entry points remain available.
- `tremolo.rs` retains the response multiplier and its neutral-preserving
  coefficient caching. `gen_tremolo.rs` retains reuse of an unchanged final
  transistor evaluation after early convergence. `dk_preamp/mod.rs`, `power_amp.rs` and `lib.rs` expose the
  circuit pairs for runtime selection. The generated preamp/power-amp solvers, reed oscillator,
  pickup, tables and learned weights are unmodified upstream source.

The v0.7 fork's SIMD, circuit-LUT and Heavy recovery modifications are removed:
those proofs were specific to the old physics/solver. `wide` is no longer needed.
The previous baseline was Ferglerz/openwurli revision
`1899523a3eb7a73a7f9c905c8c04ee04a06b415d`. Historical reports under `../../docs/`
remain evidence for that version only.

The wrapper owns the always-on MLP, disabled hiss, stable host parameter IDs,
linear output Volume, +14 dB alignment, and optional post-engine Extra Sag.
Old sessions load their settings but intentionally adopt the new sound model.
Rolling back requires restoring the pre-update wrapper, DSP and lockfile together
and reinstalling; no saved parameter schema is destructively migrated.

Verification lives in the wrapper crate, including independent upstream audio
fixtures in `../../tests/fixtures/upstream-0.9.0`. Do not regenerate those fixtures
from this locally adapted engine. Future refreshes should compare against the
pinned upstream first, port these extensions, and rerun the wrapper checks and
installer. The license remains upstream GPL-3.0-or-later.
