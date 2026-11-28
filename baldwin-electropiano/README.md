# Baldwin classroom Electropiano

Select **More pianos → Baldwin → Baldwin Electro-Piano**. Play a key or press the felt hammer in the cutaway to audition the selected pitch. CC64 lifts the dampers. CC67 operates the electronic soft pedal. **Pickups** moves the preloaded piezo elements along their supporting rail by up to 8 mm. **Extended Notes** opens MIDI 0–127 beyond the instrument’s A0–C8 compass.

## From felt hammer to electrical output

The 88-key classroom specification gives a vertical action with let-off, felt-covered birch hammers, copper-wound bass, single treble strings and pressure-sensitive transducers. The model uses one dispersive string per key. It integrates felt compression and the moving hammer through contact and retains the string motion on restrike. Bass winding tapers into the first plain wire. Each bearing applies a transverse reaction to an aluminium U-channel, whose lip compresses rubber shims and ceramic elements against the other arm. Three passive modes per native course return the moving rail’s reaction to the strings. A separately struck neighbour can therefore excite a silent course. [Baldwin specifications, printed p. 140](https://digital.library.unt.edu/ark:/67531/metadc662988/m2/1/high_res_d/1002773114-LeCroy.pdf#page=145) · [Martin and Stein’s rail patent](https://patents.google.com/patent/US3685384A/en) · [Hammer and scale](src/action.rs) · [Rail and string loads](src/rail.rs)

Chad Tipps opens his 88-key Baldwin and describes two original register outputs connected to his replacement jack. Baldwin’s patent allows multiple rail sections and gives roughly 20–25 elements for 88 keys. This implementation uses two sections, 44 flexible rail cells and 22 ceramic elements. Six elements serve the lowest 24 keys. The allocation, rail dimensions, winding and electronic loads are estimates. The electrical probe exports both loaded pickup leads and the mixed electrical output. The soft pedal attenuates the mix by 6 dB while leaving each sensor voltage unchanged. The patent and specification establish the pressure readout and electrical pedal. Original preamp gain, soft-pedal attenuation and cabinet radiation still need measurements. [Tipps’s wiring explanation](https://www.youtube.com/watch?v=z9xAuYLEOTs&t=42s) · [Electrical mix](src/lib.rs) · [Loaded piezo circuit shared with Yamaha](../pleasant-dsp/src/piezo.rs)

## What the recordings establish

Four played notes from two owners set one common hammer regulation and shim stiffness. With the same string scale, the median error in harmonics two through six fell from 12.96 to 3.11 dB on those keys and from 12.86 to 10.17 dB on four reserved keys. These figures use a −40 dBc floor. The third instrument, excluded from the fit, gave 16.42 dB median error. Its F3 became less accurate after calibration. On reserved E5, the measured second harmonic is −44.6 dBc and the model gives −16.9 dBc. Three other upper harmonics fall under the comparison floor in both recordings. The model’s measured rise is 7 ms on F♯3 and E5, against 21 and 28 ms in the videos. The cameras and amplifiers leave each player’s hammer velocity and the original input load unknown. [Three performances, measurement windows and listening-reel instructions](research/recording.md)

The tests check pressure transfer into a neighbouring course, electrical soft-pedal gain, pickup displacement and the pitched extremes of the extended keyboard at 44.1, 48 and 96 kHz. The probe writes three-channel float WAVs for all 88 native keys and four extended pitches, then times a single note and an eight-note chord. The 88 passive string loads stay active in each timing.

```sh
cargo check -p baldwin-electropiano
cargo test -p baldwin-electropiano --quiet
cargo run --release -p baldwin-electropiano --example probe -- target/baldwin-probe/stock
```

The installed VST3 comparison plays the same 11 notes at MIDI velocities 48 and 100 on Baldwin, Roadmaster and CP-70B. It measures harmonic ratios and the level change after release, and writes a level-matched listening reel. The reel plays G2, C4 and C6, each in Baldwin, Roadmaster, CP-70B order. The standard sequence sends MIDI All Sound Off between trials. Add `--isolated` to start each trial from host reset. The script needs NumPy, Mido and Pedalboard.

```sh
python3 openwurli-ui/research/upright_comparison.py \
  "$HOME/Library/Audio/Plug-Ins/VST3/Electric Pianos.vst3" target/electric-uprights
```
