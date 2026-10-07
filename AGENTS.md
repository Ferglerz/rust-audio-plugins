# Rust verify ladder

## Pull requests

Do not create or submit a pull request, including a draft or an upstream PR, without the user's explicit approval. Prepare the changes and checks first, then show the target repository, branch, proposed summary, and verification before asking to open it. Discussing a possible PR or authorizing implementation does not count as approval. A direct instruction to create or submit that PR does.

## REAPER interaction

Do not use computer-use or UI automation to inspect or control REAPER for this project. Verify plugins through source inspection, focused checks, builds, and installed bundle inspection instead.

## Plugin installation

After changing a plugin, run that plugin's `scripts/install.sh`. All seven wrappers call `scripts/install-plugin.sh`, which clears the plugin's release artifact, builds from the current checkout, verifies the bundled executable, installs it, and verifies the installed bundle. `cargo xtask bundle` alone does not update the DAW installation.

Do not run `cargo test` after every edit. Tests stay in the repo. Default verify is a compile-check of the crate you touched.

## Default

`cargo check -p <crate>`

Plugin crate names: `composure`, `damian-channel-strip`, `flattery`, `tape_stop`, `openwurli-ui`, `chordboard`, `fundament`. Check `scd-plugin` from `scd-rust/`, its separate Cargo workspace.

- No `--workspace`
- No `--all-targets` unless tests or examples themselves changed
- Never check or test vendored crates: `nih_plug`, `nih_plug_vizia`, `baseview`, `vizia_baseview`

## Run tests when

- DSP, engine, params, math, or processing changed
- `cargo check` is clean but runtime behavior is in doubt
- The user asks, or a failure needs a targeted re-run

Then: `cargo test -p <crate> <optional_filter> --quiet`

Quote failing test names and the decisive assertion only. Do not paste passing test lists.

## Full suite

CI or an explicit user request. Not every agent turn.
