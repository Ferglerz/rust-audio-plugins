# Clavinet D6 three-key reference

The Electric Pianos plugin offers the 60-key Clavinet D6 under **Hohner**. The [live-engine report](research/live/README.md) gives the five held-out hardware notes, CPU measurements and differences from the offline grid. The offline Rust reference instrument renders B♭2 (MIDI 46, wound), E4 (64, plain) and F♯5 (78, plain). These pitches have identifiable hardware strikes in the Aalto recordings. One string simulation drives two pickups simultaneously. The probe exposes the two pickup EMFs, all four selector positions and a loaded output jack through the transformer, transistor preamp and four tone tabs. The [circuit report](research/electronics/README.md) identifies the board revision and the [D6 plan](../docs/electromechanical-pianos/tangent-strings.md) covers mute-bar measurements and remaining hardware calibration.

The [Clavinet E7 report](research/e7/README.md) covers its separate electronics, shared action and comparison against an E7 owner’s chromatic run.

## Render and listen

The generated `action.html` includes a hardware comparison player for the three attacks, four pickup positions and three bass key releases. Each pair plays the recording first and the model second. The separate string-action controls let you solo the centre or bridge pickup and scrub the same key's release and restrike.

From the workspace root, run the following command for a 750 ms hold at velocity 80.

```sh
cargo run -p clavinet-d6 -- target/d6-probe
```

Each MIDI-number directory contains 44.1 kHz float WAVs. The two raw mono files, `centre.wav` and `bridge.wav`, use the same gain and sample origin. In `pickups.wav` those same samples appear on the left and right respectively. `CA.wav` and `CB.wav` are the corresponding loaded mono solos. `DA.wav` and `DB.wav` are the combined positions. The four `jack-*.wav` files give volts at a 1 MΩ DI. The complete circuit runs at 352.8 kHz before all signals are low-pass filtered and downsampled together. Every render includes 50 ms before the first strike and 500 ms after the final key-up. `trace.csv` samples unloaded tip position, string displacement at the anvil and both pickups, contact forces, net work delivered to the string, cumulative viscous loss and instantaneous stored string energy every 11 output frames. Forces are snapshots at that cadence.

The remaining positional arguments are MIDI velocity, hold time in seconds, pickup wiring and tone-tab mask. The mask is the sum of Soft (1), Medium (2), Treble (4) and Brilliant (8). The default is 15. Zero silences the jack while the pickup stems continue. In the default `da-adds` configuration DA adds the two terminals and DB subtracts them. The `da-subtracts` setting exchanges the paired positions. One service shop documents the first assignment and the DAFx investigation documents the second. The latter better fits the AMAZONA pair in this probe's terminal convention. [Chicago Electric Piano](https://chicagoelectricpiano.com/blogs/news/clavinet-controls-and-settings) · [Remaggi et al., Table 1](https://www.dafx12.york.ac.uk/papers/dafx12_submission_62.pdf)

```sh
cargo run -p clavinet-d6 -- target/d6-probe/reference-wiring 80 0.75 da-subtracts
```

## What the recordings show

### Attack

The rise time runs from −20 to −3 dB in a 1 ms RMS window. The two model measurements use the centre pickup from the same strike. The jack has all four tone tabs on. The hardware singles have undocumented pickup, tone, velocity and gain settings. The last column compares the first 25 ms of energy above 2 kHz to the full band, in dB. [Aalto hardware singles and bridge riff](https://research.spa.aalto.fi/publications/papers/icmc11-clavinet/)

| Key | Recorded rise | Selector rise | Jack rise | High-frequency energy, recorded / jack |
| --- | ---: | ---: | ---: | ---: |
| B♭2 | 1.47 ms | 0.45 ms | 0.75 ms | −15.8 / −9.6 dB |
| E4 | 0.77–0.79 ms | 0.50 ms | 0.82 ms | −12.4 to −12.2 / −11.1 dB |
| F♯5 | 0.73–0.84 ms | 0.43 ms | 0.82 ms | −16.1 to −13.5 / −11.6 dB |

The middle and upper jack rises are within two 44.1 kHz samples of the recorded ranges. The bass still arrives about 0.7 ms early and its attack has 6.2 dB more high-frequency energy than the recording. The held model fundamentals are 116.38, 330.16 and 738.34 Hz. Tension was fitted on the first strikes. The two additional recorded strikes measured 330.15 and 737.76 Hz. The single B♭2 fifth harmonic is 18.0 dB below its second on the recording and 14.6 dB below at the model selector.

Strike velocity matters most to the bass timing in this comparison. At MIDI velocities 32, 80 and 120 the modeled B♭2 jack rises in 1.25, 0.75 and 0.59 ms. The softer strike brings the model within 0.22 ms of the recording. Across the same settings E4 measures 0.91, 0.82 and 0.63 ms, while F♯5 measures 0.66, 0.82 and 0.73 ms. The hard strikes add 6.0, 7.6 and 13.0 dB respectively to the first 25 ms relative to the soft strikes. The recording does not identify the player's strike velocity.

### Pickup switching

For each labeled AMAZONA take, two 90 ms B♭2 windows contain 95–100% of their 70–1250 Hz power within 20 Hz of a B♭2 harmonic. Each frame's first ten partials are normalized to its strongest partial. The table gives the median absolute error across those ten relative partial levels after taking the median of the two hardware windows. This compares spectral shape across performances. Absolute balance and terminal polarity require simultaneous calibrated stems. [AMAZONA D6 pickup recordings](https://www.amazona.de/vintage-analog-hohner-clavinet-d6-1971/)

| Position | Selector, DA adds | Selector, DA subtracts | Jack, DA adds | Jack, DA subtracts |
| --- | ---: | ---: | ---: | ---: |
| CA, centre | 3.3 dB | 3.3 dB | 3.2 dB | 3.2 dB |
| CB, bridge | 5.7 dB | 5.7 dB | 5.3 dB | 5.3 dB |
| DA, pair | 9.2 dB | 3.4 dB | 7.9 dB | 3.3 dB |
| DB, pair | 8.8 dB | 4.6 dB | 10.5 dB | 1.2 dB |

Switching from centre to bridge raises harmonic seven relative to harmonic two by 11.7 dB in the dry performances and 9.8 dB at the modeled selector. On an independent B-pickup riff the recorded seventh-to-second ratio is −12.7 dB, versus +3.8 dB at the modeled bridge. The two hardware sources support different bridge spectra.

A separate switching test changes a running panel from CA → CB → DA → DB at 240, 430 and 620 ms into each file, while holding the same string. In the first 10 ms after a switch, the difference from a jack held at its destination setting is 7.5–20.4 dB below that destination. During the 63–98 ms window it is at least 116.4 dB below. This verifies that the preamp carries its previous capacitor and coil states through the switch. The hardware recordings identify steady positions. They do not supply a timestamped switch on the same ringing string. Each key folder includes `jack-selector-sweep.wav`; `jack-live-switches.wav` plays the three keys in sequence.

### Key release

On three short B♭2 notes from the AMAZONA centre-pickup take, the level fell 20 dB in **31, 55 and 58 ms** from estimated key-up. After a 190 ms hold, the prototype takes **46 ms at the selector and 25 ms at the jack**. The jack falls below the recorded range. The reference release markers at 5.885, 7.092 and 8.271 s come from the departure of the held partials and carry roughly 10 ms of timing uncertainty. The estimator uses 10 ms RMS and requires the crossing to persist for 6 ms. The reference level is measured 55–15 ms before the marker.

The strongest local bass peak 34–84 ms after key-up is 96.2, 95.0 and 89.7 Hz in the three takes, against 100.0 Hz in the prototype. These short performance tails include other sound and give a less certain pitch estimate than the held tones. Across B♭2, E4 and F♯5 the model drops **2.63, 2.95 and 2.99 semitones**. The published D6 measurement reports about three semitones when the afterlength rejoins the string. [Gabrielli et al., Fig. 5](https://link.springer.com/article/10.1186/1687-6180-2013-103)

The Aalto long holds make a different comparison possible. At the end of the note a brief key-up burst emerges above the background. The table measures the time from its peak to a sustained 10 dB drop, using the same 10 ms RMS window in both signals. The prototype uses the corresponding hardware hold duration. At F♯5 the hardware burst disappears within one analysis window, so the measured 2 ms is window-dependent. The 22 ms jack burst is distinctly longer. The hardware recording also contains non-harmonic energy in this transient, so this one observation does not isolate yarn drag. E4's 15.8 dB burst-to-background margin makes a 20 dB hardware estimate unreliable.

| Key | Hardware hold | Hardware burst to −10 dB | Selector burst | Jack burst |
| --- | ---: | ---: | ---: | ---: |
| B♭2 | 14.607 s | 36 ms | 25 ms | 12 ms |
| E4 | 5.303 s | 11 ms | 7 ms | 7 ms |
| F♯5 | 5.679 s | 2 ms | 23 ms | 22 ms |

## Installed instrument

Both installed plugin formats rendered the four pickup positions at 44.1, 48 and 96 kHz. A VST3 host verified silence outside F1–E6, damped audio with the mute bar engaged, silence at the jack with all tone tabs off and the same key-up with CC64 held. An older Electric Grand preset loaded over a Hohner selection recalled the CP-80 in the CLAP host.

Repeating the three-key fixture at the installed VST3 jack gives B♭2, E4 and F♯5 attack rises of 0.75, 0.79 and 0.82 ms at 44.1 kHz. After the 190 ms hold their 20 dB release times are 25, 26 and 41 ms. The listening clips above use the offline solver. The plugin uses the [reduced live grid](research/live/README.md).

## Physical parameters and assumptions

The double-precision solver advances a stiff string continuously from tailpiece to tuning pin at 352.8 kHz. Endpoints are pinned with zero bending moment. The plunger underneath the key pushes the string down. A unilateral rubber spring and rigid anvil limit travel. On release the tip retreats, exposing the already-moving afterlength to the speaking part. Viscous yarn loss acts on the afterlength throughout play. Both displacement histories survive note-off and same-key restrikes. The acoustic source is entirely calculated from the action. [Zacharias, action patent](https://patentimages.storage.googleapis.com/03/ff/9d/616c0f2fd28202/US3447410.pdf) · [Bilbao and Rath, time-domain Clavinet simulation](https://aes.org/publications/elibrary/elibrary-page/?id=15311)

| Key | Speaking / afterlength, m | Core + winding mass, g/m | Centre / bridge from tailpiece, mm | Resting string to anvil, mm | Stiffness coefficient B |
| --- | ---: | ---: | ---: | ---: | ---: |
| B♭2 | 0.800 / 0.1500 | 1.40 + 1.90 | 160 / 42 | 1.8 | 0.00015 |
| E4 | 0.440 / 0.0825 | 0.89 + 0 | 113 / 40 | 1.8 | 0.00010 |
| F♯5 | 0.174 / 0.0326 | 0.55 + 0 | 84 / 38 | 2.4 | 0.00009 |

These dimensions, gauges, gaps and loads are working estimates for the public recordings. Afterlength starts from the published release interval and rounds to the nearest grid cell. Tension is derived from speaking length, linear density, stiffness and held pitch. Tip stiffness is 12 kN/m, initial clearance below the string is 0.7 mm, overtravel is 1 mm and return speed is 0.6 m/s. Strike speed rises from 1 to 4 m/s across MIDI velocities 1–127. Each register has its own frequency-independent loss. A spatial damping term suppresses shorter wavelengths, and yarn damping rises to 1,800/s over the first sixteenth of the afterlength. [Gabrielli et al., §§2–3 and Table 1](https://link.springer.com/article/10.1186/1687-6180-2013-103)

Each pickup averages exponential flux over a 5 mm aperture and differentiates the result. The assumed field length is 3.5 mm. Positive displacement points toward the anvil. Downward travel closes the gap to the lower centre pickup and opens the gap to the upper bridge pickup, following Gabrielli et al. §2.1. The earlier DAFx text reverses the vertical descriptions. Winding references give the same small-motion voltage sign at each pickup; the selector can reverse one electrically. The simple selector probe uses equal source resistances into ten times either resistance. The jack instead uses the electrically loaded transformer and complete tone circuit. Both combined positions account for the two parallel sources. WAV levels are relative, with one fixed gain across keys, stems and selectors. A common 71 µV per model pickup unit places the medium E4 near the 100 mV jack marking (100 ms RMS, 150 ms into the file). This scale is provisional. Pickup height, string travel and wiring on the recorded units need bench measurements. [D6 pickup measurements and aperture](https://www.dafx12.york.ac.uk/papers/dafx12_submission_62.pdf)

## Reproduce the comparison

The script uses Python 3, NumPy, SciPy and SoundFile with MP3 decoding. From the repository root, download the authors' two hardware files and the four original selector takes. The script checks their SHA-256 hashes before measuring. The reference files and listening excerpts stay in the local Cargo output directory.

```sh
mkdir -p target/d6-reference
aalto=https://research.spa.aalto.fi/publications/papers/icmc11-clavinet/sound
for name in D.wav samples_wholekeyboard.mp3; do
    curl -fL "$aalto/$name" -o "target/d6-reference/$name"
done
amazona=https://www.amazona.de/wp-content/uploads/2028/01
for setting in CA CB DA-4 DB; do
    name="Hohner-Clavinet-D6_Pickup_$setting.mp3"
    curl -fL "$amazona/$name" -o "target/d6-reference/$name"
done
cargo run -p clavinet-d6 -- --reference-session target/d6-probe
python3 clavinet-d6/research/compare.py target/d6-reference target/d6-probe
```

The comparison checks every sample in the four selector WAVs against the two simultaneous stems, prints all measurements and writes listening files and a self-contained `action.html` diagram into `target/d6-probe`. Open the diagram locally to select a key, switch the pickups, or scrub through an early key-up and restrike. It displays the simulated string grid and plays either raw pickup or any of the four loaded jack outputs. Both strikes use the same moving string. In the `restrike` fixture, key-down velocities are 80 and 109 with an 18 ms gap between release and restrike. In `attacks-hardware-then-model.wav`, each key plays on hardware first and then on the model, gain-matched over the first 100 ms. `releases-hardware-then-model.wav` pairs the three short hardware B♭2 releases with the model, matched to the pre-release level. `pickup-switches.wav` changes CA → CB → DA → DB during one sustained strike per key with the default wiring.

`jack-attacks-hardware-then-model.wav` alternates each recorded Aalto attack with the loaded jack. One gain adjustment at B♭2 applies to all three comparisons. `jack-switches-hardware-then-model.wav` alternates excerpts from the four dry AMAZONA settings with their modeled jack signals (DA subtracts), using one gain adjustment at CA. The fixed-gain `jack-releases-hardware-then-model.wav` pairs three short B♭2 key-ups with the modeled jack. The reference recordings have no absolute level calibration.

The crate tests exercise unilateral contact, state continuity on release and restrike, energy loss, strike dynamics, both pickup wirings and grid refinement. The refinement doubles spatial resolution and quadruples the internal rate. Run `cargo test -p clavinet-d6 --quiet`.
