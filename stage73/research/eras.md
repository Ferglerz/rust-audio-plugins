# Evidence for the six Stage 73 instruments

Each position represents one build from its period. Production changes overlapped. A serviced piano can have replacement tines, tips, electronics and pickup adjustments. The notes below identify which parts determine the rendered sound and where an estimate enters the model.

## From the drawings to the sound

| Mode and representative build | Physical evidence | What the renderer does |
| --- | --- | --- |
| **Sparkletop · circa 1967 mono Suitcase** | Felt hammers, early tines, the original 73-pickup bus and the Jordan mono preamp. The contemporary circuit distinguishes this instrument from the later stereo Suitcase. [Original harp, fig. 10-3](https://www.fenderrhodes.com/service-manuals/1979/ch10.html) · [Jordan schematic, fig. 11-17](https://www.fenderrhodes.com/org/manual/fig11-17.jpg) · [Instrument and harp recordings](https://www.fenderrhodes.com/html/models/silvertop.html) | Softer strikes have a longer contact pulse. The original pickup grouping drives the active bass, treble, loudness and mono tremolo network. Output is taken at the preamp wiper under the cabinet input load. |
| **Early Mark I · 1972 Stage** | Tear-drop felt and square neoprene overlapped the start of this period. A photographed 1972 harp has square tips. A repairer's circuit trace shows the early grounded 10 kΩ tone pot and 1 µF shunt. [1972 tip photograph and measurements](https://www.vintagevibe.com/blogs/news/the-definitive-guide-to-vintage-vibe-hammer-tips) · [Morrin's two Stage circuits](https://sites.google.com/site/davidmorrinoldsite/home/trouble/trouble-keyboards/rhodes) | Models ⅜-inch square tips and treble cut. Fully clockwise still leaves the tone-pot load across the harp. The revised pickup wiring and 10 kΩ volume feed the jack. |
| **Mid Mark I · 1974–75 transitional Stage** | Surviving instruments document the wood/plastic hammer transition. Morrin measured a 1974-dated bass-cut panel with an unusual volume pot. The 10 kΩ volume variant is the one modeled here. [1974 instrument](https://vintagerockkeyboards.com/2015/08/04/fender-rhodes-stage-piano/) · [1975 restoration](https://chicagoelectricpiano.com/blogs/news/sale-1975-rhodes-mark-stage-73) · [Panel trace and pot variants](https://sites.google.com/site/davidmorrinoldsite/home/trouble/trouble-keyboards/rhodes) | Shares the early square-tip excitation and switches to the later bass-cut panel. There is no measured hammer-mass correction for the hybrid shank. |
| **Late Mark I · 1976–79 Stage** | The factory manual gives hammer numbers, tip heights and hardness zones, pickup bus grouping and voicing adjustments. The Stage drawing specifies both pots and the capacitor. [Factory adjustments, ch. 4](https://www.fenderrhodes.com/service-manuals/1979/ch4.html) · [Harp, fig. 10-4](https://www.fenderrhodes.com/service-manuals/1979/ch10.html) · [Stage panel](https://www.fenderrhodes.com/img/service/manual/stgschem.gif) | Uses the four-strength per-key calibration, graduated tips, 47 nF bass network and jack load. This is also the legacy-session default. |
| **Mark II · later plastic-key Stage** | The plastic keybed had two balance-rail positions. The patent gives an example spacing of ⅜ inch. Factory designer Mike Peterson describes the reduced hammer stroke. [Patent US4308783A, fig. 3](https://patents.google.com/patent/US4308783A/en) · [Peterson's account](https://loudroundrecords.com/RhodesPiano.htm) · [Mark II instruments](https://www.fenderrhodes.com/models/mark2.html) | Selects the rear position, reducing hammer swing for a given key speed. The passive Stage panel is shared. The register-dependent sustain comes from the direct-harp comparison below. Its source piano's key material is unknown. |
| **Mark V · 1984 Stage** | Rhodes advertised a two-inch hammer stroke compared with 1⅝ inches. Steve Woodyard describes bends in the damper arms that reduce whipping. A contemporary reviewer observed volume, tone and a mono jack. [1984 Rhodes advertisement](https://www.worldradiohistory.com/Archive-All-Music/Musician/1980/Musician-1984-11.pdf) · [Woodyard on dampers](https://www.fenderrhodes.com/service/action-inspection.html) · [1985 hands-on review](https://www.muzines.co.uk/articles/fender-stage-73-mk-v/5058) | More hammer swing on hard strikes and faster damping on release. The panel uses the later Stage network provisionally. A verified Mark V board trace or jack sweep would establish its component values and response. |

### Pickup wiring and output level

In figure 10-3 the bus cuts define seven parallel bass pickups followed by eleven groups of six, all in series. Figure 10-4 shows four bass pickups followed by 23 triplets. The same moving pickup therefore contributes 1/7 or 1/6 of its open-circuit voltage in the original harness and 1/4 or 1/3 in the revised harness. Relative series impedance changes from 1/7 + 11/6 to 1/4 + 23/3, a factor of about 4.01. The manual also describes a fourfold increase in basic output without specifying the excitation or load. The model computes the voltage at the selected load for the actual keys played. [Factory wiring comparison](https://www.fenderrhodes.com/service-manuals/1979/ch10.html)

The independent decks expand all 73 pickups. The renderer reduces identical pickup branches analytically. Nominal values are 180 Ω and an estimated 75 mH per pickup. The Stage jack sees a 1 MΩ input. Cable capacitance defaults to 300 pF. On the later panel at full volume and bass, moving from 100 to 1000 pF raises the 5 kHz response by 0.71 dB. At half volume the change is 0.01 dB. For scale, Mogami specifies 130 pF/m for W2524 instrument cable. The later bass control is a 50 kΩ rheostat bridged by 47 nF. Pot taper between endpoints is an approximation. [Cable specification](https://mogamicable.com/category/bulk/guitar/) · [Pickup resistance notes](https://www.fenderrhodes.com/service/pickups.html) · [Late panel drawing](https://www.fenderrhodes.com/img/service/manual/stgschem.gif) · [Early and late traces](https://sites.google.com/site/davidmorrinoldsite/home/trouble/trouble-keyboards/rhodes)

### Jordan electronics

Figure 11-17 gives the 33 kΩ and 10 kΩ input resistors, the two collector-feedback Darlington pairs, and the 100 kΩ bass and treble controls. The modeled network includes the 0.1 µF bass capacitors, 5 nF treble branch, Q6 follower and filter, 100 kΩ loudness pot and its RC return. The next stage loads the wiper through 1 µF and 10 kΩ. This output can feed an external amp or cabinet effect. The plugin models the small-signal response at that point. [Jordan AU-0598/AU-0599 schematic](https://www.fenderrhodes.com/org/manual/fig11-17.jpg)

The SPICE deck solves the biased transistor circuit at 13.5 V using a nominal silicon transistor gain of 200. The renderer uses the resulting linearized transconductances. Device spread and large-signal clipping remain unmeasured. The tremolo table comes from a transient simulation of the Q1 oscillator at three speed resistances. The renderer interpolates its shape and approximates Q7's shunt attenuation from zero to full depth. Actual FET threshold, depth-pot law and maximum attenuation need a hardware measurement. The default loudness is 80% for headroom, both tone pots are centred, and tremolo starts off. [Transistor deck](jordan.cir) · [Oscillator deck](jordan_lfo.cir)

### Mechanical assumptions

- The Late Mark I table supplies four swing anchors, pickup height and gap. Mark II starts from the harp setup used to calibrate its decay, then applies the individual pickup adjustments measured below. Across four Mark II notes, applying the new Late Mark I voicing raised the median octave-balance error to 27.09 dB. The original Mark II setup gave 13.79 dB with the shared elastic hammer. The factory's strike line varies across the keyboard. A model-wide change of tine supplier, logo, harp support or case weight has no assigned spectral coefficient. A measured per-era strike line, flexural spectrum or pickup setup could support one. [Factory adjustments](https://www.fenderrhodes.com/service-manuals/1979/ch4.html) · [Original calibration and dry strikes](https://github.com/jstolberg/rhodes/tree/dedf7156f159c85e1c43802cb0764497d0f361fb)
- For early and mid Stage tips, the model scales contact time by the square root of the ratio of the ⅜-inch cube height to the later tip height in the same zone. Later heights are ¼, 5/16, ⅜ and 7/16 inch. Relative contact time also changes excitation of the inharmonic modes. The compliance scaling is an engineering estimate. Felt uses a separate estimated contact multiplier of `1 + 0.6 × (1 − velocity)`. The recordings lack hammer-speed or contact-time measurements. [Tip profiles](https://www.vintagevibe.com/blogs/news/the-definitive-guide-to-vintage-vibe-hammer-tips) · [Factory tip table](https://www.fenderrhodes.com/service-manuals/1979/ch4.html)
- Mark II uses estimated front and rear lever arms of 7.5 and 5.5 inches with the patent's ⅜-inch shift. This gives approximately 89% of reference swing. Mark V scales swing by `1 + velocity × (2 / 1.625 − 1)`. Its damper loss rate increases by an estimated 10%. The throw dimensions and damper construction establish the direction of these changes. The velocity and release curves are modeling choices. [Mark II patent](https://patents.google.com/patent/US4308783A/en) · [Mark V advertisement](https://www.worldradiohistory.com/Archive-All-Music/Musician/1980/Musician-1984-11.pdf) · [Damper construction](https://www.fenderrhodes.com/service/action-inspection.html)

## Electrical verification

The probe excites one pickup at a time and records the panel before decimation. The comparison uses a half-second impulse at four times the host rate and the SPICE AC solution from 40 Hz to 10 kHz. Settings cover 44.1, 48 and 96 kHz, three volume settings, tone endpoints and centre, and pickups on both sides of the bass group transitions. The Jordan sweeps also exercise both tone controls. The tolerance is 0.5 dB.

| Electrical output | Sweeps | Largest magnitude difference |
| --- | ---: | ---: |
| Early Stage jack, 100 / 300 / 1000 pF | 81 | 0.185 dB |
| Later Stage jack, 100 / 300 / 1000 pF | 81 | 0.118 dB |
| Jordan loaded preamp wiper, tremolo off | 45 | 0.204 dB |

The controls change different frequency ranges. With the same G3 pickup driven and volume fully up, ngspice gives the following change from fully counterclockwise to fully clockwise. For Jordan, the other tone control is centred.

| Control travel | At 80 Hz | At 1 kHz | At 5 kHz |
| --- | ---: | ---: | ---: |
| Early Stage Tone | +0.6 dB | +24.2 dB | +43.5 dB |
| Later Stage Bass Boost | +11.0 dB | −0.2 dB | −0.2 dB |
| Jordan Bass | +17.0 dB | −0.4 dB | 0.0 dB |
| Jordan Treble | +0.8 dB | +10.0 dB | +22.7 dB |

Nine additional time-domain checks measure the envelope of a 1 kHz carrier at the Jordan output. The oscillator deck predicts 2.331, 4.648 and 8.727 Hz. Output at 50% and 100% depth tracked these within 0.010 Hz. At 0%, 50% and 100% depth the largest error in fractional envelope excursion was 0.00047. These figures measure agreement with the component model and the selected attenuation law. [Electrical checks](check_eras.py)

The transient input sweep drives the original harness’s equivalent source impedance with calibrated sine amplitudes. With both tone controls centred, volume at 80% and tremolo off, the nominal transistor deck predicts 0.25 dB of gain reduction at 100 mV peak and 3.82 dB at 300 mV peak at 1 kHz. At 80 Hz it predicts 0.07 and 0.99 dB respectively. The CSV reports second and third harmonics as well. These are simulated harp-voltage thresholds. Converting plugin level to physical volts and verifying onset of overload require a measured hardware sweep. The shipping Jordan path uses the small-signal response. [Jordan deck](jordan.cir) · [Sweep implementation](check_eras.py)

## Recording comparisons

Olivier Grall recorded his Sparkletop directly from the harp. For the upper three notes of two chords in `sparkle4.mp3`, the median octave-balance error at nominal velocity 0.69 fell from 9.28 to 5.89 dB. Lowering all tines by 0.75 mm moved the first chord’s median from 13.94 to 12.33 dB and the second’s from 2.13 to 17.88 dB. The recording has no measured strike speeds. [Grall’s harp recordings](https://www.fenderrhodes.com/html/models/silvertop.html)

Three pitches in `sparkle2.mp3` have exposed releases. The intervals below measure how long the fundamental takes to fall a further 6 dB after it first crosses −3 and −9 dB. This avoids assuming an exact key-up or pedal timestamp. All model strikes used velocity 0.69. The first A4 informed the contact time scale. The other releases show the variation across the performance. The Sparkletop now ramps felt loading quadratically, with twice the Stage time scale. This better follows the first A4 and D5. The later A4 has a quicker initial release than the model. [Release measurement](check_releases.py)

| Strike and location | Recorded −3 to −9 / −9 to −15 dB | Model |
| --- | ---: | ---: |
| A4 at 12.290 s, calibration | 19 / 10 ms | 17 / 10 ms |
| B♭4 at 12.460 s | 16 / 7 ms | 16 / 10 ms |
| A4 at 13.190 s | 10 / 11 ms | 17 / 10 ms |
| D5 at 16.655 s | 19 / 9 ms | 16 / 10 ms |

Tim Kahn recorded a Mark II harp through a Bellari tube preamp. Four individual notes are identifiable by pitch. E3 and E6 set the fundamental loss multipliers to 1.05 and 0.29, with logarithmic interpolation across semitones and fixed endpoint values outside the span. E4 and E7 were reserved for evaluation. The E7 tail still decays too slowly. Measured velocities and a keybed identification would help separate this particular harp's voicing from plastic-key action. [E3 recording](https://freesound.org/people/tim.kahn/sounds/65725/) · [E4](https://freesound.org/people/tim.kahn/sounds/65726/) · [E6](https://freesound.org/people/tim.kahn/sounds/65659/) · [E7](https://freesound.org/people/tim.kahn/sounds/65660/)

A second pass compared 17 individual notes from Kahn’s harp at a fixed modeled velocity of 0.69. At 13 pitches, the stock height and gap were adjusted to the recorded octave and third harmonic during the attack and early sustain. The fit limited changes in fundamental level to 3 dB. The pickups can be positioned individually, and interpolating the same offset to neighboring keys sometimes deepened an octave cancellation. Across all 17 notes the median absolute octave error fell from 13.49 to 1.24 dB over 90–300 ms. In the 400–620 ms window excluded from fitting, octave error fell from 14.43 to 3.85 dB and third-harmonic error from 17.08 to 11.46 dB. These are harmonic levels relative to each strike’s fundamental. The MP3 previews include the Bellari preamp and the original strike velocities are unknown. The optional neighbor filenames are listed by Freesound ID in the comparison script. [Tim Kahn’s Mark II recordings](https://freesound.org/people/tim.kahn/packs/3957/) · [Reference comparison](era_audio.py) · [Individual pickup adjustments](calibration.py)

| Mark II fundamental decay at modeled velocity 0.69 | Recording | Before decay fit | Implemented |
| --- | ---: | ---: | ---: |
| E3, fitted | −2.78 dB/s | −2.64 dB/s | −2.76 dB/s |
| E4, held out | −3.73 dB/s | −4.98 dB/s | −3.49 dB/s |
| E6, fitted | −12.32 dB/s | −42.81 dB/s | −12.78 dB/s |
| E7, held out | −44.51 dB/s | −100.19 dB/s | −29.52 dB/s |

Before refining pickup geometry, the median Mark II second-harmonic error across these four notes was 13.79 dB. The resolved attack peak is 7.24 dB too weak on E3 and 6.17 dB too weak on held-out E4, compared with 13.53 and 11.64 dB previously. On E4 the recorded peak was at 2414 Hz and the model peak was at 2386 Hz. E6 had no resolved peak by the same criterion. These recordings give partial validation of sustain and show the remaining spectral error. Isolated, velocity-controlled recordings from identified early, mid and Mark V instruments would support independent calibration of their attacks and releases. The Late Mark I recording comparisons are in the [instrument README](../README.md).

## Level-matched playback

The Electric Pianos VST3 played 25 keys from E1 through E7 in minor thirds at each of MIDI velocities 40, 64, 88 and 112. Each note sounded for 400 ms at 48 kHz with stock tone and 50% master. RMS combines the energy in both channels over all 100 held notes. Tremolo was off and the Peterson intensity was zero. The target of −34.50 dBFS retains the instrument's −1 dBFS performance peak in the separate three-rate chord sweep. Each adjustment is a fixed gain after the electronics and slews over 10 ms when switching. The raw jack and harp remain available in the offline probe.

| Era | Previous stock RMS, dBFS | Stock adjustment, dB | Peterson adjustment, dB |
| --- | ---: | ---: | ---: |
| Sparkletop | −34.06 | −0.44 | — |
| Early Mark I | −36.44 | +1.94 | +1.47 |
| Mid Mark I | −35.19 | +0.69 | +1.47 |
| Late Mark I | −36.52 | +2.02 | +2.85 |
| Mark II | −41.64 | +7.14 | +7.85 |
| Mark V | −34.04 | −0.46 | +0.27 |

A separate phrase with eight single notes and three chords spanned 2.08 dB across the six stock outputs. Five eras were within 0.45 dB. Mark II played the phrase most quietly. Register balance, touch response and release still affect the loudness of any particular performance.

### Which eras earn a selector position?

- **Sparkletop** earns its place through the felt strike, earlier pickup wiring and mono Jordan controls. The direct-harp comparison above puts the median second-harmonic error at 5.89 dB on six notes taken from chords. Those recordings provide no strike velocities.
- **Early and Mid Mark I** share the same modeled square-tip action, swing and pickup setup. At stock tone their median absolute differences were 0.30 dB for the second harmonic relative to the fundamental and 0.71 dB for the third (90–300 ms, the 100 matched strikes above). Through the Peterson output they produced identical samples on the separate phrase at zero intensity. Early’s passive panel cuts treble and Mid’s restores bass. If simplifying the selector, combine this action under one entry with both panel choices and map existing era IDs to the corresponding panel. A separate Mid action would need measured evidence for the hybrid shank.
- **Late Mark I** is the best recording-backed reference and merits its own entry. Its per-key pickup geometry and four velocity anchors give a median 3.97 dB change in octave balance versus Mid on the same grid. The independent 1977 harp comparisons still show substantial note-to-note error. [Recording fits and held-out results](../README.md#late-mark-i-recording-comparisons)
- **Mark II** merits its own entry for the different touch and longer upper-register sustain. On held-out E4 the fundamental decay was within 0.24 dB/s of the recording. On E7 it decayed about 15 dB/s too slowly. The newer 17-note comparison above measures the individual pickup adjustments. The high-register decay and several third harmonics still merit direct hardware comparison.
- **Mark V** merits a provisional entry. The longer throw and revised damper have historical support. The model's stock second and third harmonics differ from Late Mark I by median absolute amounts of 3.79 and 3.21 dB relative to the fundamental. Direct recordings at known velocities and a panel trace would establish how closely its attack, release and EQ match a particular Mark V.

Run the installed comparison with NumPy, Mido and Pedalboard. The WAV plays the same short phrase in selector order, with one export gain for all six takes. The CSV records every measured strike. [Measurement and listening-clip generator](../../openwurli-ui/research/stage_levels.py)

```sh
python3 openwurli-ui/research/stage_levels.py "$HOME/Library/Audio/Plug-Ins/VST3/Electric Pianos.vst3" target/electric-pianos-voicing
```

## Reproducing the evidence

From the workspace root, with ngspice, NumPy, SciPy and SoundFile available, run

```sh
cargo build -p stage73 --release --example probe
python3 stage73/research/check_eras.py target/release/examples/probe target/stage73-eras
python3 stage73/research/era_audio.py target/release/examples/probe target/stage73-eras
```

The first script writes signed circuit errors, tremolo measurements and `jordan-overload.csv`. The second writes fixed-gain and level-matched WAVs plus a per-strike CSV. Each WAV plays the six modes in selector order, with E2, G3 and E6 at three velocities per mode. Fixed gain preserves differences in electrical level. Level matching applies one RMS gain to each nine-strike take.

To reproduce the hardware and pickup-height comparisons, supply a third argument to `era_audio.py` pointing to the reference directory. Use Grall's original `sparkle4.mp3` and the high-quality MP3 previews for Kahn's four Freesound IDs named `65725.mp3`, `65726.mp3`, `65659.mp3` and `65660.mp3`. The scripts produce `hardware-errors.csv` and `height-check.csv`. For the release checks, run `python3 stage73/research/check_releases.py target/release/examples/probe /path/to/sparkle2.mp3 target/stage73-eras`. Only the development scripts read recordings. To regenerate the checked-in oscillator tables, run `python3 stage73/research/jordan_lfo.py`.

### Installed six-era cross-check

The November 2026 18-model Electric Pianos VST3 was replayed with the generator above. Across its 100 matched stock note/velocity pairs per era, phrase levels stayed between −34.505 and −34.469 dBFS RMS. That alignment makes it possible to compare the harmonic changes separately from the overall playback level. The table gives median absolute differences between matching strikes. The Sparkletop/Early values below describe the stock settings. Moving the Jordan controls or the early treble-cut pot samples different parts of their distinct electrical responses.

| Representative builds | Strike RMS | H2/H1 | H3/H1 |
| --- | ---: | ---: | ---: |
| Sparkletop and Early Mark I | 0.39 dB | 0.56 dB | 0.83 dB |
| Early and Mid Mark I | 0.20 dB | 0.30 dB | 0.71 dB |
| Mid and Late Mark I | 0.79 dB | 3.97 dB | 1.20 dB |
| Late Mark I and Mark II | 2.63 dB | 9.45 dB | 8.70 dB |
| Late Mark I and Mark V | 0.88 dB | 3.79 dB | 3.19 dB |

The two early square-tip models still gave identical stock-phrase level through the optional Peterson at zero intensity. Their measured native-panel difference follows the treble-cut versus bass-cut circuit. Late Mark I and Mark II include different measured pickup voicings. The Mark II lever ratio and Mark V swing/damper changes use mechanical estimates with the documented dimensions and construction. The measurements here do not establish a production-wide signature for every piano built in each period. [Installed measurement generator](../../openwurli-ui/research/stage_levels.py)
