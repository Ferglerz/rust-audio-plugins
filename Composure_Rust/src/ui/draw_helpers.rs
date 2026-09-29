//! Shared Vizia drawing helpers for GR bars and thresholds.

use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::Color as VgColor;

use super::theme;

pub fn gr_vg_color(gr: f32, alpha: f32) -> VgColor {
    if gr < 0.0 {
        VgColor::rgbaf(theme::GR_CUT_R, theme::GR_CUT_G, theme::GR_CUT_B, alpha)
    } else {
        VgColor::rgbaf(
            theme::GR_BOOST_R,
            theme::GR_BOOST_G,
            theme::GR_BOOST_B,
            alpha,
        )
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
