# Original Yamaha CP-70, 1976

The CP-70 selection in Electric Pianos uses the original wound-course inventory and an individually wired piezo harness. Its diagram shows the bearing, piezo and leads to the input board. The audio section after the harness uses a provisional TR response. Yamaha introduced the CP-70B in 1978. [Yamaha museum](https://www.yamaha.com/en/about/experience/innovation-road/collection/detail/2117/) · [Yamaha CP history](https://data.yamaha.com/files/download/brochure/3/323903/cp1_sb_en.pdf)

## Factory and instrument evidence

| Part | Original CP-70 | CP-70B |
| --- | --- | --- |
| Wound strings, E1–E4 | Set NB500880, individuals NB500890–NB501470 | NB504030–NB504610 |
| Piezo | BD500510, later serviced with BD500020. Individual wired connections visible at the hitch-pin bearings. | BD500020. The service drawing groups pickups on three branches serving 63, 2 and 8 keys. |
| Panel | TR NA500150, separate DC NA500170 and line-transformer LT NA500180 boards | TR NA500310, external supply |
| Optical driver and cells | 2SC1345(D) and MCD527, iK000150 | 2SC458(C) and later MCD527, iK000180 |

The comparative parts list identifies the same 2SK30A-Y input device, dual 4558 op amps, three EQ pot types, two-gang volume, speed and depth controls and 600:600 Ω line transformer across the two revisions. Original pickups or service replacements may be present on surviving instruments. [Yamaha parts list, electronic and frame sections](https://manuals.fdiskc.com/tree/Yamaha/Yamaha%20Electric%20Grand%20Parts%20List%20for%20CP-70%20CP-70B%20CP-80.pdf)

VIRA's photographs show the original's open pickups beside the hitch pins, multicoloured individual wires and numbered terminal rows. Their repair log follows the lead of one distorted middle G to a faulty board connection. The Yamaha patent drawings of the earlier pickup construction show a string bearing on the crest of a cantilever metal channel and the ceramic element under the upper arm. Figure 2 gives the relationship to the hitch pin and bridge. The identification of the photographed assembly with the earlier construction is an inference from these features. [Original harp and board photographs](https://revivedpiano.com/cp70cp80) · [Technician's individual pickup diagnosis](https://pianoshokunin.seesaa.net/article/201706article_2718.html) · [Yamaha US4189969A, Figs. 1–2](https://patents.google.com/patent/US4189969A/en)

## Working circuit and scale

There is one force source per key. The harness approximation sums 73 equally loaded piezos. It assumes 1 nF per ceramic, 40 pF per lead, 100 nF series coupling and a 470 kΩ gate return. With identical branches their charges sum before the single filter, including the capacitances of all resting sensors. This models the individually wired pickups, while the B simulation uses its two 4.7 kΩ mixing resistors and three different transfers. The Rust filter follows the bilinear transform of the common capacitive source and gate load. A SPICE check excites one sensor with the other 72 grounded through their ceramic capacitances. Its level and phase residuals against the rendered impulse were 0.00001 dB and 0.00081° from 20 Hz to 20 kHz. The residuals test the equation against the assumed circuit. [SPICE equivalent](harness.cir) · [Live input implementation](../../src/seventy_pickup.rs)

The output uses the later TR component values, optical curve and unbalanced jack load as estimates. The legible B service drawing and the original parts inventory establish the shared devices and controls. The original board's passive components, weighting of individual keys, per-key capacitance, absolute transduction and stereo tremolo gain have yet to be measured. The available original manual photographs identify the manual but do not resolve these values. The original's internal AC supply and separate LT board are documented as hardware features. The software renders the pair of unbalanced jacks. [Original service manual photographs](https://reverb.com/en-no/item/75213312-yamaha-cp-70-service-manual) · [CP-70B TR service drawing](https://anaphonic.com/wp-content/uploads/yamaha_cp70b_sch.pdf)

The models share Yamaha's stretch chart, E1–E7 compass, single-to-double break at G2, plain-wire transition at F4 and common hammer groups. The original selects its own wound parts per key. Its speaking lengths, wound core and winding mass, unison detuning, hammer contact law and loss envelopes still use the same working values as the B. The original plate and string routing differ visibly in the technician's photographs. A micrometer reading and speaking length for each original pilot note, and matched soft/hard/held/pedal releases, would determine which of those shared values need to change. [Yamaha tuning chart, service manual p. 16](https://anaphonic.com/wp-content/uploads/yamaha_cp70b_sch.pdf) · [Original frame photographs](https://revivedpiano.com/cp70cp80)

## Reproduce the comparison

From the workspace root, run the following commands. The energy comparison uses raw modeled jack signals at equal force, before any plugin output trim.

```sh
cargo run -p cp80 --release --example cp70b_probe -- target/cp70-proof original
cargo run -p cp80 --release --example cp70b_probe -- target/cp70-proof/b-control
python3 cp80/research/cp70/compare.py target/cp70-proof
```

Each directory contains a boundary-note WAV, stereo pedal and restrike phrase, and a 73-key table per revision. The original's stand and bus columns are blank because the numbered B groupings belong to the B harness. Read the model differences as predictions. At velocity 0.68, original/B RMS over the first 25–125 ms was −0.19 dB at F♯2, −0.10 dB at E4, −8.06 dB at G♯6 and −12.25 dB at A6. The regenerated B boundary WAV matched the pre-change file byte for byte, SHA-256 `864c1d05d5d9948108034d1b55a4c1d4b9ec6e6650c684ee747aeae482b30ef4`. A direct recording from an identified original at matched pickup or panel settings is still needed to establish the acoustic error.

Electric Pianos gives the original a +1.89 dB trim. The native and extended single-note and performance-chord sweep peaked at −1.004 dBFS across 44.1, 48 and 96 kHz. All 73 keys struck together at maximum velocity reached approximately +0.75 dBFS. At all three rates the installed CLAP and VST3 played the original. E1 and E7 sounded at the VST3 jack, and the adjacent out-of-range MIDI notes returned silence. Switching back to CP-70B reproduced its preceding note bit for bit. The CP-80 E4 regression also remained byte-identical. The original/B revision is stored separately from the two existing host automation endpoints.
