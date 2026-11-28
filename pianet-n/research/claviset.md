# Weltmeister Claviset 200 and Selmer Pianotron 8701

The two playable reconstructions share a rotating cam, a free steel reed, a felt damper and a magnetic sensing hypothesis. Their terminal circuits, modulation and performance controls differ. The evidence establishes the Selmer's transformer and the two panels. An inspectable serial-matched factory drawing for the 200, 300 or 8701 has yet to surface. The 300 remains a separately identified instrument.

## Hardware identification

| Hardware | Direct evidence | Implementation and limits |
| --- | --- | --- |
| Weltmeister Claviset 200 | The Deutsches Museum identifies the model, nine tone selectors and three modulation rates. Steve Christie shows the left volume pedal and its mechanical holding detents at 1:48–2:43, Bowden-cable damper lift at 4:10, individual tabs at 5:55–8:08 and the pickup rail and switching underneath at 10:42. | Nine mutually exclusive registers, three rates and off, power, variable foot swell and an independent sustain pedal. Continuous pedal travel approximates the gain between detents. Pedal detent spacing and electrical taper have yet to be measured. [Deutsches Museum instrument](https://artsandculture.google.com/asset/%E2%80%9Cweltmeister%E2%80%9C-claviset-200-klingenthaler-harmonikawerke-klingenthal/JgGfrxxBEqUYiQ?hl=en) · [Christie’s instrument](https://www.youtube.com/watch?v=-ncOPN2Nn2w) |
| Klingenthal pickup | A 1964 fair report describes magnetic pickups and transistor amplification on a VEB instrument. Spectrasonics describes magnetic sensing on its Claviset and a rotating rubber disc with a lip. A Czech owner describes their 200 as capacitive. The visible rail alone cannot establish the transducer principle. | The 200 models the documented magnetic configuration. The Czech 200’s bias or pickup wiring, serial boundaries and the 300 electronics remain unresolved. A service dealer lists a 200/300 sheet. No circuit values here derive from that listing. [1964 fair report](https://www.worldradiohistory.com/INTERNATIONAL/Funktechnik/60s/Funktechnik-1964-19.pdf) · [Spectrasonics instrument](https://www.spectrasonics.net/products/keyscape/models/claviset/) · [Keyscape reference, p. 136](https://www.midimanuals.com/manuals/spectrasonics/keyscape/reference_guide_v1_1/8190_14901_en_1516660869.pdf) · [Czechkeys inspection](https://czechkeys.blogspot.com/2010/05/weltmeister-claviset-200-ukazka.html) · [Service-sheet inventory](https://www.manuals.altervista.org/Other-Service-Manuals-Schematic-Diagrams.html) |
| Selmer Pianotron no. 8701 | Selmer’s catalogue shows a 61-key instrument, tone, variable vibrato and knee-operated volume. A technician describes magnetic pickup, transformer, battery amplifier and the knee lever on serial 877. Dean Coyle shows three controls at 0:36, felt and rotating cam at 1:25–1:46 and knee control at 1:58. | Separate transformer input, battery preamp approximation, continuous tone, modulation depth and speed, and knee swell. Key-up seats the individual felt. The two 4.5 V batteries energize the historical preamp. An original signal drawing, transformer winding and tape of isolated notes with marked settings are outstanding. [Selmer illustrated catalogue](https://www.vintagehofner.co.uk/selmer/misc/pianotron2.jpg) · [April 1966 advertisement](https://www.vintagehofner.co.uk/selmer/misc/pianotron.html) · [Serial 877](https://www.spheremusic.com/Bargaindtl.asp?Item=12034) · [Coyle’s instrument](https://www.youtube.com/watch?v=E5nMy8r2vVM) |

## Moving parts and terminals

The five-octave compass is mapped to C2–C7. Extended range keeps the nearest native reed and pole dimensions while tuning each additional key to its requested frequency. The cam lip lifts the reed at 91% of its length. The felt acts independently at 76%. In the projected model the cam starts at −24°, contacts a resting reed at −4°, loses lateral overlap at +38°, and reaches +76° at full key travel. It bypasses the reed on the return and rearms below 12% key travel. Those dimensions and angles are estimates from the mechanism, pending a scale and side-on slow-motion recording. The damper and lip continue to act on the moving reed through partial returns, repeated strikes and pedal transitions.

The stock magnetic clearance grows from 0.42 mm in the bass to 1.0 mm in the treble. The linked flux approximates a laterally offset pole's field at the blade tip. Its time derivative gives the common rail voltage. The selected clearance and absolute voltage constant require a measured pole profile and open-circuit coil recording.

The 200 source equivalent uses 1.55 kΩ and 0.18 H. The Selmer coil and transformer use 620 Ω, 1.6 H and a 1:4 turns ratio referred to the modeled input. Both feed an estimated common-emitter gain stage and a 1 MΩ receiver. Component values, register RC networks, bias and modulation depth are engineering estimates. The Selmer control sweeps the tone network continuously. The 200 switches nine RC combinations. Its modulation rates follow the owner recording. The 8701’s continuous speed range of 3–9.5 Hz remains provisional. The 200 tube/transistor part designations and the Selmer transformer ratio await original circuit sheets. [Small-signal models and component values](../src/claviset/panel.rs)

## Recording checks

The comparison uses the actual owner performances above. The analysis takes a 160 ms Hann window around each identified note, independently refines model and recorded frequency, and measures harmonics 2–6 relative to the fundamental. An accepted recording window has at least 91% of its weighted energy explained by a harmonic fit. The model uses MIDI velocity 80 and a window centered 210 ms after note-on. The 200 comparison uses the spoken register selections. Each A/B pair is matched in RMS for listening. The −40 dBc floor in the error calculation prevents nearly inaudible upper harmonics from dominating the statistic. RMS alignment and the unknown recorded gain rule out an absolute output comparison.

| Instrument | Recorded notes or note/register combinations | Median absolute H2–H6 difference | Median H2 difference | Median H3 difference |
| --- | ---: | ---: | ---: | ---: |
| Selmer, C2–E5 | 13 | 4.53 dB | 12.1 dB | 5.7 dB |
| 200, Bass / Celesta / Music Box / Xylophone | 11 | 6.48 dB | 8.0 dB | 10.0 dB |

The second harmonic remains an audible mismatch on both instruments. Selmer fundamental decay in dB/s was 10.99 recorded / 10.43 rendered at C4 and 27.54 / 26.42 at A4, the two central decay anchors. At the reserved E4 and G4 it was 14.70 / 17.70 and 18.90 / 22.95. Christie’s 200 gives modulation peaks at 3.71, 6.24 and 8.86 Hz. The model runs at 3.7, 6.2 and 8.9 Hz. The recordings omit synchronized physical key-up and matched pedal positions. They do not establish cam escape timing, exact pedal/knob taper, absolute terminal gain or a measured RC transfer for each register. The 8701 tone setting, output loads and subsequent recording chains are uncertain.

## Reproduce

Render the jack voltages, then decode the two original videos locally to 22,050 Hz mono signed PCM. Their SHA-256 references, selected passages, register names, pitch checks and projection method are in the script. The two comparison WAVs play a hardware excerpt followed by its model note. The CSV records each pitch, fitted-energy share and harmonic level.

```sh
cargo run --release -p pianet-n --example claviset -- target/claviset-probe
ffmpeg -i selmer-owner.webm -ac 1 -ar 22050 -c:a pcm_s16le target/claviset-probe/owner-selmer.wav
ffmpeg -i 200-owner.webm -ac 1 -ar 22050 -c:a pcm_s16le target/claviset-probe/owner-200.wav
python3 pianet-n/research/compare_claviset.py target/claviset-probe
```

The measured hardware/model files are `selmer-hardware-then-model.wav`, `200-hardware-then-model.wav` and `owner-harmonics.csv`. The separately generated registration and modulation demonstrations use identical repeated model notes. [Recording analysis](compare_claviset.py) · [Eight-board comparison and pickup regulation](variant-audit.md)
