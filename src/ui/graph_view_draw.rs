use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::{Color as VgColor, Paint, Path};

use super::display::{self, UiDisplay, DOT_TRAIL_FADE_PEAK, DOT_TRAIL_MAX_AGE_SECS, HISTOGRAM_LEN};
use super::draw_helpers::{self, gr_vg_color};
use super::graph_display;
use super::theme::{self, Color};

pub fn draw_input_trail_dots(
    canvas: &mut Canvas,
    bounds: BoundingBox,
    display: &UiDisplay,
    min_db: f64,
    range_db: f64,
    opacity: f32,
) {
    display.read_dot_trails(|dots| {
        for dot in dots {
            let alpha = display::fade_by_age(dot.age_secs, DOT_TRAIL_MAX_AGE_SECS, DOT_TRAIL_FADE_PEAK, false) * opacity;
            if alpha < 0.01 {
                continue;
            }
            let db = dot.input_db as f64;
            let x = graph_display::db_to_local_x(db, bounds, min_db, range_db);
            let y = graph_display::db_to_local_y(db, bounds, min_db, range_db);
            let mut path = Path::new();
            path.circle(x, y, 2.0);
            let paint = Paint::color(VgColor::rgbaf(0.5, 0.9, 0.5, alpha));
            canvas.fill_path(&path, &paint);
        }
    });
}

pub fn draw_graph_reflection(
    canvas: &mut Canvas,
    bounds: BoundingBox,
    detector_db: f32,
    range_db: f32,
    opacity: f32,
) {
    let weight = draw_helpers::reflection_weight_from_level(detector_db, range_db);
    let Some(alpha) = draw_helpers::reflection_alpha(weight, 0.12, opacity, 0.002) else {
        return;
    };
    let ref_y = bounds.y + bounds.h + theme::METER_REFLECTION_GAP;
    let strip_w = bounds.w + theme::METER_W * 2.0 + theme::METER_GAP + 24.0;
    let strip_x = bounds.x - 10.0;
    let levels = [0.35, 0.55, 0.75, 0.9, 0.75, 0.55, 0.35];
    draw_helpers::draw_reflection_strip(
        canvas,
        strip_x,
        ref_y,
        strip_w,
        0.55,
        0.75,
        0.55,
        alpha,
        &levels,
    );
}

pub fn draw_histograms(
    canvas: &mut Canvas,
    bounds: BoundingBox,
    display: &UiDisplay,
    min_db: f64,
    px_per_db: f32,
    opacity: f32,
) {
    let w = bounds.w as usize;
    let display_min = graph_display::display_min_db(min_db);
    display.read_histograms(|gr_hist, input_hist| {
        for i in 0..w {
            let age = (w - 1 - i) * (HISTOGRAM_LEN - 1) / w.max(1);
            let fade = if i < 100 { i as f32 / 100.0 } else { 1.0 };

            let gr = gr_hist.sample_at_age(age);
            let x = bounds.x + i as f32;
            let bar_opacity = opacity * 0.6 * fade;
            let col_bounds = BoundingBox {
                x,
                y: bounds.y,
                w: 1.0,
                h: bounds.h,
            };
            if let Some((bx, by, _, bh)) = draw_helpers::gr_bar_rect(gr, col_bounds, px_per_db) {
                let mut p = Path::new();
                p.rect(bx, by, 1.0, bh);
                canvas.fill_path(&p, &Paint::color(gr_vg_color(gr, bar_opacity)));
            }

            let lvl = input_hist.sample_at_age(age);
            if lvl > display_min as f32 {
                let bar_h = ((lvl - display_min as f32) * px_per_db).min(bounds.h);
                let bar_top = bounds.y + bounds.h - bar_h;
                let mut p = Path::new();
                p.rect(x, bar_top, 1.0, bar_h);
                let paint = Paint::color(VgColor::rgbaf(1.0, 1.0, 1.0, opacity * 0.25 * fade));
                canvas.fill_path(&p, &paint);
            }
        }
    });
}

pub fn draw_grid(
    canvas: &mut Canvas,
    bounds: BoundingBox,
    min_db: f64,
    range_db: f64,
    opacity: f32,
) {
    let divisions = graph_display::grid_line_count(range_db);
    let grid_paint = {
        let mut p = Paint::color(vg_color(theme::GRAPH_GRID, opacity * 0.6));
        p.set_line_width(1.0);
        p
    };
    for i in 0..=divisions {
        let db = -(i as f64 * graph_display::GRID_STEP_DB);
        let norm = graph_display::db_to_norm(db, min_db, range_db);
        let x = bounds.x + norm as f32 * bounds.w;
        let y = graph_display::db_to_local_y(db, bounds, min_db, range_db);
        let mut v = Path::new();
        v.move_to(x, bounds.y);
        v.line_to(x, bounds.y + bounds.h);
        canvas.stroke_path(&v, &grid_paint);
        let mut h = Path::new();
        h.move_to(bounds.x, y);
        h.line_to(bounds.x + bounds.w, y);
        canvas.stroke_path(&h, &grid_paint);
    }
}

/// Fade the out-of-scope margin around the operational square (uniform on all sides).
pub fn draw_out_of_scope_fade(
    canvas: &mut Canvas,
    bounds: BoundingBox,
    min_db: f64,
    max_db: f64,
    range_db: f64,
    opacity: f32,
) {
    let x0 = bounds.x;
    let y0 = bounds.y;
    let w = bounds.w;
    let h = bounds.h;
    let x_lo = x0 + (graph_display::db_to_norm(min_db, min_db, range_db) * w as f64) as f32;
    let x_hi = x0 + (graph_display::db_to_norm(max_db, min_db, range_db) * w as f64) as f32;
    let y_hi = y0 + h - (graph_display::db_to_norm(max_db, min_db, range_db) * h as f64) as f32;
    let y_lo = y0 + h - (graph_display::db_to_norm(min_db, min_db, range_db) * h as f64) as f32;

    let fade = Paint::color(VgColor::rgbaf(0.05, 0.05, 0.05, 0.5 * opacity));

    // Partition margin into four non-overlapping bands (avoids corner double-darkening).
    if y_lo < y0 + h {
        let mut bottom = Path::new();
        bottom.rect(x0, y_lo, w, y0 + h - y_lo);
        canvas.fill_path(&bottom, &fade);
    }
    if x_lo > x0 {
        let mut left = Path::new();
        left.rect(x0, y_hi, x_lo - x0, y_lo - y_hi);
        canvas.fill_path(&left, &fade);
    }
    if y_hi > y0 {
        let mut top = Path::new();
        top.rect(x0, y0, w, y_hi - y0);
        canvas.fill_path(&top, &fade);
    }
    if x_hi < x0 + w {
        let mut right = Path::new();
        right.rect(x_hi, y_hi, x0 + w - x_hi, y_lo - y_hi);
        canvas.fill_path(&right, &fade);
    }

    let mut edge_paint = Paint::color(vg_color(theme::GRAPH_GRID, opacity * 0.35));
    edge_paint.set_line_width(1.0);

    let mut floor = Path::new();
    floor.move_to(x0, y_lo);
    floor.line_to(x0 + w, y_lo);
    canvas.stroke_path(&floor, &edge_paint);

    let mut ceiling = Path::new();
    ceiling.move_to(x0, y_hi);
    ceiling.line_to(x0 + w, y_hi);
    canvas.stroke_path(&ceiling, &edge_paint);

    let mut left_edge = Path::new();
    left_edge.move_to(x_lo, y0);
    left_edge.line_to(x_lo, y0 + h);
    canvas.stroke_path(&left_edge, &edge_paint);

    let mut right_edge = Path::new();
    right_edge.move_to(x_hi, y0);
    right_edge.line_to(x_hi, y0 + h);
    canvas.stroke_path(&right_edge, &edge_paint);
}

/// Faded curve continuation from the anchor into the bottom-left margin (off-graph zone).
pub fn draw_bl_margin_extension(
    canvas: &mut Canvas,
    db_to_x: &dyn Fn(f64) -> f32,
    db_to_y: &dyn Fn(f64) -> f32,
    segments: &[crate::dsp::graph::CurveSegment],
    corner_x: f64,
    corner_y: f64,
    min_db: f64,
    opacity: f32,
) {
    let d_min = graph_display::display_min_db(min_db);
    if d_min >= min_db - 0.001 {
        return;
    }

    let end_y = if segments.is_empty() {
        d_min
    } else {
        let first = &segments[0];
        let dx = first.x2 - first.x1;
        if dx.abs() > 0.0001 {
            first.y1 + (first.y2 - first.y1) / dx * (d_min - first.x1)
        } else {
            corner_y + (d_min - corner_x)
        }
    };

    let mut paint = Paint::color(vg_color(theme::GRAPH_CURVE, opacity * 0.45));
    paint.set_line_width(2.0);
    let mut path = Path::new();
    path.move_to(db_to_x(corner_x), db_to_y(corner_y));
    path.line_to(db_to_x(d_min), db_to_y(end_y));
    canvas.stroke_path(&path, &paint);
}

pub fn vg_color(color: Color, opacity: f32) -> VgColor {
    let c: VgColor = color.into();
    VgColor::rgbaf(c.r, c.g, c.b, c.a * opacity)
}

pub fn draw_dot(canvas: &mut Canvas, x: f32, y: f32, radius: f32, color: VgColor) {
    let mut path = Path::new();
    path.circle(x, y, radius);
    canvas.fill_path(&path, &Paint::color(color));
}
