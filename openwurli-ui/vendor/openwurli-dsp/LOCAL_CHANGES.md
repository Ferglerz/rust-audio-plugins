Vendored from https://github.com/Ferglerz/openwurli at 1899523a3eb7a73a7f9c905c8c04ee04a06b415d (openwurli-dsp 0.7.0).

Local changes: standalone manifest metadata; explicit full-MIDI note-on and note-off entry points in engine.rs. Original clamped entry points remain compatible. The UI owns range selection; extended note-off remains available after disabling the toggle so held notes can release.

The pinned fork revision includes the exact Heavy recovery shortcut, persistent solver diagnostics, the complete `reference-full-recovery` build feature and focused state-equivalence tests. The native `melange_precise_exp` configuration remains available. See ../../docs/heavy-recovery-shortcut-2026-10-03.md for measured results and frozen baseline/candidate proof.
