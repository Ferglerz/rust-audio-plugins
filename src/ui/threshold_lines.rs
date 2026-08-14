//! Threshold lines — draw + hit-test + coordinate math (production graph page).

use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::{Color as VgColor, Paint, Path};

use crate::params::{ComposureParams, ProgramReleaseMode};

use super::draw_helpers;

pub const GRAB_RADIUS: f32 = 8.0;
const MIN_THRESHOLD_SPACING: f64 = 6.0;
const GR_THRESHOLD_MIN: f32 = 1.0;
const GR_THRESHOLD_MAX: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThresholdLine {
    InputLevel,
    GrBlendReduction,
    GrBlendAddition,
    RateChangeModifier,
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

    const fn line(self) -> ThresholdLine {
        match self {
            Self::Cut => ThresholdLine::GrBlendReduction,
            Self::Boost => ThresholdLine::GrBlendAddition,
        }
    }

    fn y(self, bounds: BoundingBox, db: f32, range_db: f32) -> f32 {
        draw_helpers::gr_threshold_y(bounds, db, range_db, self.from_top())
    }

    fn db_from_y(self, y: f32, bounds: BoundingBox, range_db: f32) -> f32 {
        draw_helpers::gr_threshold_db_from_y(y, bounds, range_db, self.from_top())
    }
}

pub fn rate_modifier_y(bounds: BoundingBox, modifier: f32) -> f32 {
    let norm = ((modifier - 0.5) / 3.5).clamp(0.0, 1.0);
    bounds.y + bounds.h - norm * bounds.h
}

fn meter_y_to_rate_modifier(y: f32, bounds: BoundingBox) -> f32 {
    let norm = ((bounds.y + bounds.h - y) / bounds.h.max(1.0)).clamp(0.0, 1.0);
    norm * 3.5 + 0.5
}

pub fn clamp_gr_threshold(v: f32) -> f32 {
    v.clamp(GR_THRESHOLD_MIN, GR_THRESHOLD_MAX)
}

/// Clamped meter threshold value from drag Y (GR cut/boost + rate modifier).
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
        ThresholdLine::RateChangeModifier => Some(clamp_rate_modifier(
            meter_y_to_rate_modifier(y, bounds),
        )),
        ThresholdLine::InputLevel => None,
    }
}

pub fn clamp_rate_modifier(v: f32) -> f32 {
    v.clamp(0.5, 4.0)
}

pub fn clamp_input_level(v: f64, min_db: f64, max_db: f64) -> f64 {
    v.clamp(min_db, max_db - MIN_THRESHOLD_SPACING)
}

pub fn find_graph_threshold(
    params: &ComposureParams,
    mouse_y: f32,
    line_y: f32,
) -> Option<ThresholdLine> {
    if !params.prog_release_mode.value().is_input_dependent() {
        return None;
    }
    if (mouse_y - line_y).abs() < GRAB_RADIUS {
        Some(ThresholdLine::InputLevel)
    } else {
        None
    }
}

pub fn find_meter_threshold(
    params: &ComposureParams,
    mouse_x: f32,
    mouse_y: f32,
    bounds: BoundingBox,
    range_db: f32,
) -> Option<ThresholdLine> {
    if mouse_x < bounds.x || mouse_x > bounds.x + bounds.w {
        return None;
    }

    match params.prog_release_mode.value() {
        ProgramReleaseMode::GrDependent => {
            let span = range_db;
            let pairs = [
                (
                    GrAxis::Cut,
                    params.gr_blend_threshold_reduction_db.value(),
                ),
                (
                    GrAxis::Boost,
                    params.gr_blend_threshold_addition_db.value(),
                ),
            ];
            for (axis, db) in pairs {
                let line_y = axis.y(bounds, db, span);
                if (mouse_y - line_y).abs() < GRAB_RADIUS {
                    return Some(axis.line());
                }
            }
        }
        ProgramReleaseMode::RateOfChange => {
            let y = rate_modifier_y(bounds, params.rate_change_threshold_modifier.value());
            if (mouse_y - y).abs() < GRAB_RADIUS {
                return Some(ThresholdLine::RateChangeModifier);
            }
        }
        ProgramReleaseMode::InputDependent => {}
    }
    None
}

pub fn draw_input_level_threshold(
    canvas: &mut Canvas,
    bounds: BoundingBox,
    params: &ComposureParams,
    db_to_y: impl Fn(f64) -> f32,
    opacity: f32,
    active: Option<ThresholdLine>,
) {
    if !params.prog_release_mode.value().is_input_dependent() {
        return;
    }
    let thresh = params.input_level_threshold_db.value();
    let y = db_to_y(thresh as f64);
    let highlight = active == Some(ThresholdLine::InputLevel);
    draw_h_line(canvas, bounds.x, y, bounds.w, line_opacity(opacity, highlight));
}

pub fn draw_gr_blend_thresholds(
    canvas: &mut Canvas,
    bounds: BoundingBox,
    params: &ComposureParams,
    range_db: f32,
    opacity: f32,
    active: Option<ThresholdLine>,
) {
    if !params.prog_release_mode.value().is_gr_dependent() {
        return;
    }

    let span = range_db.max(1.0);
    let knee_span = (span * 2.0).max(1.0);
    let blend = params.prog_release_blend.value();
    let opacity_mult = if blend <= 50.0 {
        0.2 + (blend / 50.0) * 0.8
    } else {
        1.0
    }
    .clamp(0.2, 1.0);

    draw_gr_blend_side(
        canvas,
        bounds,
        span,
        knee_span,
        params.gr_blend_threshold_reduction_db.value(),
        params.gr_blend_threshold_reduction_knee_db.value(),
        GrAxis::Cut,
        ThresholdLine::GrBlendReduction,
        opacity,
        opacity_mult,
        active,
    );
    draw_gr_blend_side(
        canvas,
        bounds,
        span,
        knee_span,
        params.gr_blend_threshold_addition_db.value(),
        params.gr_blend_threshold_addition_knee_db.value(),
        GrAxis::Boost,
        ThresholdLine::GrBlendAddition,
        opacity,
        opacity_mult,
        active,
    );
}

fn draw_gr_blend_side(
    canvas: &mut Canvas,
    bounds: BoundingBox,
    span: f32,
    knee_span: f32,
    db: f32,
    knee_db: f32,
    axis: GrAxis,
    line: ThresholdLine,
    opacity: f32,
    opacity_mult: f32,
    active: Option<ThresholdLine>,
) {
    let y = axis.y(bounds, db, span);
    if knee_db > 0.0 {
        let knee_h = (knee_db / knee_span) * bounds.h;
        let (knee_y, knee_h_actual) = if axis.from_top() {
            let knee_top = (y - knee_h).max(bounds.y);
            (knee_top, y - knee_top)
        } else {
            let knee_bottom = (y + knee_h).min(bounds.y + bounds.h);
            (y, knee_bottom - y)
        };
        draw_knee_bar(
            canvas,
            bounds.x,
            knee_y,
            bounds.w,
            knee_h_actual,
            opacity * 0.25 * opacity_mult,
        );
    }
    let highlight = active == Some(line);
    draw_h_line(
        canvas,
        bounds.x,
        y,
        bounds.w,
        line_opacity(opacity * opacity_mult, highlight),
    );
}

pub fn draw_rate_change_threshold(
    canvas: &mut Canvas,
    bounds: BoundingBox,
    params: &ComposureParams,
    opacity: f32,
    active: Option<ThresholdLine>,
) {
    if !params.prog_release_mode.value().is_rate_of_change() {
        return;
    }
    let modifier = params.rate_change_threshold_modifier.value();
    let y = rate_modifier_y(bounds, modifier);
    let highlight = active == Some(ThresholdLine::RateChangeModifier);
    draw_h_line(canvas, bounds.x, y, bounds.w, line_opacity(opacity, highlight));
}

fn line_opacity(base: f32, highlight: bool) -> f32 {
    if highlight {
        base
    } else {
        base * 0.85
    }
}

fn draw_h_line(canvas: &mut Canvas, x: f32, y: f32, w: f32, opacity: f32) {
    let mut path = Path::new();
    path.rect(x, y - 1.0, w, 2.0);
    let paint = Paint::color(VgColor::rgbaf(1.0, 1.0, 1.0, opacity));
    canvas.fill_path(&path, &paint);
}

fn draw_knee_bar(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, opacity: f32) {
    if h < 0.5 {
        return;
    }
    let mut path = Path::new();
    path.rect(x, y, w, h);
    let paint = Paint::color(VgColor::rgbaf(1.0, 1.0, 1.0, opacity));
    canvas.fill_path(&path, &paint);
}
