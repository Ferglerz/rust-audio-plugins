# Subtree development

`scd-rust/` is a Git subtree imported with full history from
https://github.com/Ferglerz/scd-rust.git. Its Cargo workspace remains separate;
run its Cargo commands from `scd-rust/`. It uses the shared `pleasant-ui/`
directory in this repository.

Run these Git commands from the audio-plugins repository root, with a clean
working tree. On a new clone, first register the remote:

```sh
git remote add scd-rust https://github.com/Ferglerz/scd-rust.git
```

Import changes from the standalone repository:

```sh
git subtree pull --prefix=scd-rust scd-rust main
```

Commit local edits normally in audio-plugins, then export the scd-rust commits:

```sh
git subtree push --prefix=scd-rust scd-rust main
```

To publish a feature branch instead, replace the final `main` with the desired
remote branch name. Pull and resolve upstream changes before exporting if the
remote has advanced. Do not force-push to bypass divergence.

A normal `git push origin` publishes audio-plugins only. Subtree pushes export
only `scd-rust/`; changes to `pleasant-ui/` are not included. A standalone
scd-rust checkout currently requires a sibling `pleasant-ui/` checkout.

The other plugins and pleasant-ui remain submodules for now.
