# 72-key Electra-Piano engine

The live engine instantiates one continuously moving fork per key from F1 to E7 (MIDI 29–100). C3, F4 and G♯5 retain the reference parameters. Between these notes each positive mass, spring ratio, contact stiffness, field width and damping time follows a power law in pitch. The same slopes extrapolate below C3 and above G♯5. The scale therefore contains three experimentally informed actions and 69 inferred ones. Drawing 680209 identifies the outer coil groups. A ten-note division of each middle group is a provisional wiring map. [Engine and scale](../src/live.rs) · [Hohner service guide](https://www.clavinet.com/hohner_electrapiano_service_manual.pdf)

## Host interface

`Engine::new(sample_rate)` allocates all voices, with a minimum supported host rate of 8 kHz. `note_on(note, velocity)` accepts normalized velocity and sets estimated impact speed to 1.7 × velocity in metres per second. No voice stealing is necessary. A restrike reapplies the hammer to the current fork position. `note_off`, `audition`, `audition_off`, `held_notes`, `choke`, `sounding` and `reset` handle key ownership, bounded choke fades and transport resets. `set_extended(true)` enables MIDI 0–127. Added notes inherit the closest factory fork’s masses, coupling, hammer, damper and magnetic loading while retuning the lower coupled eigenfrequency to the event pitch. They use the nearest series group and coil polarity. These notes are playable extrapolations beyond the cabinet. Disabling the standalone option retires extended notes with a 4 ms fade and preserves held native notes.

`set_pickup_gap_mm(offset)` moves the pickup radially by −0.25…+2.0 mm around the fitted gap. The signed offset enters the distance in the flux law at every oversample. It changes the slope and curvature with the mechanical trajectory held fixed. The zero setting recovers the original field exactly.

A silent engine retires only when modal velocity envelopes and circuit history have decayed. Normal processing, MIDI events and panel automation allocate no heap storage.

`set_sustain(bool)` lifts or reapplies the dampers. `set_controls(Controls)` configures the seven coil trims, P9 basic volume, main volume and the left electrical pedal with adjustable P8 depth. A block with unchanged controls reuses circuit coefficients. A changed control took about 25 µs to recompile on this machine. The instrument has zero frames of host-compensated latency. The fork and pickup operate at four times the host rate at 44.1/48/96 kHz. Lower host rates use additional substeps to keep contact integration at or above 176.4 kHz. The output retains the physical and electronic phase response. At 48 kHz, MIDI 29/48/65/80/100 crossed 1% of their own first-100-ms peak at 16/20/10/6/3 samples after note-on.

`render(&mut [f32])` writes the additional-amplifier jack in volts. `next_outputs()` advances once and returns the full-precision auxiliary and 4 Ω speaker-terminal voltages from that same circuit state. The regression fixture compares both taps sample for sample against each reference fork through the right pedal, key release and electrical soft switch at all three main sample rates. A second test strikes every key, including both sides of every assumed coil-group boundary, and verifies both terminals at four sample rates. [Tests and API](../src/live.rs)

## Off-anchor hardware comparison

Eight additional windows on seven previously unused pitches test interpolation and extrapolation. All model keys used 0.85 m/s, the default panel and a 1 MΩ auxiliary load. The player’s strike speeds, regulation, electrical settings and release times were unrecorded. The script checks a fourfold amplitude margin over the local spectral floor before reporting H2 or H3. Values are hardware / model. [Owner’s effects-off Peavey performance](https://www.youtube.com/watch?v=84P3RGgeqvU) · [Comparison windows and method](scale.py)

| Note | Onset (s) | F0 (Hz) | H2/H1 (dB) | H3/H1 (dB) | Early fundamental decay (dB/s) |
| --- | ---: | ---: | ---: | ---: | ---: |
| C2 | 543.904 | 65.58 / 65.33 | +10.85 / −3.06 | +11.76 / −1.28 | −1.62 / −4.16 |
| C2 repeat | 357.579 | 65.14 / 65.33 | +2.80 / −3.06 | +4.58 / −1.28 | −5.69 / −3.55 |
| G2 | 420.337 | 97.87 / 98.00 | −2.72 / −5.39 | unresolved / −3.99 | −3.83 / −5.19 |
| C4 | 949.886 | 261.33 / 261.63 | −21.54 / −11.98 | −8.88 / −10.70 | −5.77 / −6.46 |
| B♭4 | 874.428 | 464.88 / 466.16 | −21.50 / −19.06 | −15.43 / −17.76 | −11.55 / −15.87 |
| D5 | 144.551 | 583.41 / 587.33 | −11.37 / −22.62 | −9.67 / −21.57 | −17.41 / −21.11 |
| F6 | 727.058 | 1390.41 / 1396.91 | −33.60 / −44.58 | −59.92 / −54.84 | −34.76 / −67.41 |
| F♯6 | 328.812 | 1473.47 / 1479.98 | −29.32 / −46.03 | −54.60 / −57.61 | −77.64 / −72.42 |

B♭4 reproduced H2/H3 within 2.5 dB. Both C2 takes had stronger upper partials than the model. Their own H2 levels differed by 8 dB. The model understated F♯6 H2 by 16.7 dB and the F6 fundamental decayed about 33 dB/s faster than the performance. Acquisition of calibrated velocities, actual pickup clearances, upper-register damping transitions and the coil/transformer loading would distinguish excitation, pickup and damper contributions. Pitch, decay and harmonic results across the full compass have substantially different evidence coverage from the three-key calibration. The remaining 62 keys have functional numerical checks and lack note-matched comparisons here.

## CPU and headroom

The release probe timed three one-second, simultaneous fortissimo strikes per configuration on an arm64 Mac, in 128-frame blocks. Each entry below is p99 wall time as a percentage of the block deadline. Each chord is contiguous and centered in the native compass. Performance under a host also depends on the block size and other loaded effects. [Benchmark implementation](../examples/electra_live.rs)

| Held keys | 44.1 kHz | 48 kHz | 96 kHz |
| --- | ---: | ---: | ---: |
| 1 | 0.7% | 1.7% | 1.7% |
| 8 | 2.7% | 3.6% | 6.3% |
| 16 | 7.7% | 5.5% | 10.6% |
| 32 | 10.0% | 11.0% | 21.6% |
| 72 | 19.7% | 23.2% | 67.1% |

In this run, all 44.1 and 48 kHz blocks met their deadlines. The 72-key, 96 kHz stress missed six of 2,250 deadlines, with a 37.2% median and a 218% maximum. No other 96 kHz chord missed a deadline. A repeat of the 72-key, 96 kHz case had a 51.3% p99, a 103.9% maximum and one missed deadline. The table describes wall-clock observations on a shared machine.

| Simultaneous notes at 48 kHz | Default auxiliary peak (V) | Default speaker peak (V) | Auxiliary peak at full P9 and main volume (V) |
| --- | ---: | ---: | ---: |
| 1 | 0.00731 | 1.20 | 0.03750 |
| 8 | 0.03132 | 5.04 | 0.17405 |
| 16 | 0.04347 | 7.18 | 0.22000 |
| 32 | 0.06198 | 10.20 | 0.32155 |
| 72 | 0.07312 | 11.91 | 0.38236 |

The absolute pickup sensitivity, transformer ratio and amplifier headroom remain estimates. Dense full-volume chords can exceed the real amplifier's linear swing. Floating output preserves the solver's voltage for examination.

```sh
cargo run --release -p electra-piano --example electra_live -- bench
cargo run --release -p electra-piano --example electra_live -- captures target/electra-live-proof
python3 electra-piano/research/scale.py target/electra-live-proof /path/to/peavey.wav
```
