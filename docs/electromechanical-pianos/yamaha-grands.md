# Yamaha CP-70, CP-70B and the existing CP-80

Make the **CP-70B (April 1978, serial 5001 onward)** the first 73-key engineering reference. Its service manual covers the action, key tuning, pickup wiring, complete audio board and bench trims. Carry the **original 1976 CP-70** as a separate target. Yamaha lists different bass-string sets and TR boards for these two instruments. Electric Grand 80 already implements the 88-key CP-80 in this checkout and provides the regression reference. [Yamaha CP-70B service manual 006386, revised](https://anaphonic.com/wp-content/uploads/yamaha_cp70b_sch.pdf) · [Yamaha comparative parts list](https://manuals.fdiskc.com/tree/Yamaha/Yamaha%20Electric%20Grand%20Parts%20List%20for%20CP-70%20CP-70B%20CP-80.pdf) · [Existing CP-80 implementation and calibration](../../cp80/README.md)

## Build the 73-key scale

The playable CP-70B is in Electric Pianos. Its interactive String Action diagram follows the selected course. The probe generates the two revision tables, four note pairs, the pedal/restrike phrase and three pickup-to-jack transfers. The model passes the service-level electrical checks. Its action and string losses await recordings from an identified specimen. [Implementation, residuals and reproduction](../../cp80/research/cp70b/README.md)

1. Make separate CP-70 and CP-70B tables with physical key number, MIDI pitch, source part number, winding/gauge, strings per hammer, hammer group, damper and electrical pickup group. Map physical keys 1–73 to MIDI 28–100 (E1–E7). The service tuning chart numbers the same keys 8–80 on an 88-key scale. Add a fixture mapping physical key 42 to service key 49, MIDI 69 and 440 Hz. Yamaha explicitly applies the B tuning chart to the original CP-70 as well. [Service manual, printed p. 16](https://anaphonic.com/wp-content/uploads/yamaha_cp70b_sch.pdf)
2. Start with **F♯2/G2** (keys 15/16, one wound string to two) and **E4/F4** (37/38, wound pair to plain wire). A cropped CP-80 would put the wound/plain boundary at D4/E♭4. Check **A6/B♭6** (66/67) for the last fitted key damper. Use the model columns and part numbers for breaks even where the two 73-key versions share pitches. Derive speaking length, unison detune, hammer speed and loss from the selected specimen. The parts list alone gives none of those measurements. [Yamaha parts list, printed pp. 18–19, 28–31](https://manuals.fdiskc.com/tree/Yamaha/Yamaha%20Electric%20Grand%20Parts%20List%20for%20CP-70%20CP-70B%20CP-80.pdf)
3. Drive each course from a single moving hammer with finite contact area and unilateral force. Retain the oscillators between strikes and compute the string-end reaction for the piezo. Use the CP-80 offline collision and live modal implementation as the first candidate. Inspect its two-string limit, fixed contact width, rate breakpoints and MIDI-dependent voicing before putting model-specific dimensions behind the scale selection. Compare old CP-80 probe outputs before and after each extraction. [String and hammer solver](../../cp80/src/lib.rs) · [CP-80 scale and fit](../../cp80/src/scale.rs)

## CP-70B signal path and first proof

Sum piezo charge sources into the documented loaded network. Include the capacitance of the silent pickups. Transcribe the six stands, treble mixing resistors, FET front end, three tone controls, patch jacks, tremolo oscillator and optical branches, jack/line buffers and loads. Use both a stand number and a note index on the schematic transcription. The factory CP-80 deck includes a different pickup network and a brilliance switch. Share a circuit solver only after the variant decks independently reproduce their service data. [CP-70B wiring, TR board and overall circuit, printed pp. 9–14, 24–25](https://anaphonic.com/wp-content/uploads/yamaha_cp70b_sch.pdf) · [CP-80 reference deck](../../cp80/research/tr.cir)

| Deliverable | Independent check |
| --- | --- |
| A component deck and an offline transfer from each electrically distinct pickup branch | With the manual's VR1–VR6 initial settings and tremolo off, apply −32 dBm at 500 Hz to TR IN and trim P0 to −29 dBm. Follow its 50 Hz and 5 kHz jack/line tests with their specified loads and knob positions. Sample both tremolo envelopes. Record phase, level and terminal name. [Factory adjustment procedure, pp. 13–14](https://anaphonic.com/wp-content/uploads/yamaha_cp70b_sch.pdf) |
| Four dry note pairs and a pedal/restrike sequence | Capture the two construction boundaries, the damper boundary and a pair of neighboring notes across a mixing branch. Record patch send and both jacks at fixed gain. Fit soft and hard on pilot notes, reserve medium and at least one whole pitch in each construction for verification. Compare beat period, partial spacing, early/late decay, attack level and time to damp. One gain alignment per hardware take set preserves the dynamic range. |
| A playable 73-key engine | Interpolate only within documented construction groups. Check the original hardware's octave/tuning curve, hard treble aliasing against a finer offline integration, and a sustained full-keyboard burst against the host block deadline. Re-run the Electric Grand 80 comparison with the old default state. [Reference probe](../../cp80/examples/cp80_probe.rs) · [Held-out pitch comparison](../../cp80/research/validate_keys.py) |

## Original CP-70

The original prototype has its own wound-string inventory, individually wired pickup path and interactive bearing diagram. The comparison includes separate raw jack renders and an independent SPICE check of the estimated harness. [Original CP-70 implementation and evidence](../../cp80/research/cp70/README.md)

Inspect a 1976 instrument for hammer and bearing geometry and record the same pitches. Yamaha identifies an internal power supply for the original and an external adaptor for the B. The parts list identifies the original TR board as **NA 50 01 50**, the B as **NA 50 03 10** and the CP-80 as **NA 50 03 50**. Yamaha also lists the original piezo **BD 50 05 10** and its replacement **BD 50 00 20**, used on the B and 80. Identify replacement pickups before comparing capacitance or output. These identifiers give a technician a precise drawing or board to locate. [Yamaha instrument history](https://data.yamaha.com/files/download/brochure/3/323903/cp1_sb_en.pdf) · [Yamaha front-panel parts list](https://manuals.fdiskc.com/tree/Yamaha/Yamaha%20Electric%20Grand%20Parts%20List%20for%20CP-70%20CP-70B%20CP-80.pdf)

The outstanding item is a legible original-board schematic or a traced original board matched to a serial number. Resolve pickup grouping, tone network, gain, patch/output wiring and tremolo from that unit. Build the original deck and compare loaded sweeps and paired-channel tremolo at both ends of its controls. A/B the original and B with the same notes and output load. If their responses coincide within repeatability, share that verified block and keep the board and measurement records with the model.

## CP-80 follow-through

Use the existing model unchanged as the listening and numerical control during this work. Its native recordings support comparisons of attack spectra and early decay. Direct measurements of long unison beats, pickup capacitance, absolute dynamics, note-off and pedal response would benefit both branches. Capture one untouched 88-key baseline before sharing code, then rerun its historic pitch fixtures, jack/line routing and hard-treble reference. The existing plan and implementation record already cover its full signal chain. [CP-80 design](../cp80-model-plan.md) · [Calibration limits and worst-block timings](../../cp80/README.md)

## Installed revision comparison

The original CP-70 and CP-70B use the 73-key course layout, with single wound strings through F♯2, paired wound strings through E4 and plain pairs from F4. Yamaha lists different wound-string part numbers for the two revisions. The original model assigns individually wired piezos to a common capacitive load. The B uses the three pickup branches and two 4.7 kΩ mixing resistors on its service drawing. The original NA500150 TR board has yet to be traced. Its post-harness response currently uses the B board, and the individual lead capacitances and wound-string mechanical dimensions are estimates. [Original parts, repair photographs and input model](../../cp80/research/cp70/README.md) · [CP-70B factory wiring, tuning and adjustment checks](../../cp80/research/cp70b/README.md)

The CP-80 has 88 keys and changes from wound to plain pairs two semitones earlier, at E♭4. It has its own full tuning table, per-register hammer and string parameters, pickup assembly, input load and brilliance switching. At B6 and B♭5 the hammer/contact adjustments are tied to recorded strikes. Lengths, tensions, strike speeds, pickup capacitance and some unison settings remain estimates. The 73-key models borrow parts of the CP-80 loss calibration. [CP-80 scale, electronic board and recording comparisons](../../cp80/README.md)

The installed 18-model VST3 rendered 13 shared notes at two MIDI velocities per revision, at 48 kHz with stock controls. The table gives the median absolute difference between matching strikes. Attack RMS includes the stock output trim. The remaining columns compare ratios within each take. For example, a hard E4 had H2/H1 of −15.41 dB on the original, −15.66 dB on the B and −22.87 dB on the CP-80. A6 and B♭6 straddle the damper boundary in all three renders. On the CP-70B, key-up reduced the respective 80 ms windows by 105.34 and 7.35 dB. These figures describe the models. A serial-matched hardware recording would be needed to assign the same figures to an instrument.

| Revisions compared | Attack RMS | H2/H1 | H3/H1 | After/before release |
| --- | ---: | ---: | ---: | ---: |
| CP-70 and CP-70B | 0.91 dB | 1.68 dB | 2.09 dB | 0.08 dB |
| CP-70B and CP-80 | 4.77 dB | 9.36 dB | 4.36 dB | 0.70 dB |
| CP-70 and CP-80 | 1.65 dB | 9.14 dB | 5.66 dB | 0.81 dB |

The probe writes all 78 measurements and a single-gain listening file. Within each instrument the clip plays F♯2, E4, A6 and B♭6. The order is CP-70, CP-70B, CP-80. [Installed comparison script](../../openwurli-ui/research/grand_revisions.py)

```sh
python3 openwurli-ui/research/grand_revisions.py "$HOME/Library/Audio/Plug-Ins/VST3/Electric Pianos.vst3" target/grand-revision-audit
```
