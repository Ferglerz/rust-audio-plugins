# Upstream v0.9.0 audio references

These six files contain 4096 mono little-endian f32 samples each, generated from
**unmodified** hal0zer0/openwurli at
`531680b5b49eb4a734a8d8ef7a9c13cb83cf79f9`, using Rust 1.98.1 release on macOS
arm64. They precede plugin-level gain/sag/volume. The generator uses learned
voicing, speaker character 0.65, tremolo depth 0.7, neutral character, and a fixed
sequence of chord, sustain, release and restrike events. Three rates exercise
both oversampled and native-rate processing.

`../../upstream_audio.rs` compares the locally adapted runtime engine with these
samples using absolute tolerance 1e-7 plus relative tolerance 1e-5, allowing minor
platform floating-point differences. These are sound-model regressions, not CPU
benchmarks or an exhaustive stability/aliasing assessment.

To reproduce, clone the upstream repository into a temporary directory, check
out the exact revision above, and create a separate temporary Cargo binary with
`generate.rs` as `src/main.rs` and this manifest (substitute the clone path):

```toml
[package]
name = "openwurli-reference-probe"
version = "0.1.0"
edition = "2024"
[dependencies]
openwurli-dsp = { path = "/absolute/upstream/crates/openwurli-dsp", default-features = false }
[features]
fast = ["openwurli-dsp/legacy-power-amp"]
heavy = ["openwurli-dsp/melange-preamp"]
```

Run each feature separately, passing the absolute output directory:

```sh
cargo run --release --features fast -- /absolute/fixture/output
cargo run --release --features heavy -- /absolute/fixture/output
```

Do not point the generator at the adapted vendored source: the independence of
the references is what detects mistakes in the runtime integration.
