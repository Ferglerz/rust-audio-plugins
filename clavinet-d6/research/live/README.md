# Playable D6 string scale

The live engine has one persistent string per key from F1 to E6, MIDI 29–88. The first 23 strings are wound. E♭3 (51) and E3 (52) straddle the wound/plain transition on the instrument examined by Gabrielli et al. The two pickup voltages enter a single D6 preamp after summation across keys. The volume pot, pickup selector, paired wiring and all four tone tabs work during held notes. A separate mute slider applies damping to the speaking string. [D6 instrument examination, §2.1](https://link.springer.com/article/10.1186/1687-6180-2013-103) · [Hohner manual, slide control](https://www.clavinet.com/d6man.php) · [Output-jack model](../electronics/README.md)

The model advances the same stiff-string stencil, unilateral rubber contact, anvil and damped afterlength as the three-key reference. It uses up to 80 speaking-length cells per string. The wound strings advance at half the panel rate, with linear interpolation to the panel. Short treble strings take two steps per panel sample if their stable grid would lose over 10% of the target resolution. All other strings take one step. The 88.2 kHz minimum panel rate keeps the filter ladder’s upper-mid cancellations near their analog frequencies. The host output uses a windowed-sinc decimator with eight host samples of latency.

Physical and host note-off events open the clamp and preserve string state. Repeating an already-held MIDI note lifts the same tip before depressing it again. The keyboard and GUI can independently hold a note. Choke fades over 4 ms. The raw output includes the centre pickup and bridge pickup with the reference’s fixed relative polarity. Both raw and jack taps pass the same decimator. The engine returns jack volts to its caller. Host trim sets playback headroom.

## Scale and calibration

The held pitches, dimensions and pickup positions of B♭2, E4 and F♯5 follow the [three-key reference](../../README.md). Linear interpolation of length, pickup position, steel loss and stiffness connects those anchors. At F1 and E6, the centre-pickup distances of 185 mm and 65 mm follow the published physical inspection. The bridge pickup lies about 40 mm from the tailpiece. Speaking lengths at those endpoints are estimated at 0.950 m and 0.104 m. Other keys use equal temperament and a small tension correction for the clamp. The effective afterlength starts from the reported three-semitone drop and rounds to a grid cell. [Remaggi et al., §2](https://www.dafx12.york.ac.uk/papers/dafx12_submission_62.pdf) · [Gabrielli et al., §§2.3.1–2.3.2](https://link.springer.com/article/10.1186/1687-6180-2013-103)

Wound-register mass is scaled relative to the B♭2 fit using the .032, .028 and .022 inch replacement-string gauges. The wound/plain boundary follows the inspected research instrument. Replacement sets contain different transitions. Core/winding mass split, plain-wire masses and the felt strip’s 16 mm width, location and loss are working estimates. The hard mute test checks for loss at both physical taps. Calibrated mute recordings and measurements on a specific harp would establish its frequency dependence. [Vintage Vibe and Mapes replacement strings](https://www.vintagevibe.com/products/clavinet-strings)

## Reduction against the mechanical reference

The fixture plays a 750 ms hold at velocity 80 on each key. The comparison uses two independent renders, the 352.8 kHz reference and the live engine at 44.1, 48 and 96 kHz. It compares both pickups and the CA jack. Held levels use the first ten partials up to 6 kHz, normalized to the strongest partial. The attack window spans the first 25 ms after MIDI onset. Release time uses a 10 ms RMS window with a sustained 20 dB drop relative to the pre-release level.

| Largest difference over both pickups, jack and all three rates | B♭2 | E4 | F♯5 |
| --- | ---: | ---: | ---: |
| Held fundamental, cents | 0.09 | 0.05 | 0.04 |
| Median relative partial-level error, dB | 0.11 | 0.37 | 0.83 |
| Attack energy, dB | 0.18 | 0.05 | 0.07 |
| Release to −20 dB, ms | 1 | 2 | 2 |

The fresh live checks at the wound/plain boundary put the E♭3 and E3 fundamentals within 0.45 cents of their nominal tuning. The centre pickup released to −20 dB in 20 and 19 ms respectively. F1 and E6 are also rendered at all three rates.

## Hardware notes excluded from fitting

The comparison includes one long B♭1 in Aalto’s hardware singles and four notes in their hardware B-pickup riff. For each, a 90 ms hardware window is compared with a 90 ms live-engine window from the same pitch, 90 ms into its hold. Pitch is estimated from the relative spacing of the first six strong partials. The last column compares the ten strongest-normalized harmonic levels at the simulated jack. The B-pickup riff uses the bridge solo. The B♭1 comparison assumes the centre pickup. Tone settings, velocity and DI calibration are unknown. [Authors’ hardware singles and B-pickup riff](https://research.spa.aalto.fi/publications/papers/icmc11-clavinet/)

| Note | Reference and start | Hardware / live pitch, Hz | Median relative partial-level error |
| --- | --- | ---: | ---: |
| B♭1 | Singles, 0.48 s | 58.37 / 58.37 | 1.3 dB |
| C2 | Riff, 0.24 s | 65.05 / 65.54 | 3.8 dB |
| E♭2 | Riff, 1.99 s | 77.48 / 77.93 | 5.5 dB |
| F2 | Riff, 1.47 s | 86.58 / 87.46 | 9.7 dB |
| G2 | Riff, 1.62 s | 97.69 / 98.16 | 7.8 dB |

The B♭1 partials transfer well from the pilot. The largest residual is the F2 bridge timbre. These dry examples give limited coverage of the upper keys and keyboard-wide regulation.

## Timing and reproduction

The release build timed a one-second hold on an arm64 Mac in 128-frame blocks, including the attack. Each entry is p99 wall time as a percentage of the block deadline. This run missed zero deadlines. A separate run during concurrent build work incurred scheduler delays exceeding the deadline. This benchmark covers the D6 engine in isolation.

| Held strings | 44.1 kHz | 48 kHz | 96 kHz |
| --- | ---: | ---: | ---: |
| 1 | 1.6% | 1.7% | 1.7% |
| 8, spread across registers | 8.2% | 8.9% | 9.0% |
| 16, MIDI 41–56 | 11.5% | 12.5% | 14.2% |
| 32, MIDI 39–70 | 30.3% | 33.7% | 33.5% |
| 60, full compass | 72.7% | 78.9% | 83.3% |

At 48 kHz the eight-string test peaked at 1.24 V, the 32-string test at 1.47 V and the full compass at 2.56 V at velocity 0.65. The first 250 ms of the full compass at maximum velocity peaked at 3.36 V. This is output of the small-signal circuit model. Loud chord distortion still requires a nonlinear board measurement.

```sh
cargo run --release -p clavinet-d6 --example d6_live -- target/d6-live
cargo run --release -p clavinet-d6 -- target/d6-live/offline-control
python3 clavinet-d6/research/live/compare.py target/d6-live target/d6-live/offline-control --recordings target/d6-reference
cargo test -p clavinet-d6 --quiet
```

The Python comparison uses NumPy, SciPy and SoundFile. The `--recordings` option uses the two Aalto hardware files described in the [reference download instructions](../../README.md). Each live three-channel WAV contains centre, bridge and jack. The riff holdouts use CB, the other keys use CA. The short performance cycles through all pickup positions, crosses the winding break and includes a muted restrike.

## Extended notes and pickup distance

Extended Notes adds virtual strings outside the D6’s F1–E6 compass. Each virtual string uses the nearest native key’s geometry, with its linear mass density adjusted for the requested pitch. The two pickup distances can be offset independently by up to 1 mm in either direction. Moving one coil changes its field gradient and its response to large excursions. The other pickup continues to read the same string motion.

At 48 kHz the two extreme virtual strings produced fundamentals of about 8.18 Hz and 12.57 kHz. On E4, centre-pickup RMS was 2.98, 2.17 and 1.61 in raw model units at −1, 0 and +1 mm. Bridge RMS stayed at 0.978 on all three takes. These are simulation checks. The Aalto recordings cover the historical keyboard and give no calibration for the virtual notes or distance offsets.

```sh
cargo run --release -p clavinet-d6 --example d6_extension -- target/extended-pickups/d6
```
