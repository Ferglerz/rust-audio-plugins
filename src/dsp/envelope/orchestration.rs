//! Envelope orchestration ported from `Envelope/04_envelope_orchestration.jsfx-inc`.

use super::super::constants::EPS;
use super::super::dsp_utils::flush_denormal;
use super::attack::{calculate_curve_shaped_attack_coeff, update_attack_coefficients, AttackCoeffs};
use super::hold::HoldState;
use super::release::ReleaseCoeffs;
use super::utils::{
    apply_envelope_smoothing, apply_linear_envelope_smoothing,
};

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

fn smooth_linear_envelope_pair(
    db_per_sample: f64,
    current: &mut f64,
    current_before: &mut f64,
    target: f64,
    target_before: f64,
) {
    *current = apply_linear_envelope_smoothing(db_per_sample, *current, target);
    *current_before = apply_linear_envelope_smoothing(db_per_sample, *current_before, target_before);
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
            use_linear_release: p.prog_release_blend * 0.01 <= 0.5,
        }
    }
}

pub struct EnvelopeEngine {
    pub global_smoothed_gain_db: f64,
    pub global_smoothed_gain_db_before_strength: f64,
    pub prev_detector_db: f64,
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
        let current_gr_abs_before_strength =
            self.global_smoothed_gain_db_before_strength.abs();
        let is_negative_gr = self.global_smoothed_gain_db < 0.0;
        let is_attack = self.determine_attack_or_release(target_gr_abs, current_gr_abs);

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
            let rel_coef_use = self.release.calculate_release_coefficient(
                &self.params,
                self.global_smoothed_gain_db,
                target_gr_abs,
                target_gr_db,
                self.current_detector_level_db,
                current_gr_abs,
                current_gr_abs_before_strength,
                is_negative_gr,
                self.prev_detector_db,
            );

            if self.release_cache.use_linear_release {
                let c_ref = self.release.release_coeff.clamp(EPS, 1.0 - EPS);
                let c_use = rel_coef_use.clamp(EPS, 1.0 - EPS);
                let mut linear_speed_ratio = (1.0 - c_use) / (1.0 - c_ref);
                linear_speed_ratio = linear_speed_ratio.clamp(MIN_SPEED_RATIO, MAX_SPEED_RATIO);
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
    }

    pub fn flush_denormals(&mut self) {
        flush_denormal(&mut self.global_smoothed_gain_db);
        flush_denormal(&mut self.global_smoothed_gain_db_before_strength);
    }
}
