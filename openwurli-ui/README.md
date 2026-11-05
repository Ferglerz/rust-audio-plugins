# OpenWurli UI

An independent CLAP/VST3 instrument with the [OpenWurli](https://github.com/hal0zer0/openwurli) synthesis engine and a Pleasant UI editor. Its plugin IDs are distinct from upstream OpenWurli so both can be installed in one host.

## Upstream update (0.2.0)

The DSP is based on upstream **v0.9.0**, revision [`531680b5b49eb4a734a8d8ef7a9c13cb83cf79f9`](https://github.com/hal0zer0/openwurli/commit/531680b5b49eb4a734a8d8ef7a9c13cb83cf79f9), checked against upstream `main` on November 5, 2026. This replaces the v0.7-based Ferglerz fork. It brings the physical reed/finger-overlap pickup, hammer-derived reed swing, amplitude-dependent air drag, steadier partials, retrained learned voicing, revised amplifier circuits and volume network, and upstream's +14 dB plugin output alignment.

**Existing sessions retain their controls and automation IDs, but sound and level change to the v0.9 model.** The wrapper version is now 0.2.0. Keep the previous source/build if exact playback of older sessions is required.

See [local DSP changes](vendor/openwurli-dsp/LOCAL_CHANGES.md) for the integration boundaries. The earlier [CPU report](docs/cpu-optimization-results.md), [runtime-mode report](docs/runtime-modes.md), and [Heavy recovery proof](docs/heavy-recovery-shortcut-2026-10-03.md) describe the previous v0.7 engine; their performance and bit-equivalence claims do not apply to this version. The old SIMD reed/transistor paths, experimental circuit tables, shared shadow-preamp solve, and Heavy recovery shortcut are superseded by the v0.9 source. Generated preamp and power-amplifier solvers remain identical to upstream.

## Controls

The 720 × 550 panel exposes Volume, Tremolo Depth, Tremolo Response and Speaker Character above Reed Decay, Hammer Hardness and Pickup Drive, alongside Fast/Heavy, Extended Notes and Extra Sag switches. Main knobs use vertical dragging and double-click reset. Drawing and pointer input fit the artwork to the available host size; the background fills the editor.

Reed Decay spans 0.5–20×; Hammer Hardness and Tremolo Response span 0.5–2×; Pickup Drive spans 0.5–1.04×. All multipliers are neutral at 1×. Reed and hammer settings affect newly started notes. Pickup Drive scales the physical swing in millimetres, including the associated air-drag loss. Tremolo Response changes LDR timing live.

Learned voicing is always active, now using upstream's corrected-pitch training and its tuning/decay corrections. The retired MLP pickup-displacement correction is no longer applied. The old `mlp` ID remains hidden and inert. Hiss stays disabled in both native preamps; the old noise IDs also remain hidden and inert.

Volume retains its existing linear output behavior and 50% default. Internally the v0.9 physical volume network stays at upstream's nominal 80% position and factory reed-bar trim. The +14 dB alignment follows the circuit/speaker chain, then Extra Sag applies optional level-dependent compression with the existing 8 ms attack, 150 ms release and strength of 12. Volume follows these stages, so changing it does not change tone or sag. Extra Sag defaults off and saved settings survive.

Fast is the default circuit selection. Heavy uses upstream's generated preamp and power amplifier, including physical rail sag. A live change settles the incoming circuit for 100 ms, then crossfades for 20 ms without retriggering held notes. Extra Sag is a separate post-engine effect shared by both modes.

Extended Notes defaults off: normal mode accepts MIDI 33–96 and ignores notes beyond either end. Enabling it plays MIDI 0–127 at the requested pitches in both modes. Notes held when it is disabled still release correctly. Older presets restore Fast and normal note range.

## Build and verify

From the workspace root:

```sh
cargo check -p openwurli-ui
cargo test -p openwurli-ui --release --quiet
./openwurli-ui/scripts/install.sh
```

The installer rebuilds from the current source, verifies the bundled executables, installs both formats, and verifies the signed installed bundles. Reload the plugin in the host after replacing a loaded bundle.

The 21 wrapper tests pass in release mode. Compile checks (including test targets) and crate-scoped Clippy also pass. The wrapper tests cover saved parameters, MIDI timing, sustain, extended notes, character controls, linear output volume and sag. [Upstream audio fixtures](tests/fixtures/upstream-0.9.0/README.md) compare neutral controls with independent unmodified upstream renders in Fast and Heavy modes at 44.1, 48 and 96 kHz. A separate regression covers live mode changes after buffer growth and sample-rate changes. No DAW UI automation is used.

## License and attribution

OpenWurli DSP is by hal0zer0 and is licensed under GPL-3.0-or-later. This wrapper and its Pleasant UI dependency are also GPL-3.0-or-later; the license text is in [LICENSE](LICENSE). Preserve upstream's license and source attribution when distributing binaries; the complete corresponding source includes the pinned OpenWurli revision and this wrapper.
