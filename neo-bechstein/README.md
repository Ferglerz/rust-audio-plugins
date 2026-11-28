# Neo-Bechstein

The model launches a light, felt-tipped striker from the primary hammer. The striker leaves the primary at its stop, crosses the remaining clearance under gravity and rebounds from the vibrating string. Eighteen magnetic heads feed the Siemens ELV413. The output is voltage across its 6 Ω speaker terminals. The dedicated editor shows the two hammers, damper, felt rail and pickup for the last key played. [Nernst, figure 2](https://patents.google.com/patent/US1988564A/en) · [1933 eyewitness account and overhead photograph](https://r-type.org/timeline/time-259.htm) · [Amplifier drawing, measurements and nonlinear validation](research/amplifier.md)

The native keyboard spans A0–C8. **Extended Notes** enables MIDI 0–127. Virtual courses use the closest native geometry at their own pitch. The **Pickups** control offsets the nominal 2.4 mm magnetic gap by ±1 mm. **Damper pedal** (CC64) lifts the key dampers. **Felt rail** shortens vibration even on held keys. **Swell** (CC11) varies a capacitor in the amplifier. The twelve **Amplifier step** positions set its subsequent gain.

## Which measurements constrain the instrument

| Part | Evidence and implementation | Remaining uncertainty |
| --- | --- | --- |
| Action | A Berlin player described a secondary hammer near one twentieth the primary mass. Nernst illustrates a felt-capped pin sliding in a guide, a hard stop, a free flight and an independent adjustable felt edge. The model resolves two unilateral contacts and preserves string motion across restrikes. | Individual masses, launch gap, contact stiffness and playing velocity have no surviving measurements in these sources. |
| Strings | The eyewitness reports paired strings in part of the middle register and a bass sustain of roughly one minute. The model uses a dispersive course and independent modal loss in both the felt and the free string. | MIDI 45–87 pairing, lengths, tensions, loss curves and the treble hammer graduation are estimates. |
| Pickup heads | Wittig describes 18 series-connected heads and six shunt capacitors. The model combines the motion of strings under each common pole, differentiates linked flux, and loads the series network at E1. The two final four-key heads follow a second documented instrument. | The individual inductances, pickup positions, cap values, exact cap grouping and magnet sensitivities have yet to be measured. The installation assumes 16 groups of five and two of four. [Foundation instrument, serial 138994](https://www.carl-bechstein-stiftung.de/instrumentensammlung/details/neo-bechstein-138994/) |
| Regulation | A four-key C chord (MIDI 48, 55, 60, 64 at velocity 100) is set to about 100 mV RMS at E1 from 20 to 120 ms. The amplifier's published sensitivity at step 9 is 100 mV. | The choice of chord, velocity and window is a reproducible gain reference. Its original keyboard voltage is unknown. |
| Electronics | The Vienna ELV413 circuit and digitized 34-point response constrain the gain stages, the transformer-coupled RE604 pair and its load. High-drive checks include grid current and driver loading. | Most component values were omitted from the tracing. The speaker cabinet, room and recording chain are outside the terminal model. [Electrical study](research/amplifier.md) |

The historic models have different signal paths. The Storytone reads its magnetized strings into two transformer primaries and four 2A3 output valves. The Neo-Bechstein sums eighteen heads into six frequency-shaped sections and drives the ELV413 with two RE604s. The shared string solver handles the same physics in both instruments. Only the Neo-Bechstein has the secondary hammer in free flight and a separately engageable held-string felt rail. [Storytone circuit and action](../storytone/README.md)

## John Hunt's 1933 recording

HMV C.2567 includes Debussy's *Clair de lune* played by John Hunt. A contemporary review identifies the instrument and the disc. We compared a transfer of the opening F4/A♭4 dyad, the following G♭4/A4 dyad, and the first return to F4/A♭4. The four fundamentals sit about a quarter-semitone below A440 on the disc. Each harmonic comparison tracks the fundamental in its own recording. [Contemporary disc listing, July 1933](https://worldradiohistory.com/UK/Wireless-Magazine/Wireless-Magazine-1933-07-S-OCR.pdf) · [Transfer used in the comparison](https://www.youtube.com/watch?v=aDFIqZEKkzo)

The first dyad was used to select two terms of the microhammer's felt compression law. A 21-setting sweep varied the linear and quadratic stiffness at common pickup sensitivity, with the same two strikes and the same modeled board. Most of the original spectral error came from using a hard, brief contact. The selected midrange linear term is 23 N/m. The quadratic term rises from 55,200 N/m² at the bass end with register. Above A4 the stiffness grows faster to shorten contact with the decreasing string period. Only the middle-register settings have this recording check.

| Disc onset | Use | Harmonics compared | Median absolute difference, H2–H5 relative to H1 | Median using only harmonics above the comparison floor |
| --- | --- | ---: | ---: | ---: |
| 2.722 s, F4 and A♭4 | First dyad, fit | 8 | 5.52 dB | 6.54 dB |
| 6.225 s, G♭4 and A4 | Different pitches, held out | 7 | 1.67 dB | 2.37 dB |
| 9.357 s, F4 and A♭4 | Later repetition, held out | 6 | 1.77 dB | 2.00 dB |

Measurements use a 160 ms Hann window beginning 50 ms after each estimated onset. The modeled pair is played together at velocity 80 in the stock pedal state. The comparison excludes partials that overlap another identified held note, and floors both spectra at −38 dBc. Agreement at the floor only gives an upper bound on a weak partial. The principal mismatch is F4's second harmonic, 17.1 and 18.8 dB weaker in the model on the two appearances. The pianist's velocities and pedal motion, the original amp and the transfer chain are unknown. These observations constrain the signal reaching the listener. The recording cannot determine a unique hammer compliance, pickup position or capacitor set.

```sh
cargo run --release -p neo-bechstein --example notes -- target/neo-bechstein/regulated 48000 65+68 66+69
cargo run --release -p neo-bechstein --example hammer -- target/neo-bechstein/hammer
cargo run --release -p neo-bechstein --example regulation
python3 neo-bechstein/research/recording.py /path/to/hmv-c2567.webm target/neo-bechstein/regulated
cargo test -p neo-bechstein --quiet
```

The analysis checks the Opus transfer's SHA-256 before decoding. The note fixture stores first-key head EMF, loaded amplifier input and speaker output on three synchronous channels. All are in volts. [Recording analysis](research/recording.py) · [Measured harmonic differences](../target/neo-bechstein/hmv-c2567.csv)
