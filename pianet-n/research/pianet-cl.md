# Hohner Pianet C and mains Pianet L

The C and L use preloaded steel reeds. Each reed rests beneath an adhesive pad on a key lever. As the pad lifts, the preload relaxes and adhesion continues to lift the reed until detachment. The pad damps and preloads it again on return. The C manual documents five octaves F–F and modest key-speed sensitivity. The L manual shows the same action, the common grounded reed rail and the adjustable electrodes at the ends of the reeds. Both playable engines cover MIDI 29–89, retain the individual pad and reed state on repeats and have 16 host frames of latency. The present beam dimensions, contact, electrode geometry and losses come from the early Pianet N prototype. Independent measurements of the two actions remain outstanding. [C manual, action](https://hohner-pianet.com/pix/Pianet_C_Bedienungsanleitung_S_02_web.jpg) · [C compass and key-speed guidance](https://hohner-pianet.com/pix/Pianet_C_Bedienungsanleitung_S_03_web.jpg) · [L manual, action and transistors](https://hohner-pianet.com/pix/Bedienungsanleitung_Pianet_L_02_kl.jpg) · [L action drawing](https://hohner-pianet.com/pix/Bedienungsanleitung_Pianet_L_04_kl.jpg)

## Pianet C, drawing dated 28 February 1963

The 280 VAC transformer secondary feeds a rectifier, 2 MΩ/20 MΩ feed and the common electrode bank. A 22 nF capacitor couples the moving bank to the AC107 input. The circuit uses the Pianet's 1 MΩ feedback and 39 kΩ collector load. The smaller starred values on the drawing apply to the Cembalet. The AC107 collector feeds the ORP60, its parallel amplitude trimmer, a 10 µF coupling capacitor and the 250 kΩ logarithmic knee rheostat. The amplifier DIN is coupled through 33 nF. An independent AC125/AC126 branch drives the headphone socket through 100 µF. Both outputs remain electrically loaded when changing monitor selection. The CH amplifier is a separate unit. [C board](https://www.clavinet.com/Pianet_C_Schematic.pdf) · [Original C manual with its own drawing](https://hohner-pianet.com/pix/Pianet_C_Bedienungsanleitung_S_03_web.jpg)

The switch grounds one AC125 base to stop the astable. The other transistor then drives the lamp continuously. Enabling modulation lets it cycle. The model uses the 25 and 100 µF timing capacitors, 2.4 kΩ charge resistor and a 1.3 kΩ estimated setting of the 5 kΩ frequency trimmer in series with 500 Ω. At 440 Hz the first pair of modulation sidebands measure −14.41/−14.45 dBc. Relative to full, the half-open knee gives −6.6 dB and the closed rheostat gives −16.9 dB with a 100 kΩ external receiver and 200 Ω phones. The absolute lamp-to-cell transfer, bias trimmer, transistor operating points and the unreadable AC126 emitter bypass value are estimates. The input Z1 is treated as open during normal small-signal playing.

## Mains Pianet L, E71

The factory manual identifies an AC107, AC125, OC308 and two AD130s including the supply regulator. The pickup drives a 22 nF capacitor ahead of the AC107 and its 10 nF base shunt. The 1 MΩ **linear** knee rheostat feeds the second gain stage. Its collector feeds the 22 nF external-amplifier socket and the driver/power stage. The final AD130 has a single-ended two-speaker load. The factory manual confirms the external socket on instruments with their two internal speakers installed. It also confirms that the knee lever's end stop switches the mains off. The LB is a separate battery version. [Original L control and speaker description](https://hohner-pianet.com/pix/Bedienungsanleitung_Pianet_L_01_kl.jpg) · [Original, unmarked L circuit](https://hohner-pianet.com/pix/Bedienungsanleitung_Pianet_L_04_kl.jpg) · [L action and E71 schematic](https://www.clavinet.com/pianet_l_schematic.pdf)

The model solves the decoupled supply ladder, three preamp/driver stages, external receiver and power-transistor current driving both series 5 Ω units. Its loudspeaker impedance includes winding resistance and inductance, moving mass, compliance, damping and back EMF. The fixture retains both output voltage and the mechanical contribution to impedance. The selectable speaker signal is the electrical voltage at the speaker terminals. Floor reflection, cabinet radiation, microphone transfer and listening-position response need separate measurements.

Each driver's assumed 4.1 Ω DC resistance, 0.16 mH coil, 3.8 g moving mass, 0.25 mm/N compliance, 1.1 N·s/m mechanical loss and 1.7 T·m force factor await measurement. Estimated AD130 quiescent current and available supply voltage limit cutoff and saturation at high drive. The regulated supply is stiff in the audio model. The L's half-open knee measures −45.4 dB versus full in the 440 Hz test. Rest switches the circuit off. An 8 ms power transition approximates the stop. Hohner's schematic labels the transformer secondary on the pickup supply 280 VAC. Both models use an estimated 300 V DC working bias and 180 pF stationary bank.

## Electrical and recording checks

Separate ngspice decks retain the component nodes, loads and assumed device operating points. Maximum errors after the bilinear frequency mapping over 20 Hz–20 kHz were as follows.

| Output | Magnitude | Phase |
| --- | ---: | ---: |
| C amplifier DIN | 0.0012 dB | 0.0062° |
| C headphones | 0.0008 dB | 0.0097° |
| L external DIN | 0.0041 dB | 0.0259° |
| L series speaker terminals | 0.0048 dB | 0.0066° |
| L speaker back EMF | 0.0025 dB | 0.0319° |

The 75-velocity E4 strikes reached 23.1 mV at the C amplifier DIN and 190.8 mV at its headphone jack. The same key reached 16.0 mV at the L external socket and 710.7 mV at its speaker terminals. These are model outputs under the named loads. The absolute gains have yet to be compared with voltmeters on identified boards.

Hans Vermeersch published a recording of his Pianet C. A_Ruption published an L with its internal speakers fitted and described varying the original knee pot and moving toward the downward-facing speakers. We compared decaying components in these actual performances to the C external output and L speaker terminals. Neither recording is an isolated test with known control settings or synchronized key motion. The comparison uses 180 ms Hann projections spaced 40 ms apart over 640 ms. For each candidate pitch, we retained the first passage with a decaying component above the noise floor, at least a 3 dB fall and less than 0.4 dB linear-fit residual. The checker pins the exact timestamps and PCM files. [Pianet C, Hans Vermeersch](https://www.youtube.com/watch?v=5BqF4cPuzF4) · [Pianet L with internal speakers, A_Ruption](https://www.youtube.com/watch?v=v7gUYOnMlWU)

| Recording | Compared components | Median absolute decay-rate difference | Largest observed difference |
| --- | ---: | ---: | ---: |
| C | 6, G3–G4 | 3.21 dB/s | G4, recording −19.50 vs model −9.58 dB/s |
| L | 9, E3–A5 | 2.54 dB/s | C4, recording −16.97 vs model −8.26 dB/s |

The performances constrain decay over these specific passages. A matched dry C/L session, controlled key velocities, known knee positions, isolated key returns, microphone position at the L and impedance sweeps of its installed speakers would let us test the full onset and release. Chicago Electric Piano Company also lists direct L multisamples, but the public listing currently provides no downloadable archive. [CEPCo/Reverb L recording details](https://reverb.com/item/10634745-the-chicago-electric-piano-co-sample-series-vol-5-hohner-pianet-l)

## Reproduction

```sh
cargo run -p pianet-n --example pianet_cl -- target/pianet-cl-proof
python3 pianet-n/research/compare_cl.py target/pianet-cl-proof
cargo run -p pianet-n --release --example pianet_cl -- --bench
```

To reproduce the optional recording comparison, decode format 251 from each owner's linked video with yt-dlp and FFmpeg to 48 kHz WAV. Pass the paths to the same checker with `--c-demo` and `--l-demo`. The two audition WAVs render A2, E4 and A5 through the plugin engine. The five-channel WAV preserves both C and both L outputs plus speaker motion from a common excitation.

On this machine, repeated release-build runs at 48 kHz with 256-frame buffers gave 61-key p99 times of 1.35–1.36 ms for C with modulation running and 1.34–1.35 ms for L with speaker output. The buffer deadline is 5.33 ms. A run during concurrent build activity reached 8.10 ms on C. These measurements depend on host scheduling. Tests cover all six instruments at 44.1/48/96 kHz, source ownership, live monitor changes, power-off, and the native key limits.
