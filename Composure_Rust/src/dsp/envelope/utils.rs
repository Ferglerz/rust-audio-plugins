//! Envelope smoothing utilities ported from `Envelope/00_envelope_utils.jsfx-inc`.

use super::super::constants::{
    CURVE_CALIBRATION_DB, CURVE_FAST_MULTIPLIER, CURVE_SLOW_MULTIPLIER, EPS,
};
use super::super::core_math::{ms_to_coeff, sign_jsfx};

#[inline]
pub fn clamp_coeff(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

#[inline]
pub fn blend_two_coefficients(coef1: f64, coef2: f64, blend_factor: f64) -> f64 {
    coef1 * (1.0 - blend_factor) + coef2 * blend_factor
}

pub fn normalized_blend(
    blend_fast: f64,
    blend_slow: f64,
    coef_fast: f64,
    coef_slow: f64,
) -> f64 {
    let s = blend_fast + blend_slow;
    if s > EPS {
        (blend_fast * coef_fast + blend_slow * coef_slow) / s
    } else {
        coef_fast
    }
}

pub fn apply_envelope_smoothing(coeff: f64, current: f64, target: f64) -> f64 {
    let valid_coeff = coeff.clamp(EPS, 1.0 - EPS);
    valid_coeff * current + (1.0 - valid_coeff) * target
}

pub fn apply_linear_envelope_smoothing(
    db_per_sample: f64,
    current: f64,
    target: f64,
) -> f64 {
    let err = target - current;
    if err.abs() <= db_per_sample {
        target
    } else {
        current + db_per_sample * sign_jsfx(err)
    }
}

#[inline]
pub fn curve_cached_coeffs(time_ms: f64, srate: f64) -> (f64, f64) {
    if time_ms > 0.0 {
        (
            ms_to_coeff(time_ms / CURVE_FAST_MULTIPLIER, srate),
            ms_to_coeff(time_ms * CURVE_SLOW_MULTIPLIER, srate),
        )
    } else {
        (1.0 - EPS, 1.0 - EPS)
    }
}

#[inline]
pub fn blend_fast_slow_coeffs(
    fast: f64,
    slow: f64,
    curve_shape_factor: f64,
    positive_curve: bool,
) -> f64 {
    let blended = if positive_curve {
        blend_two_coefficients(fast, slow, curve_shape_factor)
    } else {
        blend_two_coefficients(slow, fast, curve_shape_factor)
    };
    clamp_coeff(blended)
}

#[inline]
pub fn apply_curve_amount_blending(
    base_coeff: f64,
    curve_shaped_coeff: f64,
    curve_amount: f64,
) -> f64 {
    let curve_blend = clamp_coeff(curve_amount / 2.0);
    blend_two_coefficients(base_coeff, curve_shaped_coeff, curve_blend)
}

#[inline]
pub fn distance_blend_factor(current_gr_db: f64, target_gr_db: f64) -> f64 {
    let distance = (current_gr_db - target_gr_db).abs();
    (distance / CURVE_CALIBRATION_DB).min(1.0)
}

/// Distance-based curve shaping shared by attack and release coefficient paths.
#[inline]
pub fn calculate_distance_curve_shaped_coeff(
    base_coeff: f64,
    fast: f64,
    slow: f64,
    current_gr_db: f64,
    target_gr_db: f64,
    curve: f64,
) -> f64 {
    if curve.abs() < EPS {
        return base_coeff;
    }
    let blend_factor = distance_blend_factor(current_gr_db, target_gr_db);
    let curve_amount = curve.abs();
    let blended_coeff = blend_fast_slow_coeffs(fast, slow, blend_factor, curve > 0.0);
    apply_curve_amount_blending(base_coeff, blended_coeff, curve_amount)
}
