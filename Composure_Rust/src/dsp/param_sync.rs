//! Host-independent block-rate settings for dirty-flag sync.

use super::envelope::EnvelopeParams;
use super::harmonics::HarmonicParams;

/// Cached settings read once per block; skip DSP retarget when unchanged.
///
/// `PartialEq` uses exact float equality on captured host values — intentional:
/// any host/automation change must flip the dirty flag.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockParamState {
    pub hp_freq: f32,
    pub lp_freq: f32,
    pub detector_eq: [pleasant_eq::BandSettings; crate::detector_eq::EQ_BANDS],
    pub envelope: EnvelopeParams,
    pub harmonic_params: HarmonicParams,
    pub harmonics_on: bool,
    pub detection_feedback: bool,
    pub rms_normalization: bool,
    pub rms_size_ms: f64,
    pub mid_side_mode: bool,
    pub sc_adjust_preview: bool,
    pub use_sidechain: bool,
    pub target_makeup_gain_linear: f64,
    pub target_strength_multiplier: f64,
    pub target_input_offset_db: f64,
    pub target_lookahead_ms: f64,
}
