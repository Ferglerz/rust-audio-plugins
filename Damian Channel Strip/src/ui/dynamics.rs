use super::*;

pub(super) const GR_METER_DB: f32 = 30.0;

pub(super) fn gr_meter_bar_heights(actual: f32, uncapped: f32, meter_h: f32) -> (f32, f32) {
    let actual_h = meter_h * (actual.max(0.0) / GR_METER_DB).clamp(0.0, 1.0);
    let uncapped_h = meter_h * (uncapped.max(actual).max(0.0) / GR_METER_DB).clamp(0.0, 1.0);
    (actual_h, uncapped_h)
}

pub(super) fn knee_pulse_bulge(bulge: &Cell<f32>, knee_hover: bool) -> f32 {
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

pub(super) fn draw_knee_zone(
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

pub(super) fn draw_gr_depth_handle(d: &mut Draw, gx_m: f32, gw_m: f32, depth_y: f32, color: C) {
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

pub(super) fn overflow_reduction(range: f64, uncapped: f64) -> Option<f64> {
    if range >= 0.0 {
        (uncapped > range + 0.05).then_some(uncapped)
    } else {
        (uncapped < range - 0.05).then_some(uncapped)
    }
}

pub(super) fn capped_reduction(range: f64, uncapped: f64) -> f64 {
    if range >= 0.0 {
        uncapped.clamp(0.0, range)
    } else {
        uncapped.clamp(range, 0.0)
    }
}

pub(super) fn uncapped_for(id: u64, meters: &[(u64, f32)]) -> Option<f64> {
    meters
        .iter()
        .find(|(band_id, _)| *band_id == id)
        .map(|(_, gr)| f64::from(*gr))
}

pub(super) fn module_header_mid() -> f32 {
    MODULE_Y + MODULE_HEADER_H * 0.5
}

pub(super) fn module_header_ctrl_y() -> f32 {
    module_header_mid() - MODULE_HEADER_CTRL * 0.5
}

/// PSE / COMP / WALL module header title size (PARAMETRIC EQ stays 16).
pub(super) const MODULE_TITLE_SIZE: f32 = 15.0;

pub(super) fn module_title_y(size: f32) -> f32 {
    module_header_mid() + size * 0.35
}

pub(super) fn meter_value_y() -> f32 {
    hud_control_text_y()
}

pub(super) fn rect_center_x(r: (f32, f32, f32, f32)) -> f32 {
    r.0 + r.2 * 0.5
}

pub(super) fn meter_value_rect(meter: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let cx = rect_center_x(meter);
    (cx - 25.0, meter_value_y() - 14.0, 50.0, 22.0)
}

pub(super) fn stacked_knob_slots(
    module_x: f32,
    module_w: f32,
    count: usize,
) -> Vec<(f32, f32, f32, f32)> {
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

pub(super) fn catch_gr_to_depth(gr: f32, depth: f32) -> f32 {
    gr.min(depth.max(0.0))
}

/// Rising edge of DAW transport play / process restart.
pub(super) fn playback_started(was_playing: bool, playing: bool) -> bool {
    playing && !was_playing
}

pub(super) fn update_playback_gr_peak(peak: f32, gr: f32) -> f32 {
    peak.max(gr.max(0.0))
}

/// Under the GR meter: peak hold since playback start, or the depth/range cap while dragged.
pub(super) fn gr_meter_readout_db(peak_db: f32, cap_db: f32, dragging_cap: bool) -> f32 {
    if dragging_cap {
        cap_db
    } else {
        peak_db
    }
}

pub(super) fn format_gr_meter_readout(db: f32) -> String {
    format!("{:.1}", db)
}

pub(super) fn format_knee_readout(knee: f32) -> String {
    format!("Knee: {}", format_gr_meter_readout(knee))
}

pub(super) fn module_header_cog_rect(module_x: f32, module_w: f32) -> (f32, f32, f32, f32) {
    (
        module_x + module_w - 14.0 - 24.0,
        module_header_ctrl_y(),
        24.0,
        MODULE_HEADER_CTRL,
    )
}

pub(super) fn draw_module_page_button(
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

pub(super) fn clamp_knee_to_meter(x: f32, y: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
    let min_y = METER_TOP - KNEE_METER_OVERHANG;
    let max_bottom = METER_TOP + METER_H + KNEE_METER_OVERHANG;
    let top = y.max(min_y);
    let bottom = (y + h).min(max_bottom);
    (x, top, w, (bottom - top).max(0.0))
}

pub(super) fn axis_label_y(line_y: f32, size: f32) -> f32 {
    line_y + size * 0.35
}

pub(super) fn axis_db_text(db: i32) -> String {
    format!("{db}")
}
