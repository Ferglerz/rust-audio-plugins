# OpenWurli UI

An independent CLAP/VST3 instrument with the [OpenWurli](https://github.com/hal0zer0/openwurli) synthesis engine and a Pleasant UI editor. Its plugin IDs are distinct from upstream OpenWurli so both can be installed in one host.

## Upstream update (0.2.1)

The DSP tracks upstream **v0.9.1 “RestlessHeart”**, revision [`c05403b9b60f95b4908ae19727682e81803a7351`](https://github.com/hal0zer0/openwurli/commit/c05403b9b60f95b4908ae19727682e81803a7351), checked against upstream `main` on November 22, 2026. This brings upstream's removal of the hammer attack noise burst, retired engine cleanup, and MIDI/CLAP note-control fixes. The local Fast/Heavy modes, character controls, extended notes, linear Volume, Extra Sag and plugin IDs are preserved.

Wildcard note-on is ignored; wildcard note-off and CC 123/124–127 release every key while respecting sustain. CC 121 lifts sustain. CC 120 and CLAP choke fade the affected voices over 5 ms without resetting the circuit. Individual choke targets the exact pitch, including extended notes held after Extended Notes is disabled.

**Existing sessions retain their controls and automation IDs.** The v0.9.1 attack differs slightly because the old noise burst is gone. Sessions originating before wrapper 0.2.0 also adopt the v0.9 physical model and its output level. Restore the previous source/build for exact older playback.

See [local DSP changes](vendor/openwurli-dsp/LOCAL_CHANGES.md) for the integration boundaries. The earlier [CPU report](docs/cpu-optimization-results.md), [runtime-mode report](docs/runtime-modes.md), and [Heavy recovery proof](docs/heavy-recovery-shortcut-2026-10-03.md) describe the previous v0.7 engine; their performance and bit-equivalence claims do not apply to this version. The old SIMD reed/transistor paths, experimental circuit tables, shared shadow-preamp solve, and Heavy recovery shortcut are superseded by the v0.9 source. Generated preamp and power-amplifier circuit equations remain unchanged; their transistor helpers reuse an unchanged final evaluation, as the local tremolo solver already did.

The [first v0.9.1 optimization pass](docs/optimizations-2026-11-22/README.md) rejected a reed SIMD candidate with a release regression. The [SIMD follow-up](docs/simd-investigation-2026-11-22/README.md) keeps the fundamental scalar for air drag and uses consistent upper-mode pairs through damping, oscillation and renormalization. This revised implementation matches the scalar output bit for bit and passes the performance gates. In the measured 32/64-voice cases, isolated reeds use about 19% less time while held and 10–11% less while releasing; complete Fast rendering improves by about 2.8/3.9% for held notes. Heavy gains are under 1%. The original rejected implementation and scalar reference remain in the wrapper tests.

## Controls

The 720 × 556 panel exposes Volume, Tremolo Depth, Tremolo Response and Speaker Character above Reed Decay, Hammer Hardness and Pickup Drive, alongside Fast/Heavy, Extended Notes and Extra Sag switches. Main knobs use vertical dragging and double-click reset. Drawing and pointer input fit the artwork to the available host size; the background fills the editor.

Reed Decay spans 0.5–20×; Hammer Hardness and Tremolo Response span 0.5–2×; Pickup Drive spans 0.5–1.04×. All multipliers are neutral at 1×. Reed and hammer settings affect newly started notes. Pickup Drive scales the physical swing in millimetres, including the associated air-drag loss. Tremolo Response changes LDR timing live.

Learned voicing is always active, now using upstream's corrected-pitch training and its tuning/decay corrections. The retired MLP pickup-displacement correction is no longer applied. The retired `mlp` and noise IDs are ignored when loading old state. Hiss stays disabled in both native preamps.

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

All 34 wrapper and integration tests pass in release mode, including exact transistor-helper comparisons in all three generated solvers and comparisons of both the original SIMD experiment and the production reed against the frozen scalar reference. Compile checks (including test targets) and crate-scoped Clippy also pass; Clippy reports three existing warnings in the shared Pleasant UI dependency, plus a fixture-reader warning with all test targets; treating dependency warnings as errors fails in Pleasant UI. The wrapper tests cover saved parameters, MIDI timing, sustain, wildcard messages, channel-mode controls, choke, extended notes, character controls, linear output volume and sag. [Upstream audio fixtures](tests/fixtures/upstream-0.9.1/README.md) compare neutral controls with independent unmodified upstream renders in Fast and Heavy modes at 44.1, 48 and 96 kHz. A separate regression covers live mode changes after buffer growth and sample-rate changes. No DAW UI automation is used.

## License and attribution

OpenWurli DSP is by hal0zer0 and is licensed under GPL-3.0-or-later. This wrapper and its Pleasant UI dependency are also GPL-3.0-or-later; the license text is in [LICENSE](LICENSE). Preserve upstream's license and source attribution when distributing binaries; the complete corresponding source includes the pinned OpenWurli revision and this wrapper.
