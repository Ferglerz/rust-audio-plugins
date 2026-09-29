//! Envelope orchestration ported from `Envelope/04_envelope_orchestration.jsfx-inc`.

use super::super::constants::EPS;
use super::super::dsp_utils::flush_denormal;
use super::attack::{
    calculate_curve_shaped_attack_coeff, update_attack_coefficients, AttackCoeffs,
};
use super::hold::HoldState;
use super::release::ReleaseCoeffs;
use super::utils::{apply_envelope_smoothing, apply_linear_envelope_smoothing};

fn smooth_envelope_pair(
    coef: f64,
    current: &mut f64,
    current_before: &mut f64,
    target: f64,
    target_before: f64,
) {
    *current = apply_envelope_smoothing(coef, *current, target);
    *current_before = apply_envelope_smoothing(coef, *current_before, target_before);
}

#[cfg(test)]
mod input_rate_tests {
    use super::*;

    #[test]
    fn input_rate_activity_only_reports_applied_release_modulation() {
        let mut engine = EnvelopeEngine::new();
        let params = EnvelopeParams {
            input_rate_amount: 1.0,
            ..EnvelopeParams::default()
        };
        engine.sync_coefficients(&params, 48000.0);
        engine.global_smoothed_gain_db = -6.0;
        engine.global_smoothed_gain_db_before_strength = -6.0;
        engine.prev_detector_db = -19.9;
        engine.process_envelope_following(0.0, -20.0, 1.0, 0.0);
        assert!(engine.input_rate_activity > 0.0);
        engine.process_envelope_following(-12.0, -20.1, 1.0, 0.0);
        assert_eq!(engine.input_rate_activity, 0.0);
        engine.reset();
        assert_eq!(engine.input_rate_activity, 0.0);
    }
    #[test]
    fn source_rings_report_speed_changes_only_during_release() {
        for (input, gr) in [(100.0, 0.0), (0.0, 100.0), (50.0, 50.0)] {
            let mut engine = EnvelopeEngine::new();
            let params = EnvelopeParams {
                input_dependence: input,
                gr_dependence: gr,
                ..EnvelopeParams::default()
            };
            engine.sync_coefficients(&params, 48000.0);
            engine.global_smoothed_gain_db = -3.0;
            engine.global_smoothed_gain_db_before_strength = -3.0;
            engine.process_envelope_following(0.0, -60.0, 1.0, 0.0);
            assert_eq!(engine.input_dependence_activity != 0.0, input != 0.0);
            assert_eq!(engine.gr_dependence_activity != 0.0, gr != 0.0);
            assert!(engine.global_smoothed_gain_db > -3.0);
            engine.process_envelope_following(-12.0, -60.0, 1.0, 0.0);
            assert_eq!(engine.input_dependence_activity, 0.0);
            assert_eq!(engine.gr_dependence_activity, 0.0);
        }
    }

    #[test]
    fn detector_rate_speed_influence_is_consistent_across_sample_rates() {
        let mut values = Vec::new();
        for srate in [44100.0, 48000.0, 96000.0, 192000.0] {
            let mut engine = EnvelopeEngine::new();
            engine.sync_coefficients(
                &EnvelopeParams {
                    input_rate_amount: 1.0,
                    ..EnvelopeParams::default()
                },
                srate,
            );
            engine.global_smoothed_gain_db = -6.0;
            engine.global_smoothed_gain_db_before_strength = -6.0;
            engine.prev_detector_db = -20.0 + 0.01 * 48000.0 / srate;
            engine.process_envelope_following(0.0, -20.0, 1.0, 0.0);
            values.push(engine.input_rate_activity);
        }
        for value in &values {
            // Finite per-sample coefficients introduce a small discretization error.
            assert!(
                ((*value - values[1]).exp2() - 1.0).abs() < 0.001,
                "speed octaves: {values:?}"
            );
        }
    }
}

fn smooth_linear_envelope_pair(
    db_per_sample: f64,
    current: &mut f64,
    current_before: &mut f64,
    target: f64,
    target_before: f64,
) {
    *current = apply_linear_envelope_smoothing(db_per_sample, *current, target);
    *current_before =
        apply_linear_envelope_smoothing(db_per_sample, *current_before, target_before);
}
use super::EnvelopeParams;

const MIN_SPEED_RATIO: f64 = 0.03125;
const MAX_SPEED_RATIO: f64 = 8.0;

#[derive(Debug, Clone, Copy)]
struct ReleaseRuntimeCache {
    use_linear_release: bool,
}

impl ReleaseRuntimeCache {
    fn from_params(p: &EnvelopeParams) -> Self {
        Self {
            use_linear_release: p.input_dependence + p.gr_dependence <= 50.0,
        }
    }
}

pub struct EnvelopeEngine {
    pub global_smoothed_gain_db: f64,
    pub global_smoothed_gain_db_before_strength: f64,
    pub prev_detector_db: f64,
    /// Signed live rate modulation: positive speeds release up, negative slows it.
    detector_rate_scale: f64,
    pub input_rate_activity: f64,
    pub input_dependence_activity: f64,
    pub gr_dependence_activity: f64,
    attack: AttackCoeffs,
    hold: HoldState,
    release: ReleaseCoeffs,
    params: EnvelopeParams,
    current_detector_level_db: f64,
    release_cache: ReleaseRuntimeCache,
    target_attack: AttackCoeffs,
    target_release: ReleaseCoeffs,
}

impl Default for EnvelopeEngine {
    fn default() -> Self {
        let default_attack = update_attack_coefficients(1000.0, 48000.0);
        let mut default_release = ReleaseCoeffs::default();
        default_release.update_release_coefficient(100.0, 48000.0);
        Self {
            global_smoothed_gain_db: 0.0,
            global_smoothed_gain_db_before_strength: 0.0,
            prev_detector_db: -20.0,
            detector_rate_scale: 1.0,
            input_rate_activity: 0.0,
            input_dependence_activity: 0.0,
            gr_dependence_activity: 0.0,
            attack: default_attack,
            hold: HoldState::default(),
            release: default_release,
            params: EnvelopeParams::default(),
            current_detector_level_db: -20.0,
            release_cache: ReleaseRuntimeCache::from_params(&EnvelopeParams::default()),
            target_attack: default_attack,
            target_release: default_release,
        }
    }
}

impl EnvelopeEngine {
    pub fn new() -> Self {
        Self::default()
    }

    fn update_target_coefficients(&mut self, p: &EnvelopeParams, srate: f64) {
        self.detector_rate_scale = srate / 48000.0;
        self.target_attack = update_attack_coefficients(p.attack, srate);
        let mut r = ReleaseCoeffs::default();
        r.update_release_coefficient(p.release_ms, srate);
        self.target_release = r;
    }

    pub fn sync_coefficients(&mut self, p: &EnvelopeParams, srate: f64) {
        self.params = *p;
        self.release_cache = ReleaseRuntimeCache::from_params(p);
        self.update_target_coefficients(p, srate);

        self.attack = self.target_attack;
        self.release = self.target_release;

        self.hold.update_hold_samples(p.hold_ms, srate);
    }

    pub fn set_target_coefficients(&mut self, p: &EnvelopeParams, srate: f64) {
        self.update_target_coefficients(p, srate);
    }

    pub fn update_runtime_params(&mut self, p: &EnvelopeParams, srate: f64, om: f64) {
        self.params = *p;
        self.release_cache = ReleaseRuntimeCache::from_params(p);
        self.hold.update_hold_samples(p.hold_ms, srate);
        self.attack = self.attack.lerp(self.target_attack, om);
        self.release = self.release.lerp(self.target_release, om);
    }

    pub fn determine_attack_or_release(&self, target_gr_abs: f64, current_gr_abs: f64) -> bool {
        let hysteresis_threshold = (current_gr_abs * 0.1).max(0.2);
        target_gr_abs > current_gr_abs + hysteresis_threshold
    }

    fn attack_coeff_for_sample(&self, target_gr_db: f64) -> f64 {
        if self.params.attack_curve.abs() >= EPS {
            calculate_curve_shaped_attack_coeff(
                &self.attack,
                self.global_smoothed_gain_db,
                target_gr_db,
                self.params.attack_curve,
            )
        } else {
            self.attack.attack_coeff
        }
    }

    pub fn process_single_stage_envelope(
        &mut self,
        target_gr_db: f64,
        target_gr_db_before_strength: f64,
    ) -> f64 {
        let target_gr_abs = target_gr_db.abs();
        let current_gr_abs = self.global_smoothed_gain_db.abs();
        let current_gr_abs_before_strength = self.global_smoothed_gain_db_before_strength.abs();
        let is_negative_gr = self.global_smoothed_gain_db < 0.0;
        let is_attack = self.determine_attack_or_release(target_gr_abs, current_gr_abs);
        self.input_rate_activity = 0.0;
        self.input_dependence_activity = 0.0;
        self.gr_dependence_activity = 0.0;

        if is_attack {
            let attack_coef = self.attack_coeff_for_sample(target_gr_db);
            smooth_envelope_pair(
                attack_coef,
                &mut self.global_smoothed_gain_db,
                &mut self.global_smoothed_gain_db_before_strength,
                target_gr_db,
                target_gr_db_before_strength,
            );
        } else {
            let (fixed, input, gr, base_rel_coef) = self.release.release_response(
                &self.params,
                self.global_smoothed_gain_db,
                target_gr_db,
                self.current_detector_level_db,
                current_gr_abs_before_strength,
                is_negative_gr,
            );
            if (self.global_smoothed_gain_db - target_gr_db).abs() > EPS {
                let speed = |coefficient: f64| {
                    let ratio = (1.0 - coefficient.clamp(EPS, 1.0 - EPS))
                        / (1.0 - self.release.release_coeff.clamp(EPS, 1.0 - EPS));
                    if self.release_cache.use_linear_release {
                        ratio.clamp(MIN_SPEED_RATIO, MAX_SPEED_RATIO)
                    } else {
                        ratio
                    }
                };
                self.input_dependence_activity = (speed(input) / speed(fixed)).log2();
                self.gr_dependence_activity = (speed(gr) / speed(fixed)).log2();
            }
            let rel_coef_use = self.release.apply_input_rate(
                &self.params,
                base_rel_coef,
                (self.prev_detector_db - self.current_detector_level_db) * self.detector_rate_scale,
            );
            if (self.global_smoothed_gain_db - target_gr_db).abs() > EPS {
                let speed = (1.0 - rel_coef_use.clamp(EPS, 1.0 - EPS))
                    / (1.0 - base_rel_coef.clamp(EPS, 1.0 - EPS));
                self.input_rate_activity = speed.log2();
            }

            if self.release_cache.use_linear_release {
                let c_ref = self.release.release_coeff.clamp(EPS, 1.0 - EPS);
                let c_use = rel_coef_use.clamp(EPS, 1.0 - EPS);
                let mut linear_speed_ratio = (1.0 - c_use) / (1.0 - c_ref);
                linear_speed_ratio = linear_speed_ratio.clamp(MIN_SPEED_RATIO, MAX_SPEED_RATIO);
                if (self.global_smoothed_gain_db - target_gr_db).abs() > EPS {
                    let base_speed_ratio = ((1.0 - base_rel_coef.clamp(EPS, 1.0 - EPS))
                        / (1.0 - c_ref))
                        .clamp(MIN_SPEED_RATIO, MAX_SPEED_RATIO);
                    self.input_rate_activity = (linear_speed_ratio / base_speed_ratio).log2();
                }
                let linear_db_per_sample =
                    self.release.linear_release_db_per_sample * linear_speed_ratio;
                smooth_linear_envelope_pair(
                    linear_db_per_sample,
                    &mut self.global_smoothed_gain_db,
                    &mut self.global_smoothed_gain_db_before_strength,
                    target_gr_db,
                    target_gr_db_before_strength,
                );
            } else {
                smooth_envelope_pair(
                    rel_coef_use,
                    &mut self.global_smoothed_gain_db,
                    &mut self.global_smoothed_gain_db_before_strength,
                    target_gr_db,
                    target_gr_db_before_strength,
                );
            }
        }

        self.global_smoothed_gain_db
    }

    pub fn process_envelope_following(
        &mut self,
        mut target_gr_db: f64,
        detector_level_db: f64,
        strength_multiplier: f64,
        hold_ms: f64,
    ) -> f64 {
        target_gr_db = self.hold.process_hold(target_gr_db, hold_ms);
        let target_gr_db_before_strength = if strength_multiplier.abs() > 0.0001 {
            target_gr_db / strength_multiplier
        } else {
            target_gr_db
        };
        self.current_detector_level_db = detector_level_db;
        self.process_single_stage_envelope(target_gr_db, target_gr_db_before_strength)
    }

    pub fn reset(&mut self) {
        self.global_smoothed_gain_db = 0.0;
        self.global_smoothed_gain_db_before_strength = 0.0;
        self.hold.counter_samples = 0;
        self.prev_detector_db = -20.0;
        self.input_rate_activity = 0.0;
        self.input_dependence_activity = 0.0;
        self.gr_dependence_activity = 0.0;
    }

    pub fn flush_denormals(&mut self) {
        flush_denormal(&mut self.global_smoothed_gain_db);
        flush_denormal(&mut self.global_smoothed_gain_db_before_strength);
    }
}
