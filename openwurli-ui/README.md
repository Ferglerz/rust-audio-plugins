# OpenWurli UI

An independent CLAP/VST3 instrument with the [OpenWurli](https://github.com/hal0zer0/openwurli) synthesis engine and a Pleasant UI editor. Its plugin IDs are distinct from upstream OpenWurli so both can be installed in one host.

The main panel exposes Volume, Tremolo Depth, Speaker Character, and MLP Corrections. The cog opens sound settings for Reed Decay, Hammer Hardness, Pickup Drive, Tremolo Response, Hiss, Noise Level, and Sag. The four voicing multipliers are neutral at 1.0×; the first three affect newly started notes. OpenWurli v0.7.0 defaults MLP Corrections to **off**; this wrapper follows that source default.

## Upstream source

The engine is a revision-pinned Git dependency from the [Ferglerz fork](https://github.com/Ferglerz/openwurli)
on `codex/pleasant-controls`; `Cargo.toml` records the exact revision. The DSP
changes are maintained independently on
[`codex/dsp-cpu-optimizations`](https://github.com/Ferglerz/openwurli/tree/codex/dsp-cpu-optimizations),
based on original upstream OpenWurli v0.7.0 (`3023a8a6c42c654c5caeec52fd22dcf2cff3cb15`),
and merged into the controls branch. This wrapper has no private DSP source copy.

The **ENGINE** page describes the active native processing and measured CPU/audio
results. See [the implementation and evidence report](docs/cpu-optimization-results.md)
for before/after timings, bitwise audio comparisons, rejected lookup candidates,
reproduction commands and limitations. Original analytical equations remain
permanent reference code; experimental tables are generated from those equations
and are **disabled** in this plugin. Original complete engines can be reconstructed
from pinned Git revisions by the fork's comparison tools.

The wrapper uses OpenWurli's default fast preamp and power amp. Its Reed, Hammer, Pickup, and Tremolo controls still call the fork's engine APIs. Hiss and Sag are inexpensive output effects in the wrapper: Hiss adds low-level noise after the engine, and Sag applies a level-dependent gain reduction. Both default off, preserving the fast engine's output until enabled. These approximate the heavier circuit features rather than reproducing their exact electrical behavior. Existing host parameter IDs and ranges are retained for preset compatibility; saved projects with Sag enabled keep that setting.

For future updates, port the optimization branch against the intended upstream
revision, repeat its native/table comparisons, then integrate it into
`codex/pleasant-controls` and update the pinned dependency here. Do not silently
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

This release keeps the prior CPU optimizations. Additional CPU candidates are
deferred pending timing qualification; the failing damper approximation is not
included. See [runtime modes and study status](docs/runtime-modes.md) for the
changes, completed checks, remaining validation and preserved research.
