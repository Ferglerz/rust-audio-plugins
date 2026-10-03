# OpenWurli UI

An independent CLAP/VST3 instrument with the [OpenWurli](https://github.com/hal0zer0/openwurli) synthesis engine and a Pleasant UI editor. Its plugin IDs are distinct from upstream OpenWurli so both can be installed in one host.

The 720 × 550 main panel has two rows of knobs: Volume, Tremolo Depth and Speaker Character above Reed Decay, Hammer Hardness and Pickup Drive. The cog opens sound settings for Tremolo Response, Extra Sag and Extended Notes. Reed Decay spans 0.5–20×; Hammer Hardness, Pickup Drive and Tremolo Response span 0.5–2×. These multipliers are neutral at 1.0×. Reed, hammer and pickup settings affect newly started notes; tremolo response and extra sag apply live. Main knobs use vertical dragging and double-click reset.

Learned voicing (MLP corrections) is **always active**. A small neural network uses recorded piano references to adjust overtone tuning, decay and pickup response once at note-on. The cog panel explains this behavior. The old `mlp` automation ID remains hidden and inert for saved-session compatibility: loading a project saved with corrections off now uses the corrected sound. The engine's correction model and source revision are unchanged.

Both drawing and pointer input fit the artwork to the host's available width and height. The background fills the complete editor with square corners, including any margins around the fitted artwork.

## Upstream source

The engine is vendored from the [Ferglerz fork](https://github.com/Ferglerz/openwurli) at revision `1899523a3eb7a73a7f9c905c8c04ee04a06b415d`. See [local DSP changes](vendor/openwurli-dsp/LOCAL_CHANGES.md) for provenance and the full-MIDI note entry points. Earlier DSP
optimizations were developed on
[`codex/dsp-cpu-optimizations`](https://github.com/Ferglerz/openwurli/tree/codex/dsp-cpu-optimizations),
based on original upstream OpenWurli v0.7.0 (`3023a8a6c42c654c5caeec52fd22dcf2cff3cb15`),
and merged into the controls branch. The pinned revision adds the exact Heavy recovery shortcut on
[`codex/heavy-recovery-shortcut`](https://github.com/Ferglerz/openwurli/tree/codex/heavy-recovery-shortcut).
The local copy preserves the original clamped API and adds explicit full-range note-on/off APIs.

The Engine information page has been removed from the editor. See
[the implementation and evidence report](docs/cpu-optimization-results.md)
for before/after timings, bitwise audio comparisons, rejected lookup candidates,
reproduction commands and limitations. Original analytical equations remain
permanent reference code; experimental tables are generated from those equations
and are **disabled** in this plugin. Original complete engines can be reconstructed
from pinned Git revisions by the fork's comparison tools.

The wrapper uses OpenWurli's default fast preamp and power amp. Its Reed, Hammer, Pickup, and Tremolo controls still call the fork's engine APIs. Extra Sag applies optional level-dependent output gain reduction and defaults off. Hiss has been removed from the wrapper and native preamps in both circuit modes. The old noise parameter IDs remain hidden and inert for saved-session compatibility. Saved projects with Sag enabled keep that setting.

For future updates, port the optimization branch against the intended upstream
revision, repeat its native/table comparisons, then integrate it into
`codex/pleasant-controls` and refresh the vendored source here, preserving the documented local changes. Do not silently
change the sound model while benchmarking a CPU patch: current upstream v0.9
changes reed physics, pickup and amplifier behavior relative to this v0.7 engine.
UI changes stay in this repository. The upstream-based branch is prepared for a
later port/review; no pull request has been opened.

## Build

From the audio-plugins workspace root:

```sh
cargo check -p openwurli-ui
cargo run -p openwurli-ui-xtask -- bundle openwurli-ui --release
```

The bundles appear in `target/bundled/`.

On macOS, run `./openwurli-ui/scripts/install.sh` from the workspace root. It uses the shared installer to rebuild from source and verify the installed CLAP and VST3 bundles. Reload the plugin in the host after replacing a loaded bundle.

## License and attribution

OpenWurli DSP is by hal0zer0 and is licensed under GPL-3.0-or-later. This wrapper and its Pleasant UI dependency are also GPL-3.0-or-later; the license text is in [LICENSE](LICENSE). Preserve upstream's license and source attribution when distributing binaries; the complete corresponding source includes the pinned OpenWurli revision and this wrapper.

## Fast and Heavy circuit modes (0.1.1)

The main-page **CPU: FAST / CPU: HEAVY** button selects and saves the circuit
model. Fast remains the default for old presets. A live change settles the
incoming circuit for 100 ms, then crossfades for 20 ms without retriggering notes.
Heavy retains its physical amplifier rails; **Extra Sag** is the existing shared
post-engine effect. The models can sound different.

An exact CPU pass reduces render time in both models while keeping their output
bit-identical to the previous 0.1.1 engine. Heavy gains most: its preamp matrix
rebuilds are shared and its transistor solves run in lockstep. The failing
damper approximation is not included. See [runtime modes and CPU
evidence](docs/runtime-modes.md) for the changes, measurements, completed checks
and remaining validation.

A later Heavy-only optimization skips recovery work that the existing amplifier
guard would discard. Paired M1 benchmarks measured 27.4% less render time for six
notes and 47.1% less for 64 notes at 50% tremolo, with exact output in the tested
matrix. Fast remains unchanged. The complete solver is retained, including a
`reference-full-recovery` build feature and the native exponential configuration.
See [the recovery shortcut proof](docs/heavy-recovery-shortcut-2026-10-03.md) for
coverage, reference builds and remaining latency limits.

## Extended notes

The advanced panel’s **Extended Notes** toggle defaults off. Normal mode accepts MIDI 33–96 and ignores notes outside either end without striking or releasing an end key. Enabling it plays the full MIDI 0–127 range at the requested pitches in Fast and Heavy modes. Notes already held still release correctly after the toggle is disabled. The setting is saved and automatable; old presets select the normal range.
