//! Shared default values for envelope runtime params (single source of truth).

pub const ATTACK: f64 = 1000.0;
pub const ATTACK_CURVE: f64 = 0.0;
pub const RELEASE_MS: f64 = 100.0;
pub const RELEASE_CURVE: f64 = 0.0;
pub const HOLD_MS: f64 = 0.0;
pub const STRENGTH: f64 = 100.0;
pub const PROG_RELEASE_BLEND: f64 = 0.0;
pub const INPUT_LEVEL_THRESHOLD_DB: f64 = -20.0;
pub const INPUT_LEVEL_THRESHOLD_2_DB: f64 = -40.0;
pub const GR_BLEND_THRESHOLD_DB: f64 = 6.0;
pub const GR_BLEND_THRESHOLD_KNEE_DB: f64 = 2.0;
pub const RATE_CHANGE_SENSITIVITY_DB: f64 = 3.0;
pub const RATE_CHANGE_THRESHOLD_MODIFIER: f64 = 1.0;
