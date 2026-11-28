# Eight reed instruments at the same key and velocity

One MIDI E4 at velocity 80 starts at frame zero, holds for 450 ms and releases. The mono 48 kHz output is the simulated jack voltage before the plugin's digital gain. Each board uses its native sensor distance, default tone, full swell and modulation off. Hohner receivers use 100 kΩ at the live engine's output, except that the N's standard panel uses its own three-jack network and loads. Both cam-reed reconstructions drive 1 MΩ. The C and L use their external amplifier sockets. The Cembalet N uses its high jack. The default 200 register is Piano and the Selmer tone control is centered.

## Drawings and test points

| Instrument | Evidence carried into the model | Independent check and remaining limits |
| --- | --- | --- |
| Pianet N, 30 002 167 D | Biased electrode bank, transformer, AC107 gain, lamp/ORP60, foot swell, AC125 final and three taps. | A full-open modeled midrange triad produces 1:2.96:5.98 peak ratios into 10 MΩ. The transformer impedance and pad regulation remain estimated. [N sheet](https://www.clavinet.com/Pianet_N_schematic.pdf) · [N probe](../README.md) |
| Pianet C, 28 February 1963 sheet | The 22 nF pickup input drives the AC107, followed by optical modulation and a 250 kΩ logarithmic knee control. The 33 nF external coupling and separate AC125/AC126 phones drive are live simultaneously. | E4 at velocity 75 gives 23.1 mV external and 190.8 mV phones, or +18.3 dB. Loaded transfer matches the independent SPICE circuit within 0.0012 dB. The lamps and transistors use estimated operating points. [C sheet](https://www.clavinet.com/Pianet_C_Schematic.pdf) · [C test and external loads](pianet-cl.md) |
| Pianet L, E71 mains | The AC107 input has a 10 nF base shunt, a 1 MΩ linear knee control, a 22 nF external output and a power branch ending in AD130 and series 5 Ω speakers. Knee rest cuts mains power. | At the same velocity, 16.0 mV external and 710.7 mV at the speakers give +33.0 dB. Each electrical output matches independent SPICE within 0.0048 dB. Moving-coil and power-stage operating parameters are estimates. [L sheet](https://www.clavinet.com/pianet_l_schematic.pdf) · [L test and speaker load](pianet-cl.md) |
| Pianet T | A silicone suction cup excites a magnetized reed, a series bank of alternating winding connections drives the output transformer with a 2 nF shunt, and there is no onboard gain stage. | At 440 Hz the physical bank model predicts +3.3 dB into 10 kΩ and +10.7 dB into 100 kΩ. Coil values and transformer ratio need measurement. Eight reserved recorded pitches give a 2.51 dB median H2 difference. [Factory drawing and load](https://www.clavinet.com/pianet_t_manual.pdf) · [T electrical and hardware checks](pianet-t.md) |
| Early Cembalet | A plectrum clears the reed and a separate sprung damper returns. The ECC83 carrier, tuned receiver and ECH81 detector convert the common bank's instantaneous capacitance to audio. The knee feeds the output. | On 0.5–1 pF tests, an independent carrier calculation agrees within 0.005 dB at the fundamental. The detector is nonlinear when multiple reeds move. Tank Q, coupling and operating point are estimated. [Hohner action Fig. 1 and receiver circuit](https://civade.com/AudioPro/Hohner/Hohner_Cembalet_Service_Manual.pdf) · [RF comparison](cembalet.md) |
| Cembalet N, 650720 | The action still uses a plectrum, while 300 V electrode polarization and a transformer drive a transistor preamp. Its 5 nF swell input, 100 kΩ low-jack leg and 600/200 kΩ middle divider distinguish it from the Pianet N sheet. | A C4/E4/G4 triad gives 25.3, 65.4 and 233.6 mV peak under the three specified service loads, all below Hohner's nominal 100, 300 and 1,500 mV. The independent terminal sweep agrees within 0.0016 dB. [Hohner 650720 Fig. 6](https://www.clavinet.com/Cembalet_N_schematic.pdf) · [Loaded service tests](cembalet.md) |
| Claviset 200, magnetic reconstruction | Owner and museum identify nine tone contacts, three modulation rates, foot swell and mechanical sustain. The specimen view shows the pickup rail. The model uses an LR source and switched RC voicings. | The recording gives 3.71, 6.24 and 8.86 Hz. A published magnetic instrument description and a conflicting capacitive 200 owner description leave the pickup revision unresolved. Register components are estimates. [Owner’s 200](https://www.youtube.com/watch?v=-ncOPN2Nn2w) · [Source and spectrum audit](claviset.md) |
| Selmer 8701, magnetic reconstruction | Catalogue and serial 877 support three continuously variable controls, knee swell, two batteries, magnetic pickup and transformer. The model uses a different transformer input and a continuous tone network. | Recorded C4 and A4 decay agree within 1.2 dB/s. On 13 notes the median H2 discrepancy is 12.1 dB. The transformer ratio, preamp parts and original gain remain unverified. [Selmer no. 8701 catalogue](https://www.vintagehofner.co.uk/selmer/misc/pianotron2.jpg) · [Serial 877](https://www.spheremusic.com/Bargaindtl.asp?Item=12034) · [Comparison](claviset.md) |

## Measured terminal output

RMS uses 50–333 ms after key-down. Harmonics use a 100–250 ms Hann window. The tail compares the RMS at 50–100 ms after key-up with the final 50 ms of held sound. The pickup columns give the RMS change after adding the indicated signed distance to each key's estimated stock clearance.

| Instrument | Stock RMS | Peak | H2 / H3 vs H1 | Tail vs held | −0.15 mm | +1.00 mm |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Pianet N | 123.81 mV | 348.60 mV | −11.26 / −9.95 dB | −82.03 dB | +7.68 dB | −21.58 dB |
| Pianet C | 8.00 mV | 23.23 mV | −11.37 / −9.97 dB | −81.94 dB | +7.73 dB | −21.58 dB |
| Pianet L | 5.69 mV | 16.11 mV | −11.33 / −10.04 dB | −78.30 dB | +7.66 dB | −21.56 dB |
| Pianet T | 39.83 mV | 76.63 mV | −13.40 / −32.74 dB | −49.43 dB | +2.75 dB | −11.71 dB |
| Cembalet | 74.19 mV | 154.66 mV | −13.32 / −28.46 dB | −40.90 dB | +7.15 dB | −21.06 dB |
| Cembalet N | 84.09 mV | 181.86 mV | −13.77 / −32.60 dB | −39.79 dB | +5.59 dB | −18.37 dB |
| Claviset 200 | 45.70 mV | 100.10 mV | −21.78 / −27.64 dB | −44.55 dB | +3.17 dB | −13.15 dB |
| Selmer 8701 | 153.69 mV | 348.08 mV | −21.59 / −27.31 dB | −44.12 dB | +3.17 dB | −13.15 dB |

The N, C and L intentionally share the early adhesive action. Their dry E4 H2 levels are within 0.11 dB. Their largest differences appear at the physical output sockets, knee settings and modulation. With a 440 Hz circuit stimulus, half knee gives −6.6 dB on C and −45.4 dB on L. Centered Selmer and Piano-tab 200 also sound similar after matching level. The separate switches, modulation and damper controls expose stronger differences. The two platforms share an estimated blade/pole scale pending physical measurements. The raw Selmer E4 has 10.54 dB more held RMS than the 200. The recorded clips have unknown gain, so that voltage difference cannot establish the corresponding hardware ratio.

The pairwise calculation fits the best single signed gain to the whole, time-aligned key-down waveform and reports residual energy relative to the comparison waveform. C/L gives −20.17 dB, N/L −18.16 dB, N/C −14.70 dB, Claviset/Selmer −16.08 dB, and the two Cembalets −7.27 dB. These differences include phase, attack and decay. They neither isolate an individual part nor establish a listening threshold.

## Extended range and return sound

Extended range accepts MIDI 0–127. An out-of-compass key keeps the nearest supported physical section, contact and nominal pickup regulation while its bending frequencies shift by the requested interval. Enabling extension reproduced every sample of the fixed native E4 event on all eight boards. At supported audio rates the endpoint check distinguishes MIDI 0 and 127 from nearby or clamped pitches. A fundamental beyond the host Nyquist frequency has no audible component.

The signed pickup control changes the electrode or pole clearance before transduction. Applying 0.00 mm and 100% pad return gave zero sample difference from the untouched stock output on every board, including the attack and complete recorded release. Stock mechanical clearances, magnetic field extrapolation and the resulting distance sweeps remain estimates.

On the adhesive N/C/L and suction T the model detects recontact and splits the pickup motion into a 520 Hz low-pass component and its complement for 18 ms. The incident displacement and velocity initialize the filter. A second seal during the same collision continues the existing window. The final 20% of the window tapers the correction smoothly. Zero percent removes that estimated high-frequency component, while 50% retains half and 100% preserves the complete stock pickup. The force, reed displacement, seating, seal recovery and duration of actual damping are identical at all three settings. Plectra and cam lips have no adhesive return adjustment.

The raw six-key experiment measures RMS above 1.8 kHz using a fourth-order Butterworth high-pass, from 7–27 ms after key-up. Values give the change from 100% at the same unadjusted pickup. Release noise here describes the modeled electrical contact contribution. Selecting zero still leaves the damped reed audible.

| Pad and note | Modeled recontact after key-up | 0% | 50% | 100% |
| --- | ---: | ---: | ---: | ---: |
| N A2 | 8.646 ms | −13.12 dB | −5.23 dB | 0 dB |
| N E4 | 8.729 ms | −8.49 dB | −4.21 dB | 0 dB |
| N A5 | 8.188 ms | −6.26 dB | −3.06 dB | 0 dB |
| T A2 | 8.115 ms | −13.20 dB | −5.22 dB | 0 dB |
| T E4 | 7.729 ms | −12.02 dB | −5.10 dB | 0 dB |
| T A5 | 8.219 ms | −5.40 dB | −2.68 dB | 0 dB |

## Reproduce and listen

```sh
cargo run --release -p pianet-n --example pad_return -- target/claviset-probe
```

- `pad-release-0-50-100.wav` plays N, C, L and T. Each group contains 0%, 50%, then 100% of the return component, cropped from 70 ms before key-up.
- `pickup-distance-near-stock-far.wav` plays the eight instruments in table order. Each group plays −0.15, 0 and +1.00 mm, with one common gain across the three settings. Both WAVs use 24-bit PCM. Gains are set independently for each instrument so all groups are audible.
- `pickup-distance.csv` preserves the unscaled jack RMS, peak, gain changes and stock difference. `variant-terminals.csv` and `variant-pairs.csv` preserve spectral, damping and pair comparisons.
- The six `return-{N,T}-{45,64,81}.wav` files preserve the electrical sensor derivatives at 96 kHz, simultaneously for 0%, 50% and 100%. The electrostatic probe uses a fixed gain of 3 × 10⁹ on the capacitance derivative. The magnetic probe stores the flux derivative. [Executable fixture](../examples/pad_return.rs)
