# Hohner Cembalet and Cembalet N

This crate models two 61-key C2–C7 instruments. The early Cembalet uses a tuned radio-frequency receiver and a knee swell. The Cembalet N uses its DC-polarized electrode bank, transformer and transistor preamplifier on drawing 650720, with a foot swell. Each key has a plectrum and an independent sprung damper. [Early Hohner service manual and parts list](https://civade.com/AudioPro/Hohner/Hohner_Cembalet_Service_Manual.pdf) · [Cembalet N manual and action drawing](https://hohner-cembalet.com/Cembalet_N.html) · [Cembalet N circuit, Fig. 6](https://www.clavinet.com/Cembalet_N_schematic.pdf)

## Mechanical evidence

The manuals describe an adjustable plectrum that raises a grounded cantilever reed from below. The reed escapes the plectrum between halfway and two-thirds through the downstroke. Releasing the key brings the sprung damper against the top of the reed and lets the plectrum slip beneath it. Both manuals assign the player's continuous dynamics to the swell. Hohner's N instructions also describe adjusting each side electrode to balance the notes. [Early manual, pp. 2, 4 and 8](https://civade.com/AudioPro/Hohner/Hohner_Cembalet_Service_Manual.pdf) · [N manual, p. 5](https://hohner-cembalet.com/pix/Cembalet_N_Bedienungs_u_Service_Anleitung_S_05_kl.jpg)

The key drives unilateral contacts at two locations on the same damped beam. Modal displacement and velocity survive the entire stroke. The blade clears at 58.5% of travel and can rearm after damper contact and sufficient return travel. The reed lengths, section, losses, effective plectrum overlap, compliance, contact locations, 4 mm stroke and electrode gaps are estimates. Six independently adjustable octave-anchor gaps per board implement the factory balancing procedure. Across the 61 keys, 20–120 ms RMS at velocity 80 spans 2.83 dB on the early circuit and 3.88 dB on 650720.

| Model result | C2 | C4 | C6 |
| --- | ---: | ---: | ---: |
| Damper contact after key-up | 11.10 ms | 6.85 ms | 6.89 ms |
| Plectrum rearm after key-up | 11.10 ms | 8.20 ms | 8.20 ms |
| Early circuit, fast/slow attack RMS | +0.48 dB | +0.07 dB | −0.21 dB |
| 650720, fast/slow attack RMS | +0.83 dB | +0.09 dB | −0.22 dB |
| Early circuit, 50–90 ms after key-up relative to held | −36.1 dB | −36.4 dB | −82.2 dB |
| 650720, same tail | −31.6 dB | −34.9 dB | −88.1 dB |

The slow and fast strokes use MIDI 35 and 105, with no velocity gain multiplier. A 40% depression and a 3.1 ms return failed to retrigger the plectrum. Completed strokes escaped at 58.63–58.70% in the 32 kHz observation stream. The bass damper landed and the pick rearmed within the same recorded interval. Solver-resolution tests check their order.

## Tube circuit

The early schematic shows an ECC83 carrier oscillator, coupled transmitting and receiving tanks, a common electrode bank and ECH81 grid detector. Its other ECC83 triode drives the amplitude modulation at 6–8 Hz. The electrical model uses a 1.75 MHz carrier, the instantaneous sum of electrode capacitances and a complex receiver state. Transconductance and output resistance approximate the two sections of the ECH81 around an estimated operating point. The small-signal audio network includes the shared cathode, 1 nF coupling, 1 MΩ feedback, 50 nF output coupling, 20 kΩ feed and 250 kΩ knee pot. The oscillator is represented by its carrier drive and the modulation source by a 7 Hz control. The estimated receiver uses 105 pF bank-plus-trimmer capacitance, Q of 28, 3 V excitation and a 0.38 V grid-current threshold. RF transformer coupling, loaded Q, grid bias and modulation depth need bench measurements. [Hohner receiver circuit and parts list, pp. 6–7](https://civade.com/AudioPro/Hohner/Hohner_Cembalet_Service_Manual.pdf) · [Philips ECH81 data](https://tubedata.jp/sheets/010/e/ECH81.pdf)

An independent harmonic-balance solve uses the absolute carrier, 40 sidebands, the current through the time-varying capacitor and 1,024 numerical grid-rectifier samples per carrier cycle. With 0.5–1.0 pF test motions at 110, 440 and 1,000 Hz, the audio-rate receiver agrees within 0.005 dB and 0.20° at the fundamental. The weak second harmonic differs by up to 0.31 dB and 1.23°. On Hohner's ten-key service stimulus, the RMS difference between one shared detector and ten independently detected reeds is −1.85 dB at resonance center and −49.95 dB on the tuned flank. The separate three-key jack stimulus gives −62.10 dB. These comparisons verify the circuit reduction and the expected regulation trend.

## Transistor board 650720

Hohner specifies **300 V at the electrodes**. The drawing shows +400 V upstream of the 20 MΩ feed. The model drives the transformer with the sum of 300 times each key's capacitance derivative. The ORP60 lamp cell modulates the level at about 6 Hz. Compared with the early Pianet N drawing, the 650720 includes 5 nF at the swell input, a separate 100 kΩ resistor from the swell wiper to the low jack and a 600 kΩ/200 kΩ divider at the middle jack. The shared transformer and small-signal AC107/AC125 operating parameters are estimates. The model omits supply sag, transformer saturation, transistor overload, the external CP amplifier and its speakers. [N instructions on level, load and working voltage, p. 3](https://hohner-cembalet.com/pix/Cembalet_N_Bedienungs_u_Service_Anleitung_S_03_kl.jpg) · [650720 drawing](https://www.clavinet.com/Cembalet_N_schematic.pdf)

The independent ngspice deck agrees with the three modeled jack transfer functions to 0.0016 dB and 0.044° from 20 Hz to 20 kHz after bilinear frequency mapping. A separately rendered C4–E4–G4 at velocity 80 and full swell gives the following jack readings over the first 200 ms.

| Jack | Hohner midrange triad | Receiver load | Model peak | Model RMS |
| --- | ---: | ---: | ---: | ---: |
| Low | 100 mV | 100 kΩ | 25.3 mV | 9.2 mV |
| Middle | 300 mV | 200 kΩ | 65.4 mV | 23.4 mV |
| High | 1,500 mV | 7 kΩ | 233.6 mV | 83.6 mV |

The absolute output falls below all three factory figures. The manual leaves specific notes, meter and measurement window unspecified. Establishing electrode charge swing, pickup-transformer ratio and loaded transistor operating points on an identified board would distinguish the remaining gain and port-ratio errors. The host's output trim is separate from these voltages.

## Recording comparison

Cinematique Instruments recorded a Cembalet with line, original tremolo and microphone articulations. Its “Just Playing” demo is a performance of that library. Its board revision, articulation, processing and key timing are unspecified. Across four G♯2 bass phrases beginning at 2.30, 10.35, 18.45 and 26.40 s, 200 ms Hann projections spaced 80 ms apart give slopes of −11.43, −11.43, −13.73 and −13.23 dB/s, with 0.71–1.33 dB RMS fit residual. Corresponding isolated models give −8.18 and −8.50 dB/s. The recording therefore decays faster in these passages. Isolated, dry, serial-identified notes with key travel and exact output loading are still needed to qualify attack polarity, level, damping and the differences between revisions. [Publisher and recording details](https://cinematique-instruments.com/sites2025/instr_ep_cembalet.php)

```sh
cargo run -p pianet-n --example cembalet -- target/cembalet-proof
curl -fL https://cinematique-instruments.com/content_audio/vep_cembaletPlay.mp3 -o target/cembalet-proof/just-playing.mp3
python3 pianet-n/research/compare_cembalet.py target/cembalet-proof --sampled-performance target/cembalet-proof/just-playing.mp3
```

The optional demo comparison pins the published file by SHA-256. The two audition WAVs contain C2, C4 and C6 at two key speeds followed by a modulated chord. Additional fixtures preserve contact and damper forces, tip displacement, capacitance, key travel, RF detector output, jack impulse, register balance and the service triad.
