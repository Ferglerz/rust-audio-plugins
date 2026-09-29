//! Per-sample parameter crossfade (kills block-rate zipper on automation).

use super::envelope::EnvelopeParams;

/// Default coeff morph time — matches RMS coeff smoothing order of magnitude.
pub const PARAM_SMOOTH_MS: f64 = 10.0;

const PARAM_CONV_EPS: f64 = 1e-12;

macro_rules! smooth_envelope_field {
    ($current:expr, $target:expr, $one_minus:expr, $field:ident) => {
        $current.$field = smooth_step($current.$field, $target.$field, $one_minus);
    };
}

macro_rules! envelope_smooth_scalar_fields {
    ($current:expr, $target:expr, $one_minus:expr) => {
        crate::envelope_scalar_fields!(smooth_envelope_field; $current, $target, $one_minus);
    };
}

macro_rules! envelope_converged_scalar_fields {
    ($current:expr, $target:expr) => {{
        let mut converged = true;
        macro_rules! __check_field {
            ($c:expr, $t:expr, $field:ident) => {
                converged = converged && scalar_converged($c.$field, $t.$field);
            };
        }
        crate::envelope_scalar_fields!(__check_field; $current, $target);
        converged
    }};
}

#[inline]
pub fn smooth_step(current: f64, target: f64, one_minus: f64) -> f64 {
    if (target - current).abs() < 1e-15 {
        target
    } else {
        current + (target - current) * one_minus
    }
}

#[inline]
pub fn smooth_one_minus(srate: f64, time_ms: f64) -> f64 {
    let time_s = (time_ms * 0.001).max(1e-6);
    1.0 - (-1.0 / (time_s * srate)).exp()
}

#[inline]
pub fn scalar_converged(current: f64, target: f64) -> bool {
    (target - current).abs() < PARAM_CONV_EPS
}

/// True when continuous envelope fields match targets (discrete modes may differ harmlessly).
pub fn envelope_params_converged(current: &EnvelopeParams, target: &EnvelopeParams) -> bool {
    envelope_converged_scalar_fields!(current, target)
}

/// Lerp continuous envelope fields; discrete mode/flags snap instantly from `target`.
pub fn smooth_envelope_params(
    current: &mut EnvelopeParams,
    target: &EnvelopeParams,
    one_minus: f64,
) {
    envelope_smooth_scalar_fields!(current, target, one_minus);
    current.prog_release_inverse = target.prog_release_inverse;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_step_reaches_target() {
        let mut v = 0.0;
        let om = smooth_one_minus(48000.0, 10.0);
        for _ in 0..10000 {
            v = smooth_step(v, 1.0, om);
        }
        assert!((v - 1.0).abs() < 1e-6);
    }
}
