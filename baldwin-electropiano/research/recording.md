# Baldwin Electropiano owner recordings

**Chad Tipps identifies his instrument as an 88-key Baldwin.** At 0:09–0:15 he states its compass. The wide view shows an upright classroom cabinet and a pedal assembly. The nameplate at 0:20–0:25 identifies the D. H. Baldwin Company in Cincinnati, Ohio. The available frame does not resolve a model or serial stamp. At 0:26–0:58 he describes removing the original amplifier and speakers and summing the two existing low- and high-register outputs into a mono jack. The 0:45–1:00 and 3:05–3:18 views show copper-coloured bass strings crossing finer strings on a plate and the exposed termination area. The camera does not trace a connection from a named string to a transducer or expose each ceramic. The footage supports using two electrical leads. The six-element bass bank, its 24-note crossover and the exact number of elements are estimates. [Tipps, exterior and nameplate](https://www.youtube.com/watch?v=z9xAuYLEOTs&t=9s) · [Pickup modification](https://www.youtube.com/watch?v=z9xAuYLEOTs&t=42s) · [Exposed strings](https://www.youtube.com/watch?v=z9xAuYLEOTs&t=185s)

LeCroy’s printed p. 140 reproduces the Baldwin classroom specification. It identifies an 88-note version, optional 64-note version, conventional vertical let-off, birch and felt hammers, copper-wrapped bass, single treble strings, pressure transducers, mechanical sustain, electronic soft pedal and solid-state amplification. The US trade investigation also distinguishes 64- and 88-note instruments. Martin and Stein’s figures 1–4 show piezoceramics compressed between the arms of an aluminium rail, a shared electrical ground, pickup shims and a resilient support against the plate. The text gives roughly 20–25 elements for 88 notes and permits both single-string and multistring constructions. Technician Bill Maxim recalls a production transition from three strings to one. Taken together these establish a plausible configuration. The images do not identify the specific patent embodiment or amplifier revision in the filmed piano. [Baldwin specification, p. 140](https://digital.library.unt.edu/ark:/67531/metadc662988/m2/1/high_res_d/1002773114-LeCroy.pdf#page=145) · [USITC report 721, p. A-60](https://www.usitc.gov/publications/337/pub721.pdf) · [Baldwin patent US3685384A](https://patents.google.com/patent/US3685384A/en) · [Bill Maxim’s service account](https://www.mmdigest.com/archives/Digests/200206/2002.06.11.02.html)

## Hardware selections

Tipps plays through a pedalboard and guitar amplifier. The selected passage runs 1:23.5–1:38.4 after he describes the clean setting with delay off. Its F♯3 window accounts for 92% of the window’s energy in one harmonic family. The hand changes notes immediately afterward. His D2 window includes an apparent octave and his E4 window contains other notes. Both are included in the listening reel and excluded from accuracy summaries. The later overdrive and delay passages are excluded. [Tipps, clean passage](https://www.youtube.com/watch?v=z9xAuYLEOTs&t=83s)

Steve Roberti plays a Baldwin after cutting down the cabinet top. Camera footage shows the exposed action and both hands playing. Emmanuel Martinez plays a second Baldwin classroom-style instrument with its lid open and the action visible. Neither supplies a serial, wiring trace, recording chain, velocity, pedal-state track or isolated note stems. Even a well-fitted harmonic family can contain a doubled octave. These recordings test how the synthesis compares across instruments and performances. They cannot calibrate the original amplifier’s gain or frequency response. [Roberti’s altered Baldwin](https://www.youtube.com/watch?v=jQSGJi1iiCc) · [Martinez’s open-lid Baldwin](https://www.youtube.com/watch?v=-AMw0zXeHE4)

| Owner | Note | Window centre, s | Single harmonic-family energy | Hardware partials 2 / 3 / 4, dB relative to fundamental |
| --- | --- | ---: | ---: | --- |
| Tipps | F♯3 | 85.90 | 92% | −18.4 / −12.4 / −9.3 |
| Roberti | D2 | 48.12 | 98% | −7.1 / −1.3 / −3.5 |
| Roberti | A4 | 30.06 | 91% | −21.0 / −30.0 / −30.3 |
| Roberti | D5 | 44.34 | 93% | −38.2 / −31.4 / −47.6 |
| Roberti | E5 | 49.56 | 99% | −44.6 / −46.2 / −68.5 |
| Martinez | G2 | 74.22 | 99% | −3.3 / −11.4 / −9.1 |
| Martinez | F3 | 31.56 | 96% | −10.6 / −26.7 / −27.3 |
| Martinez | G4 | 51.06 | 96% | −25.7 / −60.2 / −68.9 |

Each spectrum uses a 160 ms Hann window. The selection metric projects a harmonic family at its local tuning. For the partial amplitudes the script separately fits the fundamental and the first six modes of a stiff string, allowing the model’s upper partials to sharpen with mode order. The model spectrum is centred 210 ms after a velocity-0.7 key-down. Owner velocity and exact attack age differ between performances. The model channel is the jack in volts. Listening pairs are matched by RMS per note.

## Motion and decay

| Owner and note | Fundamental measurement | Current modeled jack |
| --- | --- | --- |
| Tipps F♯3 | 10–90% rise from 85.469–85.490 s, 21 ms | 7 ms |
| Roberti E5 | 10–90% rise from 49.468–49.496 s, 28 ms | 7 ms |
| Roberti D2 | 5.13 dB/s, 48.18–48.74 s | 2.77 dB/s |
| Roberti E4 | 4.35 dB/s, 25.70–26.26 s | 5.19 dB/s |
| Roberti E5 | 9.23 dB/s, 49.56–50.04 s | 26.66 dB/s |
| Martinez G2 | 3.13 dB/s, 48.68–49.24 s | 2.65 dB/s |
| Martinez G4 | 21.08 dB decline, 122.84–122.99 s | 75.24 dB from 20 ms before to 130 ms after commanded key-up |

The attack figures time the rise of the recorded fundamental above its local background. The analysis window is at least 12 ms or three cycles wide. Acoustic and pickup latency, compression and player velocity contribute. For decay the script fits a line to 120 ms fundamental measurements at 40 ms intervals. The four owner fits have residuals of 0.05–0.16 dB RMS. At 122.89 s Martinez changes hand position as G4 falls away. Other notes and room sound persist. The damper and pedal are out of view, so the G4 observation bounds the audible transition without establishing a mechanical release time.

## Choosing the hammer and shim

The first pilot put Roberti’s E5 second harmonic 20.1 dB above its fundamental. The recording puts it 44.6 dB below. We separated the hammer’s bearing reaction from the compression rail’s transfer, swept 120 hammer setups and 20 shim setups, and ranked combinations on D2, F♯3, A4 and D5. The selected setup scales the initial estimated contact stiffness by 0.035, hammer mass by 0.70 and strike line by 0.75. The preloaded supports use 250 kN/m stiffness and 10 N·s/m damping. Several material combinations yield similar spectra. These are effective values fitted to amplified performances.

The table gives median absolute error over partials two through six in the complete jack render, with a −40 dBc floor applied to both the recording and model. Both columns use the same final string scale. Bass fundamental loss was also informed by D2 and G2 decays, so G2 is reserved for the harmonic fit only.

| Harmonic-fit group | Initial hammer and shim | Selected hammer and shim |
| --- | ---: | ---: |
| Four fit keys | 12.96 dB | 3.11 dB |
| Four reserved keys, G2, F3, G4 and E5 | 12.86 dB | 10.17 dB |
| Martinez’s three keys alone | 6.10 dB | 16.42 dB |

The model improves the harmonic balance on G2 and G4, while Martinez’s F3 error rises from 4.83 to 16.67 dB. On Roberti’s reserved E5, three measured upper harmonics fall below the scoring floor. Its second harmonic remains 27.7 dB too strong in the model. Matching the original instrument’s independent register transfers and recording a known key velocity would help distinguish hammer compliance from rail and amplifier response.

## Reproduction

```sh
cargo run --release -p baldwin-electropiano --example probe -- target/baldwin-probe/stock
python3 baldwin-electropiano/research/recording.py target/baldwin-probe \
  --tipps /private/tmp/baldwin-tipps.wav --roberti /private/tmp/baldwin-roberti.wav \
  --martinez /private/tmp/baldwin-martinez.wav
```

Decode the linked owner recordings to 22,050 Hz mono PCM for the three input paths. The script checks each decoded file against the reference SHA-256 digest and prints any mismatch. It writes `hardware-harmonics.csv` and three `*-then-model.wav` listening reels in the probe directory, in the event order above, with the two contextual Tipps chords after his F♯3. The reels and source recordings are local research outputs.

To repeat the fit with NumPy, SciPy and SoundFile installed, render the initial hammer on the same scale and use its measured spectrum as the baseline.

```sh
cargo run --release -p baldwin-electropiano --example probe -- target/baldwin-probe/baseline/stock --unvoiced
python3 baldwin-electropiano/research/recording.py target/baldwin-probe/baseline \
  --tipps /private/tmp/baldwin-tipps.wav --roberti /private/tmp/baldwin-roberti.wav \
  --martinez /private/tmp/baldwin-martinez.wav
cargo run --quiet --release -p baldwin-electropiano --example voice > target/baldwin-probe/hammer-candidates.csv
cargo run --quiet --release -p baldwin-electropiano --example shim > target/baldwin-probe/shim-candidates.csv
python3 baldwin-electropiano/research/voicing.py target/baldwin-probe target/baldwin-probe/baseline/hardware-harmonics.csv
```
