//! Build-time description of the engine and its measured comparison.
//!
//! Update these claims only from the retained CPU A/B report. The editor does
//! no timing on the audio thread, and these figures are not a live CPU meter.

pub const TITLE: &str = "ENGINE PERFORMANCE";
pub const MODEL: &str = "Native equations active / 64 voices / double precision";
pub const CHANGES: [&str; 3] = [
    "Reed modes run in pairs; stable coefficients are reused.",
    "Original circuit equations remain available for comparison.",
    "Circuit lookup tables remain experimental and disabled.",
];

// Apple M1 native full comparison: docs/cpu-optimization-results.md.
pub const CPU_RESULT: &str = "64 voices: 15.7% less CPU vs original / 3.8% vs prior SIMD";
pub const AUDIO_RESULT: &str = "1,542 cases / 85.9M samples: bit-identical engine output.";
pub const CONDITIONS: &str = "M1 / 48 kHz / 256 samples. Low polyphony: roughly unchanged.";
