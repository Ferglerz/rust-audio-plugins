//! Mode-specific descriptions. Figures are retained first-pass measurements,
//! not a live CPU meter or a qualification of the runtime switching build.

use crate::params::CpuMode;

pub struct EngineInfo {
    pub title: &'static str,
    pub model: &'static str,
    pub changes: [&'static str; 3],
    pub cpu_result: &'static str,
    pub audio_result: &'static str,
    pub conditions: &'static str,
}

pub fn for_mode(mode: CpuMode) -> EngineInfo {
    match mode {
        CpuMode::Fast => EngineInfo {
            title: "FAST ENGINE / PERFORMANCE",
            model: "Legacy preamp + behavioral amp / native equations / 64 voices",
            changes: [
                "Reed modes run in pairs; stable coefficients are reused.",
                "Original circuit equations retained; lookup tables disabled.",
                "Extra Sag is optional post-engine compression.",
            ],
            // Apple M1 report: docs/cpu-optimization-results.md. This predates
            // the runtime model selector and the second optimization pass.
            cpu_result: "First pass: 64 voices / 15.7% less CPU vs original",
            audio_result: "First pass: 1,542 cases / 85.9M samples bit-identical",
            conditions: "M1 / 48 kHz / 256 samples. 3.8% vs prior SIMD; low polyphony similar.",
        },
        CpuMode::Heavy => EngineInfo {
            title: "HEAVY ENGINE / PERFORMANCE",
            model: "Full circuit preamp + 7-BJT power amp / native equations / 64 voices",
            changes: [
                "Main and shadow preamp circuits; physical amp rail sag active.",
                "Extra Sag adds the same optional compression as Fast mode.",
                "Original circuit equations retained; lookup tables disabled.",
            ],
            cpu_result: "Full circuit model / no new CPU gain claimed",
            audio_result: "Original analytical solver retained",
            conditions: "Live mode changes: 100 ms circuit settling + 20 ms crossfade.",
        },
    }
}
