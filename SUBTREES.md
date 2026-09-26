# Subtree development

All plugins and the shared UI are Git subtrees with their full histories:

- `Damian Channel Strip/`: https://github.com/Ferglerz/Damian-Channel-Strip.git
- `Flattery/`: https://github.com/Ferglerz/Flattery.git
- `Tape Stop/`: https://github.com/Ferglerz/Tape-Stop.git
- `pleasant-ui/`: https://github.com/Ferglerz/pleasant-ui.git
- `scd-rust/`: https://github.com/Ferglerz/scd-rust.git
- `Composure_Rust/`: https://github.com/Ferglerz/Composure_Rust.git
- `openwurli-ui/`: https://github.com/Ferglerz/openwurli-ui.git
- `chordboard/`: https://github.com/Ferglerz/Chordboard.git

A normal clone includes their tracked files; no submodule initialization is
needed. Commit edits normally in audio-plugins, including coordinated changes
across plugins and shared code.

## Remote setup

Remotes are local Git configuration. On a new clone, register them once:

```sh
git remote add damian https://github.com/Ferglerz/Damian-Channel-Strip.git
git remote add flattery https://github.com/Ferglerz/Flattery.git
git remote add tape-stop https://github.com/Ferglerz/Tape-Stop.git
git remote add pleasant-ui https://github.com/Ferglerz/pleasant-ui.git
git remote add scd-rust https://github.com/Ferglerz/scd-rust.git
git remote add composure https://github.com/Ferglerz/Composure_Rust.git
git remote add openwurli-ui https://github.com/Ferglerz/openwurli-ui.git
git remote add chordboard https://github.com/Ferglerz/Chordboard.git
```

## Two-way sync

Run from the audio-plugins root with a clean working tree. Import upstream
changes using the matching folder and remote:

```sh
git subtree pull --prefix="Damian Channel Strip" damian main
git subtree pull --prefix=Flattery flattery main
git subtree pull --prefix="Tape Stop" tape-stop main
git subtree pull --prefix=pleasant-ui pleasant-ui main
git subtree pull --prefix=scd-rust scd-rust main
git subtree pull --prefix=Composure_Rust composure main
git subtree pull --prefix=openwurli-ui openwurli-ui main
git subtree pull --prefix=chordboard chordboard main
```

Export committed changes to an individual repository:

```sh
git subtree push --prefix="Damian Channel Strip" damian main
git subtree push --prefix=Flattery flattery main
git subtree push --prefix="Tape Stop" tape-stop main
git subtree push --prefix=pleasant-ui pleasant-ui main
git subtree push --prefix=scd-rust scd-rust main
git subtree push --prefix=Composure_Rust composure main
git subtree push --prefix=openwurli-ui openwurli-ui main
git subtree push --prefix=chordboard chordboard main
```

Run only the commands for repositories you intend to sync. Replace the final
`main` on a push with a feature branch name to publish for review. Pull and
resolve upstream changes before exporting if the remote has advanced; do not
force-push to bypass divergence. Keep these full-history imports consistent by
omitting `--squash`.

A normal `git push origin main` publishes audio-plugins only. Subtree pushes
export only the selected folder and do not include changes to sibling folders.
Publishing coordinated changes to separate repositories takes separate pushes.

`openwurli-ui` is the independent plugin wrapper. Its pinned OpenWurli DSP
dependency remains in `Ferglerz/openwurli` and is not a subtree of this repo.

## Cargo workspaces and shared code

The original plugins and `pleasant-ui` use the root Cargo workspace.
`scd-rust` retains its separate Cargo workspace; run its Cargo commands from
`scd-rust/`.

All plugins use the shared `pleasant-ui/` directory. Standalone plugin checkouts
currently require a sibling `pleasant-ui/` checkout. Root Cargo dependency
patches also remain part of the integrated build configuration; subtree export
does not automatically include them in standalone plugin repositories.

The patched NIH-plug, baseview, and Vizia runtime sources currently live under
`Damian Channel Strip/vendor/`. Keeping those sources inside Damian's subtree
lets its standalone export include the exact host patches it uses. The root
workspace and `scd-rust/Cargo.toml` point to that copy, so a standalone SCD
checkout also needs a sibling Damian checkout for those path patches. Moving
the forks to a neutral root directory requires a replacement dependency source
for standalone exports and verification of both Cargo workspaces and lockfiles.
