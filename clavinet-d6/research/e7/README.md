# Hohner Clavinet E7

The E7 uses the same simulated tangent, anvil, string and two spatial pickup taps as the D6. It sends those taps through a separate model of Hohner circuit **801 111**. The 60-key live instrument has the four pickup positions, four tone rockers, mechanical mute, logarithmic volume slider and independent power switch. The power switch silences the electronics while the strings continue moving. Its turn-on thump and discharge transient have yet to be measured. [Hohner E7 operating manual, pp. 5–6](https://www.clavinet.com/clavinet_e7.pdf) · [Factory schematic and board](https://www.clavinet.com/Clavinet_E7_schematic.pdf)

## Circuit

Both pickup banks contain six series sections. One bank terminates at the red transformer lead. The two ganged rockers leave one bank connected in each C position, put both in parallel in the D positions and can swap both terminals of the other bank. CA selects the centre and CB selects the bridge under the service map used here. With the default winding convention DA adds. In DB the centre terminal reverses against the bridge. The [E7 netlist](../electronics/e7.cir) represents individual rocker contacts. The audio engine uses their Thévenin equivalents. Signed pickup terminals and the physical pickup-to-bank assignment still need checking on a specific instrument. [Factory circuit](https://www.clavinet.com/Clavinet_E7_schematic.pdf) · [Repair shop’s D6/E7 pickup map](https://www.jupitervintagepianos.com/clavinet-pianet/what-do-the-switches-a-b-and-c-d-mean-on-my-clavinet/)

| Component | D6 reference board | E7, drawing 801 111 |
| --- | --- | --- |
| Preamp supply | 9 V battery | BC238 emitter follower, 1.8 kΩ feed, ZPD8.2 reference, 100 µF rail capacitor |
| Q1 | BC550C, 51 kΩ collector, 3.9 MΩ feedback | BC550C, 12 kΩ collector, 1 MΩ feedback |
| Q2 | BC148B, 120 kΩ / 20 kΩ base divider, 820 Ω above bypass | BC550C, 620 kΩ / 100 kΩ divider, 2.2 kΩ above bypass |
| Master level | 100 kΩ linear | 100 kΩ positive-log slider |

The 1:15 input transformer, feedback capacitor, coupling capacitors and four tone sections are common to the two circuits. At the E7 deck’s 9 V supply and 27 °C the calculated rail is 7.470 V. Q1 and Q2 collectors sit at 1.754 V and 6.186 V. The Rust circuit includes the small-signal impedance of the pass transistor and its zener network in parallel with the rail capacitor. With all tone tabs off the simulated rail couples a trace of signal between stages. Its largest CA gain is −36.89 dBV/V at 73% slider travel. A bench test is needed to determine the bleed and noise floor of the real board. Transformer copper and inductance, winding impedance, choke resistances, transistor parameters and the assumed 10% slider midpoint are estimates. [D6 circuit, transformer evidence and component assumptions](../electronics/README.md)

### Same electrical input at both jacks

The following small-signal gains are dB relative to one open-circuit volt at CA with all tabs on, full volume and a 1 MΩ DI. The factory component changes lower the modeled gain unevenly across the band. At half mechanical travel each response falls another 6.18 dB on the D6 and 20.00 dB on the E7 at 1.6 kHz.

| Frequency | D6 jack | E7 jack |
| --- | ---: | ---: |
| 100 Hz | 43.83 | 38.07 |
| 500 Hz | 55.15 | 49.85 |
| 1.6 kHz | 63.34 | 52.72 |
| 10 kHz | 61.55 | 52.42 |

A simultaneous 1 V centre and 0.37 V bridge impulse checks all 64 pickup/tone positions against independent ngspice AC results. At 96 kHz with bilinear frequency mapping, over 25 Hz–16 kHz, the largest E7 magnitude and phase differences were 0.0007 dB and 0.0024°. The all-off AC comparison uses exact tone shunts because contact resistance becomes significant at that signal level. At 73% slider travel the E7 treble-plus-brilliant notch is at 3,417.25 Hz. With all four tone tabs enabled the local minimum moves to 3,758.25 Hz.

The realtime preamp uses its small-signal operating point. The nonlinear decks predict 1.53% D6 and 0.17% E7 THD with a 500 Hz, 0.4 mV peak sinusoid at CA. At 4 mV peak the predictions rise to 18.63% and 4.18%. The three medium reference strikes reach 0.345–0.382 mV peak at the centre pickup. Loud chords can enter a range where the linear model omits audible compression. These are circuit predictions with estimated devices and source sensitivity.

## E7 string measurements and recording check

### What survives matching the playback level

At velocity 80 with all tone tabs on, the D6 and E7 produce sample-identical centre and bridge stems on B♭2, E4 and F♯5. Across the twelve key/selector combinations, the E7 electrical jack is 5.67–8.95 dB quieter. Matching the overall RMS still leaves a difference in the held harmonic balance. At CA the median absolute H2–H10 difference relative to H1 is 0.59 dB on B♭2, 3.34 dB on E4 and 3.15 dB on F♯5. The largest difference on a single retained harmonic is 6.47 dB. These are consequences of the two modeled boards receiving identical signals. The accuracy of the shared E7 pickup geometry remains open.

DB produces opposite overall polarity on the two reference boards under the selected wiring convention. The comparison reports that separately, then aligns polarity for the waveform residual. The harmonic comparison uses a −45 dBc floor and the held interval 25–175 ms after strike. The local equal-RMS audition plays D6 then E7 on each of the three CA notes, preserving timing. [Comparison script](voice_difference.py) · [CA listening pair](../../../target/clavinet-revision-audit/CA-D6-then-E7-level-matched.wav)

```sh
cargo run --release -p clavinet-d6 -- target/clavinet-revision-audit/d6 80 0.19 da-adds 15
cargo run --release -p clavinet-d6 -- target/clavinet-revision-audit/e7 80 0.19 da-adds 15 e7
python3 clavinet-d6/research/e7/voice_difference.py target/clavinet-revision-audit
```

### E7 mechanical assumptions

| Input to the string model | E7 treatment |
| --- | --- |
| Compass | F1–E6, MIDI 29–88 |
| Speaking length at B♭2 / E4 / F♯5 | 0.800 / 0.440 / 0.174 m, borrowed from the D6 fits |
| Centre / bridge positions | 160 / 42, 113 / 40 and 84 / 38 mm from the tailpiece, borrowed from the same D6 keys |
| Pickup apertures, working gaps, string gauges and anvil travel | D6 model values. No E7 teardown measurement was available for these comparisons. |
| Mute and yarn | Shared felt contact and afterlength loss. Mute pressure, gap and release time need a measured E7. |

Timothy Stedman's owner demonstration includes an ascending chromatic run. The table compares a 75 ms excerpt of each of the three prototype keys against a velocity-80 E7 model strike. Each model uses the same key action, time window (25–100 ms after key-down) and four active tone sections. The video's precise tab settings, pickup wiring, key velocity, amplifier and recording gain are unknown. The adjacent notes prevent an isolated key-up measurement. [Owner’s E7 demonstration, chromatic run](https://www.youtube.com/watch?v=CJA0LY2xF64&t=123s)

| Key | Video time | Power near note harmonics | Hardware / model pitch, Hz | H2–H8 median error, CA / CB / DA / DB, dB |
| --- | ---: | ---: | ---: | --- |
| B♭2 | 124.728 s | 97% | 115.90 / 116.42 | 12.8 / 15.3 / 11.4 / 7.7 |
| E4 | 126.838 s | 78% | 329.74 / 330.25 | 8.5 / 7.8 / 6.0 / 8.9 |
| F♯5 | 128.446 s | 96% | 737.27 / 738.63 | 30.4 / 16.3 / 16.8 / 20.2 |

The pitch estimate uses the median of resolved partial frequencies divided by harmonic number. The preceding E♭4 overlaps E4’s fundamental. The spectral comparison uses H2–H8 for all three keys and normalizes each profile to its largest partial. Pitch follows all three keys. The upper-register spectral imbalance is substantial across every pickup choice. This recording supplies an independent playing check. A dry, identified E7 with fixed pickup/tone states, measured key travel and isolated releases is still needed to fit geometry and damping.

### Reproduce

From the repository root, render the three notes and run the circuit checks with ngspice, NumPy and SciPy. The recording comparison also needs SoundFile, ffmpeg and the owner's Opus audio stream. The script verifies the compressed file's SHA-256 before analysis.

```sh
cargo run --release -p clavinet-d6 -- target/e7-probe 80 0.19 da-adds 15 e7
python3 clavinet-d6/research/electronics/compare.py --board e7
python3 clavinet-d6/research/electronics/levels.py
yt-dlp -f 251 -o target/e7-probe/owner.webm 'https://www.youtube.com/watch?v=CJA0LY2xF64'
python3 clavinet-d6/research/e7/compare.py target/e7-probe/owner.webm target/e7-probe
```
