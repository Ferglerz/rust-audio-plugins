//! Mode-specific descriptions. Figures are retained measurements, not a live
//! CPU meter. Sources: docs/cpu-optimization-results.md (first pass) and
//! docs/runtime-modes.md (exact second pass against 0.1.1).

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
                "Reed modes run in pairs; stable coefficients and currents are reused.",
                "Original circuit equations retained; lookup tables disabled.",
                "Extra Sag is optional post-engine compression.",
            ],
            // First pass: Apple M1 against the original engine. Second pass:
            // x86-64 VM against 0.1.1; the two percentages do not add.
            cpu_result: "64 voices: 15.7% less CPU vs original; pass 2 up to 13% vs 0.1.1",
            audio_result: "Bit-identical: 1,542 cases vs original, 1,651 cases vs 0.1.1",
            conditions:
                "48 kHz / 256 samples. Pass 1 on M1; pass 2 on x86-64, most with vibrato off.",
        },
        CpuMode::Heavy => EngineInfo {
            title: "HEAVY ENGINE / PERFORMANCE",
            model: "Full circuit preamp + 7-BJT power amp / native equations / 64 voices",
            changes: [
                "Main and shadow preamp circuits; physical amp rail sag active.",
                "Shared matrix rebuilds; transistor solves run in lockstep.",
                "Original circuit equations retained; Extra Sag stays optional.",
            ],
            cpu_result: "35-56% less CPU vs 0.1.1 (1 to 64 voices)",
            audio_result: "1,651 cases / 95.0M samples bit-identical to 0.1.1",
            conditions: "x86-64 / 48 kHz / 256 samples. Mode change: 100 ms settle + 20 ms fade.",
        },
    }
}
