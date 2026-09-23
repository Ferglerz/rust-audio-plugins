//! Constants ported from `01_Utils/00_constants.jsfx-inc` and
//! `03_Compression/01_compression_constants.jsfx-inc`.

pub const EPS: f64 = 1e-30;

pub const MIN_DETECTOR_LEVEL: f64 = 1e-6;
pub const MAX_DETECTOR_LINEAR: f64 = 100.0;
pub const MAX_DETECTOR_DB: f64 = 60.0;
pub const MIN_DETECTOR_DB_FLOOR: f64 = -240.0;
pub const MAX_CURVE_INPUT_DB: f64 = 100.0;

pub const LIMITER_SCALE: f64 = 0.95;

pub const MAX_GR_DB: f64 = 50.0;

#[inline]
pub fn clamp_gr_db(v: f64) -> f64 {
    v.clamp(-MAX_GR_DB, MAX_GR_DB)
}

pub const GRAPH_RANGE_DB: f64 = 20.0;
pub const GRAPH_MIN_DB: f64 = -20.0;
pub const GRAPH_MAX_DB: f64 = 0.0;
/// Extra dB shown around the operational graph square in the UI.
pub const GRAPH_DISPLAY_PAD_DB: f64 = 1.5;
/// Grid / axis label spacing in the graph UI.
pub const GRAPH_GRID_STEP_DB: f64 = 5.0;

pub const COMP_LUT_MIN_DB: f64 = -120.0;
pub const COMP_LUT_MAX_DB: f64 = 20.0;
pub const COMP_LUT_GRANULARITY: f64 = 0.25;
pub const COMP_LUT_SIZE: usize = 560;
pub const COMP_LUT_MAX_REDUCTION_DB: f64 = 80.0;
/// LUT onset: first entry where |out − in| exceeds this (JSFX: 0.08).
pub const COMP_LUT_THRESHOLD_ONSET_DB: f64 = 0.08;

pub const BEZIER_STEPS: usize = 20;
pub const MAX_CURVE_SEGMENTS: usize = 500;
pub const MAX_POINTS: usize = 12;
pub const MIN_POINTS: usize = 4;
pub const MOUSE_CLICK_RADIUS: f64 = 18.0;

pub const CURVE_CALIBRATION_DB: f64 = 10.0;
pub const CURVE_FAST_MULTIPLIER: f64 = 1.5;
pub const CURVE_SLOW_MULTIPLIER: f64 = 1.5;

pub const LINEAR_RELEASE_FIXED_DB: f64 = 10.0;

pub const INPUT_DEPENDENT_DRAMA: f64 = 3.0;

pub const DENORMAL_THRESHOLD: f64 = 1e-15;

/// Minimum lookahead buffer size — 2000ms at 48kHz (96000 → 131072).
pub const MIN_LOOKAHEAD_BUFFER_SIZE: usize = 131_072;
