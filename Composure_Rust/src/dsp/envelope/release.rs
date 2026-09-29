//! Release coefficient calculation ported from `Envelope/03_envelope_release.jsfx-inc`.

use super::super::constants::{EPS, INPUT_DEPENDENT_DRAMA, LINEAR_RELEASE_FIXED_DB};
use super::super::core_math::{db_per_sec_to_ms, ms_to_coeff};
use super::utils::{
    apply_curve_amount_blending, blend_fast_slow_coeffs, calculate_distance_curve_shaped_coeff,
    clamp_coeff, curve_cached_coeffs, normalized_blend,
};
use super::EnvelopeParams;

#[derive(Debug, Clone, Copy)]
pub struct ReleaseCoeffs {
    pub release_coeff: f64,
    pub release_time_ms: f64,
    pub linear_release_db_per_sample: f64,
    pub rel_fast_cached: f64,
    pub rel_med_cached: f64,
    pub rel_slow_cached: f64,
    pub curve_fast_coeff_cached: f64,
    pub curve_slow_coeff_cached: f64,
    pub rel_input_fast_cached: f64,
}

impl Default for ReleaseCoeffs {
    fn default() -> Self {
        Self {
            release_coeff: 0.0,
            release_time_ms: 0.0,
            linear_release_db_per_sample: 0.0,
            rel_fast_cached: 0.0,
            rel_med_cached: 0.0,
            rel_slow_cached: 0.0,
            curve_fast_coeff_cached: 0.0,
            curve_slow_coeff_cached: 0.0,
            rel_input_fast_cached: 0.0,
        }
    }
}

crate::impl_lerp_f64!(ReleaseCoeffs {
    release_coeff,
    release_time_ms,
    linear_release_db_per_sample,
    rel_fast_cached,
    rel_med_cached,
    rel_slow_cached,
    curve_fast_coeff_cached,
    curve_slow_coeff_cached,
    rel_input_fast_cached,
});

impl ReleaseCoeffs {
    pub fn update_release_coefficient(&mut self, release_ms: f64, srate: f64) {
        let release_db_per_sec = release_ms;
        self.release_time_ms = db_per_sec_to_ms(release_db_per_sec);
        self.calculate_linear_release_rate(srate);

        if self.release_time_ms > 0.0 {
            self.release_coeff = ms_to_coeff(self.release_time_ms, srate);
        } else {
            self.release_coeff = 1.0 - EPS;
        }

        let rel_mult = 0.5 + (self.release_time_ms / 2000.0) * 1.5;
        self.rel_fast_cached = (-1.0 / (0.05 * rel_mult * srate)).exp();
        self.rel_med_cached = (-1.0 / (0.3 * rel_mult * srate)).exp();
        self.rel_slow_cached = (-1.0 / (1.0 * rel_mult * srate)).exp();

        if self.release_time_ms > 0.0 {
            let (fast, slow) = curve_cached_coeffs(self.release_time_ms, srate);
            self.curve_fast_coeff_cached = fast;
            self.curve_slow_coeff_cached = slow;
            self.rel_input_fast_cached =
                ms_to_coeff(self.release_time_ms * (0.25 / INPUT_DEPENDENT_DRAMA), srate);
        } else {
            self.curve_fast_coeff_cached = 1.0 - EPS;
            self.curve_slow_coeff_cached = 1.0 - EPS;
            self.rel_input_fast_cached = 1.0 - EPS;
        }
    }

    fn calculate_linear_release_rate(&mut self, srate: f64) {
        let db_per_second = LINEAR_RELEASE_FIXED_DB / (self.release_time_ms * 0.001);
        self.linear_release_db_per_sample = db_per_second / srate;
    }

    fn gr_dependent_blend_weights(
        gr_amount: f64,
        threshold: f64,
        knee: f64,
        inverse: bool,
    ) -> (f64, f64) {
        let th = threshold.max(0.1);
        let (mut blend_fast, mut blend_slow) = if knee < 0.05 {
            (
                clamp_coeff(1.0 - gr_amount / th),
                clamp_coeff(gr_amount / th),
            )
        } else {
            let sw = if gr_amount <= th - knee {
                0.0
            } else if gr_amount >= th {
                1.0
            } else {
                (gr_amount - (th - knee)) / knee
            };
            let sw = clamp_coeff(sw);
            (clamp_coeff(1.0 - sw), clamp_coeff(sw))
        };
        if inverse {
            std::mem::swap(&mut blend_fast, &mut blend_slow);
        }
        (blend_fast, blend_slow)
    }

    pub fn release_gr_dependent(&self, p: &EnvelopeParams, gr_amount: f64) -> f64 {
        let (blend_fast, blend_slow) = Self::gr_dependent_blend_weights(
            gr_amount,
            p.gr_blend_threshold_reduction_db,
            p.gr_blend_threshold_reduction_knee_db,
            false,
        );
        normalized_blend(
            blend_fast,
            blend_slow,
            self.rel_fast_cached,
            self.rel_slow_cached,
        )
    }

    pub fn release_gr_dependent_dual(
        &self,
        p: &EnvelopeParams,
        gr_amount: f64,
        is_negative_gr: bool,
        inverse: bool,
    ) -> f64 {
        let (threshold, knee) = if is_negative_gr {
            (
                p.gr_blend_threshold_reduction_db,
                p.gr_blend_threshold_reduction_knee_db,
            )
        } else {
            (
                p.gr_blend_threshold_addition_db,
                p.gr_blend_threshold_addition_knee_db,
            )
        };
        let (blend_fast, blend_slow) =
            Self::gr_dependent_blend_weights(gr_amount, threshold, knee, inverse);
        normalized_blend(
            blend_fast,
            blend_slow,
            self.rel_fast_cached,
            self.rel_slow_cached,
        )
    }

    pub fn release_rate_of_change(
        &self,
        p: &EnvelopeParams,
        det_delta: f64,
        is_inverse: bool,
    ) -> f64 {
        let effective_sensitivity =
            p.rate_change_sensitivity_db / (p.rate_change_threshold_modifier * 10.0);
        let mut normalized_delta = det_delta / effective_sensitivity;
        normalized_delta = normalized_delta.clamp(0.0, 2.0);
        let blend1 = clamp_coeff(normalized_delta * 0.5);
        let blend2 = clamp_coeff(1.0 - blend1);
        let (blend_fast, blend_slow) = if is_inverse {
            (blend2, blend1)
        } else {
            (blend1, blend2)
        };
        normalized_blend(
            blend_fast,
            blend_slow,
            self.rel_fast_cached,
            self.rel_slow_cached,
        )
    }

    /// Apply rate modulation to the release already chosen by curve and independent program influences.
    /// A steady/rising detector is neutral; a falling detector changes release speed.
    pub fn apply_input_rate(&self, p: &EnvelopeParams, base: f64, det_delta: f64) -> f64 {
        let amount = p.input_rate_amount;
        if amount.abs() < EPS || det_delta <= 0.0 {
            return base;
        }
        let inverse = amount < 0.0;
        let rate = self
            .release_rate_of_change(p, det_delta, inverse)
            .clamp(EPS, 1.0 - EPS);
        let neutral = self
            .release_rate_of_change(p, 0.0, inverse)
            .clamp(EPS, 1.0 - EPS);
        // Scale in release-speed space so inverse influence remains stable at 5x.
        let speed = (rate.ln() / neutral.ln())
            .powf(amount.abs())
            .clamp(0.01, 100.0);
        base.clamp(EPS, 1.0 - EPS).powf(speed).clamp(EPS, 1.0 - EPS)
    }

    fn input_dependent_u(&self, p: &EnvelopeParams, input_level_db: f64) -> f64 {
        let knee_w = (p.input_level_threshold_db - p.input_level_threshold_2_db).abs();
        let knee_w = knee_w.max(0.5);
        let level_above = input_level_db - p.input_level_threshold_db;
        clamp_coeff(INPUT_DEPENDENT_DRAMA * level_above / knee_w)
    }

    fn input_dependent_blend(&self, u: f64, inverse: bool) -> f64 {
        let (blend_fast, blend_normal) = if inverse { (u, 1.0 - u) } else { (1.0 - u, u) };
        normalized_blend(
            blend_fast,
            blend_normal,
            self.rel_input_fast_cached,
            self.release_coeff,
        )
    }

    pub fn release_input_dependent(
        &self,
        p: &EnvelopeParams,
        input_level_db: f64,
        inverse: bool,
    ) -> f64 {
        let u = self.input_dependent_u(p, input_level_db);
        self.input_dependent_blend(u, inverse)
    }

    pub fn blend_curve_with_base(
        &self,
        curve_shape_factor: f64,
        _base_coeff: f64,
        release_curve_is_positive: bool,
    ) -> f64 {
        blend_fast_slow_coeffs(
            self.curve_fast_coeff_cached,
            self.curve_slow_coeff_cached,
            curve_shape_factor,
            release_curve_is_positive,
        )
    }

    pub fn calculate_curve_shaped_release_coeff(
        &self,
        current_gr_db: f64,
        target_gr_db: f64,
        release_curve: f64,
        base_coeff: f64,
    ) -> f64 {
        let base_to_use = if base_coeff > 0.0 {
            base_coeff
        } else {
            self.release_coeff
        };
        calculate_distance_curve_shaped_coeff(
            base_to_use,
            self.curve_fast_coeff_cached,
            self.curve_slow_coeff_cached,
            current_gr_db,
            target_gr_db,
            release_curve,
        )
    }

    pub fn calculate_gr_dependent_curve_release_coeff(
        &self,
        p: &EnvelopeParams,
        current_gr_abs: f64,
        release_curve: f64,
        gr_threshold: f64,
        is_negative_gr: bool,
    ) -> f64 {
        if release_curve.abs() < EPS {
            return self.release_gr_dependent(p, current_gr_abs);
        }
        let distance_from_threshold = (current_gr_abs - gr_threshold).abs();
        let normalized_pos = if gr_threshold > EPS {
            (distance_from_threshold / gr_threshold).min(2.0)
        } else {
            0.0
        };
        let curve_shape = if release_curve > 0.0 {
            clamp_coeff((normalized_pos * 0.5) * (normalized_pos * 0.5))
        } else {
            clamp_coeff(1.0 - (normalized_pos * 0.5) * (normalized_pos * 0.5))
        };
        let blended_coeff =
            self.blend_curve_with_base(curve_shape, self.release_coeff, release_curve > 0.0);
        let curve_amount = release_curve.abs();
        let base_gr_release =
            self.release_gr_dependent_dual(p, current_gr_abs, is_negative_gr, false);
        apply_curve_amount_blending(base_gr_release, blended_coeff, curve_amount)
    }

    /// Each source blends its own release law. Normalize overlapping influences
    /// above 100% so coefficients remain between the fixed and source endpoints.
    pub fn release_response(
        &self,
        p: &EnvelopeParams,
        current_db: f64,
        target_db: f64,
        detector_db: f64,
        gr_abs: f64,
        is_cut: bool,
    ) -> (f64, f64, f64, f64) {
        let curved = p.release_curve.abs() >= EPS;
        let fixed = if curved {
            self.calculate_curve_shaped_release_coeff(current_db, target_db, p.release_curve, -1.0)
        } else {
            self.release_coeff
        };
        let input_weight = (p.input_dependence * 0.01).clamp(0.0, 1.0);
        let gr_weight = (p.gr_dependence * 0.01).clamp(0.0, 1.0);
        let total = (input_weight + gr_weight).max(1.0);
        let input_weight = input_weight / total;
        let gr_weight = gr_weight / total;
        let input = if input_weight == 0.0 {
            fixed
        } else {
            let coefficient = self.release_input_dependent(p, detector_db, p.prog_release_inverse);
            if curved {
                self.calculate_curve_shaped_release_coeff(
                    current_db,
                    target_db,
                    p.release_curve,
                    coefficient,
                )
            } else {
                coefficient
            }
        };
        let gr = if gr_weight == 0.0 {
            fixed
        } else if curved {
            let threshold = if is_cut {
                p.gr_blend_threshold_reduction_db
            } else {
                p.gr_blend_threshold_addition_db
            };
            self.calculate_gr_dependent_curve_release_coeff(
                p,
                gr_abs,
                p.release_curve,
                threshold,
                is_cut,
            )
        } else {
            self.release_gr_dependent_dual(p, gr_abs, is_cut, p.prog_release_inverse)
        };
        let input_effect = (input - fixed) * input_weight;
        let gr_effect = (gr - fixed) * gr_weight;
        (
            fixed,
            fixed + input_effect,
            fixed + gr_effect,
            fixed + input_effect + gr_effect,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::defaults::{GR_BLEND_THRESHOLD_DB, GR_BLEND_THRESHOLD_KNEE_DB};
    use super::*;

    fn coeffs() -> ReleaseCoeffs {
        let mut c = ReleaseCoeffs::default();
        c.update_release_coefficient(100.0, 48000.0);
        c
    }

    #[test]
    fn input_rate_zero_and_steady_detector_are_neutral() {
        let c = coeffs();
        let mut p = EnvelopeParams {
            ..EnvelopeParams::default()
        };
        for base in [c.rel_fast_cached, c.release_coeff, c.rel_slow_cached] {
            assert_eq!(c.apply_input_rate(&p, base, 0.1), base);
            for amount in [-5.0, -1.0, 1.0, 5.0] {
                p.input_rate_amount = amount;
                assert_eq!(c.apply_input_rate(&p, base, 0.0), base);
                assert_eq!(c.apply_input_rate(&p, base, -0.1), base);
            }
            p.input_rate_amount = 0.0;
        }
    }

    #[test]
    fn input_rate_polarity_and_amount_scale_release_speed() {
        let c = coeffs();
        let mut p = EnvelopeParams {
            ..EnvelopeParams::default()
        };
        let base = c.release_coeff;
        p.input_rate_amount = 1.0;
        let faster = c.apply_input_rate(&p, base, 0.1);
        p.input_rate_amount = 5.0;
        let fastest = c.apply_input_rate(&p, base, 0.1);
        p.input_rate_amount = -1.0;
        let slower = c.apply_input_rate(&p, base, 0.1);
        p.input_rate_amount = -5.0;
        let slowest = c.apply_input_rate(&p, base, 0.1);
        assert!(fastest <= faster && faster < base);
        assert!(slowest >= slower && slower > base);
        for amount in [-5.0, 5.0] {
            p.input_rate_amount = amount;
            for delta in [0.00001, 0.1, 1000.0] {
                let result = c.apply_input_rate(&p, base, delta);
                assert!(result.is_finite() && result > 0.0 && result < 1.0);
            }
        }
    }

    #[test]
    fn input_rate_one_matches_existing_rate_speed_influence() {
        let c = coeffs();
        let p = EnvelopeParams {
            input_rate_amount: 1.0,
            ..EnvelopeParams::default()
        };
        let old_neutral = c.release_rate_of_change(&p, 0.0, false);
        let old_rate = c.release_rate_of_change(&p, 0.1, false);
        assert!((c.apply_input_rate(&p, old_neutral, 0.1) - old_rate).abs() < 1e-10);
    }

    #[test]
    fn independent_influences_blend_and_remain_bounded() {
        let c = coeffs();
        for curve in [-0.5, 0.0, 0.5] {
            let mut p = EnvelopeParams {
                release_curve: curve,
                ..EnvelopeParams::default()
            };
            let response =
                |p: &EnvelopeParams| c.release_response(p, -6.0, 0.0, -60.0, 6.0, true).3;
            let fixed = response(&p);
            p.input_dependence = 100.0;
            let input = response(&p);
            p.input_dependence = 50.0;
            assert!((response(&p) - (fixed + input) * 0.5).abs() < 1e-12);
            p.input_dependence = 0.0;
            p.gr_dependence = 100.0;
            let gr = response(&p);
            p.gr_dependence = 50.0;
            assert!((response(&p) - (fixed + gr) * 0.5).abs() < 1e-12);
            p.input_dependence = 100.0;
            p.gr_dependence = 100.0;
            assert!((response(&p) - (input + gr) * 0.5).abs() < 1e-12);
            for input_amount in [0.0, 25.0, 100.0] {
                for gr_amount in [0.0, 50.0, 100.0] {
                    p.input_dependence = input_amount;
                    p.gr_dependence = gr_amount;
                    let coefficient = response(&p);
                    assert!(coefficient.is_finite() && coefficient > 0.0 && coefficient < 1.0);
                    p.input_rate_amount = 1.0;
                    assert!(c.apply_input_rate(&p, coefficient, 0.1) < coefficient);
                }
            }
        }
    }

    #[test]
    fn knee_blend_at_threshold_is_full_slow() {
        let c = coeffs();
        let p = EnvelopeParams {
            gr_blend_threshold_reduction_db: GR_BLEND_THRESHOLD_DB,
            gr_blend_threshold_reduction_knee_db: GR_BLEND_THRESHOLD_KNEE_DB,
            ..EnvelopeParams::default()
        };
        let (fast, slow) = ReleaseCoeffs::gr_dependent_blend_weights(6.0, 6.0, 2.0, false);
        assert!((fast - 0.0).abs() < 1e-9);
        assert!((slow - 1.0).abs() < 1e-9);
        let coef = c.release_gr_dependent_dual(&p, 6.0, true, false);
        assert!((coef - c.rel_slow_cached).abs() < 1e-9);
    }

    #[test]
    fn knee_blend_below_knee_is_full_fast() {
        let (fast, slow) = ReleaseCoeffs::gr_dependent_blend_weights(3.0, 6.0, 2.0, false);
        assert!((fast - 1.0).abs() < 1e-9);
        assert!((slow - 0.0).abs() < 1e-9);
    }

    #[test]
    fn knee_blend_mid_knee_is_half() {
        let (fast, slow) = ReleaseCoeffs::gr_dependent_blend_weights(5.0, 6.0, 2.0, false);
        assert!((fast - 0.5).abs() < 1e-9);
        assert!((slow - 0.5).abs() < 1e-9);
    }

    #[test]
    fn zero_knee_falls_back_to_linear_ramp() {
        let (fast, slow) = ReleaseCoeffs::gr_dependent_blend_weights(3.0, 6.0, 0.0, false);
        assert!((fast - 0.5).abs() < 1e-9);
        assert!((slow - 0.5).abs() < 1e-9);
    }

    #[test]
    fn inverse_swaps_blend_weights() {
        let (fast, slow) = ReleaseCoeffs::gr_dependent_blend_weights(3.0, 6.0, 2.0, true);
        assert!((fast - 0.0).abs() < 1e-9);
        assert!((slow - 1.0).abs() < 1e-9);
    }
}
