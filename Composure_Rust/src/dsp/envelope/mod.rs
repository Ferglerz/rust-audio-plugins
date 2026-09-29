//! Envelope follower ported from `03_Compression/Envelope/`.

mod attack;
pub mod defaults;
mod hold;
mod orchestration;
mod release;
pub mod rms_params;
mod scalar_fields;
mod utils;

pub use orchestration::EnvelopeEngine;

// Scalar fields: keep in sync with `envelope_scalar_fields!` in scalar_fields.rs.
/// Runtime envelope parameters (subset of plugin params).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvelopeParams {
    pub attack: f64,
    pub attack_curve: f64,
    pub release_ms: f64,
    pub release_curve: f64,
    pub hold_ms: f64,
    pub strength: f64,
    pub prog_release_inverse: bool,
    pub input_dependence: f64,
    pub gr_dependence: f64,
    pub input_rate_amount: f64,
    pub input_level_threshold_db: f64,
    pub input_level_threshold_2_db: f64,
    pub gr_blend_threshold_reduction_db: f64,
    pub gr_blend_threshold_reduction_knee_db: f64,
    pub gr_blend_threshold_addition_db: f64,
    pub gr_blend_threshold_addition_knee_db: f64,
    pub rate_change_sensitivity_db: f64,
    pub rate_change_threshold_modifier: f64,
}

impl Default for EnvelopeParams {
    fn default() -> Self {
        use defaults::{
            ATTACK, ATTACK_CURVE, GR_BLEND_THRESHOLD_DB, GR_BLEND_THRESHOLD_KNEE_DB, HOLD_MS,
            INPUT_LEVEL_THRESHOLD_2_DB, INPUT_LEVEL_THRESHOLD_DB, RATE_CHANGE_SENSITIVITY_DB,
            RATE_CHANGE_THRESHOLD_MODIFIER, RELEASE_CURVE, RELEASE_MS, STRENGTH,
        };
        Self {
            attack: ATTACK,
            attack_curve: ATTACK_CURVE,
            release_ms: RELEASE_MS,
            release_curve: RELEASE_CURVE,
            hold_ms: HOLD_MS,
            strength: STRENGTH,
            prog_release_inverse: false,
            input_dependence: 0.0,
            gr_dependence: 0.0,
            input_rate_amount: 0.0,
            input_level_threshold_db: INPUT_LEVEL_THRESHOLD_DB,
            input_level_threshold_2_db: INPUT_LEVEL_THRESHOLD_2_DB,
            gr_blend_threshold_reduction_db: GR_BLEND_THRESHOLD_DB,
            gr_blend_threshold_reduction_knee_db: GR_BLEND_THRESHOLD_KNEE_DB,
            gr_blend_threshold_addition_db: GR_BLEND_THRESHOLD_DB,
            gr_blend_threshold_addition_knee_db: GR_BLEND_THRESHOLD_KNEE_DB,
            rate_change_sensitivity_db: RATE_CHANGE_SENSITIVITY_DB,
            rate_change_threshold_modifier: RATE_CHANGE_THRESHOLD_MODIFIER,
        }
    }
}

#[cfg(test)]
mod sync_tests {
    use super::*;

    /// Compile-time guard: every smoothed scalar must exist on `EnvelopeParams`.
    #[test]
    fn smoothed_fields_exist_on_struct() {
        let mut a = EnvelopeParams::default();
        let b = a;
        let om = 0.5;
        macro_rules! touch_field {
            ($field:ident) => {
                a.$field = crate::dsp::param_smooth::smooth_step(a.$field, b.$field, om);
            };
        }
        crate::envelope_scalar_fields!(touch_field;);
    }

    /// Smoothed scalar count — keep in sync with `envelope_scalar_fields!`.
    #[test]
    fn envelope_scalar_field_count() {
        const EXPECTED_SMOOTHED_SCALARS: usize = 17;
        let mut touched = 0usize;
        macro_rules! count_field {
            ($field:ident) => {
                touched += 1;
                let _ = std::mem::size_of_val(&EnvelopeParams::default().$field);
            };
        }
        crate::envelope_scalar_fields!(count_field;);
        assert_eq!(touched, EXPECTED_SMOOTHED_SCALARS);
    }
}
