# Vintage Vibe Vibanet

Electric Pianos models the 60-key Vibanet shown in Vintage Vibe’s 2014 demonstration. It exposes both pickup solos, Fulltone and Phase, three five-position EQ switches, auto-wah sensitivity, mute bar, volume and the two pickup clearances. The factory film supplies the panel positions and the performance used below. Pickup clearance offsets and MIDI 0–127 extension are plugin controls. [Factory film](https://www.youtube.com/watch?v=uEyWJBebwoA) · [Factory description and photographs](https://www.vintagevibe.com/blogs/news/get-funky-with-the-vibanet)

## What changes from the Clavinet D6

- The same moving tangent clamps a stiff string against the anvil. Two preloaded polymer strips enclose a section of its afterlength. Their relaxed and transient shear springs store energy, and a dashpot in series with the transient branch removes energy as it creeps. Opening the tangent releases both string segments with their displacement, velocity and gel strain intact. The factory shows the strips and their retaining clips. The 34 mm contact region, effective line stiffnesses of 20 and 170 kN/m², and 0.35 ms relaxation time are effective parameters. A release fit varied the relaxation time from an initial 1.2 ms. The maker documents custom wound strings. The speaking lengths, wound-string densities, tangent travel and nominal rail positions here still use the D6 scale. [Factory gel instructions](https://www.vintagevibe.com/products/clavigel-kit) · [String and gel solver](../../src/vibanet.rs)
- The film shows four selectable positions. Around 88–150 seconds the technician plays a similar figure in each. Lead has more upper harmonic energy relative to Rhythm on the matched bass notes. The model assigns Lead to the bridge tap and Rhythm to the central tap. The official virtual instrument’s guide gives the reverse positional assignment. For the filmed unit the chosen assignment predicts the direction of the solo difference on four pitches. The exact terminal polarity and wiring of that instrument still need a continuity trace. Fulltone sums the two EMFs and Phase reverses one before summing. The related factory installation drawing identifies separate upper and lower terminals. [Keyscape reference guide, pp. 207–208](https://device.report/m/20efc34f46785bc1b7a976f288e7a55fd05ac284e410f64355ed58af4d5b2b51.pdf) · [Factory terminal drawing](https://cdn.shopify.com/s/files/1/0666/2821/files/vibanet-pcb-connections.jpg)
- The manufacturer reports matching wire, turns and magnets for both pickups and an ultrasonic resonance. Each electrical source uses a provisional 200 Ω and 100 mH, a 1 MΩ load and 220 pF at the input. This places the unloaded single-coil LC resonance near 34 kHz. The EQ and wah follow the two pickups at instrument level. The filter curves are fitted to the filmed performance. Production preamp topology, gain, detector tap, supply clipping and pot taper remain unmeasured. [Factory pickup description](https://www.vintagevibe.com/products/clavinet-vibanet-pick-up) · [Related preamp](https://www.vintagevibe.com/products/clavinet-vibanet-preamp-kit)

## Comparison with the filmed instrument

**Pickup switches.** Across H1–H16, fit one gain offset per dryer 160 ms single-note window and keep lines at least 6 dB above the local interharmonic floor. The model’s median shape errors on the reserved pitches were 5.84 dB in Lead, 5.79 in Rhythm, 6.71 in Fulltone and 7.13 in Phase. Some windows contain too few resolved partials to include, particularly D2. The Lead-minus-Rhythm difference at G2 had 4.69 dB median error, compared with 6.79 dB for identical spectra. At A2 the corresponding figures were 7.53 and 14.27 dB. Moving the bridge rail 17% improved a sine-mode approximation on G2 but worsened A2, so the shared nominal geometry was retained. The alternating positions were played on different strikes. [Timestamped windows and measurement](pickups.py)

**Release.** Detect key-up from the first 7.5% fall in the measured fundamental. Measure the time for broadband RMS to stay 12 dB below its held level for 10 ms. The key-travel and strike-force traces are unavailable. The model uses the approximate gate from the last audible attack to pitch drop and the same velocity on every strike. Gate timing matters because the gel loads a string still in motion. [Release timing and complete set of nine events](release.py)

| Recording setting and pitch | Approximate gate | Hardware T12 | Model T12 |
| --- | ---: | ---: | ---: |
| Lead F2 | 134 ms | 62 ms | 74 ms |
| Rhythm F2 | 134 ms | 76 ms | 74 ms |
| Rhythm F2, second strike | 126 ms | 98 ms | >160 ms |
| Fulltone F2, two strikes | 122 ms | 94 / 88 ms | 114 ms |
| Rhythm B♭2 | 114 ms | 32 ms | 110 ms |
| Fulltone B♭2 | 154 ms | 42 ms | 142 ms |
| Phase G2 | 118 ms | 138 ms | 66 ms |
| Fulltone G2 | 146 ms | >160 ms | >160 ms |

**Fixed EQ.** The five settings in the film are −15, −7.5, 0, +7.5 and +15 dB. Estimate a low shelf, mid bell and high shelf at the outer settings, allowing separate unknown levels for each manual chord member. Then check the intermediate settings. The estimated frequencies are 280 Hz, 1.83 kHz and 6.1 kHz with Q values 0.45, 0.55 and 0.73. Median shape errors at the two intermediate settings are respectively 3.78 / 1.65, 2.77 / 2.93 and 3.48 / 3.43 dB. The high-band endpoint fit gives little discrimination against a flat transfer, and the low cut is weaker than the no-shaping prediction on the reserved −7.5 dB example. Repeated struck notes cannot identify the circuit uniquely. [EQ caption times, fit and held-out scores](eq.py)

**Auto-wah.** Four notes begin at 226.635, 227.249, 227.839 and 228.416 seconds. Compare the movement of individual harmonics from 25 to 200 ms after attack, relative to their late levels. A rectified instrument-bus detector controls a resonant low-pass with a 310–4950 Hz range. The fit used A2 and G2, a fixed 150 ms detector attack and 82 ms release, and a Q near 3.05. Detector scaling and attack time trade off against one another. Fundamental level provides a relative drive estimate. The Rust render below gives all four keys the same velocity and sensitivity. It predicts more of the moving spectrum on E2 than a static filter. Neither version consistently captures the fast F♯2 sweep. [Detector fit and validation](wah.py)

| Pitch | Comparison role | Fixed-strike Rust error | Constant-filter error |
| --- | --- | ---: | ---: |
| A2 | fit | 2.87 dB | 5.18 dB |
| G2 | fit | 3.31 dB | 7.06 dB |
| F♯2 | reserved | 3.55 dB | 3.33 dB |
| E2 | reserved | 5.42 dB | 7.07 dB |

## Reproduce

Obtain the 2014 maker film linked above and run from the workspace root. Scripts use Python with NumPy, SciPy and SoundFile. The audition pairs are for local comparison. The plugin synthesizes the sound.

```sh
cargo run --release -p clavinet-d6 --example vibanet -- target/vibanet/strikes
ffmpeg -i factory-film.mkv -map 0:a:0 -ac 1 -ar 22050 factory-mono.wav
python3 clavinet-d6/research/vibanet/eq.py factory-film.mkv target/vibanet/eq
python3 clavinet-d6/research/vibanet/pickups.py factory-mono.wav target/vibanet/strikes target/vibanet/pickups.csv
python3 clavinet-d6/research/vibanet/release.py factory-mono.wav target/vibanet/strikes
python3 clavinet-d6/research/vibanet/wah.py factory-mono.wav target/vibanet/strikes
python3 clavinet-d6/research/vibanet/audition.py factory-mono.wav target/vibanet/strikes target/vibanet
```

Each comparison WAV from the Rust example contains centre open-circuit volts, bridge open-circuit volts, jack volts and detector cutoff divided by 10,000. The listening script pairs each hardware excerpt with the corresponding model jack and matches RMS for audition. After sharing the live tangent-keyboard code across the D6, E7 and Vibanet, all 37 native D6 fixture WAVs and all 92 native Duo fixture WAVs remained byte-identical.
