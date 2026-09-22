mod preferences;
use crate::{
    band::{infer_shape, Band, Shape},
    dsp::BandCoeffs,
    engine::Shared,
    lift::{filter_influence, LiftBand, LIFT_ID_BASE},
    params::StripParams,
    processing::{Config, ProcessingMode, MODES, RESOLUTIONS},
};
use nih_plug::prelude::*;
use nih_plug_vizia::widgets::util::ModifiersExt;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{
        prelude::*,
        vg::{Color as C, FontId, Paint, Path},
    },
    widgets::RawParamEvent,
    ViziaTheming,
};
use pleasant_ui::{
    spectrum::smooth_bins,
    theme::{rgb, BG, COLORS, GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
    ButtonAnim, Draw, ValueEdit, FONT_JETBRAINS_MONO,
};
use std::{
    cell::Cell,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

pub const LIFT_COLOR: C = rgb(110, 215, 255);
const UI_W: f32 = 1282.0;
const UI_H: f32 = 656.0;
const HEADER_H: f32 = 82.0;
const MARGIN: f32 = 16.0;
const GAP: f32 = 12.0;
const DYN_W: f32 = 122.0;
const PSE_W: f32 = 122.0;
const WALL_W: f32 = 122.0;
const MODULE_Y: f32 = 92.0;
const MODULE_H: f32 = 500.0;
const MODULE_HEADER_H: f32 = 44.0;
const MODULE_HEADER_CTRL: f32 = 24.0;
const EQ_PAGE_1: usize = 0;
const EQ_PAGE_2: usize = 1;
const EQ_PAGE_LIFT: usize = 2;
const EQ_PAGE_SC: usize = 3;
const EQ_W: f32 = UI_W - 2.0 * MARGIN - 3.0 * GAP - DYN_W - PSE_W - WALL_W;
const FOOTER_LINE_Y: f32 = 608.0;
const FOOTER_BTN_Y: f32 = 616.0;
const EQ_GRAPH_PAD_LEFT: f32 = 54.0;
const EQ_GRAPH_PAD_RIGHT: f32 = 26.0;
#[cfg(test)]
const GX: f32 = MARGIN + PSE_W + GAP + EQ_GRAPH_PAD_LEFT;
const GY: f32 = 156.0;
#[cfg(test)]
const GW: f32 = EQ_W - EQ_GRAPH_PAD_LEFT - EQ_GRAPH_PAD_RIGHT;
const GH: f32 = 384.0;
const GRAPH_BOTTOM: f32 = GY + GH;
const EQ_AXIS_LABEL_Y: f32 = GRAPH_BOTTOM + 32.0;
const METER_TOP: f32 = GY;
const METER_H: f32 = GRAPH_BOTTOM - METER_TOP;
const KNEE_METER_OVERHANG: f32 = 4.0;
const FREQ_AXIS: [(f64, &str); 8] = [
    (50.0, "50"),
    (100.0, "100"),
    (200.0, "200"),
    (500.0, "500"),
    (1000.0, "1k"),
    (2500.0, "2.5k"),
    (5000.0, "5k"),
    (10000.0, "10k"),
];
const THEME_BUTTON: (f32, f32, f32, f32) = (32.0, FOOTER_BTN_Y, 88.0, 28.0);
pub const EQ2_ID_BASE: u64 = 10_000;
pub const SC_EQ_ID_BASE: u64 = 20_000;
const GRAPH_CLIP_W: f32 = EQ_W;
const PSE_BLUE: C = rgb(108, 176, 242);
const WALL_COLOR: C = rgb(220, 108, 88);
const PSE_CONTROLS_KNOBS: [usize; 3] = [8, 2, 10];
const WALL_CONTROLS_KNOBS: [usize; 2] = [16, 17];
const WALL_MAIN_KNOBS: [usize; 1] = [18];
fn band_display_num(id: u64) -> u64 {
    if id >= SC_EQ_ID_BASE {
        id - SC_EQ_ID_BASE
    } else if id >= EQ2_ID_BASE {
        id - EQ2_ID_BASE
    } else {
        id
    }
}
fn quintic_page_progress(progress: f32) -> f32 {
    let unit = progress.floor();
    let frac = progress - unit;
    unit + frac * frac * frac * (frac * (frac * 6.0 - 15.0) + 10.0)
}
fn view_scale(bounds: BoundingBox) -> Option<f32> {
    if !bounds.w.is_finite() || !bounds.h.is_finite() || bounds.w < 8.0 || bounds.h < 8.0 {
        return None;
    }
    let scale = bounds.w / UI_W;
    (scale.is_finite() && scale > 0.01).then_some(scale)
}
fn tick_page_anim(progress: &Cell<f32>, target: f32, dt: f32) {
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
const PROCESS_BUTTON: (f32, f32, f32, f32) = (128.0, FOOTER_BTN_Y, 152.0, 28.0);
fn processing_menu_rect(resolution: bool) -> (f32, f32, f32, f32) {
    let r = if resolution {
        resolution_button_rect()
    } else {
        PROCESS_BUTTON
    };
    let height = if resolution { 120.0 } else { 72.0 };
    (r.0, r.1 - height, r.2, height)
}
fn resolution_button_rect() -> (f32, f32, f32, f32) {
    (288.0, FOOTER_BTN_Y, 104.0, 28.0)
}
fn output_gain_rect() -> (f32, f32, f32, f32) {
    (UI_W - MARGIN - WALL_W, FOOTER_BTN_Y, WALL_W, 28.0)
}
fn output_gain_value_rect() -> (f32, f32, f32, f32) {
    let r = output_gain_rect();
    (r.0 + r.2 - 62.0, r.1 + 2.0, 56.0, 16.0)
}
const SCALES: [f64; 7] = [6.0, 12.0, 24.0, 36.0, 48.0, 60.0, 72.0];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DynPage {
    Main,
    Controls,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PsePage {
    Main,
    Controls,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum WallPage {
    Main,
    Controls,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ValueTarget {
    Global(usize),
    // Frequency, gain, Q, threshold, ratio, attack, release, range.
    Band(usize),
    Lift(usize),
}
fn typed_char(code: Code, shift: bool) -> Option<char> {
    Some(match code {
        Code::Digit0 | Code::Numpad0 => '0',
        Code::Digit1 | Code::Numpad1 => '1',
        Code::Digit2 | Code::Numpad2 => '2',
        Code::Digit3 | Code::Numpad3 => '3',
        Code::Digit4 | Code::Numpad4 => '4',
        Code::Digit5 | Code::Numpad5 => '5',
        Code::Digit6 | Code::Numpad6 => '6',
        Code::Digit7 | Code::Numpad7 => '7',
        Code::Digit8 | Code::Numpad8 => '8',
        Code::Digit9 | Code::Numpad9 => '9',
        Code::Period | Code::NumpadDecimal => '.',
        Code::Comma => ',',
        Code::Minus | Code::NumpadSubtract => '-',
        Code::Equal => {
            if shift {
                '+'
            } else {
                '='
            }
        }
        Code::NumpadAdd => '+',
        Code::Slash | Code::NumpadDivide => '/',
        Code::Space => ' ',
        Code::KeyA => letter(shift, 'a'),
        Code::KeyB => letter(shift, 'b'),
        Code::KeyC => letter(shift, 'c'),
        Code::KeyD => letter(shift, 'd'),
        Code::KeyE => letter(shift, 'e'),
        Code::KeyF => letter(shift, 'f'),
        Code::KeyG => letter(shift, 'g'),
        Code::KeyH => letter(shift, 'h'),
        Code::KeyI => letter(shift, 'i'),
        Code::KeyJ => letter(shift, 'j'),
        Code::KeyK => letter(shift, 'k'),
        Code::KeyL => letter(shift, 'l'),
        Code::KeyM => letter(shift, 'm'),
        Code::KeyN => letter(shift, 'n'),
        Code::KeyO => letter(shift, 'o'),
        Code::KeyP => letter(shift, 'p'),
        Code::KeyQ => letter(shift, 'q'),
        Code::KeyR => letter(shift, 'r'),
        Code::KeyS => letter(shift, 's'),
        Code::KeyT => letter(shift, 't'),
        Code::KeyU => letter(shift, 'u'),
        Code::KeyV => letter(shift, 'v'),
        Code::KeyW => letter(shift, 'w'),
        Code::KeyX => letter(shift, 'x'),
        Code::KeyY => letter(shift, 'y'),
        Code::KeyZ => letter(shift, 'z'),
        _ => return None,
    })
}
fn letter(shift: bool, c: char) -> char {
    if shift {
        c.to_ascii_uppercase()
    } else {
        c
    }
}
fn parse_value(text: &str, target: ValueTarget) -> Option<f64> {
    let text = text.trim().to_ascii_lowercase();
    if target == ValueTarget::Global(1) && text == "off" {
        return Some(-80.0);
    }
    if target == ValueTarget::Global(10) {
        match text.as_str() {
            "a" => return Some(0.0),
            "b" => return Some(1.0),
            "c" => return Some(2.0),
            "d" => return Some(3.0),
            "e" => return Some(4.0),
            "f" => return Some(5.0),
            _ => {}
        }
        if let Some(s) = text.strip_suffix("ms") {
            if let Ok(ms) = s.trim().parse::<f64>() {
                return Some(crate::dsp::seconds_to_pse_time_pos(ms * 0.001));
            }
        }
        if let Some(s) = text.strip_suffix("s") {
            if let Ok(sec) = s.trim().parse::<f64>() {
                return Some(crate::dsp::seconds_to_pse_time_pos(sec));
            }
        }
        if let Ok(val) = text.parse::<f64>() {
            if (0.0..=5.0).contains(&val) {
                return Some(val);
            }
            if val > 5.0 {
                return Some(crate::dsp::seconds_to_pse_time_pos(val * 0.001));
            }
        }
        return None;
    }
    let suffixes: &[(&str, f64)] = match target {
        ValueTarget::Band(0) | ValueTarget::Lift(0) => {
            &[("khz", 1000.0), ("hz", 1.0), ("k", 1000.0)]
        }
        ValueTarget::Band(5 | 6) | ValueTarget::Lift(5 | 6) | ValueTarget::Global(5 | 11) => {
            &[("ms", 1.0), ("s", 1000.0)]
        }
        ValueTarget::Band(4) | ValueTarget::Lift(4) | ValueTarget::Global(6) => &[(":1", 1.0)],
        ValueTarget::Global(2 | 3 | 4 | 16 | 17) => &[("%", 1.0)],
        ValueTarget::Global(0 | 18) => &[("db", 1.0)],
        ValueTarget::Band(2) | ValueTarget::Lift(2) => &[],
        _ => &[("db", 1.0)],
    };
    let (number, multiplier) = suffixes
        .iter()
        .find_map(|(suffix, multiplier)| {
            text.strip_suffix(suffix)
                .map(|number| (number.trim(), *multiplier))
        })
        .unwrap_or((&text, 1.0));
    let value = number.parse::<f64>().ok()? * multiplier;
    value.is_finite().then_some(value)
}
fn freq_x_at(freq: f64, gx: f32, gw: f32) -> f32 {
    gx + gw * (freq / 20.0).log(1000.0).clamp(0.0, 1.0) as f32
}
fn x_freq_at(x: f32, gx: f32, gw: f32) -> f64 {
    20.0 * 1000.0_f64.powf(((x - gx) / gw).clamp(0.0, 1.0) as f64)
}
fn db_y(db: f64, range: f64) -> f32 {
    GY + GH * (0.5 - (db.clamp(-range, range) / (2.0 * range)) as f32)
}
fn y_db(y: f32, range: f64) -> f64 {
    ((0.5 - (y - GY) as f64 / GH as f64) * 2.0 * range).clamp(-24.0, 24.0)
}
/// Pixel-spaced x plus each band's exact frequency so peaks and notch zeros
/// land on a vertex instead of being skipped by a coarse chord.
fn eq_curve_xs(gx: f32, gw: f32, extra_freqs: impl IntoIterator<Item = f64>) -> Vec<f32> {
    let n = gw.max(2.0).ceil() as usize;
    let mut xs: Vec<f32> = (0..=n).map(|i| gx + gw * (i as f32 / n as f32)).collect();
    for f in extra_freqs {
        if f.is_finite() && f > 0.0 {
            let x = freq_x_at(f, gx, gw);
            if x >= gx && x <= gx + gw {
                xs.push(x);
            }
        }
    }
    xs.sort_by(|a, b| a.total_cmp(b));
    xs.dedup_by(|a, b| (*a - *b).abs() < 0.15);
    xs
}
fn eq_db_on_xs(coeff: &BandCoeffs, xs: &[f32], gx: f32, gw: f32, sr: f64, eq_sr: f64) -> Vec<f64> {
    let nyq = sr * 0.49;
    xs.iter()
        .map(|&x| coeff.response(x_freq_at(x, gx, gw).min(nyq), eq_sr))
        .collect()
}
fn eq_points(xs: &[f32], dbs: &[f64], graph_db: f64) -> Vec<(f32, f32)> {
    xs.iter()
        .zip(dbs)
        .map(|(&x, &db)| (x, db_y(db, graph_db)))
        .collect()
}
const GR_METER_DB: f32 = 30.0;
fn gr_meter_bar_heights(actual: f32, uncapped: f32, meter_h: f32) -> (f32, f32) {
    let actual_h = meter_h * (actual.max(0.0) / GR_METER_DB).clamp(0.0, 1.0);
    let uncapped_h = meter_h * (uncapped.max(actual).max(0.0) / GR_METER_DB).clamp(0.0, 1.0);
    (actual_h, uncapped_h)
}
fn knee_pulse_bulge(bulge: &Cell<f32>, knee_hover: bool) -> f32 {
    let target_b = if knee_hover { 1.0 } else { 0.0 };
    let cur_b = bulge.get();
    let next_b = cur_b + (target_b - cur_b) * 0.15;
    bulge.set(next_b);
    let now_sec = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        % 100_000) as f32
        * 0.001;
    let pulse = 0.96 + 0.04 * (now_sec * 2.5).sin();
    next_b * 1.5 * pulse
}
fn draw_knee_zone(
    d: &mut Draw,
    knee_rect: (f32, f32, f32, f32),
    bulge_w: f32,
    knee_norm: f32,
    knee_hover: bool,
    base_color: C,
    bypassed: bool,
) {
    let base_alpha = 0.75 - knee_norm * (0.75 - 0.05);
    let draw_alpha = if knee_hover {
        (base_alpha + 0.18).min(0.95)
    } else {
        base_alpha
    };
    let knee_color = C {
        r: base_color.r,
        g: base_color.g,
        b: base_color.b,
        a: draw_alpha,
    };
    let draw_knee = (
        knee_rect.0 - bulge_w,
        knee_rect.1,
        knee_rect.2 + 2.0 * bulge_w,
        knee_rect.3,
    );
    d.rect(
        draw_knee.0,
        draw_knee.1,
        draw_knee.2,
        draw_knee.3,
        knee_color,
    );
    if knee_hover {
        let highlight = if bypassed { MUTED } else { TEXT };
        d.outline(
            draw_knee,
            C {
                r: highlight.r,
                g: highlight.g,
                b: highlight.b,
                a: 0.7,
            },
        );
    }
}
fn draw_gr_depth_handle(d: &mut Draw, gx_m: f32, gw_m: f32, depth_y: f32, color: C) {
    let tw = 8.0;
    let th = 8.0;
    let y_bot = depth_y + 3.0;
    let y_top = y_bot - th;

    // Left right-triangle (vertical outer edge, horizontal bottom, hypotenuse facing inward):
    d.fill_poly(
        &[
            (gx_m - tw * 0.5, y_top),
            (gx_m - tw * 0.5, y_bot),
            (gx_m + tw * 0.5, y_bot),
        ],
        color,
    );

    // Right right-triangle (vertical outer edge, horizontal bottom, hypotenuse facing inward):
    d.fill_poly(
        &[
            (gx_m + gw_m + tw * 0.5, y_top),
            (gx_m + gw_m + tw * 0.5, y_bot),
            (gx_m + gw_m - tw * 0.5, y_bot),
        ],
        color,
    );

    // Thin 3px line connecting them at the bottom of the triangles:
    d.rect(gx_m - tw * 0.5, depth_y, gw_m + tw, 3.0, color);

    // Horizontal rule within the 3px area to suggest grabability:
    let grip = if color == TEXT { PANEL } else { BG };
    d.line(
        gx_m - 2.0,
        depth_y + 1.5,
        gx_m + gw_m + 2.0,
        depth_y + 1.5,
        grip,
        1.0,
    );
}
const DYN_PILL_R: f32 = 12.0;
const NODE_INNER_R: f32 = 7.5;
const NODE_HIT_R: f32 = 16.0;
fn overflow_reduction(range: f64, uncapped: f64) -> Option<f64> {
    if range >= 0.0 {
        (uncapped > range + 0.05).then_some(uncapped)
    } else {
        (uncapped < range - 0.05).then_some(uncapped)
    }
}
fn capped_reduction(range: f64, uncapped: f64) -> f64 {
    if range >= 0.0 {
        uncapped.clamp(0.0, range)
    } else {
        uncapped.clamp(range, 0.0)
    }
}
fn uncapped_for(id: u64, meters: &[(u64, f32)]) -> Option<f64> {
    meters
        .iter()
        .find(|(band_id, _)| *band_id == id)
        .map(|(_, gr)| f64::from(*gr))
}
fn hover_preview_label(has_selected: bool, shape: Shape) -> &'static str {
    if has_selected {
        "DESELECT"
    } else {
        shape.uppercase_name()
    }
}
fn hover_preview_text_y(circle_y: f32) -> f32 {
    circle_y + 4.0
}
fn module_header_mid() -> f32 {
    MODULE_Y + MODULE_HEADER_H * 0.5
}
fn module_header_ctrl_y() -> f32 {
    module_header_mid() - MODULE_HEADER_CTRL * 0.5
}
fn module_title_y(size: f32) -> f32 {
    module_header_mid() + size * 0.35
}
fn meter_value_y() -> f32 {
    EQ_AXIS_LABEL_Y
}

fn rect_center_x(r: (f32, f32, f32, f32)) -> f32 {
    r.0 + r.2 * 0.5
}

fn meter_value_rect(meter: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let cx = rect_center_x(meter);
    (cx - 25.0, meter_value_y() - 14.0, 50.0, 22.0)
}
fn stacked_knob_slots(module_x: f32, module_w: f32, count: usize) -> Vec<(f32, f32, f32, f32)> {
    let top = MODULE_Y + MODULE_HEADER_H + 14.0;
    let bottom = MODULE_Y + MODULE_H - 12.0;
    let gap = 8.0;
    let n = count.max(1) as f32;
    let h = (bottom - top - gap * (n - 1.0)) / n;
    let x = module_x + 8.0;
    let w = module_w - 16.0;
    (0..count)
        .map(|i| (x, top + i as f32 * (h + gap), w, h))
        .collect()
}
fn catch_gr_to_depth(gr: f32, depth: f32) -> f32 {
    gr.min(depth.max(0.0))
}
fn module_header_cog_rect(module_x: f32, module_w: f32) -> (f32, f32, f32, f32) {
    (
        module_x + module_w - 14.0 - 24.0,
        module_header_ctrl_y(),
        24.0,
        MODULE_HEADER_CTRL,
    )
}
fn draw_module_page_button(
    d: &mut Draw,
    rect: (f32, f32, f32, f32),
    controls: bool,
    bypassed: bool,
    hover: bool,
    accent: C,
) {
    let color = if bypassed {
        MUTED
    } else if hover {
        accent
    } else {
        TEXT
    };
    if controls {
        d.text_centered(
            rect.0 + rect.2 * 0.5,
            rect.1 + rect.3 * 0.5 + 5.0,
            "<",
            14.0,
            color,
        );
    } else {
        d.cog_icon(rect.0 + rect.2 * 0.5, rect.1 + rect.3 * 0.5, color);
    }
}
fn clamp_knee_to_meter(x: f32, y: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
    let min_y = METER_TOP - KNEE_METER_OVERHANG;
    let max_bottom = METER_TOP + METER_H + KNEE_METER_OVERHANG;
    let top = y.max(min_y);
    let bottom = (y + h).min(max_bottom);
    (x, top, w, (bottom - top).max(0.0))
}
fn axis_label_y(line_y: f32, size: f32) -> f32 {
    line_y + size * 0.35
}
fn axis_db_text(db: i32) -> String {
    format!("{db}")
}
fn draw_hover_preview(d: &mut Draw, x: f32, y: f32, gx: f32, gw: f32, label: &str) {
    d.circle(x, y, 5.0, MUTED, false);
    let label_w = label.len() as f32 * 12.0 * 0.6;
    let text_x = if x + 12.0 + label_w > gx + gw - 8.0 {
        (x - 12.0 - label_w).max(gx + 8.0)
    } else {
        x + 12.0
    };
    d.text(text_x, hover_preview_text_y(y), label, 12.0, MUTED);
}
fn draw_eq_preview_influence(
    d: &mut Draw,
    shape: Shape,
    freq: f64,
    q: f64,
    points: &[(f32, f32)],
    graph_db: f64,
    graph: (f32, f32),
) {
    let (gx, gw) = graph;
    let baseline_y = db_y(0.0, graph_db);
    let fill_a = 0.08;
    let stroke_a = 0.40;
    let color = MUTED;
    let c_trans = C {
        r: color.r,
        g: color.g,
        b: color.b,
        a: 0.0,
    };
    let c_solid = C {
        r: color.r,
        g: color.g,
        b: color.b,
        a: fill_a,
    };
    let s_trans = C {
        r: color.r,
        g: color.g,
        b: color.b,
        a: 0.0,
    };
    let s_solid = C {
        r: color.r,
        g: color.g,
        b: color.b,
        a: stroke_a,
    };
    match shape {
        Shape::Bell | Shape::BandPass | Shape::Notch => {
            let oct_span = (2.0 / q.clamp(0.15, 18.0)).clamp(0.5, 4.0);
            let f_left = (freq * 2.0_f64.powf(-oct_span)).max(20.0);
            let f_right = (freq * 2.0_f64.powf(oct_span)).min(20000.0);
            let x_left = freq_x_at(f_left, gx, gw);
            let x_center = freq_x_at(freq, gx, gw);
            let x_right = freq_x_at(f_right, gx, gw);
            let center_idx = points
                .iter()
                .position(|(x, _)| *x >= x_center)
                .unwrap_or(points.len() / 2);
            let left_pts = &points[..=center_idx];
            let right_pts = &points[center_idx..];
            if fill_a > 0.0 {
                d.area_gradient_span(left_pts, baseline_y, x_left, x_center, c_trans, c_solid);
                d.area_gradient_span(right_pts, baseline_y, x_center, x_right, c_solid, c_trans);
            }
            d.poly_gradient_span(left_pts, x_left, x_center, s_trans, s_solid, 1.4);
            d.poly_gradient_span(right_pts, x_center, x_right, s_solid, s_trans, 1.4);
        }
        Shape::HighShelf | Shape::HighCut => {
            let oct_span = (1.5 / q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
            let f_fade_start = (freq * 2.0_f64.powf(-oct_span)).max(20.0);
            let f_fade_end = (freq * 2.0_f64.powf(oct_span * 0.5)).min(20000.0);
            let x_start = freq_x_at(f_fade_start, gx, gw);
            let x_end = freq_x_at(f_fade_end, gx, gw);
            if fill_a > 0.0 {
                d.area_gradient_span(points, baseline_y, x_start, x_end, c_trans, c_solid);
            }
            d.poly_gradient_span(points, x_start, x_end, s_trans, s_solid, 1.4);
        }
        Shape::LowShelf | Shape::LowCut => {
            let oct_span = (1.5 / q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
            let f_fade_start = (freq * 2.0_f64.powf(-oct_span * 0.5)).max(20.0);
            let f_fade_end = (freq * 2.0_f64.powf(oct_span)).min(20000.0);
            let x_start = freq_x_at(f_fade_start, gx, gw);
            let x_end = freq_x_at(f_fade_end, gx, gw);
            if fill_a > 0.0 {
                d.area_gradient_span(points, baseline_y, x_start, x_end, c_solid, c_trans);
            }
            d.poly_gradient_span(points, x_start, x_end, s_solid, s_trans, 1.4);
        }
    }
}
fn solo_shade_rects(
    shape: Shape,
    freq: f64,
    q: f64,
    gx: f32,
    gw: f32,
) -> (Option<(f32, f32)>, Option<(f32, f32)>) {
    let graph_l = gx;
    let graph_r = gx + gw;
    let left_rect = |x_edge: f32| {
        let w = x_edge - graph_l;
        (w > 1.0).then_some((graph_l, w))
    };
    let right_rect = |x_edge: f32| {
        let w = graph_r - x_edge;
        (w > 1.0).then_some((x_edge, w))
    };
    match shape {
        Shape::LowCut => {
            let oct = (1.5 / q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
            let f_edge = (freq * 2.0_f64.powf(-oct * 0.5)).max(20.0);
            (left_rect(freq_x_at(f_edge, gx, gw)), None)
        }
        Shape::HighCut => {
            let oct = (1.5 / q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
            let f_edge = (freq * 2.0_f64.powf(oct * 0.5)).min(20000.0);
            (None, right_rect(freq_x_at(f_edge, gx, gw)))
        }
        _ => {
            let oct_span = (2.0 / q.clamp(0.15, 18.0)).clamp(0.5, 4.0);
            let x_left = freq_x_at((freq * 2.0_f64.powf(-oct_span)).max(20.0), gx, gw);
            let x_right = freq_x_at((freq * 2.0_f64.powf(oct_span)).min(20000.0), gx, gw);
            (left_rect(x_left), right_rect(x_right))
        }
    }
}
fn greyscale_darken(color: C, amount: f32) -> C {
    let y = (0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b) * (1.0 - amount);
    C {
        r: y,
        g: y,
        b: y,
        a: color.a,
    }
}
fn intersect_rect(
    a: (f32, f32, f32, f32),
    b: (f32, f32, f32, f32),
) -> Option<(f32, f32, f32, f32)> {
    let x0 = a.0.max(b.0);
    let y0 = a.1.max(b.1);
    let x1 = (a.0 + a.2).min(b.0 + b.2);
    let y1 = (a.1 + a.3).min(b.1 + b.3);
    (x1 - x0 > 0.5 && y1 - y0 > 0.5).then_some((x0, y0, x1 - x0, y1 - y0))
}
fn draw_solo_shade(
    d: &mut Draw,
    shape: Shape,
    freq: f64,
    q: f64,
    gx: f32,
    gw: f32,
    clip: (f32, f32, f32, f32),
    sum: &[(f32, f32)],
    sum_color: C,
    sum_width: f32,
    fill: Option<(&[(f32, f32)], f32, C)>,
) {
    let overlay = C::rgba(12, 12, 14, 178);
    let grey_sum = greyscale_darken(sum_color, 0.32);
    let (left, right) = solo_shade_rects(shape, freq, q, gx, gw);
    for (x, w) in [left, right].into_iter().flatten() {
        d.rounded_rect(x, GY, w, GH, 0.0, overlay);
        let Some((ix, iy, iw, ih)) = intersect_rect((x + d.offset_x, GY, w, GH), clip) else {
            continue;
        };
        d.scissor(ix, iy, iw, ih);
        if let Some((pts, baseline, fill_c)) = fill {
            d.area(pts, baseline, greyscale_darken(fill_c, 0.40));
        }
        d.poly(sum, grey_sum, sum_width);
        d.scissor(clip.0, clip.1, clip.2, clip.3);
    }
}
fn draw_eq_hover_preview(
    d: &mut Draw,
    x: f32,
    y: f32,
    gx: f32,
    gw: f32,
    graph_db: f64,
    sr: f64,
    eq_sr: f64,
    selected: Option<u64>,
    shape: Shape,
) {
    if selected.is_none() {
        let f = x_freq_at(x, gx, gw).clamp(20.0, sr * 0.49);
        let gain = if shape.has_gain() {
            y_db(y, graph_db)
        } else {
            0.0
        };
        let q = if matches!(shape, Shape::LowCut | Shape::HighCut) {
            0.707
        } else {
            1.0
        };
        let preview_band = Band {
            id: 0,
            shape,
            freq: f,
            gain,
            q,
            order: 2,
            ..Band::default()
        };
        let coeff = BandCoeffs::make(&preview_band, eq_sr);
        let xs = eq_curve_xs(gx, gw, [preview_band.freq]);
        let dbs = eq_db_on_xs(&coeff, &xs, gx, gw, sr, eq_sr);
        let points = eq_points(&xs, &dbs, graph_db);
        draw_eq_preview_influence(
            d,
            shape,
            preview_band.freq,
            preview_band.q,
            &points,
            graph_db,
            (gx, gw),
        );
    }
    draw_hover_preview(
        d,
        x,
        y,
        gx,
        gw,
        hover_preview_label(selected.is_some(), shape),
    );
}
fn range_handle_hit(
    b: &Band,
    x: f32,
    y: f32,
    graph_db: f64,
    gx: f32,
    gw: f32,
    selected: Option<u64>,
) -> bool {
    if !b.shape.has_gain() {
        return false;
    }
    let x_node = freq_x_at(b.freq, gx, gw);
    let node_y = db_y(b.gain, graph_db);
    let dist_node = (x - x_node).hypot(y - node_y);
    if dist_node <= NODE_INNER_R {
        return false;
    }
    if b.dynamic {
        let y_range = db_y(b.gain - b.range, graph_db);
        if (x - x_node).hypot(y - y_range) <= DYN_PILL_R {
            return true;
        }
        let top = node_y.min(y_range);
        let bot = node_y.max(y_range);
        if (x - x_node).abs() <= DYN_PILL_R && y >= top && y <= bot {
            return true;
        }
        return dist_node <= NODE_HIT_R;
    }
    Some(b.id) == selected && dist_node <= NODE_HIT_R
}
fn near_eq_node(b: &Band, x: f32, y: f32, graph_db: f64, gx: f32, gw: f32) -> bool {
    let nx = freq_x_at(b.freq, gx, gw);
    let ny = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, graph_db);
    if (x - nx).abs() < 15.0 && (y - ny).abs() < 15.0 {
        return true;
    }
    if b.dynamic && b.shape.has_gain() {
        let y_range = db_y(b.gain - b.range, graph_db);
        if (x - nx).abs() < 15.0 && (y - y_range).abs() < 15.0 {
            return true;
        }
        let top = ny.min(y_range);
        let bot = ny.max(y_range);
        if (x - nx).abs() <= DYN_PILL_R && y >= top - 4.0 && y <= bot + 4.0 {
            return true;
        }
    }
    false
}
fn snap_dyn_range(gain: f64, y: f32, graph_db: f64) -> f64 {
    let mut new_range = gain - y_db(y, graph_db);
    if new_range.abs() < 0.25 {
        new_range = 0.0;
    }
    new_range.clamp(-24.0, 24.0)
}
fn draw_dyn_range_stem(
    d: &mut Draw,
    b: &Band,
    graph_db: f64,
    graph: (f32, f32),
    color: C,
    uncapped: Option<f64>,
) {
    let (gx, gw) = graph;
    let x = freq_x_at(b.freq, gx, gw);
    let y = db_y(b.gain, graph_db);
    let y_range = db_y(b.gain - b.range, graph_db);
    d.pill(x, y, y_range, DYN_PILL_R, color);
    if let Some(uncapped) = uncapped {
        let live = capped_reduction(b.range, uncapped);
        if live.abs() > 0.05 {
            let y_live = db_y(b.gain - live, graph_db);
            let mut fill = color;
            fill.a = 0.45;
            d.pill_fill(x, y, y_live, DYN_PILL_R - 1.0, fill);
        }
        if let Some(over) = overflow_reduction(b.range, uncapped) {
            let y_over = db_y(b.gain - over, graph_db);
            if (y_over - y_range).abs() > 0.5 {
                d.line(x, y_range, x, y_over, MUTED, 3.0);
                d.circle(x, y_over, 3.0, MUTED, true);
            }
        }
    }
}
fn lift_gain_y(gain: f64) -> f32 {
    let norm = ((gain + 100.0) / 100.0).clamp(0.0, 1.0) as f32;
    GY + GH * (1.0 - norm)
}
fn lift_y_gain(y: f32) -> f64 {
    let norm = (1.0 - (y - GY) / GH).clamp(0.0, 1.0) as f64;
    -100.0 + norm * 100.0
}
fn inside(x: f32, y: f32, r: (f32, f32, f32, f32)) -> bool {
    x >= r.0 && x <= r.0 + r.2 && y >= r.1 && y <= r.1 + r.3
}
const HUD_PAD: f32 = 8.0;
const HUD_BTN: f32 = 22.0;
const HUD_BTN_GAP: f32 = 3.0;
const HUD_COL_GAP: f32 = 6.0;
const HUD_OVERLAY_GAP: f32 = 10.0;
const HUD_DYN_R: f32 = 13.0;
const HUD_FREQ_W: f32 = 86.0;
const HUD_GAIN_W: f32 = 52.0;
const HUD_Q_W: f32 = 40.0;
const HUD_SHAPE_W: f32 = 104.0;
const HUD_MAIN_W: f32 =
    HUD_PAD + HUD_FREQ_W + HUD_COL_GAP + HUD_GAIN_W + HUD_COL_GAP + HUD_Q_W + HUD_PAD;
const HUD_DYN_W: f32 = 400.0;
const HUD_H: f32 = 76.0;
const HUD_CUT_H: f32 = 96.0;
const HUD_DYN_FIELD_H: f32 = 28.0;
const HUD_DYN_ROW_GAP: f32 = 4.0;
const HUD_DYN_VALUE_W: f32 = 56.0;
const HUD_RADIUS: f32 = 6.0;
const LIFT_DOCK_H: f32 = 60.0;
#[derive(Clone, Copy)]
struct HudGeom {
    bx: f32,
    by: f32,
    bw: f32,
    bh: f32,
    dyn_x: f32,
    dyn_w: f32,
    show_dyn: bool,
    show_dyn_btn: bool,
}
fn band_allows_dyn(b: &Band) -> bool {
    b.shape.has_gain() && b.id < SC_EQ_ID_BASE
}
fn hud_height_for(b: &Band) -> f32 {
    if b.shape.is_cut() {
        HUD_CUT_H
    } else {
        HUD_H
    }
}
fn hud_height_for_lift(b: &LiftBand) -> f32 {
    if b.shape.is_cut() {
        HUD_CUT_H
    } else {
        HUD_H
    }
}
fn hud_place(
    node_x: f32,
    node_y: f32,
    bw: f32,
    bh: f32,
    trail: f32,
    gx: f32,
    gw: f32,
    bottom_inset: f32,
) -> (f32, f32) {
    let total = bw + trail;
    let max_x = (gx + gw - total - 8.0).max(gx + 8.0);
    let bx = (node_x - total * 0.5).clamp(gx + 8.0, max_x);
    let by = if node_y < GY + GH * 0.5 {
        (GY + GH - bottom_inset - bh - 4.0).max(GY + 4.0)
    } else {
        GY + 4.0
    };
    (bx, by)
}
fn hud_trail(show_dyn: bool, show_dyn_btn: bool) -> f32 {
    if show_dyn {
        HUD_OVERLAY_GAP + HUD_DYN_W
    } else if show_dyn_btn {
        HUD_OVERLAY_GAP * 0.5 + HUD_DYN_R
    } else {
        0.0
    }
}
fn hud_geom_for_at(b: &Band, range: f64, gx: f32, gw: f32) -> HudGeom {
    let show_dyn_btn = band_allows_dyn(b);
    let show_dyn = show_dyn_btn && b.dynamic;
    let bw = HUD_MAIN_W;
    let bh = hud_height_for(b);
    let node_x = freq_x_at(b.freq, gx, gw);
    let node_y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, range);
    let (bx, by) = hud_place(
        node_x,
        node_y,
        bw,
        bh,
        hud_trail(show_dyn, show_dyn_btn),
        gx,
        gw,
        0.0,
    );
    HudGeom {
        bx,
        by,
        bw,
        bh,
        dyn_x: bx + bw + HUD_OVERLAY_GAP,
        dyn_w: if show_dyn { HUD_DYN_W } else { 0.0 },
        show_dyn,
        show_dyn_btn,
    }
}
fn hud_rect_for_at(b: &Band, range: f64, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let g = hud_geom_for_at(b, range, gx, gw);
    (g.bx, g.by, g.bw, g.bh)
}
#[cfg(test)]
fn hud_dyn_rect_for_at(b: &Band, range: f64, gx: f32, gw: f32) -> Option<(f32, f32, f32, f32)> {
    let g = hud_geom_for_at(b, range, gx, gw);
    g.show_dyn.then_some((g.dyn_x, g.by, g.dyn_w, g.bh))
}
fn hud_bounds_for_at(b: &Band, range: f64, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let g = hud_geom_for_at(b, range, gx, gw);
    (
        g.bx,
        g.by,
        g.bw + hud_trail(g.show_dyn, g.show_dyn_btn),
        g.bh,
    )
}
fn hud_dyn_btn_center(g: HudGeom) -> (f32, f32) {
    (g.bx + g.bw + HUD_OVERLAY_GAP * 0.5, g.by + g.bh * 0.5)
}
fn hud_dyn_btn_hit(g: HudGeom, x: f32, y: f32) -> bool {
    if !g.show_dyn_btn {
        return false;
    }
    let (cx, cy) = hud_dyn_btn_center(g);
    (x - cx).hypot(y - cy) <= HUD_DYN_R
}
fn hud_btn_rect(bx: f32, by: f32, i: usize) -> (f32, f32, f32, f32) {
    (
        bx + HUD_PAD + i as f32 * (HUD_BTN + HUD_BTN_GAP),
        by + HUD_PAD,
        HUD_BTN,
        HUD_BTN,
    )
}
fn hud_bypass_rect(bx: f32, by: f32) -> (f32, f32, f32, f32) {
    hud_btn_rect(bx, by, 0)
}
fn hud_solo_rect(bx: f32, by: f32) -> (f32, f32, f32, f32) {
    hud_btn_rect(bx, by, 1)
}
fn hud_close_rect(bx: f32, by: f32) -> (f32, f32, f32, f32) {
    hud_btn_rect(bx, by, 2)
}
fn hud_shape_rect(bx: f32, by: f32, bw: f32) -> (f32, f32, f32, f32) {
    let x = bx + HUD_PAD + 3.0 * HUD_BTN + 2.0 * HUD_BTN_GAP + HUD_COL_GAP;
    let avail = (bx + bw - HUD_PAD - x).max(0.0);
    (x, by + HUD_PAD, avail.min(HUD_SHAPE_W), HUD_BTN)
}
fn hud_order_rect(bx: f32, by: f32) -> (f32, f32, f32, f32) {
    (bx + HUD_PAD, by + HUD_H - 4.0, 108.0, 18.0)
}
fn hud_value_row_y(by: f32) -> f32 {
    by + HUD_PAD + HUD_DYN_FIELD_H + HUD_DYN_ROW_GAP
}
fn hud_value_rect_at(b: &Band, range: f64, i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let g = hud_geom_for_at(b, range, gx, gw);
    let y = hud_value_row_y(g.by);
    match i {
        0 => (g.bx + HUD_PAD, y, HUD_FREQ_W, HUD_DYN_FIELD_H),
        1 => (
            g.bx + HUD_PAD + HUD_FREQ_W + HUD_COL_GAP,
            y,
            HUD_GAIN_W,
            HUD_DYN_FIELD_H,
        ),
        2 => (
            g.bx + HUD_PAD + HUD_FREQ_W + HUD_COL_GAP + HUD_GAIN_W + HUD_COL_GAP,
            y,
            HUD_Q_W,
            HUD_DYN_FIELD_H,
        ),
        other => hud_dyn_value_rect_at(b, range, if other == 3 { 0 } else { 1 }, gx, gw),
    }
}
fn hud_dyn_field_rect_at(b: &Band, range: f64, i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let g = hud_geom_for_at(b, range, gx, gw);
    let cols = 3.0;
    let col_w = (g.dyn_w - HUD_PAD * 2.0 - HUD_COL_GAP * (cols - 1.0)) / cols;
    let (col, row) = match i {
        0 => (0.0, 0.0),
        1 => (1.0, 0.0),
        2 => (2.0, 0.0),
        3 => (0.0, 1.0),
        _ => (1.0, 1.0),
    };
    let x = g.dyn_x + HUD_PAD + col * (col_w + HUD_COL_GAP);
    let y = g.by + HUD_PAD + row * (HUD_DYN_FIELD_H + HUD_DYN_ROW_GAP);
    (x, y, col_w, HUD_DYN_FIELD_H)
}
fn hud_dyn_value_rect_at(b: &Band, range: f64, i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let r = hud_dyn_field_rect_at(b, range, i, gx, gw);
    (
        r.0 + r.2 - HUD_DYN_VALUE_W - 6.0,
        r.1 + 2.0,
        HUD_DYN_VALUE_W,
        16.0,
    )
}
fn hud_rect_for_lift_at(b: &LiftBand, _range: f64, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let bw = HUD_MAIN_W;
    let bh = hud_height_for_lift(b);
    let node_x = freq_x_at(b.freq, gx, gw);
    let node_y = lift_gain_y(b.gain);
    let (bx, by) = hud_place(node_x, node_y, bw, bh, 0.0, gx, gw, LIFT_DOCK_H);
    (bx, by, bw, bh)
}
fn hud_value_rect_lift_at(
    b: &LiftBand,
    range: f64,
    i: usize,
    gx: f32,
    gw: f32,
) -> (f32, f32, f32, f32) {
    let (x, y, _, _) = hud_rect_for_lift_at(b, range, gx, gw);
    let row_y = hud_value_row_y(y);
    match i {
        0 => (x + HUD_PAD, row_y, HUD_FREQ_W, HUD_DYN_FIELD_H),
        1 => (
            x + HUD_PAD + HUD_FREQ_W + HUD_COL_GAP,
            row_y,
            HUD_GAIN_W,
            HUD_DYN_FIELD_H,
        ),
        _ => (
            x + HUD_PAD + HUD_FREQ_W + HUD_COL_GAP + HUD_GAIN_W + HUD_COL_GAP,
            row_y,
            HUD_Q_W,
            HUD_DYN_FIELD_H,
        ),
    }
}
fn hud_chrome_hit(x: f32, y: f32, bx: f32, by: f32, bw: f32, is_cut: bool) -> Option<HudChrome> {
    if inside(x, y, hud_bypass_rect(bx, by)) {
        Some(HudChrome::Bypass)
    } else if inside(x, y, hud_solo_rect(bx, by)) {
        Some(HudChrome::Solo)
    } else if inside(x, y, hud_close_rect(bx, by)) {
        Some(HudChrome::Close)
    } else if inside(x, y, hud_shape_rect(bx, by, bw)) {
        Some(HudChrome::Shape)
    } else if is_cut && inside(x, y, hud_order_rect(bx, by)) {
        Some(HudChrome::Order)
    } else {
        None
    }
}
#[derive(Clone, Copy)]
enum HudChrome {
    Bypass,
    Solo,
    Close,
    Shape,
    Order,
}
fn hud_menu_rect(anchor: (f32, f32, f32, f32), h: f32) -> (f32, f32, f32, f32) {
    let below = anchor.1 + anchor.3 + 2.0;
    let y = if below + h <= GY + GH {
        below
    } else {
        (anchor.1 - h - 2.0).max(GY)
    };
    (anchor.0, y, anchor.2.max(160.0), h)
}
fn plot_band(b: &Band, meters: &[(u64, f32)]) -> Band {
    let mut plot = b.clone();
    if plot.dynamic && plot.shape.has_gain() {
        if let Some(uncapped) = uncapped_for(plot.id, meters) {
            plot.gain -= capped_reduction(plot.range, uncapped);
        }
    }
    plot
}
fn band_rect_at(i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    band_slot_rect_at(i, gx, gw, 5)
}
fn band_slot_rect_at(i: usize, gx: f32, gw: f32, count: usize) -> (f32, f32, f32, f32) {
    let slot = (gw - 46.0) / count as f32;
    (
        gx + 38.0 + i as f32 * slot,
        GRAPH_BOTTOM - LIFT_DOCK_H,
        (slot - 4.0).max(80.0),
        52.0,
    )
}
fn band_bar_rect_at(i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let r = band_rect_at(i, gx, gw);
    (r.0 + 12.0, r.1 + 28.0, r.2 - 24.0, 16.0)
}
fn band_value_rect_at(i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let r = band_rect_at(i, gx, gw);
    (r.0 + r.2 - 68.0, r.1 + 4.0, 60.0, 20.0)
}

#[derive(Clone, Copy, PartialEq)]
enum Target {
    Global(usize),
    Band(usize),
    LiftBand(usize),
    Node(u64),
    Range(u64),
    CompKnee {
        from_top: bool,
    },
    PseKnee {
        from_top: bool,
    },
    SoloAudition {
        id: u64,
        restore: u64,
        grab_dx: f32,
        grab_dy: f32,
    },
}
#[derive(Clone, Copy, PartialEq)]
enum PendingCreate {
    Curve,
    Background,
}
const DRAG_START_THRESHOLD: f32 = 5.0;
#[derive(Clone, Copy)]
enum BandMenu {
    Shape,
    Order,
}
impl BandMenu {
    fn count(self) -> usize {
        match self {
            Self::Shape => Shape::ALL.len(),
            Self::Order => 8,
        }
    }
    fn rect_at(self, b: &Band, range: f64, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
        let h = self.count() as f32 * 24.0;
        let (bx, by, bw, _) = hud_rect_for_at(b, range, gx, gw);
        let anchor = match self {
            Self::Shape => hud_shape_rect(bx, by, bw),
            Self::Order => hud_order_rect(bx, by),
        };
        hud_menu_rect(anchor, h)
    }
    fn rect_lift_at(self, b: &LiftBand, range: f64, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
        let h = self.count() as f32 * 24.0;
        let (bx, by, bw, _) = hud_rect_for_lift_at(b, range, gx, gw);
        let anchor = match self {
            Self::Shape => hud_shape_rect(bx, by, bw),
            Self::Order => hud_order_rect(bx, by),
        };
        hud_menu_rect(anchor, h)
    }
}
pub fn create(params: Arc<StripParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            nih_plug_vizia::assets::register_noto_sans_light(cx);
            StripView {
                params: params.clone(),
                shared: shared.clone(),
                selected: None,
                dyn_page: DynPage::Main,
                pse_page: PsePage::Main,
                wall_page: WallPage::Main,
                drag: None,
                hover: None,
                font: Cell::new(None),
                signature: Cell::new(None),
                graph_db: display_range(*params.graph_range.lock().unwrap()),
                scale_menu: false,
                processing_menu: None,
                edit: None,
                down: (0.0, 0.0),
                last_drag: (0.0, 0.0),
                pending_create: None,
                menu: None,
                active_eq: Cell::new(0),
                anim_progress: Cell::new(0.0),
                anim_target: Cell::new(0.0),
                anim_start: Cell::new(0.0),
                anim_time: Cell::new(1.0),
                last_tick: Cell::new(None),
                knee_bulge: Cell::new(0.0),
                pse_knee_bulge: Cell::new(0.0),
                dyn_anim_progress: Cell::new(0.0),
                dyn_anim_target: Cell::new(0.0),
                pse_anim_progress: Cell::new(0.0),
                pse_anim_target: Cell::new(0.0),
                wall_anim_progress: Cell::new(0.0),
                wall_anim_target: Cell::new(0.0),
                pending_solo: None,
                eq_bypass_anim: ButtonAnim::new(),
                pse_bypass_anim: ButtonAnim::new(),
                dyn_bypass_anim: ButtonAnim::new(),
                wall_bypass_anim: ButtonAnim::new(),
                lift_badge_anim: ButtonAnim::new(),
                dyn_band_anim: ButtonAnim::new(),
            }
            .build(cx, |cx| {
                let timer = cx.add_timer(Duration::from_millis(16), None, |cx, action| {
                    if let TimerAction::Tick(_) = action {
                        cx.needs_redraw();
                    }
                });
                cx.start_timer(timer);
            })
            .width(Stretch(1.0))
            .height(Stretch(1.0))
            .focusable(true);
        },
    )
}
struct StripView {
    dyn_page: DynPage,
    pse_page: PsePage,
    wall_page: WallPage,
    params: Arc<StripParams>,
    shared: Arc<Shared>,
    selected: Option<u64>,
    drag: Option<Target>,
    hover: Option<(f32, f32)>,
    font: Cell<Option<FontId>>,
    signature: Cell<Option<FontId>>,
    graph_db: f64,
    scale_menu: bool,
    processing_menu: Option<bool>,
    edit: Option<ValueEdit<ValueTarget>>,
    down: (f32, f32),
    last_drag: (f32, f32),
    pending_create: Option<PendingCreate>,
    menu: Option<BandMenu>,
    active_eq: Cell<usize>,
    anim_progress: Cell<f32>,
    anim_target: Cell<f32>,
    anim_start: Cell<f32>,
    anim_time: Cell<f32>,
    last_tick: Cell<Option<std::time::Instant>>,
    knee_bulge: Cell<f32>,
    pse_knee_bulge: Cell<f32>,
    dyn_anim_progress: Cell<f32>,
    dyn_anim_target: Cell<f32>,
    pse_anim_progress: Cell<f32>,
    pse_anim_target: Cell<f32>,
    wall_anim_progress: Cell<f32>,
    wall_anim_target: Cell<f32>,
    pending_solo: Option<u64>,
    eq_bypass_anim: ButtonAnim,
    pse_bypass_anim: ButtonAnim,
    dyn_bypass_anim: ButtonAnim,
    wall_bypass_anim: ButtonAnim,
    lift_badge_anim: ButtonAnim,
    dyn_band_anim: ButtonAnim,
}
fn lift_bin_curve_fit(bins: &[f64; 256], t: f64) -> f64 {
    let u = (t * 256.0 - 0.5).clamp(0.0, 255.0);
    let k = (u.floor() as usize).min(254);
    let s = u - k as f64;
    let y0 = if k > 0 { bins[k - 1] } else { bins[0] };
    let y1 = bins[k];
    let y2 = bins[k + 1];
    let y3 = if k + 2 < 256 { bins[k + 2] } else { bins[255] };

    let a = -0.5 * y0 + 1.5 * y1 - 1.5 * y2 + 0.5 * y3;
    let b = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c = -0.5 * y0 + 0.5 * y2;
    let d = y1;

    (((a * s + b) * s + c) * s + d).max(0.0)
}
fn smooth_lift_bins(raw: &[f64; 256]) -> [f64; 256] {
    let mut smooth = [0.0_f64; 256];
    smooth_bins(raw, &mut smooth);
    smooth
}
fn lift_curve_points(b: &LiftBand, gr: &[f64; 256], gx: f32, gw: f32, sr: f64) -> Vec<(f32, f32)> {
    let graph_bottom_y = GY + GH;
    (0..=500)
        .map(|i| {
            let x = gx + gw * (i as f32 / 500.0);
            let f = x_freq_at(x, gx, gw);
            let inf = filter_influence(b.shape, b.freq, b.q, b.order, f, sr);
            let t = ((f / 20.0).log10() / 3.0).clamp(0.0, 1.0);
            let gr_db = lift_bin_curve_fit(gr, t);
            let cut_gain = (b.gain - gr_db * 1.5).clamp(-100.0, 0.0);
            let norm = ((cut_gain + 100.0) / 100.0) as f32;
            let y = graph_bottom_y - (GH * norm * inf as f32);
            (x, y)
        })
        .collect()
}
fn draw_lift_influence(
    d: &mut Draw,
    b: &LiftBand,
    points: &[(f32, f32)],
    color: C,
    alphas: (f32, f32),
    graph: (f32, f32),
) {
    let (fill_a, stroke_a) = alphas;
    let (gx, gw) = graph;
    let graph_bottom_y = GY + GH;
    let c_trans = C {
        r: color.r,
        g: color.g,
        b: color.b,
        a: 0.0,
    };
    let c_solid = C {
        r: color.r,
        g: color.g,
        b: color.b,
        a: fill_a,
    };
    let s_trans = C {
        r: color.r,
        g: color.g,
        b: color.b,
        a: 0.0,
    };
    let s_solid = C {
        r: color.r,
        g: color.g,
        b: color.b,
        a: stroke_a,
    };
    match b.shape {
        Shape::Bell | Shape::BandPass | Shape::Notch => {
            let oct_span = (2.0 / b.q.clamp(0.15, 18.0)).clamp(0.5, 4.0);
            let f_left = (b.freq * 2.0_f64.powf(-oct_span)).max(20.0);
            let f_right = (b.freq * 2.0_f64.powf(oct_span)).min(20000.0);
            let x_left = freq_x_at(f_left, gx, gw);
            let x_center = freq_x_at(b.freq, gx, gw);
            let x_right = freq_x_at(f_right, gx, gw);
            let center_idx = points
                .iter()
                .position(|(x, _)| *x >= x_center)
                .unwrap_or(points.len() / 2);
            let left_pts = &points[..=center_idx];
            let right_pts = &points[center_idx..];
            if fill_a > 0.0 {
                d.area_gradient_span(left_pts, graph_bottom_y, x_left, x_center, c_trans, c_solid);
                d.area_gradient_span(
                    right_pts,
                    graph_bottom_y,
                    x_center,
                    x_right,
                    c_solid,
                    c_trans,
                );
            }
            d.poly_gradient_span(left_pts, x_left, x_center, s_trans, s_solid, 1.8);
            d.poly_gradient_span(right_pts, x_center, x_right, s_solid, s_trans, 1.8);
        }
        Shape::HighShelf | Shape::LowCut => {
            let oct_span = (1.5 / b.q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
            let f_fade_start = (b.freq * 2.0_f64.powf(-oct_span)).max(20.0);
            let f_fade_end = (b.freq * 2.0_f64.powf(oct_span * 0.5)).min(20000.0);
            let x_start = freq_x_at(f_fade_start, gx, gw);
            let x_end = freq_x_at(f_fade_end, gx, gw);
            if fill_a > 0.0 {
                d.area_gradient_span(points, graph_bottom_y, x_start, x_end, c_trans, c_solid);
            }
            d.poly_gradient_span(points, x_start, x_end, s_trans, s_solid, 1.8);
        }
        Shape::LowShelf | Shape::HighCut => {
            let oct_span = (1.5 / b.q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
            let f_fade_start = (b.freq * 2.0_f64.powf(-oct_span * 0.5)).max(20.0);
            let f_fade_end = (b.freq * 2.0_f64.powf(oct_span)).min(20000.0);
            let x_start = freq_x_at(f_fade_start, gx, gw);
            let x_end = freq_x_at(f_fade_end, gx, gw);
            if fill_a > 0.0 {
                d.area_gradient_span(points, graph_bottom_y, x_start, x_end, c_solid, c_trans);
            }
            d.poly_gradient_span(points, x_start, x_end, s_solid, s_trans, 1.8);
        }
    }
}
fn display_range(range: f64) -> f64 {
    if !range.is_finite() {
        return 24.0;
    }
    let mut closest = SCALES[0];
    let mut min_diff = (range - closest).abs();
    for &s in &SCALES[1..] {
        let diff = (range - s).abs();
        if diff < min_diff {
            min_diff = diff;
            closest = s;
        }
    }
    closest
}
fn format_pse_time(time_val: f64, peak: bool) -> String {
    let (attack, release) = crate::dsp::pse_time_to_times(time_val, peak);
    if peak {
        if release >= 10.0 {
            format!("{:.0} ms / {:.0} s", attack * 1000.0, release)
        } else if release >= 1.0 {
            if (release * 10.0).fract().abs() < 1e-3 {
                format!("{:.0} ms / {:.1} s", attack * 1000.0, release)
            } else {
                format!("{:.0} ms / {:.2} s", attack * 1000.0, release)
            }
        } else {
            format!("{:.0} ms / {:.0} ms", attack * 1000.0, release * 1000.0)
        }
    } else if release >= 10.0 {
        format!("{:.1} s", release)
    } else if release >= 1.0 {
        if (release * 10.0).fract().abs() < 1e-3 {
            format!("{:.1} s", release)
        } else {
            format!("{:.2} s", release)
        }
    } else {
        format!("{:.0} ms", release * 1000.0)
    }
}

impl StripView {
    fn is_pre(&self) -> bool {
        self.params.comp_pre.value()
    }
    fn idle_hover(&self) -> Option<(f32, f32)> {
        pleasant_ui::idle_hover(self.hover, self.drag.is_some())
    }
    fn eq_bounds(&self) -> (f32, f32, f32, f32) {
        let x = if self.is_pre() {
            MARGIN + PSE_W + GAP + DYN_W + GAP
        } else {
            MARGIN + PSE_W + GAP
        };
        (x, MODULE_Y, EQ_W, MODULE_H)
    }
    fn dyn_bounds(&self) -> (f32, f32, f32, f32) {
        let x = if self.is_pre() {
            MARGIN + PSE_W + GAP
        } else {
            UI_W - MARGIN - WALL_W - GAP - DYN_W
        };
        (x, MODULE_Y, DYN_W, MODULE_H)
    }
    fn pse_bounds(&self) -> (f32, f32, f32, f32) {
        (MARGIN, MODULE_Y, PSE_W, MODULE_H)
    }
    fn wall_bounds(&self) -> (f32, f32, f32, f32) {
        (UI_W - MARGIN - WALL_W, MODULE_Y, WALL_W, MODULE_H)
    }
    fn gx(&self) -> f32 {
        self.eq_bounds().0 + EQ_GRAPH_PAD_LEFT
    }
    fn gw(&self) -> f32 {
        self.eq_bounds().2 - EQ_GRAPH_PAD_LEFT - EQ_GRAPH_PAD_RIGHT
    }
    fn graph_area(&self) -> (f32, f32, f32, f32) {
        (self.gx(), GY, self.gw(), GH)
    }
    fn freq_x(&self, freq: f64) -> f32 {
        freq_x_at(freq, self.gx(), self.gw())
    }
    fn x_freq(&self, x: f32) -> f64 {
        x_freq_at(x, self.gx(), self.gw())
    }
    fn hud_bounds_for(&self, b: &Band) -> (f32, f32, f32, f32) {
        hud_bounds_for_at(b, self.graph_db, self.gx(), self.gw())
    }
    fn hud_rect_for_lift(&self, b: &LiftBand) -> (f32, f32, f32, f32) {
        hud_rect_for_lift_at(b, self.graph_db, self.gx(), self.gw())
    }
    fn hud_value_rect(&self, b: &Band, i: usize) -> (f32, f32, f32, f32) {
        hud_value_rect_at(b, self.graph_db, i, self.gx(), self.gw())
    }
    fn hud_value_rect_lift(&self, b: &LiftBand, i: usize) -> (f32, f32, f32, f32) {
        hud_value_rect_lift_at(b, self.graph_db, i, self.gx(), self.gw())
    }
    fn hud_dyn_field_rect(&self, b: &Band, i: usize) -> (f32, f32, f32, f32) {
        hud_dyn_field_rect_at(b, self.graph_db, i, self.gx(), self.gw())
    }
    fn hud_dyn_value_rect(&self, b: &Band, i: usize) -> (f32, f32, f32, f32) {
        hud_dyn_value_rect_at(b, self.graph_db, i, self.gx(), self.gw())
    }
    fn hud_geom(&self, b: &Band) -> HudGeom {
        hud_geom_for_at(b, self.graph_db, self.gx(), self.gw())
    }
    fn band_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        band_rect_at(i, self.gx(), self.gw())
    }
    fn band_bar_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        band_bar_rect_at(i, self.gx(), self.gw())
    }
    fn band_value_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        band_value_rect_at(i, self.gx(), self.gw())
    }
    fn dyn_gr_uncapped(&self) -> Vec<(u64, f32)> {
        self.shared
            .dyn_gr_uncapped
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }
    fn eq_tab_1_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (
            eq_x + 200.0,
            module_header_ctrl_y(),
            48.0,
            MODULE_HEADER_CTRL,
        )
    }
    fn eq_tab_2_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (
            eq_x + 252.0,
            module_header_ctrl_y(),
            48.0,
            MODULE_HEADER_CTRL,
        )
    }
    fn eq_tab_lift_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (
            eq_x + 304.0,
            module_header_ctrl_y(),
            48.0,
            MODULE_HEADER_CTRL,
        )
    }
    fn eq_tab_sc_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (
            eq_x + 356.0,
            module_header_ctrl_y(),
            56.0,
            MODULE_HEADER_CTRL,
        )
    }
    fn eq_power_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (
            eq_x + 14.0,
            module_header_ctrl_y(),
            24.0,
            MODULE_HEADER_CTRL,
        )
    }
    fn scale_button_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (eq_x, GY - 10.0, EQ_GRAPH_PAD_LEFT, 20.0)
    }
    fn eq_header_listen_sc_rect(&self) -> (f32, f32, f32, f32) {
        let eq_b = self.eq_bounds();
        (
            eq_b.0 + eq_b.2 - 14.0 - 90.0,
            module_header_ctrl_y(),
            90.0,
            MODULE_HEADER_CTRL,
        )
    }
    fn footer_link_rect(&self) -> (f32, f32, f32, f32) {
        let out = output_gain_rect();
        (out.0 - GAP - 56.0, FOOTER_BTN_Y, 56.0, 28.0)
    }
    fn footer_auto_rect(&self) -> (f32, f32, f32, f32) {
        let link = self.footer_link_rect();
        (link.0 - 8.0 - 56.0, FOOTER_BTN_Y, 56.0, 28.0)
    }
    fn footer_comp_routing_rect(&self) -> (f32, f32, f32, f32) {
        let auto = self.footer_auto_rect();
        (auto.0 - 8.0 - 56.0, FOOTER_BTN_Y, 56.0, 28.0)
    }
    fn footer_comp_label_x(&self) -> f32 {
        self.footer_comp_routing_rect().0 - 108.0
    }
    fn scale_menu_rect(&self) -> (f32, f32, f32, f32) {
        let btn = self.scale_button_rect();
        (btn.0, btn.1 + btn.3 + 2.0, 72.0, SCALES.len() as f32 * 24.0)
    }
    fn dyn_cog_button_rect(&self) -> (f32, f32, f32, f32) {
        let b = self.dyn_bounds();
        module_header_cog_rect(b.0, b.2)
    }
    fn pse_cog_button_rect(&self) -> (f32, f32, f32, f32) {
        let b = self.pse_bounds();
        module_header_cog_rect(b.0, b.2)
    }
    fn wall_cog_button_rect(&self) -> (f32, f32, f32, f32) {
        let b = self.wall_bounds();
        module_header_cog_rect(b.0, b.2)
    }
    fn wall_main_thresh_slider_rect(&self) -> (f32, f32, f32, f32) {
        let (wx, _, ww, _) = self.wall_bounds();
        let sw = 38.0;
        (wx + (ww - sw) * 0.5, METER_TOP, sw, METER_H)
    }
    fn wall_main_thresh_handle_rect(&self, thresh_y: f32) -> (f32, f32, f32, f32) {
        let (sx, _, sw, _) = self.wall_main_thresh_slider_rect();
        (sx - 5.0, thresh_y - 8.0, sw + 10.0, 16.0)
    }
    fn dyn_power_button_rect(&self) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        (dx + 14.0, module_header_ctrl_y(), 24.0, MODULE_HEADER_CTRL)
    }
    fn pse_power_button_rect(&self) -> (f32, f32, f32, f32) {
        let px = self.pse_bounds().0;
        (px + 14.0, module_header_ctrl_y(), 24.0, MODULE_HEADER_CTRL)
    }
    fn wall_power_button_rect(&self) -> (f32, f32, f32, f32) {
        let wx = self.wall_bounds().0;
        (wx + 14.0, module_header_ctrl_y(), 24.0, MODULE_HEADER_CTRL)
    }
    fn pse_detect_mode_rect(&self) -> (f32, f32, f32, f32) {
        let px = self.pse_bounds().0;
        (px + 68.0, 316.0, 56.0, 24.0)
    }
    fn dyn_main_thresh_slider_rect(&self) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        (dx + 30.0, METER_TOP, 38.0, METER_H)
    }
    fn dyn_main_gr_meter_rect(&self) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        (dx + 84.0, METER_TOP, 16.0, METER_H)
    }
    fn dyn_main_thresh_handle_rect(&self, thresh_y: f32) -> (f32, f32, f32, f32) {
        let (sx, _, sw, _) = self.dyn_main_thresh_slider_rect();
        (sx - 5.0, thresh_y - 8.0, sw + 10.0, 16.0)
    }
    fn dyn_main_depth_handle_rect(&self, depth_y: f32) -> (f32, f32, f32, f32) {
        let (gx, _, gw, _) = self.dyn_main_gr_meter_rect();
        (gx - 6.0, depth_y - 8.0, gw + 12.0, 16.0)
    }
    fn dyn_main_knee_offset(&self) -> f32 {
        let knee_val = self.params.comp_knee.value();
        10.0 + (knee_val / 20.0).clamp(0.0, 1.0) * 45.0
    }
    fn dyn_main_knee_rect(&self, thresh_y: f32, bulge_w: f32) -> (f32, f32, f32, f32) {
        let handle = self.dyn_main_thresh_handle_rect(thresh_y);
        let knee_offset = self.dyn_main_knee_offset();
        clamp_knee_to_meter(
            handle.0 - 2.0 - bulge_w,
            handle.1 - knee_offset,
            handle.2 + 4.0 + 2.0 * bulge_w,
            handle.3 + 2.0 * knee_offset,
        )
    }
    fn pse_main_thresh_slider_rect(&self) -> (f32, f32, f32, f32) {
        let px = self.pse_bounds().0;
        (px + 30.0, METER_TOP, 38.0, METER_H)
    }
    fn pse_main_gr_meter_rect(&self) -> (f32, f32, f32, f32) {
        let px = self.pse_bounds().0;
        (px + 84.0, METER_TOP, 16.0, METER_H)
    }
    fn pse_main_thresh_handle_rect(&self, thresh_y: f32) -> (f32, f32, f32, f32) {
        let (sx, _, sw, _) = self.pse_main_thresh_slider_rect();
        (sx - 5.0, thresh_y - 8.0, sw + 10.0, 16.0)
    }
    fn pse_main_depth_handle_rect(&self, depth_y: f32) -> (f32, f32, f32, f32) {
        let (gx, _, gw, _) = self.pse_main_gr_meter_rect();
        (gx - 6.0, depth_y - 8.0, gw + 12.0, 16.0)
    }
    fn pse_main_knee_offset(&self) -> f32 {
        let knee_val = self.params.pse_knee.value();
        10.0 + (knee_val / 18.0).clamp(0.0, 1.0) * 45.0
    }
    fn pse_main_knee_rect(&self, thresh_y: f32, bulge_w: f32) -> (f32, f32, f32, f32) {
        let handle = self.pse_main_thresh_handle_rect(thresh_y);
        let knee_offset = self.pse_main_knee_offset();
        clamp_knee_to_meter(
            handle.0 - 2.0 - bulge_w,
            handle.1 - knee_offset,
            handle.2 + 4.0 + 2.0 * bulge_w,
            handle.3 + 2.0 * knee_offset,
        )
    }
    fn pse_knob_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        let px = self.pse_bounds().0;
        match i {
            8 => (px + 6.0, 226.0, 56.0, 86.0),
            2 => (px + 6.0, 356.0, 56.0, 86.0),
            10 => (px + 68.0, 226.0, 56.0, 86.0),
            _ => (px + 6.0, 226.0, 56.0, 86.0),
        }
    }
    fn wall_knob_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        let wx = self.wall_bounds().0;
        let slots = stacked_knob_slots(wx, WALL_W, 2);
        match i {
            16 => slots[0],
            17 => slots[1],
            _ => slots[0],
        }
    }
    fn global_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        match self.dyn_page {
            DynPage::Controls => {
                let slots = stacked_knob_slots(dx, DYN_W, 5);
                match i {
                    5 => slots[0],
                    11 => slots[1],
                    6 => slots[2],
                    3 => slots[3],
                    4 => slots[4],
                    _ => (0.0, -1000.0, 0.0, 0.0),
                }
            }
            DynPage::Main => {
                if i == 14 {
                    meter_value_rect(self.dyn_main_gr_meter_rect())
                } else if i == 0 {
                    meter_value_rect(self.dyn_main_thresh_slider_rect())
                } else {
                    (dx + 6.0, 226.0, 56.0, 86.0)
                }
            }
        }
    }
    fn knob_value_rect(r: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        if r.2 <= 64.0 && r.3 <= 96.0 {
            (r.0 + 6.0, r.1 + 63.0, r.2 - 12.0, 19.0)
        } else {
            (r.0 + 8.0, r.1 + r.3 - 28.0, r.2 - 16.0, 22.0)
        }
    }
    fn global_value_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        if self.dyn_page == DynPage::Main && matches!(i, 0 | 14) {
            self.global_rect(i)
        } else {
            Self::knob_value_rect(self.global_rect(i))
        }
    }
    fn pse_value_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        if self.pse_page == PsePage::Main && i == 1 {
            meter_value_rect(self.pse_main_thresh_slider_rect())
        } else {
            Self::knob_value_rect(self.pse_knob_rect(i))
        }
    }
    fn wall_value_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        if self.wall_page == WallPage::Main && i == 18 {
            meter_value_rect(self.wall_main_thresh_slider_rect())
        } else {
            Self::knob_value_rect(self.wall_knob_rect(i))
        }
    }
    fn pse_controls(&self) -> &'static [usize] {
        match self.pse_page {
            PsePage::Main => &[1],
            PsePage::Controls => &PSE_CONTROLS_KNOBS,
        }
    }
    fn wall_controls(&self) -> &'static [usize] {
        match self.wall_page {
            WallPage::Main => &WALL_MAIN_KNOBS,
            WallPage::Controls => &WALL_CONTROLS_KNOBS,
        }
    }
    fn global_hit_rects(&self) -> Vec<(usize, (f32, f32, f32, f32))> {
        let mut rects: Vec<_> = self
            .global_controls()
            .iter()
            .copied()
            .map(|i| (i, self.global_rect(i)))
            .collect();
        rects.extend(self.pse_controls().iter().copied().map(|i| {
            (
                i,
                if self.pse_page == PsePage::Main {
                    self.pse_value_rect(i)
                } else {
                    self.pse_knob_rect(i)
                },
            )
        }));
        if self.wall_page == WallPage::Controls {
            rects.extend(
                self.wall_controls()
                    .iter()
                    .copied()
                    .map(|i| (i, self.wall_knob_rect(i))),
            );
        }
        rects
    }
    fn global_value_hits(&self) -> Vec<(ValueTarget, (f32, f32, f32, f32))> {
        let mut fields: Vec<_> = self
            .global_controls()
            .iter()
            .copied()
            .map(|i| (ValueTarget::Global(i), self.global_value_rect(i)))
            .collect();
        fields.extend(
            self.pse_controls()
                .iter()
                .copied()
                .map(|i| (ValueTarget::Global(i), self.pse_value_rect(i))),
        );
        fields.extend(
            self.wall_controls()
                .iter()
                .copied()
                .map(|i| (ValueTarget::Global(i), self.wall_value_rect(i))),
        );
        fields.push((ValueTarget::Global(15), output_gain_value_rect()));
        fields
    }
    fn value_color(&self, target: ValueTarget) -> C {
        match target {
            ValueTarget::Global(1 | 7 | 8 | 9 | 10) => PSE_BLUE,
            ValueTarget::Global(2 | 3) => TEAL,
            ValueTarget::Global(16 | 17 | 18) => WALL_COLOR,
            ValueTarget::Global(_) => GOLD,
            ValueTarget::Band(_) => self
                .selected
                .map(|id| COLORS[(band_display_num(id) as usize - 1) % COLORS.len()])
                .unwrap_or(TEAL),
            ValueTarget::Lift(_) => LIFT_COLOR,
        }
    }
    fn set_graph_range(&mut self, range: f64) {
        self.graph_db = display_range(range);
        *self.params.graph_range.lock().unwrap() = self.graph_db;
    }
    fn value_at(&self, x: f32, y: f32) -> Option<(ValueTarget, (f32, f32, f32, f32))> {
        if let Some(b) = self.selected_lift() {
            for i in 0..3 {
                let active = match i {
                    1 => true,
                    2 => !(b.shape.is_cut() && b.order == 1),
                    _ => true,
                };
                let r = self.hud_value_rect_lift(&b, i);
                if active && inside(x, y, r) {
                    return Some((ValueTarget::Lift(i), r));
                }
            }
            for i in 0..5 {
                let r = self.band_value_rect(i);
                if inside(x, y, r) {
                    return Some((ValueTarget::Lift(i + 3), r));
                }
            }
        }
        if let Some(b) = self.selected.and_then(|id| self.find_band(id)) {
            for i in 0..3 {
                let active = match i {
                    1 => b.shape.has_gain(),
                    2 => !(b.shape.is_cut() && b.order == 1),
                    _ => true,
                };
                let r = self.hud_value_rect(&b, i);
                if active && inside(x, y, r) {
                    return Some((ValueTarget::Band(i), r));
                }
            }
            if b.shape.has_gain() && b.dynamic && band_allows_dyn(&b) {
                for i in 0..5 {
                    let r = self.hud_dyn_value_rect(&b, i);
                    if inside(x, y, r) {
                        let target = match i {
                            0 => 3,
                            1 => 7,
                            2 => 4,
                            3 => 5,
                            _ => 6,
                        };
                        return Some((ValueTarget::Band(target), r));
                    }
                }
            }
        }
        self.global_value_hits()
            .into_iter()
            .find(|(_, r)| inside(x, y, *r))
    }
    fn start_edit(
        &mut self,
        cx: &mut EventContext,
        target: ValueTarget,
        rect: (f32, f32, f32, f32),
    ) {
        cx.focus();
        let value = match target {
            ValueTarget::Global(i) => {
                let value = self.param(i).value() as f64;
                if i == 0 || i == 18 {
                    -value
                } else {
                    value
                }
            }
            ValueTarget::Band(i) => {
                let Some(b) = self.selected.and_then(|id| self.find_band(id)) else {
                    return;
                };
                [
                    b.freq,
                    b.gain,
                    b.q,
                    b.threshold,
                    b.ratio,
                    b.attack,
                    b.release,
                    b.range,
                ][i]
            }
            ValueTarget::Lift(i) => {
                let lift_bands = self.params.lift_bands.lock().unwrap();
                let Some(b) = lift_bands.iter().find(|b| Some(b.id) == self.selected) else {
                    return;
                };
                [
                    b.freq,
                    b.gain,
                    b.q,
                    b.threshold,
                    b.ratio,
                    b.attack,
                    b.release,
                    b.range,
                ][i]
            }
        };
        let text = if let ValueTarget::Global(10) = target {
            format_pse_time(self.param(10).value() as f64, self.params.pse_peak.value())
        } else {
            format!("{value:.3}")
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        };
        self.edit = Some(ValueEdit::new(target, rect, text));
        self.menu = None;
        self.scale_menu = false;
    }
    fn commit_edit(&mut self, cx: &mut EventContext) -> bool {
        let Some(edit) = self.edit.as_mut() else {
            return true;
        };
        if edit.text == edit.original {
            self.edit = None;
            return true;
        }
        let Some(value) = parse_value(&edit.text, edit.target) else {
            edit.invalid = true;
            return false;
        };
        match edit.target {
            ValueTarget::Global(i) => {
                let p = self.param(i);
                let norm = p.preview_normalized(if i == 0 || i == 18 {
                    -value as f32
                } else {
                    value as f32
                });
                cx.emit(RawParamEvent::BeginSetParameter(p.as_ptr()));
                cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                cx.emit(RawParamEvent::EndSetParameter(p.as_ptr()));
            }
            ValueTarget::Band(i) => self.change(|b| match i {
                0 => b.freq = value,
                1 => b.gain = value,
                2 => b.q = value,
                3 => b.threshold = value,
                4 => b.ratio = value,
                5 => b.attack = value,
                6 => b.release = value,
                _ => b.range = value,
            }),
            ValueTarget::Lift(i) => self.change_lift(|b| match i {
                0 => b.freq = value.clamp(20.0, 20000.0),
                1 => b.gain = value.clamp(-100.0, 0.0),
                2 => b.q = value,
                3 => b.threshold = value,
                4 => b.ratio = value,
                5 => b.attack = value,
                6 => b.release = value,
                _ => b.range = value,
            }),
        }
        self.edit = None;
        true
    }
    fn edit_key(&mut self, cx: &mut EventContext, code: Code) {
        if code == Code::Escape {
            self.edit = None;
            return;
        }
        if matches!(code, Code::Enter | Code::NumpadEnter | Code::Tab) {
            let target = self.edit.as_ref().unwrap().target;
            if self.commit_edit(cx) && code == Code::Tab {
                let mut fields = Vec::new();
                if let Some(b) = self.selected_lift() {
                    for i in 0..3 {
                        if i != 2 || !(b.shape.is_cut() && b.order == 1) {
                            fields.push((
                                ValueTarget::Lift(i),
                                hud_value_rect_lift_at(&b, self.graph_db, i, self.gx(), self.gw()),
                            ));
                        }
                    }
                    fields.extend(
                        (0..5).map(|i| (ValueTarget::Lift(i + 3), self.band_value_rect(i))),
                    );
                }
                if let Some(b) = self.selected.and_then(|id| self.find_band(id)) {
                    for i in 0..3 {
                        if (i != 1 || b.shape.has_gain())
                            && (i != 2 || !(b.shape.is_cut() && b.order == 1))
                        {
                            fields.push((ValueTarget::Band(i), self.hud_value_rect(&b, i)));
                        }
                    }
                    if b.shape.has_gain() && b.dynamic && band_allows_dyn(&b) {
                        for i in 0..5 {
                            let target = match i {
                                0 => 3,
                                1 => 7,
                                2 => 4,
                                3 => 5,
                                _ => 6,
                            };
                            fields
                                .push((ValueTarget::Band(target), self.hud_dyn_value_rect(&b, i)));
                        }
                    }
                }
                fields.extend(self.global_value_hits());
                let index = fields.iter().position(|(t, _)| *t == target).unwrap_or(0);
                let next = if cx.modifiers().shift() {
                    (index + fields.len() - 1) % fields.len()
                } else {
                    (index + 1) % fields.len()
                };
                self.start_edit(cx, fields[next].0, fields[next].1);
            }
            return;
        }
        let edit = self.edit.as_mut().unwrap();
        let command = cx.modifiers().command();
        match code {
            Code::KeyA if command => {
                edit.anchor = 0;
                edit.cursor = edit.text.len();
            }
            Code::KeyC | Code::KeyX if command => {
                let _ = cx.set_clipboard(edit.text[edit.selection()].to_string());
                if code == Code::KeyX {
                    edit.insert("");
                }
            }
            Code::KeyV if command => {
                if let Ok(text) = cx.get_clipboard() {
                    edit.insert(&text);
                }
            }
            Code::Backspace => edit.erase(true),
            Code::Delete => edit.erase(false),
            Code::ArrowLeft | Code::ArrowRight | Code::Home | Code::End => {
                let selecting = cx.modifiers().shift();
                edit.cursor = match code {
                    Code::Home => 0,
                    Code::End => edit.text.len(),
                    Code::ArrowLeft if !selecting && edit.cursor != edit.anchor => {
                        edit.selection().start
                    }
                    Code::ArrowRight if !selecting && edit.cursor != edit.anchor => {
                        edit.selection().end
                    }
                    Code::ArrowLeft => edit.cursor.saturating_sub(1),
                    _ => (edit.cursor + 1).min(edit.text.len()),
                };
                if !selecting {
                    edit.anchor = edit.cursor;
                }
            }
            _ => {}
        }
    }

    fn global_controls(&self) -> &'static [usize] {
        match self.dyn_page {
            DynPage::Main => &[0],
            DynPage::Controls => &[5, 11, 6, 3, 4],
        }
    }
    fn param(&self, i: usize) -> &FloatParam {
        match i {
            0 => &self.params.compression,
            1 => &self.params.gate,
            2 => &self.params.pse_voice_det,
            3 => &self.params.dry,
            4 => &self.params.wet,
            5 => &self.params.comp_attack,
            6 => &self.params.comp_ratio,
            7 => &self.params.pse_depth,
            8 => &self.params.pse_hysteresis,
            9 => &self.params.pse_knee,
            10 => &self.params.pse_time,
            11 => &self.params.comp_release,
            12 => &self.params.comp_knee,
            14 => &self.params.comp_depth,
            15 => &self.params.output_gain,
            16 => &self.params.wall_even,
            17 => &self.params.wall_odd,
            18 => &self.params.wall_threshold,
            _ => &self.params.compression,
        }
    }
    fn select(&mut self, id: Option<u64>) {
        self.menu = None;
        self.selected = id;
        self.shared
            .selected_id
            .store(id.unwrap_or(0), Ordering::Relaxed);
        if id.is_none() {
            self.shared.solo_id.store(0, Ordering::Relaxed);
        }
    }
    fn find_band(&self, id: u64) -> Option<Band> {
        if id >= LIFT_ID_BASE {
            None
        } else if id >= SC_EQ_ID_BASE {
            self.params
                .sc_eq_bands
                .lock()
                .unwrap()
                .iter()
                .find(|b| b.id == id)
                .cloned()
        } else if id >= EQ2_ID_BASE {
            self.params
                .eq2_bands
                .lock()
                .unwrap()
                .iter()
                .find(|b| b.id == id)
                .cloned()
        } else {
            self.params
                .bands
                .lock()
                .unwrap()
                .iter()
                .find(|b| b.id == id)
                .cloned()
        }
    }
    fn start_eq_page(&mut self, page: usize) {
        self.anim_start.set(self.anim_progress.get());
        self.anim_target.set(page as f32);
        self.anim_time.set(0.0);
        self.active_eq.set(page);
        self.select(None);
        self.menu = None;
        self.edit = None;
    }
    fn clone_page_bands(&self) -> Vec<Band> {
        match self.active_eq.get() {
            EQ_PAGE_2 => self.params.eq2_bands.lock().unwrap().clone(),
            EQ_PAGE_SC => self.params.sc_eq_bands.lock().unwrap().clone(),
            EQ_PAGE_LIFT => Vec::new(),
            _ => self.params.bands.lock().unwrap().clone(),
        }
    }
    fn find_range_hit(&self, x: f32, y: f32, bands: &[Band]) -> Option<u64> {
        if self.active_eq.get() == EQ_PAGE_SC || self.active_eq.get() == EQ_PAGE_LIFT {
            return None;
        }
        bands
            .iter()
            .rev()
            .find(|b| range_handle_hit(b, x, y, self.graph_db, self.gx(), self.gw(), self.selected))
            .map(|b| b.id)
    }
    fn begin_range_drag(&mut self, cx: &mut EventContext, id: u64, x: f32, y: f32) {
        self.select(Some(id));
        self.change(|b| {
            if !b.dynamic {
                b.dynamic = true;
            }
        });
        self.drag = Some(Target::Range(id));
        self.last_drag = (x, y);
        cx.capture();
    }
    fn change(&self, f: impl FnOnce(&mut Band)) {
        let Some(id) = self.selected else {
            return;
        };
        if id >= LIFT_ID_BASE {
            return;
        }
        let mut bands = if id >= SC_EQ_ID_BASE {
            self.params.sc_eq_bands.lock().unwrap()
        } else if id >= EQ2_ID_BASE {
            self.params.eq2_bands.lock().unwrap()
        } else {
            self.params.bands.lock().unwrap()
        };
        if let Some(b) = bands.iter_mut().find(|b| b.id == id) {
            f(b);
            b.sanitize();
            if id >= SC_EQ_ID_BASE {
                b.dynamic = false;
            }
        }
    }
    fn is_lift_selected(&self) -> bool {
        self.selected.is_some_and(|id| id >= LIFT_ID_BASE)
    }
    fn selected_lift(&self) -> Option<LiftBand> {
        if !self.is_lift_selected() {
            return None;
        }
        self.params
            .lift_bands
            .lock()
            .unwrap()
            .iter()
            .find(|b| Some(b.id) == self.selected)
            .cloned()
    }
    fn change_lift(&self, f: impl FnOnce(&mut LiftBand)) {
        if let Some(b) = self
            .params
            .lift_bands
            .lock()
            .unwrap()
            .iter_mut()
            .find(|b| Some(b.id) == self.selected)
        {
            f(b);
            b.sanitize();
        }
    }
    fn create_band(&mut self, x: f32, y: f32, curve: bool, dynamic: bool) {
        if self.active_eq.get() == EQ_PAGE_LIFT {
            self.create_lift_band(x, y);
            return;
        }
        let page = self.active_eq.get();
        let id_base = match page {
            EQ_PAGE_2 => EQ2_ID_BASE,
            EQ_PAGE_SC => SC_EQ_ID_BASE,
            _ => 0,
        };
        let mut bands = match page {
            EQ_PAGE_2 => self.params.eq2_bands.lock().unwrap(),
            EQ_PAGE_SC => self.params.sc_eq_bands.lock().unwrap(),
            _ => self.params.bands.lock().unwrap(),
        };
        let id = bands.iter().map(|b| b.id).max().unwrap_or(id_base) + 1;
        let shape = infer_shape((x - self.gx()) / self.gw(), (y - GY) / GH, curve);
        bands.push(Band {
            id,
            shape,
            freq: self.x_freq(x),
            gain: if shape.has_gain() {
                y_db(y, self.graph_db)
            } else {
                0.0
            },
            q: if matches!(shape, Shape::LowCut | Shape::HighCut) {
                0.707
            } else {
                1.0
            },
            dynamic: dynamic && shape.has_gain() && page != EQ_PAGE_SC,
            ..Band::default()
        });
        drop(bands);
        self.select(Some(id));
        self.drag = Some(Target::Node(id));
        self.last_drag = (x, y);
    }
    fn create_lift_band(&mut self, x: f32, y: f32) {
        let mut lift_bands = self.params.lift_bands.lock().unwrap();
        let id = lift_bands
            .iter()
            .map(|b| b.id)
            .max()
            .unwrap_or(LIFT_ID_BASE)
            + 1;
        let shape = infer_shape((x - self.gx()) / self.gw(), (y - GY) / GH, false);
        lift_bands.push(LiftBand {
            id,
            shape,
            freq: self.x_freq(x).clamp(20.0, 20000.0),
            gain: lift_y_gain(y),
            ..LiftBand::default()
        });
        drop(lift_bands);
        self.select(Some(id));
        self.drag = Some(Target::Node(id));
        self.last_drag = (x, y);
    }
    fn lift_hit_at(&self, x: f32, y: f32) -> Option<u64> {
        if self.active_eq.get() != EQ_PAGE_LIFT {
            return None;
        }
        self.params
            .lift_bands
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|b| {
                ((x - self.freq_x(b.freq)).powi(2) + (y - lift_gain_y(b.gain)).powi(2)).sqrt()
                    < 16.0
            })
            .map(|b| b.id)
    }
    fn is_auditioning(&self, id: u64) -> bool {
        self.pending_solo == Some(id) || self.shared.solo_id.load(Ordering::Relaxed) == id
    }
    fn node_xy(&self, id: u64) -> (f32, f32) {
        if id >= LIFT_ID_BASE {
            if let Some(b) = self.selected_lift() {
                return (self.freq_x(b.freq), lift_gain_y(b.gain));
            }
        } else if let Some(b) = self.find_band(id) {
            let y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db);
            return (self.freq_x(b.freq), y);
        }
        self.down
    }
    fn begin_solo_audition_drag(&mut self, id: u64) {
        let restore = self.shared.solo_id.load(Ordering::Relaxed);
        self.shared.solo_id.store(id, Ordering::Relaxed);
        self.pending_solo = None;
        let (nx, ny) = self.node_xy(id);
        self.drag = Some(Target::SoloAudition {
            id,
            restore,
            grab_dx: nx - self.down.0,
            grab_dy: ny - self.down.1,
        });
        self.last_drag = self.down;
    }
    fn apply_drag(&mut self, cx: &mut EventContext, x: f32, y: f32, shift: bool, cmd: bool) {
        let graph_db = self.graph_db;
        let (_lx, ly) = self.last_drag;
        let dy = (ly - y) as f64;
        self.last_drag = (x, y);
        match self.drag {
            Some(Target::SoloAudition {
                id,
                grab_dx,
                grab_dy,
                ..
            }) => {
                let nx = (x + grab_dx).clamp(self.gx(), self.gx() + self.gw());
                let ny = y + grab_dy;
                if id >= LIFT_ID_BASE {
                    self.change_lift(|b| {
                        b.freq = self.x_freq(nx).clamp(20.0, 20000.0);
                        b.gain = lift_y_gain(ny);
                    });
                } else {
                    self.change(|b| {
                        b.freq = self.x_freq(nx);
                        if b.shape.has_gain() {
                            b.gain = y_db(ny, graph_db);
                        }
                    });
                }
            }
            Some(Target::Node(id)) => {
                if id >= LIFT_ID_BASE {
                    if shift {
                        self.change_lift(|b| {
                            if !(b.shape.is_cut() && b.order == 1) {
                                b.q = (b.q * (1.0 + dy * 0.025)).clamp(0.15, 18.0);
                            }
                        });
                    } else if cmd {
                        self.change_lift(|b| {
                            b.ratio = (b.ratio + dy * 0.10).clamp(1.0, 20.0);
                        });
                    } else {
                        self.change_lift(|b| {
                            b.freq = self.x_freq(x).clamp(20.0, 20000.0);
                            b.gain = lift_y_gain(y);
                        });
                    }
                } else if shift {
                    self.change(|b| {
                        if !(b.shape.is_cut() && b.order == 1) {
                            b.q = (b.q * (1.0 + dy * 0.025)).clamp(0.15, 18.0);
                        }
                    });
                } else if cmd && self.active_eq.get() != EQ_PAGE_SC {
                    self.change(|b| {
                        if !b.shape.has_gain() {
                            return;
                        }
                        if !b.dynamic {
                            b.dynamic = true;
                        }
                        b.ratio = (b.ratio + dy * 0.10).clamp(1.0, 20.0);
                    });
                } else {
                    self.change(|b| {
                        b.freq = self.x_freq(x);
                        if b.shape.has_gain() {
                            b.gain = y_db(y, graph_db);
                        }
                    });
                }
            }
            Some(Target::Global(i)) => {
                let p = self.param(i);
                if i == 0 && self.dyn_page == DynPage::Main {
                    let r = self.dyn_main_thresh_slider_rect();
                    let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                } else if i == 18 && self.wall_page == WallPage::Main {
                    let r = self.wall_main_thresh_slider_rect();
                    let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                } else if i == 1 && self.pse_page == PsePage::Main {
                    let r = self.pse_main_thresh_slider_rect();
                    let norm = 1.0 - ((y - r.1) / r.3).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                } else if i == 14 && self.dyn_page == DynPage::Main {
                    let r = self.dyn_main_gr_meter_rect();
                    let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                } else if i == 7 && self.pse_page == PsePage::Main {
                    let r = self.pse_main_gr_meter_rect();
                    let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                } else if i == 15 {
                    let r = output_gain_rect();
                    let bar_x = r.0 + 8.0;
                    let bar_w = r.2 - 16.0;
                    let norm = if shift {
                        let cur = p.unmodulated_normalized_value();
                        let dx = x - self.last_drag.0;
                        (cur + (dx / bar_w) * 0.1).clamp(0.0, 1.0)
                    } else {
                        ((x - bar_x) / bar_w).clamp(0.0, 1.0)
                    };
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                } else {
                    let cur = p.unmodulated_normalized_value();
                    let delta = (dy * if shift { 0.0015 } else { 0.007 }) as f32;
                    let norm =
                        (cur + if i == 0 || i == 18 { -delta } else { delta }).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                }
            }
            Some(Target::CompKnee { from_top }) => {
                let p = &self.params.comp_knee;
                let cur = p.unmodulated_normalized_value();
                let dir = if from_top { 1.0 } else { -1.0 };
                let delta = (dy * dir * if shift { 0.003 } else { 0.012 }) as f32;
                let norm = (cur + delta).clamp(0.0, 1.0);
                cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
            }
            Some(Target::PseKnee { from_top }) => {
                let p = &self.params.pse_knee;
                let cur = p.unmodulated_normalized_value();
                let dir = if from_top { 1.0 } else { -1.0 };
                let delta = (dy * dir * if shift { 0.003 } else { 0.012 }) as f32;
                let norm = (cur + delta).clamp(0.0, 1.0);
                cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
            }
            Some(Target::Range(_id)) => {
                self.change(|b| {
                    if !b.shape.has_gain() {
                        return;
                    }
                    b.dynamic = true;
                    if shift {
                        b.range = (b.range + dy * 0.1).clamp(-24.0, 24.0);
                    } else {
                        b.range = snap_dyn_range(b.gain, y, graph_db);
                    }
                });
            }
            Some(Target::Band(i)) => {
                let Some(b) = self.selected.and_then(|id| self.find_band(id)) else {
                    return;
                };
                let r = self.hud_dyn_field_rect(&b, i);
                let bar_x = r.0 + 8.0;
                let bar_w = (r.2 - 16.0).max(1.0);
                let n = ((x - bar_x) / bar_w).clamp(0.0, 1.0) as f64;
                self.change(|b| match i {
                    0 => b.threshold = -60.0 + 60.0 * n,
                    1 => b.range = -24.0 + 48.0 * n,
                    2 => b.ratio = 1.0 + 19.0 * n,
                    3 => b.attack = 0.1 * 2000.0_f64.powf(n),
                    _ => b.release = 10.0 * 200.0_f64.powf(n),
                });
            }
            Some(Target::LiftBand(i)) => {
                let r = self.band_rect(i);
                let n = ((x - r.0 - 12.0) / (r.2 - 24.0)).clamp(0.0, 1.0) as f64;
                self.change_lift(|b| match i {
                    0 => b.threshold = -60.0 + 60.0 * n,
                    1 => b.ratio = 1.0 + 19.0 * n,
                    2 => b.attack = 0.1 * 2000.0_f64.powf(n),
                    3 => b.release = 10.0 * 200.0_f64.powf(n),
                    _ => b.range = 24.0 * n,
                });
            }
            _ => {}
        }
    }
    fn toggle(&self, cx: &mut EventContext, p: &BoolParam) {
        cx.emit(RawParamEvent::BeginSetParameter(p.as_ptr()));
        cx.emit(RawParamEvent::SetParameterNormalized(
            p.as_ptr(),
            if p.value() { 0.0 } else { 1.0 },
        ));
        cx.emit(RawParamEvent::EndSetParameter(p.as_ptr()));
    }
    fn reset_float_param(&self, cx: &mut EventContext, p: &FloatParam) {
        let ptr = p.as_ptr();
        cx.emit(RawParamEvent::BeginSetParameter(ptr));
        cx.emit(RawParamEvent::SetParameterNormalized(
            ptr,
            p.default_normalized_value(),
        ));
        cx.emit(RawParamEvent::EndSetParameter(ptr));
    }
    fn finish_param_reset(&mut self, cx: &mut EventContext) {
        self.drag = None;
        cx.release();
        cx.needs_redraw();
    }
    fn try_reset_at(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        if self.value_at(x, y).is_some() {
            return false;
        }
        if inside(x, y, output_gain_rect()) {
            self.reset_float_param(cx, self.param(15));
            self.finish_param_reset(cx);
            return true;
        }
        if self.pse_page == PsePage::Main
            && (self.pse_anim_progress.get() - self.pse_anim_target.get()).abs() <= 0.01
        {
            let r = self.pse_main_thresh_slider_rect();
            let thresh_norm = self.params.gate.unmodulated_normalized_value();
            let thresh_y = r.1 + r.3 * (1.0 - thresh_norm);
            let handle = self.pse_main_thresh_handle_rect(thresh_y);
            let handle_hit = (
                handle.0 - 4.0,
                handle.1 - 2.0,
                handle.2 + 8.0,
                handle.3 + 4.0,
            );
            let knee_rect = self.pse_main_knee_rect(thresh_y, 0.0);
            let gr = self.pse_main_gr_meter_rect();
            let depth_y = gr.1 + gr.3 * self.params.pse_depth.unmodulated_normalized_value();
            let depth_handle = self.pse_main_depth_handle_rect(depth_y);
            if inside(x, y, handle_hit) {
                self.reset_float_param(cx, self.param(1));
                self.finish_param_reset(cx);
                return true;
            }
            if inside(x, y, knee_rect) {
                self.reset_float_param(cx, &self.params.pse_knee);
                self.finish_param_reset(cx);
                return true;
            }
            if inside(x, y, depth_handle) {
                self.reset_float_param(cx, self.param(7));
                self.finish_param_reset(cx);
                return true;
            }
            if inside(x, y, (r.0 - 10.0, r.1 - 10.0, r.2 + 20.0, r.3 + 20.0)) {
                self.reset_float_param(cx, self.param(1));
                self.finish_param_reset(cx);
                return true;
            }
        }
        if self.dyn_page == DynPage::Main
            && (self.dyn_anim_progress.get() - self.dyn_anim_target.get()).abs() <= 0.01
        {
            let r = self.dyn_main_thresh_slider_rect();
            let gr = self.dyn_main_gr_meter_rect();
            let thresh_norm = self.params.compression.unmodulated_normalized_value();
            let thresh_y = r.1 + r.3 * thresh_norm;
            let handle = self.dyn_main_thresh_handle_rect(thresh_y);
            let handle_hit = (
                handle.0 - 4.0,
                handle.1 - 2.0,
                handle.2 + 8.0,
                handle.3 + 4.0,
            );
            let knee_rect = self.dyn_main_knee_rect(thresh_y, 0.0);
            let depth_y = gr.1 + gr.3 * self.params.comp_depth.unmodulated_normalized_value();
            let depth_handle = self.dyn_main_depth_handle_rect(depth_y);
            if inside(x, y, handle_hit) {
                self.reset_float_param(cx, self.param(0));
                self.finish_param_reset(cx);
                return true;
            }
            if inside(x, y, knee_rect) {
                self.reset_float_param(cx, &self.params.comp_knee);
                self.finish_param_reset(cx);
                return true;
            }
            if inside(x, y, depth_handle) {
                self.reset_float_param(cx, self.param(14));
                self.finish_param_reset(cx);
                return true;
            }
            if inside(
                x,
                y,
                (r.0 - 10.0, r.1 - 10.0, r.2 + gr.2 + 20.0, r.3 + 20.0),
            ) {
                self.reset_float_param(cx, self.param(0));
                self.finish_param_reset(cx);
                return true;
            }
        }
        if self.wall_page == WallPage::Main
            && (self.wall_anim_progress.get() - self.wall_anim_target.get()).abs() <= 0.01
        {
            let r = self.wall_main_thresh_slider_rect();
            let thresh_norm = self.params.wall_threshold.unmodulated_normalized_value();
            let thresh_y = r.1 + r.3 * thresh_norm;
            let handle = self.wall_main_thresh_handle_rect(thresh_y);
            let handle_hit = (
                handle.0 - 4.0,
                handle.1 - 2.0,
                handle.2 + 8.0,
                handle.3 + 4.0,
            );
            if inside(x, y, handle_hit)
                || inside(x, y, (r.0 - 10.0, r.1 - 10.0, r.2 + 20.0, r.3 + 20.0))
            {
                self.reset_float_param(cx, self.param(18));
                self.finish_param_reset(cx);
                return true;
            }
        }
        if let Some(i) = self
            .global_hit_rects()
            .into_iter()
            .find(|(_, r)| inside(x, y, *r))
            .map(|(i, _)| i)
        {
            self.reset_float_param(cx, self.param(i));
            self.finish_param_reset(cx);
            return true;
        }
        if self.is_lift_selected() {
            if let Some(i) = (0..5).find(|i| {
                inside(x, y, self.band_bar_rect(*i)) && !inside(x, y, self.band_value_rect(*i))
            }) {
                let defaults = LiftBand::default();
                self.change_lift(|b| match i {
                    0 => b.threshold = defaults.threshold,
                    1 => b.ratio = defaults.ratio,
                    2 => b.attack = defaults.attack,
                    3 => b.release = defaults.release,
                    _ => b.range = defaults.range,
                });
                self.finish_param_reset(cx);
                return true;
            }
        } else if let Some(b) = self
            .selected
            .and_then(|id| self.find_band(id))
            .filter(|b| b.shape.has_gain() && b.dynamic && band_allows_dyn(b))
        {
            if let Some(i) = (0..5).find(|i| {
                inside(x, y, self.hud_dyn_field_rect(&b, *i))
                    && !inside(x, y, self.hud_dyn_value_rect(&b, *i))
            }) {
                let defaults = Band::default();
                self.change(|band| match i {
                    0 => band.threshold = defaults.threshold,
                    1 => band.range = defaults.range,
                    2 => band.ratio = defaults.ratio,
                    3 => band.attack = defaults.attack,
                    _ => band.release = defaults.release,
                });
                self.finish_param_reset(cx);
                return true;
            }
        }
        false
    }
    fn delete(&mut self) {
        if let Some(id) = self.selected {
            if id >= LIFT_ID_BASE {
                self.params
                    .lift_bands
                    .lock()
                    .unwrap()
                    .retain(|b| b.id != id);
            } else if id >= SC_EQ_ID_BASE {
                self.params
                    .sc_eq_bands
                    .lock()
                    .unwrap()
                    .retain(|b| b.id != id);
            } else if id >= EQ2_ID_BASE {
                self.params.eq2_bands.lock().unwrap().retain(|b| b.id != id);
            } else {
                self.params.bands.lock().unwrap().retain(|b| b.id != id);
            }
            if self.shared.solo_id.load(Ordering::Relaxed) == id {
                self.shared.solo_id.store(0, Ordering::Relaxed);
            }
            self.select(None);
        }
    }
}
impl View for StripView {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, meta| {
            let bounds = cx.bounds();
            let Some(scale) = view_scale(bounds) else {
                return;
            };
            let x = (cx.mouse().cursorx - bounds.x) / scale;
            let y = (cx.mouse().cursory - bounds.y) / scale;
            if self.edit.is_some() {
                match e {
                    WindowEvent::CharInput(c) => {
                        if !cx.modifiers().command() && c.is_ascii() && !c.is_control() {
                            self.edit.as_mut().unwrap().insert(&c.to_string());
                        }
                        meta.consume();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::KeyDown(code, key) => {
                        self.edit_key(cx, *code);
                        // Some hosts deliver only KeyDown (empty characters / keyCode).
                        let has_char = matches!(key, Some(Key::Character(_)));
                        if !has_char && !cx.modifiers().command() {
                            if let Some(c) = typed_char(*code, cx.modifiers().shift()) {
                                self.edit.as_mut().unwrap().insert(&c.to_string());
                            }
                        }
                        meta.consume();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::MouseDown(MouseButton::Left) => {
                        if inside(x, y, self.edit.as_ref().unwrap().rect) {
                            let edit = self.edit.as_mut().unwrap();
                            let capacity = ((edit.rect.2 - 8.0) / 6.6) as usize;
                            let start = edit.cursor.saturating_sub(capacity);
                            edit.cursor = (start
                                + (((x - edit.rect.0 - 4.0) / 6.6).round().max(0.0) as usize))
                                .min(edit.text.len());
                            edit.anchor = edit.cursor;
                            cx.needs_redraw();
                            return;
                        }
                        if !self.commit_edit(cx) {
                            self.edit = None;
                            cx.needs_redraw();
                        }
                    }
                    WindowEvent::FocusOut => {
                        if !self.commit_edit(cx) {
                            self.edit = None;
                        }
                    }
                    WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                        let edit = self.edit.as_mut().unwrap();
                        edit.anchor = 0;
                        edit.cursor = edit.text.len();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::MouseDoubleClick(_)
                    | WindowEvent::MouseScroll(_, _)
                    | WindowEvent::MouseDown(_) => return,
                    _ => {}
                }
            }
            match e {
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some((x, y));
                    if let Some(pending) = self.pending_create {
                        if (x - self.down.0).hypot(y - self.down.1) >= DRAG_START_THRESHOLD {
                            let (dx, dy) = self.down;
                            let curve = matches!(pending, PendingCreate::Curve);
                            self.create_band(dx, dy, curve, cx.modifiers().alt());
                            self.pending_create = None;
                        }
                    }
                    if let Some(id) = self.pending_solo {
                        if (x - self.down.0).hypot(y - self.down.1) >= DRAG_START_THRESHOLD {
                            self.begin_solo_audition_drag(id);
                        }
                    }
                    if self.drag.is_some() {
                        let shift = cx.modifiers().shift();
                        let cmd = cx.modifiers().command();
                        self.apply_drag(cx, x, y, shift, cmd);
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseLeave => {
                    self.hover = None;
                    cx.needs_redraw();
                }
                WindowEvent::MouseDown(MouseButton::Left) => {
                    cx.focus();
                    if inside(x, y, THEME_BUTTON) {
                        preferences::toggle();
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.footer_comp_routing_rect()) {
                        self.toggle(cx, &self.params.comp_pre);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.footer_auto_rect()) {
                        self.toggle(cx, &self.params.auto_makeup);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.footer_link_rect()) {
                        self.toggle(cx, &self.params.stereo_link);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, output_gain_rect()) {
                        if let Some((target, rect)) = self.value_at(x, y) {
                            self.start_edit(cx, target, rect);
                            cx.needs_redraw();
                            return;
                        }
                        self.drag = Some(Target::Global(15));
                        self.down = (x, y);
                        self.last_drag = (x, y);
                        cx.capture();
                        let r = output_gain_rect();
                        let bar_x = r.0 + 8.0;
                        let bar_w = r.2 - 16.0;
                        let norm = ((x - bar_x) / bar_w).clamp(0.0, 1.0);
                        let p = self.param(15);
                        cx.emit(RawParamEvent::BeginSetParameter(p.as_ptr()));
                        cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.dyn_power_button_rect()) {
                        self.dyn_bypass_anim.trigger_click();
                        self.toggle(cx, &self.params.comp_on);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.pse_power_button_rect()) {
                        self.pse_bypass_anim.trigger_click();
                        self.toggle(cx, &self.params.pse_on);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.wall_power_button_rect()) {
                        self.wall_bypass_anim.trigger_click();
                        self.toggle(cx, &self.params.wall_on);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.dyn_cog_button_rect()) {
                        match self.dyn_page {
                            DynPage::Main => {
                                self.dyn_page = DynPage::Controls;
                                self.dyn_anim_target.set(1.0);
                            }
                            DynPage::Controls => {
                                self.dyn_page = DynPage::Main;
                                self.dyn_anim_target.set(0.0);
                            }
                        }
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.pse_cog_button_rect()) {
                        match self.pse_page {
                            PsePage::Main => {
                                self.pse_page = PsePage::Controls;
                                self.pse_anim_target.set(1.0);
                            }
                            PsePage::Controls => {
                                self.pse_page = PsePage::Main;
                                self.pse_anim_target.set(0.0);
                            }
                        }
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.wall_cog_button_rect()) {
                        match self.wall_page {
                            WallPage::Main => {
                                self.wall_page = WallPage::Controls;
                                self.wall_anim_target.set(1.0);
                            }
                            WallPage::Controls => {
                                self.wall_page = WallPage::Main;
                                self.wall_anim_target.set(0.0);
                            }
                        }
                        cx.needs_redraw();
                        return;
                    }
                    match self.pse_page {
                        PsePage::Main => {
                            if (self.pse_anim_progress.get() - self.pse_anim_target.get()).abs()
                                > 0.01
                                && inside(x, y, self.pse_bounds())
                            {
                                return;
                            }
                            let r = self.pse_main_thresh_slider_rect();
                            let gate_ptr = self.params.gate.as_ptr();
                            let thresh_norm = self.params.gate.unmodulated_normalized_value();
                            let thresh_y = r.1 + r.3 * (1.0 - thresh_norm);
                            let handle = self.pse_main_thresh_handle_rect(thresh_y);
                            let handle_hit = (
                                handle.0 - 4.0,
                                handle.1 - 2.0,
                                handle.2 + 8.0,
                                handle.3 + 4.0,
                            );
                            let knee_rect = self.pse_main_knee_rect(thresh_y, 0.0);
                            if inside(x, y, handle_hit) {
                                self.drag = Some(Target::Global(1));
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(gate_ptr));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                            if inside(x, y, knee_rect) {
                                let from_top = y < thresh_y;
                                self.drag = Some(Target::PseKnee { from_top });
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(
                                    self.params.pse_knee.as_ptr(),
                                ));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                            let gr = self.pse_main_gr_meter_rect();
                            let pse_depth_ptr = self.params.pse_depth.as_ptr();
                            let depth_norm = self.params.pse_depth.unmodulated_normalized_value();
                            let depth_y = gr.1 + gr.3 * depth_norm;
                            let depth_handle = self.pse_main_depth_handle_rect(depth_y);
                            if inside(x, y, depth_handle) {
                                self.drag = Some(Target::Global(7));
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(pse_depth_ptr));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                            if inside(x, y, (r.0 - 10.0, r.1 - 10.0, r.2 + 20.0, r.3 + 20.0)) {
                                self.drag = Some(Target::Global(1));
                                self.last_drag = (x, y);
                                let norm = 1.0 - ((y - r.1) / r.3).clamp(0.0, 1.0);
                                cx.emit(RawParamEvent::BeginSetParameter(gate_ptr));
                                cx.emit(RawParamEvent::SetParameterNormalized(gate_ptr, norm));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                        }
                        PsePage::Controls => {
                            if (self.pse_anim_progress.get() - self.pse_anim_target.get()).abs()
                                > 0.01
                                && inside(x, y, self.pse_bounds())
                            {
                                return;
                            }
                            if inside(x, y, self.pse_detect_mode_rect()) {
                                self.toggle(cx, &self.params.pse_peak);
                                cx.needs_redraw();
                                return;
                            }
                        }
                    }
                    match self.dyn_page {
                        DynPage::Main => {
                            if (self.dyn_anim_progress.get() - self.dyn_anim_target.get()).abs()
                                > 0.01
                                && inside(x, y, self.dyn_bounds())
                            {
                                return;
                            }
                            let r = self.dyn_main_thresh_slider_rect();
                            let gr = self.dyn_main_gr_meter_rect();
                            let comp_ptr = self.params.compression.as_ptr();
                            let thresh_norm =
                                self.params.compression.unmodulated_normalized_value();
                            let thresh_y = r.1 + r.3 * thresh_norm;
                            let handle = self.dyn_main_thresh_handle_rect(thresh_y);
                            let handle_hit = (
                                handle.0 - 4.0,
                                handle.1 - 2.0,
                                handle.2 + 8.0,
                                handle.3 + 4.0,
                            );
                            let knee_rect = self.dyn_main_knee_rect(thresh_y, 0.0);
                            if inside(x, y, handle_hit) {
                                self.drag = Some(Target::Global(0));
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(comp_ptr));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                            if inside(x, y, knee_rect) && !inside(x, y, handle_hit) {
                                let from_top = y < thresh_y;
                                self.drag = Some(Target::CompKnee { from_top });
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(
                                    self.params.comp_knee.as_ptr(),
                                ));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                            let comp_depth_ptr = self.params.comp_depth.as_ptr();
                            let depth_norm = self.params.comp_depth.unmodulated_normalized_value();
                            let depth_y = gr.1 + gr.3 * depth_norm;
                            let depth_handle = self.dyn_main_depth_handle_rect(depth_y);
                            if inside(x, y, depth_handle) {
                                self.drag = Some(Target::Global(14));
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(comp_depth_ptr));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                            let combined = (r.0 - 10.0, r.1 - 10.0, r.2 + gr.2 + 20.0, r.3 + 20.0);
                            if inside(x, y, combined) {
                                self.drag = Some(Target::Global(0));
                                self.last_drag = (x, y);
                                let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                                cx.emit(RawParamEvent::BeginSetParameter(comp_ptr));
                                cx.emit(RawParamEvent::SetParameterNormalized(comp_ptr, norm));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                        }
                        DynPage::Controls => {
                            if (self.dyn_anim_progress.get() - self.dyn_anim_target.get()).abs()
                                > 0.01
                                && inside(x, y, self.dyn_bounds())
                            {
                                return;
                            }
                        }
                    }
                    match self.wall_page {
                        WallPage::Main => {
                            if (self.wall_anim_progress.get() - self.wall_anim_target.get()).abs()
                                > 0.01
                                && inside(x, y, self.wall_bounds())
                            {
                                return;
                            }
                            let r = self.wall_main_thresh_slider_rect();
                            let wall_ptr = self.params.wall_threshold.as_ptr();
                            let thresh_norm =
                                self.params.wall_threshold.unmodulated_normalized_value();
                            let thresh_y = r.1 + r.3 * thresh_norm;
                            let handle = self.wall_main_thresh_handle_rect(thresh_y);
                            let handle_hit = (
                                handle.0 - 4.0,
                                handle.1 - 2.0,
                                handle.2 + 8.0,
                                handle.3 + 4.0,
                            );
                            if inside(x, y, handle_hit)
                                || inside(x, y, (r.0 - 10.0, r.1 - 10.0, r.2 + 20.0, r.3 + 20.0))
                            {
                                self.drag = Some(Target::Global(18));
                                self.last_drag = (x, y);
                                let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                                cx.emit(RawParamEvent::BeginSetParameter(wall_ptr));
                                cx.emit(RawParamEvent::SetParameterNormalized(wall_ptr, norm));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                        }
                        WallPage::Controls => {
                            if (self.wall_anim_progress.get() - self.wall_anim_target.get()).abs()
                                > 0.01
                                && inside(x, y, self.wall_bounds())
                            {
                                return;
                            }
                        }
                    }
                    if let Some(resolution) = self.processing_menu.take() {
                        let r = processing_menu_rect(resolution);
                        if inside(x, y, r) {
                            let row = ((y - r.1) / 24.0) as usize;
                            let (ptr, normalized) = if resolution {
                                (self.params.linear_resolution.as_ptr(), row as f32 / 4.0)
                            } else {
                                (self.params.processing_mode.as_ptr(), row as f32 / 2.0)
                            };
                            cx.emit(RawParamEvent::BeginSetParameter(ptr));
                            cx.emit(RawParamEvent::SetParameterNormalized(ptr, normalized));
                            cx.emit(RawParamEvent::EndSetParameter(ptr));
                        }
                        cx.needs_redraw();
                        return;
                    }
                    let resolution = self.params.processing_mode.value()
                        == ProcessingMode::LinearPhase
                        && inside(x, y, resolution_button_rect());
                    if inside(x, y, PROCESS_BUTTON) || resolution {
                        self.processing_menu = Some(resolution);
                        self.menu = None;
                        self.scale_menu = false;
                        cx.needs_redraw();
                        return;
                    }
                    if self.scale_menu {
                        self.scale_menu = false;
                        if inside(x, y, self.scale_menu_rect()) {
                            let index = ((y - self.scale_menu_rect().1) / 24.0) as usize;
                            if let Some(range) = SCALES.get(index) {
                                self.set_graph_range(*range);
                            }
                        }
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.scale_button_rect()) {
                        self.scale_menu = true;
                        self.menu = None;
                        cx.needs_redraw();
                        return;
                    }
                    if let Some(menu) = self.menu.take() {
                        let handled = if let Some(b) = self.selected_lift() {
                            let r = menu.rect_lift_at(&b, self.graph_db, self.gx(), self.gw());
                            if inside(x, y, r) {
                                let row = ((y - r.1) / 24.0) as usize;
                                if row < menu.count() {
                                    self.change_lift(|b| match menu {
                                        BandMenu::Shape => {
                                            let shape = Shape::ALL[row];
                                            if shape.is_cut() && !b.shape.is_cut() {
                                                b.q = std::f64::consts::FRAC_1_SQRT_2;
                                            }
                                            b.shape = shape;
                                        }
                                        BandMenu::Order => b.order = row as u8 + 1,
                                    });
                                }
                            }
                            true
                        } else {
                            false
                        };
                        if !handled {
                            let band = self.selected.and_then(|id| self.find_band(id));
                            if let Some(b) = band {
                                let r = menu.rect_at(&b, self.graph_db, self.gx(), self.gw());
                                if inside(x, y, r) {
                                    let row = ((y - r.1) / 24.0) as usize;
                                    if row < menu.count() {
                                        self.change(|b| match menu {
                                            BandMenu::Shape => {
                                                let shape = Shape::ALL[row];
                                                if shape.is_cut() && !b.shape.is_cut() {
                                                    b.q = std::f64::consts::FRAC_1_SQRT_2;
                                                }
                                                b.shape = shape;
                                                if !shape.has_gain() {
                                                    b.dynamic = false;
                                                }
                                            }
                                            BandMenu::Order => b.order = row as u8 + 1,
                                        });
                                    }
                                }
                            }
                        }
                        cx.needs_redraw();
                        return;
                    }
                    if let Some((target, rect)) = self.value_at(x, y) {
                        self.start_edit(cx, target, rect);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.eq_tab_1_rect()) {
                        if self.active_eq.get() != EQ_PAGE_1 {
                            self.start_eq_page(EQ_PAGE_1);
                            cx.needs_redraw();
                        }
                        return;
                    }
                    if inside(x, y, self.eq_tab_2_rect()) {
                        if self.active_eq.get() != EQ_PAGE_2 {
                            self.start_eq_page(EQ_PAGE_2);
                            cx.needs_redraw();
                        }
                        return;
                    }
                    if inside(x, y, self.eq_tab_sc_rect()) {
                        if self.active_eq.get() != EQ_PAGE_SC {
                            self.start_eq_page(EQ_PAGE_SC);
                            cx.needs_redraw();
                        }
                        return;
                    }
                    if inside(x, y, self.eq_tab_lift_rect()) {
                        if self.active_eq.get() != EQ_PAGE_LIFT {
                            self.start_eq_page(EQ_PAGE_LIFT);
                            cx.needs_redraw();
                        }
                        return;
                    }
                    if self.active_eq.get() == EQ_PAGE_SC
                        && inside(x, y, self.eq_header_listen_sc_rect())
                    {
                        self.toggle(cx, &self.params.pse_listen);
                        cx.needs_redraw();
                        return;
                    }
                    self.down = (x, y);
                    self.last_drag = (x, y);
                    if inside(x, y, self.eq_power_rect()) {
                        self.eq_bypass_anim.trigger_click();
                        match self.active_eq.get() {
                            EQ_PAGE_2 => self.toggle(cx, &self.params.eq2_on),
                            EQ_PAGE_LIFT => self.toggle(cx, &self.params.lift_on),
                            EQ_PAGE_SC => self.toggle(cx, &self.params.sc_eq_on),
                            _ => self.toggle(cx, &self.params.eq_on),
                        }
                    } else {
                        if (self.anim_progress.get() - self.anim_target.get()).abs() > 0.01
                            && inside(x, y, self.eq_bounds())
                        {
                            return;
                        }
                        // Lift dock sits on top of HUD and graph.
                        let mut bottom_control_handled = false;
                        let bottom_dock = (
                            self.gx(),
                            GRAPH_BOTTOM - LIFT_DOCK_H,
                            self.gw(),
                            LIFT_DOCK_H,
                        );
                        if let Some(i) = self
                            .global_hit_rects()
                            .into_iter()
                            .find(|(_, r)| inside(x, y, *r))
                            .map(|(i, _)| i)
                        {
                            bottom_control_handled = true;
                            self.drag = Some(Target::Global(i));
                            self.last_drag = (x, y);
                            cx.emit(RawParamEvent::BeginSetParameter(self.param(i).as_ptr()));
                            cx.capture();
                        } else if self.is_lift_selected() {
                            let badge_rect = (self.gx() + 8.0, GRAPH_BOTTOM - 48.0, 32.0, 28.0);
                            if inside(x, y, badge_rect) {
                                self.lift_badge_anim.trigger_click();
                                bottom_control_handled = true;
                                self.change_lift(|b| b.enabled = !b.enabled);
                            } else if let Some(i) =
                                (0..5).find(|i| inside(x, y, self.band_bar_rect(*i)))
                            {
                                bottom_control_handled = true;
                                self.drag = Some(Target::LiftBand(i));
                                self.last_drag = (x, y);
                                cx.capture();
                                self.apply_drag(cx, x, y, false, false);
                            } else if inside(x, y, bottom_dock) {
                                bottom_control_handled = true;
                            }
                        }

                        if !bottom_control_handled {
                            // Check if click hits selected band HUD
                            let mut hud_consumed = false;
                            if let Some(sel_id) = self.selected {
                                if sel_id >= LIFT_ID_BASE {
                                    if let Some(b) = self.selected_lift() {
                                        let (bx, by, bw, bh) = self.hud_rect_for_lift(&b);
                                        if inside(x, y, (bx, by, bw, bh)) {
                                            hud_consumed = true;
                                            match hud_chrome_hit(x, y, bx, by, bw, b.shape.is_cut())
                                            {
                                                Some(HudChrome::Bypass) => {
                                                    self.change_lift(|b| b.enabled = !b.enabled);
                                                }
                                                Some(HudChrome::Shape) => {
                                                    self.menu = Some(BandMenu::Shape);
                                                }
                                                Some(HudChrome::Solo) => {
                                                    self.pending_solo = Some(b.id);
                                                    self.down = (x, y);
                                                    self.last_drag = (x, y);
                                                    cx.capture();
                                                }
                                                Some(HudChrome::Close) => self.delete(),
                                                Some(HudChrome::Order) => {
                                                    self.menu = Some(BandMenu::Order);
                                                }
                                                None => {}
                                            }
                                        }
                                    }
                                } else if let Some(b) = self.find_band(sel_id) {
                                    let g = self.hud_geom(&b);
                                    let in_hud = inside(x, y, self.hud_bounds_for(&b))
                                        || hud_dyn_btn_hit(g, x, y);
                                    if in_hud {
                                        hud_consumed = true;
                                        if hud_dyn_btn_hit(g, x, y) {
                                            self.dyn_band_anim.trigger_click();
                                            self.change(|b| b.dynamic = !b.dynamic);
                                        } else {
                                            match hud_chrome_hit(
                                                x,
                                                y,
                                                g.bx,
                                                g.by,
                                                g.bw,
                                                b.shape.is_cut(),
                                            ) {
                                                Some(HudChrome::Bypass) => {
                                                    self.change(|b| b.enabled = !b.enabled);
                                                }
                                                Some(HudChrome::Shape) => {
                                                    self.menu = Some(BandMenu::Shape);
                                                }
                                                Some(HudChrome::Solo) => {
                                                    self.pending_solo = Some(b.id);
                                                    self.down = (x, y);
                                                    self.last_drag = (x, y);
                                                    cx.capture();
                                                }
                                                Some(HudChrome::Close) => self.delete(),
                                                Some(HudChrome::Order) => {
                                                    self.menu = Some(BandMenu::Order);
                                                }
                                                None => {
                                                    if b.dynamic && band_allows_dyn(&b) {
                                                        if let Some(i) = (0..5).find(|i| {
                                                            inside(
                                                                x,
                                                                y,
                                                                self.hud_dyn_field_rect(&b, *i),
                                                            )
                                                        }) {
                                                            self.drag = Some(Target::Band(i));
                                                            self.last_drag = (x, y);
                                                            cx.capture();
                                                            self.apply_drag(cx, x, y, false, false);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if !hud_consumed && inside(x, y, self.graph_area()) {
                                let lift_hit = self.lift_hit_at(x, y);
                                if let Some(id) = lift_hit {
                                    self.select(Some(id));
                                    if cx.modifiers().alt() {
                                        self.change_lift(|b| b.enabled = !b.enabled);
                                    } else {
                                        self.drag = Some(Target::Node(id));
                                        self.last_drag = (x, y);
                                        cx.capture();
                                    }
                                } else {
                                    let bands = self.clone_page_bands();
                                    if let Some(id) = self.find_range_hit(x, y, &bands) {
                                        self.begin_range_drag(cx, id, x, y);
                                    } else {
                                        let hit = bands
                                            .iter()
                                            .rev()
                                            .find(|b| {
                                                ((x - self.freq_x(b.freq)).powi(2)
                                                    + (y - db_y(
                                                        if b.shape.has_gain() {
                                                            b.gain
                                                        } else {
                                                            0.0
                                                        },
                                                        self.graph_db,
                                                    ))
                                                    .powi(2))
                                                .sqrt()
                                                    < 16.0
                                            })
                                            .map(|b| b.id);
                                        if let Some(id) = hit {
                                            self.select(Some(id));
                                            if cx.modifiers().alt() {
                                                self.change(|b| b.enabled = !b.enabled);
                                            } else {
                                                self.drag = Some(Target::Node(id));
                                                self.last_drag = (x, y);
                                                cx.capture();
                                            }
                                        } else {
                                            let sr = self.shared.sample_rate.load(Ordering::Relaxed)
                                                as f64;
                                            let config = self.params.processing_config();
                                            let eq_sr = config.mode.rate(sr);
                                            let meters = self.dyn_gr_uncapped();
                                            let db = bands
                                                .iter()
                                                .filter(|b| b.enabled)
                                                .map(|b| {
                                                    BandCoeffs::make(&plot_band(b, &meters), eq_sr)
                                                        .response(self.x_freq(x), eq_sr)
                                                })
                                                .sum::<f64>();
                                            if (y - db_y(db, self.graph_db)).abs() < 12.0 {
                                                self.pending_create = Some(PendingCreate::Curve);
                                                cx.capture();
                                            } else if self.selected.is_none() {
                                                self.create_band(x, y, false, cx.modifiers().alt());
                                                cx.capture();
                                            } else {
                                                self.pending_create =
                                                    Some(PendingCreate::Background);
                                                cx.capture();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    if self.menu.is_some() || self.scale_menu || self.processing_menu.is_some() {
                        return;
                    }
                    if self.try_reset_at(cx, x, y) {
                        return;
                    }
                    if inside(x, y, self.graph_area()) && self.drag.is_none() {
                        if self.is_lift_selected() {
                            let bottom_dock = (
                                self.gx(),
                                GRAPH_BOTTOM - LIFT_DOCK_H,
                                self.gw(),
                                LIFT_DOCK_H,
                            );
                            if inside(x, y, bottom_dock) {
                                return;
                            }
                        }
                        let mut over_hud = false;
                        if let Some(sel_id) = self.selected {
                            if sel_id >= LIFT_ID_BASE {
                                if let Some(b) = self.selected_lift() {
                                    if inside(x, y, self.hud_rect_for_lift(&b)) {
                                        over_hud = true;
                                    }
                                }
                            } else if let Some(b) = self.find_band(sel_id) {
                                if inside(x, y, self.hud_bounds_for(&b)) {
                                    over_hud = true;
                                }
                            }
                        }
                        if !over_hud {
                            let lift_hit = self.lift_hit_at(x, y);
                            if let Some(id) = lift_hit {
                                self.select(Some(id));
                                self.change_lift(|b| b.enabled = !b.enabled);
                            } else {
                                let bands = self.clone_page_bands();
                                if let Some(id) = self.find_range_hit(x, y, &bands) {
                                    self.select(Some(id));
                                    self.change(|b| {
                                        b.dynamic = true;
                                        b.range = 0.0;
                                    });
                                } else {
                                    let hit = bands
                                        .iter()
                                        .rev()
                                        .find(|b| {
                                            ((x - self.freq_x(b.freq)).powi(2)
                                                + (y - db_y(
                                                    if b.shape.has_gain() { b.gain } else { 0.0 },
                                                    self.graph_db,
                                                ))
                                                .powi(2))
                                            .sqrt()
                                                < 16.0
                                        })
                                        .map(|b| b.id);
                                    if let Some(id) = hit {
                                        self.select(Some(id));
                                        self.change(|b| b.enabled = !b.enabled);
                                    } else {
                                        self.pending_create = None;
                                        self.create_band(x, y, false, cx.modifiers().alt());
                                        cx.capture();
                                    }
                                }
                            }
                        }
                    }
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    if let Some(Target::Global(i)) = self.drag {
                        cx.emit(RawParamEvent::EndSetParameter(self.param(i).as_ptr()));
                    }
                    if let Some(Target::CompKnee { .. }) = self.drag {
                        cx.emit(RawParamEvent::EndSetParameter(
                            self.params.comp_knee.as_ptr(),
                        ));
                    }
                    if let Some(Target::PseKnee { .. }) = self.drag {
                        cx.emit(RawParamEvent::EndSetParameter(
                            self.params.pse_knee.as_ptr(),
                        ));
                    }
                    if let Some(Target::SoloAudition { restore, .. }) = self.drag {
                        self.shared.solo_id.store(restore, Ordering::Relaxed);
                    }
                    if self.pending_create.is_some() {
                        self.select(None);
                    }
                    if let Some(id) = self.pending_solo.take() {
                        let cur = self.shared.solo_id.load(Ordering::Relaxed);
                        if cur == id {
                            self.shared.solo_id.store(0, Ordering::Relaxed);
                        } else {
                            self.shared.solo_id.store(id, Ordering::Relaxed);
                        }
                    }
                    self.drag = None;
                    self.pending_create = None;
                    cx.release();
                    cx.needs_redraw();
                }
                WindowEvent::MouseDown(MouseButton::Right) => {
                    if (self.anim_progress.get() - self.anim_target.get()).abs() > 0.01
                        && inside(x, y, self.eq_bounds())
                    {
                        return;
                    }
                    if inside(x, y, self.graph_area()) {
                        let mut over_hud = false;
                        if let Some(sel_id) = self.selected {
                            if sel_id >= LIFT_ID_BASE {
                                if let Some(b) = self.selected_lift() {
                                    if inside(x, y, self.hud_rect_for_lift(&b)) {
                                        over_hud = true;
                                    }
                                }
                            } else if let Some(b) = self.find_band(sel_id) {
                                if inside(x, y, self.hud_bounds_for(&b)) {
                                    over_hud = true;
                                }
                            }
                        }
                        if !over_hud {
                            let lift_id = self.lift_hit_at(x, y);
                            if lift_id.is_some() {
                                self.select(lift_id);
                                self.delete();
                            } else {
                                let bands = self.clone_page_bands();
                                let id = bands
                                    .iter()
                                    .find(|b| {
                                        (x - self.freq_x(b.freq)).abs() < 15.0
                                            && (y - db_y(
                                                if b.shape.has_gain() { b.gain } else { 0.0 },
                                                self.graph_db,
                                            ))
                                            .abs()
                                                < 15.0
                                    })
                                    .map(|b| b.id);
                                if id.is_some() {
                                    self.select(id);
                                    self.delete();
                                }
                            }
                        }
                    }
                }
                WindowEvent::MouseScroll(_, dy) => {
                    if (self.anim_progress.get() - self.anim_target.get()).abs() > 0.01
                        && inside(x, y, self.eq_bounds())
                    {
                        return;
                    }
                    if inside(
                        x,
                        y,
                        (
                            self.gx() - EQ_GRAPH_PAD_LEFT,
                            GY,
                            EQ_GRAPH_PAD_LEFT,
                            GRAPH_BOTTOM - GY + 32.0,
                        ),
                    ) && *dy != 0.0
                    {
                        let cur_idx = SCALES
                            .iter()
                            .position(|&s| (s - self.graph_db).abs() < 0.1)
                            .unwrap_or(2);
                        let next_idx = if *dy > 0.0 {
                            cur_idx.saturating_sub(1)
                        } else {
                            (cur_idx + 1).min(SCALES.len() - 1)
                        };
                        self.set_graph_range(SCALES[next_idx]);
                        cx.needs_redraw();
                        return;
                    }
                    if self.menu.is_some() || self.scale_menu || self.processing_menu.is_some() {
                        return;
                    }
                    if self.dyn_page == DynPage::Main {
                        let r = self.dyn_main_thresh_slider_rect();
                        let gr = self.dyn_main_gr_meter_rect();
                        if inside(
                            x,
                            y,
                            (r.0 - 10.0, r.1 - 10.0, r.2 + gr.2 + 20.0, r.3 + 20.0),
                        ) && *dy != 0.0
                        {
                            let p = self.param(0);
                            let cur = p.unmodulated_normalized_value();
                            let norm = (cur - *dy * 0.02).clamp(0.0, 1.0);
                            cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                            cx.needs_redraw();
                            return;
                        }
                    }
                    if let Some((hx, hy)) = self.hover {
                        if let Some(i) = self
                            .global_hit_rects()
                            .into_iter()
                            .find(|(_, r)| inside(hx, hy, *r))
                            .map(|(i, _)| i)
                        {
                            let p = self.param(i);
                            let cur = p.unmodulated_normalized_value();
                            let norm =
                                (cur + *dy * if i == 0 { -0.03 } else { 0.03 }).clamp(0.0, 1.0);
                            cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                            cx.needs_redraw();
                            return;
                        }
                    }
                    if inside(x, y, self.graph_area()) {
                        if self.selected.is_none() {
                            let lift_hit = self.lift_hit_at(x, y);
                            if lift_hit.is_some() {
                                self.select(lift_hit);
                            } else {
                                let bands = self.clone_page_bands();
                                let hit = bands
                                    .iter()
                                    .rev()
                                    .find(|b| {
                                        ((x - self.freq_x(b.freq)).powi(2)
                                            + (y - db_y(
                                                if b.shape.has_gain() { b.gain } else { 0.0 },
                                                self.graph_db,
                                            ))
                                            .powi(2))
                                        .sqrt()
                                            < 18.0
                                    })
                                    .map(|b| b.id);
                                if hit.is_some() {
                                    self.select(hit);
                                }
                            }
                        }
                        if self.is_lift_selected() {
                            self.change_lift(|b| {
                                if !(b.shape.is_cut() && b.order == 1) {
                                    b.q = (b.q * (1.0 + *dy as f64 * 0.08)).clamp(0.15, 18.0);
                                }
                            });
                            cx.needs_redraw();
                        } else if self.selected.is_some() {
                            self.change(|b| {
                                if !(b.shape.is_cut() && b.order == 1) {
                                    b.q = (b.q * (1.0 + *dy as f64 * 0.08)).clamp(0.15, 18.0);
                                }
                            });
                            cx.needs_redraw();
                        }
                    }
                }
                WindowEvent::KeyDown(Code::Delete | Code::Backspace, _) => {
                    self.delete();
                    cx.needs_redraw();
                }
                WindowEvent::KeyDown(Code::Escape, _) => {
                    self.processing_menu = None;
                    self.menu = None;
                    self.scale_menu = false;
                    self.pending_create = None;
                    cx.needs_redraw();
                }
                _ => {}
            }
        });
    }
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let Some(scale) = view_scale(bounds) else {
            return;
        };
        if self.font.get().is_none() {
            self.font.set(canvas.add_font_mem(FONT_JETBRAINS_MONO).ok());
        }
        if self.signature.get().is_none() {
            self.signature.set(
                canvas
                    .add_font_mem(include_bytes!("assets/Allura-Regular.ttf"))
                    .ok(),
            );
        }
        let mut d = Draw::new(
            canvas,
            preferences::light(),
            scale,
            bounds.x,
            bounds.y,
            self.font.get(),
        );
        d.rect(0.0, 0.0, UI_W, UI_H, BG);
        d.rect(0.0, 0.0, UI_W, HEADER_H, PANEL);
        d.text(32.0, 46.0, "dB", 28.0, GOLD);
        d.text(88.0, 46.0, "SIGNATURE CHANNEL STRIP", 16.0, TEXT);
        draw_signature(&mut d, self.signature.get());

        let now = std::time::Instant::now();
        let dt = self
            .last_tick
            .get()
            .map(|prev| (now - prev).as_secs_f32())
            .unwrap_or(0.016)
            .clamp(0.001, 0.1);
        self.last_tick.set(Some(now));

        let cur_t = self.anim_time.get();
        let next_t = if cur_t < 1.0 {
            (cur_t + dt / 0.25).min(1.0)
        } else {
            1.0
        };
        self.anim_time.set(next_t);
        let eased_t = next_t * next_t * next_t * (next_t * (next_t * 6.0 - 15.0) + 10.0);
        let start_p = self.anim_start.get();
        let target_p = self.anim_target.get();
        let next_p = start_p + (target_p - start_p) * eased_t;
        self.anim_progress.set(next_p);
        tick_page_anim(&self.dyn_anim_progress, self.dyn_anim_target.get(), dt);
        tick_page_anim(&self.pse_anim_progress, self.pse_anim_target.get(), dt);
        tick_page_anim(&self.wall_anim_progress, self.wall_anim_target.get(), dt);

        let offset_1 = (0.0 - next_p) * GRAPH_CLIP_W;
        let offset_2 = (1.0 - next_p) * GRAPH_CLIP_W;
        let offset_lift = (EQ_PAGE_LIFT as f32 - next_p) * GRAPH_CLIP_W;
        let offset_sc = (EQ_PAGE_SC as f32 - next_p) * GRAPH_CLIP_W;

        let page = self.active_eq.get();
        let eq1_on = self.params.eq_on.value();
        let eq2_on = self.params.eq2_on.value();
        let lift_on = self.params.lift_on.value();
        let sc_eq_on = self.params.sc_eq_on.value();
        let active_bypassed = match page {
            EQ_PAGE_2 => !eq2_on,
            EQ_PAGE_LIFT => !lift_on,
            EQ_PAGE_SC => !sc_eq_on,
            _ => !eq1_on,
        };
        let bands = self.params.bands.lock().unwrap().clone();
        let eq2_bands = self.params.eq2_bands.lock().unwrap().clone();
        let sc_eq_bands = self.params.sc_eq_bands.lock().unwrap().clone();
        let lift_bands = self.params.lift_bands.lock().unwrap().clone();

        let eq_b = self.eq_bounds();
        let eq_x = eq_b.0;
        let gx = self.gx();
        let gw = self.gw();
        d.rect(eq_b.0, eq_b.1, eq_b.2, eq_b.3, PANEL);
        d.outline(eq_b, LINE);

        let eq_power_r = self.eq_power_rect();
        let eq_power_hovered = self
            .idle_hover()
            .is_some_and(|(hx, hy)| inside(hx, hy, eq_power_r));
        let eq_power_click = self.eq_bypass_anim.step();
        d.bypass_button(
            eq_power_r,
            active_bypassed,
            TEAL,
            eq_power_hovered,
            eq_power_click,
        );
        d.text(
            eq_x + 44.0,
            module_title_y(16.0),
            match page {
                EQ_PAGE_LIFT => "LIFT",
                EQ_PAGE_SC => "SIDECHAIN EQ",
                _ => "PARAMETRIC EQ",
            },
            16.0,
            if active_bypassed { MUTED } else { TEXT },
        );
        d.tab_button(
            self.eq_tab_1_rect(),
            "EQ 1",
            page == EQ_PAGE_1,
            if eq1_on { TEAL } else { MUTED },
        );
        d.tab_button(
            self.eq_tab_2_rect(),
            "EQ 2",
            page == EQ_PAGE_2,
            if eq2_on { TEAL } else { MUTED },
        );
        d.tab_button(
            self.eq_tab_lift_rect(),
            "LIFT",
            page == EQ_PAGE_LIFT,
            if lift_on { LIFT_COLOR } else { MUTED },
        );
        d.tab_button(
            self.eq_tab_sc_rect(),
            "SC EQ",
            page == EQ_PAGE_SC,
            if sc_eq_on { TEAL } else { MUTED },
        );
        if page == EQ_PAGE_SC {
            d.button(
                self.eq_header_listen_sc_rect(),
                "LISTEN SC",
                self.params.pse_listen.value(),
                TEAL,
            );
        }

        if active_bypassed {
            d.text(
                eq_x + 478.0,
                module_title_y(12.0),
                match page {
                    EQ_PAGE_2 => "EQ 2 STAGE BYPASSED",
                    EQ_PAGE_LIFT => "LIFT STAGE BYPASSED",
                    EQ_PAGE_SC => "SC EQ STAGE BYPASSED",
                    _ => "EQ 1 STAGE BYPASSED",
                },
                12.0,
                GOLD,
            );
        } else if self.shared.solo_id.load(Ordering::Relaxed) != 0 {
            d.text(
                eq_x + 478.0,
                module_title_y(12.0),
                "SOLO AUDITION ACTIVE",
                12.0,
                GOLD,
            );
        }

        let spectrum: Vec<_> = (0..128)
            .map(|i| {
                let db = self.shared.spectrum[i].load(Ordering::Relaxed);
                (
                    gx + i as f32 / 127.0 * gw,
                    GY + GH - (db + 90.0).clamp(0.0, 90.0) / 90.0 * GH * 0.86,
                )
            })
            .collect();
        let sr = self.shared.sample_rate.load(Ordering::Relaxed) as f64;
        let config = self.params.processing_config();
        let eq_sr = config.mode.rate(sr);

        let mut raw_lift_gr = [0.0_f64; 256];
        let mut raw_lift_uncapped = [0.0_f64; 256];
        for k in 0..256 {
            raw_lift_gr[k] = self.shared.lift_bin_gr[k].load(Ordering::Relaxed) as f64;
            raw_lift_uncapped[k] =
                self.shared.lift_bin_gr_uncapped[k].load(Ordering::Relaxed) as f64;
        }
        let smooth_lift_gr = smooth_lift_bins(&raw_lift_gr);
        let smooth_lift_uncapped = smooth_lift_bins(&raw_lift_uncapped);
        let dyn_uncapped = self
            .shared
            .dyn_gr_uncapped
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|e| e.into_inner().clone());

        let scale_btn = self.scale_button_rect();
        let scale_menu_r = self.scale_menu_rect();
        let draw_grid_and_spectrum = |d: &mut Draw, bypassed: bool, eq_idx: usize| {
            let step = if self.graph_db <= 6.0 { 6 } else { 12 };
            for db in (-(self.graph_db as i32)..=self.graph_db as i32).step_by(step) {
                let y = db_y(db as f64, self.graph_db);
                d.line(
                    gx,
                    y,
                    gx + gw,
                    y,
                    if db == 0 { C::rgb(75, 81, 85) } else { LINE },
                    1.0,
                );
                if db != self.graph_db as i32 {
                    d.text_centered(
                        gx - EQ_GRAPH_PAD_LEFT * 0.5,
                        axis_label_y(y, 13.0),
                        &axis_db_text(db),
                        13.0,
                        MUTED,
                    );
                }
            }
            for (freq, label) in FREQ_AXIS {
                let x = freq_x_at(freq, gx, gw);
                d.line(x, GY, x, GRAPH_BOTTOM, LINE, 1.0);
                d.text_centered(x, EQ_AXIS_LABEL_Y, label, 12.0, MUTED);
            }
            d.area(
                &spectrum,
                GY + GH,
                if bypassed {
                    C::rgba(80, 90, 100, 10)
                } else {
                    C::rgba(125, 143, 159, 28)
                },
            );
            d.poly(
                &spectrum,
                if bypassed {
                    C::rgba(90, 100, 110, 20)
                } else {
                    C::rgba(144, 161, 175, 62)
                },
                1.0,
            );
            let scale_hover = self
                .idle_hover()
                .is_some_and(|(hx, hy)| inside(hx, hy, scale_btn));
            d.rect(scale_btn.0, scale_btn.1, scale_btn.2, scale_btn.3, PANEL);
            d.text_centered(
                scale_btn.0 + scale_btn.2 * 0.5,
                axis_label_y(GY, 13.0),
                &format!("±{} ▾", self.graph_db as i32),
                13.0,
                if scale_hover { TEXT } else { MUTED },
            );
            if self.scale_menu && self.active_eq.get() == eq_idx {
                let r = scale_menu_r;
                d.rect(r.0, r.1, r.2, r.3, PANEL);
                d.outline(r, LINE);
                for (i, range) in SCALES.iter().enumerate() {
                    let row = (r.0, r.1 + i as f32 * 24.0, r.2, 24.0);
                    if *range == self.graph_db {
                        d.rect(row.0, row.1, row.2, row.3, LINE);
                    }
                    d.text(
                        row.0 + 10.0,
                        row.1 + 16.0,
                        &format!("±{}", *range as i32),
                        11.0,
                        TEXT,
                    );
                }
            }
        };

        d.scissor(eq_b.0, eq_b.1, eq_b.2, eq_b.3);

        // --- DRAW EQ 1 (if visible) ---
        if offset_1 > -GRAPH_CLIP_W && offset_1 < GRAPH_CLIP_W {
            d.offset_x = offset_1;
            let eq1_bypassed = !eq1_on;
            draw_grid_and_spectrum(&mut d, eq1_bypassed, 0);

            let extras = bands.iter().filter(|b| b.enabled).map(|b| b.freq);
            let xs = eq_curve_xs(gx, gw, extras);
            let mut sum = vec![0.0; xs.len()];
            let mut selected_curve: Option<(Vec<(f32, f32)>, C)> = None;
            for b in &bands {
                if !b.enabled {
                    continue;
                }
                let coeff = BandCoeffs::make(&plot_band(b, &dyn_uncapped), eq_sr);
                let color = if eq1_bypassed {
                    MUTED
                } else {
                    COLORS[(b.id as usize - 1) % COLORS.len()]
                };
                let dbs = eq_db_on_xs(&coeff, &xs, gx, gw, sr, eq_sr);
                for (i, db) in dbs.iter().enumerate() {
                    sum[i] += db;
                }
                if Some(b.id) == self.selected {
                    let points = eq_points(&xs, &dbs, self.graph_db);
                    let mut fill = color;
                    fill.a = if eq1_bypassed { 0.02 } else { 0.07 };
                    d.area(&points, db_y(0.0, self.graph_db), fill);
                    d.poly(&points, color, 1.2);
                    selected_curve = Some((points, color));
                }
            }
            let points = eq_points(&xs, &sum, self.graph_db);
            let sum_color = if eq1_bypassed { MUTED } else { GOLD };
            d.poly(&points, sum_color, 2.2);
            if let Some(b) = bands
                .iter()
                .find(|b| b.id == self.shared.solo_id.load(Ordering::Relaxed))
            {
                let fill = selected_curve.as_ref().map(|(pts, color)| {
                    let mut fill = *color;
                    fill.a = if eq1_bypassed { 0.02 } else { 0.07 };
                    (pts.as_slice(), db_y(0.0, self.graph_db), fill)
                });
                draw_solo_shade(
                    &mut d, b.shape, b.freq, b.q, gx, gw, eq_b, &points, sum_color, 2.2, fill,
                );
            }

            for b in &bands {
                let color = if !b.enabled || eq1_bypassed {
                    MUTED
                } else {
                    COLORS[(b.id as usize - 1) % COLORS.len()]
                };
                let x = freq_x_at(b.freq, gx, gw);
                let y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db);
                if b.dynamic && b.shape.has_gain() {
                    draw_dyn_range_stem(
                        &mut d,
                        b,
                        self.graph_db,
                        (gx, gw),
                        color,
                        uncapped_for(b.id, &dyn_uncapped),
                    );
                } else if Some(b.id) == self.selected {
                    d.circle(x, y, 12.0, color, false);
                }
                d.circle(x, y, 7.0, color, true);
            }

            if let Some(b) = bands.iter().find(|b| Some(b.id) == self.selected) {
                let color = if !b.enabled || eq1_bypassed {
                    MUTED
                } else {
                    COLORS[(b.id as usize - 1) % COLORS.len()]
                };
                let is_solo = self.is_auditioning(b.id);
                let g = hud_geom_for_at(b, self.graph_db, gx, gw);
                let dyn_hover = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| hud_dyn_btn_hit(g, hx, hy));
                // Node-drag clearance: hide the selected-band HUD card while dragging by node
                // so the underlying graph stays fully visible.
                if !matches!(self.drag, Some(Target::Node(_))) {
                    band_hud(
                        &mut d,
                        b,
                        is_solo,
                        color,
                        self.graph_db,
                        config.mode == ProcessingMode::LinearPhase,
                        gx,
                        gw,
                        dyn_hover,
                        self.dyn_band_anim.step(),
                    );
                }
            }

            // Hover cursor on EQ 1
            if page == 0 && (next_p - target_p).abs() < 0.05 {
                if let Some((x, y)) = self.idle_hover() {
                    if inside(x, y, (gx, GY, gw, GH))
                        && self.drag.is_none()
                        && self.menu.is_none()
                        && !self.scale_menu
                        && self.processing_menu.is_none()
                        && self.edit.is_none()
                        && !bands.iter().any(|b| {
                            Some(b.id) == self.selected
                                && inside(x, y, hud_bounds_for_at(b, self.graph_db, gx, gw))
                        })
                        && !eq1_bypassed
                    {
                        let near = bands
                            .iter()
                            .any(|b| near_eq_node(b, x, y, self.graph_db, gx, gw));
                        if !near {
                            let db = bands
                                .iter()
                                .filter(|b| b.enabled)
                                .map(|b| {
                                    BandCoeffs::make(&plot_band(b, &dyn_uncapped), eq_sr)
                                        .response(x_freq_at(x, gx, gw), eq_sr)
                                })
                                .sum::<f64>();
                            let curve = (y - db_y(db, self.graph_db)).abs() < 12.0;
                            let shape = infer_shape((x - gx) / gw, (y - GY) / GH, curve);
                            draw_eq_hover_preview(
                                &mut d,
                                x,
                                y,
                                gx,
                                gw,
                                self.graph_db,
                                sr,
                                eq_sr,
                                self.selected,
                                shape,
                            );
                        }
                    }
                }
            }
        }

        // --- DRAW LIFT (if visible) ---
        if offset_lift > -GRAPH_CLIP_W && offset_lift < GRAPH_CLIP_W {
            d.offset_x = offset_lift;
            let lift_bypassed = !lift_on;
            for db in [0, -20, -40, -60, -80, -100] {
                let y = lift_gain_y(db as f64);
                d.line(
                    gx,
                    y,
                    gx + gw,
                    y,
                    if db == 0 { C::rgb(75, 81, 85) } else { LINE },
                    1.0,
                );
                d.text_centered(
                    gx - 22.0,
                    axis_label_y(y, 13.0),
                    &axis_db_text(db),
                    13.0,
                    MUTED,
                );
            }
            for (freq, label) in FREQ_AXIS {
                let x = freq_x_at(freq, gx, gw);
                d.line(x, GY, x, GRAPH_BOTTOM, LINE, 1.0);
                d.text_centered(x, EQ_AXIS_LABEL_Y, label, 12.0, MUTED);
            }
            d.area(
                &spectrum,
                GY + GH,
                if lift_bypassed {
                    C::rgba(80, 90, 100, 10)
                } else {
                    C::rgba(125, 143, 159, 28)
                },
            );
            d.poly(
                &spectrum,
                if lift_bypassed {
                    C::rgba(90, 100, 110, 20)
                } else {
                    C::rgba(144, 161, 175, 62)
                },
                1.0,
            );

            for b in &lift_bands {
                if !b.enabled {
                    continue;
                }
                let color = if lift_bypassed { MUTED } else { LIFT_COLOR };
                let points = lift_curve_points(b, &smooth_lift_gr, gx, gw, sr);
                draw_lift_influence(
                    &mut d,
                    b,
                    &points,
                    color,
                    (
                        if lift_bypassed { 0.05 } else { 0.22 },
                        if lift_bypassed { 0.25 } else { 0.90 },
                    ),
                    (gx, gw),
                );
                let overflow = !lift_bypassed
                    && smooth_lift_uncapped
                        .iter()
                        .zip(smooth_lift_gr.iter())
                        .any(|(uncapped, capped)| *uncapped > *capped + 0.15);
                if overflow {
                    let over_pts = lift_curve_points(b, &smooth_lift_uncapped, gx, gw, sr);
                    draw_lift_influence(&mut d, b, &over_pts, MUTED, (0.0, 0.55), (gx, gw));
                }
            }
            if let Some(b) = lift_bands
                .iter()
                .find(|b| b.id == self.shared.solo_id.load(Ordering::Relaxed))
            {
                let lift_pts = lift_curve_points(b, &smooth_lift_gr, gx, gw, sr);
                let lift_color = if lift_bypassed { MUTED } else { LIFT_COLOR };
                let mut fill = lift_color;
                fill.a = if lift_bypassed { 0.05 } else { 0.22 };
                draw_solo_shade(
                    &mut d,
                    b.shape,
                    b.freq,
                    b.q,
                    gx,
                    gw,
                    eq_b,
                    &lift_pts,
                    lift_color,
                    1.8,
                    Some((lift_pts.as_slice(), GY + GH, fill)),
                );
            }

            for (idx, b) in lift_bands.iter().enumerate() {
                let color = if !b.enabled || lift_bypassed {
                    MUTED
                } else {
                    LIFT_COLOR
                };
                let x = freq_x_at(b.freq, gx, gw);
                let y = lift_gain_y(b.gain);
                if Some(b.id) == self.selected {
                    d.circle(x, y, 13.0, color, false);
                }
                d.circle(x, y, 7.0, color, true);
                let label = if lift_bands.len() > 1 {
                    format!("{} LIFT {}", b.shape.uppercase_name(), idx + 1)
                } else {
                    format!("{} LIFT", b.shape.uppercase_name())
                };
                d.text(x - 12.0, y - 17.0, &label, 10.0, color);
            }

            if let Some(b) = lift_bands.iter().find(|b| Some(b.id) == self.selected) {
                let color = if !b.enabled || lift_bypassed {
                    MUTED
                } else {
                    LIFT_COLOR
                };
                let is_solo = self.is_auditioning(b.id);
                band_hud_lift(&mut d, b, is_solo, color, self.graph_db, gx, gw);
                let c = if lift_on { LIFT_COLOR } else { MUTED };
                let badge_rect = (gx + 8.0, GRAPH_BOTTOM - 48.0, 32.0, 28.0);
                let badge_hovered = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, badge_rect));
                let badge_click = self.lift_badge_anim.step();
                d.bypass_button(
                    badge_rect,
                    !b.enabled,
                    LIFT_COLOR,
                    badge_hovered,
                    badge_click,
                );
                for (i, label, val, n) in [
                    (
                        0,
                        "THRESHOLD",
                        format!("{:.1}", b.threshold),
                        (b.threshold + 60.0) / 60.0,
                    ),
                    (
                        1,
                        "RATIO",
                        format!("{:.1}:1", b.ratio),
                        (b.ratio - 1.0) / 19.0,
                    ),
                    (
                        2,
                        "ATTACK",
                        format!("{:.1} ms", b.attack),
                        (b.attack / 0.1).log(2000.0),
                    ),
                    (
                        3,
                        "RELEASE",
                        format!("{:.0} ms", b.release),
                        (b.release / 10.0).log(200.0),
                    ),
                    (4, "RANGE", format!("{:.1}", b.range), b.range / 24.0),
                ] {
                    d.control(band_rect_at(i, gx, gw), label, &val, n as f32, c);
                }
            }

            if page == EQ_PAGE_LIFT && (next_p - target_p).abs() < 0.05 {
                if let Some((x, y)) = self.idle_hover() {
                    if inside(x, y, (gx, GY, gw, GH))
                        && self.drag.is_none()
                        && self.menu.is_none()
                        && self.edit.is_none()
                        && !lift_bypassed
                        && !lift_bands.iter().any(|b| {
                            Some(b.id) == self.selected
                                && inside(x, y, hud_rect_for_lift_at(b, self.graph_db, gx, gw))
                        })
                    {
                        let near = lift_bands.iter().any(|b| {
                            (x - freq_x_at(b.freq, gx, gw)).abs() < 15.0
                                && (y - lift_gain_y(b.gain)).abs() < 15.0
                        });
                        if !near {
                            let shape = infer_shape((x - gx) / gw, (y - GY) / GH, false);
                            if self.selected.is_none() {
                                let preview_band = LiftBand {
                                    id: 0,
                                    shape,
                                    freq: x_freq_at(x, gx, gw).clamp(20.0, 20000.0),
                                    gain: lift_y_gain(y),
                                    ..LiftBand::default()
                                };
                                let points =
                                    lift_curve_points(&preview_band, &smooth_lift_gr, gx, gw, sr);
                                draw_lift_influence(
                                    &mut d,
                                    &preview_band,
                                    &points,
                                    MUTED,
                                    (0.08, 0.40),
                                    (gx, gw),
                                );
                            }
                            draw_hover_preview(
                                &mut d,
                                x,
                                y,
                                gx,
                                gw,
                                hover_preview_label(self.selected.is_some(), shape),
                            );
                        }
                    }
                }
            }
        }

        // --- DRAW EQ 2 / SC EQ (if visible) ---
        for (page_idx, offset, page_bands, page_on, allow_dyn) in [
            (EQ_PAGE_2, offset_2, eq2_bands.as_slice(), eq2_on, true),
            (
                EQ_PAGE_SC,
                offset_sc,
                sc_eq_bands.as_slice(),
                sc_eq_on,
                false,
            ),
        ] {
            if !(offset > -GRAPH_CLIP_W && offset < GRAPH_CLIP_W) {
                continue;
            }
            d.offset_x = offset;
            let page_bypassed = !page_on;
            draw_grid_and_spectrum(&mut d, page_bypassed, page_idx);

            let extras = page_bands.iter().filter(|b| b.enabled).map(|b| b.freq);
            let xs = eq_curve_xs(gx, gw, extras);
            let mut sum = vec![0.0; xs.len()];
            let mut selected_curve: Option<(Vec<(f32, f32)>, C)> = None;
            for b in page_bands {
                if !b.enabled {
                    continue;
                }
                let coeff = BandCoeffs::make(&plot_band(b, &dyn_uncapped), eq_sr);
                let num = band_display_num(b.id);
                let color = if page_bypassed {
                    MUTED
                } else {
                    COLORS[(num as usize - 1) % COLORS.len()]
                };
                let dbs = eq_db_on_xs(&coeff, &xs, gx, gw, sr, eq_sr);
                for (i, db) in dbs.iter().enumerate() {
                    sum[i] += db;
                }
                if Some(b.id) == self.selected {
                    let points = eq_points(&xs, &dbs, self.graph_db);
                    let mut fill = color;
                    fill.a = if page_bypassed { 0.02 } else { 0.07 };
                    d.area(&points, db_y(0.0, self.graph_db), fill);
                    d.poly(&points, color, 1.2);
                    selected_curve = Some((points, color));
                }
            }
            let points = eq_points(&xs, &sum, self.graph_db);
            let sum_color = if page_bypassed { MUTED } else { GOLD };
            d.poly(&points, sum_color, 2.2);
            if let Some(b) = page_bands
                .iter()
                .find(|b| b.id == self.shared.solo_id.load(Ordering::Relaxed))
            {
                let fill = selected_curve.as_ref().map(|(pts, color)| {
                    let mut fill = *color;
                    fill.a = if page_bypassed { 0.02 } else { 0.07 };
                    (pts.as_slice(), db_y(0.0, self.graph_db), fill)
                });
                draw_solo_shade(
                    &mut d, b.shape, b.freq, b.q, gx, gw, eq_b, &points, sum_color, 2.2, fill,
                );
            }

            for b in page_bands {
                let num = band_display_num(b.id);
                let color = if !b.enabled || page_bypassed {
                    MUTED
                } else {
                    COLORS[(num as usize - 1) % COLORS.len()]
                };
                let x = freq_x_at(b.freq, gx, gw);
                let y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db);
                if allow_dyn && b.dynamic && b.shape.has_gain() {
                    draw_dyn_range_stem(
                        &mut d,
                        b,
                        self.graph_db,
                        (gx, gw),
                        color,
                        uncapped_for(b.id, &dyn_uncapped),
                    );
                } else if Some(b.id) == self.selected {
                    d.circle(x, y, 12.0, color, false);
                }
                d.circle(x, y, 7.0, color, true);
            }

            if let Some(b) = page_bands.iter().find(|b| Some(b.id) == self.selected) {
                let num = band_display_num(b.id);
                let color = if !b.enabled || page_bypassed {
                    MUTED
                } else {
                    COLORS[(num as usize - 1) % COLORS.len()]
                };
                let is_solo = self.is_auditioning(b.id);
                let g = hud_geom_for_at(b, self.graph_db, gx, gw);
                let dyn_hover = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| hud_dyn_btn_hit(g, hx, hy));
                band_hud(
                    &mut d,
                    b,
                    is_solo,
                    color,
                    self.graph_db,
                    config.mode == ProcessingMode::LinearPhase,
                    gx,
                    gw,
                    dyn_hover,
                    self.dyn_band_anim.step(),
                );
            }

            if page == page_idx && (next_p - target_p).abs() < 0.05 {
                if let Some((x, y)) = self.idle_hover() {
                    if inside(x, y, (gx, GY, gw, GH))
                        && self.drag.is_none()
                        && self.menu.is_none()
                        && !self.scale_menu
                        && self.processing_menu.is_none()
                        && self.edit.is_none()
                        && !page_bands.iter().any(|b| {
                            Some(b.id) == self.selected
                                && inside(x, y, hud_bounds_for_at(b, self.graph_db, gx, gw))
                        })
                        && !page_bypassed
                    {
                        let near = page_bands
                            .iter()
                            .any(|b| near_eq_node(b, x, y, self.graph_db, gx, gw));
                        if !near {
                            let db = page_bands
                                .iter()
                                .filter(|b| b.enabled)
                                .map(|b| {
                                    BandCoeffs::make(&plot_band(b, &dyn_uncapped), eq_sr)
                                        .response(x_freq_at(x, gx, gw), eq_sr)
                                })
                                .sum::<f64>();
                            let curve = (y - db_y(db, self.graph_db)).abs() < 12.0;
                            let shape = infer_shape((x - gx) / gw, (y - GY) / GH, curve);
                            draw_eq_hover_preview(
                                &mut d,
                                x,
                                y,
                                gx,
                                gw,
                                self.graph_db,
                                sr,
                                eq_sr,
                                self.selected,
                                shape,
                            );
                        }
                    }
                }
            }
        }

        let seam_frac = next_p - next_p.floor();
        if seam_frac > 0.005 && seam_frac < 0.995 {
            d.offset_x = 0.0;
            let seam_x = eq_b.0 + (next_p.floor() + 1.0 - next_p) * GRAPH_CLIP_W;
            d.line(seam_x, eq_b.1, seam_x, eq_b.1 + eq_b.3, LINE, 1.0);
        }

        d.reset_scissor();
        d.offset_x = 0.0;
        d.alpha_mul = 1.0;
        let comp_bypassed = !self.params.comp_on.value();
        let pse_bypassed = !self.params.pse_on.value();
        let (px, py, pw, ph) = self.pse_bounds();
        let (dx, dy, dw, dh) = self.dyn_bounds();
        let is_pre = self.is_pre();

        d.rect(px, py, pw, ph, PANEL);
        d.outline((px, py, pw, ph), LINE);
        let pse_power_r = self.pse_power_button_rect();
        let pse_hovered = self
            .idle_hover()
            .is_some_and(|(hx, hy)| inside(hx, hy, pse_power_r));
        let pse_click = self.pse_bypass_anim.step();
        d.bypass_button(pse_power_r, pse_bypassed, PSE_BLUE, pse_hovered, pse_click);
        d.text(
            px + 44.0,
            module_title_y(15.0),
            "PSE",
            15.0,
            if pse_bypassed { MUTED } else { TEXT },
        );
        let pse_cog_r = self.pse_cog_button_rect();
        draw_module_page_button(
            &mut d,
            pse_cog_r,
            self.pse_page == PsePage::Controls,
            pse_bypassed,
            !pse_bypassed
                && self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, pse_cog_r)),
            PSE_BLUE,
        );

        d.scissor(px, py, pw, ph);
        let pse_eased = quintic_page_progress(self.pse_anim_progress.get());
        let pse_main_off = (0.0 - pse_eased) * pw;
        let pse_ctrl_off = (1.0 - pse_eased) * pw;
        if pse_main_off > -pw && pse_main_off < pw {
            d.offset_x = pse_main_off;
            {
                let (sx, sy, sw, sh) = self.pse_main_thresh_slider_rect();
                let (gx_m, gy_m, gw_m, gh_m) = self.pse_main_gr_meter_rect();
                d.rect(sx, sy, sw, sh, LINE);
                d.rect(gx_m, gy_m, gw_m, gh_m, LINE);

                let sc_level = if pse_bypassed {
                    -90.0
                } else {
                    self.shared.sc_level.load(Ordering::Relaxed) as f64
                };
                let sc_db = sc_level.clamp(-48.0, 0.0);
                let sig_frac = ((sc_db + 48.0) / 48.0) as f32;
                let sig_h = sh * sig_frac;
                let sig_top_y = sy + sh - sig_h;

                let p1 = self.param(1);
                let thresh_norm = p1.unmodulated_normalized_value();
                let thresh_y = sy + sh * (1.0 - thresh_norm);

                if !pse_bypassed && sig_h > 0.5 {
                    let below_thresh_top = sig_top_y.max(thresh_y);
                    let below_thresh_h = (sy + sh) - below_thresh_top;
                    if below_thresh_h > 0.0 {
                        d.rect(
                            sx,
                            below_thresh_top,
                            sw,
                            below_thresh_h,
                            C::rgb(45, 120, 165),
                        );
                    }
                    if sig_top_y < thresh_y {
                        d.rect(sx, sig_top_y, sw, thresh_y - sig_top_y, PSE_BLUE);
                    }
                }

                let pse_gr = if pse_bypassed {
                    0.0
                } else {
                    self.shared.pse_gr.load(Ordering::Relaxed)
                };
                let depth_norm = self.param(7).unmodulated_normalized_value();
                let pse_gr = catch_gr_to_depth(pse_gr, depth_norm * 20.0);
                let gr_bar_h = gh_m * (pse_gr / 20.0).clamp(0.0, 1.0);
                if !pse_bypassed && gr_bar_h > 0.5 {
                    d.rect(gx_m, gy_m, gw_m, gr_bar_h, PSE_BLUE);
                }

                let depth_y = gy_m + gh_m * depth_norm;
                let depth_handle = self.pse_main_depth_handle_rect(depth_y);
                let depth_hover = !pse_bypassed
                    && (self
                        .idle_hover()
                        .is_some_and(|(hx, hy)| inside(hx, hy, depth_handle))
                        || self.drag == Some(Target::Global(7)));
                let handle_color = if pse_bypassed {
                    MUTED
                } else if depth_hover {
                    TEXT
                } else {
                    PSE_BLUE
                };
                draw_gr_depth_handle(&mut d, gx_m, gw_m, depth_y, handle_color);

                let handle = self.pse_main_thresh_handle_rect(thresh_y);
                let handle_hit = (
                    handle.0 - 4.0,
                    handle.1 - 2.0,
                    handle.2 + 8.0,
                    handle.3 + 4.0,
                );
                let knee_norm = (self.params.pse_knee.value() / 18.0).clamp(0.0, 1.0);
                let knee_hover = !pse_bypassed
                    && (self.idle_hover().is_some_and(|(hx, hy)| {
                        inside(hx, hy, self.pse_main_knee_rect(thresh_y, 0.0))
                            && !inside(hx, hy, handle_hit)
                    }) || matches!(self.drag, Some(Target::PseKnee { .. })));
                let bulge_w = knee_pulse_bulge(&self.pse_knee_bulge, knee_hover);
                draw_knee_zone(
                    &mut d,
                    self.pse_main_knee_rect(thresh_y, 0.0),
                    bulge_w,
                    knee_norm,
                    knee_hover,
                    if pse_bypassed { MUTED } else { PSE_BLUE },
                    pse_bypassed,
                );
                let handle_hover = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, handle_hit))
                    || self.drag == Some(Target::Global(1));
                d.grab_bar(
                    handle,
                    if pse_bypassed {
                        MUTED
                    } else if handle_hover {
                        TEXT
                    } else {
                        PSE_BLUE
                    },
                );

                let gate_val_str = if p1.value() <= -79.9 {
                    "OFF".to_owned()
                } else {
                    p1.normalized_value_to_string(p1.unmodulated_normalized_value(), true)
                };
                d.text_centered(
                    rect_center_x(self.pse_main_thresh_slider_rect()),
                    meter_value_y(),
                    &gate_val_str,
                    12.0,
                    if pse_bypassed { MUTED } else { PSE_BLUE },
                );
                let depth_val = self
                    .param(7)
                    .normalized_value_to_string(self.param(7).unmodulated_normalized_value(), true);
                d.text_centered(
                    rect_center_x(self.pse_main_gr_meter_rect()),
                    meter_value_y(),
                    &depth_val,
                    11.0,
                    if pse_bypassed { MUTED } else { PSE_BLUE },
                );
            }
        }
        if pse_ctrl_off > -pw && pse_ctrl_off < pw {
            d.offset_x = pse_ctrl_off;
            {
                for (i, label) in [(8, "HYSTERESIS"), (2, "VOICE DET"), (10, "TIME")] {
                    let p = self.param(i);
                    let value = if i == 10 {
                        format_pse_time(p.value() as f64, self.params.pse_peak.value())
                    } else {
                        p.normalized_value_to_string(p.unmodulated_normalized_value(), true)
                    };
                    d.knob(
                        self.pse_knob_rect(i),
                        label,
                        &value,
                        p.unmodulated_normalized_value(),
                        if i == 2 { TEAL } else { PSE_BLUE },
                        pse_bypassed,
                    );
                }

                let time_r = self.pse_knob_rect(10);
                let det_r = self.pse_detect_mode_rect();
                let joined_card = (
                    time_r.0 - 2.0,
                    time_r.1 - 2.0,
                    time_r.2 + 4.0,
                    (det_r.1 + det_r.3) - time_r.1 + 4.0,
                );
                d.outline(joined_card, LINE);
                let is_peak = self.params.pse_peak.value();
                let det_hover = !pse_bypassed
                    && self
                        .idle_hover()
                        .is_some_and(|(hx, hy)| inside(hx, hy, det_r));
                d.button(
                    det_r,
                    if is_peak { "PEAK" } else { "RMS" },
                    true,
                    if pse_bypassed {
                        MUTED
                    } else if det_hover {
                        TEXT
                    } else {
                        PSE_BLUE
                    },
                );

                let speech_r = (px + 68.0, 356.0, 56.0, 86.0);
                d.text_centered(
                    speech_r.0 + speech_r.2 * 0.5,
                    speech_r.1 + 22.0,
                    "SPEECH",
                    9.2,
                    if pse_bypassed { MUTED } else { TEXT },
                );
                let speech_env = self
                    .shared
                    .speech_env
                    .load(Ordering::Relaxed)
                    .clamp(0.0, 1.0);
                let bar_x = speech_r.0 + 4.0;
                let bar_y = speech_r.1 + 40.0;
                let bar_w = speech_r.2 - 8.0;
                let bar_h = 10.0;
                d.rect(bar_x, bar_y, bar_w, bar_h, rgb(28, 33, 40));
                d.outline((bar_x, bar_y, bar_w, bar_h), LINE);
                if !pse_bypassed && speech_env > 0.02 {
                    d.rect(
                        bar_x + 1.0,
                        bar_y + 1.0,
                        (bar_w - 2.0) * speech_env,
                        bar_h - 2.0,
                        TEAL,
                    );
                }
                let status_text = if pse_bypassed || self.params.pse_voice_det.value() <= 0.0 {
                    "OFF"
                } else if speech_env > 0.35 {
                    "VOICE"
                } else if speech_env > 0.05 {
                    "DETECT"
                } else {
                    "IDLE"
                };
                d.text_centered(
                    speech_r.0 + speech_r.2 * 0.5,
                    speech_r.1 + 68.0,
                    status_text,
                    10.0,
                    if pse_bypassed || status_text == "OFF" {
                        MUTED
                    } else if status_text == "VOICE" {
                        TEAL
                    } else {
                        PSE_BLUE
                    },
                );
            }
        }
        d.reset_scissor();
        d.offset_x = 0.0;

        d.rect(dx, dy, dw, dh, PANEL);
        d.outline((dx, dy, dw, dh), LINE);
        let dyn_power_r = self.dyn_power_button_rect();
        let dyn_hovered = self
            .idle_hover()
            .is_some_and(|(hx, hy)| inside(hx, hy, dyn_power_r));
        let dyn_click = self.dyn_bypass_anim.step();
        d.bypass_button(dyn_power_r, comp_bypassed, GOLD, dyn_hovered, dyn_click);
        d.text(
            dx + 44.0,
            module_title_y(15.0),
            "COMP",
            15.0,
            if comp_bypassed { MUTED } else { TEXT },
        );
        let dyn_cog_r = self.dyn_cog_button_rect();
        draw_module_page_button(
            &mut d,
            dyn_cog_r,
            self.dyn_page == DynPage::Controls,
            comp_bypassed,
            !comp_bypassed
                && self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, dyn_cog_r)),
            GOLD,
        );

        d.scissor(dx, dy, dw, dh);
        let dyn_eased = quintic_page_progress(self.dyn_anim_progress.get());
        let dyn_main_off = (0.0 - dyn_eased) * dw;
        let dyn_ctrl_off = (1.0 - dyn_eased) * dw;
        if dyn_main_off > -dw && dyn_main_off < dw {
            d.offset_x = dyn_main_off;
            {
                let (sx, sy, sw, sh) = self.dyn_main_thresh_slider_rect();
                let (gx_m, gy_m, gw_m, gh_m) = self.dyn_main_gr_meter_rect();
                d.rect(sx, sy, sw, sh, LINE);
                d.rect(gx_m, gy_m, gw_m, gh_m, LINE);

                let sc_level = if comp_bypassed {
                    -90.0
                } else {
                    self.shared.sc_level.load(Ordering::Relaxed) as f64
                };
                let sc_db = sc_level.clamp(-48.0, 0.0);
                let sig_frac = ((sc_db + 48.0) / 48.0) as f32;
                let sig_h = sh * sig_frac;
                let sig_top_y = sy + sh - sig_h;

                let p0 = self.param(0);
                let thresh_norm = p0.unmodulated_normalized_value();
                let thresh_y = sy + sh * thresh_norm;

                if !comp_bypassed && sig_h > 0.5 {
                    let below_thresh_top = sig_top_y.max(thresh_y);
                    let below_thresh_h = (sy + sh) - below_thresh_top;
                    if below_thresh_h > 0.0 {
                        d.rect(
                            sx,
                            below_thresh_top,
                            sw,
                            below_thresh_h,
                            C::rgb(70, 160, 140),
                        );
                    }
                    if sig_top_y < thresh_y {
                        d.rect(sx, sig_top_y, sw, thresh_y - sig_top_y, GOLD);
                    }
                }

                let gr = if comp_bypassed {
                    0.0
                } else {
                    self.shared.gr.load(Ordering::Relaxed)
                };
                let gr_uncapped = if comp_bypassed {
                    0.0
                } else {
                    self.shared.gr_uncapped.load(Ordering::Relaxed)
                };
                let depth_norm = self.param(14).unmodulated_normalized_value();
                let gr = catch_gr_to_depth(gr, depth_norm * GR_METER_DB);
                let (gr_bar_h, gr_uncapped_h) = gr_meter_bar_heights(gr, gr_uncapped, gh_m);
                if !comp_bypassed && gr_uncapped_h > gr_bar_h + 0.5 {
                    d.rect(gx_m, gy_m + gr_bar_h, gw_m, gr_uncapped_h - gr_bar_h, MUTED);
                }
                if !comp_bypassed && gr_bar_h > 0.5 {
                    d.rect(gx_m, gy_m, gw_m, gr_bar_h, GOLD);
                }

                let depth_y = gy_m + gh_m * depth_norm;
                let depth_handle = self.dyn_main_depth_handle_rect(depth_y);
                let depth_hover = !comp_bypassed
                    && (self
                        .idle_hover()
                        .is_some_and(|(hx, hy)| inside(hx, hy, depth_handle))
                        || self.drag == Some(Target::Global(14)));
                let handle_color = if comp_bypassed {
                    MUTED
                } else if depth_hover {
                    TEXT
                } else {
                    GOLD
                };
                draw_gr_depth_handle(&mut d, gx_m, gw_m, depth_y, handle_color);

                let handle = self.dyn_main_thresh_handle_rect(thresh_y);
                let handle_hit = (
                    handle.0 - 4.0,
                    handle.1 - 2.0,
                    handle.2 + 8.0,
                    handle.3 + 4.0,
                );
                let knee_norm = (self.params.comp_knee.value() / 20.0).clamp(0.0, 1.0);
                let knee_hover = !comp_bypassed
                    && (self.idle_hover().is_some_and(|(hx, hy)| {
                        inside(hx, hy, self.dyn_main_knee_rect(thresh_y, 0.0))
                            && !inside(hx, hy, handle_hit)
                    }) || matches!(self.drag, Some(Target::CompKnee { .. })));
                let bulge_w = knee_pulse_bulge(&self.knee_bulge, knee_hover);
                draw_knee_zone(
                    &mut d,
                    self.dyn_main_knee_rect(thresh_y, 0.0),
                    bulge_w,
                    knee_norm,
                    knee_hover,
                    if comp_bypassed { MUTED } else { GOLD },
                    comp_bypassed,
                );
                let handle_hover = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, handle_hit))
                    || self.drag == Some(Target::Global(0));
                d.grab_bar(
                    handle,
                    if comp_bypassed {
                        MUTED
                    } else if handle_hover {
                        TEXT
                    } else {
                        GOLD
                    },
                );

                let thresh_val_str =
                    p0.normalized_value_to_string(p0.unmodulated_normalized_value(), true);
                d.text_centered(
                    rect_center_x(self.dyn_main_thresh_slider_rect()),
                    meter_value_y(),
                    &thresh_val_str,
                    12.0,
                    if comp_bypassed { MUTED } else { GOLD },
                );
                let depth_val = self.param(14).normalized_value_to_string(
                    self.param(14).unmodulated_normalized_value(),
                    true,
                );
                d.text_centered(
                    rect_center_x(self.dyn_main_gr_meter_rect()),
                    meter_value_y(),
                    &depth_val,
                    11.0,
                    if comp_bypassed { MUTED } else { GOLD },
                );
            }
        }
        if dyn_ctrl_off > -dw && dyn_ctrl_off < dw {
            d.offset_x = dyn_ctrl_off;
            {
                for (i, label, color) in [
                    (5, "ATTACK", GOLD),
                    (11, "RELEASE", GOLD),
                    (6, "RATIO", GOLD),
                    (3, "DRY", TEAL),
                    (4, "WET", GOLD),
                ] {
                    let p = self.param(i);
                    let value =
                        p.normalized_value_to_string(p.unmodulated_normalized_value(), true);
                    d.knob(
                        self.global_rect(i),
                        label,
                        &value,
                        p.unmodulated_normalized_value(),
                        color,
                        comp_bypassed,
                    );
                }
            }
        }
        d.reset_scissor();
        d.offset_x = 0.0;

        let wall_bypassed = !self.params.wall_on.value();
        let (wx, wy, ww, wh) = self.wall_bounds();
        d.rect(wx, wy, ww, wh, PANEL);
        d.outline((wx, wy, ww, wh), LINE);
        let wall_power_r = self.wall_power_button_rect();
        let wall_hovered = self
            .idle_hover()
            .is_some_and(|(hx, hy)| inside(hx, hy, wall_power_r));
        let wall_click = self.wall_bypass_anim.step();
        d.bypass_button(
            wall_power_r,
            wall_bypassed,
            WALL_COLOR,
            wall_hovered,
            wall_click,
        );
        d.text(
            wx + 44.0,
            module_title_y(15.0),
            "WALL",
            15.0,
            if wall_bypassed { MUTED } else { TEXT },
        );
        let wall_cog_r = self.wall_cog_button_rect();
        draw_module_page_button(
            &mut d,
            wall_cog_r,
            self.wall_page == WallPage::Controls,
            wall_bypassed,
            !wall_bypassed
                && self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, wall_cog_r)),
            WALL_COLOR,
        );

        d.scissor(wx, wy, ww, wh);
        let wall_eased = quintic_page_progress(self.wall_anim_progress.get());
        let wall_main_off = (0.0 - wall_eased) * ww;
        let wall_ctrl_off = (1.0 - wall_eased) * ww;
        if wall_main_off > -ww && wall_main_off < ww {
            d.offset_x = wall_main_off;
            {
                let (sx, sy, sw, sh) = self.wall_main_thresh_slider_rect();
                d.rect(sx, sy, sw, sh, LINE);

                let wall_level = if wall_bypassed {
                    -90.0
                } else {
                    self.shared.wall_level.load(Ordering::Relaxed) as f64
                };
                let wall_db = wall_level.clamp(-48.0, 0.0);
                let sig_frac = ((wall_db + 48.0) / 48.0) as f32;
                let sig_h = sh * sig_frac;
                let sig_top_y = sy + sh - sig_h;

                let p18 = self.param(18);
                let thresh_norm = p18.unmodulated_normalized_value();
                let thresh_y = sy + sh * thresh_norm;

                if !wall_bypassed && sig_h > 0.5 {
                    let below_thresh_top = sig_top_y.max(thresh_y);
                    let below_thresh_h = (sy + sh) - below_thresh_top;
                    if below_thresh_h > 0.0 {
                        d.rect(
                            sx,
                            below_thresh_top,
                            sw,
                            below_thresh_h,
                            C::rgb(70, 160, 140),
                        );
                    }
                    if sig_top_y < thresh_y {
                        d.rect(sx, sig_top_y, sw, thresh_y - sig_top_y, WALL_COLOR);
                    }
                }

                let handle = self.wall_main_thresh_handle_rect(thresh_y);
                let handle_hit = (
                    handle.0 - 4.0,
                    handle.1 - 2.0,
                    handle.2 + 8.0,
                    handle.3 + 4.0,
                );
                let handle_hover = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, handle_hit))
                    || self.drag == Some(Target::Global(18));
                d.grab_bar(
                    handle,
                    if wall_bypassed {
                        MUTED
                    } else if handle_hover {
                        TEXT
                    } else {
                        WALL_COLOR
                    },
                );

                let thresh_val =
                    p18.normalized_value_to_string(p18.unmodulated_normalized_value(), true);
                d.text_centered(
                    rect_center_x(self.wall_main_thresh_slider_rect()),
                    meter_value_y(),
                    &thresh_val,
                    12.0,
                    if wall_bypassed { MUTED } else { WALL_COLOR },
                );
            }
        }
        if wall_ctrl_off > -ww && wall_ctrl_off < ww {
            d.offset_x = wall_ctrl_off;
            {
                for (i, label) in [(16, "EVEN"), (17, "ODD")] {
                    let p = self.param(i);
                    let value =
                        p.normalized_value_to_string(p.unmodulated_normalized_value(), true);
                    d.knob(
                        self.wall_knob_rect(i),
                        label,
                        &value,
                        p.unmodulated_normalized_value(),
                        WALL_COLOR,
                        wall_bypassed,
                    );
                }
            }
        }
        d.reset_scissor();
        d.offset_x = 0.0;

        d.line(32.0, FOOTER_LINE_Y, UI_W - 32.0, FOOTER_LINE_Y, LINE, 1.0);
        d.button(
            THEME_BUTTON,
            if d.light { "DARK MODE" } else { "LIGHT MODE" },
            false,
            TEXT,
        );
        d.button(
            PROCESS_BUTTON,
            &format!("{} ▾", config.mode.label()),
            false,
            GOLD,
        );
        if config.mode == ProcessingMode::LinearPhase {
            d.button(
                resolution_button_rect(),
                &format!("{} ▾", config.resolution.label()),
                false,
                GOLD,
            );
        }
        let active = Config::decode(self.shared.active_config.load(Ordering::Relaxed));
        let latency = self.shared.latency.load(Ordering::Relaxed);
        let status = if active != config {
            "APPLYING...".to_string()
        } else if config.mode == ProcessingMode::LinearPhase {
            format!("{:.1} ms / STATIC EQ", latency as f64 * 1000.0 / sr)
        } else if latency > 0 {
            format!("{:.2} ms", latency as f64 * 1000.0 / sr)
        } else {
            String::new()
        };
        if !status.is_empty() {
            let status_x = if config.mode == ProcessingMode::LinearPhase {
                400.0
            } else {
                288.0
            };
            d.text(status_x, FOOTER_BTN_Y + 18.0, &status, 10.0, MUTED);
        }
        d.text(
            self.footer_comp_label_x(),
            FOOTER_BTN_Y + 19.0,
            "Compression:",
            12.0,
            if comp_bypassed { MUTED } else { TEXT },
        );
        d.button(
            self.footer_comp_routing_rect(),
            if is_pre { "PRE" } else { "POST" },
            true,
            if is_pre { TEAL } else { GOLD },
        );
        d.button(
            self.footer_auto_rect(),
            "AUTO",
            self.params.auto_makeup.value(),
            if comp_bypassed { MUTED } else { GOLD },
        );
        d.button(
            self.footer_link_rect(),
            "LINK",
            self.params.stereo_link.value(),
            if comp_bypassed { MUTED } else { GOLD },
        );

        let r = output_gain_rect();
        d.rect(r.0, r.1, r.2, r.3, PANEL);
        let out_hover = self.idle_hover().is_some_and(|(hx, hy)| inside(hx, hy, r));
        let out_dragging = self.drag == Some(Target::Global(15));
        d.outline(
            r,
            if out_hover || out_dragging {
                GOLD
            } else {
                LINE
            },
        );

        let p_out = self.param(15);
        let out_norm = p_out.unmodulated_normalized_value();
        let out_val = p_out.value();
        let val_str = if out_val.abs() < 0.05 {
            "0.0 dB".to_string()
        } else {
            format!("{:+0.1} dB", out_val)
        };

        d.text(r.0 + 8.0, r.1 + 13.0, "OUTPUT", 9.5, MUTED);
        let val_r = output_gain_value_rect();
        if let Some(edit) = &self.edit {
            if edit.target == ValueTarget::Global(15) {
                d.value_edit(edit, GOLD);
            } else {
                d.text_centered(val_r.0 + val_r.2 * 0.5, r.1 + 13.0, &val_str, 10.5, TEXT);
            }
        } else {
            d.text_centered(val_r.0 + val_r.2 * 0.5, r.1 + 13.0, &val_str, 10.5, TEXT);
            if out_hover
                && self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, val_r))
            {
                d.value_underline(val_r, GOLD);
            }
        }

        let bar_x = r.0 + 8.0;
        let bar_w = r.2 - 16.0;
        let bar_y = r.1 + 19.0;
        let bar_h = 4.0;
        d.rect(bar_x, bar_y, bar_w, bar_h, LINE);
        let mid_x = bar_x + bar_w * 0.5;
        if out_norm > 0.5 {
            let fill_w = bar_w * (out_norm - 0.5);
            d.rect(mid_x, bar_y, fill_w, bar_h, GOLD);
        } else if out_norm < 0.5 {
            let fill_w = bar_w * (0.5 - out_norm);
            d.rect(mid_x - fill_w, bar_y, fill_w, bar_h, GOLD);
        }
        d.line(mid_x, bar_y - 1.0, mid_x, bar_y + bar_h + 1.0, TEXT, 1.0);
        if let Some(menu) = self.menu {
            let menu_info = self
                .selected
                .and_then(|id| self.find_band(id))
                .map(|b| {
                    (
                        menu.rect_at(&b, self.graph_db, gx, gw),
                        b.shape,
                        b.order as usize,
                    )
                })
                .or_else(|| {
                    self.selected_lift().map(|b| {
                        (
                            menu.rect_lift_at(&b, self.graph_db, gx, gw),
                            b.shape,
                            b.order as usize,
                        )
                    })
                });
            if let Some((r, shape, order)) = menu_info {
                d.rect(r.0, r.1, r.2, r.3, PANEL);
                for row in 0..menu.count() {
                    let (label, selected) = match menu {
                        BandMenu::Shape => {
                            (Shape::ALL[row].name().to_string(), shape == Shape::ALL[row])
                        }
                        BandMenu::Order => (
                            format!("{} dB/oct  /  order {}", (row + 1) * 6, row + 1),
                            order == row + 1,
                        ),
                    };
                    let row_y = r.1 + row as f32 * 24.0;
                    if selected
                        || self
                            .idle_hover()
                            .is_some_and(|(x, y)| inside(x, y, (r.0, row_y, r.2, 24.0)))
                    {
                        d.rect(r.0, row_y, r.2, 24.0, LINE);
                    }
                    d.text(
                        r.0 + 10.0,
                        row_y + 16.0,
                        &label,
                        11.0,
                        if selected { GOLD } else { TEXT },
                    );
                }
            }
        }
        if let Some(resolution) = self.processing_menu {
            let r = processing_menu_rect(resolution);
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            d.outline(r, LINE);
            let count = if resolution {
                RESOLUTIONS.len()
            } else {
                MODES.len()
            };
            for i in 0..count {
                let (label, selected) = if resolution {
                    (RESOLUTIONS[i].label(), config.resolution == RESOLUTIONS[i])
                } else {
                    (MODES[i].label(), config.mode == MODES[i])
                };
                let row = (r.0, r.1 + i as f32 * 24.0, r.2, 24.0);
                if selected || self.idle_hover().is_some_and(|(x, y)| inside(x, y, row)) {
                    d.rect(row.0, row.1, row.2, row.3, LINE);
                }
                d.text(
                    row.0 + 10.0,
                    row.1 + 16.0,
                    label,
                    10.0,
                    if selected { GOLD } else { TEXT },
                );
            }
        }
        if self.edit.is_none()
            && self.menu.is_none()
            && !self.scale_menu
            && self.processing_menu.is_none()
        {
            if let Some((target, r)) = self.idle_hover().and_then(|(x, y)| self.value_at(x, y)) {
                d.line(
                    r.0 + 4.0,
                    r.1 + r.3 - 1.0,
                    r.0 + r.2 - 4.0,
                    r.1 + r.3 - 1.0,
                    self.value_color(target),
                    1.0,
                );
            }
        }
        if let Some(edit) = &self.edit {
            let accent = self.value_color(edit.target);
            let r = edit.rect;
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            d.outline(
                r,
                if edit.invalid {
                    rgb(238, 110, 95)
                } else {
                    accent
                },
            );
            // Keep a long entry's caret in the field while editing.
            let capacity = ((r.2 - 8.0) / 6.6) as usize;
            let start = edit.cursor.saturating_sub(capacity);
            let end = (start + capacity).min(edit.text.len());
            let selection = edit.selection();
            let left = selection.start.max(start).min(end);
            let right = selection.end.min(end).max(left);
            d.rect(
                r.0 + 4.0 + (left - start) as f32 * 6.6,
                r.1 + 2.0,
                (right - left) as f32 * 6.6,
                r.3 - 4.0,
                C { a: 0.25, ..accent },
            );
            d.text(
                r.0 + 4.0,
                r.1 + r.3 * 0.5 + 4.0,
                &edit.text[start..end],
                11.0,
                TEXT,
            );
            let caret_x = r.0 + 4.0 + (edit.cursor - start) as f32 * 6.6;
            d.line(caret_x, r.1 + 3.0, caret_x, r.1 + r.3 - 3.0, accent, 1.0);
        }
    }
}
fn draw_hud_panel(d: &mut Draw, x: f32, y: f32, w: f32, h: f32, color: C) {
    d.rounded_rect(x, y, w, h, HUD_RADIUS, C::rgba(12, 12, 14, 214));
    d.outline_rounded(x, y, w, h, HUD_RADIUS, C::rgba(40, 44, 50, 180), 1.0);
    d.line(x + 8.0, y, x + w - 8.0, y, color, 1.5);
}
fn draw_hud_chrome(
    d: &mut Draw,
    bx: f32,
    by: f32,
    bw: f32,
    enabled: bool,
    is_solo: bool,
    color: C,
    shape_label: &str,
) {
    let bypass = hud_bypass_rect(bx, by);
    let solo = hud_solo_rect(bx, by);
    let close = hud_close_rect(bx, by);
    let shape = hud_shape_rect(bx, by, bw);
    d.rounded_rect(
        bypass.0,
        bypass.1,
        bypass.2,
        bypass.3,
        4.0,
        C::rgba(35, 40, 48, 150),
    );
    d.power_icon(
        bypass.0 + bypass.2 * 0.5,
        bypass.1 + bypass.3 * 0.5,
        if enabled { color } else { MUTED },
    );
    d.rounded_rect(
        solo.0,
        solo.1,
        solo.2,
        solo.3,
        4.0,
        if is_solo {
            C::rgba(110, 85, 30, 180)
        } else {
            C::rgba(35, 40, 48, 150)
        },
    );
    d.headphones(
        solo.0 + solo.2 * 0.5,
        solo.1 + solo.3 * 0.5,
        if is_solo { GOLD } else { MUTED },
    );
    d.rounded_rect(
        close.0,
        close.1,
        close.2,
        close.3,
        4.0,
        C::rgba(35, 40, 48, 150),
    );
    d.close_icon(close.0 + close.2 * 0.5, close.1 + close.3 * 0.5, MUTED);
    d.rounded_rect(
        shape.0,
        shape.1,
        shape.2,
        shape.3,
        4.0,
        C::rgba(35, 40, 48, 150),
    );
    d.text(
        shape.0 + 8.0,
        shape.1 + shape.3 * 0.5 + 4.0,
        shape_label,
        10.0,
        if enabled { TEXT } else { MUTED },
    );
}
fn hz(f: f64) -> String {
    if f >= 1000.0 {
        format!("{:.2} kHz", f / 1000.0)
    } else {
        format!("{:.0} Hz", f)
    }
}
fn band_hud(
    d: &mut Draw,
    b: &Band,
    is_solo: bool,
    color: C,
    range: f64,
    _linear: bool,
    gx: f32,
    gw: f32,
    dyn_hover: bool,
    dyn_click: f32,
) {
    let g = hud_geom_for_at(b, range, gx, gw);
    draw_hud_panel(d, g.bx, g.by, g.bw, g.bh, color);
    draw_hud_chrome(
        d,
        g.bx,
        g.by,
        g.bw,
        b.enabled,
        is_solo,
        color,
        &format!("{} ▾", b.shape.name()),
    );
    for (i, label, value, active) in [
        (0, "FREQ", hz(b.freq), true),
        (
            1,
            "GAIN",
            if b.shape.has_gain() {
                format!("{:+.2}", b.gain)
            } else {
                "—".into()
            },
            b.shape.has_gain(),
        ),
        (
            2,
            "Q",
            if b.shape.is_cut() && b.order == 1 {
                "—".into()
            } else {
                format!("{:.2}", b.q)
            },
            !(b.shape.is_cut() && b.order == 1),
        ),
    ] {
        let r = hud_value_rect_at(b, range, i, gx, gw);
        d.text(r.0 + 2.0, r.1 + 10.0, label, 8.0, MUTED);
        d.text(
            r.0 + 2.0,
            r.1 + 24.0,
            &value,
            11.0,
            if b.enabled && active { color } else { MUTED },
        );
    }
    if b.shape.is_cut() {
        let order = hud_order_rect(g.bx, g.by);
        d.rounded_rect(
            order.0,
            order.1,
            order.2,
            order.3,
            4.0,
            C::rgba(35, 40, 48, 150),
        );
        d.text(
            order.0 + 6.0,
            order.1 + 13.0,
            &format!("{} dB/oct ▾", b.order * 6),
            9.0,
            if b.enabled { TEXT } else { MUTED },
        );
    }
    if g.show_dyn {
        draw_hud_panel(d, g.dyn_x, g.by, g.dyn_w, g.bh, color);
        for (i, label, value, n, bipolar) in [
            (
                0,
                "THRESH",
                format!("{:.1}", b.threshold),
                ((b.threshold + 60.0) / 60.0) as f32,
                false,
            ),
            (
                1,
                "RANGE",
                format!("{:.1}", b.range),
                ((b.range + 24.0) / 48.0) as f32,
                true,
            ),
            (
                2,
                "RATIO",
                format!("{:.1}:1", b.ratio),
                ((b.ratio - 1.0) / 19.0) as f32,
                false,
            ),
            (
                3,
                "ATTACK",
                format!("{:.1} ms", b.attack),
                (b.attack / 0.1).log(2000.0) as f32,
                false,
            ),
            (
                4,
                "RELEASE",
                format!("{:.0} ms", b.release),
                (b.release / 10.0).log(200.0) as f32,
                false,
            ),
        ] {
            let r = hud_dyn_field_rect_at(b, range, i, gx, gw);
            let val_r = hud_dyn_value_rect_at(b, range, i, gx, gw);
            d.text(r.0 + 8.0, r.1 + 13.0, label, 8.5, MUTED);
            d.text_centered(
                val_r.0 + val_r.2 * 0.5,
                r.1 + 13.0,
                &value,
                10.5,
                if b.enabled { color } else { MUTED },
            );
            let bar_x = r.0 + 8.0;
            let bar_w = r.2 - 16.0;
            let bar_y = r.1 + 19.0;
            let bar_h = 4.0;
            d.rect(bar_x, bar_y, bar_w, bar_h, LINE);
            let n = n.clamp(0.0, 1.0);
            let fill_color = if b.enabled { color } else { MUTED };
            if bipolar {
                let mid_x = bar_x + bar_w * 0.5;
                if n > 0.5 {
                    d.rect(mid_x, bar_y, bar_w * (n - 0.5), bar_h, fill_color);
                } else if n < 0.5 {
                    let fill_w = bar_w * (0.5 - n);
                    d.rect(mid_x - fill_w, bar_y, fill_w, bar_h, fill_color);
                }
                d.line(
                    mid_x,
                    bar_y - 1.0,
                    mid_x,
                    bar_y + bar_h + 1.0,
                    if b.enabled { TEXT } else { MUTED },
                    1.0,
                );
            } else {
                d.rect(bar_x, bar_y, bar_w * n, bar_h, fill_color);
            }
        }
    }
    if g.show_dyn_btn {
        let (cx, cy) = hud_dyn_btn_center(g);
        let on = b.dynamic;
        let mut fill = C::rgba(12, 12, 14, 230);
        if dyn_click > 0.01 {
            fill.a = 1.0;
        }
        d.circle(cx, cy, HUD_DYN_R, fill, true);
        let ring = if !b.enabled {
            MUTED
        } else if on {
            color
        } else if dyn_hover {
            TEXT
        } else {
            MUTED
        };
        d.circle(cx, cy, HUD_DYN_R, ring, false);
        d.text_centered(cx, cy + 3.0, "DYN", 8.0, ring);
    }
}

fn band_hud_lift(
    d: &mut Draw,
    b: &LiftBand,
    is_solo: bool,
    color: C,
    range: f64,
    gx: f32,
    gw: f32,
) {
    let (bx, by, bw, bh) = hud_rect_for_lift_at(b, range, gx, gw);
    draw_hud_panel(d, bx, by, bw, bh, color);
    draw_hud_chrome(
        d,
        bx,
        by,
        bw,
        b.enabled,
        is_solo,
        color,
        &format!("{} LIFT ▾", b.shape.uppercase_name()),
    );
    for (i, label, value, active) in [
        (0, "FREQ", hz(b.freq), true),
        (
            1,
            "GAIN",
            if b.gain <= -99.5 {
                "-inf".into()
            } else {
                format!("{:.1}", b.gain)
            },
            true,
        ),
        (
            2,
            "Q",
            if b.shape.is_cut() && b.order == 1 {
                "—".into()
            } else {
                format!("{:.2}", b.q)
            },
            !(b.shape.is_cut() && b.order == 1),
        ),
    ] {
        let r = hud_value_rect_lift_at(b, range, i, gx, gw);
        d.text(r.0 + 2.0, r.1 + 10.0, label, 8.0, MUTED);
        d.text(
            r.0 + 2.0,
            r.1 + 24.0,
            &value,
            11.0,
            if b.enabled && active { color } else { MUTED },
        );
    }
    if b.shape.is_cut() {
        let order = hud_order_rect(bx, by);
        d.rounded_rect(
            order.0,
            order.1,
            order.2,
            order.3,
            4.0,
            C::rgba(35, 40, 48, 150),
        );
        d.text(
            order.0 + 6.0,
            order.1 + 13.0,
            &format!("{} dB/oct ▾", b.order * 6),
            9.0,
            if b.enabled { TEXT } else { MUTED },
        );
    }
}

fn draw_signature(d: &mut Draw, font: Option<FontId>) {
    let shift = UI_W - 1120.0;
    let mut paint = Paint::linear_gradient(
        d.ox + (800.0 + shift) * d.s,
        d.oy + 13.0 * d.s,
        d.ox + (810.0 + shift) * d.s,
        d.oy + 63.0 * d.s,
        d.color(C::rgb(255, 230, 163)),
        d.color(C::rgb(176, 119, 33)),
    );
    if let Some(f) = font {
        paint.set_font(&[f]);
    }
    paint.set_font_size(44.0 * d.s);
    let _ = d.c.fill_text(
        d.ox + (794.0 + shift) * d.s,
        d.oy + 51.0 * d.s,
        "Damian Birdsey",
        &paint,
    );
    let mut underline = Path::new();
    for (i, (x, y)) in [
        (821.0 + shift, 67.0),
        (870.0 + shift, 64.0),
        (972.0 + shift, 65.0),
        (1073.0 + shift, 59.0),
    ]
    .into_iter()
    .enumerate()
    {
        if i == 0 {
            underline.move_to(d.ox + x * d.s, d.oy + y * d.s);
        } else {
            underline.line_to(d.ox + x * d.s, d.oy + y * d.s);
        }
    }
    let gold = d.color(GOLD);
    let mut paint = Paint::linear_gradient(
        d.ox + (821.0 + shift) * d.s,
        d.oy,
        d.ox + (1073.0 + shift) * d.s,
        d.oy,
        C { a: 0.0, ..gold },
        gold,
    );
    paint.set_line_width(d.s);
    d.c.stroke_path(&underline, &paint);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hud_rect_for(b: &Band, range: f64) -> (f32, f32, f32, f32) {
        hud_rect_for_at(b, range, GX, GW)
    }
    fn hud_value_rect(b: &Band, range: f64, i: usize) -> (f32, f32, f32, f32) {
        hud_value_rect_at(b, range, i, GX, GW)
    }
    fn hud_rect_for_lift(b: &LiftBand, range: f64) -> (f32, f32, f32, f32) {
        hud_rect_for_lift_at(b, range, GX, GW)
    }
    fn hud_value_rect_lift(b: &LiftBand, range: f64, i: usize) -> (f32, f32, f32, f32) {
        hud_value_rect_lift_at(b, range, i, GX, GW)
    }

    #[test]
    fn entries_accept_units_and_reject_invalid_numbers() {
        for (input, target, expected) in [
            ("1.25 kHz", ValueTarget::Band(0), 1250.0),
            ("20k", ValueTarget::Band(0), 20000.0),
            ("178 Hz", ValueTarget::Band(0), 178.0),
            ("-11.75 dB", ValueTarget::Band(1), -11.75),
            ("1.77", ValueTarget::Band(2), 1.77),
            ("4:1", ValueTarget::Band(4), 4.0),
            ("0.1 ms", ValueTarget::Band(5), 0.1),
            ("1.4 s", ValueTarget::Band(6), 1400.0),
            ("-20 dB", ValueTarget::Band(3), -20.0),
            ("12 dB", ValueTarget::Band(7), 12.0),
            ("8:1", ValueTarget::Global(6), 8.0),
            ("50 %", ValueTarget::Global(2), 50.0),
            ("25 %", ValueTarget::Global(3), 25.0),
            ("OFF", ValueTarget::Global(1), -80.0),
            ("100 ms", ValueTarget::Global(10), 1.0),
            ("c", ValueTarget::Global(10), 2.0),
            ("1.5 s", ValueTarget::Global(10), 4.0),
            ("2.5", ValueTarget::Global(10), 2.5),
            ("10 ms", ValueTarget::Global(5), 10.0),
            ("1.2 s", ValueTarget::Global(11), 1200.0),
            ("250 ms", ValueTarget::Global(11), 250.0),
        ] {
            assert_eq!(parse_value(input, target), Some(expected), "{input}");
        }
        for input in ["", "-", "NaN", "inf", "1e999", "12garbage", "2 ms", "--3"] {
            assert_eq!(parse_value(input, ValueTarget::Band(0)), None, "{input}");
        }
    }

    #[test]
    fn view_scale_rejects_empty_and_non_finite_bounds() {
        assert_eq!(
            view_scale(BoundingBox {
                x: 0.0,
                y: 0.0,
                w: UI_W,
                h: UI_H
            }),
            Some(1.0)
        );
        assert!(view_scale(BoundingBox {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: UI_H
        })
        .is_none());
        assert!(view_scale(BoundingBox {
            x: 0.0,
            y: 0.0,
            w: UI_W,
            h: 0.0
        })
        .is_none());
        assert!(view_scale(BoundingBox {
            x: 0.0,
            y: 0.0,
            w: f32::NAN,
            h: UI_H
        })
        .is_none());
        assert!(view_scale(BoundingBox {
            x: 0.0,
            y: 0.0,
            w: f32::INFINITY,
            h: UI_H
        })
        .is_none());
        assert!(view_scale(BoundingBox {
            x: 0.0,
            y: 0.0,
            w: 4.0,
            h: 4.0
        })
        .is_none());
    }

    #[test]
    fn graph_mapping_tracks_display_scale_without_expanding_gain_limits() {
        for range in SCALES {
            assert_eq!(db_y(range, range), GY);
            assert_eq!(db_y(-range, range), GY + GH);
            assert_eq!(db_y(0.0, range), GY + GH * 0.5);
            for gain in [-24.0_f64, -12.0, 0.0, 12.0, 24.0] {
                if gain.abs() <= range {
                    assert!((y_db(db_y(gain, range), range) - gain).abs() < 0.0001);
                }
            }
            assert!(y_db(GY - GH, range) <= 24.0);
            assert!(y_db(GY + GH * 2.0, range) >= -24.0);
        }
    }

    #[test]
    fn hud_stays_in_graph_and_leaves_node_clearance_at_edges_and_all_scales() {
        for range in SCALES {
            for freq in [20.0, 178.0, 1000.0, 20000.0] {
                for gain in -24..=24 {
                    let b = Band {
                        freq,
                        gain: gain as f64,
                        ..Band::default()
                    };
                    let r = hud_rect_for(&b, range);
                    let node_y = db_y(b.gain, range);
                    assert!(r.0 >= GX && r.0 + r.2 <= GX + GW);
                    assert!(r.1 >= GY && r.1 + r.3 <= GY + GH);
                    let gap = if node_y < r.1 {
                        r.1 - node_y
                    } else {
                        node_y - r.1 - r.3
                    };
                    assert!(gap >= 26.0, "scale={range}, gain={gain}, gap={gap}");
                    let at_bottom = node_y < GY + GH * 0.5;
                    if at_bottom {
                        assert!((r.1 + r.3 - (GY + GH - 4.0)).abs() < 0.01);
                    } else {
                        assert!((r.1 - (GY + 4.0)).abs() < 0.01);
                    }
                    for i in 0..3 {
                        let value = hud_value_rect(&b, range, i);
                        assert!(inside(value.0, value.1, r));
                        assert!(inside(value.0 + value.2, value.1 + value.3, r));
                    }
                }
            }
        }
        let wide = Band {
            freq: 1000.0,
            gain: 12.0,
            dynamic: true,
            ..Band::default()
        };
        let main = hud_rect_for(&wide, 24.0);
        let dyn_overlay = hud_dyn_rect_for_at(&wide, 24.0, GX, GW).expect("dyn overlay");
        assert!((dyn_overlay.0 - (main.0 + main.2 + HUD_OVERLAY_GAP)).abs() < 0.01);
        assert_eq!(dyn_overlay.1, main.1);
        assert_eq!(dyn_overlay.3, main.3);
        assert!((main.2 - HUD_MAIN_W).abs() < 0.01);
        assert_eq!(HUD_GAIN_W, 52.0);
        assert_eq!(HUD_Q_W, 40.0);
        assert_eq!(HUD_SHAPE_W, 104.0);
        assert_eq!(HUD_FREQ_W, 86.0);
        assert_eq!(HUD_DYN_FIELD_H, 28.0);
        for i in 0..5 {
            let field = hud_dyn_field_rect_at(&wide, 24.0, i, GX, GW);
            assert!(inside(field.0, field.1, dyn_overlay));
            assert!(inside(field.0 + field.2, field.1 + field.3, dyn_overlay));
            assert_eq!(field.3, HUD_DYN_FIELD_H);
            let value = hud_dyn_value_rect_at(&wide, 24.0, i, GX, GW);
            assert!(inside(value.0, value.1, field));
            assert!(inside(value.0 + value.2, value.1 + value.3, field));
            assert!(value.0 > field.0 + 8.0);
        }
        let attack = hud_dyn_field_rect_at(&wide, 24.0, 3, GX, GW);
        for i in 0..3 {
            let value = hud_value_rect_at(&wide, 24.0, i, GX, GW);
            assert_eq!(value.1, attack.1);
            assert_eq!(value.3, attack.3);
        }
    }

    #[test]
    fn editing_replaces_selection_and_deletes_without_touching_the_band() {
        let mut edit = ValueEdit::new(ValueTarget::Band(0), (0.0, 0.0, 100.0, 24.0), "1000".into());
        edit.insert("1.25 kHz");
        assert_eq!(parse_value(&edit.text, edit.target), Some(1250.0));
        edit.cursor = 4;
        edit.anchor = 1;
        edit.insert(".5");
        assert_eq!(edit.text, "1.5 kHz");
        edit.erase(true);
        assert_eq!(edit.text, "1. kHz");
        edit.cursor = 0;
        edit.anchor = edit.text.len();
        edit.erase(false);
        assert!(edit.text.is_empty());
        edit.insert("NaN");
        assert_eq!(parse_value(&edit.text, edit.target), None);
        assert_eq!(edit.original, "1000");
    }

    #[test]
    fn lift_entries_accept_units_and_parse_properly() {
        for (input, target, expected) in [
            ("5 kHz", ValueTarget::Lift(0), 5000.0),
            ("12k", ValueTarget::Lift(0), 12000.0),
            ("400 Hz", ValueTarget::Lift(0), 400.0),
            ("0 dB", ValueTarget::Lift(1), 0.0),
            ("-18 dB", ValueTarget::Lift(1), -18.0),
            ("-50.5 dB", ValueTarget::Lift(1), -50.5),
            ("-100 dB", ValueTarget::Lift(1), -100.0),
            ("0.707", ValueTarget::Lift(2), 0.707),
            ("-18 dB", ValueTarget::Lift(3), -18.0),
            ("6:1", ValueTarget::Lift(4), 6.0),
            ("10 ms", ValueTarget::Lift(5), 10.0),
            ("150 ms", ValueTarget::Lift(6), 150.0),
            ("12 dB", ValueTarget::Lift(7), 12.0),
        ] {
            assert_eq!(parse_value(input, target), Some(expected), "{input}");
        }
    }

    #[test]
    fn hud_rect_for_lift_stays_in_graph() {
        for range in SCALES {
            for freq in [400.0, 1000.0, 8000.0, 20000.0] {
                for gain in [-100.0, -75.0, -50.0, -24.0, -12.0, 0.0] {
                    let b = LiftBand {
                        freq,
                        gain,
                        ..LiftBand::default()
                    };
                    let r = hud_rect_for_lift(&b, range);
                    assert!(r.0 >= GX && r.0 + r.2 <= GX + GW);
                    assert!(r.1 >= GY && r.1 + r.3 <= GY + GH);
                    for i in 0..3 {
                        let value = hud_value_rect_lift(&b, range, i);
                        assert!(inside(value.0, value.1, r));
                        assert!(inside(value.0 + value.2, value.1 + value.3, r));
                    }
                }
            }
        }
    }

    #[test]
    fn solo_shade_covers_sides_when_applicable() {
        let gx = 100.0;
        let gw = 800.0;
        let (bell_l, bell_r) = solo_shade_rects(Shape::Bell, 1000.0, 1.0, gx, gw);
        assert!(bell_l.is_some() && bell_r.is_some());
        let (low_l, low_r) = solo_shade_rects(Shape::LowCut, 1000.0, 0.707, gx, gw);
        assert!(low_l.is_some() && low_r.is_none());
        let (high_l, high_r) = solo_shade_rects(Shape::HighCut, 1000.0, 0.707, gx, gw);
        assert!(high_l.is_none() && high_r.is_some());
        if let Some((x, w)) = bell_l {
            assert!((x - gx).abs() < 0.01);
            assert!(w > 1.0);
            assert!(x + w < gx + gw);
        }
        let gold = GOLD;
        let grey = greyscale_darken(gold, 0.32);
        assert!((grey.r - grey.g).abs() < 1e-5 && (grey.g - grey.b).abs() < 1e-5);
        assert!(grey.r < gold.r && grey.r < gold.g);
        let hit = intersect_rect((100.0, 10.0, 50.0, 20.0), (120.0, 0.0, 80.0, 40.0));
        assert_eq!(hit, Some((120.0, 10.0, 30.0, 20.0)));
    }

    #[test]
    fn dual_eq_animation_math_and_id_ranges() {
        assert_eq!(EQ2_ID_BASE, 10_000);
        assert_eq!(SC_EQ_ID_BASE, 20_000);
        assert!(EQ2_ID_BASE < SC_EQ_ID_BASE);
        assert!(SC_EQ_ID_BASE < LIFT_ID_BASE);
        assert_eq!(GRAPH_CLIP_W, EQ_W);
        assert_eq!(band_display_num(3), 3);
        assert_eq!(band_display_num(EQ2_ID_BASE + 2), 2);
        assert_eq!(band_display_num(SC_EQ_ID_BASE + 1), 1);

        let p0 = 0.0_f32;
        let eased0 = quintic_page_progress(p0);
        assert_eq!((0.0 - eased0) * GRAPH_CLIP_W, 0.0);
        assert_eq!((1.0 - eased0) * GRAPH_CLIP_W, GRAPH_CLIP_W);
        assert_eq!((2.0 - eased0) * GRAPH_CLIP_W, 2.0 * GRAPH_CLIP_W);

        let p1 = 1.0_f32;
        let eased1 = quintic_page_progress(p1);
        assert_eq!((0.0 - eased1) * GRAPH_CLIP_W, -GRAPH_CLIP_W);
        assert_eq!((1.0 - eased1) * GRAPH_CLIP_W, 0.0);
        assert_eq!((2.0 - eased1) * GRAPH_CLIP_W, GRAPH_CLIP_W);

        let p2 = 2.0_f32;
        let eased2 = quintic_page_progress(p2);
        assert_eq!((0.0 - eased2) * GRAPH_CLIP_W, -2.0 * GRAPH_CLIP_W);
        assert_eq!((1.0 - eased2) * GRAPH_CLIP_W, -GRAPH_CLIP_W);
        assert_eq!((2.0 - eased2) * GRAPH_CLIP_W, 0.0);
        assert_eq!((3.0 - eased2) * GRAPH_CLIP_W, GRAPH_CLIP_W);

        let p3 = 3.0_f32;
        let eased3 = quintic_page_progress(p3);
        assert_eq!((3.0 - eased3) * GRAPH_CLIP_W, 0.0);

        let p_mid = 0.5_f32;
        let eased_mid = quintic_page_progress(p_mid);
        assert!((eased_mid - 0.5).abs() < 1e-6);
        let p_sc_mid = 1.5_f32;
        let eased_sc = quintic_page_progress(p_sc_mid);
        assert!((eased_sc - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_lift_bin_curve_fit_smoothness() {
        let mut bins = [0.0_f64; 256];
        // Set a peak at bin 128
        bins[128] = 12.0;

        // Curve fit at bin center 128 (t = 128.5 / 256.0)
        let t_center = 128.5 / 256.0;
        let v_center = lift_bin_curve_fit(&bins, t_center);
        assert!((v_center - 12.0).abs() < 1e-6);

        // At intermediate points, curve should be continuous and non-negative
        for step in 0..=100 {
            let t = 0.45 + 0.10 * (step as f64 / 100.0);
            let val = lift_bin_curve_fit(&bins, t);
            assert!(val >= 0.0);
            assert!(val <= 12.01);
        }
    }

    #[test]
    fn dynamics_routing_and_layout_geometry() {
        let params = Arc::new(StripParams::default());
        let shared = Shared::new(
            params.bands.clone(),
            params.eq2_bands.clone(),
            params.sc_eq_bands.clone(),
            params.lift_bands.clone(),
        );
        let mut view = StripView {
            params: params.clone(),
            shared,
            selected: None,
            dyn_page: DynPage::Main,
            pse_page: PsePage::Main,
            wall_page: WallPage::Main,
            drag: None,
            hover: None,
            font: Cell::new(None),
            signature: Cell::new(None),
            graph_db: 24.0,
            scale_menu: false,
            processing_menu: None,
            edit: None,
            down: (0.0, 0.0),
            last_drag: (0.0, 0.0),
            pending_create: None,
            menu: None,
            active_eq: Cell::new(0),
            anim_progress: Cell::new(0.0),
            anim_target: Cell::new(0.0),
            anim_start: Cell::new(0.0),
            anim_time: Cell::new(1.0),
            last_tick: Cell::new(None),
            knee_bulge: Cell::new(0.0),
            pse_knee_bulge: Cell::new(0.0),
            dyn_anim_progress: Cell::new(0.0),
            dyn_anim_target: Cell::new(0.0),
            pse_anim_progress: Cell::new(0.0),
            pse_anim_target: Cell::new(0.0),
            wall_anim_progress: Cell::new(0.0),
            wall_anim_target: Cell::new(0.0),
            pending_solo: None,
            eq_bypass_anim: ButtonAnim::new(),
            pse_bypass_anim: ButtonAnim::new(),
            dyn_bypass_anim: ButtonAnim::new(),
            wall_bypass_anim: ButtonAnim::new(),
            lift_badge_anim: ButtonAnim::new(),
            dyn_band_anim: ButtonAnim::new(),
        };

        // Post mode (default): PSE on the left, EQ in the middle, Dynamics on the right
        assert!(!view.is_pre());
        assert_eq!(view.pse_bounds(), (MARGIN, MODULE_Y, PSE_W, MODULE_H));
        assert_eq!(
            view.eq_bounds(),
            (MARGIN + PSE_W + GAP, MODULE_Y, EQ_W, MODULE_H)
        );
        assert_eq!(
            view.dyn_bounds(),
            (
                UI_W - MARGIN - WALL_W - GAP - DYN_W,
                MODULE_Y,
                DYN_W,
                MODULE_H
            )
        );
        assert_eq!(
            view.wall_bounds(),
            (UI_W - MARGIN - WALL_W, MODULE_Y, WALL_W, MODULE_H)
        );
        assert!(view.pse_bounds().0 + view.pse_bounds().2 <= view.eq_bounds().0);
        assert!(view.eq_bounds().0 + view.eq_bounds().2 <= view.dyn_bounds().0);
        assert!(view.dyn_bounds().0 + view.dyn_bounds().2 <= view.wall_bounds().0);
        assert_eq!(view.gx(), MARGIN + PSE_W + GAP + EQ_GRAPH_PAD_LEFT);
        assert_eq!(view.global_controls(), &[0]);
        assert_eq!(PSE_CONTROLS_KNOBS, [8, 2, 10]);

        let tab2 = view.eq_tab_2_rect();
        let tab_lift = view.eq_tab_lift_rect();
        let tab_sc = view.eq_tab_sc_rect();
        let bypass = view.eq_power_rect();
        let header_listen_sc = view.eq_header_listen_sc_rect();
        assert!(tab2.0 + tab2.2 < tab_lift.0);
        assert!(tab_lift.0 + tab_lift.2 < tab_sc.0);
        assert!(tab_sc.0 + tab_sc.2 < header_listen_sc.0);
        assert!(header_listen_sc.0 + header_listen_sc.2 <= view.eq_bounds().0 + view.eq_bounds().2);
        assert_eq!(tab_lift.1, tab2.1);
        assert_eq!(tab_sc.1, tab2.1);
        assert_eq!(bypass.1, tab2.1);
        assert_eq!(header_listen_sc.1, tab2.1);
        assert_eq!(header_listen_sc.3, MODULE_HEADER_CTRL);
        assert!((bypass.1 + bypass.3 * 0.5 - module_header_mid()).abs() < 0.01);
        assert_eq!(axis_db_text(12), "12");
        assert_eq!(axis_db_text(0), "0");
        assert_eq!(axis_db_text(-12), "-12");

        let slider = view.dyn_main_thresh_slider_rect();
        let gr_meter = view.dyn_main_gr_meter_rect();
        assert_eq!(gr_meter.0, slider.0 + slider.2 + 16.0);
        assert_eq!(gr_meter.1, slider.1);
        assert_eq!(gr_meter.3, slider.3);
        assert_eq!(slider.1, GY);
        assert_eq!(slider.1 + slider.3, GRAPH_BOTTOM);
        assert_eq!(gr_meter.1 + gr_meter.3, GRAPH_BOTTOM);
        assert_eq!(meter_value_y(), EQ_AXIS_LABEL_Y);
        let knee_at_top = view.dyn_main_knee_rect(slider.1, 0.0);
        assert_eq!(knee_at_top.1, slider.1 - KNEE_METER_OVERHANG);
        assert!(knee_at_top.1 + knee_at_top.3 <= slider.1 + slider.3 + KNEE_METER_OVERHANG);
        let knee_at_bottom = view.dyn_main_knee_rect(slider.1 + slider.3, 0.0);
        assert_eq!(
            knee_at_bottom.1 + knee_at_bottom.3,
            slider.1 + slider.3 + KNEE_METER_OVERHANG
        );
        assert!(knee_at_bottom.1 >= slider.1 - KNEE_METER_OVERHANG);
        let pse_knee_at_top = view.pse_main_knee_rect(slider.1, 0.0);
        assert_eq!(pse_knee_at_top.1, slider.1 - KNEE_METER_OVERHANG);
        let cog = view.dyn_cog_button_rect();
        assert_eq!(cog.1, module_header_ctrl_y());
        assert_eq!(cog.1, bypass.1);
        assert_eq!(cog.3, MODULE_HEADER_CTRL);
        assert_eq!(cog.0, view.dyn_bounds().0 + DYN_W - 14.0 - 24.0);
        assert!(cog.0 > view.dyn_power_button_rect().0 + view.dyn_power_button_rect().2);
        let pse_cog = view.pse_cog_button_rect();
        assert_eq!(pse_cog.1, cog.1);
        assert_eq!(pse_cog.0, view.pse_bounds().0 + PSE_W - 14.0 - 24.0);
        let scale_btn = view.scale_button_rect();
        assert_eq!(
            scale_btn.0 + scale_btn.2 * 0.5,
            view.gx() - EQ_GRAPH_PAD_LEFT * 0.5
        );
        assert_eq!(scale_btn.1, GY - 10.0);
        assert_eq!(scale_btn.3, 20.0);
        assert_eq!(FREQ_AXIS[5], (2500.0, "2.5k"));
        assert!(FREQ_AXIS
            .iter()
            .all(|(f, label)| *f != 20000.0 && *label != "0" && *label != "20k"));
        let handle = view.dyn_main_thresh_handle_rect(slider.1 + slider.3 * 0.5);
        assert!(handle.2 <= slider.2 + 12.0);
        assert!(handle.3 >= 12.0);
        let knee = view.dyn_main_knee_rect(slider.1 + slider.3 * 0.5, 0.0);
        let knee_offset = view.dyn_main_knee_offset();
        assert_eq!(knee.1, handle.1 - knee_offset);
        assert_eq!(knee.3, handle.3 + 2.0 * knee_offset);

        let pse_slider = view.pse_main_thresh_slider_rect();
        let pse_gr = view.pse_main_gr_meter_rect();
        assert_eq!(pse_slider.1, slider.1);
        assert_eq!(pse_gr.0, pse_slider.0 + pse_slider.2 + 16.0);
        let depth_handle = view.dyn_main_depth_handle_rect(gr_meter.1 + gr_meter.3 * 0.5);
        assert!(inside(
            depth_handle.0 + 4.0,
            depth_handle.1 + 4.0,
            view.dyn_bounds()
        ));

        assert!(GRAPH_BOTTOM + 22.0 <= MODULE_Y + MODULE_H);
        assert!(MODULE_Y + MODULE_H < FOOTER_LINE_Y);
        assert!(FOOTER_LINE_Y < FOOTER_BTN_Y);
        assert!(FOOTER_BTN_Y + 28.0 <= UI_H);
        assert_eq!(HEADER_H, 82.0);

        let out_r = output_gain_rect();
        assert_eq!(out_r, (UI_W - MARGIN - WALL_W, FOOTER_BTN_Y, WALL_W, 28.0));
        assert_eq!(THEME_BUTTON, (32.0, FOOTER_BTN_Y, 88.0, 28.0));
        assert_eq!(PROCESS_BUTTON.0, 128.0);
        let thresh_val = view.global_value_rect(0);
        assert_eq!(thresh_val, view.global_rect(0));
        assert!(thresh_val.1 + thresh_val.3 <= FOOTER_LINE_Y);
        assert!(
            (rect_center_x(thresh_val) - rect_center_x(slider)).abs() < 0.5,
            "DYN threshold value must sit under the meter"
        );
        let dyn_depth_val = view.global_value_rect(14);
        assert!(
            (rect_center_x(dyn_depth_val) - rect_center_x(gr_meter)).abs() < 0.5,
            "DYN depth value must sit under the GR meter"
        );
        let pse_thresh_val = view.pse_value_rect(1);
        assert!(
            (rect_center_x(pse_thresh_val) - rect_center_x(pse_slider)).abs() < 0.5,
            "PSE threshold value must sit under the meter"
        );
        assert!(view
            .value_at(out_r.0 + 20.0, out_r.1 + out_r.3 * 0.5)
            .is_none());
        let routing = view.footer_comp_routing_rect();
        assert!(view
            .value_at(routing.0 + routing.2 * 0.5, routing.1 + routing.3 * 0.5)
            .is_none());

        view.dyn_page = DynPage::Controls;
        assert_eq!(view.global_controls(), &[5, 11, 6, 3, 4]);
        let attack = view.global_rect(5);
        let release = view.global_rect(11);
        let ratio = view.global_rect(6);
        assert!(attack.2 > 90.0);
        assert!(attack.3 > 70.0);
        assert_eq!(attack.0, release.0);
        assert!(release.1 > attack.1 + attack.3 - 1.0);
        assert!(ratio.1 > release.1);
        assert!(view.global_rect(4).1 > view.global_rect(3).1);
        let routing = view.footer_comp_routing_rect();
        let auto = view.footer_auto_rect();
        let link = view.footer_link_rect();
        assert_eq!(auto.1, routing.1);
        assert_eq!(link.1, routing.1);
        assert!(routing.0 + routing.2 <= auto.0);
        assert!(auto.0 + auto.2 <= link.0);
        assert!((out_r.0 - (link.0 + link.2) - GAP).abs() < f32::EPSILON);
        assert!(
            !view.global_controls().contains(&2),
            "SC HPF must not appear on the compressor controls page"
        );

        view.pse_page = PsePage::Controls;
        assert_eq!(view.pse_controls(), &[8, 2, 10]);
        let time_r = view.pse_knob_rect(10);
        let det_r = view.pse_detect_mode_rect();
        assert_eq!(det_r.0, time_r.0);
        assert!(time_r.1 + time_r.3 <= det_r.1);
        assert!(inside(
            view.pse_bounds().0 + 20.0,
            view.pse_knob_rect(8).1 + 10.0,
            view.pse_bounds()
        ));

        view.wall_page = WallPage::Main;
        assert_eq!(view.wall_controls(), &[18]);
        let wall_slider = view.wall_main_thresh_slider_rect();
        assert_eq!(wall_slider.1, slider.1);
        assert_eq!(wall_slider.2, slider.2);
        assert_eq!(wall_slider.3, slider.3);
        let wall_handle = view.wall_main_thresh_handle_rect(wall_slider.1 + wall_slider.3 * 0.5);
        assert!(wall_handle.2 <= wall_slider.2 + 12.0);
        assert!(wall_handle.3 >= 12.0);
        assert!(inside(
            wall_slider.0 + 4.0,
            wall_slider.1 + 4.0,
            view.wall_bounds()
        ));
        let wall_thresh_val = view.wall_value_rect(18);
        assert_eq!(wall_thresh_val.1, meter_value_y() - 14.0);
        assert!(
            (rect_center_x(wall_slider) - rect_center_x(view.wall_bounds())).abs() < 0.5,
            "WALL meter must sit on the module midline"
        );
        assert!(
            (rect_center_x(wall_thresh_val) - rect_center_x(wall_slider)).abs() < 0.5,
            "WALL threshold value must sit under the meter"
        );
        view.wall_page = WallPage::Controls;
        assert_eq!(view.wall_controls(), &[16, 17]);
        let even_r = view.wall_knob_rect(16);
        let odd_r = view.wall_knob_rect(17);
        assert!(even_r.2 > 90.0);
        assert!(even_r.3 > 160.0);
        assert_eq!(even_r.0, odd_r.0);
        assert!(odd_r.1 > even_r.1 + even_r.3 - 1.0);
        assert!(inside(even_r.0 + 8.0, even_r.1 + 8.0, view.wall_bounds()));
        let wall_cog = view.wall_cog_button_rect();
        assert_eq!(wall_cog.1, cog.1);
        assert_eq!(wall_cog.0, view.wall_bounds().0 + WALL_W - 14.0 - 24.0);
        assert!(wall_cog.0 > view.wall_power_button_rect().0 + view.wall_power_button_rect().2);

        let params_pre = Arc::new(StripParams {
            comp_pre: BoolParam::new("Dynamics routing", true),
            ..StripParams::default()
        });
        view.params = params_pre;
        assert!(view.is_pre());
        assert_eq!(view.pse_bounds(), (MARGIN, MODULE_Y, PSE_W, MODULE_H));
        assert_eq!(
            view.dyn_bounds(),
            (MARGIN + PSE_W + GAP, MODULE_Y, DYN_W, MODULE_H)
        );
        assert_eq!(
            view.eq_bounds(),
            (MARGIN + PSE_W + GAP + DYN_W + GAP, MODULE_Y, EQ_W, MODULE_H)
        );
        assert_eq!(
            view.gx(),
            MARGIN + PSE_W + GAP + DYN_W + GAP + EQ_GRAPH_PAD_LEFT
        );
        assert_eq!(
            view.wall_bounds(),
            (UI_W - MARGIN - WALL_W, MODULE_Y, WALL_W, MODULE_H)
        );
        assert!(view.eq_bounds().0 + view.eq_bounds().2 <= view.wall_bounds().0);
        view.dyn_page = DynPage::Main;
        let pre_slider = view.dyn_main_thresh_slider_rect();
        assert_eq!(pre_slider.0, MARGIN + PSE_W + GAP + 30.0);
        assert_eq!(
            view.dyn_main_gr_meter_rect().0,
            pre_slider.0 + pre_slider.2 + 16.0
        );
    }

    #[test]
    fn gr_overflow_extends_past_range_and_depth() {
        let (actual, uncapped) = gr_meter_bar_heights(5.0, 12.0, 100.0);
        assert!((actual - 100.0 * 5.0 / 30.0).abs() < 0.001);
        assert!((uncapped - 100.0 * 12.0 / 30.0).abs() < 0.001);
        assert!(uncapped > actual);
        let (full, clipped) = gr_meter_bar_heights(30.0, 48.0, 100.0);
        assert!((full - 100.0).abs() < 0.001);
        assert_eq!(clipped, full);
        assert_eq!(overflow_reduction(6.0, 6.0), None);
        assert_eq!(overflow_reduction(6.0, 9.5), Some(9.5));
        assert_eq!(overflow_reduction(-6.0, -9.5), Some(-9.5));
        assert_eq!(overflow_reduction(-6.0, -2.0), None);
        assert_eq!(uncapped_for(3, &[(1, 2.0), (3, 11.5)]), Some(11.5));
        assert_eq!(uncapped_for(2, &[(1, 2.0)]), None);
        assert_eq!(catch_gr_to_depth(20.0, 10.0), 10.0);
        assert_eq!(catch_gr_to_depth(5.0, 10.0), 5.0);
        assert_eq!(catch_gr_to_depth(10.0, 10.0), 10.0);
        assert_eq!(catch_gr_to_depth(8.0, -1.0), 0.0);
    }

    #[test]
    fn hover_cursor_preview_centering_and_deselect_label() {
        for shape in Shape::ALL {
            let upper = shape.uppercase_name();
            assert!(
                !upper.starts_with('+'),
                "Uppercase name must not have '+': {upper}"
            );
            assert_eq!(upper, upper.to_uppercase());
        }
        assert_eq!(hover_preview_label(true, Shape::Bell), "DESELECT");
        assert_eq!(hover_preview_label(false, Shape::Bell), "BELL");
        assert_eq!(hover_preview_label(false, Shape::LowCut), "LOW CUT");
        assert_eq!(hover_preview_text_y(200.0), 204.0);
        assert_eq!(axis_db_text(18), "18");
        assert!((module_header_ctrl_y() + 24.0 * 0.5 - module_header_mid()).abs() < 0.01);
        assert!(module_title_y(16.0) > module_header_mid());
    }

    #[test]
    fn hover_preview_band_curve_and_fading_ranges() {
        let gx = GX;
        let gw = GW;
        let sr = 48000.0;
        let _graph_db = 24.0;

        // Test preview band inference
        let f_mid = x_freq_at(gx + gw * 0.5, gx, gw);
        assert!((f_mid - 632.45).abs() < 1.0);

        // Bell filter preview response & fading spans
        let bell_shape = Shape::Bell;
        let bell_gain = 6.0;
        let bell_q = 1.0;
        let preview_bell = Band {
            id: 0,
            shape: bell_shape,
            freq: 1000.0,
            gain: bell_gain,
            q: bell_q,
            order: 2,
            ..Band::default()
        };
        let coeff_bell = BandCoeffs::make(&preview_bell, sr);
        assert!((coeff_bell.response(1000.0, sr) - 6.0).abs() < 0.01);
        assert!(coeff_bell.response(20.0, sr).abs() < 0.1);
        assert!(coeff_bell.response(20000.0, sr).abs() < 0.1);

        let oct_span_bell = (2.0 / bell_q.clamp(0.15, 18.0)).clamp(0.5, 4.0);
        let f_left = (preview_bell.freq * 2.0_f64.powf(-oct_span_bell)).max(20.0);
        let f_right = (preview_bell.freq * 2.0_f64.powf(oct_span_bell)).min(20000.0);
        assert_eq!(f_left, 250.0);
        assert_eq!(f_right, 4000.0);
        assert!(freq_x_at(f_left, gx, gw) < freq_x_at(preview_bell.freq, gx, gw));
        assert!(freq_x_at(preview_bell.freq, gx, gw) < freq_x_at(f_right, gx, gw));

        // LowCut filter preview response & fading spans
        let preview_lowcut = Band {
            id: 0,
            shape: Shape::LowCut,
            freq: 100.0,
            gain: 0.0,
            q: 0.707,
            order: 2,
            ..Band::default()
        };
        let coeff_lowcut = BandCoeffs::make(&preview_lowcut, sr);
        assert!(coeff_lowcut.response(20.0, sr) < -12.0);
        assert!((coeff_lowcut.response(10000.0, sr)).abs() < 0.05);

        let oct_span_cut = (1.5 / preview_lowcut.q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
        let f_fade_start = (preview_lowcut.freq * 2.0_f64.powf(-oct_span_cut * 0.5)).max(20.0);
        let f_fade_end = (preview_lowcut.freq * 2.0_f64.powf(oct_span_cut)).min(20000.0);
        assert!(f_fade_start < preview_lowcut.freq);
        assert!(f_fade_end > preview_lowcut.freq);
        assert!(freq_x_at(f_fade_start, gx, gw) < freq_x_at(f_fade_end, gx, gw));

        // HighCut filter preview response & fading spans
        let preview_highcut = Band {
            id: 0,
            shape: Shape::HighCut,
            freq: 5000.0,
            gain: 0.0,
            q: 0.707,
            order: 2,
            ..Band::default()
        };
        let coeff_highcut = BandCoeffs::make(&preview_highcut, sr);
        assert!((coeff_highcut.response(100.0, sr)).abs() < 0.05);
        assert!(coeff_highcut.response(20000.0, sr) < -12.0);

        // HighShelf & LowShelf response
        let preview_highshelf = Band {
            id: 0,
            shape: Shape::HighShelf,
            freq: 2000.0,
            gain: 6.0,
            q: 1.0,
            order: 2,
            ..Band::default()
        };
        let coeff_highshelf = BandCoeffs::make(&preview_highshelf, sr);
        assert!((coeff_highshelf.response(100.0, sr)).abs() < 0.1);
        assert!((coeff_highshelf.response(15000.0, sr) - 6.0).abs() < 0.2);

        let preview_lowshelf = Band {
            id: 0,
            shape: Shape::LowShelf,
            freq: 200.0,
            gain: 6.0,
            q: 1.0,
            order: 2,
            ..Band::default()
        };
        let coeff_lowshelf = BandCoeffs::make(&preview_lowshelf, sr);
        assert!((coeff_lowshelf.response(30.0, sr) - 6.0).abs() < 0.2);
        assert!((coeff_lowshelf.response(10000.0, sr)).abs() < 0.1);
    }

    #[test]
    fn dynamic_eq_range_drag_and_bipolar_normalization() {
        let graph_db = 18.0;
        let gain = 4.0;
        let y_node = db_y(gain, graph_db);

        let target_atten = -2.0;
        let y_atten = db_y(target_atten, graph_db);
        assert!(y_atten > y_node, "Attenuation drags downward (higher y)");
        let range_atten = snap_dyn_range(gain, y_atten, graph_db);
        assert!((range_atten - 6.0).abs() < 1e-4);

        let target_boost = 10.0;
        let y_boost = db_y(target_boost, graph_db);
        assert!(y_boost < y_node, "Boost drags upward (lower y)");
        let range_boost = snap_dyn_range(gain, y_boost, graph_db);
        assert!((range_boost - (-6.0)).abs() < 1e-4);

        let y_near_zero = db_y(gain - 0.15, graph_db);
        assert_eq!(snap_dyn_range(gain, y_near_zero, graph_db), 0.0);

        let norm_fn = |range: f64| (range + 24.0) / 48.0;
        let denorm_fn = |n: f64| -24.0 + 48.0 * n;
        assert_eq!(norm_fn(-24.0), 0.0);
        assert_eq!(norm_fn(0.0), 0.5);
        assert_eq!(norm_fn(24.0), 1.0);
        assert_eq!(denorm_fn(0.0), -24.0);
        assert_eq!(denorm_fn(0.5), 0.0);
        assert_eq!(denorm_fn(1.0), 24.0);

        let mut b = Band {
            freq: 1000.0,
            gain: 4.0,
            dynamic: true,
            range: 6.0,
            ..Band::default()
        };
        let gx = GX;
        let gw = GW;
        let x_node = freq_x_at(b.freq, gx, gw);
        let node_y = db_y(b.gain, graph_db);
        let y_range = db_y(b.gain - b.range, graph_db);
        assert!(range_handle_hit(
            &b,
            x_node,
            y_range,
            graph_db,
            gx,
            gw,
            Some(b.id)
        ));
        assert!(!range_handle_hit(
            &b,
            x_node,
            node_y,
            graph_db,
            gx,
            gw,
            Some(b.id)
        ));
        assert!(!range_handle_hit(
            &b,
            x_node + 80.0,
            y_range,
            graph_db,
            gx,
            gw,
            Some(b.id)
        ));
        b.dynamic = false;
        assert!(range_handle_hit(
            &b,
            x_node + 10.0,
            node_y,
            graph_db,
            gx,
            gw,
            Some(b.id)
        ));
        assert!(!range_handle_hit(
            &b,
            x_node + 10.0,
            node_y,
            graph_db,
            gx,
            gw,
            None
        ));

        b.dynamic = true;
        assert_eq!(capped_reduction(6.0, 9.0), 6.0);
        assert_eq!(capped_reduction(6.0, 2.0), 2.0);
        assert_eq!(capped_reduction(-6.0, -9.0), -6.0);
        assert_eq!(capped_reduction(-6.0, -2.0), -2.0);
        assert_eq!(hud_height_for(&b), HUD_H);
        let hud = hud_rect_for_at(&b, graph_db, gx, gw);
        for i in 0..3 {
            let value = hud_value_rect_at(&b, graph_db, i, gx, gw);
            assert!(inside(value.0, value.1, hud));
            assert!(inside(value.0 + value.2, value.1 + value.3, hud));
        }
        let dyn_overlay = hud_dyn_rect_for_at(&b, graph_db, gx, gw).expect("dyn overlay");
        for i in 0..5 {
            let value = hud_dyn_value_rect_at(&b, graph_db, i, gx, gw);
            assert!(inside(value.0, value.1, dyn_overlay));
            assert!(inside(value.0 + value.2, value.1 + value.3, dyn_overlay));
        }
        let live = plot_band(&b, &[(b.id, 9.0)]);
        assert!((live.gain - (b.gain - 6.0)).abs() < 1e-9);
        let idle = plot_band(&b, &[]);
        assert_eq!(idle.gain, b.gain);
    }

    #[test]
    fn eq_freq_axis_drops_edges_and_uses_two_point_five_k() {
        assert_eq!(FREQ_AXIS[0], (50.0, "50"));
        assert_eq!(FREQ_AXIS[5], (2500.0, "2.5k"));
        assert_eq!(FREQ_AXIS.last(), Some(&(10000.0, "10k")));
        assert!(FREQ_AXIS.iter().all(|(freq, label)| {
            *freq != 20.0 && *freq != 20000.0 && *label != "0" && *label != "20k" && *label != "2k"
        }));
    }

    #[test]
    fn eq_curve_samples_notch_zero_instead_of_skipping_it() {
        let sr = 48000.0;
        let gx = 0.0;
        let gw = 768.0;
        let x_mid = gw * 200.5 / 419.0;
        let freq = x_freq_at(x_mid, gx, gw);
        let band = Band {
            shape: Shape::Notch,
            freq,
            q: 1.0,
            ..Band::default()
        };
        let coeff = BandCoeffs::make(&band, sr);
        let coarse_min = (0..420)
            .map(|i| {
                let x = gw * i as f32 / 419.0;
                coeff.response(x_freq_at(x, gx, gw).min(sr * 0.49), sr)
            })
            .fold(0.0_f64, f64::min);
        let xs = eq_curve_xs(gx, gw, [freq]);
        let dbs = eq_db_on_xs(&coeff, &xs, gx, gw, sr, sr);
        let sampled_min = dbs.iter().copied().fold(0.0_f64, f64::min);
        assert!(
            sampled_min < -80.0,
            "curve must hit the notch zero, got {sampled_min}"
        );
        assert!(
            coarse_min > sampled_min + 10.0,
            "coarse grid stayed at {coarse_min}, sampled {sampled_min}"
        );
    }
}
