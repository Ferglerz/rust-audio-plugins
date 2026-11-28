# Vivi-Tone Clavier C 1004

This model reconstructs the playable signal path of Lloyd Loar’s personal 73-key instrument, National Music Museum object 10878. A rising picker on a pivoting jack deflects one end of a flat metal bar. The opposite end moves relative to a magnetic pole face. The plugin output is the loaded instrument terminal before an external amplifier. The lower third, electrically dead during the museum visit, is reconstructed with functioning coils. [Museum catalog and provenance](https://emuseum.nmmusd.org/objects/16691/vivitone-clavier) · [museum account of the examination](https://www.nmmusd.org/nmm-notes/what-happened-to-the-first-electric-piano) · [actual instrument, pickup condition at 10:45](https://www.youtube.com/watch?v=Edomefgocsg&t=645s)

## Evidence behind the action and pickup

| Observed or drawn | Implementation | Limits |
| --- | --- | --- |
| In C 1004 at 13:08–14:21, Emelia Kleidon demonstrates the removed wippen, projecting screw-adjustable wooden picker, damper spoon and felt. The picker lifts and slips off the reed. | A moving wippen brings the pick under the bar. Contact force acts off-center on the jack. Jack inertia and its spring determine how far the pick lifts the reed before its overlap runs out. The upper stop holds it back on return. A new pluck can follow after about one-third key return, as Loar specifies. | Picker covering, overlap, effective inertia and friction on this serial are unmeasured. The black tip material shown on the separate restoration belongs to that example. [Museum action inspection](https://www.youtube.com/watch?v=Edomefgocsg&t=788s) · [Loar, US 1,992,317, Fig. 3 and reset description](https://patents.google.com/patent/US1992317A/en) |
| At 14:29 the camera shows wide, flat, rectangular-section ends under individual cylindrical felts. Loar’s Fig. 3 shows front and rear overhangs, a central span and two opposed pairs of bearing rods. The rear armature sets the speaking pitch. | A steel beam with two constrained translations and finite rotational restraint. A 200-element calculation supplies signed motion at the front picker, rear damper and pickup. Force at the front excites modes shared with the rear. The more heavily damped front mode is separately observable. | Bearing positions of 36% and 67% of length are approximated from Fig. 3. Material modulus, dimensions, rotational restraint, and front/rear loss and mode separation remain estimates. The audible little-used near-unison tuning that Loar describes has no matched measurement for this specimen. [C 1004 reed close-up](https://www.youtube.com/watch?v=Edomefgocsg&t=869s) · [bearing arrangement and independent end damping](https://patents.google.com/patent/US1992317A/en) |
| Fig. 3 shows one adjustable facing pole below an armature. Figs. 4, 6 and 7 show three armatures per face and parallel coils on a permanent-magnet return. In the museum footage the common rail contains capacitors; the on-screen correction says these primarily shape tone. | Signed variation of the rear air gap changes reluctance, inducing `−dΦ/dt`. An equivalent bank of 25 parallel sources feeds the panel. Pickup Distance shifts the pole by −0.15 to +1.00 mm. | Grouping follows the patent. The exact windings, polarity, capacitor distribution, magnetic return and clearances on this serial require measurements. The localized magnetic readout does not directly mix in the pick contact. [Loar pickup and circuit](https://patents.google.com/patent/US1992317A/en) · [museum rail and capacitors](https://www.youtube.com/watch?v=Edomefgocsg&t=648s) |
| C 1004 has a nameboard control marked 0–100, a left electrical volume pedal, a center bass sustain and a right full sustain. The examination shows the rod driving the pedal potentiometer. | Independent nameboard and expression wipers, a bass mechanical lift and a full mechanical lift. Either damper rail can hold a subsequent strike in its register. A released key can reapply the front felt while the pedal holds the rear felt off. | The center pedal’s exact split and latching behavior were not measured. The model uses a continuous lift through B2 (MIDI 47). The patent drawing with *two* counter-moving electrical pedals and the separately restored instrument with bypassed pedal circuitry have distinct controls. [C 1004 pedals](https://www.youtube.com/watch?v=Edomefgocsg&t=517s) · [volume linkage](https://www.youtube.com/watch?v=Edomefgocsg&t=884s) · [other specimen’s bypass](https://www.youtube.com/watch?v=cEG7hD28dW4&t=982s) |

The museum describes a six-octave C-to-C keyboard and the film confirms 73 keys. The model maps C1–C7 (MIDI 24–96). A cross-check on the technician’s separately recorded raw samples found 32.63 Hz for the file labeled `c0` and 261.97 Hz for `C3`. Extended mode retunes the two-ended bar to the requested pitch across MIDI 0–127 with nearest native geometry. Native notes are sample-identical with the extension on or off. [Museum instrument](https://emuseum.nmmusd.org/objects/16691/vivitone-clavier) · [technician’s archive and provenance notice](https://www.youtube.com/watch?v=Edomefgocsg)

## Measured response

At E4 and MIDI velocities 32, 80 and 115, the model begins pulling the bar at 10.45, 5.06 and 3.67 ms after key-down. The jack retracts at 13.13, 7.46 and 5.65 ms. Rear-armature peaks are 0.091, 0.181 and 0.206 mm. Corresponding 1 MΩ terminal peaks are 0.513, 1.139 and 1.208 mV. These are predicted quantities. The footage supplies no timed key contacts, absolute displacement reference or electrical level calibration.

| Modeled E4 clearance adjustment | Held terminal RMS, velocity 80 |
| --- | ---: |
| −0.15 mm | 0.324 mV |
| Stock | 0.266 mV |
| +1.00 mm | 0.099 mV |

B2 remained at 0.121 mV RMS 100–200 ms after release with the bass pedal down. The adjacent C3 fell below 0.000001 mV with that pedal alone and reached 0.119 mV with full sustain down. The tests also cover overlapping pedals, a note struck after the pedal went down, mid-return restrike, choke, pointer plus MIDI ownership and both ends of the extended compass at 44.1, 48 and 96 kHz.

The terminal uses an estimated 9.2 kΩ and 0.72 H per coil, a lumped 33 nF shaping capacitance, two 100 kΩ pot tracks and a 470 pF cable. Eight independent impedance sweeps through two control settings per pot and loads of 100 kΩ and 1 MΩ agree with the circuit solver within 0.000008 dB and 0.00005 degrees. Those numbers verify the mathematical network. The museum's original Webster amplifier and motorized speaker remain separate artifacts with their own unknown transfer functions. [Playback system at the museum](https://www.youtube.com/watch?v=Edomefgocsg&t=526s) · [museum speaker cabinet](https://emuseum.nmmusd.org/objects/12412/vivitone-clavier-speaker-cabinet)

## Two recordings with different provenance

**C 1004.** The National Music Museum identifies the instrument Kleidon filmed. She connected a contemporary recorder with an adapter and later boosted its quiet output. Fourteen measured pitches come from 15:25–16:40. Each 160 ms excerpt explained at least 92% of its windowed energy at the identified pitch and harmonic frequencies. Playing velocity, control positions and phase relative to key-down are unknown. [Museum’s identification](https://www.nmmusd.org/nmm-notes/what-happened-to-the-first-electric-piano) · [C 1004 playing](https://www.youtube.com/watch?v=Edomefgocsg&t=925s)

**Later restored instrument.** The technician’s current “Original Sample Pack” contains individual recordings from another restoration. Her description explicitly distinguishes the updated files from the museum visit. The comparison uses 27 pitched, independently recorded notes, including nine bass notes below C2. The existing names use a different octave convention, so each fundamental was checked. Source audio is by Emelia Kleidon (Emma Repairs), licensed CC BY 4.0. Comparison excerpts were cropped, level matched and interleaved with the model. [Technician’s attribution, license and restoration](https://www.youtube.com/watch?v=cEG7hD28dW4) · [original sample archive](https://drive.google.com/drive/folders/1r09sMJSkF3zfDLNsgPUbGQwLIou7BTOa) · [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)

| Recording | Pitches | Median second-harmonic difference | Reference H2–H6 partials above −45 dBc | Median difference on those partials |
| --- | ---: | ---: | ---: | ---: |
| C 1004 at the museum | 14 | 5.63 dB | 18 of 69 | 7.87 dB |
| Separate restoration | 27 | 2.55 dB | 54 of 132 | 5.06 dB |

The reference generally contains a strong fundamental with very weak upper partials. A pooled median with a −45 dBc floor comes out to zero for both recordings because most partials fall below it. The values above describe the audible differences. Median absolute discrepancy in fundamental-decay rate across the isolated recordings was **1.97 dB/s**. A 6 ms RMS aperture gave median 10–90% rises of 10.02 ms in the recordings and 16.10 ms in the model. C1 gave 26.2 and 59.4 ms respectively. The bass attack is the clearest remaining mechanical discrepancy. Key-up times and pedal positions were unmarked, so hardware release timing remains unverified.

## Reproduction

```sh
cargo run --release -p pianet-n --example vivi -- target/vivi-probe
python3 pianet-n/research/vivi_section.py
python3 pianet-n/research/compare_vivi.py target/vivi-probe /path/to/original-samples.zip
```

Decode the museum video soundtrack to mono PCM16 at 22,050 Hz as `museum.wav` in the output directory. The script checks both input SHA-256 digests. `museum-then-model.wav` and `restored-then-model.wav` use level-matched excerpts. `pedals-and-pickup.wav` plays seven common-gain tests in the order shown in `regulation.csv`, with ×140 listening gain. Model note WAVs carry terminal volts, raw linked-pickup volts, front displacement and rear displacement. The per-note CSV contains the simulated forces’ contact and retraction times. The plugin diagram exaggerates movement for inspection.
