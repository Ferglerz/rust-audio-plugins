mod preferences;
use crate::{
    band::{infer_shape, Band, Shape},
    dsp::BandCoeffs,
    engine::Shared,
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
const COLORS: [C; 6] = [
    rgb(111, 210, 188),
    rgb(171, 151, 238),
    rgb(236, 177, 107),
    rgb(108, 176, 242),
    rgb(228, 130, 164),
    rgb(193, 214, 118),
];
const GX: f32 = 66.0;
const GY: f32 = 151.0;
const GW: f32 = 1008.0;
// Keep node gestures above the controls while extending the frequency grid beneath them.
const GH: f32 = 338.0;
const GRAPH_BOTTOM: f32 = 552.0;
const THEME_BUTTON: (f32, f32, f32, f32) = (970.0, 750.0, 116.0, 28.0);
const EQ_POWER: (f32, f32, f32, f32) = (1050.0, 103.0, 32.0, 28.0);
const COMP_POWER: (f32, f32, f32, f32) = (1038.0, 592.0, 32.0, 26.0);
const PSE_BACK_BUTTON: (f32, f32, f32, f32) = (68.0, 592.0, 88.0, 26.0);
const PSE_COG_BUTTON: (f32, f32, f32, f32) = (334.0, 646.0, 22.0, 22.0);
const PROCESS_BUTTON: (f32, f32, f32, f32) = (130.0, 750.0, 216.0, 28.0);
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
    (354.0, 750.0, 104.0, 28.0)
}
const SCALE_BUTTON: (f32, f32, f32, f32) = (12.0, GRAPH_BOTTOM + 7.0, 40.0, 24.0);
const SCALES: [f64; 6] = [12.0, 24.0, 36.0, 48.0, 60.0, 72.0];
const SCALE_MENU: (f32, f32, f32, f32) = (14.0, GRAPH_BOTTOM - 144.0, 98.0, 144.0);

#[derive(Clone, Copy, Debug, PartialEq)]
enum ValueTarget {
    Global(usize),
    // Frequency, gain, Q, threshold, ratio, attack, release, range.
    Band(usize),
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
        ValueTarget::Band(0) | ValueTarget::Global(2) => {
            &[("khz", 1000.0), ("hz", 1.0), ("k", 1000.0)]
        }
        ValueTarget::Band(5 | 6) => &[("ms", 1.0), ("s", 1000.0)],
        ValueTarget::Band(4) | ValueTarget::Global(6) => &[(":1", 1.0)],
        ValueTarget::Global(3 | 4) => &[("%", 1.0)],
        ValueTarget::Band(2) => &[],
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
fn hud_value_rect(b: &Band, range: f64, i: usize) -> (f32, f32, f32, f32) {
    let (x, y, _, _) = hud_rect_for(b, range);
    match i {
        0 => (x + 10.0, y + 48.0, 92.0, 24.0),
        1 => (x + 110.0, y + 48.0, 100.0, 24.0),
        _ => (x + 218.0, y + 48.0, 76.0, 24.0),
    }
}
fn global_value_rect(i: usize) -> (f32, f32, f32, f32) {
    let r = global_rect(i);
    (r.0 + 6.0, r.1 + 63.0, r.2 - 12.0, 19.0)
}
fn band_value_rect(i: usize) -> (f32, f32, f32, f32) {
    let r = band_rect(i);
    (r.0 + r.2 - 88.0, r.1 + 4.0, 80.0, 20.0)
}

#[derive(Clone, Copy, PartialEq)]
enum Target {
    Global(usize),
    Band(usize),
    Node(u64),
}
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
    fn rect(self, b: &Band, range: f64) -> (f32, f32, f32, f32) {
        let (x, y, _, _) = hud_rect_for(b, range);
        let h = self.count() as f32 * 24.0;
        (x + 40.0, (y + 32.0).min(GY + GH - h), 192.0, h)
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
                pse_page: false,
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
                pending_create: false,
                menu: None,
            }
            .build(cx, |cx| {
                let timer = cx.add_timer(Duration::from_millis(33), None, |cx, action| {
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
    pse_page: bool,
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
    pending_create: bool,
    menu: Option<BandMenu>,
}
fn freq_x(freq: f64) -> f32 {
    GX + GW * (freq / 20.0).log(1000.0).clamp(0.0, 1.0) as f32
}
fn x_freq(x: f32) -> f64 {
    20.0 * 1000.0_f64.powf(((x - GX) / GW).clamp(0.0, 1.0) as f64)
}
fn db_y(db: f64, range: f64) -> f32 {
    GY + GH * (0.5 - (db.clamp(-range, range) / (2.0 * range)) as f32)
}
fn y_db(y: f32, range: f64) -> f64 {
    ((0.5 - (y - GY) as f64 / GH as f64) * 2.0 * range).clamp(-24.0, 24.0)
}
fn inside(x: f32, y: f32, r: (f32, f32, f32, f32)) -> bool {
    x >= r.0 && x <= r.0 + r.2 && y >= r.1 && y <= r.1 + r.3
}
fn hud_rect_for(b: &Band, range: f64) -> (f32, f32, f32, f32) {
    let node_x = freq_x(b.freq);
    let node_y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, range);
    let bw = 304.0;
    let bh = 104.0;
    let gap = 36.0;
    let bx = (node_x - bw / 2.0).clamp(GX + 8.0, GX + GW - bw - 8.0);
    let above_room = node_y - GY;
    let below_room = GY + GH - node_y;
    let by = if above_room >= bh + gap || above_room >= below_room {
        (node_y - bh - gap).max(GY + 4.0)
    } else {
        (node_y + gap).min(GY + GH - bh - 4.0)
    };
    (bx, by, bw, bh)
}
fn band_rect(i: usize) -> (f32, f32, f32, f32) {
    (
        GX + 44.0 + i as f32 * 196.0,
        GRAPH_BOTTOM - 60.0,
        188.0,
        52.0,
    )
}
fn band_dyn_power_rect(dynamic: bool) -> (f32, f32, f32, f32) {
    let w = if dynamic { 28.0 } else { 130.0 };
    (GX + 8.0, GRAPH_BOTTOM - 48.0, w, 28.0)
}
fn band_bar_rect(i: usize) -> (f32, f32, f32, f32) {
    let r = band_rect(i);
    (r.0 + 12.0, r.1 + 28.0, r.2 - 24.0, 16.0)
}
fn mode_rect(i: usize) -> (f32, f32, f32, f32) {
    (420.0 + i as f32 * 142.0, 592.0, 132.0, 26.0)
}
fn global_rect(i: usize) -> (f32, f32, f32, f32) {
    let x = match i {
        0 | 7 => 44.0,
        6 | 8 => 152.0,
        1 => 260.0,
        2 => 380.0,
        3 | 9 => 500.0,
        4 => 608.0,
        10 => 687.0,
        _ => 740.0,
    };
    (x, 638.0, 100.0, 82.0)
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
    fn value_color(&self, target: ValueTarget) -> C {
        match target {
            ValueTarget::Global(2 | 3) => TEAL,
            ValueTarget::Global(_) => GOLD,
            ValueTarget::Band(_) => self
                .selected
                .map(|id| COLORS[(id as usize - 1) % COLORS.len()])
                .unwrap_or(TEAL),
        }
    }
    fn set_graph_range(&mut self, range: f64) {
        self.graph_db = display_range(range);
        *self.params.graph_range.lock().unwrap() = self.graph_db;
    }
    fn value_at(&self, x: f32, y: f32) -> Option<(ValueTarget, (f32, f32, f32, f32))> {
        if let Some(b) = self
            .params
            .bands
            .lock()
            .unwrap()
            .iter()
            .find(|b| Some(b.id) == self.selected)
        {
            for i in 0..3 {
                let active = match i {
                    1 => b.shape.has_gain(),
                    2 => !(b.shape.is_cut() && b.order == 1),
                    _ => true,
                };
                let r = hud_value_rect(b, self.graph_db, i);
                if active && inside(x, y, r) {
                    return Some((ValueTarget::Band(i), r));
                }
            }
            if b.shape.has_gain() && b.dynamic {
                for i in 0..5 {
                    let r = band_value_rect(i);
                    if inside(x, y, r) {
                        return Some((ValueTarget::Band(i + 3), r));
                    }
                }
            }
        }
        self.global_controls().iter().copied().find_map(|i| {
            let r = global_value_rect(i);
            inside(x, y, r).then_some((ValueTarget::Global(i), r))
        })
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
                let bands = self.params.bands.lock().unwrap();
                let Some(b) = bands.iter().find(|b| Some(b.id) == self.selected) else {
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
                if let Some(b) = self
                    .params
                    .bands
                    .lock()
                    .unwrap()
                    .iter()
                    .find(|b| Some(b.id) == self.selected)
                {
                    for i in 0..3 {
                        if (i != 1 || b.shape.has_gain())
                            && (i != 2 || !(b.shape.is_cut() && b.order == 1))
                        {
                            fields
                                .push((ValueTarget::Band(i), hud_value_rect(b, self.graph_db, i)));
                        }
                    }
                    if b.shape.has_gain() {
                        fields
                            .extend((0..5).map(|i| (ValueTarget::Band(i + 3), band_value_rect(i))));
                    }
                }
                fields.extend(
                    self.global_controls()
                        .iter()
                        .copied()
                        .map(|i| (ValueTarget::Global(i), global_value_rect(i))),
                );
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
        if self.pse_page {
            &[7, 8, 1, 2, 9, 10]
        } else {
            &[0, 6, 1, 2, 3, 4, 5]
        }
    }
    fn param(&self, i: usize) -> &FloatParam {
        match i {
            0 => &self.params.compression,
            1 => &self.params.gate,
            2 => &self.params.sc_hpf,
            3 => &self.params.dry,
            4 => &self.params.wet,
            5 => &self.params.output,
            7 => &self.params.pse_depth,
            8 => &self.params.pse_hysteresis,
            9 => &self.params.pse_knee,
            10 => &self.params.pse_time,
            _ => &self.params.comp_ratio,
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
    fn change(&self, f: impl FnOnce(&mut Band)) {
        if let Some(b) = self
            .params
            .bands
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
        let mut bands = self.params.bands.lock().unwrap();
        let id = bands.iter().map(|b| b.id).max().unwrap_or(0) + 1;
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
            Some(Target::Node(_)) => {
                if shift {
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
                        b.freq = x_freq(x);
                        if b.shape.has_gain() {
                            b.gain = y_db(y, graph_db);
                        }
                    });
                }
            }
            Some(Target::Global(i)) => {
                let p = self.param(i);
                let cur = p.unmodulated_normalized_value();
                let delta = (dy * if shift { 0.0015 } else { 0.007 }) as f32;
                let norm = (cur + if i == 0 { -delta } else { delta }).clamp(0.0, 1.0);
                cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
            }
            Some(Target::Band(i)) => {
                let r = band_rect(i);
                let n = ((x - r.0 - 12.0) / (r.2 - 24.0)).clamp(0.0, 1.0) as f64;
                self.change(|b| match i {
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
        if self.selected.is_some() {
            self.params
                .bands
                .lock()
                .unwrap()
                .retain(|b| Some(b.id) != self.selected);
            self.shared.solo_id.store(0, Ordering::Relaxed);
            self.select(None);
        }
    }
}
impl View for StripView {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, meta| {
            let bounds = cx.bounds();
            let scale = bounds.w / 1120.0;
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
                    if self.pending_create
                        && ((x - self.down.0).abs() + (y - self.down.1).abs()) > 4.0
                    {
                        let (dx, dy) = self.down;
                        self.create_band(dx, dy, true, cx.modifiers().alt());
                        self.pending_create = false;
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
                    if inside(x, y, PSE_COG_BUTTON) && !self.pse_page {
                        self.pse_page = true;
                        cx.needs_redraw();
                        return;
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
                        if inside(x, y, SCALE_MENU) {
                            let index = ((y - SCALE_MENU.1) / 24.0) as usize;
                            if let Some(range) = SCALES.get(index) {
                                self.set_graph_range(*range);
                            }
                        }
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, SCALE_BUTTON) {
                        self.scale_menu = true;
                        self.menu = None;
                        cx.needs_redraw();
                        return;
                    }
                    if let Some(menu) = self.menu.take() {
                        let band = self
                            .params
                            .bands
                            .lock()
                            .unwrap()
                            .iter()
                            .find(|b| Some(b.id) == self.selected)
                            .cloned();
                        if let Some(b) = band {
                            let r = menu.rect(&b, self.graph_db);
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
                        cx.needs_redraw();
                        return;
                    }
                    if let Some((target, rect)) = self.value_at(x, y) {
                        self.start_edit(target, rect);
                        cx.needs_redraw();
                        return;
                    }
                    if self.pse_page && inside(x, y, PSE_BACK_BUTTON) {
                        self.pse_page = false;
                        cx.needs_redraw();
                        return;
                    }
                    if let Some(i) = (0..3).find(|i| inside(x, y, mode_rect(*i))) {
                        if self.pse_page {
                            match i {
                                0 => self.toggle(cx, &self.params.pse_peak),
                                1 => self.toggle(cx, &self.params.pse_listen),
                                _ => {}
                            }
                        } else {
                            let p = match i {
                                0 => &self.params.soft_knee,
                                1 => &self.params.auto_makeup,
                                _ => &self.params.stereo_link,
                            };
                            self.toggle(cx, p);
                        }
                        cx.needs_redraw();
                        return;
                    }
                    self.down = (x, y);
                    self.last_drag = (x, y);
                    if inside(x, y, EQ_POWER) {
                        self.toggle(cx, &self.params.eq_on);
                    } else if inside(x, y, COMP_POWER) {
                        self.toggle(cx, &self.params.comp_on);
                    } else {
                        // Check if click hits selected band HUD
                        let mut hud_consumed = false;
                        if let Some(sel_id) = self.selected {
                            let band_opt = self
                                .params
                                .bands
                                .lock()
                                .unwrap()
                                .iter()
                                .find(|b| b.id == sel_id)
                                .cloned();
                            if let Some(b) = band_opt {
                                let (bx, by, bw, bh) = hud_rect_for(&b, self.graph_db);
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
                                    } else if inside(x, y, (bx + bw - 32.0, by + 6.0, 26.0, 26.0)) {
                                        self.delete();
                                    } else if inside(x, y, (bx + 10.0, by + 78.0, 108.0, 22.0))
                                        && b.shape.is_cut()
                                    {
                                        self.menu = Some(BandMenu::Order);
                                    }
                                }
                            }
                        }
                        if !hud_consumed {
                            if inside(x, y, (GX, GY, GW, GH)) {
                                let hit = self
                                    .params
                                    .bands
                                    .lock()
                                    .unwrap()
                                    .iter()
                                    .rev()
                                    .find(|b| {
                                        ((x - freq_x(b.freq)).powi(2)
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
                                    let sr = self.shared.sample_rate.load(Ordering::Relaxed) as f64;
                                    let config = self.params.processing_config();
                                    let eq_sr = config.mode.rate(sr);
                                    let db = self
                                        .params
                                        .bands
                                        .lock()
                                        .unwrap()
                                        .iter()
                                        .filter(|b| b.enabled)
                                        .map(|b| {
                                            BandCoeffs::make(b, eq_sr).response(x_freq(x), eq_sr)
                                        })
                                        .sum::<f64>();
                                    if (y - db_y(db, self.graph_db)).abs() < 12.0 {
                                        self.pending_create = true;
                                        cx.capture();
                                    } else if self.selected.is_none() {
                                        self.create_band(x, y, false, cx.modifiers().alt());
                                        cx.capture();
                                    } else {
                                        self.select(None);
                                    }
                                }
                            } else if let Some(i) = self
                                .global_controls()
                                .iter()
                                .copied()
                                .find(|i| inside(x, y, global_rect(*i)))
                            {
                                self.drag = Some(Target::Global(i));
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(self.param(i).as_ptr()));
                                cx.capture();
                            } else if self
                                .params
                                .bands
                                .lock()
                                .unwrap()
                                .iter()
                                .any(|b| Some(b.id) == self.selected && b.shape.has_gain())
                            {
                                let is_dynamic = self
                                    .params
                                    .bands
                                    .lock()
                                    .unwrap()
                                    .iter()
                                    .find(|b| Some(b.id) == self.selected)
                                    .map(|b| b.dynamic)
                                    .unwrap_or(false);
                                if inside(x, y, band_dyn_power_rect(is_dynamic)) {
                                    self.change(|b| b.dynamic = !b.dynamic);
                                } else if is_dynamic {
                                    if let Some(i) =
                                        (0..5).find(|i| inside(x, y, band_bar_rect(*i)))
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
                    if inside(x, y, (GX, GY, GW, GH)) && self.drag.is_none() {
                        let mut over_hud = false;
                        if let Some(sel_id) = self.selected {
                            if let Some(b) = self
                                .params
                                .bands
                                .lock()
                                .unwrap()
                                .iter()
                                .find(|b| b.id == sel_id)
                            {
                                if inside(x, y, hud_rect_for(b, self.graph_db)) {
                                    over_hud = true;
                                }
                            }
                        }
                        if !over_hud {
                            let hit = self
                                .params
                                .bands
                                .lock()
                                .unwrap()
                                .iter()
                                .rev()
                                .find(|b| {
                                    ((x - freq_x(b.freq)).powi(2)
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
                                self.create_band(x, y, false, cx.modifiers().alt());
                                cx.capture();
                            }
                        }
                    }
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    if let Some(Target::Global(i)) = self.drag {
                        cx.emit(RawParamEvent::EndSetParameter(self.param(i).as_ptr()));
                    }
                    if self.pending_create {
                        self.select(None);
                    }
                    self.drag = None;
                    self.pending_create = false;
                    cx.release();
                    cx.needs_redraw();
                }
                WindowEvent::MouseDown(MouseButton::Right) => {
                    if inside(x, y, global_rect(1)) && self.drag.is_none() {
                        self.pse_page = true;
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, (GX, GY, GW, GH)) {
                        let mut over_hud = false;
                        if let Some(sel_id) = self.selected {
                            if let Some(b) = self
                                .params
                                .bands
                                .lock()
                                .unwrap()
                                .iter()
                                .find(|b| b.id == sel_id)
                            {
                                if inside(x, y, hud_rect_for(b, self.graph_db)) {
                                    over_hud = true;
                                }
                            }
                        }
                        if !over_hud {
                            let id = self
                                .params
                                .bands
                                .lock()
                                .unwrap()
                                .iter()
                                .find(|b| {
                                    (x - freq_x(b.freq)).abs() < 15.0
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
                WindowEvent::MouseScroll(_, dy) => {
                    if inside(x, y, (14.0, GY, GX - 14.0, GRAPH_BOTTOM - GY + 32.0)) && *dy != 0.0 {
                        self.set_graph_range(self.graph_db - dy.signum() as f64 * 12.0);
                        cx.needs_redraw();
                        return;
                    }
                    if self.menu.is_some() || self.scale_menu || self.processing_menu.is_some() {
                        return;
                    }
                    if let Some((hx, hy)) = self.hover {
                        if let Some(i) = self
                            .global_controls()
                            .iter()
                            .copied()
                            .find(|i| inside(hx, hy, global_rect(*i)))
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
                    if inside(x, y, (GX, GY, GW, GH)) {
                        if self.selected.is_none() {
                            let hit = self
                                .params
                                .bands
                                .lock()
                                .unwrap()
                                .iter()
                                .rev()
                                .find(|b| {
                                    ((x - freq_x(b.freq)).powi(2)
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
                        if self.selected.is_some() {
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
            s: bounds.w / 1120.0,
            ox: bounds.x,
            oy: bounds.y,
            font: self.font.get(),
            light: preferences::light(),
        };
        d.rect(0.0, 0.0, 1120.0, 800.0, BG);
        d.rect(0.0, 0.0, 1120.0, 82.0, PANEL);
        d.text(32.0, 46.0, "dB", 28.0, GOLD);
        d.text(88.0, 46.0, "SIGNATURE CHANNEL STRIP", 16.0, TEXT);
        d.signature(self.signature.get());
        let eq_bypassed = !self.params.eq_on.value();
        let bands = self.params.bands.lock().unwrap().clone();
        d.text(
            32.0,
            123.0,
            "01",
            13.0,
            if eq_bypassed { MUTED } else { TEAL },
        );
        d.text(
            68.0,
            123.0,
            "PARAMETRIC EQ",
            16.0,
            if eq_bypassed { MUTED } else { TEXT },
        );
        if eq_bypassed {
            d.text(520.0, 123.0, "EQ STAGE BYPASSED", 12.0, GOLD);
        } else if self.shared.solo_id.load(Ordering::Relaxed) != 0 {
            d.text(520.0, 123.0, "SOLO AUDITION ACTIVE", 12.0, GOLD);
        }
        d.bypass_button(EQ_POWER, eq_bypassed, TEAL);

        for db in (-(self.graph_db as i32)..=self.graph_db as i32).step_by(12) {
            let y = db_y(db as f64, self.graph_db);
            d.line(
                GX,
                y,
                GX + GW,
                y,
                if db == 0 { C::rgb(75, 81, 85) } else { LINE },
                1.0,
            );
            d.text(23.0, y + 5.0, &format!("{:+}", db), 13.0, MUTED);
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
            let x = freq_x(freq);
            d.line(x, GY, x, GRAPH_BOTTOM, LINE, 1.0);
            d.text(x - 9.0, GRAPH_BOTTOM + 18.0, label, 12.0, MUTED);
        }
        let spectrum: Vec<_> = (0..128)
            .map(|i| {
                let db = self.shared.spectrum[i].load(Ordering::Relaxed);
                (
                    GX + i as f32 / 127.0 * GW,
                    GY + GH - (db + 90.0).clamp(0.0, 90.0) / 90.0 * GH * 0.86,
                )
            })
            .collect();
        d.area(
            &spectrum,
            GY + GH,
            if eq_bypassed {
                C::rgba(80, 90, 100, 10)
            } else {
                C::rgba(125, 143, 159, 28)
            },
        );
        d.poly(
            &spectrum,
            if eq_bypassed {
                C::rgba(90, 100, 110, 20)
            } else {
                C::rgba(144, 161, 175, 62)
            },
            1.0,
        );
        let sr = self.shared.sample_rate.load(Ordering::Relaxed) as f64;
        let config = self.params.processing_config();
        let eq_sr = config.mode.rate(sr);
        let mut sum = vec![0.0; 420];
        for b in &bands {
            if !b.enabled {
                continue;
            }
            let coeff = BandCoeffs::make(b, eq_sr);
            let color = if eq_bypassed {
                MUTED
            } else {
                COLORS[(b.id as usize - 1) % COLORS.len()]
            };
            let points: Vec<_> = (0..420)
                .map(|i| {
                    let x = GX + GW * i as f32 / 419.0;
                    let db = coeff.response(x_freq(x).min(sr * 0.49), eq_sr);
                    sum[i] += db;
                    (x, db_y(db, self.graph_db))
                })
                .collect();
            if Some(b.id) == self.selected {
                let mut fill = color;
                fill.a = if eq_bypassed { 0.02 } else { 0.07 };
                d.area(&points, db_y(0.0, self.graph_db), fill);
                d.poly(&points, color, 1.2);
            }
        }
        let points: Vec<_> = sum
            .iter()
            .enumerate()
            .map(|(i, db)| (GX + GW * i as f32 / 419.0, db_y(*db, self.graph_db)))
            .collect();
        d.poly(&points, if eq_bypassed { MUTED } else { GOLD }, 2.2);

        for b in &bands {
            let color = if !b.enabled || eq_bypassed {
                MUTED
            } else {
                COLORS[(b.id as usize - 1) % COLORS.len()]
            };
            let x = freq_x(b.freq);
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
        if let Some(b) = bands.iter().find(|b| Some(b.id) == self.selected) {
            let color = if eq_bypassed {
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
            );
        }
        if let Some((x, y)) = self.hover {
            if inside(x, y, (GX, GY, GW, GH))
                && self.drag.is_none()
                && self.menu.is_none()
                && !self.scale_menu
                && self.processing_menu.is_none()
                && self.edit.is_none()
                && !bands.iter().any(|b| {
                    Some(b.id) == self.selected && inside(x, y, hud_rect_for(b, self.graph_db))
                })
                && !eq_bypassed
            {
                let near = bands.iter().any(|b| {
                    (x - freq_x(b.freq)).abs() < 15.0
                        && (y - db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db))
                            .abs()
                            < 15.0
                });
                if !near {
                    let db = bands
                        .iter()
                        .filter(|b| b.enabled)
                        .map(|b| BandCoeffs::make(b, eq_sr).response(x_freq(x), eq_sr))
                        .sum::<f64>();
                    let curve = (y - db_y(db, self.graph_db)).abs() < 12.0;
                    let shape = infer_shape((x - GX) / GW, (y - GY) / GH, curve);
                    d.circle(x, y, 5.0, MUTED, false);
                    d.text(
                        (x + 12.0).min(910.0),
                        (y - 12.0).max(GY + 18.0),
                        &format!("+ {}", shape.name()),
                        12.0,
                        MUTED,
                    );
                }
            }
        }
        if let Some(b) = bands
            .iter()
            .find(|b| Some(b.id) == self.selected && b.shape.has_gain())
        {
            let color = COLORS[(b.id as usize - 1) % COLORS.len()];
            let power_rect = band_dyn_power_rect(b.dynamic);
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
                let active = !eq_bypassed;
                let c = if active { color } else { MUTED };
                d.outline(
                    power_rect,
                    if hovered {
                        TEXT
                    } else if active {
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
                    d.control(band_rect(i), label, &val, n as f32, c);
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
        }
        let comp_bypassed = !self.params.comp_on.value();
        if self.pse_page {
            d.button(PSE_BACK_BUTTON, "< BACK", false, TEXT);
            d.text(
                176.0,
                611.0,
                "PSE",
                16.0,
                if comp_bypassed { MUTED } else { GOLD },
            );
            d.button(
                mode_rect(0),
                if self.params.pse_peak.value() {
                    "DETECT: PEAK"
                } else {
                    "DETECT: RMS"
                },
                true,
                if comp_bypassed { MUTED } else { GOLD },
            );
            d.button(
                mode_rect(1),
                "LISTEN SC",
                self.params.pse_listen.value(),
                if comp_bypassed { MUTED } else { TEAL },
            );
            d.bypass_button(COMP_POWER, comp_bypassed, GOLD);
            d.rect(32.0, 634.0, 1054.0, 96.0, PANEL);
            d.outline((32.0, 634.0, 1054.0, 96.0), LINE);

            // Bay dividers
            d.line(372.0, 646.0, 372.0, 718.0, LINE, 1.0);
            d.line(492.0, 646.0, 492.0, 718.0, LINE, 1.0);
            d.line(615.0, 646.0, 615.0, 718.0, LINE, 1.0);
            d.line(858.0, 646.0, 858.0, 718.0, LINE, 1.0);

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
                    global_rect(i),
                    label,
                    &value,
                    p.unmodulated_normalized_value(),
                    if i == 2 { TEAL } else { GOLD },
                    comp_bypassed,
                );
            }
        } else {
            d.text(
                32.0,
                610.0,
                "02",
                13.0,
                if comp_bypassed { MUTED } else { GOLD },
            );
            d.text(
                68.0,
                611.0,
                "VOICE COMPRESSOR",
                16.0,
                if comp_bypassed { MUTED } else { TEXT },
            );
            for (i, label, active) in [
                (0, "SOFT KNEE", self.params.soft_knee.value()),
                (1, "AUTO MAKEUP", self.params.auto_makeup.value()),
                (2, "LINKED STEREO", self.params.stereo_link.value()),
            ] {
                d.button(
                    mode_rect(i),
                    label,
                    active,
                    if comp_bypassed { MUTED } else { GOLD },
                );
            }
            d.bypass_button(COMP_POWER, comp_bypassed, GOLD);

            // Compressor rack chassis
            d.rect(32.0, 634.0, 1054.0, 96.0, PANEL);
            d.outline((32.0, 634.0, 1054.0, 96.0), LINE);

            // Bay dividers
            d.line(372.0, 646.0, 372.0, 718.0, LINE, 1.0);
            d.line(492.0, 646.0, 492.0, 718.0, LINE, 1.0);
            d.line(728.0, 646.0, 728.0, 718.0, LINE, 1.0);
            d.line(858.0, 646.0, 858.0, 718.0, LINE, 1.0);

            // Threshold, ratio, PSE, detector, parallel mix, and output:
            let p0 = self.param(0);
            d.knob(
                global_rect(0),
                "THRESHOLD",
                &p0.normalized_value_to_string(p0.unmodulated_normalized_value(), true),
                1.0 - p0.unmodulated_normalized_value(),
                GOLD,
                comp_bypassed,
            );

            let ratio = self.param(6);
            d.knob(
                global_rect(6),
                "RATIO",
                &ratio.normalized_value_to_string(ratio.unmodulated_normalized_value(), true),
                ratio.unmodulated_normalized_value(),
                GOLD,
                comp_bypassed,
            );

            let p1 = self.param(1);
            let pse_val = if p1.value() <= -79.9 {
                "OFF".into()
            } else {
                p1.normalized_value_to_string(p1.unmodulated_normalized_value(), true)
            };
            d.knob(
                global_rect(1),
                "PSE",
                &pse_val,
                p1.unmodulated_normalized_value(),
                GOLD,
                comp_bypassed,
            );
            let cog_hover = !comp_bypassed
                && self
                    .hover
                    .is_some_and(|(hx, hy)| inside(hx, hy, PSE_COG_BUTTON));
            let cog_color = if comp_bypassed {
                MUTED
            } else if cog_hover {
                GOLD
            } else {
                MUTED
            };
            d.cog_icon(
                PSE_COG_BUTTON.0 + PSE_COG_BUTTON.2 * 0.5,
                PSE_COG_BUTTON.1 + PSE_COG_BUTTON.3 * 0.5,
                cog_color,
            );

            let p2 = self.param(2);
            d.knob(
                global_rect(2),
                "SC HPF",
                &p2.normalized_value_to_string(p2.unmodulated_normalized_value(), true),
                p2.unmodulated_normalized_value(),
                TEAL,
                comp_bypassed,
            );

            let p3 = self.param(3);
            d.knob(
                global_rect(3),
                "DRY",
                &p3.normalized_value_to_string(p3.unmodulated_normalized_value(), true),
                p3.unmodulated_normalized_value(),
                TEAL,
                comp_bypassed,
            );

            let p4 = self.param(4);
            d.knob(
                global_rect(4),
                "WET",
                &p4.normalized_value_to_string(p4.unmodulated_normalized_value(), true),
                p4.unmodulated_normalized_value(),
                GOLD,
                comp_bypassed,
            );

            let p5 = self.param(5);
            d.knob(
                global_rect(5),
                "OUTPUT",
                &p5.normalized_value_to_string(p5.unmodulated_normalized_value(), true),
                p5.unmodulated_normalized_value(),
                GOLD,
                comp_bypassed,
            );

            // Gain reduction meter
            let gr = if comp_bypassed {
                0.0
            } else {
                self.shared.gr.load(Ordering::Relaxed)
            };
            d.text(
                872.0 + 12.0,
                640.0 + 44.0,
                &format!("{:.1}", gr),
                22.0,
                if comp_bypassed { MUTED } else { GOLD },
            );
            d.text(872.0 + 82.0, 640.0 + 44.0, "dB", 11.0, MUTED);
            let meter_x = 872.0 + 12.0;
            let meter_y = 640.0 + 58.0;
            let meter_w = 186.0;
            let meter_h = 6.0;
            d.rect(meter_x, meter_y, meter_w, meter_h, LINE);
            let active_w = meter_w * (gr / 30.0).clamp(0.0, 1.0);
            if !comp_bypassed && active_w > 0.5 {
                d.rect(
                    meter_x + meter_w - active_w,
                    meter_y,
                    active_w,
                    meter_h,
                    GOLD,
                );
            }
            for (pos_frac, lbl) in [
                (0.0, "30"),
                (0.4, "18"),
                (0.6, "12"),
                (0.8, "6"),
                (0.9, "3"),
                (1.0, "0"),
            ] {
                let lx = meter_x + meter_w * pos_frac;
                d.line(lx, 640.0 + 66.0, lx, 640.0 + 70.0, LINE, 1.0);
                d.text_centered(lx, 640.0 + 79.0, lbl, 8.5, MUTED);
            }
        }

        d.line(32.0, 738.0, 1086.0, 738.0, LINE, 1.0);
        d.text(32.0, 769.0, &format!("{:.1} kHz", sr / 1000.0), 11.0, MUTED);
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
        d.text(status_x, 768.0, &status, 10.0, MUTED);
        d.button(
            THEME_BUTTON,
            if d.light { "DARK MODE" } else { "LIGHT MODE" },
            false,
            TEXT,
        );
        if let (Some(menu), Some(b)) = (
            self.menu,
            bands.iter().find(|b| Some(b.id) == self.selected),
        ) {
            let r = menu.rect(b, self.graph_db);
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            for row in 0..menu.count() {
                let (label, selected) = match menu {
                    BandMenu::Shape => (
                        Shape::ALL[row].name().to_string(),
                        b.shape == Shape::ALL[row],
                    ),
                    BandMenu::Order => (
                        format!("{} dB/oct  /  order {}", (row + 1) * 6, row + 1),
                        b.order as usize == row + 1,
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
        d.rect(
            SCALE_BUTTON.0,
            SCALE_BUTTON.1,
            SCALE_BUTTON.2,
            SCALE_BUTTON.3,
            PANEL,
        );
        d.text_centered(
            SCALE_BUTTON.0 + SCALE_BUTTON.2 * 0.5,
            SCALE_BUTTON.1 + 16.0,
            &format!("±{} ▾", self.graph_db as i32),
            9.5,
            MUTED,
        );
        if self.scale_menu {
            let r = SCALE_MENU;
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
}
impl Draw<'_> {
    fn color(&self, c: C) -> C {
        if !self.light {
            return c;
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
        result.a = c.a;
        result
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: C) {
        let mut p = Path::new();
        p.rounded_rect(
            self.ox + x * self.s,
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
                p.move_to(self.ox + x * self.s, self.oy + y * self.s);
            } else {
                p.line_to(self.ox + x * self.s, self.oy + y * self.s);
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
        p.move_to(self.ox + points[0].0 * self.s, self.oy + bottom * self.s);
        for (x, y) in points {
            p.line_to(self.ox + x * self.s, self.oy + y * self.s);
        }
        p.line_to(
            self.ox + points.last().unwrap().0 * self.s,
            self.oy + bottom * self.s,
        );
        p.close();
        self.c.fill_path(&p, &Paint::color(self.color(color)));
    }
    fn circle(&mut self, x: f32, y: f32, r: f32, color: C, fill: bool) {
        let mut p = Path::new();
        p.circle(self.ox + x * self.s, self.oy + y * self.s, r * self.s);
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
        let _ = self
            .c
            .fill_text(self.ox + x * self.s, self.oy + y * self.s, text, &p);
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
    fn outline(&mut self, r: (f32, f32, f32, f32), color: C) {
        let mut path = Path::new();
        path.rounded_rect(
            self.ox + r.0 * self.s,
            self.oy + r.1 * self.s,
            r.2 * self.s,
            r.3 * self.s,
            5.0 * self.s,
        );
        let mut paint = Paint::color(self.color(color));
        paint.set_line_width(self.s);
        self.c.stroke_path(&path, &paint);
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
            self.ox + (cx - width * 0.5) * self.s,
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
    fn band_hud(&mut self, b: &Band, is_solo: bool, color: C, range: f64, _linear: bool) {
        let (bx, by, bw, _) = hud_rect_for(b, range);
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
            let r = hud_value_rect(b, range, i);
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
        let mut paint = Paint::linear_gradient(
            self.ox + 800.0 * self.s,
            self.oy + 13.0 * self.s,
            self.ox + 810.0 * self.s,
            self.oy + 63.0 * self.s,
            self.color(C::rgb(255, 230, 163)),
            self.color(C::rgb(176, 119, 33)),
        );
        if let Some(f) = font {
            paint.set_font(&[f]);
        }
        paint.set_font_size(44.0 * self.s);
        let _ = self.c.fill_text(
            self.ox + 794.0 * self.s,
            self.oy + 51.0 * self.s,
            "Damian Birdsey",
            &paint,
        );
        let mut underline = Path::new();
        for (i, (x, y)) in [(821.0, 67.0), (870.0, 64.0), (972.0, 65.0), (1073.0, 59.0)]
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
            self.ox + 821.0 * self.s,
            self.oy,
            self.ox + 1073.0 * self.s,
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
}
