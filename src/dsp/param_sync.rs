//! Block-rate parameter snapshot for dirty-flag sync.

use crate::params::{
    ComposureParams, DetectionMode,
};

use super::envelope::EnvelopeParams;
use super::harmonics::HarmonicParams;

/// Cached plugin values read once per block; skip DSP retarget when unchanged.
///
/// `PartialEq` uses exact float equality on values read from atomic params — intentional:
/// any host/automation change must flip the dirty flag.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockParamState {
    pub graph_range_db: f64,
    pub hp_freq: f32,
    pub lp_freq: f32,
    pub envelope: EnvelopeParams,
    pub harmonic_params: HarmonicParams,
    pub detection_feedback: bool,
    pub rms_normalization: bool,
    pub rms_size_ms: f64,
    pub brickwall_limiter: bool,
    pub mid_side_mode: bool,
    pub sc_adjust_preview: bool,
    pub use_sidechain: bool,
    pub target_makeup_gain_linear: f64,
    pub target_strength_multiplier: f64,
    pub target_input_offset_db: f64,
    pub target_lookahead_ms: f64,
}

impl BlockParamState {
    pub fn capture(plugin_params: &ComposureParams) -> Self {
        let graph_range_db = plugin_params.graph_range_mode.value().range_db();

        Self {
            graph_range_db,
            hp_freq: plugin_params.hp_freq.value(),
            lp_freq: plugin_params.lp_freq.value(),
            envelope: EnvelopeParams::from_plugin(plugin_params),
            harmonic_params: HarmonicParams::from_plugin(plugin_params),
            detection_feedback: plugin_params.detection_mode.value() == DetectionMode::Feedback,
            rms_normalization: plugin_params.rms_normalization.value(),
            rms_size_ms: plugin_params.rms_size_ms.value() as f64,
            brickwall_limiter: plugin_params.brickwall_limiter.value(),
            mid_side_mode: plugin_params.mid_side_mode.value(),
            sc_adjust_preview: plugin_params.sc_adjust_preview.value(),
            use_sidechain: plugin_params.use_sidechain.value(),
            target_makeup_gain_linear: super::core_math::db_to_linear(
                plugin_params.makeup_gain_db.value() as f64,
            ),
            target_strength_multiplier: plugin_params.strength.value() as f64 / 100.0,
            target_input_offset_db: plugin_params.input_offset_db.value() as f64,
            target_lookahead_ms: plugin_params.lookahead_ms.value() as f64,
        }
    }
}
