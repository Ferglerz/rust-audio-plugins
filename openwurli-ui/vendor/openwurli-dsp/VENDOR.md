Vendored from Ferglerz/openwurli at `6614ab9519471e956ecb0c82edc4fcf295f724dc` (`codex/pleasant-controls`), plus the reed SIMD and mix-bus changes from this PR.

This crate is GPL-3.0-or-later (OpenWurli / hal0zer0). Keep it here only until those DSP commits can land on the fork; then switch `Cargo.toml` back to a git `rev` and delete this directory.

This agent cannot push to `Ferglerz/openwurli` (token has no write). Land the SIMD commit there with:

```sh
git checkout codex/pleasant-controls
git am ../openwurli-ui/docs/upstream-reed-simd.patch
git push origin codex/pleasant-controls
```

Then pin that SHA here and drop this directory.
