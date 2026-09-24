use nih_plug_vizia::vizia::prelude::BoundingBox;
use nih_plug_vizia::vizia::vg::Color as C;
use pleasant_ui::theme::rgb;
use std::cell::Cell;
use std::time::Duration;

pub(super) const LIFT_COLOR: C = rgb(110, 215, 255);
pub(super) const UI_W: f32 = 1282.0;
pub(super) const UI_H: f32 = 672.0;
pub(super) const HEADER_H: f32 = 82.0;
pub(super) const MARGIN: f32 = 16.0;
pub(super) const GAP: f32 = 12.0;
pub(super) const DYN_W: f32 = 116.0;
pub(super) const PSE_W: f32 = 116.0;
pub(super) const WALL_W: f32 = 116.0;
pub(super) const MODULE_Y: f32 = 92.0;
pub(super) const MODULE_H: f32 = 516.0;
pub(super) const MODULE_HEADER_H: f32 = 44.0;
pub(super) const MODULE_HEADER_CTRL: f32 = 24.0;
pub(super) const MODULE_HEADER_INSET: f32 = 10.0;
pub(super) const EQ_PAGE_1: usize = 0;
pub(super) const EQ_PAGE_2: usize = 1;
pub(super) const EQ_PAGE_LIFT: usize = 2;
pub(super) const EQ_PAGE_SC: usize = 3;
pub(super) const EQ_W: f32 = UI_W - 2.0 * MARGIN - 3.0 * GAP - DYN_W - PSE_W - WALL_W;
pub(super) const FOOTER_LINE_Y: f32 = MODULE_Y + MODULE_H + 16.0;
pub(super) const FOOTER_BTN_Y: f32 = FOOTER_LINE_Y + 8.0;
pub(super) const EQ_GRAPH_PAD_LEFT: f32 = 54.0;
pub(super) const EQ_GRAPH_PAD_RIGHT: f32 = 26.0;
#[cfg(test)]
pub(super) const GX: f32 = MARGIN + PSE_W + GAP + EQ_GRAPH_PAD_LEFT;
pub(super) const GY: f32 = 156.0;
#[cfg(test)]
pub(super) const GW: f32 = EQ_W - EQ_GRAPH_PAD_LEFT - EQ_GRAPH_PAD_RIGHT;
pub(super) const GH: f32 = 384.0;
pub(super) const GRAPH_BOTTOM: f32 = GY + GH;
/// Vertical midline of the gap under the EQ graph (shared by axis labels + meter readouts).
pub(super) const EQ_AXIS_LABEL_Y: f32 = GRAPH_BOTTOM + (MODULE_Y + MODULE_H - GRAPH_BOTTOM) * 0.5;
pub(super) const METER_TOP: f32 = GY;
pub(super) const METER_H: f32 = GRAPH_BOTTOM - METER_TOP;
pub(super) const KNEE_METER_OVERHANG: f32 = 4.0;
pub(super) const FREQ_AXIS: [(f64, &str); 8] = [
    (50.0, "50"),
    (100.0, "100"),
    (200.0, "200"),
    (500.0, "500"),
    (1000.0, "1k"),
    (2500.0, "2.5k"),
    (5000.0, "5k"),
    (10000.0, "10k"),
];
pub(super) const THEME_BUTTON: (f32, f32, f32, f32) = (32.0, FOOTER_BTN_Y, 88.0, 28.0);
pub(super) const EQ2_ID_BASE: u64 = 10_000;
pub(super) const SC_EQ_ID_BASE: u64 = 20_000;
pub(super) const GRAPH_CLIP_W: f32 = EQ_W;
pub(super) const PSE_BLUE: C = rgb(108, 176, 242);
pub(super) const WALL_COLOR: C = rgb(220, 108, 88);
pub(super) const PSE_CONTROLS_KNOBS: [usize; 3] = [8, 10, 2];
pub(super) const WALL_CONTROLS_KNOBS: [usize; 2] = [16, 17];
pub(super) const WALL_MAIN_KNOBS: [usize; 1] = [18];

pub(super) fn band_display_num(id: u64) -> u64 {
    if id >= SC_EQ_ID_BASE {
        id - SC_EQ_ID_BASE
    } else if id >= EQ2_ID_BASE {
        id - EQ2_ID_BASE
    } else {
        id
    }
}

pub(super) fn quintic_page_progress(progress: f32) -> f32 {
    let unit = progress.floor();
    let frac = progress - unit;
    unit + pleasant_ui::page_slide::ease(frac)
}

pub(super) fn view_scale(bounds: BoundingBox) -> Option<f32> {
    if !bounds.w.is_finite() || !bounds.h.is_finite() || bounds.w < 8.0 || bounds.h < 8.0 {
        return None;
    }
    let scale = bounds.w / UI_W;
    (scale.is_finite() && scale > 0.01).then_some(scale)
}

pub(super) const MODULE_SETTINGS_HOLD: Duration = Duration::from_secs(2);

pub(super) fn tick_page_anim(progress: &Cell<f32>, target: f32, dt: f32) {
    let cur = progress.get();
    let next = if cur < target {
        (cur + dt / 0.25).min(target)
    } else if cur > target {
        (cur - dt / 0.25).max(target)
    } else {
        cur
    };
    progress.set(next);
}

pub(super) const PROCESS_BUTTON: (f32, f32, f32, f32) = (128.0, FOOTER_BTN_Y, 152.0, 28.0);

pub(super) fn processing_menu_rect(resolution: bool) -> (f32, f32, f32, f32) {
    let r = if resolution {
        resolution_button_rect()
    } else {
        PROCESS_BUTTON
    };
    let rows = if resolution { 5.0 } else { 3.0 };
    let height = rows * dropdown_row_h(PROCESS_BUTTON_TEXT);
    (r.0, r.1 - height, r.2, height)
}

pub(super) fn resolution_button_rect() -> (f32, f32, f32, f32) {
    (288.0, FOOTER_BTN_Y, 104.0, 28.0)
}

pub(super) fn output_gain_rect() -> (f32, f32, f32, f32) {
    (UI_W - MARGIN - WALL_W, FOOTER_BTN_Y, WALL_W, 28.0)
}

pub(super) fn output_gain_value_rect() -> (f32, f32, f32, f32) {
    let r = output_gain_rect();
    (r.0 + r.2 - 62.0, r.1 + 2.0, 56.0, 16.0)
}

pub(super) const SCALES: [f64; 12] = [
    6.0, 12.0, 18.0, 24.0, 30.0, 36.0, 42.0, 48.0, 54.0, 60.0, 66.0, 72.0,
];

/// Graph scale button (±24 ▾) and its dropdown rows.
pub(super) const SCALE_BUTTON_TEXT: f32 = 13.0;
/// Footer `d.button` label size (process / resolution) and matching dropdowns.
pub(super) const PROCESS_BUTTON_TEXT: f32 = 11.0;
/// Vertical padding so a dropdown row fits its label.
pub(super) const DROPDOWN_ROW_PAD: f32 = 12.0;

pub(super) fn dropdown_row_h(text_size: f32) -> f32 {
    text_size + DROPDOWN_ROW_PAD
}
