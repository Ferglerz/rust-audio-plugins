# Composure

Composure is a member of the shared audio-plugins Cargo workspace. It uses the
workspace's patched NIH-plug/Vizia dependencies and `pleasant-ui` renderer.

The editor's **Light / Dark / Analog** selector changes only presentation:

- **Dark** (default) and **Light** use the Pleasant palette, knobs, flat switches,
  labeled sections and numeric slider readouts, following Composure's layout.
- **Analog** keeps the original image-based faceplate and controls, with the
  interactive curve screen embedded in the panel.

All three share the same widget instances, event handlers, parameter bindings,
curve editor, telemetry and DSP. Changing appearance does not recreate the editor,
reset the graph, alter automation IDs, or change sound. Appearance is a local
preference saved under the platform's application-support directory for Composure;
it is not an automated audio parameter. Other plugins keep their two-state switch.

From the repository root:

```sh
cargo check -p composure
./scripts/install.sh
```

`scripts/install.sh` builds the release VST3 and CLAP bundles, then copies and
ad-hoc signs them into the macOS user plugin folders. From this folder,
`cargo xtask bundle composure --release` only bundles.
Build output and dependency locking belong to the workspace root; the old nested
`target/` and ignored `Cargo.lock` are no longer used.

Host smoke check: automate a parameter, edit curve points and switch through all
three appearances. Confirm values and points persist, drags/reset/fine adjustment
work in each skin, the curve/meters animate during playback, and reopening the
editor restores the selected appearance.

The previous repository's Git metadata is preserved locally in the ignored
`../.composure-git-backup/` directory. Composure's source is now tracked by the
shared repository rather than as a nested repository.
