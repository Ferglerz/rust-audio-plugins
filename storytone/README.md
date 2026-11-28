# Story & Clark / RCA Storytone

The 88-key electric spinet model takes a felt hammer strike through a magnetized steel string, its individual pickup, a two-input transformer and the RCA tube amplifier. The mono output is the auxiliary connection in the 1940 service article. The illustrated cabinet has a direct speaker opening on the left and a folded exponential labyrinth opening on the right. The electrical rendering ends at the secondary of the output transformer, with both the speaker load and auxiliary divider connected. A microphone recording also includes the cabinet, room and possible speaker-to-string feedback. [Radio-Craft, February 1940, pp. 469, 490–491, figures 1–3](https://archive.org/details/sim_electronics-now_1940-02_11_8/page/469/mode/1up) · [National Music Centre, 1941 specimen](https://collections.nmc.ca/objects/320/story--clark-storytone) · [National Music Museum, object 15733](https://emuseum.nmmusd.org/objects/80661/storytone)

## Action and pickup

Pressing a key launches a moving hammer into a dispersive string. The shared struck-string solver integrates compression-only felt contact at ≥176.4 kHz, then advances the free modes at the host rate. Strike position, felt width and the spatial extent of the pickup enter each mode separately. The CP-80 uses the same mechanics with bridge-force readout. This instrument integrates string displacement over an effective magnetic aperture. Its pickup voltage is the time derivative of linked flux. A signed per-note voicing factor represents the serviceman’s procedure of magnetizing a string along or against the existing field. Figure 2 shows the coil on a soft-iron pole, a separate permanent magnet and their common mounting plate. [1940 service article, figure 2](https://archive.org/details/sim_electronics-now_1940-02_11_8/page/490/mode/1up)

The eight physical plates and their possible capacities of 8, 10 or 16 pickups are documented. The assumed plate order is 8/10/10/16 on each of two 44-note runs. The actual grouping, signs and full compass stringing require inspection. A single equivalent course per key covers A0–C8. Speaking lengths run from 1.09 m at A0 to 0.69 m at C♯3 and 48 mm at C8, with an assumed wound/plain transition at C♯3. Tension (560 N), stiffness, high-frequency loss, unison count, felt compliance, cutoff above E6 for dampers, and magnet strength are scale estimates. The soft pedal changes the next hammer speed by a factor of 0.62. Sustain raises the modeled dampers. The three-pedal layout is documented on the museum specimen. [National Music Museum](https://emuseum.nmmusd.org/objects/80661/storytone) · [Piano Technicians Guild Foundation, 36-inch 1939 spinet](https://ptgfoundation.org/museum-collection/1939-story-clark-electric-piano/) · [1939 owner instrument and 88-note inventory](https://www.ebay.com/itm/285983360821)

A5 and D6 from the owner performance below constrained a shared upper-register hammer and pickup geometry. The fitted effective hammer mass is 0.45× and felt stiffness is 2× the initial scale above F♯5, with continuous interpolation from E♭3. The fitted aperture is 30 mm wide, centered 60 mm from the bearing or 40% of the speaking length on the shortest courses. These parameters can trade off against each other and against cabinet response. At three candidate key velocities (0.45, 0.60, 0.75), the pickup’s mean absolute H2/H3 error was 6.23 dB on A5/D6. B5 was withheld. Its error was 9.27 dB over the same velocities against the first B5 window. [Parameter search](research/contact.py) · [Mechanical sweep](examples/contact_probe.rs)

## Amplifier

The period circuit has an installer’s optimum-volume screwdriver adjustment at the 6J7 grid, a manual volume at the keyboard, and an electrical swell pedal. At rest the pedal takes the signal from the manual wiper. At full travel it takes the AC signal at C4, bypassing the manual attenuation. The optional external radio/phono input has its own tone rheostat. Its source jack is modeled open, with the R13/R12/C6 loading branch set at mid travel. [Schematic and control description](https://www.worldradiohistory.com/Archive-Radio-Craft/1940s/Radio-Craft-1940-02.pdf#page=23)

| Stage | Reconstructed from the service sheet | Estimated or fitted |
| --- | --- | --- |
| T1 and 6J7 | Dual inputs, 0.36/0.49 Ω primary and 670 Ω secondary winding resistances, 250 kΩ optimum pot, cathode and screen networks, 220 kΩ plate load | 35× ratio, 8 Ω and 12 mH per pickup run, core inductance, tube slopes and winding capacitance |
| 6C5 and T2 | Manual and swell pots, coupling and bypass capacitors, 820 Ω cathode resistor, 2.2 kΩ primary and 1.6 kΩ total secondary copper, center tap, grid shunts and snubber | 0.85× half-secondary turns, 40 H magnetizing inductance and high-frequency parasitics |
| Four 2A3 and T3 | Parallel pairs in push-pull, 127 Ω full primary, 0.6 Ω secondary, plate capacitors, 15 Ω nominal load | Effective 4.79× half-primary ratio, magnetizing inductance and nonlinear plate-current characteristic |
| Auxiliary | Figure 3 places 3.9 kΩ and 220 Ω across the 15 Ω speaker circuit. The electrical monitor sees 220/4120 of the loaded speaker voltage. | An amplifier receiving that signal is assumed to have high input impedance. |

The service table gives 60 V/1 mA at the 6J7 plate, 300 V/7.5 mA at the 6C5 and 300 V/40 mA per 2A3. The AC circuit uses these operating points. The preamp and driver use local small-signal models. For each output pair, the nonlinear plate current follows its grid and its instantaneous plate voltage with a Child–Langmuir exponent and an estimated 30 V collection knee. Each new sample solves both pair currents together with the reflected load. The 2A3 amplification factor of 4.2 comes from RCA. The local 4.6 mS slope is estimated at the service bias. The rectified supply is held at its filtered operating voltage. The output load is resistive. Positive-grid driver current, supply sag, transformer saturation, diaphragm impedance and acoustic radiation require additional hardware evidence. [RCA 2A3 specification](https://www.r-type.org/pdfs/2a3.pdf) · [Service voltages and winding markings](https://www.worldradiohistory.com/Archive-Radio-Craft/1940s/Radio-Craft-1940-02.pdf#page=23)

A separate ngspice deck uses original-side windings and controlled voltage/current sources. Across seven optimum/manual/swell settings and 27.5 Hz–14 kHz, the small-signal sweep matched the Rust impulse response within 0.000006 dB and 0.00041° after bilinear frequency mapping. A 400 Hz input into one primary with all controls full produced the results below. The effective T3 ratio was fitted to the published 15.8 W at 2% RMA distortion. The two numerical models validate the circuit implementation. An original chassis sweep and harmonic spectrum would independently test its physical parameters. [Independent netlist](research/rca.cir) · [Comparison driver](research/circuit.py)

| Input, mV peak | Speaker fundamental, V peak | Rust THD through H7 | ngspice THD through H7 |
| ---: | ---: | ---: | ---: |
| 0.20 | 0.7335 | 0.0010% | 0.0010% |
| 1.00 | 3.6652 | 0.0245% | 0.0245% |
| 4.00 | 14.4533 | 0.5809% | 0.5811% |
| 6.28 | 21.8405 (15.90 W into 15 Ω) | 2.0036% | 2.0049% |
| 8.00 | 27.3515 | 1.9489% | 1.9500% |

## Recording and pedal checks

SynthDan1 identifies the piano in his demonstration as a 1939 Storytone. The recording contains musical phrases, accompaniment, room sound and the speaker system. Each row gives two 120 ms Hann-window comparisons. Model columns use the auxiliary output at velocity 0.60, 120–240 ms after note-on. The direct model fundamentals were within 4.2 cents of the room peaks on those windows. Different playing levels and unknown strike ages limit the harmonic comparison. The E4 second harmonic can coincide with another played E. The independent B5 has 8–12 dB more second harmonic and 12–26 dB more third harmonic in the model. D6’s model H3 is within 2 dB of both room observations. [Original owner performance](https://www.youtube.com/watch?v=OINs12sXRK0) · [Window and spectral measurement script](research/recordings.py)

| Note | Recording window starts (s) | Room H2/H1 (dB) | Model H2/H1 | Room H3/H1 (dB) | Model H3/H1 |
| --- | --- | --- | ---: | --- | ---: |
| E4 | 25.70, 41.15 | +4.8, −4.9 | −4.3 | −4.0, −3.8 | −7.2 |
| A5 | 26.30, 44.49 | −31.6, −34.9 | −16.7 | −32.9, −29.8 | −30.8 |
| B5 | 46.88, 47.88 | −26.3, −30.5 | −18.1 | −38.6, −52.3 | −26.1 |
| D6 | 10.16, 47.56 | −26.3, −31.2 | −20.5 | −37.9, −35.2 | −37.0 |

At model velocity 0.60, the pickup first reached 1% of its attack peak after 1.25, 0.67, 0.65 and 0.71 ms on A0/D3/E4/A5. After note-off the first 10 ms block at least 20 dB below the preceding RMS began at 40, 40, 30 and 30 ms. C7 remained ringing beyond the 240 ms measurement. The musical recording has no synchronized contact or key-up signal. Dry key and pickup recordings are needed to validate those times or the assumed damper cutoff. On held notes the swell changes the electrical output while leaving the pickup voltage bit-identical. Sustain holds the mechanical ring after note-off, and soft reduces the next impulse. Tests exercise all 88 keys, pickup polarity, these pedal distinctions, restrikes, simultaneous audition/MIDI ownership, overdrive and silence. [Physical tests](tests/physical.rs)

## Host and reproduction

Construct `engine::Engine::new(sample_rate)` once per sample rate. `note_on`, `note_off`, `audition`, `audition_off`, `held_notes`, `choke`, `set_soft`, `set_sustain` and `set_controls` operate on preallocated state. `render(&mut [f32])` writes raw auxiliary volts. `next_outputs()` returns each plate EMF, both pickup input EMFs, 6J7 grid voltage, 15 Ω speaker-terminal voltage and auxiliary voltage in f64. Each observation comes from the same solver step. There is zero block buffering. String travel still contributes physical onset delay. Unchanged circuit controls reuse their compiled state matrices.

Extended Notes optionally enables MIDI 0–127 at the requested pitch. Each added key inherits the nearest factory course geometry, pickup plate, damper behaviour and contact law. These extensions have no historical specimen counterpart. Standalone `set_extended(false)` fades active extended keys in 4 ms and preserves native keys. `set_pickup_gap_mm(offset)` moves the effective magnetic gap from its nominal 3 mm by −0.25…+2.0 mm. Wider spacing reduces the voltage gradient and curvature of the flux–displacement curve. Gap movement rebases the flux history at the current displacement, keeping the string motion continuous.

In a dedicated release run with 128-frame blocks, simultaneous full-velocity centered notes gave the following wall-clock percentages of the deadline. The machine runs other work. A second run concurrent with rendering and compilation reached 147% p99 and 213% worst for the 88-key, 96 kHz stress case.

| Keys | 44.1 kHz p99 | 48 kHz p99 | 96 kHz p99 | 96 kHz worst | 48 kHz auxiliary peak |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1.65% | 1.97% | 3.47% | 4.31% | 0.127 V |
| 8 | 3.78% | 3.24% | 6.84% | 11.10% | 0.916 V |
| 32 | 7.19% | 6.49% | 12.76% | 28.48% | 1.871 V |
| 88 | 20.72% | 28.61% | 42.96% | 81.30% | 1.893 V |

Run the commands from the workspace root. The notes probe writes 15 notes × three velocities as four-channel float WAVs in pickup/grid/speaker/auxiliary order, plus a mono chord audition. Raw electrical volts can exceed digital full scale. The Python tools use NumPy and SciPy. The circuit check also requires ngspice. For the room comparison, decode the owner recording to mono PCM at 48 kHz. The compressed audio used here had SHA-256 prefix `05dde5e7d214f3c8`.

```sh
cargo check -p storytone --all-targets
cargo test -p storytone --quiet
cargo run --release -p storytone --example storytone_probe -- notes target/storytone-proof
cargo run --release -p storytone --example storytone_probe -- bench
python3 storytone/research/circuit.py
python3 storytone/research/recordings.py /path/to/1939-performance.wav target/storytone-proof
cargo run --release -p storytone --example contact_probe > target/storytone-contacts.csv
python3 storytone/research/contact.py target/storytone-contacts.csv
```
