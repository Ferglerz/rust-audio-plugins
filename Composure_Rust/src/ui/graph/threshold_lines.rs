//! Threshold lines — coordinate math for meter dragging.

use nih_plug_vizia::vizia::prelude::*;

use super::draw_helpers;

const MIN_THRESHOLD_SPACING: f64 = 6.0;
const GR_THRESHOLD_MIN: f32 = 1.0;
const GR_THRESHOLD_MAX: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThresholdLine {
    InputLevel,
    GrBlendReduction,
    GrBlendAddition,
}

/// GR meter axis: cut measured from top, boost from bottom.
#[derive(Clone, Copy)]
enum GrAxis {
    Cut,
    Boost,
}

impl GrAxis {
    const fn from_top(self) -> bool {
        matches!(self, Self::Cut)
    }

    fn db_from_y(self, y: f32, bounds: BoundingBox, range_db: f32) -> f32 {
        draw_helpers::gr_threshold_db_from_y(y, bounds, range_db, self.from_top())
    }
}

pub fn clamp_gr_threshold(v: f32) -> f32 {
    v.clamp(GR_THRESHOLD_MIN, GR_THRESHOLD_MAX)
}

/// Clamped meter threshold value from drag Y (GR cut/boost).
pub fn meter_threshold_value_from_y(
    line: ThresholdLine,
    y: f32,
    bounds: BoundingBox,
    range_db: f32,
) -> Option<f32> {
    match line {
        ThresholdLine::GrBlendReduction => Some(clamp_gr_threshold(
            GrAxis::Cut.db_from_y(y, bounds, range_db),
        )),
        ThresholdLine::GrBlendAddition => Some(clamp_gr_threshold(
            GrAxis::Boost.db_from_y(y, bounds, range_db),
        )),
        ThresholdLine::InputLevel => None,
    }
}

pub fn clamp_input_level(v: f64, min_db: f64, max_db: f64) -> f64 {
    v.clamp(min_db, max_db - MIN_THRESHOLD_SPACING)
}
