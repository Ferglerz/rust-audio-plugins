# Siemens ELV413 keyboard amplifier

Richard Wittig traced and tested the amplifier of the Technisches Museum Wien Neo-Bechstein in 2005. His 2007 article identifies a likely 1933 chassis with three triode gain stages, a 12-position master, a transformer-coupled pair of RE604 output triodes in class A and a separate 6 Ω speaker. He tested the restored amplifier with replacement NOS preamp tubes. Figure 8 supplies the circuit connections and tube types. The resistor, capacitor and winding values are unlabelled. Figure 10 and the published input sensitivities constrain a reconstruction of that specimen. [Wittig, Funkamateur 2/2007, pp. 153–155](https://www.worldradiohistory.com/INTERNATIONAL/FunkAmateur/FunkAmateur-2007-02.pdf#page=37) · [Archived journal scan](https://web.archive.org/web/20240609043414id_/https://www.worldradiohistory.com/INTERNATIONAL/FunkAmateur/FunkAmateur-2007-02.pdf#page=37)

## Signal path

- E1 connects the pickup terminal through a series capacitor to the REN904 grid. The engine supplies loaded terminal voltage. The pickup solver terminates the coil network in a 22 nF series coupling capacitor and 1 MΩ grid return, with an estimated 50 pF of stray capacitance at the jack.
- Each of the first two plate circuits has a resistor between the anode and the junction of its supply resistor and coupling capacitor. After the first stage, the external left pedal varies the capacitor between SW pins 3 and 4 from 350 to 700 pF. Its interaction with the second grid’s shunt capacitance and bias return sets both level and frequency dependence. Full pedal gives the larger capacitance in this reconstruction.
- The second stage drives the top of P2 through a coupling capacitor. A 12-position resistor switch drives the third stage from its wiper. The estimated total resistance is 250 kΩ. The assumed wiper fractions from bottom to top are 0, .020, .035, .060, .100, .150, .220, .320, .460, .610, .800 and 1.000. The publication documents 12 positions and terminal sensitivity at position 9. It gives no resistance schedule. The independent radio/phono source is open.
- The third triode drives the primary of TT through a series capacitor. Its center-tapped secondary applies opposite voltages to the two RE604 grids. Each triode’s plate drives one half of the output transformer. The speaker terminal sees 6 Ω. There is no overall feedback loop. [Wittig, figure 8 and legend](https://www.worldradiohistory.com/INTERNATIONAL/FunkAmateur/FunkAmateur-2007-02.pdf#page=38)

## Component estimates

The REN904 data sheet gives approximately 3.5 mA/V transconductance and amplification factor 25. Those slopes also approximate the two RS4324 stages on this unit. The AC stage resistors are 4.7 kΩ series and 68 kΩ plate feed. The two bias returns around the swell are 1 MΩ, and the effective second-grid capacitance is 5.87 nF. At the output, each RE604 uses an estimated 250 V/40 mA resting point, 2.5 mA/V local grid slope and amplification factor 3.4. The circuit solves the two nonlinear plate currents together with the transformer load each sample. An estimated 28 V anode collection knee bounds current near low plate voltages. [Telefunken REN904 data](https://tubedata.jp/sheets/105/r/REN904.pdf) · [Telefunken RE604 data](https://tubedata.jp/sheets/110/r/RE604.pdf) · [Independent electrical netlist](elv413.cir)

The fit uses a 0.673 µF plate-to-TT coupling capacitor, 9.78 H magnetizing inductance, 0.82× turns per secondary half and 300 Ω series copper. A lossy 2.27 H leakage branch with 119 pF feedthrough capacitance produces a transmission minimum near 9.7 kHz. The output transformer has fitted 14.4× half-primary turns, 100 Ω per half and 0.3 Ω secondary copper. The effective secondary leakage, shunt capacitance and magnetizing inductance are 68.9 µH, 3.01 µF and 0.30 H. These are equivalent values referred to the indicated winding. They jointly reproduce the measured dip and high-frequency recovery. Wittig suggests the capacitively coupled TT as a possible cause. A winding impedance measurement is needed to identify which individual parts produced that curve. [Wittig, discussion and measurement](https://www.worldradiohistory.com/INTERNATIONAL/FunkAmateur/FunkAmateur-2007-02.pdf#page=39)

The third triode’s 68 kΩ load feeds an estimated 120 V / 2 mA plate operating point from a 256 V rail. A nonlinear driver plate current includes finite anode collection. The two RE604 grids draw forward current above the estimated −45 V idle bias, with an effective 330 Ω grid resistance above threshold. The secondary load is reflected through TT into the driver solve. At strong input the grids load the transformer and reduce its gain. Bias-supply ripple and sag, first-stage microphonics, exact positive-grid characteristics and core saturation require further measurements. A resistive speaker load permits comparison at the terminals. Acoustic output in the room also depends on the separate cabinet. The model can deliver more than the rated 3 W at excessive inputs. Its overdrive power is an extrapolation.

## Validation

The figure supplies a normalized amplitude curve. The 34 sampled points span 10 Hz–19 kHz. Thin curve crossings and line thickness limit accuracy at the narrow notch. Compared directly with the normalized host-rate digital impulse, RMS deviations were 1.69 dB at 44.1 kHz, 1.65 dB at 48 kHz and 1.63 dB at 96 kHz. At all rates the largest deviation was 6.22 dB at 10 Hz. The midband peak near 3.5 kHz is about 1.4 dB lower in the model. The 17 kHz result follows the plotted line to within 0.2 dB in the analog comparison. [Digitized observations](fig10.csv) · [Sweep and large-signal test](amplifier.py)

| Frequency | Figure 10 | Analog reconstruction |
| ---: | ---: | ---: |
| 10 Hz | −37.15 dB | −43.37 dB |
| 20 Hz | −29.06 dB | −23.55 dB |
| 65 Hz | −2.90 dB | −3.58 dB |
| 1 kHz | +0.11 dB | 0.00 dB |
| 3.5 kHz | +1.24 dB | −0.14 dB |
| 9.5 kHz | −9.30 dB | −9.75 dB |
| 11.6 kHz | +1.15 dB | +0.31 dB |
| 17 kHz | −3.84 dB | −3.96 dB |

The published E1 sensitivity is 100 mV RMS at switch position 9 for approximately 3 W, with total distortion below 10% at full drive. Pedal position and test frequency are unspecified. At 1 kHz and full swell, the digital model gave 2.996 W fundamental power at 0.788% H2–H9 distortion into 6 Ω. The parallel ngspice test gave 2.996 W and 0.789%. At twice the input level the two simulations gave 5.412 and 5.411 W, with approximately 33% distortion. At 2 V RMS the digital and ngspice peaks were both 7.29 V. Above the rated input, strong compression and distortion are informed estimates. These comparisons validate numerical agreement and the rated sensitivity. The specimen’s harmonic spectrum at individual drive levels remains unmeasured here.

A previous linear-driver implementation allowed voltages over 1 kV at the third plate. With the nonlinear driver and grid load, two upper-register eight-note chords at full velocity produced speaker peaks of 7.26–7.61 V across 44.1, 48 and 96 kHz, at the published settings 9 and 12 with respective rest/full pedal. At 44.1 kHz, the former worst chord rooted at E5 drove 1.158 V peak into E1. At setting 12 the digital speaker peaked at 7.609 V against 7.554 V from an independent ngspice transient driven by that same waveform. Increasing that input to 7.617 V peak gave 7.887 and 7.897 V at the two speaker terminals, with 4.6% relative RMS waveform error. Under the latter stress the grids reached a 46.9 V excursion and the unloaded side of the capacitively coupled driver could see positive transformer kickback. There is no digital output limiter in that comparison. [Transient comparison and exact chord stimuli](overdrive.py)

The full native sweep covers 88 single keys and 60 chord roots at two velocities. Stock master 9/rest pedal reached 7.491, 7.410 and 7.273 V peak at 44.1, 48 and 96 kHz. Master 12/full swell reached 7.697, 7.546 and 7.378 V peak.

All circuit and signal history is preallocated. The audio path uses four internal samples per host sample with Blackman FIR interpolation and decimation. Together the two filters add exactly 24 host samples of delay. The four representative electrical settings agreed with independent ngspice from 15 Hz through 19 kHz to under 0.0001 dB and 0.0013° at 44.1, 48 and 96 kHz after compensation for the known FIR response, latency and trapezoidal frequency map. A separate Rust test checks the switch’s twelve ordered steps, swell travel, reset silence, and 3 W sensitivity. [Amplifier implementation](../src/amp.rs) · [Full-rate probe](../examples/elv413.rs)

Run from the workspace root with ngspice, NumPy and SciPy installed.

```sh
python3 neo-bechstein/research/amplifier.py
python3 neo-bechstein/research/overdrive.py
cargo test -p neo-bechstein amp::tests --quiet
```
