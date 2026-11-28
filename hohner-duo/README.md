# Hohner Pianet/Clavinet Duo

The Duo drives a tangent string and a suction-pad reed from one key. The probe records the Clavinet recording terminal, the Pianet recording terminal and the active mixed jack as three synchronous channels. An owner’s three performances give separate timbre references. The largest measured discrepancy is in the low Pianet register, where the model loses upper harmonics heard in the performance.

## Action and registration

In Electric Pianos, Extended Notes gives both actions the full MIDI 0–127 compass. The split still falls between C3 and C♯3. In Pickups, the two Clavinet coils and the Pianet reed pickup each have their own distance adjustment. Suction Cup Return sets the contribution of the cup reseating at key-up, from 0% to the stock 100%. The cup continues to damp the reed throughout that range.

Hohner describes a 60-key F1–E6 compass (MIDI 29–88). A tangent presses a string against the anvil, and a suction pad lifts and releases a magnetized reed. Key return lets the damping yarn quiet the string and the pad settle back onto the reed. Hohner describes a stronger touch response for the Clavinet. The mute slider damps the strings, the four tone switches act on the Clavinet circuit, and the footswitch selects a registration. [Hohner service manual, printed pp. 11–15](https://anaphonic.com/wp-content/uploads/hohner_pianetclavinetduo_sm.pdf)

Each key retains an independent string and reed state. The string action supplies the key position for the reed. Changing a split or footswitch during a note can reveal an already-vibrating branch. A repeated note allows the key to return before driving it again. String geometry and damping start from the Clavinet model, and the reed, pickup field and suction-pad parameters start from the Pianet T model. Duo-specific string lengths, pad clearances, release forces and coil orientation remain estimates. [Shared-key engine](src/engine.rs) · [Clavinet mechanism](../clavinet-d6/README.md) · [Pianet mechanisms](../pianet-n/README.md)

The bank selector divides the pickups between TA20 and TA21, at C3/C♯3 (MIDI 48/49). The two preset rockers also allow foot contact A to ground the Pianet slider or B to ground the Clavinet slider. With both rockers forward both paths remain connected. With both back, the footswitch alternates the two branches. [Hohner Duo schematic](https://www.clavinet.com/Clavinet_Pianet_Duo_Schematic.pdf)

| Selector | F1–C3 | C♯3–E6 |
| --- | --- | --- |
| Preset, both rockers forward | Clavinet + Pianet | Clavinet + Pianet |
| Pianet treble | Clavinet | Pianet |
| Clavinet treble | Pianet | Clavinet |
| Both treble | Pianet | Clavinet + Pianet |

The regulation probe exposes the modeled contact events. At E4, velocity 80, the tangent first touches the string at 0.249 ms, the string reaches the anvil at 0.952 ms and the reed detaches at 3.481 ms. After key-up the string leaves the anvil at 1.440 ms and the reed reseats at 8.628 ms. At velocity 32 on the highest E the reed detaches at 1.939 ms, before the string reaches the anvil at 2.098 ms. At all seven pitches the simulated reed detaches at the same time for velocities 32, 80 and 120. A simultaneous recording of both mechanisms, with a measured key-motion reference, is needed to calibrate these predictions. [Regulation probe](examples/duo_regulation.rs)

## Electronics

The schematic connects two selectable string pickups through the transformer, transistor stages and four-section tone network. The series reed bank feeds its own BC550C stage. Two 100 kΩ level controls feed the separate recording terminals. Two 470 kΩ resistors and a 0.1 µF coupling capacitor feed the active mixed-output stage. The four audio transistor stages share the regulated supply. Branch leakage, mixer phase and the effects of loading follow from the electrical connections. The probe assumes a 1 MΩ load at each output. [Duo schematic](https://www.clavinet.com/Clavinet_Pianet_Duo_Schematic.pdf) · [Panel circuit](src/circuit.rs)

The solver uses small-signal transistor parameters at a nominal 9 V supply. The transformer estimate uses a 1:15 voltage ratio, 6 H primary inductance and 70 Ω/10 kΩ winding resistances. The model estimates 200 Ω per complete string pickup and 18 Ω plus 1.1 mH per reed coil. Bank selection scales the series impedance. These values, the slider taper, the reed polarity pattern and the large-signal headroom require measurements on an identified Duo. [Independent transistor netlist](research/duo.cir)

The ngspice comparison exercised 99 control combinations, two independent branch drives and all three outputs at ten frequencies. It covers all four pickup selections with all 16 tone combinations, the split/rocker/foot contacts and slider endpoints. Worst gain and phase differences were 0.00014 dB and 0.00222° against the netlist. Below 1 µV/V the largest absolute difference was 1.34 × 10⁻¹¹ V/V. This tests the circuit implementation against the nominal component model. [AC verification](research/circuit.py)

## Comparison with the owner recordings

ZeeByeZon posted separate Clavinet, Pianet and mixed Duo performances on 22 December 2012. Each clip has its own playing and gain. The source gives no pickup settings, levels, playing velocities, signal chain or common key-event timing. Comparison therefore uses harmonics relative to the fundamental of each identified pitch. [Owner’s index](https://fr.audiofanzine.com/piano-electrique/hohner/CLAVINET-PIANET-DUO/medias/audio/) · [Clavinet](https://medias.audiofanzine.com/audio/469901.mp3) · [Pianet](https://medias.audiofanzine.com/audio/469902.mp3) · [Mix](https://medias.audiofanzine.com/audio/469903.mp3)

| Owner recording | Identified MIDI notes | Windows | Median absolute H2–H6 difference | Median range across repeated windows |
| --- | --- | ---: | ---: | ---: |
| Clavinet | 41, 58 | 5 | 7.21 dB | 12.83 dB |
| Pianet | 29, 31, 33, 36, 39, 40, 41, 45, 76 | 13 | 7.76 dB | 3.08 dB |
| Mix | 37, 39, 41 | 8 | 6.19 dB | 5.52 dB |

The two lowest Pianet examples have a stronger H4 than H1, by 4.6 dB at F1 and 6.1 dB at G1. The model gives −34.7 and −31.0 dBc at those pitches. At E5, the owner recording and model both place H3–H6 below −39 dBc. In the mix, C♯2 has an H5 of −7.2 dBc versus −37.4 dBc in the model. Harmonic overlap from other notes, especially octaves in the low register, can survive the pitch test. The size of these differences and the variation between repeated Clavinet notes limit claims about the voicing. Isolated single notes at known velocities and switch settings would help separate pickup, regulation and playing effects.

Windows were chosen from the performances before comparing timbre. A 160 ms Hann window advanced in 40 ms steps across each recording. A harmonic pitch scan covered the instrument range, rejecting quiet windows and candidates with weak fundamentals. The retained excerpts are separated appearances for each pitch. For each excerpt the comparison independently refines the fundamental within ±35 cents and projects harmonics up to order 24 or 4.2 kHz, whichever comes first. Those harmonics explain 94.0–100.0% of weighted signal energy in the 26 retained windows. A good harmonic fit establishes periodicity, and the identity of the played key remains inferred. The timestamps are in the script. [Analysis and selection](research/recordings.py)

The model comparison uses velocity 80, both sliders full, preset rockers forward, C/A pickup and all tone tabs engaged. Its window is centered 160 ms after note-on. The table compares H2–H6 against H1 with a −40 dBc floor, taking the median of repeated observations at a pitch before aggregating the errors. The repeat range is the median of the per-harmonic ranges for pitches that recur. The CSV preserves individual unfloored ratios, pitch fits, and the model’s range over velocities 32/80/120 and three analysis times. The recordings do not identify key-down or key-up times, so they cannot establish attack offsets, damper timing or the mixer's inter-branch phase.

Each local listening file plays the first listed owner excerpt for a pitch, then the modeled terminal at the same pitch, in the table’s note order. Each excerpt lasts 320 ms. The pair is RMS-matched over the central window and receives a common peak reduction where necessary. The three-channel model probes preserve their original relative gain. [Clavinet pairs](../target/duo-probe/duo-clavinet-comparison.wav) · [Pianet pairs](../target/duo-probe/duo-pianet-comparison.wav) · [Mix pairs](../target/duo-probe/duo-mix-comparison.wav) · [Per-window CSV](../target/duo-probe/recording-harmonics.csv)

## Reproduce

From the workspace root, use a Python environment with NumPy, SciPy and SoundFile with MP3 decoding. The circuit comparison also requires ngspice.

```sh
cargo run --release -p hohner-duo --example duo_probe -- target/duo-probe
curl -fL 'https://medias.audiofanzine.com/audio/46990[1-3].mp3' -o 'target/duo-probe/46990#1.mp3'
python3 hohner-duo/research/recordings.py target/duo-probe
python3 hohner-duo/research/circuit.py
cargo run --release -p hohner-duo --example duo_regulation > target/duo-probe/regulation.csv
cargo run --release -p hohner-duo --example duo_probe -- --bench
```

The note captures use a common 44.1 kHz clock with note-on at 50 ms and key-up at 800 ms. `held-split.wav` holds C3/C♯3 across a registration change, releases and restrikes C3, and engages the string mute. `foot.wav` changes the foot contact and reveals both branches during a single held note. Four focused engine tests cover adjacent banks, leakage, restoring a held branch, foot contacts, mute, power, input ownership and range. The engine exposes eight host frames of latency. [Capture tool](examples/duo_probe.rs) · [Engine tests](src/lib.rs)

The two routing auditions play the same footswitch passage followed by the C3/C♯3 split. One uses the active mono mixer. The other places the Clavinet recording socket on the left and the Pianet socket on the right. A single playback gain preserves levels between sockets and passages. [Combined jack](../target/duo-probe/duo-routing-mix.wav) · [Stereo recording sockets](../target/duo-probe/duo-routing-stereo.wav)

The local release benchmark held 32 notes at 44.1, 48 and 96 kHz in 128-frame blocks. Mean processing used 59.9%, 73.7% and 73.4% of the block deadline, respectively, with no missed blocks. Holding all 60 notes used 120.3%, 129.9% and 136.3% and missed deadlines at every rate. The engine can render all 60 notes offline. Real-time polyphony depends on the host and machine.
