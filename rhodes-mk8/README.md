# Rhodes MK8 S and MK8 FX

The 73-key model has a dry pickup send and the standard stereo preamp. Rhodes specifies a spruce Kluge keybed, ABS hammer shanks and combs, four tip zones, natural wool dampers, custom steel tines and tonebars, and flat-ended Alnico pickups. The hardware compass is E1–E7, MIDI 28–100. Its factory numbers are E8–E80. The optional MK8 FX board adds a VCA compressor, stereo bucket-brigade chorus, four-stage phaser and stereo bucket-brigade delay. The host stores the selected board and its control settings. [Factory specifications](https://rhodesmusic.com/rhodes-mk8-specifications/) · [owner manual, pp. 3–9, 24](https://rhodesmusic.com/wp-content/uploads/2023/07/MK8-User-Guide.pdf)

## What determines the sound

**The harp has its own mechanical solution.** Twelve tapered Euler–Bernoulli beam elements give a mass and stiffness matrix for each steel tine. A local tuning spring adds a point-mass term, including off-diagonal coupling between coordinates. The four lowest modes provide natural frequency ratios and signed projections at the hammer and the moving tip. A near-unison tonebar adds a second low mode, and slight lateral excitation drives a second polarization. The hammer tip compresses with a 3/2-power contact force and releases at separation. A player can restrike a vibrating note with its prior momentum. The wool damper adds loss as it returns. [Beam solver and all geometry assumptions](research/beam.py) · [contact and motion](src/action.rs)

Rhodes' current replacement blank is 170 mm long before cutting. Its production cutting chart ships in the kit. The compatible Vintage Vibe replacement chart in the Stage 73 calibration supplies free lengths here. This is an estimated production scale. The MK8 and Stage 73 share that length source and the Rust physical DSP primitives. Stage 73's fitted oscillation amplitudes, chamfered field, per-key profiles and passive circuits are independent of the MK8 model. The manufacturer describes comparison instruments spanning early Mark I through Mark 7 during development. [Rhodes blank and fit](https://rhodesmusic.com/product/tines-full-length/) · [Rhodes design comparisons](https://rhodesmusic.com/faq/) · [Stage 73 source table](../stage73/src/calibration.rs)

Taper length, steel diameter, per-key spring masses, tonebar coupling, strike line, hammer velocity mapping, loss constants and the exact four production tip transitions are estimates. The model uses transitions at MIDI 46, 62 and 81. The official retrofit installation sheet describes five tip groups for vintage harps with two hard-red transition notes. Establishing MK8 production boundaries still requires an identified harp or parts list. [Factory four-zone specification](https://rhodesmusic.com/rhodes-mk8-specifications/) · [retrofit installation sheet, p. 3](https://rhodesmusic.com/wp-content/uploads/2024/02/Rhodes-Hammer-Tip-Kit-Installation-Procedure-v.2.pdf)

**The pickup integrates the face of a flat pole.** The first preparation integrates 576 surface elements over gap and radial position. The changing squared field at the moving steel tip yields flux linkage, and its time derivative gives the induced voltage. The axisymmetric 3 mm radius, magnet strength and coil bank equivalent are estimates. Rhodes' product photograph shows a flattened lower contour on the spare head. Its manual's voicing sketch illustrates three vertical offsets. Centering the tine emphasizes the second harmonic. Moving away from that symmetry brings back the fundamental. The manual also reports bass pitch pull and physical contact at extreme regulation. Magnetic back-reaction and contact with the pole still require measured force and clearance. [Maker's pickup photo and description](https://rhodesmusic.com/product/individual-pickup-white/) · [manual, p. 24](https://rhodesmusic.com/wp-content/uploads/2023/07/MK8-User-Guide.pdf) · [field integral](src/field.rs)

The global linkage constant sets C4 at velocity 80 to 38.1 mV RMS over its first 100 ms at the dry send. The setting is provisional because none of the recording levels is voltage-calibrated. At velocities 48/80/112 the contact solver gives C2 peak tine travel of 2.93/6.01/9.65 mm, C4 travel of 1.13/2.33/3.74 mm and C6 travel of 0.37/0.80/1.32 mm. Those excursions are model observables.

## Standard analogue board

The manual places a velocity-sensitive diode drive ahead of the EQ and the master volume after the stereo processing. Its low control centers at 75 Hz, its swept mid spans 100 Hz–2 kHz, and its high band centers at 3 kHz. Each reaches ±15 dB. An envelope follower moves the voltage-controlled mid frequency. The alternative resonance adjustment changes its bandwidth. Vari-Pan has an enable switch, four waveforms, depth and a published 0.1 Hz–3 kHz range. The send carries the pickups independently, and the switched return can accept an external source. [Manual, pp. 3, 5–9, 17](https://rhodesmusic.com/wp-content/uploads/2023/07/MK8-User-Guide.pdf)

A component-level production schematic and repeatable production return sweep are still outstanding. The model uses an equivalent 1.5 kΩ/0.65 H winding into 470 pF and 1 MΩ, a symmetric diode pair and three peaking sections. The assumed 4.7 kΩ source, 2 nA diode saturation current, 44 mV effective thermal voltage, control tapers, follower time constants and EQ widths are published in the source. The final model assumes ±6.5 V working swing from the externally supplied 15 V. The mono and stereo line model treats the connector load as high impedance. Individual XLR drivers and the separate headphone amplifier need measurements. The special expression-pedal slow-phase mode is also outside the verified transfer. [Panel circuit and assumptions](src/panel.rs)

At the return, the three modeled band centers meet their marked gains within 0.02 dB. The diode transfer agrees with an independently solved equivalent ngspice circuit within 0.015 mV at eight input levels. These checks validate the implementation of the proposed network. The maker's boosted, cut and flat EQ performances start with different notes, so their spectra cannot yield a matched electrical transfer. [Diode circuit comparison](research/diodes.py) · [factory prototype recordings](https://soundcloud.com/user-509797554-985358061/sets/rhodes-mk8-prototype-sound)

## Factory FX option

The optional board has independent effects switches, tap tempo with delay spillover and the factory alternative adjustments. Two production instruments supplied stereo chorus references. Three phaser settings and two delay times supplied separate checks of the moving notches and repeat spacing. The reported delay times agree within 0.17 ms at these settings. Compressor timing and absolute return gain are estimates. [Controls, waveform comparisons, limitations and CPU](research/fx.md)

## Recorded comparisons

Rhodes labels its 28.7-second prototype take as direct audio with the EQ flat. The published material does not identify it as an isolated send tap. Its first exposed C2 supplied the bass voicing datum. H2 through H7 relative to H1 are +4.5, +2.6, −15.6, −4.9, −14.5 and −18.6 dB. With an inferred strike velocity of 68 the model yields +1.5, +4.4, −14.9, −5.3, −14.9 and −18.9 dB, a 1.48 dB RMS residual. The later C2 at 18.31 s has a 4.01 dB RMS residual with velocity 72. Absolute recording gain, input impedance and impact speeds are unknown. [Rhodes flat EQ take](https://soundcloud.com/user-509797554-985358061/mk8-flat-eq) · [measurement script](research/recordings.py)

Bonners' 2024 production instrument demonstration with Rhodes' Jamie Bullock provides an independent playing session. Seven 70 ms windows in the preamp/EQ chapter have a clear fundamental. The nearest modeled H2/H1 selects the unknown impact velocity on a fixed grid. H3/H1 is the independent comparison. The median absolute error is 4.2 dB, with individual errors of 1.8–6.9 dB. The demonstration's output settings, performance timing and possible room or production tail limit how much of a discrepancy can be assigned to a specific component. [Bonners production demonstration](https://www.youtube.com/watch?v=3o7f3QRSqIg) · [timestamps and procedure](research/recordings.py)

| MIDI note | Time, seconds | Recorded / modeled H3, dBc |
|---|---:|---:|
| 60 | 553.51 | −8.9 / −10.7 |
| 74 | 563.32 | −21.5 / −27.8 |
| 75 | 563.50 | −15.0 / −12.2 |
| 80 | 565.81 | −15.8 / −20.6 |
| 79 | 566.00 | −21.6 / −28.5 |
| 52 | 574.70 | −2.3 / +1.9 |
| 62 | 581.10 | −22.3 / −19.9 |

On the production D5 release near 566.61 s, the fundamental takes 7.19 ms to fall from −3 to −13 dB relative to the local held level. The modeled release takes 8.65 ms. The fall from −3 to −23 dB takes 88.58 ms in the recording and 15.44 ms in the model. This leaves a substantial difference in the late release. [Band-limited release measurement](research/recordings.py)

Stereo channel-level ratios isolate modulation from the changing performance level. The maker's square test gives 1.678 Hz with H3/H5 at −9.5/−14.3 dBc. The model gives 1.678 Hz and −9.5/−14.0 dBc at the corresponding rate. The ramp take gives 2.869 Hz and H2/H3 at −6.9/−10.6 dBc, versus −6.0/−9.5 dBc in the model. The audio-rate demonstration gives 321.777 Hz versus a modeled 321.808 Hz. Rates and depths were set to the recordings. The square take's asymmetric level range and additional spectral lines remain audible differences. [Maker's modulation takes](https://soundcloud.com/user-509797554-985358061/sets/rhodes-mk8-prototype-sound) · [stereo-ratio measurement](research/pan.py)

## Matched dry strikes against Stage 73

At 48 kHz, the same velocity-80 events produce these differences before the MK8 preamp. Stage 73 uses the representative late Mark I harp output. Both renders include their respective harp loading conventions. Upper attack is the fraction of energy above 1.8 kHz in the first 35 ms. The comparison holds MIDI input fixed. Corresponding hammer speeds and measured pickup voltages from two actual instruments remain unknown. [Matched-strike probe](research/compare_stage.py)

| Note | H2, Stage 73 / MK8 dBc | H3, Stage 73 / MK8 dBc | Upper attack, Stage 73 / MK8 dB |
|---|---:|---:|---:|
| C2 | −2.3 / +8.5 | +2.3 / +10.4 | −39.8 / −26.6 |
| C4 | −15.8 / −5.1 | −12.3 / −10.7 | −41.4 / −18.9 |
| C6 | −23.4 / −9.8 | −60.0 / −40.1 | −22.8 / −9.3 |

## Host API and verification

`live::Engine::new(rate)` prepares 128 keys and defaults to the 73-key native compass. `set_extended(true)` enables actual pitches MIDI 0–127 using nearest native geometry at the two ends. `set_pickup_gap_mm` applies a signed −0.25 to +2 mm offset around each note's native setback. Moving the pickup rebases the flux to prevent a parameter impulse. `note_on`, `note_off`, `audition`, `audition_off`, `set_sustain`, `choke`, `held_notes`, `sounding` and `reset` share the key ownership conventions of the other struck instruments. `set_controls` accepts the standard preamp controls. `set_fx_controls` selects the hardware option and configures the additional board. `next_outputs` returns aligned dry-send, left-line and right-line volts. `render_stereo` writes stereo line volts. An offline `panel::Panel` also accepts an independent `process_return` signal. The common anti-alias FIR adds 12 host frames. [Live engine](src/live.rs)

Release-build elapsed seconds per one second of audio with 64-frame blocks on the development machine:

| Sample rate | 1 voice | 8 | 16 | 32 | 73 |
|---|---:|---:|---:|---:|---:|
| 44.1 kHz | 0.015 | 0.047 | 0.080 | 0.149 | 0.333 |
| 48 kHz | 0.012 | 0.046 | 0.085 | 0.163 | 0.361 |
| 96 kHz | 0.012 | 0.047 | 0.086 | 0.165 | 0.366 |

```sh
cargo run --release -p rhodes-mk8 --example mk8_probe -- captures target/mk8-probe
cargo run --release -p rhodes-mk8 --example mk8_probe -- panel target/mk8-panel
cargo run --release -p rhodes-mk8 --example mk8_probe -- bench
python3 rhodes-mk8/research/diodes.py
python3 rhodes-mk8/research/recordings.py FLAT.wav BONNERS.wav target/mk8-probe
python3 rhodes-mk8/research/pan.py SQUARE.wav RAMP.wav AUDIO_RATE.wav target/mk8-panel
python3 rhodes-mk8/research/compare_stage.py target/mk8-probe target/mk8-stage-compare
```

The scripts use NumPy, SciPy and SoundFile. The diode circuit check requires ngspice. The scripts analyze locally supplied copies of the referenced recordings.
