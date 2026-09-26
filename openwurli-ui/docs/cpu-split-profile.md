# CPU split profile (release)

Measured on this agent VM (`x86_64`, `cargo test --release --test cpu_split_profile -- --ignored`).
Shipping path: legacy 8-node preamp + behavioral power amp, 2× oversample, 44.1 kHz, 64-sample blocks, 2 s of audio after `warm_up()`.

```
cargo test --manifest-path vendor/openwurli-dsp/Cargo.toml --release \
  --test cpu_split_profile -- --ignored --nocapture
```

## 6-note ff chord (MIDI 48 55 60 63 67 70, v=0.95)

8.5% of realtime.

| Stage | % of wall | % of audio |
|---|---:|---:|
| circuit (tremolo + preamp + amp) | 54.7 | 4.6 |
| reed | 41.3 | 3.5 |
| pickup | 2.0 | 0.2 |
| speaker / mix / OS | <1 each | ~0 |

Isolated replay of the shared stages (same 2 s equivalent):

| Stage | % of audio |
|---|---:|
| tremolo | 2.2 |
| preamp | 1.7 |
| power amp | 0.3 |
| oversampler | ~0 |
| speaker | 0.1 |

## 32-note hold

24.6% of realtime.

| Stage | % of wall | % of audio |
|---|---:|---:|
| reed | 75.6 | 18.6 |
| circuit | 18.7 | 4.6 |
| pickup | 3.6 | 0.9 |

Circuit cost does not grow with voice count (one bus). Reed does.

## What this means

- Intra-reed SIMD was the right first exact win. At 32 notes the reed is still three quarters of the CPU.
- A typical 6-note chord is **circuit-bound**, not reed-bound. Inside the circuit, **tremolo + legacy preamp** beat the behavioral amp (~7× the amp).
- An amp `tanh` LUT is the wrong next optimization. Yield cap is ~0.3% of audio.
- Next fidelity-preserving CPU work, in order: more reed (cross-voice only if a later profile still shows reed first), then tremolo `exp`/`powf`, then BJT `exp(Vbe/Vt)`. Amp LUT last.

Numbers are this VM, not an Apple Silicon host sitting in a DAW with 64 voices. Re-run the probe there before picking a LUT.
