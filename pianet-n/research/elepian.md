# Columbia Elepian EP-61C

The passive EP-61C stands on a recovered original circuit and action drawing, a technician’s demonstration of both regulation screws, and a second serviced instrument with a recorded performance. It has 61 independent hammer-struck steel reeds, a rear damper and pedal rod, a series electromagnetic pickup bank, and passive volume and tone. The implementation maps the native C-to-C compass to C2–C7. Optional extension supplies MIDI 0–127. The active EP-7C with built-in tremolo and the US Electrokey/Maestro capacitive spinet are different instruments. [Columbia’s October 1981 catalogue](https://guitar-catalog.com/keyboard-and-effector/columbia/1981/en_index.html) · [Serviced EP-61C and accessories](https://organ69.net/ep010.shtml)

## What the service instrument establishes

| Time in technician’s film | Observed hardware | Modeled behavior |
| --- | --- | --- |
| 0:54–1:22 | Upright-style action, rear dampers, individual magnetic pickups and their wiring | Hammer strike into a vertical bending reed, independent felt return and series coil sum |
| 1:23–2:20 | The lower screw alters fundamental/harmonic balance. The upper screw changes clearance and level | Separate lateral pole alignment and axial pickup distance. Change either during a held note while preserving mechanical displacement. The flux derivative is rebased at the moving pickup. |
| 2:22–3:19 | Hammer-blow regulation on the jack, independent let-off screw, felt rail and whole-action adjustment | Driven jack, release into free hammer flight, unilateral tip contact, rebound and backcheck. A repeat preserves the ringing reed through the jack reset. |
| 3:21–3:51 | Harp signal is available ahead of the passive control panel. The technician records it directly. The original schematic shows the full series bank, 10 kΩ resistor, 100 kΩ tone and volume pots, 0.047 µF capacitor and 15 kΩ nominal output. | A raw harp probe and a separately loaded terminal. Knobs change the physical wiper impedances, including the varying load seen by the other pot. |
| 3:52–4:03 and 4:28–5:08 | Original labeled action drawing and a demonstration of the pedal rod retracting the rear dampers | A key lifts its own damper. The common rail can retain all notes through key-up and releases those dampers when the pedal returns. |

The technician’s specimen had a replacement pedal rod. Organ69’s separate instrument had a faulty highest C. That note is excluded from the recording checks. [PreparedRhodes inspection, original drawings and direct-harp performance](https://www.youtube.com/watch?v=Kv1VYiWfWxU) · [Shinichiro Sudo plays the shop’s EP-61C](https://www.youtube.com/watch?v=iE5FT6v9IAo)

## Action and pickup

The model solves a clamped steel beam with a point mass at the tip. The end weight changes both modal frequencies and moving mass. The root condition includes the inertial shear at the tip, and each modal mass includes the distributed steel and the lumped weight. The assumed steel section is 0.60 × 4.7 mm. Free length follows the target pitch and the estimated end-weight ratio. The hammers strike at 68% of the free length and the felt returns at 82%. A bounded contact spring and dashpot exchange force with the moving hammer and reed.

The source identifies the mechanical adjustments. The model's 5 mm effective hammer blow, 72% key travel at let-off, hammer mass, tip compliance, contact locations and wire polarities have yet to be measured on a specimen. Effective modal losses use four decay anchors from organ69. At fixed MIDI velocities 40, 80 and 115 the modeled E4 first contacts after 10.68, 6.46 and 4.81 ms. The corresponding E4 peak displacements are 0.435, 0.744 and 1.010 mm. These are predictions. Neither recording contains synchronized key travel.

Nippon Columbia’s patent Fig. 14 shows a tip weight, a coil and separate vertical and lateral regulation screws, consistent with the service film. Its Figs. 11–18 describe a finite pole face and give measured spectra while changing its coverage of reed travel. The implementation uses the Fig. 11 constant-reluctance region with fringing at both edges, then differentiates flux linkage. The remaining unknown is which pole contour was fitted under the bobbins of the observed production EP-61Cs. The bass/treble face lengths, field scale and clearances are estimates. [Nippon Columbia US 4,175,461, Figs. 11–18](https://patents.google.com/patent/US4175461A/en)

F4 in the technician’s regulation demonstration has H2 at −18.26 dBc at 1:40.50 and −48.55 dBc at 2:00.90. A model sweep across ±1 mm moves H2 through a minimum of −43.19 dBc. H2 is −22.79 dBc at stock, −43.19 dBc at +0.10 mm and −17.00 dBc at +0.50 mm. This reproduces the direction change near a second-harmonic null. The absolute screw travel and touch were unmeasured in the recording. The deepest recorded null is 5.36 dB below the sampled model minimum. The separate pickup clearance control moves the axial gap from −0.15 to +1.00 mm relative to each native key’s regulation.

## Traced passive output

The 61 source coils are wired in series with alternating terminal connections. Relative pole magnetization remains unmeasured. The control panel takes the harp through 10 kΩ into two parallel 100 kΩ tracks. The tone wiper shunts through 47 nF. The volume wiper feeds the output jack directly. The default receiver is 1 MΩ with an estimated 470 pF cable. The coil bank's assumed 0.122 H and effective 11.43 kΩ copper remain to be measured. The copper estimate makes the complete model's low-frequency output impedance equal the printed 15 kΩ with controls fully open. Pot tapers are provisionally linear.

An independent complex impedance-divider calculation checks every wiper and load, with bilinear frequency mapping for the discrete solve. Twelve combinations of 100 kΩ/1 MΩ load, three tone positions and two volume positions match within 0.000004 dB and 0.00004° at 17 frequencies from 20 Hz to 20 kHz. This checks the solver against the traced circuit and the stated coil/cable estimates. An impedance sweep on a physical unit would test those estimates.

| Frequency | Tone dark | Tone center | Tone bright |
| --- | ---: | ---: | ---: |
| 110 Hz | −4.14 dB | −3.74 dB | −3.23 dB |
| 440 Hz | −9.97 dB | −4.34 dB | −3.23 dB |
| 3 kHz | −25.71 dB | −4.43 dB | −3.22 dB |
| 7 kHz | −33.43 dB | −4.45 dB | −3.23 dB |

The table is jack voltage relative to open-circuit induced voltage at full volume into 1 MΩ. With stock controls, E4 model jack peaks were 43.67, 64.19 and 77.68 mV at velocities 40, 80 and 115. Plugin output calibration is separate. [Passive network](../src/elepian/panel.rs)

## Hardware and model comparisons

The pinned note windows use 160 ms Hann analysis, independent frequency refinement and a harmonic-fit energy threshold of 91%. The technician’s direct capture is compared against the modeled raw harp. Organ69’s performance is compared against the modeled passive jack with stock control settings. Model notes use MIDI velocity 80. A −40 dBc floor keeps inaudibly weak upper harmonics from dominating the error statistic. Listening pairs use independent RMS alignment. Their unknown gain chains preclude an absolute voltage comparison.

| Source | Spectral comparisons | Median absolute H2–H6 difference | H2 / H3 median difference | Reserved decay comparisons | Median absolute decay difference |
| --- | ---: | ---: | ---: | ---: | ---: |
| PreparedRhodes, harp | 10 notes | 2.95 dB | 5.92 / 10.17 dB | 15 | 1.24 dB/s |
| Organ69, output | 11 notes | 5.84 dB | 4.94 / 4.75 dB | 13 | 1.53 dB/s |

Decay uses 15 overlapping 160 ms projections from an identified, monotonically falling component. Four additional organ69 anchors at MIDI 44, 60, 72 and 84 gave a median 0.38 dB/s difference. The shop performance can contain other sounding notes. The chain, pole adjustment, exact playing velocities and preceding key movement are unknown. The 10.17 dB H3 discrepancy on the raw harp and the width of the F4 null are priorities for measurements of production pole and reed geometry. Held-out pitch and decay alone cannot establish a match to the original hammer force or a particular felt regulation.

## Reproduce

```sh
cargo run --release -p pianet-n --example elepian -- target/elepian-probe
ffmpeg -i technician-video.mkv -ac 1 -ar 22050 -c:a pcm_s16le target/elepian-probe/harp-tech.wav
ffmpeg -i organ69-owner.webm -ac 1 -ar 22050 -c:a pcm_s16le target/elepian-probe/organ69.wav
python3 pianet-n/research/compare_elepian.py target/elepian-probe
```

Each `note-*.wav` stores passive jack on the left and harp EMF on the right. The CSVs preserve contact, harmonic, decay and two-screw regulation observations. The paired clips are `harp-tech-then-model.wav` and `organ69-then-model.wav`. Both original source digests and timestamps are in the analysis script. The editor diagram follows the side elevation on the original action sheet. Striking the diagram animates the jack and hammer. The pedal retracts the damper and the two pickup adjustments move its pole in separate directions. [Numerical fixture](../examples/elepian.rs) · [Reference checker](compare_elepian.py)
