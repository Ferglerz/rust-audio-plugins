# Rhodes MK8 FX factory board

The **Board** switch selects MK8 S or MK8 FX. Standard S is the default. Fitting the optional board exposes the four independent switches, the normal panel controls and the alternative adjustments. The choice, switch positions and control values are host parameters. Preamp, Effects and Alt select the editor page. Three or more taps set the delay time, including times beyond the 800 ms dial maximum. Moving the time slider replaces the tapped value. Both configurations have the same hammer action, pickup send and pedal. [Rhodes factory specifications](https://rhodesmusic.com/rhodes-mk8-specifications/) · [owner guide, pp. 10–18](https://rhodesmusic.com/wp-content/uploads/2023/07/MK8-User-Guide.pdf)

## Signal path

The pickup send stays available before the electronics. The switched return drives the compressor, diode drive, EQ, chorus, phaser, stereo delay, Vari-Pan and master volume, in that order. The owner guide places compressor makeup ahead of Drive and documents the EQ-to-FX trim. Chicago Electric Piano Company identifies the complete routing after inspecting and playing the unit in its workshop. The model exposes the same return for offline testing. [Workshop account, Onboard Effects Upgrade](https://chicagoelectricpiano.com/blogs/news/rhodes-mk8) · [factory board description](https://rhodesmusic.com/the-rhodes-mk8-new-world-of-sonic-possibilities/) · [Rust board](../src/fx.rs)

| Block | Factory controls and construction | Physical reduction and evidence limits |
|---|---|---|
| VCA compressor | On/off, Amount and Makeup, alternative ratio | A rectified detector drives a power-law VCA. The threshold falls with Amount. The 8 ms attack, 130 ms recovery, 1–20 ratio range and maximum 24 dB makeup are estimates. |
| Stereo chorus | Independently clocked BBD audio lines, Rate, Depth, alternative center time | Two 512-cell packet stores use opposing sinusoidal clock sweeps. Two production performances support an inverted left wet contribution and positive right wet contribution. The fixed wet/dry voltage ratio is 0.62. Actual chip, cell count and clock filters are unknown. |
| Stereo phaser | Mono input, four voltage-controlled stages, Rate, Depth, alternative center frequency | Four first-order all-pass sections feed each wet output. Summing the dry branch places two notches around the swept pole. The two modulation phases are one quarter-cycle apart. The dry stereo branch preserves the preceding chorus. The specific VCA, capacitors and wet summing resistances need a service drawing. |
| Stereo delay | Two BBD circuits, 60–800 ms dial, tap tempo, Feedback, Mix, alternative Spread and Jitter | Two variable clocks transport charge through 4,096 cells each. Clock speed limits recording/playback bandwidth. Wet inversion and a fixed inter-channel time offset follow the captures below. Input stops at bypass, while stored charge and feedback continue. Maximum feedback exceeds unity and a finite charge swing bounds sustained repeats. The 2 s tap ceiling, available spread, jitter range and bucket bias are estimates. |
| Transfer levels | Alternative drive output and EQ-to-FX send levels | The model provides ±12 dB at each location on the fitted board. Equivalent gain range and zero positions require return-level measurements. |

The delay and chorus clocks sample a filtered input at each transfer edge. Short delay times can require multiple transfers per integration step. Stored charge travels through a fixed number of stages, so changing the clock alters the pitch and arrival time of packets already in flight. The model omits individual transfer mismatch, oscillator feedthrough, device hiss and explicit compander or bias networks. The hardware's production schematic and a voltage-calibrated return sweep remain unavailable. [Board implementation](../src/fx.rs) · [maker's effect descriptions and alternative controls](https://rhodesmusic.com/the-rhodes-mk8-new-world-of-sonic-possibilities/)

## Production recording comparisons

The Synth Shed's gold loan instrument has separately captioned compressor, chorus, phaser and delay performances. Bonners' production demo with Rhodes' Jamie Bullock provides a second instrument and an independently recorded chorus. The audio is stereo program output with unknown knob positions, gain, loading and codec processing. The maker's prototype mid-filter pedal demonstration belongs to the standard resonant EQ. [The Synth Shed, FX chapter](https://www.youtube.com/watch?v=FaCyg4qM_5Y&t=401s) · [Bonners, factory FX performance](https://www.youtube.com/watch?v=3o7f3QRSqIg&t=1460s)

For the modulation comparison, the analysis measures complex left/right spectra of audible harmonic peaks in 64 ms windows. Two thirds of the frames identify the rate, excursion, center and initial phase. The reconstructed mono excitation drives the actual Rust panel. The stereo error in the last two columns uses every third frame held back from parameter fitting. For each bin the squared sine of the angle between the two complex stereo vectors is zero for matching stereo direction and one for orthogonal vectors. A mono baseline uses the same recorded bins. Source amplitude cancels in the comparison. The reported widths also show where the approximation misses the hardware. [Analysis and reference timestamps](fx.py) · [oversampled Rust return probe](../examples/mk8_fx_probe.rs)

| Recording and interval, s | Identified oscillator | Stereo width, recorded / model, dB side/mid | Stereo error, model | Stereo error, mono |
|---|---:|---:|---:|---:|
| Synth Shed phaser 439–445.8 | 1.801 Hz | −3.96 / −5.43 | 0.031 | 0.322 |
| Synth Shed phaser 450–456.9 | 0.659 Hz | −1.92 / −2.33 | 0.050 | 0.397 |
| Synth Shed phaser 460–468.4 | 4.680 Hz | −1.21 / −1.68 | 0.056 | 0.442 |
| Synth Shed chorus 419–431.5 | 3.253 Hz | −5.91 / −8.50 | 0.033 | 0.191 |
| Bonners chorus 1638–1653 | 3.228 Hz | −7.15 / −7.77 | 0.080 | 0.182 |

All three phaser passages support a center near 420 Hz and inferred excursions of 0.67, 1.53 and 1.30 octaves respectively. The fast passage shows a stereo spectral line at 9.36 Hz and its fundamental at 4.68 Hz in both renders. The chorus fits imply effective center delays of 5.16 and 6.49 ms with 1.09 and 0.69 ms excursions. The two strongest stereo modulation lines are 6.51 and 3.25 Hz in the first performance, 6.46 and 3.22 Hz in the second. These controls reproduce those passages. Factory calibration and exact knob tapers remain open. [Comparison script](fx.py)

**The delay has opposite-polarity echoes and measurable stereo spacing.** High-frequency whitening suppresses pitch-period ambiguities in the recorded autocorrelation. In the first Bonners passage the model's base time was adjusted to the observed mean and spread to the measured channel separation. A later change of delay time retains approximately the same 13.2 ms separation. Holding the same modeled spread reproduces the second pair to 0.17 ms. The feedback knob and absolute wet voltage cannot be recovered from these mixed performances. [Bonners, Delay and Pseudo Reverb chapters](https://www.youtube.com/watch?v=3o7f3QRSqIg&t=1460s) · [lag analysis](fx.py)

| Bonners interval | First left echo, recorded / model | First right echo, recorded / model |
|---|---:|---:|
| 1467–1478 s | 400.17 / 400.08 ms | 387.00 / 386.88 ms |
| 1591–1597 s | 184.38 / 184.29 ms | 171.25 / 171.08 ms |

In the first passage, normalized left/right repeat correlations in the 400–1,500 Hz band are −0.45/−0.49. At 4.5–9 kHz these weaken to −0.10/−0.11. With separate seeded noise excitation, the model gives −0.63/−0.63 and −0.06/−0.06 in the same bands. The recordings establish the wet polarity, timing and a darker repeat. The synthetic repeat rolls off more strongly by this measure. A controlled return impulse is needed to compare actual bandwidth. [Band-limited repeat analysis](fx.py)

**The compressor demonstrations identify a change in dynamics but leave its transfer underdetermined.** In 20 ms audible frames, the 95th-to-10th-percentile RMS spread is 23.62 dB in the Synth Shed clean passage, 6.89 dB in its compressor passage, and 15.91 dB in the Bonners compressor example. Those are different performances with unknown velocity and settings. The modeled compressor applied to the Synth Shed clean excerpt gives 15.38 dB at a provisional 0.3 V peak, Amount 0.70, ratio 4 and unity makeup. Independently, a tenfold stepped steady input tests the modeled 4:1 slope to within 0.001 dB. Actual threshold, detector time constants and VCA gain law await matched input/output levels. [Measurements](fx.py) · [VCA and notch tests](../src/fx.rs)

## Playback and verification

All four bypass switches can be automated. The physical delay's buffered tail keeps the engine active through silent gaps. Reset clears stored charge and modulation history. The dry send remains available with all four effects running. Release-build timing with all four enabled, 64-frame blocks and a single rendering thread was as follows on the development machine. Values are elapsed seconds per second of audio. The common anti-alias filter contributes 12 host frames; the wet line's intentional delay is separate. [Benchmark](../examples/mk8_probe.rs) · [live API](../src/live.rs)

| Rate | 1 voice | 8 | 16 | 32 | 73 |
|---|---:|---:|---:|---:|---:|
| 44.1 kHz | 0.038 | 0.094 | 0.174 | 0.325 | 0.685 |
| 48 kHz | 0.034 | 0.093 | 0.173 | 0.307 | 0.661 |
| 96 kHz | 0.038 | 0.103 | 0.153 | 0.269 | 0.589 |

From the repository root, run the following with local copies of the two identified complete performances. The script calls FFmpeg, NumPy, SciPy, SoundFile and the Rust probe. It writes only derived comparisons to the supplied output directory.

```sh
python3 rhodes-mk8/research/fx.py BONNERS.webm SYNTH_SHED.webm target/mk8-fx-probe
cargo run --release -p rhodes-mk8 --example mk8_probe -- bench-fx
cargo test -p rhodes-mk8 --quiet
```
