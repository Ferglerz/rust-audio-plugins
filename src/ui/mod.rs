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
use std::{
    cell::Cell,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

const fn rgb(r: u8, g: u8, b: u8) -> C {
    C {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}
const BG: C = rgb(19, 22, 27);
const PANEL: C = rgb(27, 31, 37);
const LINE: C = rgb(45, 50, 58);
const TEXT: C = rgb(229, 233, 237);
const MUTED: C = rgb(143, 153, 164);
const GOLD: C = rgb(225, 187, 101);
const TEAL: C = rgb(106, 213, 189);
pub const LIFT_COLOR: C = rgb(110, 215, 255);
const COLORS: [C; 6] = [
    rgb(111, 210, 188),
    rgb(171, 151, 238),
    rgb(236, 177, 107),
    rgb(108, 176, 242),
    rgb(228, 130, 164),
    rgb(193, 214, 118),
];
const UI_W: f32 = 1280.0;
const UI_H: f32 = 656.0;
const HEADER_H: f32 = 82.0;
const MARGIN: f32 = 16.0;
const GAP: f32 = 12.0;
const SIDE_W: f32 = 236.0;
const MODULE_Y: f32 = 92.0;
const MODULE_H: f32 = 500.0;
const EQ_W: f32 = UI_W - MARGIN * 2.0 - SIDE_W * 2.0 - GAP * 2.0;
const FOOTER_LINE_Y: f32 = 608.0;
const FOOTER_BTN_Y: f32 = 616.0;
const GX: f32 = MARGIN + 44.0;
const GY: f32 = 136.0;
const GW: f32 = EQ_W - 60.0;
const GH: f32 = 420.0;
const GRAPH_BOTTOM: f32 = GY + GH;
const THEME_BUTTON: (f32, f32, f32, f32) = (UI_W - MARGIN - 116.0, FOOTER_BTN_Y, 116.0, 28.0);
pub const EQ2_ID_BASE: u64 = 10_000;
const GRAPH_CLIP_W: f32 = EQ_W;
const PROCESS_BUTTON: (f32, f32, f32, f32) = (130.0, FOOTER_BTN_Y, 216.0, 28.0);
const PSE_KNOBS: [usize; 6] = [7, 8, 1, 2, 9, 10];
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
    (354.0, FOOTER_BTN_Y, 104.0, 28.0)
}
const SCALES: [f64; 6] = [12.0, 24.0, 36.0, 48.0, 60.0, 72.0];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DynPage {
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
struct ValueEdit {
    target: ValueTarget,
    rect: (f32, f32, f32, f32),
    text: String,
    original: String,
    cursor: usize,
    anchor: usize,
    invalid: bool,
}
impl ValueEdit {
    fn selection(&self) -> std::ops::Range<usize> {
        self.cursor.min(self.anchor)..self.cursor.max(self.anchor)
    }
    fn insert(&mut self, text: &str) {
        let text = text
            .trim_matches(['\r', '\n'])
            .replace('−', "-")
            .replace('\u{a0}', " ");
        if !text.is_ascii() || text.chars().any(|c| c.is_control()) {
            return;
        }
        let selection = self.selection();
        if self.text.len() - selection.len() + text.len() > 24 {
            return;
        }
        self.cursor = selection.start + text.len();
        self.anchor = self.cursor;
        self.text.replace_range(selection, &text);
        self.invalid = false;
    }
    fn erase(&mut self, backwards: bool) {
        if self.cursor == self.anchor {
            if backwards {
                self.anchor = self.cursor.saturating_sub(1);
            } else {
                self.anchor = (self.cursor + 1).min(self.text.len());
            }
        }
        self.insert("");
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
        ValueTarget::Band(0) | ValueTarget::Lift(0) | ValueTarget::Global(2) => {
            &[("khz", 1000.0), ("hz", 1.0), ("k", 1000.0)]
        }
        ValueTarget::Band(5 | 6) | ValueTarget::Lift(5 | 6) | ValueTarget::Global(5 | 11) => {
            &[("ms", 1.0), ("s", 1000.0)]
        }
        ValueTarget::Band(4) | ValueTarget::Lift(4) | ValueTarget::Global(6) => &[(":1", 1.0)],
        ValueTarget::Global(3 | 4) => &[("%", 1.0)],
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
#[allow(dead_code)]
fn freq_x(freq: f64) -> f32 {
    freq_x_at(freq, GX, GW)
}
#[allow(dead_code)]
fn x_freq(x: f32) -> f64 {
    x_freq_at(x, GX, GW)
}
fn db_y(db: f64, range: f64) -> f32 {
    GY + GH * (0.5 - (db.clamp(-range, range) / (2.0 * range)) as f32)
}
fn y_db(y: f32, range: f64) -> f64 {
    ((0.5 - (y - GY) as f64 / GH as f64) * 2.0 * range).clamp(-24.0, 24.0)
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
fn hud_rect_for_at(b: &Band, range: f64, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let node_x = freq_x_at(b.freq, gx, gw);
    let node_y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, range);
    let bw = 304.0;
    let bh = 104.0;
    let gap = 36.0;
    let bx = (node_x - bw / 2.0).clamp(gx + 8.0, gx + gw - bw - 8.0);
    let above_room = node_y - GY;
    let below_room = GY + GH - node_y;
    let by = if above_room >= bh + gap || above_room >= below_room {
        (node_y - bh - gap).max(GY + 4.0)
    } else {
        (node_y + gap).min(GY + GH - bh - 4.0)
    };
    (bx, by, bw, bh)
}
#[allow(dead_code)]
fn hud_rect_for(b: &Band, range: f64) -> (f32, f32, f32, f32) {
    hud_rect_for_at(b, range, GX, GW)
}
fn hud_value_rect_at(b: &Band, range: f64, i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let (x, y, _, _) = hud_rect_for_at(b, range, gx, gw);
    match i {
        0 => (x + 10.0, y + 48.0, 92.0, 24.0),
        1 => (x + 110.0, y + 48.0, 100.0, 24.0),
        _ => (x + 218.0, y + 48.0, 76.0, 24.0),
    }
}
#[allow(dead_code)]
fn hud_value_rect(b: &Band, range: f64, i: usize) -> (f32, f32, f32, f32) {
    hud_value_rect_at(b, range, i, GX, GW)
}
fn hud_rect_for_lift_at(b: &LiftBand, _range: f64, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let node_x = freq_x_at(b.freq, gx, gw);
    let node_y = lift_gain_y(b.gain);
    let bw = 304.0;
    let bh = 104.0;
    let gap = 36.0;
    let bx = (node_x - bw / 2.0).clamp(gx + 8.0, gx + gw - bw - 8.0);
    let above_room = node_y - GY;
    let below_room = GY + GH - node_y;
    let by = if above_room >= bh + gap || above_room >= below_room {
        (node_y - bh - gap).max(GY + 4.0)
    } else {
        (node_y + gap).min(GY + GH - bh - 4.0)
    };
    (bx, by, bw, bh)
}
#[allow(dead_code)]
fn hud_rect_for_lift(b: &LiftBand, range: f64) -> (f32, f32, f32, f32) {
    hud_rect_for_lift_at(b, range, GX, GW)
}
fn hud_value_rect_lift_at(
    b: &LiftBand,
    range: f64,
    i: usize,
    gx: f32,
    gw: f32,
) -> (f32, f32, f32, f32) {
    let (x, y, _, _) = hud_rect_for_lift_at(b, range, gx, gw);
    match i {
        0 => (x + 10.0, y + 48.0, 92.0, 24.0),
        1 => (x + 110.0, y + 48.0, 100.0, 24.0),
        _ => (x + 218.0, y + 48.0, 76.0, 24.0),
    }
}
#[allow(dead_code)]
fn hud_value_rect_lift(b: &LiftBand, range: f64, i: usize) -> (f32, f32, f32, f32) {
    hud_value_rect_lift_at(b, range, i, GX, GW)
}
fn band_rect_at(i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let slot = (gw - 46.0) / 5.0;
    (
        gx + 38.0 + i as f32 * slot,
        GRAPH_BOTTOM - 60.0,
        (slot - 4.0).max(80.0),
        52.0,
    )
}
#[allow(dead_code)]
fn band_rect(i: usize) -> (f32, f32, f32, f32) {
    band_rect_at(i, GX, GW)
}
fn band_dyn_power_rect_at(dynamic: bool, gx: f32) -> (f32, f32, f32, f32) {
    let w = if dynamic { 28.0 } else { 130.0 };
    (gx + 8.0, GRAPH_BOTTOM - 48.0, w, 28.0)
}
#[allow(dead_code)]
fn band_dyn_power_rect(dynamic: bool) -> (f32, f32, f32, f32) {
    band_dyn_power_rect_at(dynamic, GX)
}
fn band_bar_rect_at(i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let r = band_rect_at(i, gx, gw);
    (r.0 + 12.0, r.1 + 28.0, r.2 - 24.0, 16.0)
}
#[allow(dead_code)]
fn band_bar_rect(i: usize) -> (f32, f32, f32, f32) {
    band_bar_rect_at(i, GX, GW)
}
fn band_value_rect_at(i: usize, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
    let r = band_rect_at(i, gx, gw);
    (r.0 + r.2 - 68.0, r.1 + 4.0, 60.0, 20.0)
}
#[allow(dead_code)]
fn band_value_rect(i: usize) -> (f32, f32, f32, f32) {
    band_value_rect_at(i, GX, GW)
}

#[derive(Clone, Copy, PartialEq)]
enum Target {
    Global(usize),
    Band(usize),
    LiftBand(usize),
    Node(u64),
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
        let (x, y, _, _) = hud_rect_for_at(b, range, gx, gw);
        let h = self.count() as f32 * 24.0;
        (x + 40.0, (y + 32.0).min(GY + GH - h), 192.0, h)
    }
    fn rect_lift_at(self, b: &LiftBand, range: f64, gx: f32, gw: f32) -> (f32, f32, f32, f32) {
        let (x, y, _, _) = hud_rect_for_lift_at(b, range, gx, gw);
        let h = self.count() as f32 * 24.0;
        (x + 40.0, (y + 32.0).min(GY + GH - h), 192.0, h)
    }
    #[allow(dead_code)]
    fn rect(self, b: &Band, range: f64) -> (f32, f32, f32, f32) {
        self.rect_at(b, range, GX, GW)
    }
    #[allow(dead_code)]
    fn rect_lift(self, b: &LiftBand, range: f64) -> (f32, f32, f32, f32) {
        self.rect_lift_at(b, range, GX, GW)
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
                last_tick: Cell::new(None),
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
            .height(Stretch(1.0));
        },
    )
}
struct StripView {
    dyn_page: DynPage,
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
    edit: Option<ValueEdit>,
    down: (f32, f32),
    last_drag: (f32, f32),
    pending_create: Option<PendingCreate>,
    menu: Option<BandMenu>,
    active_eq: Cell<usize>,
    anim_progress: Cell<f32>,
    anim_target: Cell<f32>,
    last_tick: Cell<Option<std::time::Instant>>,
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
fn display_range(range: f64) -> f64 {
    if range.is_finite() {
        (range / 12.0).round().clamp(1.0, 6.0) * 12.0
    } else {
        24.0
    }
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
    fn eq_bounds(&self) -> (f32, f32, f32, f32) {
        let x = if self.is_pre() {
            MARGIN + SIDE_W + GAP
        } else {
            MARGIN
        };
        (x, MODULE_Y, EQ_W, MODULE_H)
    }
    fn dyn_bounds(&self) -> (f32, f32, f32, f32) {
        let x = if self.is_pre() {
            MARGIN
        } else {
            MARGIN + EQ_W + GAP
        };
        (x, MODULE_Y, SIDE_W, MODULE_H)
    }
    fn pse_bounds(&self) -> (f32, f32, f32, f32) {
        (UI_W - MARGIN - SIDE_W, MODULE_Y, SIDE_W, MODULE_H)
    }
    fn gx(&self) -> f32 {
        self.eq_bounds().0 + 44.0
    }
    fn gw(&self) -> f32 {
        self.eq_bounds().2 - 60.0
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
    fn hud_rect_for(&self, b: &Band) -> (f32, f32, f32, f32) {
        hud_rect_for_at(b, self.graph_db, self.gx(), self.gw())
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
    fn band_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        band_rect_at(i, self.gx(), self.gw())
    }
    fn band_bar_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        band_bar_rect_at(i, self.gx(), self.gw())
    }
    fn band_value_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        band_value_rect_at(i, self.gx(), self.gw())
    }
    fn band_dyn_power_rect(&self, dynamic: bool) -> (f32, f32, f32, f32) {
        band_dyn_power_rect_at(dynamic, self.gx())
    }
    #[allow(dead_code)]
    fn bottom_dyn_area(&self) -> (f32, f32, f32, f32) {
        (self.gx(), GRAPH_BOTTOM - 60.0, self.gw(), 52.0)
    }
    fn eq_tab_1_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (eq_x + 200.0, 106.0, 52.0, 24.0)
    }
    fn eq_tab_2_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (eq_x + 258.0, 106.0, 52.0, 24.0)
    }
    fn eq_add_lift_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (eq_x + 320.0, 106.0, 84.0, 24.0)
    }
    fn eq_power_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (eq_x + EQ_W - 40.0, 104.0, 28.0, 28.0)
    }
    fn scale_button_rect(&self) -> (f32, f32, f32, f32) {
        (self.gx() - 52.0, GRAPH_BOTTOM + 4.0, 42.0, 24.0)
    }
    fn scale_menu_rect(&self) -> (f32, f32, f32, f32) {
        (
            self.scale_button_rect().0 + 2.0,
            GRAPH_BOTTOM - 144.0,
            98.0,
            144.0,
        )
    }
    fn dyn_routing_button_rect(&self) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        if self.dyn_page == DynPage::Main {
            (dx + 128.0, 106.0, 44.0, 24.0)
        } else {
            (dx + 154.0, 106.0, 44.0, 24.0)
        }
    }
    fn dyn_cog_button_rect(&self) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        (dx + 176.0, 106.0, 24.0, 24.0)
    }
    fn dyn_power_button_rect(&self) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        (dx + 204.0, 106.0, 24.0, 24.0)
    }
    fn dyn_back_button_rect(&self) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        (dx + 10.0, 106.0, 60.0, 24.0)
    }
    fn dyn_mode_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        (dx + 12.0, 142.0 + i as f32 * 26.0, 212.0, 22.0)
    }
    fn pse_mode_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        let px = self.pse_bounds().0;
        (px + 12.0, 142.0 + i as f32 * 26.0, 212.0, 22.0)
    }
    fn dyn_main_thresh_slider_rect(&self) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        let y = 180.0;
        let h = MODULE_Y + MODULE_H - y - 16.0;
        (dx + 92.0, y, 36.0, h)
    }
    fn dyn_main_gr_meter_rect(&self) -> (f32, f32, f32, f32) {
        let (x, y, w, h) = self.dyn_main_thresh_slider_rect();
        (x + w, y, 16.0, h)
    }
    fn dyn_main_thresh_handle_rect(&self, thresh_y: f32) -> (f32, f32, f32, f32) {
        let (sx, _, sw, _) = self.dyn_main_thresh_slider_rect();
        let gr_w = self.dyn_main_gr_meter_rect().2;
        let bar_w = sw + gr_w + 12.0;
        (sx - 6.0, thresh_y - 6.0, bar_w, 12.0)
    }
    fn knob_cell(
        module_x: f32,
        col: usize,
        row: usize,
        y0: f32,
        pitch: f32,
    ) -> (f32, f32, f32, f32) {
        let x = if col == 0 {
            module_x + 14.0
        } else {
            module_x + 122.0
        };
        (x, y0 + row as f32 * pitch, 100.0, 82.0)
    }
    fn pse_knob_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        let px = self.pse_bounds().0;
        let (col, row) = match i {
            7 => (0, 0),
            8 => (1, 0),
            1 => (0, 1),
            2 => (1, 1),
            9 => (0, 2),
            10 => (1, 2),
            _ => (0, 0),
        };
        Self::knob_cell(px, col, row, 208.0, 96.0)
    }
    fn global_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        match self.dyn_page {
            DynPage::Controls => {
                let (col, row) = match i {
                    0 => (0, 0),
                    6 => (1, 0),
                    5 => (0, 1),
                    11 => (1, 1),
                    2 => (0, 2),
                    3 => (1, 2),
                    4 => (0, 3),
                    _ => (0, 0),
                };
                Self::knob_cell(dx, col, row, 210.0, 92.0)
            }
            DynPage::Main => {
                let dx = self.dyn_bounds().0;
                (dx + 16.0, 148.0, 72.0, 32.0)
            }
        }
    }
    fn knob_value_rect(r: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        (r.0 + 6.0, r.1 + 63.0, r.2 - 12.0, 19.0)
    }
    fn global_value_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        if self.dyn_page == DynPage::Main && i == 0 {
            let dx = self.dyn_bounds().0;
            (dx + 16.0, 154.0, 72.0, 20.0)
        } else {
            Self::knob_value_rect(self.global_rect(i))
        }
    }
    fn pse_value_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        Self::knob_value_rect(self.pse_knob_rect(i))
    }
    fn global_hit_rects(&self) -> Vec<(usize, (f32, f32, f32, f32))> {
        let mut rects: Vec<_> = self
            .global_controls()
            .iter()
            .copied()
            .map(|i| (i, self.global_rect(i)))
            .collect();
        rects.extend(
            PSE_KNOBS
                .iter()
                .copied()
                .map(|i| (i, self.pse_knob_rect(i))),
        );
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
            PSE_KNOBS
                .iter()
                .copied()
                .map(|i| (ValueTarget::Global(i), self.pse_value_rect(i))),
        );
        fields
    }
    fn value_color(&self, target: ValueTarget) -> C {
        match target {
            ValueTarget::Global(2 | 3) => TEAL,
            ValueTarget::Global(_) => GOLD,
            ValueTarget::Band(_) => self
                .selected
                .map(|id| {
                    let num = if id >= EQ2_ID_BASE {
                        id - EQ2_ID_BASE
                    } else {
                        id
                    };
                    COLORS[(num as usize - 1) % COLORS.len()]
                })
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
            if b.shape.has_gain() && b.dynamic {
                for i in 0..5 {
                    let r = self.band_value_rect(i);
                    if inside(x, y, r) {
                        return Some((ValueTarget::Band(i + 3), r));
                    }
                }
            }
        }
        self.global_value_hits()
            .into_iter()
            .find(|(_, r)| inside(x, y, *r))
    }
    fn start_edit(&mut self, target: ValueTarget, rect: (f32, f32, f32, f32)) {
        let value = match target {
            ValueTarget::Global(i) => {
                let value = self.param(i).value() as f64;
                if i == 0 {
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
        self.edit = Some(ValueEdit {
            target,
            rect,
            original: text.clone(),
            cursor: text.len(),
            anchor: 0,
            text,
            invalid: false,
        });
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
                let norm = p.preview_normalized(if i == 0 { -value as f32 } else { value as f32 });
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
                    if b.shape.has_gain() {
                        fields.extend(
                            (0..5).map(|i| (ValueTarget::Band(i + 3), self.band_value_rect(i))),
                        );
                    }
                }
                fields.extend(self.global_value_hits());
                let index = fields.iter().position(|(t, _)| *t == target).unwrap_or(0);
                let next = if cx.modifiers().shift() {
                    (index + fields.len() - 1) % fields.len()
                } else {
                    (index + 1) % fields.len()
                };
                self.start_edit(fields[next].0, fields[next].1);
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
            DynPage::Controls => &[0, 6, 5, 11, 2, 3, 4],
        }
    }
    fn param(&self, i: usize) -> &FloatParam {
        match i {
            0 => &self.params.compression,
            1 => &self.params.gate,
            2 => &self.params.sc_hpf,
            3 => &self.params.dry,
            4 => &self.params.wet,
            5 => &self.params.comp_attack,
            6 => &self.params.comp_ratio,
            7 => &self.params.pse_depth,
            8 => &self.params.pse_hysteresis,
            9 => &self.params.pse_knee,
            10 => &self.params.pse_time,
            11 => &self.params.comp_release,
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
    fn change(&self, f: impl FnOnce(&mut Band)) {
        let Some(id) = self.selected else {
            return;
        };
        if id >= LIFT_ID_BASE {
            return;
        }
        if id >= EQ2_ID_BASE {
            if let Some(b) = self
                .params
                .eq2_bands
                .lock()
                .unwrap()
                .iter_mut()
                .find(|b| b.id == id)
            {
                f(b);
                b.sanitize();
            }
        } else if let Some(b) = self
            .params
            .bands
            .lock()
            .unwrap()
            .iter_mut()
            .find(|b| b.id == id)
        {
            f(b);
            b.sanitize();
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
        let is_eq2 = self.active_eq.get() == 1;
        let id_base = if is_eq2 { EQ2_ID_BASE } else { 0 };
        let mut bands = if is_eq2 {
            self.params.eq2_bands.lock().unwrap()
        } else {
            self.params.bands.lock().unwrap()
        };
        let id = bands.iter().map(|b| b.id).max().unwrap_or(id_base) + 1;
        let shape = infer_shape((x - GX) / GW, (y - GY) / GH, curve);
        bands.push(Band {
            id,
            shape,
            freq: x_freq(x),
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
            dynamic: dynamic && shape.has_gain(),
            ..Band::default()
        });
        drop(bands);
        self.select(Some(id));
        self.drag = Some(Target::Node(id));
        self.last_drag = (x, y);
    }
    fn apply_drag(&mut self, cx: &mut EventContext, x: f32, y: f32, shift: bool, cmd: bool) {
        let graph_db = self.graph_db;
        let (_lx, ly) = self.last_drag;
        let dy = (ly - y) as f64;
        self.last_drag = (x, y);
        match self.drag {
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
                } else if cmd {
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
                } else {
                    let cur = p.unmodulated_normalized_value();
                    let delta = (dy * if shift { 0.0015 } else { 0.007 }) as f32;
                    let norm = (cur + if i == 0 { -delta } else { delta }).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                }
            }
            Some(Target::Band(i)) => {
                let r = self.band_rect(i);
                let n = ((x - r.0 - 12.0) / (r.2 - 24.0)).clamp(0.0, 1.0) as f64;
                self.change(|b| match i {
                    0 => b.threshold = -60.0 + 60.0 * n,
                    1 => b.ratio = 1.0 + 19.0 * n,
                    2 => b.attack = 0.1 * 2000.0_f64.powf(n),
                    3 => b.release = 10.0 * 200.0_f64.powf(n),
                    _ => b.range = 24.0 * n,
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
    fn delete(&mut self) {
        if let Some(id) = self.selected {
            if id >= LIFT_ID_BASE {
                self.params
                    .lift_bands
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
            let scale = bounds.w / UI_W;
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
                    WindowEvent::KeyDown(code, _) => {
                        self.edit_key(cx, *code);
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
                            cx.needs_redraw();
                            return;
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
                    if self.drag.is_some() {
                        let shift = cx.modifiers().shift();
                        let cmd = cx.modifiers().command();
                        self.apply_drag(cx, x, y, shift, cmd);
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseDown(MouseButton::Left) => {
                    cx.focus();
                    if inside(x, y, THEME_BUTTON) {
                        preferences::toggle();
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.dyn_routing_button_rect()) {
                        self.toggle(cx, &self.params.comp_pre);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.dyn_power_button_rect()) {
                        self.toggle(cx, &self.params.comp_on);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.pse_mode_rect(0)) {
                        self.toggle(cx, &self.params.pse_peak);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.pse_mode_rect(1)) {
                        self.toggle(cx, &self.params.pse_listen);
                        cx.needs_redraw();
                        return;
                    }
                    match self.dyn_page {
                        DynPage::Main => {
                            if inside(x, y, self.dyn_cog_button_rect()) {
                                self.dyn_page = DynPage::Controls;
                                cx.needs_redraw();
                                return;
                            }
                            let r = self.dyn_main_thresh_slider_rect();
                            let gr = self.dyn_main_gr_meter_rect();
                            let combined = (r.0 - 10.0, r.1 - 10.0, r.2 + gr.2 + 20.0, r.3 + 20.0);
                            if inside(x, y, combined) {
                                self.drag = Some(Target::Global(0));
                                self.last_drag = (x, y);
                                let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                                cx.emit(RawParamEvent::BeginSetParameter(self.param(0).as_ptr()));
                                cx.emit(RawParamEvent::SetParameterNormalized(
                                    self.param(0).as_ptr(),
                                    norm,
                                ));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                        }
                        DynPage::Controls => {
                            if inside(x, y, self.dyn_back_button_rect()) {
                                self.dyn_page = DynPage::Main;
                                cx.needs_redraw();
                                return;
                            }
                            for (i, p) in [
                                (0, &self.params.soft_knee),
                                (1, &self.params.auto_makeup),
                                (2, &self.params.stereo_link),
                            ] {
                                if inside(x, y, self.dyn_mode_rect(i)) {
                                    self.toggle(cx, p);
                                    cx.needs_redraw();
                                    return;
                                }
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
                        self.start_edit(target, rect);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.eq_tab_1_rect()) {
                        if self.active_eq.get() != 0 {
                            self.active_eq.set(0);
                            self.anim_target.set(0.0);
                            self.select(None);
                            self.menu = None;
                            self.edit = None;
                            cx.needs_redraw();
                        }
                        return;
                    }
                    if inside(x, y, self.eq_tab_2_rect()) {
                        if self.active_eq.get() != 1 {
                            self.active_eq.set(1);
                            self.anim_target.set(1.0);
                            self.select(None);
                            self.menu = None;
                            self.edit = None;
                            cx.needs_redraw();
                        }
                        return;
                    }
                    if inside(x, y, self.eq_add_lift_rect()) {
                        let mut lift_bands = self.params.lift_bands.lock().unwrap();
                        let id = lift_bands
                            .iter()
                            .map(|b| b.id)
                            .max()
                            .unwrap_or(LIFT_ID_BASE)
                            + 1;
                        lift_bands.push(LiftBand {
                            id,
                            ..LiftBand::default()
                        });
                        drop(lift_bands);
                        if self.active_eq.get() != 0 {
                            self.active_eq.set(0);
                            self.anim_target.set(0.0);
                        }
                        self.select(Some(id));
                        self.menu = None;
                        self.edit = None;
                        cx.needs_redraw();
                        return;
                    }
                    self.down = (x, y);
                    self.last_drag = (x, y);
                    if inside(x, y, self.eq_power_rect()) {
                        if self.active_eq.get() == 0 {
                            self.toggle(cx, &self.params.eq_on);
                        } else {
                            self.toggle(cx, &self.params.eq2_on);
                        }
                    } else {
                        if (self.anim_progress.get() - self.anim_target.get()).abs() > 0.01
                            && inside(x, y, self.eq_bounds())
                        {
                            return;
                        }
                        // Check if click hits selected band HUD
                        let mut hud_consumed = false;
                        if let Some(sel_id) = self.selected {
                            if sel_id >= LIFT_ID_BASE {
                                if let Some(b) = self.selected_lift() {
                                    let (bx, by, bw, bh) = self.hud_rect_for_lift(&b);
                                    if inside(x, y, (bx, by, bw, bh)) {
                                        hud_consumed = true;
                                        if inside(x, y, (bx + 6.0, by + 6.0, 26.0, 26.0)) {
                                            self.change_lift(|b| b.enabled = !b.enabled);
                                        } else if inside(x, y, (bx + 40.0, by + 6.0, 148.0, 26.0)) {
                                            self.menu = Some(BandMenu::Shape);
                                        } else if inside(x, y, (bx + 222.0, by + 6.0, 28.0, 26.0)) {
                                            let cur = self.shared.solo_id.load(Ordering::Relaxed);
                                            if cur == b.id {
                                                self.shared.solo_id.store(0, Ordering::Relaxed);
                                            } else {
                                                self.shared.solo_id.store(b.id, Ordering::Relaxed);
                                            }
                                        } else if inside(
                                            x,
                                            y,
                                            (bx + bw - 32.0, by + 6.0, 26.0, 26.0),
                                        ) {
                                            self.delete();
                                        } else if inside(x, y, (bx + 10.0, by + 78.0, 108.0, 22.0))
                                            && b.shape.is_cut()
                                        {
                                            self.menu = Some(BandMenu::Order);
                                        }
                                    }
                                }
                            } else {
                                let band_opt = self.find_band(sel_id);
                                if let Some(b) = band_opt {
                                    let (bx, by, bw, bh) = self.hud_rect_for(&b);
                                    if inside(x, y, (bx, by, bw, bh)) {
                                        hud_consumed = true;
                                        if inside(x, y, (bx + 6.0, by + 6.0, 26.0, 26.0)) {
                                            self.change(|b| b.enabled = !b.enabled);
                                        } else if inside(x, y, (bx + 40.0, by + 6.0, 148.0, 26.0)) {
                                            self.menu = Some(BandMenu::Shape);
                                        } else if inside(x, y, (bx + 222.0, by + 6.0, 28.0, 26.0)) {
                                            let cur = self.shared.solo_id.load(Ordering::Relaxed);
                                            if cur == b.id {
                                                self.shared.solo_id.store(0, Ordering::Relaxed);
                                            } else {
                                                self.shared.solo_id.store(b.id, Ordering::Relaxed);
                                            }
                                        } else if inside(
                                            x,
                                            y,
                                            (bx + bw - 32.0, by + 6.0, 26.0, 26.0),
                                        ) {
                                            self.delete();
                                        } else if inside(x, y, (bx + 10.0, by + 78.0, 108.0, 22.0))
                                            && b.shape.is_cut()
                                        {
                                            self.menu = Some(BandMenu::Order);
                                        }
                                    }
                                }
                            }
                        }
                        if !hud_consumed {
                            if inside(x, y, self.graph_area()) {
                                let is_eq2 = self.active_eq.get() == 1;
                                let lift_hit = if !is_eq2 {
                                    self.params
                                        .lift_bands
                                        .lock()
                                        .unwrap()
                                        .iter()
                                        .rev()
                                        .find(|b| {
                                            ((x - self.freq_x(b.freq)).powi(2)
                                                + (y - lift_gain_y(b.gain)).powi(2))
                                            .sqrt()
                                                < 16.0
                                        })
                                        .map(|b| b.id)
                                } else {
                                    None
                                };
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
                                    let bands = if is_eq2 {
                                        self.params.eq2_bands.lock().unwrap().clone()
                                    } else {
                                        self.params.bands.lock().unwrap().clone()
                                    };
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
                                        if cx.modifiers().alt() {
                                            self.change(|b| b.enabled = !b.enabled);
                                        } else {
                                            self.drag = Some(Target::Node(id));
                                            self.last_drag = (x, y);
                                            cx.capture();
                                        }
                                    } else {
                                        let sr =
                                            self.shared.sample_rate.load(Ordering::Relaxed) as f64;
                                        let config = self.params.processing_config();
                                        let eq_sr = config.mode.rate(sr);
                                        let db = bands
                                            .iter()
                                            .filter(|b| b.enabled)
                                            .map(|b| {
                                                BandCoeffs::make(b, eq_sr)
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
                                            self.pending_create = Some(PendingCreate::Background);
                                            cx.capture();
                                        }
                                    }
                                }
                            } else if let Some(i) = self
                                .global_hit_rects()
                                .into_iter()
                                .find(|(_, r)| inside(x, y, *r))
                                .map(|(i, _)| i)
                            {
                                self.drag = Some(Target::Global(i));
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(self.param(i).as_ptr()));
                                cx.capture();
                            } else if self.is_lift_selected() {
                                let badge_rect = (self.gx() + 8.0, GRAPH_BOTTOM - 48.0, 32.0, 28.0);
                                if inside(x, y, badge_rect) {
                                    self.change_lift(|b| b.enabled = !b.enabled);
                                } else if let Some(i) =
                                    (0..5).find(|i| inside(x, y, self.band_bar_rect(*i)))
                                {
                                    self.drag = Some(Target::LiftBand(i));
                                    self.last_drag = (x, y);
                                    cx.capture();
                                    self.apply_drag(cx, x, y, false, false);
                                }
                            } else if self
                                .selected
                                .and_then(|id| self.find_band(id))
                                .is_some_and(|b| b.shape.has_gain())
                            {
                                let is_dynamic = self
                                    .selected
                                    .and_then(|id| self.find_band(id))
                                    .map(|b| b.dynamic)
                                    .unwrap_or(false);
                                if inside(x, y, self.band_dyn_power_rect(is_dynamic)) {
                                    self.change(|b| b.dynamic = !b.dynamic);
                                } else if is_dynamic {
                                    if let Some(i) =
                                        (0..5).find(|i| inside(x, y, self.band_bar_rect(*i)))
                                    {
                                        self.drag = Some(Target::Band(i));
                                        self.last_drag = (x, y);
                                        cx.capture();
                                        self.apply_drag(cx, x, y, false, false);
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
                    if self.dyn_page == DynPage::Main {
                        let r = self.dyn_main_thresh_slider_rect();
                        if inside(x, y, (r.0 - 10.0, r.1 - 10.0, r.2 + 20.0, r.3 + 20.0)) {
                            let p = self.param(0);
                            cx.emit(RawParamEvent::BeginSetParameter(p.as_ptr()));
                            cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), 0.0));
                            cx.emit(RawParamEvent::EndSetParameter(p.as_ptr()));
                            cx.needs_redraw();
                            return;
                        }
                    }
                    if inside(x, y, self.graph_area()) && self.drag.is_none() {
                        let mut over_hud = false;
                        if let Some(sel_id) = self.selected {
                            if sel_id >= LIFT_ID_BASE {
                                if let Some(b) = self.selected_lift() {
                                    if inside(x, y, self.hud_rect_for_lift(&b)) {
                                        over_hud = true;
                                    }
                                }
                            } else if let Some(b) = self.find_band(sel_id) {
                                if inside(x, y, self.hud_rect_for(&b)) {
                                    over_hud = true;
                                }
                            }
                        }
                        if !over_hud {
                            let is_eq2 = self.active_eq.get() == 1;
                            let lift_hit = if !is_eq2 {
                                self.params
                                    .lift_bands
                                    .lock()
                                    .unwrap()
                                    .iter()
                                    .rev()
                                    .find(|b| {
                                        ((x - self.freq_x(b.freq)).powi(2)
                                            + (y - lift_gain_y(b.gain)).powi(2))
                                        .sqrt()
                                            < 16.0
                                    })
                                    .map(|b| b.id)
                            } else {
                                None
                            };
                            if let Some(id) = lift_hit {
                                self.select(Some(id));
                                self.change_lift(|b| b.enabled = !b.enabled);
                            } else {
                                let bands = if is_eq2 {
                                    self.params.eq2_bands.lock().unwrap().clone()
                                } else {
                                    self.params.bands.lock().unwrap().clone()
                                };
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
                WindowEvent::MouseUp(MouseButton::Left) => {
                    if let Some(Target::Global(i)) = self.drag {
                        cx.emit(RawParamEvent::EndSetParameter(self.param(i).as_ptr()));
                    }
                    if self.pending_create.is_some() {
                        self.select(None);
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
                                if inside(x, y, self.hud_rect_for(&b)) {
                                    over_hud = true;
                                }
                            }
                        }
                        if !over_hud {
                            let is_eq2 = self.active_eq.get() == 1;
                            let lift_id = if !is_eq2 {
                                self.params
                                    .lift_bands
                                    .lock()
                                    .unwrap()
                                    .iter()
                                    .find(|b| {
                                        (x - self.freq_x(b.freq)).abs() < 15.0
                                            && (y - lift_gain_y(b.gain)).abs() < 15.0
                                    })
                                    .map(|b| b.id)
                            } else {
                                None
                            };
                            if lift_id.is_some() {
                                self.select(lift_id);
                                self.delete();
                            } else {
                                let bands = if is_eq2 {
                                    self.params.eq2_bands.lock().unwrap().clone()
                                } else {
                                    self.params.bands.lock().unwrap().clone()
                                };
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
                    if inside(x, y, (self.gx() - 50.0, GY, 36.0, GRAPH_BOTTOM - GY + 32.0))
                        && *dy != 0.0
                    {
                        self.set_graph_range(self.graph_db - dy.signum() as f64 * 12.0);
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
                        let is_eq2 = self.active_eq.get() == 1;
                        if self.selected.is_none() {
                            let lift_hit = if !is_eq2 {
                                self.params
                                    .lift_bands
                                    .lock()
                                    .unwrap()
                                    .iter()
                                    .rev()
                                    .find(|b| {
                                        ((x - self.freq_x(b.freq)).powi(2)
                                            + (y - lift_gain_y(b.gain)).powi(2))
                                        .sqrt()
                                            < 18.0
                                    })
                                    .map(|b| b.id)
                            } else {
                                None
                            };
                            if lift_hit.is_some() {
                                self.select(lift_hit);
                            } else {
                                let bands = if is_eq2 {
                                    self.params.eq2_bands.lock().unwrap().clone()
                                } else {
                                    self.params.bands.lock().unwrap().clone()
                                };
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
        if self.font.get().is_none() {
            self.font.set(
                canvas
                    .add_font_mem(include_bytes!("assets/JetBrainsMono-Medium.ttf"))
                    .ok(),
            );
        }
        if self.signature.get().is_none() {
            self.signature.set(
                canvas
                    .add_font_mem(include_bytes!("assets/Allura-Regular.ttf"))
                    .ok(),
            );
        }
        let mut d = Draw {
            c: canvas,
            s: bounds.w / UI_W,
            ox: bounds.x,
            oy: bounds.y,
            font: self.font.get(),
            light: preferences::light(),
            offset_x: 0.0,
            alpha_mul: 1.0,
        };
        d.rect(0.0, 0.0, UI_W, UI_H, BG);
        d.rect(0.0, 0.0, UI_W, HEADER_H, PANEL);
        d.text(32.0, 46.0, "dB", 28.0, GOLD);
        d.text(88.0, 46.0, "SIGNATURE CHANNEL STRIP", 16.0, TEXT);
        d.signature(self.signature.get());

        let now = std::time::Instant::now();
        let dt = self
            .last_tick
            .get()
            .map(|prev| (now - prev).as_secs_f32())
            .unwrap_or(0.016)
            .clamp(0.001, 0.1);
        self.last_tick.set(Some(now));

        let cur_p = self.anim_progress.get();
        let target_p = self.anim_target.get();
        let next_p = if cur_p < target_p {
            (cur_p + dt / 0.25).min(target_p)
        } else if cur_p > target_p {
            (cur_p - dt / 0.25).max(target_p)
        } else {
            cur_p
        };
        self.anim_progress.set(next_p);

        // Quintic smootherstep ease-in-out: 6t^5 - 15t^4 + 10t^3
        let ease = next_p * next_p * next_p * (next_p * (next_p * 6.0 - 15.0) + 10.0);
        let offset_1 = -ease * GRAPH_CLIP_W;
        let offset_2 = (1.0 - ease) * GRAPH_CLIP_W;

        let is_eq2 = self.active_eq.get() == 1;
        let eq1_on = self.params.eq_on.value();
        let eq2_on = self.params.eq2_on.value();
        let active_bypassed = if is_eq2 { !eq2_on } else { !eq1_on };
        let bands = self.params.bands.lock().unwrap().clone();
        let eq2_bands = self.params.eq2_bands.lock().unwrap().clone();
        let lift_bands = self.params.lift_bands.lock().unwrap().clone();

        let eq_b = self.eq_bounds();
        let eq_x = eq_b.0;
        let gx = self.gx();
        let gw = self.gw();
        d.rect(eq_b.0, eq_b.1, eq_b.2, eq_b.3, PANEL);
        d.outline(eq_b, LINE);

        d.text(
            eq_x + 12.0,
            123.0,
            if self.is_pre() { "02" } else { "01" },
            13.0,
            if active_bypassed { MUTED } else { TEAL },
        );
        d.text(
            eq_x + 48.0,
            123.0,
            "PARAMETRIC EQ",
            16.0,
            if active_bypassed { MUTED } else { TEXT },
        );
        d.tab_button(
            self.eq_tab_1_rect(),
            "EQ 1",
            !is_eq2,
            if eq1_on { TEAL } else { MUTED },
        );
        d.tab_button(
            self.eq_tab_2_rect(),
            "EQ 2",
            is_eq2,
            if eq2_on { TEAL } else { MUTED },
        );

        let add_lift_rect = self.eq_add_lift_rect();
        let add_lift_hover = self
            .hover
            .is_some_and(|(hx, hy)| inside(hx, hy, add_lift_rect));
        d.rect(
            add_lift_rect.0,
            add_lift_rect.1,
            add_lift_rect.2,
            add_lift_rect.3,
            PANEL,
        );
        d.outline(
            add_lift_rect,
            if add_lift_hover { LIFT_COLOR } else { LINE },
        );
        d.text_centered(
            add_lift_rect.0 + add_lift_rect.2 * 0.5,
            add_lift_rect.1 + add_lift_rect.3 * 0.5 + 4.0,
            "+ ADD LIFT",
            11.0,
            if add_lift_hover { LIFT_COLOR } else { MUTED },
        );

        if active_bypassed {
            d.text(
                eq_x + 450.0,
                123.0,
                if is_eq2 {
                    "EQ 2 STAGE BYPASSED"
                } else {
                    "EQ 1 STAGE BYPASSED"
                },
                12.0,
                GOLD,
            );
        } else if self.shared.solo_id.load(Ordering::Relaxed) != 0 {
            d.text(eq_x + 450.0, 123.0, "SOLO AUDITION ACTIVE", 12.0, GOLD);
        }
        d.bypass_button(self.eq_power_rect(), active_bypassed, TEAL);

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
        for (k, val) in raw_lift_gr.iter_mut().enumerate() {
            *val = self.shared.lift_bin_gr[k].load(Ordering::Relaxed) as f64;
        }
        let mut smooth_lift_gr = [0.0_f64; 256];
        for (k, smooth_val) in smooth_lift_gr.iter_mut().enumerate() {
            let mut sum = 0.0;
            let mut w_sum = 0.0;
            for offset in -3..=3 {
                let idx = (k as isize + offset).clamp(0, 255) as usize;
                let w = match offset.abs() {
                    0 => 0.38,
                    1 => 0.24,
                    2 => 0.06,
                    3 => 0.01,
                    _ => 0.0,
                };
                sum += raw_lift_gr[idx] * w;
                w_sum += w;
            }
            *smooth_val = sum / w_sum;
        }

        let scale_btn = self.scale_button_rect();
        let scale_menu_r = self.scale_menu_rect();
        let draw_grid_and_spectrum = |d: &mut Draw, bypassed: bool, eq_idx: usize| {
            for db in (-(self.graph_db as i32)..=self.graph_db as i32).step_by(12) {
                let y = db_y(db as f64, self.graph_db);
                d.line(
                    gx,
                    y,
                    gx + gw,
                    y,
                    if db == 0 { C::rgb(75, 81, 85) } else { LINE },
                    1.0,
                );
                d.text(gx - 41.0, y + 5.0, &format!("{:+}", db), 13.0, MUTED);
            }
            for (freq, label) in [
                (20.0, "20"),
                (50.0, "50"),
                (100.0, "100"),
                (200.0, "200"),
                (500.0, "500"),
                (1000.0, "1k"),
                (2000.0, "2k"),
                (5000.0, "5k"),
                (10000.0, "10k"),
                (20000.0, "20k"),
            ] {
                let x = freq_x_at(freq, gx, gw);
                d.line(x, GY, x, GRAPH_BOTTOM, LINE, 1.0);
                d.text(x - 9.0, GRAPH_BOTTOM + 16.0, label, 12.0, MUTED);
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
            d.rect(scale_btn.0, scale_btn.1, scale_btn.2, scale_btn.3, PANEL);
            d.text_centered(
                scale_btn.0 + scale_btn.2 * 0.5,
                scale_btn.1 + 16.0,
                &format!("±{} ▾", self.graph_db as i32),
                9.5,
                MUTED,
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
                        &format!("±{} dB", *range as i32),
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

            let mut sum = vec![0.0; 420];
            for b in &bands {
                if !b.enabled {
                    continue;
                }
                let coeff = BandCoeffs::make(b, eq_sr);
                let color = if eq1_bypassed {
                    MUTED
                } else {
                    COLORS[(b.id as usize - 1) % COLORS.len()]
                };
                let points: Vec<_> = (0..420)
                    .map(|i| {
                        let x = gx + gw * i as f32 / 419.0;
                        let db = coeff.response(x_freq_at(x, gx, gw).min(sr * 0.49), eq_sr);
                        sum[i] += db;
                        (x, db_y(db, self.graph_db))
                    })
                    .collect();
                if Some(b.id) == self.selected {
                    let mut fill = color;
                    fill.a = if eq1_bypassed { 0.02 } else { 0.07 };
                    d.area(&points, db_y(0.0, self.graph_db), fill);
                    d.poly(&points, color, 1.2);
                }
            }
            let points: Vec<_> = sum
                .iter()
                .enumerate()
                .map(|(i, db)| (gx + gw * i as f32 / 419.0, db_y(*db, self.graph_db)))
                .collect();
            d.poly(&points, if eq1_bypassed { MUTED } else { GOLD }, 2.2);

            for b in &lift_bands {
                if !b.enabled {
                    continue;
                }
                let color = if eq1_bypassed { MUTED } else { LIFT_COLOR };
                let graph_bottom_y = GY + GH;

                // Smooth influence curve using center of bins and Catmull-Rom curve fit
                let points: Vec<_> = (0..=500)
                    .map(|i| {
                        let x = gx + gw * (i as f32 / 500.0);
                        let f = x_freq_at(x, gx, gw);
                        let inf = filter_influence(b.shape, b.freq, b.q, b.order, f, sr);
                        let t = ((f / 20.0).log10() / 3.0).clamp(0.0, 1.0);
                        let gr_db = lift_bin_curve_fit(&smooth_lift_gr, t);
                        let cut_gain = (b.gain - gr_db * 1.5).clamp(-100.0, 0.0);
                        let norm = ((cut_gain + 100.0) / 100.0) as f32;
                        let y = graph_bottom_y - (GH * norm * inf as f32);
                        (x, y)
                    })
                    .collect();

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
                    a: if eq1_bypassed { 0.05 } else { 0.22 },
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
                    a: if eq1_bypassed { 0.25 } else { 0.90 },
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

                        d.area_gradient_span(
                            left_pts,
                            graph_bottom_y,
                            x_left,
                            x_center,
                            c_trans,
                            c_solid,
                        );
                        d.poly_gradient_span(left_pts, x_left, x_center, s_trans, s_solid, 1.8);

                        d.area_gradient_span(
                            right_pts,
                            graph_bottom_y,
                            x_center,
                            x_right,
                            c_solid,
                            c_trans,
                        );
                        d.poly_gradient_span(right_pts, x_center, x_right, s_solid, s_trans, 1.8);
                    }
                    Shape::HighShelf | Shape::LowCut => {
                        let oct_span = (1.5 / b.q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
                        let f_fade_start = (b.freq * 2.0_f64.powf(-oct_span)).max(20.0);
                        let f_fade_end = (b.freq * 2.0_f64.powf(oct_span * 0.5)).min(20000.0);
                        let x_start = freq_x_at(f_fade_start, gx, gw);
                        let x_end = freq_x_at(f_fade_end, gx, gw);

                        d.area_gradient_span(
                            &points,
                            graph_bottom_y,
                            x_start,
                            x_end,
                            c_trans,
                            c_solid,
                        );
                        d.poly_gradient_span(&points, x_start, x_end, s_trans, s_solid, 1.8);
                    }
                    Shape::LowShelf | Shape::HighCut => {
                        let oct_span = (1.5 / b.q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
                        let f_fade_start = (b.freq * 2.0_f64.powf(-oct_span * 0.5)).max(20.0);
                        let f_fade_end = (b.freq * 2.0_f64.powf(oct_span)).min(20000.0);
                        let x_start = freq_x_at(f_fade_start, gx, gw);
                        let x_end = freq_x_at(f_fade_end, gx, gw);

                        d.area_gradient_span(
                            &points,
                            graph_bottom_y,
                            x_start,
                            x_end,
                            c_solid,
                            c_trans,
                        );
                        d.poly_gradient_span(&points, x_start, x_end, s_solid, s_trans, 1.8);
                    }
                }

                if b.gain <= -98.0 && Some(b.id) == self.selected {
                    let guide: Vec<_> = (0..=500)
                        .map(|i| {
                            let x = gx + gw * (i as f32 / 500.0);
                            let f = x_freq_at(x, gx, gw);
                            let inf = filter_influence(b.shape, b.freq, b.q, b.order, f, sr) as f32;
                            let y = graph_bottom_y - (GH * 0.08 * inf);
                            (x, y)
                        })
                        .collect();
                    let g_trans = C {
                        r: color.r,
                        g: color.g,
                        b: color.b,
                        a: 0.0,
                    };
                    let g_solid = C {
                        r: color.r,
                        g: color.g,
                        b: color.b,
                        a: 0.35,
                    };
                    match b.shape {
                        Shape::Bell | Shape::BandPass | Shape::Notch => {
                            let oct_span = (2.0 / b.q.clamp(0.15, 18.0)).clamp(0.5, 4.0);
                            let x_left =
                                freq_x_at((b.freq * 2.0_f64.powf(-oct_span)).max(20.0), gx, gw);
                            let x_center = freq_x_at(b.freq, gx, gw);
                            let x_right =
                                freq_x_at((b.freq * 2.0_f64.powf(oct_span)).min(20000.0), gx, gw);
                            let c_idx = guide
                                .iter()
                                .position(|(x, _)| *x >= x_center)
                                .unwrap_or(guide.len() / 2);
                            d.poly_gradient_span(
                                &guide[..=c_idx],
                                x_left,
                                x_center,
                                g_trans,
                                g_solid,
                                1.0,
                            );
                            d.poly_gradient_span(
                                &guide[c_idx..],
                                x_center,
                                x_right,
                                g_solid,
                                g_trans,
                                1.0,
                            );
                        }
                        Shape::HighShelf | Shape::LowCut => {
                            let oct_span = (1.5 / b.q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
                            let x_start =
                                freq_x_at((b.freq * 2.0_f64.powf(-oct_span)).max(20.0), gx, gw);
                            let x_end = freq_x_at(
                                (b.freq * 2.0_f64.powf(oct_span * 0.5)).min(20000.0),
                                gx,
                                gw,
                            );
                            d.poly_gradient_span(&guide, x_start, x_end, g_trans, g_solid, 1.0);
                        }
                        Shape::LowShelf | Shape::HighCut => {
                            let oct_span = (1.5 / b.q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
                            let x_start = freq_x_at(
                                (b.freq * 2.0_f64.powf(-oct_span * 0.5)).max(20.0),
                                gx,
                                gw,
                            );
                            let x_end =
                                freq_x_at((b.freq * 2.0_f64.powf(oct_span)).min(20000.0), gx, gw);
                            d.poly_gradient_span(&guide, x_start, x_end, g_solid, g_trans, 1.0);
                        }
                    }
                }
            }

            for b in &bands {
                let color = if !b.enabled || eq1_bypassed {
                    MUTED
                } else {
                    COLORS[(b.id as usize - 1) % COLORS.len()]
                };
                let x = freq_x_at(b.freq, gx, gw);
                let y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db);
                if Some(b.id) == self.selected {
                    d.circle(x, y, 12.0, color, false);
                }
                if b.dynamic && b.shape.has_gain() {
                    d.line(x, y, x, db_y(b.gain - b.range, self.graph_db), color, 3.0);
                    d.circle(x, db_y(b.gain - b.range, self.graph_db), 3.0, color, true);
                }
                d.circle(x, y, 7.0, color, true);
                d.text(x - 4.0, y - 17.0, &b.id.to_string(), 11.0, color);
            }

            for (idx, b) in lift_bands.iter().enumerate() {
                let color = if !b.enabled || eq1_bypassed {
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
                    format!("LIFT {}", idx + 1)
                } else {
                    "LIFT".to_string()
                };
                d.text(x - 12.0, y - 17.0, &label, 10.0, color);
            }

            if let Some(b) = bands.iter().find(|b| Some(b.id) == self.selected) {
                let color = if eq1_bypassed {
                    MUTED
                } else {
                    COLORS[(b.id as usize - 1) % COLORS.len()]
                };
                let is_solo = self.shared.solo_id.load(Ordering::Relaxed) == b.id;
                d.band_hud(
                    b,
                    is_solo,
                    color,
                    self.graph_db,
                    config.mode == ProcessingMode::LinearPhase,
                    gx,
                    gw,
                );
            } else if let Some(b) = lift_bands.iter().find(|b| Some(b.id) == self.selected) {
                let color = if eq1_bypassed { MUTED } else { LIFT_COLOR };
                let is_solo = self.shared.solo_id.load(Ordering::Relaxed) == b.id;
                d.band_hud_lift(b, is_solo, color, self.graph_db, gx, gw);
            }

            // EQ 1 Bottom dynamic area
            if self.is_lift_selected() {
                if let Some(b) = self.selected_lift() {
                    let c = if !eq1_on { MUTED } else { LIFT_COLOR };
                    let badge_rect = (gx + 8.0, GRAPH_BOTTOM - 48.0, 32.0, 28.0);
                    d.bypass_button(badge_rect, !b.enabled, LIFT_COLOR);

                    for (i, label, val, n) in [
                        (
                            0,
                            "THRESHOLD",
                            format!("{:.1} dB", b.threshold),
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
                        (4, "RANGE", format!("{:.1} dB", b.range), b.range / 24.0),
                    ] {
                        d.control(band_rect_at(i, gx, gw), label, &val, n as f32, c);
                    }
                }
            } else if let Some(b) = self
                .selected
                .and_then(|id| self.find_band(id))
                .filter(|b| b.id < EQ2_ID_BASE && b.shape.has_gain())
            {
                let color = COLORS[(b.id as usize - 1) % COLORS.len()];
                let power_rect = band_dyn_power_rect_at(b.dynamic, gx);
                let hovered = self
                    .hover
                    .is_some_and(|(hx, hy)| inside(hx, hy, power_rect));
                d.rect(
                    power_rect.0,
                    power_rect.1,
                    power_rect.2,
                    power_rect.3,
                    PANEL,
                );
                if b.dynamic {
                    let c = if eq1_on { color } else { MUTED };
                    d.outline(
                        power_rect,
                        if hovered {
                            TEXT
                        } else if eq1_on {
                            c
                        } else {
                            LINE
                        },
                    );
                    d.power_icon(
                        power_rect.0 + power_rect.2 * 0.5,
                        power_rect.1 + power_rect.3 * 0.5,
                        c,
                    );
                    for (i, label, val, n) in [
                        (
                            0,
                            "THRESHOLD",
                            format!("{:.1} dB", b.threshold),
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
                        (4, "RANGE", format!("{:.1} dB", b.range), b.range / 24.0),
                    ] {
                        d.control(band_rect_at(i, gx, gw), label, &val, n as f32, c);
                    }
                } else {
                    d.outline(power_rect, if hovered { TEXT } else { LINE });
                    d.power_icon(
                        power_rect.0 + 14.0,
                        power_rect.1 + power_rect.3 * 0.5,
                        if hovered { TEXT } else { MUTED },
                    );
                    d.text(
                        power_rect.0 + 28.0,
                        power_rect.1 + power_rect.3 * 0.5 + 4.0,
                        "BAND DYNAMICS",
                        11.0,
                        if hovered { TEXT } else { MUTED },
                    );
                }
            } else if self.selected.is_none() {
                let add_rect = (gx + 8.0, GRAPH_BOTTOM - 50.0, gw - 16.0, 32.0);
                d.rect(add_rect.0, add_rect.1, add_rect.2, add_rect.3, PANEL);
                d.outline(add_rect, LINE);
                d.text_centered(
                    add_rect.0 + add_rect.2 * 0.5,
                    add_rect.1 + 20.0,
                    "SELECT A BAND TO CONFIGURE DYNAMICS",
                    11.5,
                    MUTED,
                );
            }

            // Hover cursor on EQ 1
            if !is_eq2 && (next_p - target_p).abs() < 0.05 {
                if let Some((x, y)) = self.hover {
                    if inside(x, y, (gx, GY, gw, GH))
                        && self.drag.is_none()
                        && self.menu.is_none()
                        && !self.scale_menu
                        && self.processing_menu.is_none()
                        && self.edit.is_none()
                        && !bands.iter().any(|b| {
                            Some(b.id) == self.selected
                                && inside(x, y, hud_rect_for_at(b, self.graph_db, gx, gw))
                        })
                        && !lift_bands.iter().any(|b| {
                            Some(b.id) == self.selected
                                && inside(x, y, hud_rect_for_lift_at(b, self.graph_db, gx, gw))
                        })
                        && !eq1_bypassed
                    {
                        let near = bands.iter().any(|b| {
                            (x - freq_x_at(b.freq, gx, gw)).abs() < 15.0
                                && (y - db_y(
                                    if b.shape.has_gain() { b.gain } else { 0.0 },
                                    self.graph_db,
                                ))
                                .abs()
                                    < 15.0
                        }) || lift_bands.iter().any(|b| {
                            (x - freq_x_at(b.freq, gx, gw)).abs() < 15.0
                                && (y - lift_gain_y(b.gain)).abs() < 15.0
                        });
                        if !near {
                            let db = bands
                                .iter()
                                .filter(|b| b.enabled)
                                .map(|b| {
                                    BandCoeffs::make(b, eq_sr).response(x_freq_at(x, gx, gw), eq_sr)
                                })
                                .sum::<f64>();
                            let curve = (y - db_y(db, self.graph_db)).abs() < 12.0;
                            let shape = infer_shape((x - gx) / gw, (y - GY) / GH, curve);
                            d.circle(x, y, 5.0, MUTED, false);
                            d.text(
                                (x + 12.0).min(gx + gw - 20.0),
                                (y - 12.0).max(GY + 18.0),
                                &format!("+ {}", shape.name()),
                                12.0,
                                MUTED,
                            );
                        }
                    }
                }
            }
        }

        // --- DRAW EQ 2 (if visible) ---
        if offset_2 > -GRAPH_CLIP_W && offset_2 < GRAPH_CLIP_W {
            d.offset_x = offset_2;
            let eq2_bypassed = !eq2_on;
            draw_grid_and_spectrum(&mut d, eq2_bypassed, 1);

            let mut sum = vec![0.0; 420];
            for b in &eq2_bands {
                if !b.enabled {
                    continue;
                }
                let coeff = BandCoeffs::make(b, eq_sr);
                let num = if b.id >= EQ2_ID_BASE {
                    b.id - EQ2_ID_BASE
                } else {
                    b.id
                };
                let color = if eq2_bypassed {
                    MUTED
                } else {
                    COLORS[(num as usize - 1) % COLORS.len()]
                };
                let points: Vec<_> = (0..420)
                    .map(|i| {
                        let x = gx + gw * i as f32 / 419.0;
                        let db = coeff.response(x_freq_at(x, gx, gw).min(sr * 0.49), eq_sr);
                        sum[i] += db;
                        (x, db_y(db, self.graph_db))
                    })
                    .collect();
                if Some(b.id) == self.selected {
                    let mut fill = color;
                    fill.a = if eq2_bypassed { 0.02 } else { 0.07 };
                    d.area(&points, db_y(0.0, self.graph_db), fill);
                    d.poly(&points, color, 1.2);
                }
            }
            let points: Vec<_> = sum
                .iter()
                .enumerate()
                .map(|(i, db)| (gx + gw * i as f32 / 419.0, db_y(*db, self.graph_db)))
                .collect();
            d.poly(&points, if eq2_bypassed { MUTED } else { GOLD }, 2.2);

            for b in &eq2_bands {
                let num = if b.id >= EQ2_ID_BASE {
                    b.id - EQ2_ID_BASE
                } else {
                    b.id
                };
                let color = if !b.enabled || eq2_bypassed {
                    MUTED
                } else {
                    COLORS[(num as usize - 1) % COLORS.len()]
                };
                let x = freq_x_at(b.freq, gx, gw);
                let y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db);
                if Some(b.id) == self.selected {
                    d.circle(x, y, 12.0, color, false);
                }
                if b.dynamic && b.shape.has_gain() {
                    d.line(x, y, x, db_y(b.gain - b.range, self.graph_db), color, 3.0);
                    d.circle(x, db_y(b.gain - b.range, self.graph_db), 3.0, color, true);
                }
                d.circle(x, y, 7.0, color, true);
                d.text(x - 4.0, y - 17.0, &num.to_string(), 11.0, color);
            }

            if let Some(b) = eq2_bands.iter().find(|b| Some(b.id) == self.selected) {
                let num = if b.id >= EQ2_ID_BASE {
                    b.id - EQ2_ID_BASE
                } else {
                    b.id
                };
                let color = if eq2_bypassed {
                    MUTED
                } else {
                    COLORS[(num as usize - 1) % COLORS.len()]
                };
                let is_solo = self.shared.solo_id.load(Ordering::Relaxed) == b.id;
                d.band_hud(
                    b,
                    is_solo,
                    color,
                    self.graph_db,
                    config.mode == ProcessingMode::LinearPhase,
                    gx,
                    gw,
                );
            }

            // EQ 2 Bottom dynamic area
            if let Some(b) = self
                .selected
                .and_then(|id| self.find_band(id))
                .filter(|b| b.id >= EQ2_ID_BASE && b.shape.has_gain())
            {
                let num = if b.id >= EQ2_ID_BASE {
                    b.id - EQ2_ID_BASE
                } else {
                    b.id
                };
                let color = COLORS[(num as usize - 1) % COLORS.len()];
                let power_rect = band_dyn_power_rect_at(b.dynamic, gx);
                let hovered = self
                    .hover
                    .is_some_and(|(hx, hy)| inside(hx, hy, power_rect));
                d.rect(
                    power_rect.0,
                    power_rect.1,
                    power_rect.2,
                    power_rect.3,
                    PANEL,
                );
                if b.dynamic {
                    let c = if eq2_on { color } else { MUTED };
                    d.outline(
                        power_rect,
                        if hovered {
                            TEXT
                        } else if eq2_on {
                            c
                        } else {
                            LINE
                        },
                    );
                    d.power_icon(
                        power_rect.0 + power_rect.2 * 0.5,
                        power_rect.1 + power_rect.3 * 0.5,
                        c,
                    );
                    for (i, label, val, n) in [
                        (
                            0,
                            "THRESHOLD",
                            format!("{:.1} dB", b.threshold),
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
                        (4, "RANGE", format!("{:.1} dB", b.range), b.range / 24.0),
                    ] {
                        d.control(band_rect_at(i, gx, gw), label, &val, n as f32, c);
                    }
                } else {
                    d.outline(power_rect, if hovered { TEXT } else { LINE });
                    d.power_icon(
                        power_rect.0 + 14.0,
                        power_rect.1 + power_rect.3 * 0.5,
                        if hovered { TEXT } else { MUTED },
                    );
                    d.text(
                        power_rect.0 + 28.0,
                        power_rect.1 + power_rect.3 * 0.5 + 4.0,
                        "BAND DYNAMICS",
                        11.0,
                        if hovered { TEXT } else { MUTED },
                    );
                }
            } else if self.selected.is_none() {
                let add_rect = (gx + 8.0, GRAPH_BOTTOM - 50.0, gw - 16.0, 32.0);
                d.rect(add_rect.0, add_rect.1, add_rect.2, add_rect.3, PANEL);
                d.outline(add_rect, LINE);
                d.text_centered(
                    add_rect.0 + add_rect.2 * 0.5,
                    add_rect.1 + 20.0,
                    "SELECT A BAND TO CONFIGURE DYNAMICS",
                    11.5,
                    MUTED,
                );
            }

            // Hover cursor on EQ 2
            if is_eq2 && (next_p - target_p).abs() < 0.05 {
                if let Some((x, y)) = self.hover {
                    if inside(x, y, (gx, GY, gw, GH))
                        && self.drag.is_none()
                        && self.menu.is_none()
                        && !self.scale_menu
                        && self.processing_menu.is_none()
                        && self.edit.is_none()
                        && !eq2_bands.iter().any(|b| {
                            Some(b.id) == self.selected
                                && inside(x, y, hud_rect_for_at(b, self.graph_db, gx, gw))
                        })
                        && !eq2_bypassed
                    {
                        let near = eq2_bands.iter().any(|b| {
                            (x - freq_x_at(b.freq, gx, gw)).abs() < 15.0
                                && (y - db_y(
                                    if b.shape.has_gain() { b.gain } else { 0.0 },
                                    self.graph_db,
                                ))
                                .abs()
                                    < 15.0
                        });
                        if !near {
                            let db = eq2_bands
                                .iter()
                                .filter(|b| b.enabled)
                                .map(|b| {
                                    BandCoeffs::make(b, eq_sr).response(x_freq_at(x, gx, gw), eq_sr)
                                })
                                .sum::<f64>();
                            let curve = (y - db_y(db, self.graph_db)).abs() < 12.0;
                            let shape = infer_shape((x - gx) / gw, (y - GY) / GH, curve);
                            d.circle(x, y, 5.0, MUTED, false);
                            d.text(
                                (x + 12.0).min(gx + gw - 20.0),
                                (y - 12.0).max(GY + 18.0),
                                &format!("+ {}", shape.name()),
                                12.0,
                                MUTED,
                            );
                        }
                    }
                }
            }
        }

        // Clean seam divider during horizontal slide
        if ease > 0.005 && ease < 0.995 {
            d.offset_x = 0.0;
            d.line(
                eq_b.0 + offset_2,
                eq_b.1,
                eq_b.0 + offset_2,
                eq_b.1 + eq_b.3,
                LINE,
                1.0,
            );
        }

        d.reset_scissor();
        d.offset_x = 0.0;
        d.alpha_mul = 1.0;
        let comp_bypassed = !self.params.comp_on.value();
        let (dx, dy, dw, dh) = self.dyn_bounds();
        let is_pre = self.is_pre();
        let is_animating = (cur_p - target_p).abs() > 0.005;

        // Dynamics module chassis (drawn over sliding EQ, faded during animation)
        if is_animating {
            d.rect(dx, dy, dw, dh, C::rgba(21, 24, 29, 215));
        } else {
            d.rect(dx, dy, dw, dh, PANEL);
        }
        d.outline((dx, dy, dw, dh), LINE);

        let routing_btn = self.dyn_routing_button_rect();
        let power_btn = self.dyn_power_button_rect();

        match self.dyn_page {
            DynPage::Main => {
                // Header
                let num_label = if is_pre { "01" } else { "02" };
                d.text(
                    dx + 12.0,
                    123.0,
                    num_label,
                    13.0,
                    if comp_bypassed { MUTED } else { GOLD },
                );
                d.text(
                    dx + 38.0,
                    123.0,
                    "DYNAMICS",
                    15.0,
                    if comp_bypassed { MUTED } else { TEXT },
                );
                d.button(
                    routing_btn,
                    if is_pre { "PRE" } else { "POST" },
                    true,
                    if is_pre { TEAL } else { GOLD },
                );
                let cog_r = self.dyn_cog_button_rect();
                let cog_hover =
                    !comp_bypassed && self.hover.is_some_and(|(hx, hy)| inside(hx, hy, cog_r));
                let cog_color = if comp_bypassed {
                    MUTED
                } else if cog_hover {
                    GOLD
                } else {
                    TEXT
                };
                d.rect(cog_r.0, cog_r.1, cog_r.2, cog_r.3, PANEL);
                d.outline(cog_r, if cog_hover { GOLD } else { LINE });
                d.cog_icon(cog_r.0 + cog_r.2 * 0.5, cog_r.1 + cog_r.3 * 0.5, cog_color);
                d.bypass_button(power_btn, comp_bypassed, GOLD);

                d.line(dx + 10.0, 138.0, dx + dw - 10.0, 138.0, LINE, 1.0);

                let (sx, sy, sw, sh) = self.dyn_main_thresh_slider_rect();
                let (gx_m, gy_m, gw_m, gh_m) = self.dyn_main_gr_meter_rect();

                d.text_centered(dx + 52.0, 150.0, "THRESHOLD", 9.5, MUTED);
                let p0 = self.param(0);
                let thresh_val_str =
                    p0.normalized_value_to_string(p0.unmodulated_normalized_value(), true);
                d.text_centered(
                    dx + 52.0,
                    166.0,
                    &thresh_val_str,
                    12.0,
                    if comp_bypassed { MUTED } else { GOLD },
                );

                d.text_centered(dx + dw - 52.0, 150.0, "GR", 9.5, MUTED);
                let gr = if comp_bypassed {
                    0.0
                } else {
                    self.shared.gr.load(Ordering::Relaxed)
                };
                d.text_centered(
                    dx + dw - 52.0,
                    166.0,
                    &format!("{:.1}", gr),
                    12.0,
                    if comp_bypassed { MUTED } else { GOLD },
                );

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
                        let over_h = thresh_y - sig_top_y;
                        d.rect(sx, sig_top_y, sw, over_h, GOLD);
                    }
                }

                let gr_frac = (gr / 30.0).clamp(0.0, 1.0);
                let gr_bar_h = gh_m * gr_frac;
                if !comp_bypassed && gr_bar_h > 0.5 {
                    d.rect(gx_m, gy_m, gw_m, gr_bar_h, GOLD);
                }

                let handle = self.dyn_main_thresh_handle_rect(thresh_y);
                let handle_hover = self.hover.is_some_and(|(hx, hy)| {
                    inside(
                        hx,
                        hy,
                        (
                            handle.0 - 4.0,
                            handle.1 - 6.0,
                            handle.2 + 8.0,
                            handle.3 + 12.0,
                        ),
                    )
                }) || self.drag == Some(Target::Global(0));
                let handle_color = if comp_bypassed {
                    MUTED
                } else if handle_hover {
                    TEXT
                } else {
                    GOLD
                };
                d.grab_bar(handle, handle_color);
            }
            DynPage::Controls => {
                d.button(self.dyn_back_button_rect(), "< BACK", false, TEXT);
                d.button(
                    routing_btn,
                    if is_pre { "PRE" } else { "POST" },
                    true,
                    if is_pre { TEAL } else { GOLD },
                );
                d.bypass_button(power_btn, comp_bypassed, GOLD);

                for (i, label, active) in [
                    (0, "SOFT KNEE", self.params.soft_knee.value()),
                    (1, "AUTO MAKEUP", self.params.auto_makeup.value()),
                    (2, "LINKED STEREO", self.params.stereo_link.value()),
                ] {
                    d.button(
                        self.dyn_mode_rect(i),
                        label,
                        active,
                        if comp_bypassed { MUTED } else { GOLD },
                    );
                }

                for (i, label, color) in [
                    (0, "THRESHOLD", GOLD),
                    (6, "RATIO", GOLD),
                    (5, "ATTACK", GOLD),
                    (11, "RELEASE", GOLD),
                    (2, "SC HPF", TEAL),
                    (3, "DRY", TEAL),
                    (4, "WET", GOLD),
                ] {
                    let p = self.param(i);
                    let value =
                        p.normalized_value_to_string(p.unmodulated_normalized_value(), true);
                    let norm = if i == 0 {
                        1.0 - p.unmodulated_normalized_value()
                    } else {
                        p.unmodulated_normalized_value()
                    };
                    d.knob(
                        self.global_rect(i),
                        label,
                        &value,
                        norm,
                        color,
                        comp_bypassed,
                    );
                }
            }
        }

        let (px, py, pw, ph) = self.pse_bounds();
        if is_animating {
            d.rect(px, py, pw, ph, C::rgba(21, 24, 29, 215));
        } else {
            d.rect(px, py, pw, ph, PANEL);
        }
        d.outline((px, py, pw, ph), LINE);
        d.text(
            px + 12.0,
            123.0,
            "03",
            13.0,
            if comp_bypassed { MUTED } else { GOLD },
        );
        d.text(
            px + 38.0,
            123.0,
            "PSE",
            15.0,
            if comp_bypassed { MUTED } else { TEXT },
        );
        d.line(px + 10.0, 138.0, px + pw - 10.0, 138.0, LINE, 1.0);
        d.button(
            self.pse_mode_rect(0),
            if self.params.pse_peak.value() {
                "DETECT: PEAK"
            } else {
                "DETECT: RMS"
            },
            true,
            if comp_bypassed { MUTED } else { GOLD },
        );
        d.button(
            self.pse_mode_rect(1),
            "LISTEN SC",
            self.params.pse_listen.value(),
            if comp_bypassed { MUTED } else { TEAL },
        );
        for (i, label) in [
            (7, "DEPTH"),
            (8, "HYSTERESIS"),
            (1, "THRESHOLD"),
            (2, "SC HPF"),
            (9, "KNEE"),
            (10, "TIME"),
        ] {
            let p = self.param(i);
            let value = if i == 1 && p.value() <= -79.9 {
                "OFF".to_owned()
            } else if i == 10 {
                format_pse_time(p.value() as f64, self.params.pse_peak.value())
            } else {
                p.normalized_value_to_string(p.unmodulated_normalized_value(), true)
            };
            d.knob(
                self.pse_knob_rect(i),
                label,
                &value,
                p.unmodulated_normalized_value(),
                if i == 2 { TEAL } else { GOLD },
                comp_bypassed,
            );
        }

        d.line(32.0, FOOTER_LINE_Y, UI_W - 32.0, FOOTER_LINE_Y, LINE, 1.0);
        d.text(
            32.0,
            FOOTER_BTN_Y + 20.0,
            &format!("{:.1} kHz", sr / 1000.0),
            11.0,
            MUTED,
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
        let status_x = if config.mode == ProcessingMode::LinearPhase {
            470.0
        } else {
            358.0
        };
        d.text(status_x, FOOTER_BTN_Y + 20.0, &status, 10.0, MUTED);
        d.button(
            THEME_BUTTON,
            if d.light { "DARK MODE" } else { "LIGHT MODE" },
            false,
            TEXT,
        );
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
                            .hover
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
                if selected || self.hover.is_some_and(|(x, y)| inside(x, y, row)) {
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
            if let Some((target, r)) = self.hover.and_then(|(x, y)| self.value_at(x, y)) {
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
fn hz(f: f64) -> String {
    if f >= 1000.0 {
        format!("{:.2} kHz", f / 1000.0)
    } else {
        format!("{:.0} Hz", f)
    }
}
struct Draw<'a> {
    light: bool,
    c: &'a mut Canvas,
    s: f32,
    ox: f32,
    oy: f32,
    font: Option<FontId>,
    offset_x: f32,
    alpha_mul: f32,
}
impl Draw<'_> {
    fn scissor(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.c.scissor(
            self.ox + x * self.s,
            self.oy + y * self.s,
            w * self.s,
            h * self.s,
        );
    }
    fn reset_scissor(&mut self) {
        self.c.reset_scissor();
    }
    fn color(&self, c: C) -> C {
        if !self.light {
            let mut result = c;
            result.a = (result.a * self.alpha_mul).clamp(0.0, 1.0);
            return result;
        }
        let mut result = if c == BG {
            rgb(245, 243, 238)
        } else if c == PANEL {
            rgb(233, 230, 223)
        } else if c == LINE {
            rgb(201, 201, 195)
        } else if c == TEXT {
            rgb(30, 37, 44)
        } else if c == MUTED {
            rgb(95, 105, 114)
        } else if c == GOLD {
            rgb(145, 98, 24)
        } else if c == TEAL {
            rgb(21, 122, 104)
        } else if c.r.max(c.g).max(c.b) < 0.35 {
            C {
                r: 0.90 - c.r * 0.35,
                g: 0.90 - c.g * 0.35,
                b: 0.88 - c.b * 0.35,
                a: c.a,
            }
        } else {
            C {
                r: c.r * 0.64,
                g: c.g * 0.64,
                b: c.b * 0.64,
                a: c.a,
            }
        };
        result.a = (result.a * self.alpha_mul).clamp(0.0, 1.0);
        result
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: C) {
        let mut p = Path::new();
        p.rounded_rect(
            self.ox + (x + self.offset_x) * self.s,
            self.oy + y * self.s,
            w * self.s,
            h * self.s,
            5.0_f32.min(w * 0.5).min(h * 0.5) * self.s,
        );
        self.c.fill_path(&p, &Paint::color(self.color(color)));
    }
    fn line(&mut self, x: f32, y: f32, ex: f32, ey: f32, color: C, width: f32) {
        self.poly(&[(x, y), (ex, ey)], color, width);
    }
    fn poly(&mut self, points: &[(f32, f32)], color: C, width: f32) {
        let mut p = Path::new();
        for (i, (x, y)) in points.iter().enumerate() {
            if i == 0 {
                p.move_to(self.ox + (x + self.offset_x) * self.s, self.oy + y * self.s);
            } else {
                p.line_to(self.ox + (x + self.offset_x) * self.s, self.oy + y * self.s);
            }
        }
        let mut paint = Paint::color(self.color(color));
        paint.set_line_width(width * self.s);
        self.c.stroke_path(&p, &paint);
    }
    fn area(&mut self, points: &[(f32, f32)], bottom: f32, color: C) {
        if points.is_empty() {
            return;
        }
        let mut p = Path::new();
        p.move_to(
            self.ox + (points[0].0 + self.offset_x) * self.s,
            self.oy + bottom * self.s,
        );
        for (x, y) in points {
            p.line_to(self.ox + (x + self.offset_x) * self.s, self.oy + y * self.s);
        }
        p.line_to(
            self.ox + (points.last().unwrap().0 + self.offset_x) * self.s,
            self.oy + bottom * self.s,
        );
        p.close();
        self.c.fill_path(&p, &Paint::color(self.color(color)));
    }
    fn poly_gradient_span(
        &mut self,
        points: &[(f32, f32)],
        sx: f32,
        ex: f32,
        c1: C,
        c2: C,
        width: f32,
    ) {
        if points.is_empty() {
            return;
        }
        let mut p = Path::new();
        p.move_to(
            self.ox + (points[0].0 + self.offset_x) * self.s,
            self.oy + points[0].1 * self.s,
        );
        for (x, y) in &points[1..] {
            p.line_to(self.ox + (x + self.offset_x) * self.s, self.oy + y * self.s);
        }
        let mut paint = Paint::linear_gradient(
            self.ox + (sx + self.offset_x) * self.s,
            self.oy,
            self.ox + (ex + self.offset_x) * self.s,
            self.oy,
            self.color(c1),
            self.color(c2),
        );
        paint.set_line_width(width * self.s);
        self.c.stroke_path(&p, &paint);
    }
    fn area_gradient_span(
        &mut self,
        points: &[(f32, f32)],
        bottom: f32,
        sx: f32,
        ex: f32,
        c1: C,
        c2: C,
    ) {
        if points.is_empty() {
            return;
        }
        let mut p = Path::new();
        p.move_to(
            self.ox + (points[0].0 + self.offset_x) * self.s,
            self.oy + bottom * self.s,
        );
        for (x, y) in points {
            p.line_to(self.ox + (x + self.offset_x) * self.s, self.oy + y * self.s);
        }
        p.line_to(
            self.ox + (points.last().unwrap().0 + self.offset_x) * self.s,
            self.oy + bottom * self.s,
        );
        p.close();
        let paint = Paint::linear_gradient(
            self.ox + (sx + self.offset_x) * self.s,
            self.oy,
            self.ox + (ex + self.offset_x) * self.s,
            self.oy,
            self.color(c1),
            self.color(c2),
        );
        self.c.fill_path(&p, &paint);
    }
    fn circle(&mut self, x: f32, y: f32, r: f32, color: C, fill: bool) {
        let mut p = Path::new();
        p.circle(
            self.ox + (x + self.offset_x) * self.s,
            self.oy + y * self.s,
            r * self.s,
        );
        let mut paint = Paint::color(self.color(color));
        paint.set_line_width(self.s);
        if fill {
            self.c.fill_path(&p, &paint);
        } else {
            self.c.stroke_path(&p, &paint);
        }
    }
    fn text(&mut self, x: f32, y: f32, text: &str, size: f32, color: C) {
        let mut p = Paint::color(self.color(color));
        if let Some(font) = self.font {
            p.set_font(&[font]);
        }
        p.set_font_size(size * self.s);
        let _ = self.c.fill_text(
            self.ox + (x + self.offset_x) * self.s,
            self.oy + y * self.s,
            text,
            &p,
        );
    }
    fn button(&mut self, r: (f32, f32, f32, f32), label: &str, on: bool, color: C) {
        self.rect(r.0, r.1, r.2, r.3, PANEL);
        if on {
            self.rect(r.0, r.1 + r.3 - 2.0, r.2, 2.0, color);
        }
        self.text(
            r.0 + 10.0,
            r.1 + r.3 / 2.0 + 4.0,
            label,
            11.0,
            if on { color } else { MUTED },
        );
    }
    fn tab_button(&mut self, r: (f32, f32, f32, f32), label: &str, on: bool, color: C) {
        self.rect(r.0, r.1, r.2, r.3, PANEL);
        if on {
            self.rect(r.0, r.1 + r.3 - 2.0, r.2, 2.0, color);
        }
        self.text_centered(
            r.0 + r.2 * 0.5,
            r.1 + r.3 / 2.0 + 4.0,
            label,
            11.0,
            if on { color } else { MUTED },
        );
    }
    fn outline(&mut self, r: (f32, f32, f32, f32), color: C) {
        let mut path = Path::new();
        path.rounded_rect(
            self.ox + (r.0 + self.offset_x) * self.s,
            self.oy + r.1 * self.s,
            r.2 * self.s,
            r.3 * self.s,
            5.0 * self.s,
        );
        let mut paint = Paint::color(self.color(color));
        paint.set_line_width(self.s);
        self.c.stroke_path(&path, &paint);
    }
    fn grab_bar(&mut self, r: (f32, f32, f32, f32), color: C) {
        self.rect(r.0, r.1, r.2, r.3, color);
        let grip = C {
            r: color.r * 0.35,
            g: color.g * 0.35,
            b: color.b * 0.35,
            a: 1.0,
        };
        let cx = r.0 + r.2 * 0.5;
        let cy = r.1 + r.3 * 0.5;
        let line_w = (r.2 * 0.38).min(22.0);
        self.line(
            cx - line_w * 0.5,
            cy - 2.2,
            cx + line_w * 0.5,
            cy - 2.2,
            grip,
            1.4,
        );
        self.line(
            cx - line_w * 0.5,
            cy + 2.2,
            cx + line_w * 0.5,
            cy + 2.2,
            grip,
            1.4,
        );
    }
    fn bypass_button(&mut self, r: (f32, f32, f32, f32), bypassed: bool, color: C) {
        self.rect(r.0, r.1, r.2, r.3, PANEL);
        self.outline(r, if bypassed { LINE } else { color });
        self.power_icon(
            r.0 + r.2 * 0.5,
            r.1 + r.3 * 0.5,
            if bypassed { MUTED } else { color },
        );
    }
    fn text_centered(&mut self, cx: f32, y: f32, text: &str, size: f32, color: C) {
        let mut p = Paint::color(self.color(color));
        if let Some(font) = self.font {
            p.set_font(&[font]);
        }
        p.set_font_size(size * self.s);
        let width = if let Ok(m) = self.c.measure_text(0.0, 0.0, text, &p) {
            m.width() / self.s
        } else {
            text.len() as f32 * size * 0.55
        };
        let _ = self.c.fill_text(
            self.ox + (cx + self.offset_x - width * 0.5) * self.s,
            self.oy + y * self.s,
            text,
            &p,
        );
    }
    fn knob(
        &mut self,
        r: (f32, f32, f32, f32),
        label: &str,
        value: &str,
        n: f32,
        color: C,
        bypassed: bool,
    ) {
        let cx = r.0 + r.2 * 0.5;
        let cy = r.1 + 45.0;
        let rad = 17.0;

        self.text_centered(
            cx,
            r.1 + 22.0,
            label,
            9.2,
            if bypassed { MUTED } else { TEXT },
        );

        let start_angle = 135.0_f32.to_radians();
        let total_sweep = 270.0_f32.to_radians();
        let cur_angle = start_angle + total_sweep * n.clamp(0.0, 1.0);

        let mut bg_pts = Vec::with_capacity(25);
        for i in 0..=24 {
            let a = start_angle + total_sweep * (i as f32 / 24.0);
            bg_pts.push((cx + rad * a.cos(), cy + rad * a.sin()));
        }
        self.poly(&bg_pts, LINE, 2.5);

        if !bypassed && n > 0.005 {
            let steps = ((24.0 * n).ceil() as usize).max(2);
            let mut val_pts = Vec::with_capacity(steps + 1);
            for i in 0..=steps {
                let a = start_angle + (cur_angle - start_angle) * (i as f32 / steps as f32);
                val_pts.push((cx + rad * a.cos(), cy + rad * a.sin()));
            }
            self.poly(&val_pts, color, 2.5);
        }

        self.circle(cx, cy, 13.0, rgb(28, 33, 40), true);
        self.circle(
            cx,
            cy,
            13.0,
            if bypassed {
                LINE
            } else {
                C {
                    r: (rgb(48, 56, 68).r + color.r * 0.25).min(1.0),
                    g: (rgb(48, 56, 68).g + color.g * 0.25).min(1.0),
                    b: (rgb(48, 56, 68).b + color.b * 0.25).min(1.0),
                    a: 1.0,
                }
            },
            false,
        );

        let ind_color = if bypassed { MUTED } else { color };
        let nx = cx + 11.5 * cur_angle.cos();
        let ny = cy + 11.5 * cur_angle.sin();
        let n_inner_x = cx + 5.0 * cur_angle.cos();
        let n_inner_y = cy + 5.0 * cur_angle.sin();
        self.line(n_inner_x, n_inner_y, nx, ny, ind_color, 2.0);

        self.text_centered(
            cx,
            r.1 + 75.0,
            value,
            11.0,
            if bypassed { MUTED } else { color },
        );
    }
    fn control(&mut self, r: (f32, f32, f32, f32), label: &str, value: &str, n: f32, color: C) {
        self.rect(r.0, r.1, r.2, r.3, C::rgba(27, 31, 37, 225));
        self.outline(
            r,
            C {
                r: (LINE.r + color.r * 0.3).min(1.0),
                g: (LINE.g + color.g * 0.3).min(1.0),
                b: (LINE.b + color.b * 0.3).min(1.0),
                a: 1.0,
            },
        );
        self.text(r.0 + 12.0, r.1 + 18.0, label, 9.5, MUTED);
        let mut paint = Paint::color(self.color(TEXT));
        if let Some(font) = self.font {
            paint.set_font(&[font]);
        }
        paint.set_font_size(11.0 * self.s);
        let width = self
            .c
            .measure_text(0.0, 0.0, value, &paint)
            .map(|m| m.width() / self.s)
            .unwrap_or(70.0);
        self.text(r.0 + r.2 - 12.0 - width, r.1 + 18.0, value, 11.0, TEXT);
        let y = r.1 + 28.0;
        self.rect(r.0 + 12.0, y, r.2 - 24.0, 16.0, LINE);
        self.rect(r.0 + 12.0, y, (r.2 - 24.0) * n.clamp(0.0, 1.0), 16.0, color);
    }
    fn cog_icon(&mut self, cx: f32, cy: f32, color: C) {
        let mut p = Path::new();
        let r_outer = 6.0 * self.s;
        let r_inner = 3.8 * self.s;
        let ox = self.ox + cx * self.s;
        let oy = self.oy + cy * self.s;
        for i in 0..6 {
            let mid = (i as f32 * 60.0).to_radians();
            let a1 = mid - 18.0_f32.to_radians();
            let a2 = mid - 8.0_f32.to_radians();
            let a3 = mid + 8.0_f32.to_radians();
            let a4 = mid + 18.0_f32.to_radians();
            if i == 0 {
                p.move_to(ox + r_inner * a1.cos(), oy + r_inner * a1.sin());
            } else {
                p.line_to(ox + r_inner * a1.cos(), oy + r_inner * a1.sin());
            }
            p.line_to(ox + r_outer * a2.cos(), oy + r_outer * a2.sin());
            p.line_to(ox + r_outer * a3.cos(), oy + r_outer * a3.sin());
            p.line_to(ox + r_inner * a4.cos(), oy + r_inner * a4.sin());
        }
        p.close();
        self.c.fill_path(&p, &Paint::color(self.color(color)));
        self.circle(cx, cy, 1.8, PANEL, true);
    }
    fn headphones(&mut self, cx: f32, cy: f32, color: C) {
        let mut points = Vec::new();
        for i in 0..=12 {
            let angle = std::f32::consts::PI + std::f32::consts::PI * i as f32 / 12.0;
            points.push((cx + 6.0 * angle.cos(), cy + 1.0 + 6.0 * angle.sin()));
        }
        self.poly(&points, color, 1.4);
        self.rect(cx - 7.5, cy, 3.0, 6.0, color);
        self.rect(cx + 4.5, cy, 3.0, 6.0, color);
    }
    fn power_icon(&mut self, cx: f32, cy: f32, color: C) {
        let mut points = Vec::new();
        for i in 0..=16 {
            let angle = -std::f32::consts::FRAC_PI_2
                + 0.8
                + (std::f32::consts::TAU - 1.6) * i as f32 / 16.0;
            points.push((cx + 5.5 * angle.cos(), cy + 5.5 * angle.sin()));
        }
        self.poly(&points, color, 1.4);
        self.line(cx, cy - 6.5, cx, cy - 1.0, color, 1.4);
    }
    fn close_icon(&mut self, cx: f32, cy: f32, color: C) {
        self.line(cx - 4.0, cy - 4.0, cx + 4.0, cy + 4.0, color, 1.4);
        self.line(cx + 4.0, cy - 4.0, cx - 4.0, cy + 4.0, color, 1.4);
    }
    fn band_hud(
        &mut self,
        b: &Band,
        is_solo: bool,
        color: C,
        range: f64,
        _linear: bool,
        gx: f32,
        gw: f32,
    ) {
        let (bx, by, bw, _) = hud_rect_for_at(b, range, gx, gw);
        // The card has no backdrop or pointer: the EQ stays visible around its controls.
        self.line(bx + 10.0, by, bx + bw - 10.0, by, color, 1.5);
        self.power_icon(bx + 19.0, by + 19.0, if b.enabled { color } else { MUTED });
        self.rect(bx + 40.0, by + 6.0, 148.0, 26.0, C::rgba(35, 40, 48, 150));
        self.text(
            bx + 50.0,
            by + 23.0,
            &format!("{} ▾", b.shape.name()),
            11.0,
            TEXT,
        );
        self.rect(
            bx + 222.0,
            by + 6.0,
            28.0,
            26.0,
            if is_solo {
                C::rgba(110, 85, 30, 180)
            } else {
                C::rgba(35, 40, 48, 150)
            },
        );
        self.headphones(bx + 236.0, by + 18.0, if is_solo { GOLD } else { MUTED });
        self.close_icon(bx + bw - 19.0, by + 19.0, MUTED);

        for (i, label, value, active) in [
            (0, "FREQ", hz(b.freq), true),
            (
                1,
                "GAIN",
                if b.shape.has_gain() {
                    format!("{:+.2} dB", b.gain)
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
            self.text(r.0 + 4.0, by + 44.0, label, 8.5, MUTED);
            self.text(
                r.0 + 4.0,
                by + 65.0,
                &value,
                12.0,
                if active { color } else { MUTED },
            );
        }
        if b.shape.is_cut() {
            self.rect(bx + 10.0, by + 78.0, 108.0, 22.0, C::rgba(35, 40, 48, 150));
            self.text(
                bx + 18.0,
                by + 93.0,
                &format!("{} dB/oct ▾", b.order * 6),
                9.0,
                TEXT,
            );
        }
    }
    fn band_hud_lift(
        &mut self,
        b: &LiftBand,
        is_solo: bool,
        color: C,
        range: f64,
        gx: f32,
        gw: f32,
    ) {
        let (bx, by, bw, _) = hud_rect_for_lift_at(b, range, gx, gw);
        self.line(bx + 10.0, by, bx + bw - 10.0, by, color, 1.5);
        self.power_icon(bx + 19.0, by + 19.0, if b.enabled { color } else { MUTED });
        self.rect(bx + 40.0, by + 6.0, 148.0, 26.0, C::rgba(35, 40, 48, 150));
        self.text(
            bx + 50.0,
            by + 23.0,
            &format!("{} ▾", b.shape.name()),
            11.0,
            TEXT,
        );
        self.rect(
            bx + 222.0,
            by + 6.0,
            28.0,
            26.0,
            if is_solo {
                C::rgba(110, 85, 30, 180)
            } else {
                C::rgba(35, 40, 48, 150)
            },
        );
        self.headphones(bx + 236.0, by + 18.0, if is_solo { GOLD } else { MUTED });
        self.close_icon(bx + bw - 19.0, by + 19.0, MUTED);

        for (i, label, value, active) in [
            (0, "FREQ", hz(b.freq), true),
            (
                1,
                "GAIN",
                if b.gain <= -99.5 {
                    "-inf dB".into()
                } else {
                    format!("{:.1} dB", b.gain)
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
            self.text(r.0 + 4.0, by + 44.0, label, 8.5, MUTED);
            self.text(
                r.0 + 4.0,
                by + 65.0,
                &value,
                12.0,
                if active { color } else { MUTED },
            );
        }
        if b.shape.is_cut() {
            self.rect(bx + 10.0, by + 78.0, 108.0, 22.0, C::rgba(35, 40, 48, 150));
            self.text(
                bx + 18.0,
                by + 93.0,
                &format!("{} dB/oct ▾", b.order * 6),
                9.0,
                TEXT,
            );
        }
    }
    fn signature(&mut self, font: Option<FontId>) {
        let shift = UI_W - 1120.0;
        let mut paint = Paint::linear_gradient(
            self.ox + (800.0 + shift) * self.s,
            self.oy + 13.0 * self.s,
            self.ox + (810.0 + shift) * self.s,
            self.oy + 63.0 * self.s,
            self.color(C::rgb(255, 230, 163)),
            self.color(C::rgb(176, 119, 33)),
        );
        if let Some(f) = font {
            paint.set_font(&[f]);
        }
        paint.set_font_size(44.0 * self.s);
        let _ = self.c.fill_text(
            self.ox + (794.0 + shift) * self.s,
            self.oy + 51.0 * self.s,
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
                underline.move_to(self.ox + x * self.s, self.oy + y * self.s);
            } else {
                underline.line_to(self.ox + x * self.s, self.oy + y * self.s);
            }
        }
        let gold = self.color(GOLD);
        let mut paint = Paint::linear_gradient(
            self.ox + (821.0 + shift) * self.s,
            self.oy,
            self.ox + (1073.0 + shift) * self.s,
            self.oy,
            C { a: 0.0, ..gold },
            gold,
        );
        paint.set_line_width(self.s);
        self.c.stroke_path(&underline, &paint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            ("80 Hz", ValueTarget::Global(2), 80.0),
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
                    for i in 0..3 {
                        let value = hud_value_rect(&b, range, i);
                        assert!(inside(value.0, value.1, r));
                        assert!(inside(value.0 + value.2, value.1 + value.3, r));
                    }
                }
            }
        }
    }

    #[test]
    fn editing_replaces_selection_and_deletes_without_touching_the_band() {
        let mut edit = ValueEdit {
            target: ValueTarget::Band(0),
            rect: (0.0, 0.0, 100.0, 24.0),
            text: "1000".into(),
            original: "1000".into(),
            cursor: 4,
            anchor: 0,
            invalid: false,
        };
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
    fn dual_eq_animation_math_and_id_ranges() {
        assert_eq!(EQ2_ID_BASE, 10_000);
        assert!(EQ2_ID_BASE < LIFT_ID_BASE);
        assert_eq!(GRAPH_CLIP_W, EQ_W);

        // Quintic smootherstep at 0.0 (start): EQ 1 on screen, EQ 2 off screen right
        let p0 = 0.0_f32;
        let ease0 = p0 * p0 * p0 * (p0 * (p0 * 6.0 - 15.0) + 10.0);
        let offset1_0 = -ease0 * GRAPH_CLIP_W;
        let offset2_0 = (1.0 - ease0) * GRAPH_CLIP_W;
        assert_eq!(offset1_0, 0.0);
        assert_eq!(offset2_0, GRAPH_CLIP_W);

        // Quintic smootherstep at 1.0 (switched): EQ 1 off screen left, EQ 2 on screen
        let p1 = 1.0_f32;
        let ease1 = p1 * p1 * p1 * (p1 * (p1 * 6.0 - 15.0) + 10.0);
        let offset1_1 = -ease1 * GRAPH_CLIP_W;
        let offset2_1 = (1.0 - ease1) * GRAPH_CLIP_W;
        assert_eq!(offset1_1, -GRAPH_CLIP_W);
        assert_eq!(offset2_1, 0.0);

        // Midpoint
        let p_mid = 0.5_f32;
        let ease_mid = p_mid * p_mid * p_mid * (p_mid * (p_mid * 6.0 - 15.0) + 10.0);
        assert!((ease_mid - 0.5).abs() < 1e-6);
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
            params.lift_bands.clone(),
        );
        let mut view = StripView {
            params: params.clone(),
            shared,
            selected: None,
            dyn_page: DynPage::Main,
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
            last_tick: Cell::new(None),
        };

        // Post mode (default): EQ, Dynamics, then PSE as its own module after EQ
        assert!(!view.is_pre());
        assert_eq!(view.eq_bounds(), (MARGIN, MODULE_Y, EQ_W, MODULE_H));
        assert_eq!(
            view.dyn_bounds(),
            (MARGIN + EQ_W + GAP, MODULE_Y, SIDE_W, MODULE_H)
        );
        assert_eq!(
            view.pse_bounds(),
            (UI_W - MARGIN - SIDE_W, MODULE_Y, SIDE_W, MODULE_H)
        );
        assert!(view.eq_bounds().0 + view.eq_bounds().2 <= view.dyn_bounds().0);
        assert!(view.dyn_bounds().0 + view.dyn_bounds().2 <= view.pse_bounds().0);
        assert_eq!(view.gx(), MARGIN + 44.0);
        assert_eq!(view.global_controls(), &[0]);
        assert_eq!(PSE_KNOBS, [7, 8, 1, 2, 9, 10]);

        // Add Lift button is positioned in header row between EQ 2 and bypass
        let tab2 = view.eq_tab_2_rect();
        let add_lift = view.eq_add_lift_rect();
        let bypass = view.eq_power_rect();
        assert!(tab2.0 + tab2.2 < add_lift.0);
        assert!(add_lift.0 + add_lift.2 < bypass.0);
        assert_eq!(add_lift.1, tab2.1);

        // Combined meters sit flush with no scale reticules between them
        let slider = view.dyn_main_thresh_slider_rect();
        let gr_meter = view.dyn_main_gr_meter_rect();
        assert_eq!(gr_meter.0, slider.0 + slider.2);
        assert_eq!(gr_meter.1, slider.1);
        assert_eq!(gr_meter.3, slider.3);
        assert!(gr_meter.2 < slider.2);
        let handle = view.dyn_main_thresh_handle_rect(slider.1 + slider.3 * 0.5);
        assert!(handle.2 > slider.2);
        assert!(handle.3 >= 12.0);

        // Graph and frequency labels stay above the footer
        assert!(GRAPH_BOTTOM + 22.0 <= MODULE_Y + MODULE_H);
        assert!(MODULE_Y + MODULE_H < FOOTER_LINE_Y);
        assert!(FOOTER_LINE_Y < FOOTER_BTN_Y);
        assert!(FOOTER_BTN_Y + 28.0 <= UI_H);
        assert_eq!(HEADER_H, 82.0);

        // Controls keep the original knob size
        view.dyn_page = DynPage::Controls;
        assert_eq!(view.global_controls(), &[0, 6, 5, 11, 2, 3, 4]);
        let knob = view.global_rect(0);
        assert_eq!((knob.2, knob.3), (100.0, 82.0));
        let pse_knob = view.pse_knob_rect(7);
        assert_eq!((pse_knob.2, pse_knob.3), (100.0, 82.0));
        assert!(inside(
            view.pse_bounds().0 + 20.0,
            view.pse_knob_rect(7).1 + 10.0,
            view.pse_bounds()
        ));

        // Pre mode: Dynamics on left, EQ in the middle, PSE stays after EQ on the right
        let params_pre = Arc::new(StripParams {
            comp_pre: BoolParam::new("Dynamics routing", true),
            ..StripParams::default()
        });
        view.params = params_pre;
        assert!(view.is_pre());
        assert_eq!(view.dyn_bounds(), (MARGIN, MODULE_Y, SIDE_W, MODULE_H));
        assert_eq!(
            view.eq_bounds(),
            (MARGIN + SIDE_W + GAP, MODULE_Y, EQ_W, MODULE_H)
        );
        assert_eq!(
            view.pse_bounds(),
            (UI_W - MARGIN - SIDE_W, MODULE_Y, SIDE_W, MODULE_H)
        );
        assert!(view.dyn_bounds().0 + view.dyn_bounds().2 <= view.eq_bounds().0);
        assert!(view.eq_bounds().0 + view.eq_bounds().2 <= view.pse_bounds().0);
        assert_eq!(view.gx(), MARGIN + SIDE_W + GAP + 44.0);
        view.dyn_page = DynPage::Main;
        let pre_slider = view.dyn_main_thresh_slider_rect();
        assert_eq!(pre_slider.0, MARGIN + 92.0);
        assert_eq!(view.dyn_main_gr_meter_rect().0, pre_slider.0 + pre_slider.2);
    }
}
