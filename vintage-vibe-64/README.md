# Vintage Vibe 64 Classic

The model follows the original passive 64 Classic specification, with A1–C7, a struck asymmetrical tuning fork at each key, a bridle-operated damper, a common sustain pedal and a mono jack. Vintage Vibe based the 2013 Classic on its 64 Custom action and offered a separate powered stereo option. The recording comparison uses the maker's later passive-64 performance series. The serial number and production revision of that piano are unknown. [2013 launch sheet, p. 3](https://www.fenderrhodes.com/pdf/64classic.pdf) · [maker's identification of Mikko Taylor's piano](https://vimeo.com/217684956)


The powered StereoVibe option uses the serviced Rev 4.1 panel, the same 64-key harp, three selectable input circuits and separately rendered left and right optical tremolo outputs. The maker’s 2014 active Classic recording and two other demonstrations provide the stereo benchmarks. [StereoVibe sources, limits, measurements and listening fixtures](research/stereovibe.md)

## Action and regulation

The maker's owner guide describes a hinged hammer with graduated neoprene tips, a tine and tonebar bolted to the same head, a movable tuning spring, an adjustable pickup and a damper strap. Horizontal pickup setback changes level. Vertical tine alignment changes harmonic balance. The guide's 3/8-inch setup dimension is the clearance under the **tonebar** above the harp. The 2018 Variable Voice rail and modern action revisions belong to later instruments. [Owner guide](https://cdn.shopify.com/s/files/1/0666/2821/files/owners_manual.pdf) · [maker's timeline](https://www.vintagevibe.com/pages/vintage-vibe-timeline) · [maker's design interview](https://www.vintagevibe.com/blogs/news/the-inside-story-of-the-vintage-vibe-electric-piano)

Each of the 64 free shaft lengths comes from Vintage Vibe's Rev C replacement-tine drawing, which the maker lists under Fender Rhodes service. Its six-inch calibration line sets the scale. A1 measures 110.728 mm and C7 measures 21.431 mm on that drawing. This supplies a reproducible replacement scale. Original per-key dimensions for the 2013 production instrument are still unmeasured. [Tine cutting chart](https://cdn.shopify.com/s/files/1/0666/2821/files/Tine_Cutting_Chart_RevC16APR18.pdf) · [service index](https://www.vintagevibe.com/pages/service-manuals) · [vector extraction](research/lengths.py)

The solver computes a unilateral Hertz impact, hammer recoil and separation. A pair of coupled normal modes carries tine and tonebar motion at the common head. Two higher Euler–Bernoulli modes use the cantilever displacement at the strike and spring positions. Equivalent shaft diameter, tuning mass, tonebar inertia, head compliance, strike line, hammer mass, tip stiffness and per-note decay are estimates. A positive higher-mode participation factor enters both the drive and observation coordinates. Its value and the nominal spring position were fitted to the B4 attack below. Each restrike keeps the fork's prior motion. A released damper approaches the tine over 5 ms and adds loss even during the tail of a chord once the pedal returns. [Action implementation](src/action.rs) · [shared beam primitive](../pleasant-dsp/src/beam.rs)

Voltage is the negative time derivative of flux through an offset end pole. The finite-radius field depends on both gap and tine motion. The estimated stock horizontal setback runs from 1.55 to 1.22 mm. The height changes from 0.60 to 1.10 mm and the effective pole radius is 1.2 mm. A global linkage of 0.00032 Wb-turn gives middle C at MIDI velocity 80 a modeled 32.0 mV RMS over its first 100 ms at the open panel. This is a level convention. The recordings establish relative spectrum and timing, with unknown capture gain, control positions and input impedance. [Maker's pickup adjustment instructions](https://cdn.shopify.com/s/files/1/0666/2821/files/owners_manual.pdf)

| Note at velocity 80 | Last hammer contact | Peak force | Maximum tine excursion in first 52 ms | First 100 ms at jack |
|---|---:|---:|---:|---:|
| A1 | 1.052 ms | 19.28 N | 1.253 mm | 9.28 mV RMS |
| C4 | 0.667 ms | 22.19 N | 0.782 mm | 32.01 mV RMS |
| C7 | 0.521 ms | 23.05 N | 0.253 mm | 87.72 mV RMS |

## Passive electronics

The launch sheet lists volume, low and high frequency controls. The maker describes pots and capacitors that cut the bass or treble and a mono output. The original pot codes, tapers, capacitances, winding bank diagram and measured complex impedance are outstanding. The solver therefore implements a **proposed passive network** with exposed source, loaded harp and jack voltages. A pickup resonance can raise a narrow band even though the network has no active gain. [Maker's passive electronics description](https://www.vintagevibe.com/blogs/news/vintage-vibe-pianos-active-vs-passive) · [network implementation](src/panel.rs)

| Element | Assumed value and connection |
|---|---|
| Equivalent harp | 2.5 kΩ and 0.90 H in series, 150 pF shunt capacitance |
| Bass | 2.2 nF in series, bypassed by a variable 0–500 kΩ resistance |
| Treble | Shunt branch of 0–500 kΩ in series with 22 nF |
| Volume | 250 kΩ divider with an estimated audio taper |
| Cable and receiver | 470 pF and 1 MΩ to ground at the jack |

Opening all three controls gives −0.14 dB at 40 Hz and +3.18 dB at 4 kHz. Full bass cut gives −12.65 dB at 40 Hz and +1.21 dB at 4 kHz. Full treble cut gives −0.10 dB at 40 Hz and −21.54 dB at 4 kHz. The digital panel agreed with an independent ngspice AC solve of these assumed parts to 0.0021 dB across 24 points and four panel settings. Hardware response remains unmeasured. [ngspice comparison and netlist](research/panel.py)

## Recordings and outstanding differences

Vintage Vibe published two Bach performances by Mikko Taylor on YouTube. Its Vimeo performance series names his instrument as a new passive 64. The timecodes below refer to the YouTube versions. They contain live playing, unknown velocities and some simultaneous voices. The scripts use local decoded 48 kHz mono copies. [Taylor, part 1](https://www.youtube.com/watch?v=3qv2LOGmTsY) · [Taylor, part 2](https://www.youtube.com/watch?v=IMr0kmhSSOA) · [maker's passive 64 identification](https://vimeo.com/217684956) · [analysis](research/performances.py)

Part 2 starts with an isolated B4 at 1.226 s. Its first nonharmonic bending peak occurs at 3,535 Hz in a Hann window spanning 8–36 ms after onset. With the initial 0.83-length spring location the model put that peak at 4,815 Hz. Moving the effective position to 0.939 of the free length gave 3,535 Hz. The fitted higher-mode participation yields −43.6 dBc against −40.6 dBc in the recording. The 5 ms RMS 10–90% attack lasts 2.25 ms in the model and 4.44 ms in the recording. The modeled release falls 20 dB in 11.48 ms against 11.10 ms after fitting the damper loss.

| Other attacks in part 2 | Window onset | Recorded bending peak | Modeled bending peak | Recorded / modeled level relative to F0 |
|---|---:|---:|---:|---:|
| A4 | 95.407 s | 2,990 Hz | 3,174 Hz | −45.8 / −32.2 dBc |
| D5 | 96.485 s | 4,121 Hz | 4,215 Hz | −46.1 / −35.0 dBc |
| E5 | 125.878 s | 4,535 Hz | 4,712 Hz | −44.4 / −30.9 dBc |

These three peaks run 2.3–6.2% high and 11.1–13.6 dB loud. One global spring position and bending mass cannot recover the piano's individual regulation. For nine 90 ms tone windows, third-harmonic level selected the nearest simulated velocity from a fixed grid. Comparing the second harmonic separately gives a median absolute discrepancy of 6.5 dB and a worst discrepancy of 19.3 dB on the part 1 A5. The part 1 D3 reaches the model's maximum velocity and still misses H3 by 10.5 dB. These figures include pitch overlap where another player voice falls inside the harmonic bins. The published performances do not isolate the original passive board. [Measurement and timestamps](research/performances.py)

## Matched strikes against Stage 73

The comparison drives both live engines at MIDI velocities 48, 80 and 110 with identical timing, rates, native voicing and open stock panels. Both engines use the same manufacturer replacement chart on the overlapping keys, rounded to 0.001 mm here. Its length values supply no independent evidence of a Vintage Vibe versus Rhodes scale difference. The Stage 73 model uses its late Mark I pickup bank weights and calibrated harp tap. The table gives the fraction of attack energy above 1.8 kHz, in dB relative to the total in the first 35 ms, at velocity 80. The last column substitutes an exponential tine-length scale with the same two endpoint lengths while retaining every other Vintage Vibe parameter. [Comparison procedure](research/stage_comparison.py) · [Stage 73 calibration and probe](../stage73/examples/probe.rs)

| Note | Stage 73 harp | Vintage Vibe source | Vintage Vibe jack | Vintage Vibe source with smooth lengths |
|---|---:|---:|---:|---:|
| E2 | −39.14 | −35.95 | −35.23 | −35.18 |
| C4 | −41.37 | −17.34 | −16.61 | −17.43 |
| C6 | −22.80 | −31.69 | −29.74 | −29.55 |

At C4 the 24.0 dB difference is already present at the two source taps. The estimated Vintage Vibe panel adds 0.73 dB to that energy fraction, and substituting smooth lengths changes it by 0.09 dB. At C6 the chart substitution changes it by 2.14 dB and the panel by 1.95 dB. Most of the difference in these strikes comes from the combined hammer, modal and magnetic models, whose individual contributions the recordings do not identify. At C6 the open Stage jack gives H2 at −24.49 dBc versus −43.96 dBc at the Vintage Vibe jack. At C4 they give −15.87 and −15.36 dBc, despite the difference in the inharmonic attack.

## Host use and reproduction

`live::Engine::new(rate)` prepares 128 actions. The native range is MIDI 33–96. `set_extended(true)` enables MIDI 0–127 at the requested pitch using the nearest physical regulation beyond either end. `set_pickup_gap_mm` moves the pickups by −0.25 to +2 mm around the default of zero. Changing the gap rebases the flux, preserving mechanical motion. `set_sustain`, MIDI and audition note ownership, choke, reset and block rendering are allocation-free. `next_outputs()` returns open-source, loaded-harp and mono-jack volts. `render` returns mono jack volts. The common anti-alias FIR adds 12 host frames. A release build rendered 1/8/16/32/64 active notes in 9/39/90/148/200 ms per audio second at 44.1 kHz, 13/35/80/135/242 ms at 48 kHz and 14/43/73/182/238 ms at 96 kHz. These are single runs on the development machine.

```sh
cargo run --release -p vintage-vibe-64 --example vv64_probe -- captures target/vv64-probe
cargo run --release -p vintage-vibe-64 --example vv64_probe -- references target/vv64-reference
python3 vintage-vibe-64/research/performances.py PART1_48K_MONO.wav PART2_48K_MONO.wav target/vv64-reference
python3 vintage-vibe-64/research/panel.py
cargo build --release -p stage73 --example probe
python3 vintage-vibe-64/research/stage_comparison.py target/vv64-comparison
cargo run --release -p vintage-vibe-64 --example vv64_probe -- bench
```

The recording and Stage 73 comparisons require NumPy and SoundFile. The Stage comparison also uses SciPy. The panel verification requires NumPy and ngspice.
