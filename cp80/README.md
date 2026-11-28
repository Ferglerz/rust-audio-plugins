# Electric Grand 80

The 88-key, sample-free string model in the Electric Pianos CLAP and VST3 instrument. It models the original Yamaha CP-80 documented in September 1978. The stereo outputs correspond to the instrument's volume-controlled jacks. The plugin opens with Medium brilliance, centered EQ, full volume and tremolo off.

Electric Grand also offers the 73-key Yamaha CP-70B with its own stringing, stretch tuning, pickup branches and panel circuit. See the [CP-70B model, renders and calibration](research/cp70b/README.md).

Play A0–C8 from MIDI or the editor keyboard. CC64 operates the dampers. A restrike excites the strings already in motion. The uppermost 15 keys keep ringing when released. Jack Volume turns the dual 100 kΩ pot feeding FET9 and FET8. The shared master follows the instrument. Both levels, EQ, tremolo and brilliance are automatable. Processing reports 16 samples of latency.

## Mechanical and electrical evidence

The String Action cutaway follows MIDI and the editor keyboard. It shows the selected key’s hammer, string course and piezo at the bearing, with the strike slowed for visibility. Click and hold the hammer to audition that pitch. Simultaneous strikes display the highest new note.

| Part | Implementation and evidence |
| --- | --- |
| Stringing and tuning | One wound string through F♯2, paired wound strings from G2 to D4, and paired plain strings from E♭4 to C8. All 88 stretch offsets follow Yamaha's table. The plain-wire stiffness interpolation accounts for the documented wire-number changes using nominal European diameters. [Parts list, pp. 28–31](https://manuals.fdiskc.com/tree/Yamaha/Yamaha%20Electric%20Grand%20Parts%20List%20for%20CP-70%20CP-70B%20CP-80.pdf) · [Service manual, p. 14](https://www.synthxl.com/wp-content/uploads/2020/04/Yamaha-CP-80-Service-Manual.pdf#page=14) · [Piano wire sizes](https://www.roeslau-draht.com/fileadmin/user_upload/PDF/Datenblaetter/EN/Overview_of_important_parameters.pdf) |
| Hammer and strings | A finite-width, compression-only hammer drives dispersive string modes. Its force grows quadratically with compression. B6 also has a linear stiffness term fitted to native strikes. End force drives the pickup. Speaking length, tension and the mapping from MIDI velocity to hammer speed are estimates. Effective contact stiffness, strike position, mass and losses use native-pitch PP and FF recordings. The effective parameters can compensate for one another. At the three verified rates the live solver integrates the collision at 176.4 or 192 kHz and doubles this for the top nine notes. It carries the free-vibration state between samples. The offline solver uses double precision and can run at 384 kHz. [Scale table](src/scale.rs) · [String and hammer equations](src/lib.rs) |
| Dampers and unisons | A common-support loss couples the two strings of each paired note. Unison spread defaults to an estimated 0.8 cent. B6 uses 0.35 cent from its recorded decay. Coupling strength and damper loss are estimated. CC64 has two positions. Frame Ring adds weak transfer between keys through a shared frame mode. Each receiving course uses its own stretch tuning and fundamental loss. The top 15 courses can respond with the pedal up. Frame Ring starts off. Its level, 300 Hz frame frequency and 190 Hz bandwidth are provisional pending a dry pedal comparison. |
| Pickup | All 88 piezos load the circuit, including silent keys. Keys 1–82 share the lower bus behind the 22 nF mixing capacitor. Keys 83–88 feed the upper bus. Each pickup has an estimated 1 nF capacitance. Absolute force-to-voltage sensitivity is provisional. A soft overload knee before decimation provides digital headroom for simultaneous strikes. [Yamaha drawing 006806](https://www.synthxl.com/wp-content/uploads/2020/04/yamaha_cp-80_overal_circuit.pdf) |
| Audio circuit | The component deck includes the JFETs, bias resistors, tone pots, both brilliance switch poles, normalled patch loop, jack buffers and transformer branches. The plugin solves a small-signal reduction with trapezoidal capacitor states at 4× host rate. Pot tapers, equivalent transistor curves, photocell rest resistance and transformer properties are fitted or assumed. The balanced outputs and patch jacks are observation points in the reference circuit. [Component deck](research/tr.cir) · [Live panel](src/panel.rs) |
| Stereo tremolo | Each channel follows its own simulated LED current from the RC4558 oscillator and inverting stage. Both traces share a time reference at three speed settings. The modulation depth accounts for the midband load of the line-buffer gates and the volume-controlled jack-buffer gates. Optical attack and recovery are estimated at 2 and 8 ms. The modeled speed range is 0.687–10.045 Hz. Both channels meet the factory fast-speed depth limits at full jack volume. [Oscillator and drivers](research/tremolo.cir) · [Factory adjustments](https://www.synthxl.com/wp-content/uploads/2020/04/Yamaha-CP-80-Service-Manual.pdf#page=11) |

## Calibration and limits

The reference set is Greg Sullivan's CP-80, distributed as an SFZ conversion by kinwie. Tests use the unlooped beginnings of the original FLAC recordings at their native pitches. The source contains mono, near-full-scale recordings across dynamic layers. Those files support relative harmonic and early-decay comparisons. Absolute velocity response, note-off, stereo-pedal captures and long-term unison beating still need direct instrument recordings. The library maps G2 to F♯2, so D3 provides a directly recorded paired-wound reference. The generated three-note prototype includes G2. [Sullivan's recording notes](https://www.sullivang.net/yamaha-cp80-electric-grand-instrument-for-gigasamplergigastudio) · [Reference files and license](https://github.com/sfzinstruments/GregSullivan.E-Pianos/tree/8c3e581acda3594b553948ff0222d4f84a698376)

On F♯2, D3 and E♭4, the two unused playing dynamics (MP and F) produced median errors of 1.45 dB for relative levels of partials 2–6, 1.18 cents for fundamental pitch and 0.77 dB/s for early fundamental decay. The first 70 ms differed by a median 2.78 dB at one fixed gain aligned using F♯2 PP and FF. The A/B alternates recording first and synthesis second, progressing from PP through FF on each pitch. All excerpts share one playback normalization. The included recording credit identifies the source, license and edits.

Nine complete pitches were excluded from the hammer, dispersion and loss fits. Across their 27 available strikes, the median absolute fundamental error was 1.69 cents. The median per-strike error of audible upper partial levels was 3.33 dB. The largest was 9.67 dB on E6 fortissimo. An earlier pass exposed a large error around E5. G♯5 then became a training pitch, and the intervening treble keys generally use interpolated PP and FF spectra. E5 stayed outside the fit, and its MP error fell from 29.64 to 5.12 dB. The model still needs direct treble captures with repeatable hammer velocities. [Generated contact values](src/contact.rs) · [Pitch comparison method](research/validate_keys.py)

### B♭5 hammer contact

Sullivan’s native MP and FF strikes constrain the B♭5 hammer contact and string decay. The MP fundamental resolves into two unequal components about 0.84 Hz apart. The model uses a 1.5-cent string pair and unequal bearing contributions, preserving some output through the beat minimum. The effective contact combines a 16.79 kN/m initial stiffness with a quadratic term. Neighboring keys bound the fundamental level because the recordings have different gains and B♭5 has no native PP recording.

At 44.1 kHz, third-partial errors relative to the fundamental over 65–240 ms are +0.30 dB at MP, +1.76 dB at the held-out F and +0.66 dB at FF. The preceding model measured −0.07, +10.28 and +9.77 dB. On F the fifth-partial error also fell from +13.76 to −1.1 dB. The F third partial remains 8.2 dB high over 250–400 ms, then 1.8 dB high over 400–620 ms. Reproduce the contact fit with `fit.py PROBE SAMPLES --note 82` and check the output against the native layers with `validate_keys.py`. [Hammer fit](research/fit.py) · [Measurement method](research/validate_keys.py)

### D5 hammer contact

D5 (MIDI 74) has no native strike in the Sullivan archive. Its original interpolated fit scored partials relative to the fundamental. A soft quadratic spring and a heavy hammer could therefore pass the fit while leaving very little energy in the ringing string. At velocity 92 the simulated hammer last touched the strings 3.87 ms into the strike. From 50 to 150 ms the D5 fundamental was 18.15 dB below the mean level in dB of C♯5 and E♭5.

D5 now uses an effective 3.0 g hammer, a quadratic stiffness of 24.96 MN/m² and a strike position 10.932% along the string. The fit uses both absolute fundamental level and relative upper partials across strike strengths. Contact ends at 1.94 ms at velocity 92 and the fundamental is 0.11 dB below the neighboring midpoint at 48 kHz. Across tested velocities 25–127 and rates of 44.1, 48 and 96 kHz, the fundamental stays within 0.44 dB of that midpoint. At MIDI velocity 1 it is 2.84 dB above. These are local continuity measurements. A native D5 recording would provide an independent timbre target. [Mechanical calibration](src/scale.rs) · [Regression test](src/lib.rs)

### B6 hammer and decay

The earlier B6 fit used PP and FF spectra from G6 and C♯7. Its medium-soft target was interpolated. At velocity 65 the original modeled hammer contacted the strings for 1.54 ms, compared with 1.42 ms at velocity 116. Projecting the contact force onto the two modal frequencies gave a second-to-first ratio of −11.4 dB at 65 and −28.1 dB at 116. The pickup and tone circuit raised the second partial by about 2.0 dB relative to the fundamental at both velocities. This locates the excessive velocity dependence in the collision.

The B6 contact fit uses the native MP and FF recordings. A linear spring in parallel with the quadratic spring stabilizes the excitation spectrum across strike strengths. The B6 fundamental envelope also has a later minimum than the original unison model predicted. The fitted string spacing is 0.35 cent, and the second mode's intrinsic loss is 40 dB/s higher than the fundamental's. These effective parameters describe the modeled note. The recording archive supplies one native strike per available B6 dynamic and supplies no native B6 PP.

| B6 strike | Recorded second partial | Previous model | Revised model |
| --- | ---: | ---: | ---: |
| MP, velocity 65 | −30.48 dB | −10.92 dB | −30.55 dB |
| F, velocity 92, excluded from fitting | −30.61 dB | −21.54 dB | −29.69 dB |
| FF, velocity 116 | −28.47 dB | −27.85 dB | −28.48 dB |

These levels are relative to the fundamental over 65–240 ms at 44.1 kHz. The revision also improves the MP error over 250–400 ms from 26.46 to 4.47 dB. The same late window on FF is now 4.65 dB too quiet, up from 0.88 dB of error. Seven surrounding keys from G6 through D7 were rendered at four velocities and three sample rates. All 84 stereo comparisons against the previous engine were sample-identical. [B6 comparison and listening-clip generator](research/check_b6.py) · [Voicing coefficients](src/scale.rs)

Against a double-precision 384 kHz reference, the live model's largest audible-partial level difference was 0.47 dB on a hard C8 at 44.1 kHz. This comparison covers F♯2, E♭4, E6, G7 and C8 at each of 44.1, 48 and 96 kHz. It measures the cost of the real-time approximation separately from the fit to an instrument.

The Yamaha board-level checks cover gain trim, bass, treble, all brilliance settings, the open and connected patch loop and the muted jack versus the independent line output. The Rust small-signal audio circuit differs from the independent ngspice network by at most 0.196 dB and 0.64° across 54 sweeps from 40 Hz to 10 kHz. Inputs on both sides of the pickup capacitor, three control combinations, all brilliance settings and three sample rates are included. These are checks against the transcribed schematic and the manual's adjustment limits. Measuring a particular CP-80 would independently constrain the provisional components and overload behavior.

## Playback verification

Headless CLAP and VST3 hosts loaded the installed macOS bundles and rendered timed MIDI notes at 44.1, 48 and 96 kHz. Checks covered pre-note silence, finite output and damper release. Both dry channels matched exactly. Enabling tremolo in the VST3 produced a left–right difference at every rate. The generated editor preview uses the plugin's drawing routine.

The following release-build timings are from an Apple M1 MacBook Air with all 88 keys struck on the same sample under sustain, in 64-frame blocks. At 44.1 kHz the longest block exceeded the deadline by 0.10 ms. At 96 kHz the longest block left 0.007 ms for the host. This is a stress case for an otherwise playable instrument.

| Host rate | CPU seconds per audio second | 99th percentile | Longest block | Block deadline |
| --- | ---: | ---: | ---: | ---: |
| 44.1 kHz | 0.203 | 0.414 ms | 1.550 ms | 1.451 ms |
| 48 kHz | 0.226 | 0.369 ms | 1.214 ms | 1.333 ms |
| 96 kHz | 0.427 | 0.357 ms | 0.660 ms | 0.667 ms |

## Build and reproduce

From the workspace root, install with `./cp80/scripts/install.sh`. The shared installer builds, verifies and signs both Electric Pianos macOS bundles. Build the offline tool **after** installation because the installer cleans the plugin's release artifacts.

```sh
cargo build --release -p cp80 --example cp80_probe
probe=target/release/examples/cp80_probe
proof=target/cp80-proof
mkdir -p "$proof"
"$probe" demo "$proof/electric-grand-80.wav"
"$probe" bench
```

The demo plays the same pedal phrase twice, first dry and then with stereo tremolo. The benchmark strikes all 88 notes at one instant, releases the keys under sustain and lifts the pedal after one second of sound. It reports a full second of 64-frame callback timings and checks both the sustained and released output. Timing depends on the host CPU and scheduler.

The research scripts use ngspice on PATH and Python with NumPy, SciPy and SoundFile. Point `samples` at the reference archive's `CP80/Samples` directory.

```sh
python3 cp80/research/circuit.py
python3 cp80/research/check_rust.py "$probe"
python3 cp80/research/compare.py --live "$probe" "$samples" "$proof"
python3 cp80/research/validate_keys.py "$probe" "$samples" "$proof"
python3 cp80/research/check_b6.py "$probe" "$samples" "$proof/b6"
```

Pass `--before PATH_TO_PREVIOUS_PROBE` to the B6 check to add the earlier model to the listening clip and compare neighboring outputs sample for sample.

For a dry render of the balanced line, jack or patch send, run `python3 cp80/research/render_output.py "$probe" line 63 0.75 0.4 "$proof/line.wav"`. Replace `line` with `jack` or `patch` to hear those observation points. The render uses the corresponding complex transfer at neutral tone settings.

The generated WAVs, credits, strike metrics and per-partial CSVs land in the proof directory. The plugin binary contains the model coefficients and oscillator curves. The recordings are only an input to the offline comparison. [Design and source register](../docs/cp80-model-plan.md)
