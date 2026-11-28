# Hohner Electra-Piano

C3, F4 and G♯5 (MIDI 48, 65 and 80) have independent suspended fork models. The probe writes both prong displacements, coil EMF, additional-amplifier jack voltage and speaker-terminal voltage. Hohner’s six-octave console uses a keyboard-operated rebound hammer, an asymmetric fork per key, one inductively sensed prong, seven groups of series pickups and a four-speaker amplifier. A technician’s photos of a 1968 unit show the rail, damper mechanism, hammers and return hardware. [Hohner service guide, pp. 2–8, drawing 680209](https://www.clavinet.com/hohner_electrapiano_service_manual.pdf) · [HonkLabs restoration](https://honklabs.com/hohner/piano-electra/hohner-piano-electra-part-2/)

The real-time engine covers the native 72 notes. It uses the three reference forks and interpolates or extrapolates the remaining keys. Seven additional notes have owner-recording comparisons, with the largest differences in the bass harmonics and treble decay. The two electrical taps remain independently accessible. [Host API, full-scale measurements and CPU timings](research/live.md)

## Render and listen

```sh
cargo run --release -p electra-piano -- notes target/electra-proof
```

`three-key-audition.wav` is a six-second mono presentation of the auxiliary jack at one gain of 64 digital units per volt. Each key rings under the right pedal, the left pedal engages during the ring and lifting the right pedal damps it. The 36 individual five-channel float WAVs preserve physical units and gain. Channel order is sensed-arm motion (m), return-arm tip motion (m), winding-signed pickup EMF (V), auxiliary jack (V into 1 MΩ), and speaker terminals (V into 4 Ω). The probe uses 48 kHz. Hammer impact speeds are 0.35, 0.85 and 1.7 m/s.

| Sequence | Events relative to the strike |
| --- | --- |
| hold | Ring for two seconds |
| release | Release key at 500 ms |
| pedals | Right pedal down at 354.2 ms, release key at 500 ms, left pedal down at 625 ms, right pedal up at 927.1 ms |
| restrike | Release at 354.2 ms, strike the moving fork at 372.9 ms |

## Action and pickup

The two arm-tip masses share a spring representing compliant root motion. Diagonalizing the mass-normalized stiffness matrix gives two independent normal coordinates. Their projections determine both tips’ displacements. The lighter arm also contributes the second and third uniform cantilever bending shapes. A unilateral Hertz contact applies the same force at the assumed 55% strike line across the modes and accelerates the rebounding hammer. A new strike preserves the fork's motion. The right pedal lifts a six-millisecond damper ramp. Motion at the 91% pickup line passes through an offset finite-width field. Differencing flux linkage produces coil EMF. Contact, induction and the sixth-order output anti-alias filter run at four times the host rate. [Implementation and parameter values](src/fork.rs)

| Quantity | C3 | F4 | G♯5 | Basis |
| --- | ---: | ---: | ---: | --- |
| Struck / return effective mass (g) | 4 / 20 | 1.8 / 10 | 0.5 / 4 | Estimated |
| Hammer mass (g) | 6 | 4.5 | 2.2 | Estimated |
| Effective magnetic field width (mm) | 0.82 | 0.64 | 0.71 | Fit to harmonic ratios at assumed 0.85 m/s |
| Free fundamental T60 (s) | 6.8 | 5.0 | 1.7 | Fit to early electrical decay |
| Damped T60 (s) | 0.15 | 0.11 | 0.08 | Estimated |

Actual fork dimensions, contact stiffness, support compliance, strike and pickup positions, magnet signs and absolute flux scale remain unmeasured. Fundamental tuning uses equal temperament at A4 = 440 Hz. Each fork has a separate magnet polarity parameter. Neighboring coil windings alternate independently of the assumed common magnet orientation. The second arm's resonance follows its mass, stiffness and coupling. A second closely spaced frequency has no measured basis on these three notes.

## Electrical trace

The seven pickup groups form a series chain. Each group has a series RL winding equivalent shunted by its 100 Ω balance trim. The left pedal inserts P8 (5 kΩ). The P9 basic-volume pot (5 kΩ) drives transformer 620620. The secondary feeds a 100 kΩ log pot. Its wiper feeds both the external-amplifier socket and T1 through 1 µF. T1–T3, the bootstrapped BC140/AD149 output stage, the 2,500 µF output capacitor, and the 1 kΩ / 300 pF feedback to T2 drive the 4 Ω load. The four speakers are two series pairs in parallel. The default circuit uses the speaker-connected jack position. Cabinet radiation, speaker reactance, power-stage clipping and mains sag require further data. [Rust circuit](src/circuit.rs) · [ngspice transistor deck](research/680209.cir)

Hohner labels F1–D2 and F6–E7 on the drawing. Splitting the intervening 50 keys into five equal groups of ten is provisional. All coil/transformer parameters are bench estimates. Each coil has 8 Ω and 0.5 mH. The transformer has a 1:7 voltage ratio, 0.8 H primary magnetization, 35 Ω primary and 250 Ω secondary copper resistance, referred in one series element. Supply is 35 V. Bias was set to a 17.5 V midpoint and 35 mA upper power-transistor collector current on generic silicon/germanium device cards. Full knob travel uses a 100:1 exponential law. The service guide identifies the devices and controls but gives no specimen operating point, leakage inductance, pot taper or measured winding impedance.

The Rust trapezoidal solver accepts simultaneous voltage drives on all seven groups and reads both output nodes. ngspice operates on a separate primary-side pickup circuit, ideal transformer, nonlinear BJT models and their DC operating point. Comparing impulsive transfer to the bilinear images of 35 Hz–14 kHz across three drive groups, quarter/half/full volume and both left-pedal states gives maximum disagreement of 0.000003 dB and 0.000012°. This verifies the discretization and circuit implementation against the assumed netlist. A bench sweep on a physical 680209 unit is still needed.

```sh
python3 electra-piano/research/reference.py
```

The script requires ngspice, NumPy and SciPy. Its circuit values and the component trace can be reviewed independently. [Sweep and transformer fixture](research/reference.py)

## Recorded comparison

An identified Hohner console played through a Peavey Valverb with effects and reverb off supplies six comparisons. Three notes were selected because their early harmonic components and held fundamentals could be isolated in that performance. For each note, one take informed field width and loss and a second take was reserved. Hardware impact speed, P1–P9, recording gain, loading and individual key-up times are unknown. The comparison uses the auxiliary jack at the same modeled 0.85 m/s for all takes. Relative harmonic and decay measurements require no gain alignment. [Effects-off hardware performance](https://www.youtube.com/watch?v=84P3RGgeqvU)

| Key and take | Recording onset (s) | F0 hardware / model (Hz) | H2/H1 hardware / model (dB) | H3/H1 hardware / model (dB) | Early slope hardware / model (dB/s) |
| --- | ---: | ---: | ---: | ---: | ---: |
| C3 fit | 297.015 | 130.83 / 130.81 | −7.03 / −6.91 | −5.70 / −5.63 | −6.51 / −6.74 |
| C3 held out | 241.892 | 130.75 / 130.81 | −7.20 / −6.91 | −6.55 / −5.63 | −6.46 / −6.39 |
| F4 fit | 875.327 | 347.87 / 349.23 | −15.17 / −14.59 | −13.63 / −13.26 | −11.24 / −10.67 |
| F4 held out | 149.688 | 348.14 / 349.23 | −14.65 / −14.59 | −13.11 / −13.26 | −10.31 / −10.35 |
| G♯5 fit | 663.886 | 827.27 / 830.61 | −30.11 / −27.87 | −27.64 / −27.84 | −35.17 / −34.93 |
| G♯5 held out | 664.757 | 827.21 / 830.61 | −27.05 / −27.87 | −25.96 / −27.84 | −34.60 / −34.78 |

Attack ratios use a 65 ms Hann window beginning 20 ms after the detected onset. Decay slopes follow the fundamental through the isolated part of each note. The F4/G♯5 fundamentals run 5–7 cents flat on this recording. At 1.7 m/s the model’s C3 H2/H1 rises to +5.5 dB and H3/H1 to +4.7 dB. The recording lacks a measured loud-strike series to establish whether that excursion is correct. The subjectively reported driven-amp bite also needs a large-signal amp model. [Technician’s account of the amplifier](https://honklabs.com/hohner/piano-electra/hohner-piano-electra-part-1/)

Olivier Grall identifies a separate 1968 instrument. Its three C3 ring windows at 52.39, 53.31 and 55.54 s gave H2/H1 of −7.6 to −7.0 dB and H3/H1 of −5.7 to −4.8 dB. Model ring windows 80–180 ms after impact gave −10.2 to −8.2 dB and −8.6 to −6.8 dB. The preceding chord and unknown contact time limit this check to a spectral comparison. [Grall’s 1968 console recording](https://www.youtube.com/watch?v=cMarNNEoMes)

To reproduce, decode each source from its original audio stream to mono at 48 kHz with ffmpeg. Format 251 payload SHA-256 prefixes for the reported comparison were `478e8351b82494ec` (Peavey) and `ef08f52e498b53bc` (Grall). NumPy and SoundFile are required. Audio from the recordings stays outside the repository.

```sh
python3 electra-piano/research/recordings.py target/electra-proof /path/to/peavey.wav --grall /path/to/grall.wav
```

On the synthetic 0.85 m/s release, auxiliary output reaches 20 dB below a held control in 52.5, 37.5 and 25 ms in register order. The right pedal preserves both moving arms after key-up. The left pedal then changes settled jack level by −12.8, −10.7 and −10.0 dB while both arms and the open-circuit pickup remain sample-identical. The recordings do not establish either pedal’s exact motion or timing. A dated, direct-jack, velocity-matched, pedal-annotated capture would test these predictions.

## Measurements for the complete scale

Measure two-arm motion, pickup clearance, contact speed and damper return on these keys plus both sides of each confirmed construction or coil-group change. Calibrate the unsensed prong against motion or an isolated inharmonic resonance, and reserve at least one new key per register. Check P1–P9, transformer ratio and impedance with the specified output load. Capture repeated soft/medium/hard strikes, early/late damping, pedal catch and release, then a sustained chord across adjacent pickup groups. The current 72-key scale uses interpolated and extrapolated parameter estimates. These measurements would revise its register transitions and establish where the amplifier leaves the small-signal regime. [Family roadmap](../docs/electromechanical-pianos/struck-metal.md)
