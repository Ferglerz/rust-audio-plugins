//! Convert host parameter values into plain processing settings once per block.

use crate::dsp::core_math::db_to_linear;
use crate::dsp::envelope::{EnvelopeParams, ReleaseMode};
use crate::dsp::harmonics::HarmonicParams;
use crate::dsp::param_sync::BlockParamState;
use crate::params::{ComposureParams, DetectionMode, HarmonicType};

pub(crate) fn capture_block_state(params: &ComposureParams) -> BlockParamState {
    BlockParamState {
        hp_freq: params.hp_freq.value(),
        lp_freq: params.lp_freq.value(),
        detector_eq: std::array::from_fn(|i| params.detector_eq[i].settings()),
        envelope: capture_envelope(params),
        harmonic_params: capture_harmonics(params),
        harmonics_on: params.harmonics_on.value(),
        detection_feedback: params.detection_mode.value() == DetectionMode::Feedback,
        rms_normalization: params.rms_normalization.value(),
        rms_size_ms: params.rms_size_ms.value() as f64,
        mid_side_mode: params.mid_side_mode.value(),
        sc_adjust_preview: params.sc_adjust_preview.value(),
        use_sidechain: params.use_sidechain.value(),
        target_makeup_gain_linear: db_to_linear(params.makeup_gain_db.value() as f64),
        target_strength_multiplier: params.strength.value() as f64 / 100.0,
        target_input_offset_db: params.input_offset_db.value() as f64,
        target_lookahead_ms: params.lookahead_ms.value() as f64,
    }
}

fn capture_envelope(params: &ComposureParams) -> EnvelopeParams {
    let prog_release_mode = match (params.program_input_enabled(), params.program_gr_enabled()) {
        (true, true) => ReleaseMode::InputAndGr,
        (true, false) => ReleaseMode::InputDependent,
        (false, true) => ReleaseMode::GrDependent,
        (false, false) => ReleaseMode::Disabled,
    };
    EnvelopeParams {
        attack: params.attack.value() as f64,
        attack_curve: params.attack_curve.value() as f64,
        release_ms: params.release.value() as f64,
        release_curve: params.release_curve.value() as f64,
        hold_ms: params.hold_ms.value() as f64,
        strength: params.strength.value() as f64,
        prog_release_mode,
        prog_release_inverse: params.prog_release_inverse.value(),
        prog_release_blend: params.prog_release_blend.value() as f64,
        input_rate_amount: params.input_rate_amount.value() as f64,
        input_level_threshold_db: params.input_level_threshold_db.value() as f64,
        input_level_threshold_2_db: params.input_level_threshold_2_db.value() as f64,
        gr_blend_threshold_reduction_db: params.gr_blend_threshold_reduction_db.value() as f64,
        gr_blend_threshold_reduction_knee_db: params.gr_blend_threshold_reduction_knee_db.value()
            as f64,
        gr_blend_threshold_addition_db: params.gr_blend_threshold_addition_db.value() as f64,
        gr_blend_threshold_addition_knee_db: params.gr_blend_threshold_addition_knee_db.value()
            as f64,
        rate_change_sensitivity_db: params.rate_change_sensitivity_db.value() as f64,
        rate_change_threshold_modifier: params.rate_change_threshold_modifier.value() as f64,
    }
}

fn capture_harmonics(params: &ComposureParams) -> HarmonicParams {
    HarmonicParams {
        harmonic_type: match params.harmonic_type.value() {
            HarmonicType::Tube => 0,
            HarmonicType::Transformer => 1,
        },
        drive: params.harmonic_drive.value() as f64,
        mix: params.harmonic_mix.value() as f64 / 100.0,
        even_boost: params.harmonic_even_boost.value() as f64,
        odd_boost: params.harmonic_odd_boost.value() as f64,
        harmonic_amount: params.harmonic_amount.value() as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nih_plug::prelude::Param;

    #[test]
    fn default_host_values_capture_plain_settings() {
        let params = ComposureParams::default();
        let captured = capture_block_state(&params);
        assert_eq!(
            captured.envelope.prog_release_mode,
            ReleaseMode::InputDependent
        );
        assert_eq!(captured.harmonic_params.harmonic_type, 0);
        assert!(captured.harmonics_on);
        assert!(captured.target_makeup_gain_linear.is_finite());
        assert_eq!(params.rms_normalization.name(), "Adaptive");
    }

    #[test]
    fn program_enables_are_independent_booleans() {
        use nih_plug::prelude::BoolParam;
        for (input, gr, expected) in [
            (true, true, ReleaseMode::InputAndGr),
            (false, false, ReleaseMode::Disabled),
            (true, false, ReleaseMode::InputDependent),
            (false, true, ReleaseMode::GrDependent),
        ] {
            let mut params = ComposureParams::default();
            params.program_input_enable = BoolParam::new("Program Input", input);
            params.program_gr_enable = BoolParam::new("Program GR", gr);
            assert_eq!(capture_envelope(&params).prog_release_mode, expected);
        }
    }
}
