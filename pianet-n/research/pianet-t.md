# Hohner Pianet T

The T covers F1–E6 (MIDI 29–88). Pressing a key lifts a soft suction pad from a weakly magnetized reed. The reed rings freely until the returning pad damps it. Each pole shoe feeds a coil. Alternate coil terminals are reversed along the series bank. The transformer secondary drives the jack with a 2 nF capacitor in parallel. Hohner specifies amp inputs between 10 and 100 kΩ and describes an external volume pedal. The amplifier sensitivity of 200–300 mV RMS gives a receiver specification. It does not specify how loud any individual key should be. [Hohner manual, pp. 2–4, service drawing](https://www.clavinet.com/pianet_t_manual.pdf)

```mermaid
flowchart LR
    K[Key and lever] --> C[Compressible suction pad]
    C <-->|Contact, trapped air and lip release| R[Clamped steel reed]
    R --> M[Magnetic pole shoe and coil]
    M --> S[60 coils in series]
    S --> T[Output transformer]
    T --> J[Jack with 2 nF in parallel]
    J --> A[10–100 kΩ amplifier input]
```

## Model

The same cantilever mode solver used by the N accepts a contact force at 81% of the T reed’s free length. The T pad initially compresses the reed. As the cup expands above it, the pressure falls according to its trapped air volume. The force acts over the seated area. Each tick solves pressure, reed compliance and volume together. Air leaks toward ambient with an estimated 35 ms time constant. The lip vents when its stretch reaches the seating-dependent release height. A returning pad compresses the reed and rebuilds its seal over approximately 12 ms. A very brief key return fails to establish contact. A partially reseated return gives a weaker subsequent pull. Neither note-off nor an immediate repeat resets moving modes.

The magnetic model integrates an estimated local field gradient over blade travel, then differentiates the resulting flux linkage to get coil voltage. An exponential approximation to the local sensitivity allows the strong bass second harmonic and the much more nearly linear treble pickup response to be fitted independently. Measured P-layer pitch, decay and spectral shape at MIDI 29, 37, 45, 52, 61, 69, 77 and 88 provide anchors. Other sections interpolate. Physical reed lengths, thicknesses, pole gaps and coil polarities relative to upward reed motion still require case measurements. [Sullivan’s recording notes](https://www.sullivang.net/hohner-pianet-t-electric-piano-instrument-for-gigasamplergigastudio)

Each of the 60 coils contributes an estimated 18 Ω and 1.1 mH to the series impedance, including idle keys. The estimated transformer steps up fourfold with 5 H primary magnetizing inductance. The RLC solver processes the summed induced voltages once. At 440 Hz the 10, 100 and 1,000 kΩ input loads give predicted gains of 3.3, 10.7 and 11.9 dB. At 5 kHz the gains are −1.2, 6.9 and 7.6 dB. The independent continuous impedance-divider calculation agrees within 0.01 dB at seven frequencies through 10 kHz. Coil and transformer sweeps would establish the actual transfer under each load.

## Dry hardware comparison

Greg Sullivan recorded his own T directly from its jack through a high-impedance solid-state preamp. He gives two possible serial readings, 885041 or 865041, two nominal strike strengths and a single set of key releases. The SFZ conversion provides native mono 44.1 kHz FLAC files. Comparisons use the signed PCM directly. The P files peak at approximately −3 dBFS individually, which removes absolute key balance. The conversion names also omit the original GIG’s toothpick-pluck suffixes. The acoustic comparisons therefore cover freely ringing reeds and separately recorded releases. Mechanical attack attribution needs the original labels and matched pad-travel recordings. [Owner’s method and serial](https://www.sullivang.net/hohner-pianet-t-electric-piano-instrument-for-gigasamplergigastudio) · [Pinned SFZ conversion by kinwie, CC BY 3.0](https://github.com/sfzinstruments/GregSullivan.E-Pianos/tree/8c3e581acda3594b553948ff0222d4f84a698376/Pianet%20T)

The analysis measures pitch in a 500 ms held window, fundamental decay over 0.2–1.1 s and harmonic levels during 0.12–0.32 s. P and FF model strikes use MIDI velocities 40 and 112. All FF takes are independent of the fitted layer. Values below are median absolute residuals.

| Layer and keys | Pitches | Pitch | Fundamental decay | H2 relative to H1 | H3 relative to H1 |
| --- | ---: | ---: | ---: | ---: | ---: |
| P, anchors | 8 | 0.02 cent | 0.39 dB/s | 0.12 dB | 2.63 dB |
| P, reserved pitches | 8 | 1.69 cents | 1.50 dB/s | 2.51 dB | 5.28 dB |
| FF, anchor pitches | 7 | 1.11 cents | 1.12 dB/s | 6.60 dB | 11.43 dB |
| FF, reserved pitches | 10 | 1.66 cents | 2.44 dB/s | 4.56 dB | 8.76 dB |

Measured from the release burst peak to a sustained 10 dB drop, A2 takes **31.4 ms on hardware and 8.5 ms in the model**, F4 takes 10.5/4.0 ms and A5 takes 4.5/3.5 ms. The files contain individually trimmed release events without matching preceding key-down times. Several different keys show energy around 440–480 Hz in these releases. A simultaneous reed displacement and jack recording would help distinguish case motion and returning-pad contact. The bass tail and the third harmonic at high velocity are the clearest discrepancies.

## Reproduce

```sh
cargo run -p pianet-n --example pianet_t -- target/pianet-t-proof
git clone https://github.com/sfzinstruments/GregSullivan.E-Pianos target/sullivan-pianos
git -C target/sullivan-pianos checkout 8c3e581acda3594b553948ff0222d4f84a698376
python3 pianet-n/research/compare_t.py 'target/sullivan-pianos/Pianet T/Samples' target/pianet-t-proof
cargo run --release -p pianet-n --example pianet_t -- --bench
```

The comparison uses NumPy and SoundFile and verifies the 49 original file hashes. `pianet-t-audition.wav` plays A2, F4 and A5 at two velocities. `suction-repeats.wav` repeats the same chord after 3 and 20 ms of key return. Its CSV reports pressure, contact force, pad height and signed open-circuit flux at each reed. Both audio files use common gain. No recorded sample is used to synthesize notes.
