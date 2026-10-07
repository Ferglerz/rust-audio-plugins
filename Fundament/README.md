# Fundament

Polyphonic fundamental tracking with partial cuts and additive resynthesis (CLAP/VST3, nih-plug).

Fundament analyses the input, picks the most prominent fundamentals and their harmonics, and tracks them as stable voices. Each voice fades in bell cuts at its partials and blends in phase-continuous synthetic partials. The aim is a cleaner, more controllable source for bass and guitar rather than a 1:1 reconstruction.

## Signal flow

1. `dsp::stft` – mono-sum Hann STFT (4096 at 48 kHz, scaled with sample rate, hop N/16).
2. `dsp::peaks` – local maxima with phase-vocoder / parabolic frequency refinement.
3. `dsp::salience` – iterative harmonic-sum F0 estimation with cancellation.
4. `dsp::tracker` – candidate-to-voice matching, birth hysteresis, attack/release presence.
5. `dsp::cut_bank` – smoothed per-partial bell cuts (`pleasant-dsp` biquads).
6. `dsp::synth` – additive oscillator bank blending measured partials with a waveform spectrum.
7. `dsp::delay` / `dsp::engine` – dry path delayed by half the analysis window, reported as plugin latency.

`src/dsp` has no host dependencies. `params.rs` converts host parameters to `EngineSettings`; `telemetry.rs` publishes voices and a display spectrum to the editor in `src/ui`.

## Build and install

From the audio-plugins root:

```sh
cargo check -p fundament
cargo test -p fundament --quiet
Fundament/scripts/install.sh
```
