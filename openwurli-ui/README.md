# OpenWurli UI

An independent CLAP/VST3 instrument with the [OpenWurli](https://github.com/hal0zer0/openwurli) synthesis engine and a Pleasant UI editor. Its plugin IDs are distinct from upstream OpenWurli so both can be installed in one host.

The main panel exposes Volume, Tremolo Depth, Speaker Character, and MLP Corrections. The cog opens sound settings for Reed Decay, Hammer Hardness, Pickup Drive, Tremolo Response, Hiss, Noise Level, and Sag. The four voicing multipliers are neutral at 1.0×; the first three affect newly started notes. OpenWurli v0.7.0 defaults MLP Corrections to **off**; this wrapper follows that source default.

## Upstream source

The dependency is the [Ferglerz fork](https://github.com/Ferglerz/openwurli), pinned in `Cargo.toml` to commit `6614ab9519471e956ecb0c82edc4fcf295f724dc` on `codex/pleasant-controls`. The fork's `main` stays equal to upstream commit `3023a8a6c42c654c5caeec52fd22dcf2cff3cb15` (OpenWurli v0.7.0). The branch adds only DSP control APIs and a short design note; all plugin and UI code lives in this directory.

The wrapper uses OpenWurli's default fast preamp and power amp. Its Reed, Hammer, Pickup, and Tremolo controls still call the fork's engine APIs. Hiss and Sag are inexpensive output effects in the wrapper: Hiss adds low-level noise after the engine, and Sag applies a level-dependent gain reduction. Both default off, preserving the fast engine's output until enabled. These approximate the heavier circuit features rather than reproducing their exact electrical behavior. Existing host parameter IDs and ranges are retained for preset compatibility; saved projects with Sag enabled keep that setting.

For an update, fetch upstream in a clone of the fork, verify the fork's `main` is an ancestor of the target upstream commit, and fast-forward `main`. Rebase `codex/pleasant-controls` onto the new upstream commit, resolve any DSP API conflicts there, and update the dependency `rev` here. Keep plugin and UI changes outside the fork. Run the fork's targeted DSP tests and wrapper checks before releasing.

## Build

From the audio-plugins workspace root:

```sh
cargo check -p openwurli-ui
cargo test -p openwurli-ui --quiet
cargo run -p openwurli-ui-xtask -- bundle openwurli-ui --release
```

The bundles appear in `target/bundled/`.

On macOS, run `./openwurli-ui/scripts/install.sh` from the workspace root. It uses the shared installer to rebuild from source and verify the installed CLAP and VST3 bundles. Reload the plugin in the host after replacing a loaded bundle.

## License and attribution

OpenWurli DSP is by hal0zer0 and is licensed under GPL-3.0-or-later. This wrapper and its Pleasant UI dependency are also GPL-3.0-or-later; the license text is in [LICENSE](LICENSE). Preserve upstream's license and source attribution when distributing binaries; the complete corresponding source includes the pinned OpenWurli revision and this wrapper.
