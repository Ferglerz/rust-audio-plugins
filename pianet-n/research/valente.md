# Valente production electric piano

The 61-key C2–C7 Valente has a weighted driver, a separate short-throw counter-hammer, individually regulated steel reeds, felt dampers on a common pull and custom neodymium electromagnetic pickups. The two passive panel controls are Volume and Brilliance. The model covers the native compass and offers pitched MIDI 0–127 with the nearest native mechanical section outside the compass. The modeled stock clearance can be adjusted from −0.15 to +1.00 mm. [Maker's production film](https://www.youtube.com/watch?v=k6Wk-lRI75M&t=80s) · [production instrument no. 9 and its specifications](https://reverb.com/item/44822656-valente-electric-piano-modern-vintage-electromechanical-hammer-action-weighted-61-key-piano) · [Simon Beck’s owner review](https://equipboard.com/items/valente-electric-piano)

## What the hardware establishes

| Primary evidence | Consequence for the model | Still unmeasured |
| --- | --- | --- |
| At 1:25 in the production film, the maker shows a finite-element rendering of a flat blade with a bolted, broad root, curved shoulders and narrow free section. Individual adjustment follows at 1:30. | A variable-width cantilever provides bending modes. A 256-element calculation gives frequency ratios 1, 5.772, 15.168 and 28.534 for the first four modes of the estimated section. A tip-normalized mode shape projects force independently at the hammer and damper. | Production reed free lengths, thicknesses and the flexible portion of the widened root across the register. [Maker’s reed and assembly view](https://www.youtube.com/watch?v=k6Wk-lRI75M&t=85s) |
| The maker describes counter-hammers and a polymer action. Beck describes the weighted driver of a modified FATAR keybed operating a second, small hammer. He reports a miss during the slowest key strokes. | Tip-referred driver mass transfers momentum to the free counter-hammer. Its return torque can stop the hammer before it reaches the reed. A unilateral compliant contact drives the reed and decelerates the hammer. | The original pivot distances, masses, nose material, coefficient of restitution and player velocity at the miss threshold. [Maker's action](https://www.youtube.com/watch?v=k6Wk-lRI75M&t=80s) · [owner’s inspection and playing account](https://equipboard.com/items/valente-electric-piano) |
| Josh Paxton documents repairing a loose damper felt with Tiago Valente. He describes hearing other reeds resonate with the pedal down. | Each key opens its own felt. The pull also opens neighboring reed receivers coupled by a lossy frame mode. Changing pickup clearance changes the readout of both paths. | Frame modes, key-to-key coupling strength, elapsed rail travel and any register-specific damper adjustment. [Owner's repair and listening account](https://forums.musicplayer.com/topic/178121-not-ot-i-just-got-a-valente-ask-me-anything/) |
| At 1:55, separately wound bobbins and metal pole pieces appear across a common mount. The maker specifies neodymium magnets. Beck identifies humbucking operation. | Each moving steel tip changes the linked flux of a finite pole. The sum of the induced voltages drives a passive inductive source and loaded control circuit. | Pole contours and magnet polarity, winding order, electrical grouping, resistance, inductance and distributed capacitance. [Maker's pickup close-up](https://www.youtube.com/watch?v=k6Wk-lRI75M&t=115s) · [owner's observations](https://equipboard.com/items/valente-electric-piano) |
| Production no. 9 is described as wholly lead-free, approximately 102 × 42 × 15 cm and 42 lb. Its owner describes adjusting pickup position. The maker names the paired volume/bass and brilliance controls. Beck hears a treble bleed on the volume control and a treble cut on the brilliance control. | Volume loads the signal and bypasses upper frequencies to the wiper. Brilliance sets a shunt RC branch. A separate output load and cable capacitance affect both. | Component trace, part values and absolute terminal sensitivity. The case dimensions establish the scale of the instrument only. [Serial-numbered owner's listing](https://reverb.com/item/44822656-valente-electric-piano-modern-vintage-electromechanical-hammer-action-weighted-61-key-piano) · [player’s control description](https://equipboard.com/items/valente-electric-piano) |

The maker’s 2015 open-action prototype is a separate instrument. Its exposed keyboard offers a view of the earlier arrangement. The modeled reed contour, assembly layout and pickup rack come from production footage. [Tiago Valente playing the prototype](https://www.youtube.com/watch?v=9OAvwy2GMQk)

## Comparison with adjacent models

| Model | Energy enters through | Vibrating section | Transducer and terminal |
| --- | --- | --- | --- |
| Valente | Weighted FATAR driver transfers momentum to a separate short-throw counter-hammer | Steel blade with a bolted, wide shoulder and narrow tip | Neodymium pickup bank, passive volume with treble bleed and passive brilliance cut. Component values are estimates. |
| Columbia EP-61C | Underlever and jack release a hammer caught by the backcheck | Weighted, vertically mounted cantilever | Adjustable angled pole and series bank. The original circuit shows a 10 kΩ resistor, two 100 kΩ pots and a 47 nF capacitor. [EP-61C drawings and verification](elepian.md) |
| Wurlitzer 200A | Escapement action launches a felt hammer | Individually tuned cantilever reeds in a polarized harp | Variable capacitance into a biased pickup and active amplifier. [200A factory manual](https://www.jupitervintagepianos.com/wp-content/uploads/2023/10/Wurlitzer-Series-200-Service-Manual.pdf) |
| Vintage Vibe 64 Classic passive | Hammer drives a tine coupled to its tone bar | Tine–tone-bar assembly | Magnetic pickup into the Classic passive panel. [Maker’s Classic introduction](https://www.fenderrhodes.com/pdf/64classic.pdf) |

The shared reed contact solver calculates force on the moving hammer and damper. Each instrument independently supplies its modes, masses and contact locations. All 53 existing EP-61C audio renders were byte-identical before and after extracting that solver. The common key bank preserves action state during a restrike and during transitions between MIDI and pointer ownership.

## Bench results

At MIDI velocities 40, 80 and 115, E4 first contacted the reed at 11.25, 6.19 and 4.44 ms. Key-down at velocity 8 launched the counter-hammer but produced zero contact force at C2, E4 and C7. E4 peak output into the modeled 1 MΩ receiver was 86.7, 202.9 and 358.9 mV. These are simulation predictions. The strike-time distributions, action masses and absolute gain await a velocity-marked hardware recording.

With E4 at velocity 80, the held jack signal at −0.15, 0 and +1.00 mm clearance offset measured 73.78, 30.28 and 0.88 mV RMS in the 100–200 ms window. At stock clearance, 100–200 ms after key release, the felt-down signal measured 0.106 mV RMS and the pedal-held signal 21.58 mV RMS. The listening fixture uses a common gain throughout. The left channel carries the loaded jack and the right channel carries the primary reed pickup. The four excerpts play in the order listed in `regulation.csv`.

An equivalent 3.4 kΩ / 0.24 H source feeds a 250 kΩ volume pot with a 470 pF treble bleed. Brilliance varies the resistance over 8.2–108.2 kΩ in series with a 6.8 nF shunt. An independent complex-impedance calculation into 100 kΩ and 1 MΩ matched twelve model control sweeps to 0.000001 dB and 0.000003 degrees. At 7 kHz and 1 MΩ, minimum and maximum brilliance gave −3.89 and +1.43 dB relative to source EMF. This verifies the solver for the estimated network. Hardware component values and control tapers remain open.

## Production recordings

The BridgeSet demonstration sends the piano jack through a Warm Audio WA-412 and WA-2A. The performer uses no amp or reverb. The maker’s channel also supplies a separate Asen Doykin performance. Its amplification and processing are unspecified. Neither excerpt has known key velocity, knob positions, gap or electrical reference level. [Fabio Fonseca, recording chain in the description](https://www.youtube.com/watch?v=Mp-kixn4KiE) · [Asen Doykin, published by Valente](https://www.youtube.com/watch?v=mfm0g-BZqlA)

A 160 ms Hann window and complex harmonic projection identify the recorded pitch. One window per pitch/source passed a harmonic fit above 0.92. Each is compared with a fixed velocity-80 modeled jack window centered 210 ms after key-down. An octave-related simultaneous tone can pass this test. The clips retain the neighboring performance for listening; the spectral values alone cannot distinguish every doubled note. Whole-performance channel gains are unknown. The A/B clips equalize center-window RMS, and the table caps both signals at −40 dBc when computing harmonic error.

| Source | Harmonic windows | Median absolute H2–H6 difference | Decay windows | Median absolute fundamental-decay difference |
| --- | ---: | ---: | ---: | ---: |
| Fonseca | 15 | 4.05 dB | 18 | 2.68 dB/s |
| Doykin | 5 | 14.81 dB | 7 | 2.85 dB/s |

Fonseca’s F2, F3 and D5 supplied three inferred clearance and neutral-offset anchors. The remaining twelve Fonseca pitches had a median absolute harmonic difference of 5.67 dB. The independent Doykin performance has stronger middle-register partials and remains the larger mismatch. Both loss comparisons use 0.72-second stretches selected for a decaying fundamental. These performances do not independently establish the attack and damper timing, isolated raw pickup voltage or physical clearances. A marked dry-DI strike at two measured clearances and two known loads would resolve those ambiguities directly.

## Reproduction

```sh
cargo run --release -p pianet-n --example valente -- target/valente-probe
python3 pianet-n/research/valente_section.py
python3 pianet-n/research/compare_valente.py target/valente-probe
```

The comparison expects local mono PCM16 decodes at 22,050 Hz named `fonseca.wav` and `doykin.wav`. It checks SHA-256, writes annotated harmonic/decay CSVs and creates `fonseca-then-model.wav` and `doykin-then-model.wav`. Model note files carry loaded jack volts, primary pickup volts and tip displacement in meters. The interactive diagram uses a slowed schematic motion to show the driver, counter-hammer, reed, adjustable pickup and damper pull. The physical force and timing data come from `action.csv`.
