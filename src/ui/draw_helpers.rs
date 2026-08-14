//! Shared Vizia drawing helpers (GR bars, reflection strips).

use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::{Color as VgColor, Paint, Path};

use super::theme;

pub fn gr_vg_color(gr: f32, alpha: f32) -> VgColor {
    if gr < 0.0 {
        VgColor::rgbaf(theme::GR_CUT_R, theme::GR_CUT_G, theme::GR_CUT_B, alpha)
    } else {
        VgColor::rgbaf(theme::GR_BOOST_R, theme::GR_BOOST_G, theme::GR_BOOST_B, alpha)
    }
}

/// GR meter threshold line Y: cut from top, boost from bottom.
pub fn gr_threshold_y(bounds: BoundingBox, db: f32, range_db: f32, from_top: bool) -> f32 {
    let norm = db / range_db.max(1.0);
    if from_top {
        bounds.y + norm * bounds.h
    } else {
        bounds.y + bounds.h - norm * bounds.h
    }
}

pub fn gr_threshold_db_from_y(y: f32, bounds: BoundingBox, range_db: f32, from_top: bool) -> f32 {
    let norm = if from_top {
        (y - bounds.y) / bounds.h.max(1.0)
    } else {
        (bounds.y + bounds.h - y) / bounds.h.max(1.0)
    };
    norm * range_db.max(1.0)
}

/// Vertical GR bar rect inside `bounds`: cut from top, boost from bottom.
pub fn gr_bar_rect(gr: f32, bounds: BoundingBox, px_per_db: f32) -> Option<(f32, f32, f32, f32)> {
    let gr_height = (gr.abs() * px_per_db).min(bounds.h);
    if gr_height < 0.5 {
        return None;
    }
    if gr < 0.0 {
        Some((bounds.x, bounds.y, bounds.w, gr_height))
    } else {
        let y = bounds.y + bounds.h - gr_height;
        Some((bounds.x, y, bounds.w, gr_height))
    }
}

/// Horizontal GR trail segment Y, clamped toward `center_y` (0 dB).
pub fn gr_trail_line_y(
    gr: f32,
    bounds: BoundingBox,
    px_per_db: f32,
    center_y: f32,
) -> Option<f32> {
    let gr_height = (gr.abs() * px_per_db).min(bounds.h);
    if gr_height < 0.5 {
        return None;
    }
    if gr < 0.0 {
        Some((bounds.y + gr_height).min(center_y))
    } else if gr > 0.0 {
        Some((bounds.y + bounds.h - gr_height).max(center_y))
    } else {
        None
    }
}

pub fn gr_fill_bar(canvas: &mut Canvas, gr: f32, x: f32, y: f32, w: f32, h: f32, opacity: f32) {
    let mut path = Path::new();
    path.rect(x, y, w, h);
    canvas.fill_path(&path, &Paint::color(gr_vg_color(gr, opacity)));
}

pub fn reflection_weight_from_level(level_db: f32, range_db: f32) -> f32 {
    (level_db.abs() / range_db.max(1.0)).min(1.0)
}

pub fn reflection_alpha(weight: f32, peak: f32, opacity: f32, min_alpha: f32) -> Option<f32> {
    let alpha = peak * weight * opacity;
    if alpha < min_alpha {
        None
    } else {
        Some(alpha)
    }
}

pub fn draw_reflection_strip(
    canvas: &mut Canvas,
    x: f32,
    y: f32,
    w: f32,
    r: f32,
    g: f32,
    b: f32,
    base_alpha: f32,
    levels: &[f32],
) {
    if base_alpha < 0.001 || levels.is_empty() {
        return;
    }
    let strip_h = theme::METER_REFLECTION_H / levels.len() as f32;
    for (i, &level) in levels.iter().enumerate() {
        let mut path = Path::new();
        path.rect(x, y + i as f32 * strip_h, w, strip_h);
        canvas.fill_path(
            &path,
            &Paint::color(VgColor::rgbaf(r, g, b, base_alpha * level)),
        );
    }
}
