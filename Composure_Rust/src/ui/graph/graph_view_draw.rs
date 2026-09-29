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
            let alpha = display::fade_by_age(
                dot.age_secs,
                DOT_TRAIL_MAX_AGE_SECS,
                DOT_TRAIL_FADE_PEAK,
                false,
            ) * opacity;
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

fn histogram_edge_fade(x: f32, width: f32, scale: f32) -> f32 {
    if scale == 0.0 {
        return (x / 100.0).clamp(0.0, 1.0);
    }
    let left = (x / (200.0 * scale)).clamp(0.0, 1.0);
    let right = ((width - x) / (80.0 * scale)).clamp(0.0, 1.0);
    left.min(right)
}

pub fn draw_histograms(
    canvas: &mut Canvas,
    bounds: BoundingBox,
    display: &UiDisplay,
    min_db: f64,
    px_per_db: f32,
    opacity: f32,
    extension_scale: f32,
) {
    let left_extension = 32.0 * extension_scale;
    let right_extension = 12.0 * extension_scale;
    let width = bounds.w + left_extension + right_extension;
    let w = width.ceil() as usize;
    let display_min = graph_display::display_min_db(min_db);
    display.read_histograms(|gr_hist, input_hist| {
        for i in 0..w {
            let age = (w - 1 - i) * (HISTOGRAM_LEN - 1) / w.max(1);
            let fade = histogram_edge_fade(i as f32, width, extension_scale);

            let gr = gr_hist.sample_at_age(age);
            let x = bounds.x - left_extension + i as f32;
            let bar_opacity = opacity * 0.6 * fade;
            let col_bounds = BoundingBox {
                x,
                y: bounds.y,
                w: 1.0,
                h: bounds.h,
            };
            if gr.is_finite() {
                if let Some((bx, by, _, bh)) = draw_helpers::gr_bar_rect(gr, col_bounds, px_per_db)
                {
                    let mut p = Path::new();
                    p.rect(bx, by, 1.0, bh);
                    canvas.fill_path(&p, &Paint::color(gr_vg_color(gr, bar_opacity)));
                }
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
        let db = -(i as f64 * graph_display::grid_step_db(range_db));
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

/// Faded curve continuation from the anchor into the bottom-left margin (off-graph zone).
pub fn draw_bl_margin_extension(
    canvas: &mut Canvas,
    db_to_x: &dyn Fn(f64) -> f32,
    db_to_y: &dyn Fn(f64) -> f32,
    corner_x: f64,
    corner_y: f64,
    min_db: f64,
    opacity: f32,
) {
    let d_min = graph_display::display_min_db(min_db);
    if d_min >= min_db - 0.001 {
        return;
    }

    // Below the operational floor the processor is unity, regardless of
    // the first editable segment's slope.
    let end_y = corner_y + (d_min - corner_x);

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
