use super::*;
use std::time::Instant;

pub(super) const HUD_BTN: f32 = 32.0;
/// Band power button — same size as module header bypasses (`MODULE_HEADER_CTRL`).
pub(super) const HUD_BYPASS: f32 = MODULE_HEADER_CTRL;
pub(super) const HUD_COL_GAP: f32 = 4.0;
/// Room past the bypass left edge so hover/click halo is not scissor-clipped.
pub(super) const HUD_BYPASS_HALO_PAD: f32 = 10.0;
pub(super) const HUD_DYN_W: f32 = 52.0;
pub(super) const HUD_DYN_H: f32 = 28.0;
pub(super) const HUD_DYN_TEXT: f32 = MODULE_TITLE_SIZE;
pub(super) const HUD_FREQ_W: f32 = 160.0;
pub(super) const HUD_GAIN_W: f32 = 120.0;
pub(super) const HUD_Q_W: f32 = 84.0;
pub(super) const HUD_SHAPE_W: f32 = 172.0;
pub(super) const HUD_ORDER_W: f32 = 112.0;
/// FREQ/GAIN/Q labels + values, shape/order, and meter readouts — matches `MODULE_TITLE_SIZE`.
pub(super) const HUD_LABEL_SIZE: f32 = MODULE_TITLE_SIZE;
pub(super) const HUD_VALUE_SIZE: f32 = HUD_LABEL_SIZE;
pub(super) const HUD_SHAPE_TEXT: f32 = HUD_LABEL_SIZE;
pub(super) const HUD_COG: f32 = 24.0;
pub(super) const HUD_RIGHT_GAP: f32 = 6.0;
pub(super) const HUD_DYN_FIELD_H: f32 = 40.0;
pub(super) const HUD_DYN_VALUE_W: f32 = 72.0;
pub(super) const AXIS_STRIP_H: f32 = HUD_DYN_FIELD_H;
pub(super) const NODE_CHROME_BTN: f32 = 22.0;
pub(super) const NODE_CHROME_GAP: f32 = 4.0;
pub(super) const NODE_CHROME_CLEAR: f32 = 18.0;
pub(super) const LIFT_DOCK_H: f32 = 60.0;

#[derive(Clone, Copy)]
pub(super) struct HudGeom {
    pub(super) bx: f32,
    pub(super) by: f32,
    pub(super) bw: f32,
    pub(super) bh: f32,
    pub(super) show_dyn_btn: bool,
    /// Gear / dyn-page control — only when band dynamics are on.
    pub(super) show_cog: bool,
}

pub(super) fn band_allows_dyn(b: &Band) -> bool {
    b.shape.has_gain() && b.id < SC_EQ_ID_BASE
}

pub(super) fn band_keeps_dyn_page(b: Option<&Band>) -> bool {
    b.is_some_and(|b| b.dynamic && band_allows_dyn(b))
}

pub(super) fn axis_strip_y() -> f32 {
    EQ_AXIS_LABEL_Y - AXIS_STRIP_H * 0.5
}

pub(super) fn axis_strip_rect(gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    (gx, axis_strip_y(), gw, AXIS_STRIP_H)
}

pub(super) fn node_chrome_above(gain: f64, dyn_range: Option<f64>) -> bool {
    match dyn_range {
        Some(range) if range > 0.0 => true,
        Some(range) if range < 0.0 => false,
        _ => gain >= 0.0,
    }
}

pub(super) fn band_chrome_above(b: &Band) -> bool {
    let gain = if b.shape.has_gain() { b.gain } else { 0.0 };
    node_chrome_above(gain, b.dynamic.then_some(b.range))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum NodeChromeLayout {
    Above,
    Below,
    /// Solo left / delete right of the node (detached-above meter only).
    Flank,
}

pub(super) fn band_chrome_layout(
    b: &Band,
    graph_db: f64,
    gx: f32,
    gw: f32,
    selected: bool,
) -> NodeChromeLayout {
    if selected && b.dynamic && b.shape.has_gain() {
        let geom = dyn_meter_geom(b, graph_db, gx, gw);
        if geom.detached_above {
            return NodeChromeLayout::Flank;
        }
    }
    if band_chrome_above(b) {
        NodeChromeLayout::Above
    } else {
        NodeChromeLayout::Below
    }
}

pub(super) fn module_settings_should_return(
    pointer_in: bool,
    page_controls: bool,
    away_since: &Cell<Option<Instant>>,
    now: Instant,
) -> bool {
    if !page_controls {
        away_since.set(None);
        return false;
    }
    if pointer_in {
        away_since.set(None);
        false
    } else {
        let start = away_since.get().unwrap_or(now);
        away_since.set(Some(start));
        now.saturating_duration_since(start) >= MODULE_SETTINGS_HOLD
    }
}

pub(super) fn hud_dyn_btn_rect_at(gx: f32, gw: f32, by: f32, bh: f32) -> (f32, f32, f32, f32) {
    (
        gx + gw - HUD_DYN_W,
        by + (bh - HUD_DYN_H).max(0.0) * 0.5,
        HUD_DYN_W,
        HUD_DYN_H,
    )
}

pub(super) fn hud_dyn_btn_center_at(gx: f32, gw: f32, by: f32, bh: f32) -> (f32, f32) {
    let r = hud_dyn_btn_rect_at(gx, gw, by, bh);
    (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5)
}

pub(super) fn hud_cog_rect_at(gx: f32, gw: f32, by: f32, bh: f32) -> (f32, f32, f32, f32) {
    let dyn_r = hud_dyn_btn_rect_at(gx, gw, by, bh);
    let x = dyn_r.0 - HUD_RIGHT_GAP - HUD_COG;
    (x, by + (bh - HUD_COG).max(0.0) * 0.5, HUD_COG, HUD_COG)
}

pub(super) fn hud_content_x_w(
    gx: f32,
    gw: f32,
    show_dyn_btn: bool,
    show_cog: bool,
) -> (f32, f32) {
    if show_dyn_btn {
        let by = axis_strip_y();
        let right = if show_cog {
            hud_cog_rect_at(gx, gw, by, AXIS_STRIP_H).0
        } else {
            // No dead cog gap — content runs up to the DYN button.
            hud_dyn_btn_rect_at(gx, gw, by, AXIS_STRIP_H).0
        };
        (gx, (right - HUD_RIGHT_GAP - gx).max(0.0))
    } else {
        (gx, gw)
    }
}

pub(super) fn hud_geom_for_at(b: &Band, _range: f64, gx: f32, gw: f32) -> HudGeom {
    let show_dyn_btn = band_allows_dyn(b);
    let show_cog = show_dyn_btn && b.dynamic;
    let (bx, bw) = hud_content_x_w(gx, gw, show_dyn_btn, show_cog);
    HudGeom {
        bx,
        by: axis_strip_y(),
        bw,
        bh: AXIS_STRIP_H,
        show_dyn_btn,
        show_cog,
    }
}

pub(super) fn hud_rect_for_at(b: &Band, range: f64, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let g = hud_geom_for_at(b, range, gx, gw);
    (g.bx, g.by, g.bw, g.bh)
}

pub(super) fn hud_bounds_for_at(b: &Band, range: f64, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let g = hud_geom_for_at(b, range, gx, gw);
    if g.show_dyn_btn {
        axis_strip_rect(gx, gw)
    } else {
        (g.bx, g.by, g.bw, g.bh)
    }
}

pub(super) fn hud_dyn_btn_center(g: HudGeom, gx: f32, gw: f32) -> (f32, f32) {
    hud_dyn_btn_center_at(gx, gw, g.by, g.bh)
}

pub(super) fn hud_dyn_btn_hit(g: HudGeom, x: f32, y: f32, gx: f32, gw: f32) -> bool {
    if !g.show_dyn_btn {
        return false;
    }
    inside(x, y, hud_dyn_btn_rect_at(gx, gw, g.by, g.bh))
}

pub(super) fn hud_cog_hit(g: HudGeom, x: f32, y: f32, gx: f32, gw: f32) -> bool {
    g.show_cog && inside(x, y, hud_cog_rect_at(gx, gw, g.by, g.bh))
}

pub(super) fn hud_bypass_rect(bx: f32, by: f32) -> (f32, f32, f32, f32) {
    (
        bx,
        by + (AXIS_STRIP_H - HUD_BYPASS) * 0.5,
        HUD_BYPASS,
        HUD_BYPASS,
    )
}

/// Shared Y for Bell shape text, FREQ/GAIN/Q, and module meter readouts.
pub(super) fn hud_control_text_y() -> f32 {
    EQ_AXIS_LABEL_Y + HUD_LABEL_SIZE * 0.32
}

pub(super) fn hud_shape_rect(bx: f32, by: f32, _bw: f32) -> (f32, f32, f32, f32) {
    let x = bx
        + HUD_BYPASS
        + HUD_COL_GAP
        + HUD_FREQ_W
        + HUD_COL_GAP
        + HUD_GAIN_W
        + HUD_COL_GAP
        + HUD_Q_W
        + HUD_COL_GAP;
    (x, by + (AXIS_STRIP_H - HUD_BTN) * 0.5, HUD_SHAPE_W, HUD_BTN)
}

pub(super) fn hud_order_rect(bx: f32, by: f32) -> (f32, f32, f32, f32) {
    let shape = hud_shape_rect(bx, by, 0.0);
    (
        shape.0 + shape.2 + HUD_COL_GAP,
        shape.1,
        HUD_ORDER_W,
        HUD_BTN,
    )
}

pub(super) fn hud_value_rect_at(
    b: &Band,
    range: f64,
    i: usize,
    gx: f32,
    gw: f32,
) -> (f32, f32, f32, f32) {
    let g = hud_geom_for_at(b, range, gx, gw);
    let x0 = g.bx + HUD_BYPASS + HUD_COL_GAP;
    match i {
        0 => (x0, g.by, HUD_FREQ_W, g.bh),
        1 => (x0 + HUD_FREQ_W + HUD_COL_GAP, g.by, HUD_GAIN_W, g.bh),
        2 => (
            x0 + HUD_FREQ_W + HUD_COL_GAP + HUD_GAIN_W + HUD_COL_GAP,
            g.by,
            HUD_Q_W,
            g.bh,
        ),
        other => hud_dyn_value_rect_at(b, range, if other == 3 { 0 } else { 1 }, gx, gw),
    }
}

pub(super) fn hud_dyn_field_rect_at(
    b: &Band,
    range: f64,
    i: usize,
    gx: f32,
    gw: f32,
) -> (f32, f32, f32, f32) {
    let g = hud_geom_for_at(b, range, gx, gw);
    let cols = 5.0;
    let col_w = (g.bw - HUD_COL_GAP * (cols - 1.0)).max(0.0) / cols;
    let x = g.bx + i.min(4) as f32 * (col_w + HUD_COL_GAP);
    (x, g.by, col_w, g.bh)
}

pub(super) fn hud_dyn_value_rect_at(
    b: &Band,
    range: f64,
    i: usize,
    gx: f32,
    gw: f32,
) -> (f32, f32, f32, f32) {
    let r = hud_dyn_field_rect_at(b, range, i, gx, gw);
    if i == 1 {
        let w = (r.2 * 0.55).max(HUD_DYN_VALUE_W);
        return (r.0 + r.2 - w, r.1, w, r.3);
    }
    (
        r.0 + r.2 - HUD_DYN_VALUE_W - 6.0,
        r.1 + 2.0,
        HUD_DYN_VALUE_W,
        22.0,
    )
}

pub(super) fn hud_rect_for_lift_at(
    _b: &LiftBand,
    _range: f64,
    gx: f32,
    gw: f32,
) -> (f32, f32, f32, f32) {
    let (bx, bw) = hud_content_x_w(gx, gw, false, false);
    (bx, axis_strip_y(), bw, AXIS_STRIP_H)
}

pub(super) fn hud_value_rect_lift_at(
    b: &LiftBand,
    range: f64,
    i: usize,
    gx: f32,
    gw: f32,
) -> (f32, f32, f32, f32) {
    let (x, y, _, h) = hud_rect_for_lift_at(b, range, gx, gw);
    let x0 = x + HUD_BYPASS + HUD_COL_GAP;
    match i {
        0 => (x0, y, HUD_FREQ_W, h),
        1 => (x0 + HUD_FREQ_W + HUD_COL_GAP, y, HUD_GAIN_W, h),
        _ => (
            x0 + HUD_FREQ_W + HUD_COL_GAP + HUD_GAIN_W + HUD_COL_GAP,
            y,
            HUD_Q_W,
            h,
        ),
    }
}

pub(super) fn hud_row_hit(x: f32, y: f32, bx: f32, by: f32, is_cut: bool) -> Option<HudChrome> {
    if inside(x, y, hud_bypass_rect(bx, by)) {
        Some(HudChrome::Bypass)
    } else if inside(x, y, hud_shape_rect(bx, by, 0.0)) {
        Some(HudChrome::Shape)
    } else if is_cut && inside(x, y, hud_order_rect(bx, by)) {
        Some(HudChrome::Order)
    } else {
        None
    }
}

#[derive(Clone, Copy)]
pub(super) enum HudChrome {
    Bypass,
    Shape,
    Order,
}

#[derive(Clone, Copy)]
pub(super) enum NodeChrome {
    Solo,
    Close,
}

pub(super) fn node_chrome_rects(
    node_x: f32,
    node_y: f32,
    layout: NodeChromeLayout,
    gx: f32,
    gw: f32,
) -> ((f32, f32, f32, f32), (f32, f32, f32, f32)) {
    match layout {
        NodeChromeLayout::Flank => {
            let y = (node_y - NODE_CHROME_BTN * 0.5)
                .clamp(GY + 4.0, GRAPH_BOTTOM - NODE_CHROME_BTN - 4.0);
            let solo_x = (node_x - NODE_CHROME_CLEAR - NODE_CHROME_BTN).max(gx + 4.0);
            let close_x =
                (node_x + NODE_CHROME_CLEAR).min(gx + gw - NODE_CHROME_BTN - 4.0);
            let solo = (solo_x, y, NODE_CHROME_BTN, NODE_CHROME_BTN);
            let close = (close_x, y, NODE_CHROME_BTN, NODE_CHROME_BTN);
            (solo, close)
        }
        NodeChromeLayout::Above | NodeChromeLayout::Below => {
            let pair_w = NODE_CHROME_BTN * 2.0 + NODE_CHROME_GAP;
            let x = (node_x - pair_w * 0.5).clamp(gx + 4.0, gx + gw - pair_w - 4.0);
            let y = if layout == NodeChromeLayout::Above {
                node_y - NODE_CHROME_CLEAR - NODE_CHROME_BTN
            } else {
                node_y + NODE_CHROME_CLEAR
            }
            .clamp(GY + 4.0, GRAPH_BOTTOM - NODE_CHROME_BTN - 4.0);
            let solo = (x, y, NODE_CHROME_BTN, NODE_CHROME_BTN);
            let close = (
                x + NODE_CHROME_BTN + NODE_CHROME_GAP,
                y,
                NODE_CHROME_BTN,
                NODE_CHROME_BTN,
            );
            (solo, close)
        }
    }
}

pub(super) fn node_chrome_hit(
    x: f32,
    y: f32,
    node_x: f32,
    node_y: f32,
    layout: NodeChromeLayout,
    gx: f32,
    gw: f32,
) -> Option<NodeChrome> {
    let (solo, close) = node_chrome_rects(node_x, node_y, layout, gx, gw);
    if inside(x, y, solo) {
        Some(NodeChrome::Solo)
    } else if inside(x, y, close) {
        Some(NodeChrome::Close)
    } else {
        None
    }
}

/// Invisible union covering solo↔X gap (stacked) and solo↔node↔X (flank), so the
/// graph create/deselect overlay does not flash while crossing chrome cracks.
pub(super) fn node_chrome_gap_rect(
    node_x: f32,
    node_y: f32,
    layout: NodeChromeLayout,
    gx: f32,
    gw: f32,
) -> (f32, f32, f32, f32) {
    let (solo, close) = node_chrome_rects(node_x, node_y, layout, gx, gw);
    let left = solo.0.min(close.0);
    let right = (solo.0 + solo.2).max(close.0 + close.2);
    match layout {
        NodeChromeLayout::Flank => {
            let top = solo.1.min(close.1);
            let bot = (solo.1 + solo.3).max(close.1 + close.3);
            (left, top, right - left, bot - top)
        }
        NodeChromeLayout::Above => {
            let top = solo.1.min(close.1);
            let bot = node_y.max((solo.1 + solo.3).max(close.1 + close.3));
            (left, top, right - left, (bot - top).max(0.0))
        }
        NodeChromeLayout::Below => {
            let top = node_y.min(solo.1.min(close.1));
            let bot = (solo.1 + solo.3).max(close.1 + close.3);
            (left, top, right - left, (bot - top).max(0.0))
        }
    }
}

pub(super) fn node_chrome_gap_hit(
    x: f32,
    y: f32,
    node_x: f32,
    node_y: f32,
    layout: NodeChromeLayout,
    gx: f32,
    gw: f32,
) -> bool {
    inside(x, y, node_chrome_gap_rect(node_x, node_y, layout, gx, gw))
}

pub(super) fn hud_menu_rect(anchor: (f32, f32, f32, f32), h: f32) -> (f32, f32, f32, f32) {
    let below = anchor.1 + anchor.3 + 2.0;
    let y = if below + h <= GY + GH {
        below
    } else {
        (anchor.1 - h - 2.0).max(GY)
    };
    (anchor.0, y, anchor.2.max(160.0), h)
}

pub(super) fn band_rect_at(i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    band_slot_rect_at(i, gx, gw, 5)
}

pub(super) fn band_slot_rect_at(i: usize, gx: f32, gw: f32, count: usize) -> (f32, f32, f32, f32) {
    let slot = (gw - 46.0) / count as f32;
    (
        gx + 38.0 + i as f32 * slot,
        GRAPH_BOTTOM - LIFT_DOCK_H,
        (slot - 4.0).max(80.0),
        52.0,
    )
}

pub(super) fn band_bar_rect_at(i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let r = band_rect_at(i, gx, gw);
    (r.0 + 12.0, r.1 + 28.0, r.2 - 24.0, 16.0)
}

pub(super) fn band_value_rect_at(i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let r = band_rect_at(i, gx, gw);
    (r.0 + r.2 - 68.0, r.1 + 4.0, 60.0, 20.0)
}

pub(super) fn hz(f: f64) -> String {
    if f >= 1000.0 {
        format!("{:.2} kHz", f / 1000.0)
    } else {
        format!("{:.0} Hz", f)
    }
}

