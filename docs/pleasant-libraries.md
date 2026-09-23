# Pleasant library contracts

The reusable audio layer lives in this repository so plugin adapters and library APIs can evolve together. None of these crates depends on NIH-plug, Vizia, plugin parameter trees, DAW callbacks, or editor state.

## Dependency direction

- `pleasant-dsp` contains numerical units, graph-axis math, spectrum helpers, biquad design/state, envelope coefficients, and interpolation.
- `pleasant-curves` contains normalized editable Bézier geometry and analytic S-curves.
- `pleasant-eq` depends on `pleasant-dsp` and contains prepared static, dynamic, oversampled, and linear-phase EQ processors.
- `pleasant-dynamics` depends on `pleasant-dsp` and contains the vocal compressor, PSE, and wall saturation.
- `pleasant-ui` may consume numerical and curve data. DSP crates never consume `pleasant-ui`.
- Plugin crates own host parameter IDs, automation gestures, persistence keys, presets, routing, telemetry, and latency publication.

## Preparation and processing

Construct or replace processors outside the audio callback. `BandCoefficients::prepare`, `StaticEqProcessor::prepare`, `LinearPhaseEq::prepare`, FFT planning, curve serialization, and vector growth are preparation operations.

Sample-processing methods use prepared storage:

- `BandProcessor::process`
- `StaticEqProcessor::process_sample`
- `OversampledEq::process`
- `LinearPhaseEq::process_sample`
- `VocalComp::tick`, `tick_comp`, and `tick_pse`
- `tick_wall`

Configuration publication must retain the last complete prepared state until a newer generation is ready. Meter and display updates may be dropped when their nonblocking destination is busy. Configuration updates must not be silently dropped.

## Units and ranges

- Audio samples and gains use `f64` unless an interpolation API explicitly uses `f32`.
- Amplitude conversion uses `20 * log10(x)`. Legacy floors remain explicit: pleasant-ui uses `1e-9`; Damian dynamics uses `1e-12`.
- Frequency is hertz. Sample rate is hertz.
- EQ gain, threshold, range, reduction, and response use decibels.
- Attack and release fields on EQ and compressor settings use milliseconds.
- PSE attack/release tables use seconds.
- Normalized curve coordinates use `0.0..=1.0`; SCD converts MIDI velocity `1..=127` in its adapter.
- `spectrum_fall_db` preserves Damian's reference of 3 dB per 2048 processed samples. It is not wall-clock decay.

## EQ capabilities

`StaticEqProcessor` is zero latency. `OversampledEq` contributes 64 host samples of group delay. `LinearPhaseEq` reports the prepared kernel latency and uses 1024-sample partitions. Dynamic EQ keeps Damian's 32-sample coefficient update cadence and 15 ms parameter smoothing.

Damian keeps dual-EQ routing, sidechain EQ, band IDs, audition, Lift, transition ownership, host latency reporting, and serialized `Band` values. The adapter converts each `Band` into host-free `BandSettings`.

## Curve and graph boundaries

`pleasant-curves` preserves the serialized `nodes`, `x`, `y`, `in_handle`, and `out_handle` fields used by SCD. Drum articulation grouping, MIDI rasterization, persisted banks, and atomic lookup tables remain in SCD.

`pleasant-ui::graph` provides viewport transforms, axis mappings, nearest-point hit testing, and semantic graph actions. Plugin adapters remain responsible for automation begin/set/end events and domain-specific edits.

## Compatibility

`pleasant-ui::math` and `pleasant-ui::spectrum` temporarily re-export their original symbols from `pleasant-dsp`. New audio code should import `pleasant-dsp` directly. Existing plugin parameter IDs, persistence keys, plugin identifiers, and saved-state JSON shapes are compatibility boundaries.

## Verification

Run `scripts/verify-modularity.sh` from the repository root. SCD remains a nested workspace with its own lockfile, so it is checked separately. Live DAW validation remains required for resizing, pointer alignment, automation gesture completion, saved-project reopen, and DPI changes.
