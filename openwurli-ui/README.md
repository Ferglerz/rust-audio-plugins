# Electric Pianos

Electric Pianos is a stereo CLAP and VST3 instrument. The header selects OpenWurli, Stage 73 and Electric Grand directly. The **More pianos** menu groups additional instruments by family. Its Hohner section selects Clavinet D6 and E7, Pianet/Clavinet Duo, Pianet C, L, N and T, Cembalet and Cembalet N, and Electra-Piano. The Electric Grand panel offers CP-80, CP-70B and the original CP-70, with shared EQ, jack volume and tremolo settings. Legacy Grand Model automation retains its CP-80 and CP-70B endpoints. The separate Original CP-70 parameter selects the early revision of the 73-key model. The bottom panel contains master volume, extended note range, appearance and the playable keyboard.

Stage 73 shows **Panel Volume** and **Cable Load** when the passive jack is selected. Electric Grand 80 has its own **Jack Volume**, and its tremolo speed readout shows cycles per second. Both instrument volumes start fully open. The master adjusts the level after the selected instrument’s electronics.

The instrument uses the previous OpenWurli UI host IDs. Existing OpenWurli sessions open on the reed model and use its Fast circuit, including sessions saved with Heavy. Existing standalone Stage 73 and Electric Grand installations have separate host IDs and can still recall their original sessions.

**Extended Notes** makes MIDI 0–127 playable on every model. Notes beyond the factory compass take their pitch from MIDI and use the closest native key for mechanical and pickup regulation. Disabling the switch limits new strikes. Held notes still accept releases.

**Sustain Mod** adds a pad latch to Pianet C, L, N, T and the Pianet side of the Duo. Each switch starts off and is saved with its own model. Turn it on, then hold MIDI CC64 or click **Sustain pedal**. Released pads stay clear of the reeds until the pedal comes up. Key-up noise follows each pad’s physical return. Switching the modification off returns the pads even while the pedal is held. On the Duo, stock CC64 changes registration A/B. Enabling the modification gives CC64 to Pianet sustain and leaves the A/B registration under its panel footswitch. The Clavinet side continues to release on key-up.

**Pickups** opens the selected instrument’s voicing controls. Distance is an offset in millimetres from each native key’s setup. OpenWurli adjusts the reed’s resting clearance from the near comb face in both registers, independently of the existing comb-height adjustment. Stage 73 uses the gap along the tine axis. The CP-70, CP-70B and CP-80 use piezos bonded to the string bearings. Pianet N, C and L expose adhesive pad return from 0–100%. Pianet T and the Duo expose suction cup return on the same scale. These controls affect the returning pad while mechanical damping remains active.

## Hohner actions

**Clavinet D6** covers F1–E6. Click the tangent or keyboard to play, or click a pickup to solo it. The two rows of panel controls select the pickups and four tone tabs. All tabs off silences the jack. The mute bar damps the physical strings. Reverse DA / DB swaps the two paired positions for the alternate wiring. The three-key offline probe includes both raw pickups, output-jack recordings and a time-scrubbable view of the simulated string during the restrike. [D6 model and recordings](../clavinet-d6/README.md)

**Clavinet E7** uses its own tone and output board. The horizontal volume slider controls the output pot, and the power switch mutes the electronics while held strings carry on vibrating. The panel keeps the two solo pickups, both pairs, the four tone tabs and the mechanical mute. [Hohner instrument identification](https://www.clavinet.com/ident.php)

**Electra-Piano** covers F1–E7. Click the hammer to strike the lighter arm of the fork. The copper winding senses its vibration as energy transfers between the arms. The left pedal switches an adjustable attenuation resistance, and the right pedal raises the dampers. They can also be played with MIDI CC67 and CC64. The instrument sends the auxiliary jack to both plugin channels. The seven pickup-group trims and internal basic volume use the bench calibration. [Electra-Piano 680209 circuit and fork measurements](../electra-piano/README.md)

**Cembalet and Cembalet N** span C2–C7. Click the lower plectrum to pluck a key or the separate upper damper to hear it release. The early Cembalet uses an RF pickup circuit and knee swell. The N uses a biased side electrode, transistor preamp and foot swell. Both have a vibrato switch and damp when the key rises. [Hohner service manual, action and electrical diagrams](https://civade.com/AudioPro/Hohner/Hohner_Cembalet_Service_Manual.pdf)

**Pianet C and Pianet L** use adhesive pads and electrostatic reeds. The C panel offers the external amplifier DIN, headphone DIN, a knee swell and vibrato. The L panel offers the external amplifier DIN or the voltage across its two series speakers. Its knee lever switches the power off at zero. The plugin aligns the output levels before its four-millisecond monitor crossfade. The modeling probe preserves the electrical voltages at both terminals. [C and L boards](../pianet-n/src/early.rs)

**Pianet/Clavinet Duo** drives both actions from each of its 60 keys. The pickup-bank split falls between MIDI 48 and 49. The two tabs set the footswitch registrations, and MIDI CC64 selects the foot contact. The sliders can reveal a branch that is already sounding. The normal jack combines both instruments. Separate Recording Outputs sends Clavinet to the left channel and Pianet to the right with the same conversion from volts. [Duo registrations and 20/40 pickup banks](../hohner-duo/src/lib.rs)

**Pianet N** uses adhesive pads and an electrostatic pickup. The panel has swell and optical modulation controls. **Pianet T** uses silicone suction pads and individual magnetic pickups into a passive output. Click the pad to play the displayed reed. Both use native ranges and release through pad contact. The instrument-family parameter defaults to the original Piano selector on old sessions. [Pianet reference and comparison](../pianet-n/README.md)

## Claviset family

The Weltmeister Claviset 200 and Selmer Pianotron use a rotating cam to pluck each reed. The 200 has nine interlocking timbre tabs, three modulation speeds, a power switch and a Bowden-cable sustain pedal. The Pianotron has continuous tone, vibrato depth and vibrato rate controls. MIDI CC11 operates the 200 foot swell or the Pianotron knee swell. CC64 raises the 200 damper rail. [Selmer no. 8701 catalogue](https://www.vintagehofner.co.uk/selmer/misc/pianotron2.jpg)

## Electric uprights

Choose **More pianos → Electric Uprights → Storytone** for the 88-key electric upright. The diagram follows the hammer, strings and pickup plates. The soft pedal shortens hammer travel, the swell pedal raises the manual volume toward the optimum setting, and the damper pedal sustains the strings. MIDI CC67, CC11 and CC64 operate those pedals respectively. The mono output uses the amplifier's auxiliary tap. [Storytone mechanics, amplifier and reference limits](../storytone/README.md)

## Columbia

The Elepian EP-61C has a 61-key C-to-C reed keyboard. The magnetic pickup feeds a passive tone and volume network. The pickup’s vertical adjustment sets distance, and its lateral alignment changes harmonic balance. The damper pedal raises its rail. MIDI CC64 operates the same pedal. [Elepian action and jack circuit](../pianet-n/src/elepian/mod.rs)

## Historical grands

The Neo-Bechstein uses an escapement and a light microhammer to excite string courses under shared magnetic pickup heads. Its left pedal varies an electrical capacitance, and the right pedal lifts the dampers. MIDI CC11 and CC64 operate the swell and damper pedals. The felt rail changes the strike, the twelve-stop master selects amplifier gain, and **Pickups** adjusts head distance. The mono plugin output follows the Siemens amplifier’s 6 Ω speaker terminals. [Neo-Bechstein string and microhammer model](../neo-bechstein/src/action.rs) · [Siemens ELV413 circuit evidence](../neo-bechstein/research/amplifier.md)

## Helpinstill and Baldwin uprights

**Roadmaster 64** has a working soundboard, two strings per note and three magnetic pickup strips. The Mono jack button listens to their passive sum. Soundboard listens to area-weighted bridge-coupled surface motion. Pickups adjusts the bass, middle and treble clearances independently. The Damper pedal and MIDI CC64 raise the dampers. [Roadmaster sources, measurements and remaining gaps](../roadmaster/README.md)

**Baldwin Electro-Piano** uses two preloaded rails with pressure piezos. The Pickups panel moves their positions laterally in millimetres. The soft pedal (CC67) attenuates the electrical signal and the damper pedal (CC64) raises the dampers. [Baldwin action and rail](../baldwin-electropiano/README.md)

## Rhodes MK8

The Rhodes MK8 has a 73-key E1–E7 tine action and a stereo Vari-Pan section. The factory FX option provides compression, chorus, phaser and delay. The standard S board is the recalled default. [MK8 action and electronics](../rhodes-mk8/README.md)

## Vintage Vibe StereoVibe

The active 64 uses the serviced Rev 4.1 StereoVibe board. The input switch selects the Classic, FET or Parallel paths. Stereo tremolo and its half-speed switch operate independently of bass, treble and level. The passive Vintage Vibe 64 remains a separate selection. Both use the A1–C7 action, damper pedal and adjustable magnetic pickup clearance. [Vintage Vibe action and electronics](../vintage-vibe-64/README.md)

## Output calibration

The fixed trims target a −1 dBFS sample peak at the default 50% master volume. The offline sweep measures both output channels at 44.1, 48 and 96 kHz with stock tone settings. It covers full-velocity solo notes across MIDI 0–127, eight-note two-hand chords transposed across each native keyboard, and two overlapping chords with sustain held. Stage 73 applies a fixed RMS adjustment per era and output, followed by the shared Stage peak trim below. Its sweep includes the optional Peterson path at zero intensity, where both optical gains are highest. [Era measurements and listening comparison](../stage73/research/eras.md#level-matched-playback) The Grand sweep covers both Frame Ring settings.

| Piano | Highest performance peak before trim | Applied trim |
| --- | ---: | ---: |
| OpenWurli | +2.21 dBFS | −3.21 dB |
| Stage 73 | +3.66 dBFS | −4.66 dB |
| Electric Grand 80 | −2.27 dBFS | +1.24 dB |
| Electric Grand CP-70B | −1.88 dBFS | +0.88 dB |
| Electric Grand CP-70 | −2.89 dBFS | +1.89 dB |
| Clavinet D6 | +6.18 dBFS | −7.19 dB |
| Clavinet E7 | −2.16 dBFS | +1.15 dB |
| Cembalet | −1.05 dBFS | +0.05 dB |
| Cembalet N | −0.78 dBFS | −0.22 dB |
| Electra-Piano | −26.41 dBFS | +25.40 dB |
| Pianet/Clavinet Duo, common conversion for all jacks | +9.96 dBFS | −10.97 dB |
| Pianet C, external amplifier DIN | −18.76 dBFS | +17.75 dB |
| Pianet C, headphone DIN | −0.61 dBFS | −0.40 dB |
| Pianet L, external amplifier DIN | −22.10 dBFS | +21.09 dB |
| Pianet L, speaker terminals | +10.01 dBFS | −11.02 dB |
| Pianet N | +4.26 dBFS | −5.26 dB |
| Pianet T | −4.14 dBFS | +3.13 dB |
| Claviset 200, nine registers | +5.63 dBFS | −6.63 dB |
| Selmer Pianotron, three tone positions | +12.38 dBFS | −13.39 dB |
| Columbia Elepian EP-61C | −7.40 dBFS | +6.39 dB |
| Neo-Bechstein, 6 Ω speaker | +17.70 dBFS | −18.71 dB |
| Storytone auxiliary | −0.65 dBFS | −0.36 dB |

The Hohner sweep uses native solo notes and eight-note two-hand chords across the compass at two playing strengths, including all four Clavinet pickup positions with all tone tabs on. Reproduce it with `cargo test -p openwurli-ui hohner_performance_headroom -- --ignored --nocapture`.

The original piano probe also reports the full keyboard struck simultaneously as a separate stress measurement. That gesture, boosted tone settings or a higher master volume can exceed full scale. Master volume is linear and reaches another 6 dB above its default. The calibration applies a constant multiplier to both channels.

```sh
cargo test -p openwurli-ui --release --lib measure_peak_trims -- --ignored --nocapture
```

## Comparing the held chord

On Stage 73, hold E2–B2–E3 and press sustain with all three keys still down. The other tines become free to ring. On Electric Grand, enable **Frame Ring** to hear the same comparison through its string frame. Its top 15 notes have no dampers. Both models sum the sympathetic motion before the piano’s own electronics. [Stage model and Peterson bench](../stage73/README.md) · [Grand mechanics](../cp80/README.md)

The installed VST3 comparison writes the two dry takes per setting, their difference and a stereo listening clip of the same phrase through the passive panel and Peterson. It needs Pedalboard, Mido, NumPy and SciPy.

```sh
python3 openwurli-ui/research/pedal_comparison.py \
  "$HOME/Library/Audio/Plug-Ins/VST3/Electric Pianos.vst3" target/electric-pianos-pedal
```

At 48 kHz and MIDI velocity 96, the installed Stage pedal-down difference measured **−18.58 dB** relative to the held pedal-up chord. Energy at the unplayed F2 rose from −84.18 to −55.16 dBFS. The Grand gave identical held-chord samples at its stock setting and a **−19.65 dB** difference with Frame Ring enabled. At 60–80 ms after lifting the pedal, the difference signal had fallen by 62.7 dB on Stage and 35.5 dB on Grand relative to its final 100 ms under sustain. These figures describe the model. Matching dry hardware takes would set the physical coupling strengths.

For a hardware comparison, use tremolo off, fixed recording gain and the same direct output throughout. Strike the chord at 0.05 s, lower the pedal at 0.35 s on the second take, lift it at 1.15 s and release all three keys at 1.50 s. Pass the aligned takes with `--hardware up.wav down.wav` to compare pitch levels. Separate repeated strikes vary in phase and touch, so the hardware analysis reports each take’s spectrum. Isolated note-offs with known release times would further constrain the damper model.

## Upstream update (0.2.1)

The DSP tracks upstream **v0.9.1 “RestlessHeart”**, revision [`c05403b9b60f95b4908ae19727682e81803a7351`](https://github.com/hal0zer0/openwurli/commit/c05403b9b60f95b4908ae19727682e81803a7351), checked against upstream `main` on November 22, 2026. This brings upstream's removal of the hammer attack noise burst, retired engine cleanup, and MIDI/CLAP note-control fixes. The 0.2.1 wrapper introduced the character controls, extended notes, linear Volume and Extra Sag used here.

Wildcard note-on is ignored; wildcard note-off and CC 123/124–127 release every key while respecting sustain. CC 121 lifts sustain. CC 120 and CLAP choke fade the affected voices over 5 ms without resetting the circuit. Individual choke targets the exact pitch, including extended notes held after Extended Notes is disabled.

Existing Wurli voicing and volume automation retain their parameter IDs. The v0.9.1 attack differs slightly because the old noise burst is gone. Sessions originating before wrapper 0.2.0 also adopt the v0.9 physical model and its output level. Restore the previous source/build for exact older playback.

See [local DSP changes](vendor/openwurli-dsp/LOCAL_CHANGES.md) for the integration boundaries. The earlier [CPU report](docs/cpu-optimization-results.md), [runtime-mode report](docs/runtime-modes.md), and [Heavy recovery proof](docs/heavy-recovery-shortcut-2026-10-03.md) describe the previous v0.7 engine; their performance and bit-equivalence claims do not apply to this version. The old SIMD reed/transistor paths, experimental circuit tables, shared shadow-preamp solve, and Heavy recovery shortcut are superseded by the v0.9 source. Generated preamp and power-amplifier circuit equations remain unchanged; their transistor helpers reuse an unchanged final evaluation, as the local tremolo solver already did.

The [first v0.9.1 optimization pass](docs/optimizations-2026-11-22/README.md) rejected a reed SIMD candidate with a release regression. The [SIMD follow-up](docs/simd-investigation-2026-11-22/README.md) keeps the fundamental scalar for air drag and uses consistent upper-mode pairs through damping, oscillation and renormalization. This revised implementation matches the scalar output bit for bit and passes the performance gates. In the measured 32/64-voice cases, isolated reeds use about 19% less time while held and 10–11% less while releasing; complete Fast rendering improves by about 2.8/3.9% for held notes. Heavy gains are under 1%. The original rejected implementation and scalar reference remain in the wrapper tests.

## Controls

The Reed Action cutaway shows a felt hammer striking a steel reed between the fingers of a capacitive pickup. Play a MIDI note or click the on-screen keyboard to see the strike in slow motion. Drag the hammer, reed or pickup vertically to adjust Hammer Hardness, Reed Decay or Pickup Height. Double-click a part to restore its default.

The 728 × 588 editor puts the component knobs beside their diagram labels. Stage 73 has a Rhodes era dropdown and draggable tine and pickup adjustments. The Electric Grand cutaway follows the sounding key’s string course and its hammer can audition that pitch. The shared range switch selects the physical compass of the current piano or the full MIDI keyboard.

Pickup Height moves all pickup fingers up or down by up to 0.120 mm from their stock positions. Sounding reeds follow a height change over 10 ms, including notes held by the sustain pedal. Raising the fingers moves them farther from the resting bass reeds and closer to the treble reeds. The range is a plugin design choice. Wurlitzer describes voicing individual reeds by adjusting their pickup fingers vertically. [Series 200 service manual, figure O](https://www.jupitervintagepianos.com/wp-content/uploads/2023/10/Wurlitzer-Series-200-Service-Manual.pdf#page=16)

Reed Decay spans 0.5–20×, Hammer Hardness spans 0.5–2× and Pickup Drive spans 0.5–1.04×. All multipliers are neutral at 1×. Reed and hammer settings affect newly started notes. Pickup Drive scales the physical swing in millimetres, including the associated air-drag loss. Cell Response shows the recovery time of the tremolo photocell, from 4.4 to 280 ms. At the stock 35 ms setting its attack is 2.5 ms. The two times scale together. Existing saved multipliers keep their timing. Host automation stored as normalized knob positions spans the wider range, with the stock cell at the same one-third position. With Tremolo Depth above zero, shorter recovery sharpens the pulse and longer recovery smooths it.

Learned voicing is always active, now using upstream's corrected-pitch training and its tuning/decay corrections. The retired MLP pickup-displacement correction is no longer applied. The retired `mlp` and noise IDs are ignored when loading old state. Hiss stays disabled in both native preamps.

Volume retains its existing linear output behavior and 50% default. Internally the v0.9 physical volume network stays at upstream's nominal 80% position and factory reed-bar trim. The +14 dB alignment follows the circuit/speaker chain, then Extra Sag applies optional level-dependent compression with the existing 8 ms attack, 150 ms release and strength of 12. Volume follows these stages, so changing it does not change tone or sag. Extra Sag defaults off and saved settings survive.

The shipping Wurli uses the handwritten preamp and behavioral power amplifier. The optional `heavy-reference` Cargo feature preserves access to the original dual-circuit DSP regression tests. The installed plugin exposes one circuit.

The normal MIDI ranges are 33–96 for OpenWurli, 28–100 for Stage 73, CP-70 and CP-70B, and 21–108 for CP-80. Extended Notes plays pitches outside these ranges using the nearest physical key’s calibration. Every note retains its MIDI identity for note-off, sustain and choke. [CP-70 scale and pickup harness](../cp80/research/cp70/README.md) · [CP-70B scale, signal path and verification](../cp80/research/cp70b/README.md)

## Build and verify

From the workspace root:

```sh
cargo check -p openwurli-ui
cargo test -p openwurli-ui --release --quiet
./openwurli-ui/scripts/install.sh
```

The installer rebuilds from the current source, verifies the bundled executables, installs both formats, and verifies the signed installed bundles. Reload the plugin in the host after replacing a loaded bundle.

Release tests cover the model switch, MIDI timing, range changes, voicing, stereo tremolo, master volume, sag and the saved parameter schema. The fixture comparisons use independent upstream renders. [Audio fixtures](tests/fixtures/upstream-0.9.1/README.md)

## License and attribution

OpenWurli DSP is by hal0zer0 and is licensed under GPL-3.0-or-later. This wrapper and its Pleasant UI dependency are also GPL-3.0-or-later; the license text is in [LICENSE](LICENSE). Preserve upstream's license and source attribution when distributing binaries; the complete corresponding source includes the pinned OpenWurli revision and this wrapper.
