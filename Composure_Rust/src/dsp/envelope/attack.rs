//! Attack coefficient calculation ported from `Envelope/01_envelope_attack.jsfx-inc`.

use super::super::constants::EPS;
use super::super::core_math::{db_per_sec_to_ms, ms_to_coeff};
use super::utils::{calculate_distance_curve_shaped_coeff, curve_cached_coeffs};

#[derive(Debug, Clone, Copy, Default)]
pub struct AttackCoeffs {
    pub attack_coeff: f64,
    pub curve_fast_cached: f64,
    pub curve_slow_cached: f64,
}

crate::impl_lerp_f64!(AttackCoeffs {
    attack_coeff,
    curve_fast_cached,
    curve_slow_cached,
});

pub fn update_attack_coefficients(attack: f64, srate: f64) -> AttackCoeffs {
    let attack_ms = db_per_sec_to_ms(attack);
    let attack_coeff = if attack_ms > 0.0 {
        ms_to_coeff(attack_ms, srate)
    } else {
        1.0 - EPS
    };
    let (curve_fast_cached, curve_slow_cached) = curve_cached_coeffs(attack_ms, srate);

    AttackCoeffs {
        attack_coeff,
        curve_fast_cached,
        curve_slow_cached,
    }
}

pub fn calculate_curve_shaped_attack_coeff(
    coeffs: &AttackCoeffs,
    current_gr_db: f64,
    target_gr_db: f64,
    attack_curve: f64,
) -> f64 {
    calculate_distance_curve_shaped_coeff(
        coeffs.attack_coeff,
        coeffs.curve_fast_cached,
        coeffs.curve_slow_cached,
        current_gr_db,
        target_gr_db,
        attack_curve,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faster_attack_gives_lower_coeff() {
        let slow = update_attack_coefficients(100.0, 48000.0);
        let fast = update_attack_coefficients(10000.0, 48000.0);
        assert!(fast.attack_coeff < slow.attack_coeff);
    }

    #[test]
    fn distance_curve_changes_with_error() {
        let c = update_attack_coefficients(1000.0, 48000.0);
        let near = calculate_curve_shaped_attack_coeff(&c, -5.0, -6.0, 1.0);
        let far = calculate_curve_shaped_attack_coeff(&c, -1.0, -10.0, 1.0);
        assert!(near != far);
    }
}
