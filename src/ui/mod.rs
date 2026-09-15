use crate::{
    band::{infer_shape, Band, Shape},
    dsp::Coeff,
    engine::Shared,
    params::StripParams,
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
const GH: f32 = 270.0;
#[derive(Clone, Copy, PartialEq)]
enum Target {
    Global(usize),
    Band(usize),
    Node(u64),
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
                drag: None,
                hover: None,
                font: Cell::new(None),
                signature: Cell::new(None),
                history: Vec::new(),
                future: Vec::new(),
                down: (0.0, 0.0),
                pending_create: false,
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
    params: Arc<StripParams>,
    shared: Arc<Shared>,
    selected: Option<u64>,
    drag: Option<Target>,
    hover: Option<(f32, f32)>,
    font: Cell<Option<FontId>>,
    signature: Cell<Option<FontId>>,
    history: Vec<Vec<Band>>,
    future: Vec<Vec<Band>>,
    down: (f32, f32),
    pending_create: bool,
}
fn freq_x(freq: f64) -> f32 {
    GX + GW * (freq / 20.0).log(1000.0).clamp(0.0, 1.0) as f32
}
fn x_freq(x: f32) -> f64 {
    20.0 * 1000.0_f64.powf(((x - GX) / GW).clamp(0.0, 1.0) as f64)
}
fn db_y(db: f64) -> f32 {
    GY + GH * (0.5 - db.clamp(-24.0, 24.0) as f32 / 48.0)
}
fn y_db(y: f32) -> f64 {
    ((0.5 - (y - GY) / GH) * 48.0).clamp(-24.0, 24.0) as f64
}
fn inside(x: f32, y: f32, r: (f32, f32, f32, f32)) -> bool {
    x >= r.0 && x <= r.0 + r.2 && y >= r.1 && y <= r.1 + r.3
}
fn band_rect(i: usize) -> (f32, f32, f32, f32) {
    if i < 3 {
        (32.0 + i as f32 * 157.0, 464.0, 145.0, 52.0)
    } else {
        (220.0 + (i - 3) as f32 * 172.0, 534.0, 158.0, 50.0)
    }
}
fn global_rect(i: usize) -> (f32, f32, f32, f32) {
    (32.0 + i as f32 * 181.0, 641.0, 165.0, 79.0)
}
impl StripView {
    fn param(&self, i: usize) -> &FloatParam {
        match i {
            0 => &self.params.compression,
            1 => &self.params.gate,
            2 => &self.params.sc_hpf,
            3 => &self.params.output,
            _ => &self.params.mix,
        }
    }
    fn checkpoint(&mut self) {
        self.history.push(self.params.bands.lock().unwrap().clone());
        if self.history.len() > 80 {
            self.history.remove(0);
        }
        self.future.clear();
    }
    fn select(&mut self, id: Option<u64>) {
        self.selected = id;
        self.shared
            .selected_id
            .store(id.unwrap_or(0), Ordering::Relaxed);
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
        self.checkpoint();
        let mut bands = self.params.bands.lock().unwrap();
        let id = bands.iter().map(|b| b.id).max().unwrap_or(0) + 1;
        let shape = infer_shape((x - GX) / GW, (y - GY) / GH, curve);
        bands.push(Band {
            id,
            shape,
            freq: x_freq(x),
            gain: if shape.has_gain() { y_db(y) } else { 0.0 },
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
    }
    fn apply(&self, cx: &mut EventContext, x: f32, y: f32) {
        match self.drag {
            Some(Target::Node(_)) => self.change(|b| {
                b.freq = x_freq(x);
                if b.shape.has_gain() {
                    b.gain = y_db(y);
                }
            }),
            Some(Target::Global(i)) => {
                let r = global_rect(i);
                let norm = ((x - r.0 - 12.0) / (r.2 - 24.0)).clamp(0.0, 1.0);
                cx.emit(RawParamEvent::SetParameterNormalized(
                    self.param(i).as_ptr(),
                    norm,
                ));
            }
            Some(Target::Band(i)) => {
                let r = band_rect(i);
                let n = ((x - r.0 - 8.0) / (r.2 - 16.0)).clamp(0.0, 1.0) as f64;
                self.change(|b| match i {
                    0 => b.freq = 20.0 * 1000.0_f64.powf(n),
                    1 => b.gain = -24.0 + 48.0 * n,
                    2 => b.q = 0.15 * 120.0_f64.powf(n),
                    3 => b.threshold = -60.0 + 60.0 * n,
                    4 => b.ratio = 1.0 + 19.0 * n,
                    5 => b.attack = 0.1 * 2000.0_f64.powf(n),
                    6 => b.release = 10.0 * 200.0_f64.powf(n),
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
            self.checkpoint();
            self.params
                .bands
                .lock()
                .unwrap()
                .retain(|b| Some(b.id) != self.selected);
            self.select(None);
        }
    }
}
impl View for StripView {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| {
            let bounds = cx.bounds();
            let scale = bounds.w / 1120.0;
            let x = (cx.mouse().cursorx - bounds.x) / scale;
            let y = (cx.mouse().cursory - bounds.y) / scale;
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
                        self.apply(cx, x, y);
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseDown(MouseButton::Left) => {
                    cx.focus();
                    self.down = (x, y);
                    if inside(x, y, (982.0, 26.0, 104.0, 32.0)) {
                        self.toggle(cx, &self.params.bypass);
                    } else if inside(x, y, (990.0, 103.0, 96.0, 28.0)) {
                        self.toggle(cx, &self.params.eq_on);
                    } else if inside(x, y, (978.0, 601.0, 108.0, 28.0)) {
                        self.toggle(cx, &self.params.comp_on);
                    } else if inside(x, y, (790.0, 26.0, 68.0, 32.0)) && !self.history.is_empty() {
                        let state = self.history.pop().unwrap();
                        let old = std::mem::replace(&mut *self.params.bands.lock().unwrap(), state);
                        self.future.push(old);
                        self.select(None);
                    } else if inside(x, y, (866.0, 26.0, 68.0, 32.0)) && !self.future.is_empty() {
                        let state = self.future.pop().unwrap();
                        let old = std::mem::replace(&mut *self.params.bands.lock().unwrap(), state);
                        self.history.push(old);
                        self.select(None);
                    } else if inside(x, y, (32.0, 433.0, 100.0, 24.0)) {
                        self.create_band(freq_x(1000.0), db_y(0.0), false, false);
                        self.drag = None;
                    } else if inside(x, y, (GX, GY, GW, GH)) {
                        let hit = self
                            .params
                            .bands
                            .lock()
                            .unwrap()
                            .iter()
                            .rev()
                            .find(|b| {
                                ((x - freq_x(b.freq)).powi(2)
                                    + (y - db_y(if b.shape.has_gain() { b.gain } else { 0.0 }))
                                        .powi(2))
                                .sqrt()
                                    < 14.0
                            })
                            .map(|b| b.id);
                        if let Some(id) = hit {
                            self.select(Some(id));
                            self.checkpoint();
                            if cx.modifiers().alt() {
                                self.change(|b| b.enabled = !b.enabled);
                            } else {
                                self.drag = Some(Target::Node(id));
                                cx.capture();
                            }
                        } else {
                            let sr = self.shared.sample_rate.load(Ordering::Relaxed) as f64;
                            let db = self
                                .params
                                .bands
                                .lock()
                                .unwrap()
                                .iter()
                                .filter(|b| b.enabled)
                                .map(|b| {
                                    Coeff::make(b.shape, b.freq, b.gain, b.q, sr)
                                        .response(x_freq(x), sr)
                                })
                                .sum::<f64>();
                            if (y - db_y(db)).abs() < 12.0 {
                                self.pending_create = true;
                                cx.capture();
                            } else if self.selected.is_none() || cx.modifiers().command() {
                                self.create_band(x, y, false, cx.modifiers().alt());
                                cx.capture();
                            } else {
                                self.select(None);
                            }
                        }
                    } else if self.selected.is_some() && inside(x, y, (515.0, 464.0, 154.0, 52.0)) {
                        self.checkpoint();
                        self.change(|b| b.shape = b.shape.next());
                    } else if self.selected.is_some() && inside(x, y, (683.0, 464.0, 140.0, 52.0)) {
                        self.checkpoint();
                        self.change(|b| {
                            if b.shape.has_gain() {
                                b.dynamic = !b.dynamic;
                            }
                        });
                    } else if self.selected.is_some() && inside(x, y, (837.0, 464.0, 113.0, 52.0)) {
                        self.checkpoint();
                        self.change(|b| b.enabled = !b.enabled);
                    } else if self.selected.is_some() && inside(x, y, (964.0, 464.0, 122.0, 52.0)) {
                        self.delete();
                    } else if let Some(i) = (0..5).find(|i| inside(x, y, global_rect(*i))) {
                        self.drag = Some(Target::Global(i));
                        cx.emit(RawParamEvent::BeginSetParameter(self.param(i).as_ptr()));
                        cx.capture();
                        self.apply(cx, x, y);
                    } else if self.selected.is_some() {
                        if let Some(i) = (0..8).find(|i| inside(x, y, band_rect(*i))) {
                            self.checkpoint();
                            self.drag = Some(Target::Band(i));
                            cx.capture();
                            self.apply(cx, x, y);
                        }
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    if inside(x, y, (GX, GY, GW, GH)) && self.drag.is_none() {
                        self.create_band(x, y, false, cx.modifiers().alt());
                        cx.capture();
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
                    if inside(x, y, (GX, GY, GW, GH)) {
                        let id = self
                            .params
                            .bands
                            .lock()
                            .unwrap()
                            .iter()
                            .find(|b| {
                                (x - freq_x(b.freq)).abs() < 15.0
                                    && (y - db_y(if b.shape.has_gain() { b.gain } else { 0.0 }))
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
                WindowEvent::MouseScroll(_, dy) => {
                    if inside(x, y, (GX, GY, GW, GH)) && self.selected.is_some() {
                        self.checkpoint();
                        self.change(|b| b.q = (b.q * (1.0 + *dy as f64 * 0.08)).clamp(0.15, 18.0));
                        cx.needs_redraw();
                    }
                }
                WindowEvent::KeyDown(code, _) => {
                    if *code == Code::Delete || *code == Code::Backspace {
                        self.delete();
                        cx.needs_redraw();
                    }
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
        };
        d.rect(0.0, 0.0, 1120.0, 800.0, BG);
        d.rect(0.0, 0.0, 1120.0, 82.0, PANEL);
        d.text(32.0, 43.0, "DB", 28.0, GOLD);
        d.text(92.0, 39.0, "DAMIAN", 22.0, TEXT);
        d.text(93.0, 59.0, "SIGNATURE CHANNEL STRIP", 10.0, MUTED);
        d.text(399.0, 43.0, "01  EQ   /   02  VOICE   /   OUT", 11.0, MUTED);
        d.button(
            (790.0, 26.0, 68.0, 32.0),
            "Undo",
            !self.history.is_empty(),
            MUTED,
        );
        d.button(
            (866.0, 26.0, 68.0, 32.0),
            "Redo",
            !self.future.is_empty(),
            MUTED,
        );
        d.button(
            (982.0, 26.0, 104.0, 32.0),
            if self.params.bypass.value() {
                "BYPASSED"
            } else {
                "ACTIVE"
            },
            true,
            if self.params.bypass.value() {
                MUTED
            } else {
                TEAL
            },
        );
        let bands = self.params.bands.lock().unwrap().clone();
        d.text(32.0, 123.0, "01", 13.0, TEAL);
        d.text(68.0, 123.0, "PARAMETRIC EQ", 16.0, TEXT);
        d.text(268.0, 123.0, "DYNAMIC BANDS", 11.0, MUTED);
        d.text(775.0, 123.0, &format!("{} BANDS", bands.len()), 12.0, MUTED);
        d.button(
            (990.0, 103.0, 96.0, 28.0),
            if self.params.eq_on.value() {
                "EQ ON"
            } else {
                "EQ OFF"
            },
            self.params.eq_on.value(),
            TEAL,
        );
        for db in [-24, -12, 0, 12, 24] {
            let y = db_y(db as f64);
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
            d.line(x, GY, x, GY + GH, LINE, 1.0);
            d.text(x - 9.0, GH + GY + 22.0, label, 13.0, MUTED);
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
        d.area(&spectrum, GY + GH, C::rgba(125, 143, 159, 28));
        d.poly(&spectrum, C::rgba(144, 161, 175, 62), 1.0);
        let sr = self.shared.sample_rate.load(Ordering::Relaxed) as f64;
        let mut sum = vec![0.0; 420];
        for b in &bands {
            if !b.enabled {
                continue;
            }
            let coeff = Coeff::make(b.shape, b.freq, b.gain, b.q, sr);
            let color = COLORS[(b.id as usize - 1) % COLORS.len()];
            let points: Vec<_> = (0..420)
                .map(|i| {
                    let x = GX + GW * i as f32 / 419.0;
                    let db = coeff.response(x_freq(x).min(sr * 0.49), sr);
                    sum[i] += db;
                    (x, db_y(db))
                })
                .collect();
            if Some(b.id) == self.selected {
                let mut fill = color;
                fill.a = 0.07;
                d.area(&points, db_y(0.0), fill);
                d.poly(&points, color, 1.2);
            }
        }
        let points: Vec<_> = sum
            .iter()
            .enumerate()
            .map(|(i, db)| (GX + GW * i as f32 / 419.0, db_y(*db)))
            .collect();
        d.poly(&points, GOLD, 2.2);
        for b in &bands {
            let color = if b.enabled {
                COLORS[(b.id as usize - 1) % COLORS.len()]
            } else {
                MUTED
            };
            let x = freq_x(b.freq);
            let y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 });
            if Some(b.id) == self.selected {
                d.circle(x, y, 12.0, color, false);
            }
            if b.dynamic {
                d.line(x, y, x, db_y(b.gain - b.range), color, 3.0);
                d.circle(x, db_y(b.gain - b.range), 3.0, color, true);
            }
            d.circle(x, y, 7.0, color, true);
            d.text(x - 4.0, y - 17.0, &b.id.to_string(), 11.0, color);
        }
        if let Some((x, y)) = self.hover {
            if inside(x, y, (GX, GY, GW, GH)) && self.drag.is_none() {
                let near = bands.iter().any(|b| {
                    (x - freq_x(b.freq)).abs() < 15.0
                        && (y - db_y(if b.shape.has_gain() { b.gain } else { 0.0 })).abs() < 15.0
                });
                if !near {
                    let db = bands
                        .iter()
                        .filter(|b| b.enabled)
                        .map(|b| {
                            Coeff::make(b.shape, b.freq, b.gain, b.q, sr).response(x_freq(x), sr)
                        })
                        .sum::<f64>();
                    let curve = (y - db_y(db)).abs() < 12.0;
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
        d.text(32.0, 450.0, "+ ADD BAND", 10.0, TEAL);
        d.text(
            350.0,
            450.0,
            "CLICK TO ADD  /  DRAG TO SHAPE  /  SCROLL FOR Q  /  ALT = DYNAMIC",
            9.5,
            MUTED,
        );
        if let Some(b) = bands.iter().find(|b| Some(b.id) == self.selected) {
            let color = COLORS[(b.id as usize - 1) % COLORS.len()];
            d.control(
                band_rect(0),
                "FREQUENCY",
                &hz(b.freq),
                ((b.freq / 20.0).log(1000.0)) as f32,
                color,
            );
            d.control(
                band_rect(1),
                "GAIN",
                &format!("{:+.1} dB", b.gain),
                ((b.gain + 24.0) / 48.0) as f32,
                color,
            );
            d.control(
                band_rect(2),
                "Q / WIDTH",
                &format!("{:.2}", b.q),
                ((b.q / 0.15).log(120.0)) as f32,
                color,
            );
            d.button((515.0, 464.0, 154.0, 52.0), b.shape.name(), true, color);
            d.button(
                (683.0, 464.0, 140.0, 52.0),
                if b.dynamic {
                    "DYNAMIC ON"
                } else {
                    "DYNAMIC OFF"
                },
                b.dynamic,
                color,
            );
            d.button(
                (837.0, 464.0, 113.0, 52.0),
                if b.enabled { "ENABLED" } else { "BYPASSED" },
                b.enabled,
                color,
            );
            d.button((964.0, 464.0, 122.0, 52.0), "DELETE", false, MUTED);
            d.text(
                32.0,
                553.0,
                "BAND DYNAMICS",
                11.0,
                if b.dynamic { color } else { MUTED },
            );
            d.text(
                32.0,
                576.0,
                &format!("GR  {:.1} dB", self.shared.band_gr.load(Ordering::Relaxed)),
                16.0,
                color,
            );
            for (i, label, val, n) in [
                (
                    3,
                    "THRESHOLD",
                    format!("{:.1} dB", b.threshold),
                    (b.threshold + 60.0) / 60.0,
                ),
                (
                    4,
                    "RATIO",
                    format!("{:.1}:1", b.ratio),
                    (b.ratio - 1.0) / 19.0,
                ),
                (
                    5,
                    "ATTACK",
                    format!("{:.1} ms", b.attack),
                    (b.attack / 0.1).log(2000.0),
                ),
                (
                    6,
                    "RELEASE",
                    format!("{:.0} ms", b.release),
                    (b.release / 10.0).log(200.0),
                ),
                (7, "RANGE", format!("{:.1} dB", b.range), b.range / 24.0),
            ] {
                d.control(
                    band_rect(i),
                    label,
                    &val,
                    n as f32,
                    if b.dynamic { color } else { MUTED },
                );
            }
        } else {
            d.rect(32.0, 464.0, 1054.0, 120.0, PANEL);
            d.text(
                57.0,
                498.0,
                "A little space. A little air. Your sound.",
                17.0,
                TEXT,
            );
            d.text(
                57.0,
                526.0,
                "Add a band anywhere on the graph, or pull the gold curve.",
                12.0,
                MUTED,
            );
            d.text(
                57.0,
                552.0,
                "Edges: cuts   /   Curve ends: shelves   /   Bottom: notch",
                12.0,
                MUTED,
            );
            d.text(890.0, 535.0, "NO BAND LIMIT", 12.0, TEAL);
        }
        d.line(32.0, 596.0, 1086.0, 596.0, LINE, 1.0);
        d.text(32.0, 622.0, "02", 13.0, GOLD);
        d.text(68.0, 622.0, "VOICE COMPRESSOR", 16.0, TEXT);
        d.text(
            315.0,
            622.0,
            "SOFT KNEE  /  AUTO MAKEUP  /  LINKED STEREO",
            10.0,
            MUTED,
        );
        d.button(
            (978.0, 601.0, 108.0, 28.0),
            if self.params.comp_on.value() {
                "COMP ON"
            } else {
                "COMP OFF"
            },
            self.params.comp_on.value(),
            GOLD,
        );
        for (i, name) in ["COMPRESSION", "GATE", "SIDECHAIN HPF", "OUTPUT", "MIX"]
            .iter()
            .enumerate()
        {
            let p = self.param(i);
            let val = if i == 1 && p.value() <= -79.9 {
                "OFF".into()
            } else {
                p.normalized_value_to_string(p.unmodulated_normalized_value(), true)
            };
            d.control(
                global_rect(i),
                name,
                &val,
                p.unmodulated_normalized_value(),
                if i == 2 { TEAL } else { GOLD },
            );
        }
        let gr = self.shared.gr.load(Ordering::Relaxed);
        d.text(955.0, 657.0, "REDUCTION", 10.0, MUTED);
        d.text(955.0, 684.0, &format!("{:.1}", gr), 24.0, GOLD);
        d.text(1028.0, 684.0, "dB", 12.0, MUTED);
        d.rect(955.0, 704.0, 125.0, 4.0, LINE);
        d.rect(955.0, 704.0, 125.0 * (gr / 30.0).clamp(0.0, 1.0), 4.0, GOLD);
        d.line(32.0, 738.0, 1086.0, 738.0, LINE, 1.0);
        d.text(32.0, 769.0, "IN", 10.0, MUTED);
        d.text(
            57.0,
            769.0,
            &format!(
                "{:.1} dB",
                self.shared.input.load(Ordering::Relaxed).max(-90.0)
            ),
            12.0,
            TEXT,
        );
        d.text(157.0, 769.0, "OUT", 10.0, MUTED);
        d.text(
            191.0,
            769.0,
            &format!(
                "{:.1} dB",
                self.shared.output.load(Ordering::Relaxed).max(-90.0)
            ),
            12.0,
            if self.shared.output.load(Ordering::Relaxed) > 0.0 {
                C::rgb(238, 125, 112)
            } else {
                TEXT
            },
        );
        d.text(
            358.0,
            769.0,
            &format!("{:.1} kHz   /   ZERO LATENCY", sr / 1000.0),
            10.0,
            MUTED,
        );
        d.signature(self.signature.get());
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
    c: &'a mut Canvas,
    s: f32,
    ox: f32,
    oy: f32,
    font: Option<FontId>,
}
impl Draw<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: C) {
        let mut p = Path::new();
        p.rect(
            self.ox + x * self.s,
            self.oy + y * self.s,
            w * self.s,
            h * self.s,
        );
        self.c.fill_path(&p, &Paint::color(color));
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
        let mut paint = Paint::color(color);
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
        self.c.fill_path(&p, &Paint::color(color));
    }
    fn circle(&mut self, x: f32, y: f32, r: f32, color: C, fill: bool) {
        let mut p = Path::new();
        p.circle(self.ox + x * self.s, self.oy + y * self.s, r * self.s);
        let mut paint = Paint::color(color);
        paint.set_line_width(self.s);
        if fill {
            self.c.fill_path(&p, &paint);
        } else {
            self.c.stroke_path(&p, &paint);
        }
    }
    fn text(&mut self, x: f32, y: f32, text: &str, size: f32, color: C) {
        let mut p = Paint::color(color);
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
    fn control(&mut self, r: (f32, f32, f32, f32), label: &str, value: &str, n: f32, color: C) {
        self.rect(r.0, r.1, r.2, r.3, PANEL);
        self.text(r.0 + 12.0, r.1 + 17.0, label, 10.0, MUTED);
        self.text(
            r.0 + 12.0,
            r.1 + 38.0,
            value,
            if r.3 > 60.0 { 21.0 } else { 15.0 },
            TEXT,
        );
        let y = r.1 + r.3 - 8.0;
        self.rect(r.0 + 12.0, y, r.2 - 24.0, 2.0, LINE);
        self.rect(r.0 + 12.0, y, (r.2 - 24.0) * n.clamp(0.0, 1.0), 2.0, color);
    }
    fn signature(&mut self, font: Option<FontId>) {
        let mut paint = Paint::linear_gradient(
            self.ox + 800.0 * self.s,
            self.oy + 743.0 * self.s,
            self.ox + 810.0 * self.s,
            self.oy + 793.0 * self.s,
            C::rgb(255, 230, 163),
            C::rgb(176, 119, 33),
        );
        if let Some(f) = font {
            paint.set_font(&[f]);
        }
        paint.set_font_size(44.0 * self.s);
        let _ = self.c.fill_text(
            self.ox + 770.0 * self.s,
            self.oy + 781.0 * self.s,
            "Damian Birdsey",
            &paint,
        );
        self.poly(
            &[
                (797.0, 787.0),
                (846.0, 784.0),
                (948.0, 785.0),
                (1049.0, 779.0),
            ],
            GOLD,
            1.0,
        );
        self.line(1068.0, 752.0, 1068.0, 766.0, GOLD, 1.0);
        self.line(1061.0, 759.0, 1075.0, 759.0, GOLD, 1.0);
    }
}
