# Stage 73

Stage 73 is the tine model in the Electric Pianos CLAP and VST3 instrument. Its Rhodes era dropdown selects one of six representative builds. It plays E1 through E7 (MIDI 28–100), responds to velocity and sustain (CC64), and offers the era’s original output or a stereo Peterson Suitcase path on the five passive-panel eras. Late Mark I is the default and the sound recalled by sessions saved before the era selector.

| Era | What changes when you play |
| --- | --- |
| Sparkletop, 1965–69 | Felt hammer contact, original pickup wiring and a circa-1967 mono Jordan Suitcase preamp. Its volume, bass, treble, tremolo rate and depth are independent of the Stage controls. |
| Early Mark I, 1970–73 | Square neoprene tips and the early Stage panel. Turning Tone down rolls off treble. |
| Mid Mark I, 1974–75 | Shares the Early Mark I square-tip action. Turning Bass Boost up restores bass through the later passive panel. |
| Late Mark I, 1976–79 | Four strike-strength anchors per key, graduated hammer-tip heights and the later Stage panel. |
| Mark II, 1979–83 | Later plastic-key action at its rear fulcrum position. Hard strikes have less swing and the upper register sustains longer. |
| Mark V, 1984 | A longer hammer stroke gives hard strikes more swing. The damper responds faster. Its passive Tone circuit is provisional. |

You can switch era under a held chord or sustain pedal. The circuitry crossfades over 10 ms. Each key takes on the selected mechanical response at its next strike. The evidence notes identify the instrument chosen for each era and give the circuit measurements and direct-harp comparisons. [Drawings, assumptions and reproducible measurements](research/eras.md)

The plugin matches average playback RMS across the six eras. Each era and output has a fixed gain after its modeled circuit, so velocity, pickup adjustments and tremolo still change the level as you play. The offline probe exposes the original electrical levels for hardware comparisons. [Measured trims and era overlap](research/eras.md#level-matched-playback)

## Playing and voicing

| Control | What you hear |
| --- | --- |
| Panel Volume | On the passive Stage panels the 10 kΩ pot changes the signal at the loaded jack. Fully down mutes it. Sparkletop has a separate 100 kΩ loudness control. |
| Cable Load | Adjusts the shunt capacitance at the passive jack from 100 to 1000 pF. At 300 pF it recalls the original load. The effect depends on the volume pot position. |
| Stage Tone | The early Tone control cuts treble. On the later panel, turning Bass Boost up progressively bypasses the 47 nF capacitor. Both use the original host automation ID. |
| Jordan Bass and Treble | Adjust the active tone network in Sparkletop. Both start at their centre positions. |
| Tremolo | Switch Sparkletop’s mono tremolo on or off. Rate spans approximately 2.33–8.73 Hz. New instances start with tremolo off and depth at 50%. Switching off preserves both settings. |
| Peterson Suitcase | Routes the harp to the 80-watt Suitcase preamp with centered EQ. Speed sets the stereo movement from 1.92 to 7.23 Hz. Intensity sets its depth. At zero, the preamp gives a steady signal to both channels. |
| Tine Height | Moves every tine vertically relative to its pickup by up to ±3 mm. Approaching the pickup centre strengthens the octave relative to the fundamental. Individual keys reach the centre at different settings. |
| Pickup Gap | Changes the end-to-end distance by up to ±1 mm. Bringing the pickup closer generally increases output and changes how the tone responds to a harder strike. |

The editor follows the last struck key. Drag the steel tine up or down to set its height, or drag the copper-wound pickup left or right to change the gap. The diagram follows the selected era’s stock geometry for that key. The voicing knobs show the offsets applied to all 73 keys. The era and knobs support host automation. Double-click a part or knob to restore its default.

A repeated strike transfers more momentum when the tine is moving toward the hammer. All modes receive the same finite contact force. On release, the felt progressively loads the vibrating tine. The sustain pedal lifts the dampers for the whole keyboard.

## Harp interaction and Peterson vibrato

With E2–B2–E3 fingered, pressing sustain opens the rest of the harp to sympathetic motion. The model sends mechanical tine velocity through one lossy support mode. Each undamped receiving tine rings at its own fundamental and contributes through its pickup at the current height and gap. The source’s own contribution is removed because its directly struck motion is already modeled. The 220 Hz support frequency, 140 Hz bandwidth and transfer strength are provisional. The factory describes this held-chord pedal comparison. [Rhodes service manual, chapter 2](https://www.fenderrhodes.com/service-manuals/1979/ch2.html)

Peterson Suitcase takes its signal directly from the harp and crossfades under held notes. A single free-running oscillator drives opposing #19 lamps. The model tracks filament temperature and photocell response separately, then solves the two ORP61 paths and their shared intensity network. The preamp uses a linearized input stage, fixed center tone settings and a fixed output alignment. A SPICE reduction of the 5 µF timing circuit gives a 1.92–7.23 Hz speed range. Lamp and cell response times are estimates. [Peterson schematic, figure 11-8](https://cdn.shopify.com/s/files/1/0666/2821/files/peterson_pre_amp_schematic_-_rebuild_kit.jpg) · [Timing circuit](research/peterson_lfo.cir)

The bench drives the preamp with a steady 1 kHz tone at full intensity. Rise and fall measure 10–90% of each channel’s envelope range. Depth and mono ripple measure the maximum-to-minimum ratio in each channel and in the mono fold-down, respectively.

| Speed | Channel rise | Channel fall | Depth | Mono ripple |
| --- | ---: | ---: | ---: | ---: |
| 1.92 Hz | 17–18 ms | 65 ms | 35.53 dB | 5.12 dB |
| 5 Hz | 18 ms | 59 ms | 22.87 dB | 4.68 dB |
| 7.23 Hz | 18 ms | 44–45 ms | 10.90 dB | 3.14 dB |

Run `ngspice -b stage73/research/peterson_lfo.cir` for the oscillator endpoints and `cargo test -p stage73 --release steady_tone_measures -- --nocapture` for both channel envelopes.

## Late Mark I reference

The Stage drawing shows a 50 kΩ bass control bridged by 47 nF, feeding the top of a 10 kΩ volume control. The jack connects to the volume wiper. The 73-key harp has four bass pickups in parallel followed by 23 groups of three in parallel, with all groups wired in series. The renderer reduces this network to its Thévenin equivalent. A moving bass pickup contributes one quarter of its induced voltage and a pickup in a triplet contributes one third. The independent SPICE deck expands all 73 branches. [Stage control schematic](https://www.fenderrhodes.com/img/service/manual/stgschem.gif) · [Factory harp wiring, figure 10-4](https://www.fenderrhodes.com/service-manuals/1979/ch10.html)

The model uses 180 Ω per pickup, an estimated 75 mH per pickup and a nominal 1 MΩ input with 300 pF of cable capacitance. Pot travel uses cubic audio and reverse-audio curves. These values specify the comparison circuit. Coil inductance, pot position and cable loading were unavailable for the recorded instruments. [Rhodes pickup service notes](https://www.fenderrhodes.com/service/pickups.html)

The factory manual identifies the tine and tonebar as a coupled assembly, specifies the hammer-tip transitions, and illustrates how height and gap change voicing. Key lengths come from the printed scale of Vintage Vibe's cutting chart. The renderer excites damped modes with a finite hammer pulse, sums their physical motion, then computes induced voltage from motion through a chamfered pole's magnetic field. The coupled fundamental, lateral motion and damper contact are approximations. The pickup and circuit run at four times the host rate. The output filter adds 12 samples of reported latency. [Tone generator](https://www.fenderrhodes.com/service-manuals/1979/ch1.html) · [Factory adjustments, figures 4-8 and 4-9](https://www.fenderrhodes.com/service-manuals/1979/ch4.html) · [Tine cutting chart, revision C](https://cdn.shopify.com/s/files/1/0666/2821/files/Tine_Cutting_Chart_RevC16APR18.pdf) · [Shear and Wright, 2011](https://www.mat.ucsb.edu/Publications/Shear_Wright_NIME_2011.pdf) · [Pfeifle, 2017](https://www.dafx17.eca.ed.ac.uk/papers/DAFx17_paper_79.pdf)

Stage 73 and OpenWurli share the oscillator rotation in `pleasant-dsp`, the Electric Pianos editor, host and installation tooling. The hammer, tine, magnetic pickup and passive network live in this crate.

## Late Mark I recording comparisons

The panel magnitude response agreed with ngspice to **0.089 dB** over 40 Hz–10 kHz, across 27 combinations of host rate, volume and bass setting. This measures the numerical implementation against the component model.

Published fits provide four tine-swing estimates per key. E2 and E6 anchor a shared loaded-jack calibration at soft and hard dynamics. Height, gap and swing are estimated jointly for the missing keys using a local register trend. Leave-one-key-out prediction on the training keys selected a six-semitone bandwidth. G3 and B♭4 were excluded from the per-key voicing fit. The dry archive labels pitches one octave below scientific pitch. [Published fits and dry excerpts](https://github.com/jstolberg/rhodes/tree/dedf7156f159c85e1c43802cb0764497d0f361fb) · [Recording notes](https://www.pianobook.co.uk/packs/matts-fender-rhodes/)

The Late Mark I fit uses the soft and hard E2 and E6 jack recordings to anchor octave balance and fundamental level. The outer layers on eight alternate pitches from Learman’s 1977 harp determine the trend between those anchors. The fit adjusts each key’s resting height and pickup gap by up to 0.7 mm, and tilts its four swing estimates by up to 13% while preserving their order. The objective constrains octave growth and total level. The other five eras use their own existing setup. [Voicing fit and training pitches](research/fit_octave.py)

The table compares the elastic-hammer build before and after the Late Mark I voicing change. Errors are median absolute differences in dB. Both builds use one comparison gain per recording set, derived from the same two E2 strikes. **Octave level** measures the second harmonic directly. **Octave balance** measures it relative to the fundamental, 90–300 ms into each note. **Growth** measures the change in that balance from soft to hard on the same key.

| Recording set | Octave level | Octave balance | Growth | Attack level |
| --- | ---: | ---: | ---: | ---: |
| Matt’s dry jack, 6 strikes, 3 pairs including the 2 fitted keys | 4.76 → 3.07 | 6.63 → 2.31 | 2.78 → 2.12 | 1.46 → 1.07 |
| Learman’s 1977 harp, 35 strikes on 15 keys, 11 pairs | 10.93 → 8.30 | 9.69 → 7.57 | 5.40 → 4.42 | 6.71 → 6.86 |
| Subset on the 7 Learman pitches outside the fit, 16 strikes, 5 pairs | 11.67 → 8.47 | 12.12 → 8.79 | 2.91 → 2.89 | 5.66 → 6.23 |
| Learman’s reserved intermediate layers, 30 additional strikes | 9.54 → 7.73 | 10.97 → 8.04 | — | 5.55 → 4.85 |

On held-out B♭4, the soft octave balance was 9.56 dB too low and is now 1.54 dB too low. On the hard strike it went from 12.34 to 4.35 dB too low. The recording grew 20.42 dB from soft to hard. The previous build grew 17.64 dB and the new build grows 17.61 dB. Fundamental decay-slope errors are 1.72 dB/s on Matt’s recordings and 1.58 dB/s on Learman’s original 35 comparisons. Learman recorded at the harp connector with EQ and intentionally spaced layer levels. His comparisons use the pre-panel harp tap and nominal layer velocities. [Learman’s recording and sampling notes](https://github.com/sfzinstruments/jlearman.jRhodes3d/tree/4508215a0ec6613a2af19d6f4dc7b329e0e7bba0)

Late Mark I interpolates log tine swing with a monotone, smooth curve through the four measured strike strengths on each key. On Learman’s 30 reserved intermediate strikes, median octave-balance error moved from 8.04 to 7.83 dB and absolute octave-level error from 7.73 to 7.76 dB. The measured soft and hard anchors, including the B♭4 comparison above, retain their endpoint levels.

The hammer uses a short elastic contact with force proportional to compression raised to the 3/2 power. The incoming speed is relative to the moving tine. Its effective mass ratio and contact duration are approximations. The upper-mode force weights are estimated from published partial amplitudes on training keys. The research harness searches for a resolved inharmonic peak during the first 180 ms and records its frequency and level. It leaves the measurement undefined when the peak overlaps a harmonic or falls below the detection floor. [Calibration script](research/calibration.py) · [Peak measurement](research/measurements.py)

At G3, centring the tine makes the octave dominant. Opening the gap from −0.75 to +0.75 mm lowers the soft-to-hard peak range from 15.06 to 14.72 dB. Model releases take about 55 ms at E2, 40 ms at G3 and 25 ms at E6 to fall 20 dB. For the loudest E7 at the closest gap, the spectral difference from a higher-rate render is −69, −75 and −80 dB at 44.1, 48 and 96 kHz. Sparkletop release comparisons and Jordan circuit measurements are in the [era evidence](research/eras.md).

The output preserves the summed pickup voltages. At default Late Mark I settings and 48 kHz, simultaneous MIDI-127 strikes on E2, B2, F♯3, D4, F♯4 and A4 peaked at −7.2 dBFS in the installed VST3. Striking all 73 keys together at that velocity reached +9.7 dBFS in floating point.

## Build and reproduce

From the workspace root, run `./stage73/scripts/install.sh` to build and install Electric Pianos in both formats. The plugin and the offline renderer build using the checked-in calibration table.

```sh
cargo check -p stage73
cargo test -p stage73 --release --quiet
cargo build -p stage73 --release --example probe
mkdir -p target/stage73-proof
target/release/examples/probe demo target/stage73-proof/stage73-demo.wav
target/release/examples/probe polyphony
python3 stage73/research/check_panel.py target/release/examples/probe
```

The panel check needs NumPy and ngspice. For the audio comparison, install NumPy, SciPy and SoundFile in a Python environment and point the following command at the two public checkouts linked above. Only development scripts read recordings.

```sh
python3 stage73/research/check_audio.py target/release/examples/probe \
  /path/to/rhodes /path/to/jlearman.jRhodes3d target/stage73-proof
```

`three-note-comparison.wav` plays the recording followed by the model, soft then hard, at E2, G3 and E6. G3 uses a 1977 recording excluded from the voicing fit. `g3-voicing.wav` plays the same strike at stock geometry, centred height, close gap and wide gap. The script writes signed per-strike errors to `audio-errors.csv`, paired soft-to-hard changes to `dynamic-errors.csv`, and aggregate results to `audio-summary.txt`.

To regenerate the per-key table, run `python3 stage73/research/calibration.py /path/to/rhodes /path/to/Tine_Cutting_Chart_RevC16APR18.pdf /path/to/jlearman.jRhodes3d`. The script writes `stage73/src/calibration.rs`.
