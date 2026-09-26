//! Detector equalizer, using the same response model and node artwork as Damian.
use std::sync::{atomic::Ordering, Arc};

use nih_plug::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::{util::ModifiersExt, RawParamEvent};
use pleasant_eq::{BandCoefficients, BandSettings, EqShape};
use pleasant_ui::theme::EQ_COLORS as COLORS;
use pleasant_ui::{
    pointer::ValuePress, ButtonAnim, Draw, ValueEdit, GOLD, LINE, MUTED, PANEL, TEXT,
};

use super::display::UiDisplay;
use crate::{detector_eq::DetectorShape, params::ComposureParams};

// Use artwork coordinates directly: enlarging the graph must not enlarge its UI chrome.
const WIDTH: f32 = super::graph_pages::PAGE_W;
const HEIGHT: f32 = super::appearance::PLEASANT_GRAPH_SIZE + super::graph_pages::CURVE_HEADER_H;
const GRAPH: (f32, f32, f32, f32) = (36.0, 76.0, WIDTH - 52.0, HEIGHT - 148.0);
const CENTER_X: f32 = WIDTH * 0.5;
const ACTIONS_Y: f32 = HEIGHT - 36.0;
const FOOTER_X: f32 = (WIDTH - 434.0) * 0.5;
const BYPASS_RECT: (f32, f32, f32, f32) = (FOOTER_X, ACTIONS_Y, 26.0, 28.0);
const SHAPE_RECT: (f32, f32, f32, f32) = (FOOTER_X + 32.0, ACTIONS_Y, 100.0, 28.0);
const REMOVE_RECT: (f32, f32, f32, f32) = (FOOTER_X + 416.0, ACTIONS_Y, 18.0, 28.0);
const LISTEN_RECT: (f32, f32, f32, f32) = (36.0, 38.0, 28.0, 26.0);
const SIDECHAIN_RECT: (f32, f32, f32, f32) = (76.0, 38.0, 210.0, 26.0);
const MENU_ROW_H: f32 = 26.0;
const SHAPE_MENU: (f32, f32, f32, f32) = (
    SHAPE_RECT.0,
    ACTIONS_Y - MENU_ROW_H * DetectorShape::ALL.len() as f32 - 4.0,
    SHAPE_RECT.2,
    MENU_ROW_H * DetectorShape::ALL.len() as f32,
);
fn menu_shape(point: (f32, f32)) -> Option<DetectorShape> {
    if !contains(SHAPE_MENU, point) {
        return None;
    }
    DetectorShape::ALL
        .get(((point.1 - SHAPE_MENU.1) / MENU_ROW_H) as usize)
        .copied()
}
const GAIN_DB: f32 = 24.0;
const FIELDS_Y: f32 = ACTIONS_Y;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Node {
    Band(usize),
    HighPass,
    LowPass,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Frequency,
    Gain,
    Q,
}
#[derive(Clone, Copy, Debug)]
struct Target {
    node: Node,
    field: Field,
}

struct NodeDrag {
    node: Node,
    fields: Vec<Field>,
    origin: (f32, f32),
    frequency: f32,
    gain: f32,
    q: f32,
}
struct ReadoutPress {
    press: ValuePress<Target>,
    start: f32,
    active: bool,
}

pub struct DetectorEqView {
    params: Arc<ComposureParams>,
    display: Arc<UiDisplay>,
    selected: Option<Node>,
    drag: Option<NodeDrag>,
    press: Option<ReadoutPress>,
    edit: Option<ValueEdit<Target>>,
    hover: Option<(f32, f32)>,
    shape_menu: bool,
    bypass_anim: ButtonAnim,
}

fn contains(r: (f32, f32, f32, f32), p: (f32, f32)) -> bool {
    p.0 >= r.0 && p.0 <= r.0 + r.2 && p.1 >= r.1 && p.1 <= r.1 + r.3
}
fn frequency_x(frequency: f32) -> f32 {
    GRAPH.0 + GRAPH.2 * (frequency.clamp(20.0, 20_000.0) / 20.0).ln() / 1000.0_f32.ln()
}
fn x_frequency(x: f32) -> f32 {
    20.0 * 1000.0_f32.powf(((x - GRAPH.0) / GRAPH.2).clamp(0.0, 1.0))
}
fn gain_y(gain: f32) -> f32 {
    GRAPH.1 + GRAPH.3 * (0.5 - gain.clamp(-GAIN_DB, GAIN_DB) / (GAIN_DB * 2.0))
}
fn y_gain(y: f32) -> f32 {
    ((0.5 - (y - GRAPH.1) / GRAPH.3) * GAIN_DB * 2.0).clamp(-GAIN_DB, GAIN_DB)
}
fn field_rect(field: Field, _fields: &[Field]) -> (f32, f32, f32, f32) {
    let (offset, width) = match field {
        Field::Frequency => (138.0, 96.0),
        Field::Gain => (240.0, 100.0),
        Field::Q => (346.0, 64.0),
    };
    (FOOTER_X + offset, FIELDS_Y, width, 28.0)
}

fn node_fields(node: Node, shape: EqShape) -> Vec<Field> {
    if matches!(node, Node::Band(_)) {
        if shape.has_gain() {
            vec![Field::Frequency, Field::Gain, Field::Q]
        } else {
            vec![Field::Frequency, Field::Q]
        }
    } else {
        vec![Field::Frequency]
    }
}

fn response_frequency(x: f32, sample_rate: f64) -> f64 {
    (x_frequency(x) as f64).min(sample_rate.max(1.0) * 0.49)
}

fn node_color(node: Node) -> nih_plug_vizia::vizia::vg::Color {
    COLORS[match node {
        Node::Band(i) => i,
        Node::HighPass => 6,
        Node::LowPass => 7,
    } % COLORS.len()]
}
fn set_once<P: Param>(cx: &mut EventContext, param: &P, normalized: f32) {
    cx.emit(RawParamEvent::BeginSetParameter(param.as_ptr()));
    cx.emit(RawParamEvent::SetParameterNormalized(
        param.as_ptr(),
        normalized.clamp(0.0, 1.0),
    ));
    cx.emit(RawParamEvent::EndSetParameter(param.as_ptr()));
}

impl DetectorEqView {
    pub fn new(
        cx: &mut Context,
        params: Arc<ComposureParams>,
        display: Arc<UiDisplay>,
    ) -> Handle<'_, Self> {
        Self {
            params,
            display,
            selected: None,
            drag: None,
            press: None,
            edit: None,
            hover: None,
            shape_menu: false,
            bypass_anim: ButtonAnim::new(),
        }
        .build(cx, |cx| {
            let timer = cx.add_timer(std::time::Duration::from_millis(16), None, |cx, action| {
                if matches!(action, TimerAction::Tick(_)) {
                    cx.needs_redraw();
                }
            });
            cx.start_timer(timer);
        })
        .focusable(true)
    }
    fn close_menu(&mut self, cx: &mut EventContext) {
        if self.shape_menu {
            self.shape_menu = false;
            cx.release();
            cx.needs_redraw();
        }
    }
    fn point(cx: &EventContext) -> (f32, f32) {
        let b = cx.bounds();
        pleasant_ui::local_xy(b.x, b.y, b.w, WIDTH, cx.mouse().cursorx, cx.mouse().cursory)
            .unwrap_or((-1.0, -1.0))
    }
    fn settings(&self, node: Node) -> BandSettings {
        match node {
            Node::Band(i) => self.params.detector_eq[i].settings(),
            Node::HighPass | Node::LowPass => BandSettings {
                shape: if node == Node::HighPass {
                    EqShape::LowCut
                } else {
                    EqShape::HighCut
                },
                order: 2,
                frequency_hz: self
                    .param(Target {
                        node,
                        field: Field::Frequency,
                    })
                    .value() as f64,
                gain_db: 0.0,
                q: std::f64::consts::FRAC_1_SQRT_2,
                ..BandSettings::default()
            },
        }
    }
    fn nodes(&self) -> Vec<Node> {
        let mut nodes: Vec<_> = self
            .params
            .detector_eq
            .iter()
            .enumerate()
            .filter(|(_, band)| band.active.value())
            .map(|(i, _)| Node::Band(i))
            .collect();
        if self.params.hp_freq.value() > 0.0 {
            nodes.push(Node::HighPass);
        }
        if self.params.lp_freq.value() > 0.0 {
            nodes.push(Node::LowPass);
        }
        nodes
    }
    fn hit(&self, p: (f32, f32)) -> Option<Node> {
        let mut nodes = self.nodes();
        // The selected node stays on top when nodes overlap.
        nodes.sort_by_key(|node| Some(*node) == self.selected);
        nodes.into_iter().rev().find(|node| {
            let b = self.settings(*node);
            (frequency_x(b.frequency_hz as f32) - p.0).hypot(
                gain_y(if b.shape.has_gain() {
                    b.gain_db as f32
                } else {
                    0.0
                }) - p.1,
            ) <= 16.0
        })
    }
    fn param(&self, target: Target) -> &FloatParam {
        match target.node {
            Node::HighPass => &self.params.hp_freq,
            Node::LowPass => &self.params.lp_freq,
            Node::Band(i) => match target.field {
                Field::Frequency => &self.params.detector_eq[i].freq,
                Field::Gain => &self.params.detector_eq[i].gain,
                Field::Q => &self.params.detector_eq[i].q,
            },
        }
    }
    fn fields(&self, node: Node) -> Vec<Field> {
        node_fields(node, self.settings(node).shape)
    }
    fn value_hit(&self, p: (f32, f32)) -> Option<Target> {
        let node = self.selected?;
        self.fields(node)
            .into_iter()
            .find(|field| contains(field_rect(*field, &self.fields(node)), p))
            .map(|field| Target { node, field })
    }
    fn begin_node(&mut self, cx: &mut EventContext, node: Node, p: (f32, f32)) {
        self.begin_node_with_settings(cx, node, p, self.settings(node));
    }
    fn begin_node_with_settings(
        &mut self,
        cx: &mut EventContext,
        node: Node,
        p: (f32, f32),
        b: BandSettings,
    ) {
        let fields = node_fields(node, b.shape);
        for &field in &fields {
            cx.emit(RawParamEvent::BeginSetParameter(
                self.param(Target { node, field }).as_ptr(),
            ));
        }
        self.drag = Some(NodeDrag {
            node,
            fields,
            origin: p,
            frequency: b.frequency_hz as f32,
            gain: b.gain_db as f32,
            q: b.q as f32,
        });
        self.display.set_filter_preview_drag(true);
        cx.capture();
        cx.focus();
    }
    fn finish_gesture(&mut self, cx: &mut EventContext) {
        let captured = self.drag.is_some() || self.press.is_some();
        if let Some(drag) = self.drag.take() {
            for field in drag.fields {
                cx.emit(RawParamEvent::EndSetParameter(
                    self.param(Target {
                        node: drag.node,
                        field,
                    })
                    .as_ptr(),
                ));
            }
        }
        if let Some(press) = self.press.take() {
            if press.active {
                cx.emit(RawParamEvent::EndSetParameter(
                    self.param(press.press.target).as_ptr(),
                ));
            }
        }
        if captured {
            self.display.set_filter_preview_drag(false);
            cx.release();
        }
    }
    fn remove(&mut self, cx: &mut EventContext, node: Node) {
        match node {
            Node::Band(i) => set_once(cx, &self.params.detector_eq[i].active, 0.0),
            _ => {
                let p = self.param(Target {
                    node,
                    field: Field::Frequency,
                });
                set_once(cx, p, p.preview_normalized(0.0));
            }
        }
        self.selected = None;
        self.edit = None;
    }
    fn create(&mut self, cx: &mut EventContext, p: (f32, f32)) {
        if let Some(i) = self
            .params
            .detector_eq
            .iter()
            .position(|band| !band.active.value())
        {
            let b = &self.params.detector_eq[i];
            set_once(cx, &b.freq, b.freq.preview_normalized(x_frequency(p.0)));
            set_once(cx, &b.gain, b.gain.preview_normalized(y_gain(p.1)));
            set_once(cx, &b.q, b.q.preview_normalized(1.0));
            set_once(
                cx,
                &b.shape,
                b.shape.preview_normalized(DetectorShape::Bell),
            );
            set_once(cx, &b.enabled, 1.0);
            set_once(cx, &b.active, 1.0);
            self.selected = Some(Node::Band(i));
            // Parameter events are queued. Seed the drag from the values just
            // submitted, not the inactive band's previous parameter values.
            self.begin_node_with_settings(
                cx,
                Node::Band(i),
                p,
                BandSettings {
                    shape: EqShape::Bell,
                    frequency_hz: x_frequency(p.0) as f64,
                    gain_db: y_gain(p.1) as f64,
                    q: 1.0,
                    ..BandSettings::default()
                },
            );
        }
    }
    fn commit_edit(&mut self, cx: &mut EventContext) -> bool {
        if let Some(edit) = self.edit.as_ref() {
            let param = self.param(edit.target);
            let value = param.string_to_normalized_value(&edit.text).or_else(|| {
                pleasant_ui::value_edit::parse_number_with_units(
                    &edit.text,
                    &[("khz", 1000.0), ("hz", 1.0), ("k", 1000.0), ("db", 1.0)],
                )
                .filter(|v| v.is_finite())
                .map(|v| param.preview_normalized(v as f32))
            });
            if let Some(value) = value {
                set_once(cx, param, value);
                self.edit = None;
            } else {
                self.edit.as_mut().unwrap().invalid = true;
                return false;
            }
        }
        true
    }
    fn scroll(&mut self, cx: &mut EventContext, target: Target, delta: f32) {
        self.edit = None;
        let param = self.param(target);
        let step = if cx.modifiers().shift() { 0.001 } else { 0.01 };
        set_once(
            cx,
            param,
            param.unmodulated_normalized_value() + delta * step,
        );
    }
    fn draw_curve(
        d: &mut Draw,
        coeffs: &[BandCoefficients],
        rate: f64,
        color: nih_plug_vizia::vizia::vg::Color,
        width: f32,
    ) {
        let points: [(f32, f32); 513] = std::array::from_fn(|i| {
            let x = GRAPH.0 + GRAPH.2 * i as f32 / 512.0;
            let frequency = response_frequency(x, rate);
            let value: f64 = coeffs.iter().map(|c| c.response_db(frequency, rate)).sum();
            let y = GRAPH.1 + GRAPH.3 * (0.5 - value as f32 / (GAIN_DB * 2.0));
            (x, y)
        });
        d.poly(&points, color, width);
    }
}

impl View for DetectorEqView {
    fn element(&self) -> Option<&'static str> {
        Some("detector-eq")
    }
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let mut d = super::appearance::painter(cx, canvas, WIDTH);
        d.text(
            16.0,
            22.0,
            "DETECTOR EQ",
            super::appearance::MODULE_TITLE_SIZE,
            pleasant_ui::TEXT,
        );
        let listening = self.params.sc_adjust_preview.value();
        super::appearance::button(&mut d, LISTEN_RECT, "", listening, GOLD);
        d.headphones(
            LISTEN_RECT.0 + LISTEN_RECT.2 * 0.5,
            LISTEN_RECT.1 + LISTEN_RECT.3 * 0.5,
            if listening { GOLD } else { MUTED },
        );
        super::appearance::button(
            &mut d,
            SIDECHAIN_RECT,
            "SIDECHAIN TO DETECTOR",
            self.params.use_sidechain.value(),
            pleasant_ui::TEAL,
        );
        for (frequency, label) in [
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
            let x = frequency_x(frequency);
            d.line(x, GRAPH.1, x, GRAPH.1 + GRAPH.3, LINE, 0.7);
            d.text_centered(x, GRAPH.1 + GRAPH.3 + 18.0, label, 13.0, MUTED);
        }
        for gain in [-24.0, -12.0, 0.0, 12.0, 24.0] {
            let y = gain_y(gain);
            d.text_right(GRAPH.0 - 8.0, y + 4.0, &format!("{gain:.0}"), 13.0, MUTED);
            d.line(
                GRAPH.0,
                y,
                GRAPH.0 + GRAPH.2,
                y,
                LINE,
                if gain == 0.0 { 1.0 } else { 0.6 },
            );
        }
        let rate = (self.display.sample_rate.load(Ordering::Relaxed) as f64).max(1.0);
        let mut nodes = self.nodes();
        nodes.sort_by_key(|node| Some(*node) == self.selected);
        let mut total = Vec::new();
        d.c.save();
        d.c.intersect_scissor(
            d.ox + (GRAPH.0 + d.offset_x) * d.s,
            d.oy + GRAPH.1 * d.s,
            GRAPH.2 * d.s,
            GRAPH.3 * d.s,
        );
        for node in &nodes {
            let settings = self.settings(*node);
            if settings.enabled {
                let coeffs = BandCoefficients::prepare(&settings, rate);
                let mut color = node_color(*node);
                color.a = 0.5;
                Self::draw_curve(&mut d, &[coeffs], rate, color, 1.0);
                total.push(coeffs);
            }
        }
        Self::draw_curve(&mut d, &total, rate, GOLD, 1.7);
        d.c.restore();
        for node in nodes {
            let settings = self.settings(node);
            let x = frequency_x(settings.frequency_hz as f32);
            let y = gain_y(if settings.shape.has_gain() {
                settings.gain_db as f32
            } else {
                0.0
            });
            let mut color = node_color(node);
            if !settings.enabled {
                color.a = 0.35;
            }
            d.eq_node(x, y, color, self.selected == Some(node));
        }
        if let Some(node) = self.selected {
            let color = node_color(node);
            match node {
                Node::Band(i) => {
                    let b = &self.params.detector_eq[i];
                    super::appearance::button(
                        &mut d,
                        SHAPE_RECT,
                        &format!("{} ▾", b.shape.value().label().to_uppercase()),
                        true,
                        color,
                    );
                    d.bypass_button(
                        BYPASS_RECT,
                        !b.enabled.value(),
                        color,
                        self.hover.is_some_and(|p| contains(BYPASS_RECT, p)),
                        self.bypass_anim.step(),
                    );
                }
                Node::HighPass => {
                    super::appearance::button(&mut d, SHAPE_RECT, "LOW CUT", true, color)
                }
                Node::LowPass => {
                    super::appearance::button(&mut d, SHAPE_RECT, "HIGH CUT", true, color)
                }
            }
            super::appearance::button(&mut d, REMOVE_RECT, "×", false, MUTED);
            for field in self.fields(node) {
                let target = Target { node, field };
                let r = field_rect(field, &self.fields(node));
                let highlighted = self.hover.is_some_and(|p| contains(r, p))
                    || self.press.as_ref().is_some_and(|p| p.press.rect == r);
                if highlighted {
                    d.outline(r, color);
                }
                d.text(
                    r.0 + 4.0,
                    r.1 + 18.0,
                    match field {
                        Field::Frequency => "FREQ",
                        Field::Gain => "GAIN",
                        Field::Q => "Q",
                    },
                    11.0,
                    MUTED,
                );
                let p = self.param(target);
                let value = match field {
                    Field::Frequency if p.value() >= 1000.0 => {
                        format!("{:.2}k", p.value() / 1000.0)
                    }
                    Field::Frequency => format!("{:.0}", p.value()),
                    Field::Gain => format!("{:+.1} dB", p.value()),
                    Field::Q => format!("{:.2}", p.value()),
                };
                d.text_right(r.0 + r.2 - 4.0, r.1 + 18.0, &value, 13.0, color);
            }
        } else {
            d.text_centered(
                CENTER_X,
                ACTIONS_Y + 20.0,
                "CLICK TO ADD A BAND",
                11.0,
                MUTED,
            );
        }
        if let Some(edit) = &self.edit {
            d.value_edit(edit, node_color(edit.target.node));
        }
        if self.shape_menu {
            if let Some(Node::Band(i)) = self.selected {
                d.rect(
                    SHAPE_MENU.0,
                    SHAPE_MENU.1,
                    SHAPE_MENU.2,
                    SHAPE_MENU.3,
                    PANEL,
                );
                for (row, shape) in DetectorShape::ALL.iter().enumerate() {
                    let y = SHAPE_MENU.1 + row as f32 * MENU_ROW_H;
                    let selected = self.params.detector_eq[i].shape.value() == *shape;
                    if selected || self.hover.is_some_and(|p| menu_shape(p) == Some(*shape)) {
                        d.rect(SHAPE_MENU.0, y, SHAPE_MENU.2, MENU_ROW_H, LINE);
                    }
                    d.text(
                        SHAPE_MENU.0 + 10.0,
                        y + MENU_ROW_H * 0.5 + 13.0 * 0.32,
                        shape.label(),
                        13.0,
                        if selected { GOLD } else { TEXT },
                    );
                }
                d.outline(SHAPE_MENU, LINE);
            }
        }
    }
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        if !super::graph_pages::detector_active(cx) {
            self.close_menu(cx);
            self.finish_gesture(cx);
            self.edit = None;
            return;
        }
        event.map(|e: &WindowEvent, meta| {
            let point = Self::point(cx);
            match e {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    if self.shape_menu {
                        if let (Some(Node::Band(i)), Some(shape)) =
                            (self.selected, menu_shape(point))
                        {
                            let param = &self.params.detector_eq[i].shape;
                            set_once(cx, param, param.preview_normalized(shape));
                        }
                        self.close_menu(cx);
                        meta.consume();
                        return;
                    }
                    if let Some(edit) = self.edit.as_mut() {
                        if contains(edit.rect, point) {
                            edit.handle_mouse_down(point.0);
                            cx.needs_redraw();
                            meta.consume();
                            return;
                        }
                        if !self.commit_edit(cx) {
                            meta.consume();
                            return;
                        }
                    }
                    if contains(LISTEN_RECT, point) || contains(SIDECHAIN_RECT, point) {
                        let param = if contains(LISTEN_RECT, point) {
                            &self.params.sc_adjust_preview
                        } else {
                            &self.params.use_sidechain
                        };
                        set_once(cx, param, if param.value() { 0.0 } else { 1.0 });
                        cx.needs_redraw();
                        meta.consume();
                        return;
                    }
                    if let Some(target) = self.value_hit(point) {
                        self.press = Some(ReadoutPress {
                            press: ValuePress::new(
                                target,
                                field_rect(target.field, &self.fields(target.node)),
                                point,
                            ),
                            start: self.param(target).unmodulated_normalized_value(),
                            active: false,
                        });
                        cx.capture();
                        cx.focus();
                    } else if let Some(node) = self.hit(point) {
                        self.selected = Some(node);
                        self.begin_node(cx, node, point);
                    } else if let Some(node) = self
                        .selected
                        .filter(|_| point.1 >= ACTIONS_Y && point.1 <= ACTIONS_Y + 28.0)
                    {
                        if contains(REMOVE_RECT, point) {
                            self.remove(cx, node);
                        } else if let Node::Band(i) = node {
                            let b = &self.params.detector_eq[i];
                            if contains(SHAPE_RECT, point) {
                                self.shape_menu = true;
                                cx.focus();
                                cx.capture();
                            } else if contains(BYPASS_RECT, point) {
                                self.bypass_anim.trigger_click();
                                set_once(cx, &b.enabled, if b.enabled.value() { 0.0 } else { 1.0 });
                            }
                        }
                    } else if contains(GRAPH, point) {
                        self.create(cx, point);
                        cx.focus();
                    } else {
                        self.selected = None;
                    }
                    cx.needs_redraw();
                    meta.consume();
                }
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some(point);
                    if let Some(drag) = &self.drag {
                        let node = drag.node;
                        if cx.modifiers().shift() && drag.fields.contains(&Field::Q) {
                            let p = self.param(Target {
                                node,
                                field: Field::Q,
                            });
                            let q = drag.q * 2.0_f32.powf((drag.origin.1 - point.1) / 60.0);
                            cx.emit(RawParamEvent::SetParameterNormalized(
                                p.as_ptr(),
                                p.preview_normalized(q),
                            ));
                        } else {
                            let p = self.param(Target {
                                node,
                                field: Field::Frequency,
                            });
                            let frequency =
                                x_frequency(frequency_x(drag.frequency) + point.0 - drag.origin.0);
                            cx.emit(RawParamEvent::SetParameterNormalized(
                                p.as_ptr(),
                                p.preview_normalized(frequency),
                            ));
                            if drag.fields.contains(&Field::Q) && self.settings(node).shape.is_cut()
                            {
                                let param = self.param(Target {
                                    node,
                                    field: Field::Q,
                                });
                                let q = drag.q * 2.0_f32.powf((point.1 - drag.origin.1) / 60.0);
                                cx.emit(RawParamEvent::SetParameterNormalized(
                                    param.as_ptr(),
                                    param.preview_normalized(q),
                                ));
                            }
                            if drag.fields.contains(&Field::Gain)
                                && self.settings(node).shape.has_gain()
                            {
                                let p = self.param(Target {
                                    node,
                                    field: Field::Gain,
                                });
                                let gain = y_gain(gain_y(drag.gain) + point.1 - drag.origin.1);
                                cx.emit(RawParamEvent::SetParameterNormalized(
                                    p.as_ptr(),
                                    p.preview_normalized(gain),
                                ));
                            }
                        }
                        meta.consume();
                    }
                    if let Some(mut press) = self.press.take() {
                        if press.press.update(point.0, point.1) {
                            let p = self.param(press.press.target);
                            if !press.active {
                                cx.emit(RawParamEvent::BeginSetParameter(p.as_ptr()));
                                press.active = true;
                                self.display.set_filter_preview_drag(true);
                            }
                            let delta = pleasant_ui::pointer::readout_drag_delta(
                                point.0 - press.press.origin.0,
                                point.1 - press.press.origin.1,
                                cx.modifiers().shift(),
                            );
                            cx.emit(RawParamEvent::SetParameterNormalized(
                                p.as_ptr(),
                                (press.start + delta).clamp(0.0, 1.0),
                            ));
                        }
                        self.press = Some(press);
                        meta.consume();
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    let clicked = self.press.as_mut().and_then(|p| {
                        p.press
                            .released_as_click(point.0, point.1)
                            .then_some(p.press.target)
                    });
                    self.finish_gesture(cx);
                    if let Some(target) = clicked {
                        let value = self.param(target).normalized_value_to_string(
                            self.param(target).unmodulated_normalized_value(),
                            false,
                        );
                        self.edit = Some(ValueEdit::new(
                            target,
                            field_rect(target.field, &self.fields(target.node)),
                            value,
                        ));
                    }
                    cx.needs_redraw();
                    meta.consume();
                }
                WindowEvent::MouseScroll(_, dy) => {
                    if self.shape_menu {
                        meta.consume();
                        return;
                    }
                    if self.drag.is_none() && self.press.is_none() {
                        let target = self.value_hit(point).or_else(|| {
                            self.hit(point).map(|node| Target {
                                node,
                                field: if matches!(node, Node::Band(_)) {
                                    Field::Q
                                } else {
                                    Field::Frequency
                                },
                            })
                        });
                        if let Some(target) = target {
                            self.scroll(cx, target, *dy);
                            cx.needs_redraw();
                            meta.consume();
                        }
                    }
                }
                WindowEvent::MouseDown(MouseButton::Right) => {
                    if self.shape_menu {
                        self.close_menu(cx);
                        meta.consume();
                        return;
                    }
                    if let Some(node) = self.hit(point) {
                        self.remove(cx, node);
                        cx.needs_redraw();
                        meta.consume();
                    }
                }
                WindowEvent::KeyDown(code, key) => {
                    if self.shape_menu {
                        if *code == Code::Escape {
                            self.close_menu(cx);
                        }
                        meta.consume();
                        return;
                    }
                    if self.edit.is_some() {
                        match code {
                            Code::Enter | Code::NumpadEnter => {
                                self.commit_edit(cx);
                            }
                            Code::Escape => self.edit = None,
                            _ => {
                                let edit = self.edit.as_mut().unwrap();
                                edit.handle_key(cx, *code);
                                if !matches!(key, Some(Key::Character(_)))
                                    && !cx.modifiers().command()
                                {
                                    if let Some(c) =
                                        pleasant_ui::typed_char(*code, cx.modifiers().shift())
                                    {
                                        edit.insert(&c.to_string());
                                    }
                                }
                            }
                        }
                        cx.needs_redraw();
                        meta.consume();
                    } else if matches!(code, Code::Delete | Code::Backspace) {
                        if let Some(node) = self.selected {
                            self.remove(cx, node);
                            cx.needs_redraw();
                            meta.consume();
                        }
                    }
                }
                WindowEvent::CharInput(c) => {
                    if let Some(edit) = self.edit.as_mut() {
                        if !cx.modifiers().command() && !c.is_control() {
                            edit.insert(&c.to_string());
                        }
                        cx.needs_redraw();
                        meta.consume();
                    }
                }
                WindowEvent::MouseLeave => {
                    self.hover = None;
                    cx.needs_redraw();
                }
                WindowEvent::FocusOut => {
                    self.close_menu(cx);
                    self.finish_gesture(cx);
                    self.commit_edit(cx);
                    cx.needs_redraw();
                }
                _ => {}
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropdown_selects_every_shape_and_rejects_outside_clicks() {
        for (row, shape) in DetectorShape::ALL.iter().enumerate() {
            assert_eq!(
                menu_shape((
                    SHAPE_MENU.0 + 12.0,
                    SHAPE_MENU.1 + (row as f32 + 0.5) * MENU_ROW_H
                )),
                Some(*shape)
            );
        }
        assert_eq!(menu_shape((SHAPE_RECT.0, ACTIONS_Y + 10.0)), None);
        assert_eq!(menu_shape((SHAPE_MENU.0 - 1.0, SHAPE_MENU.1)), None);
        assert!(SHAPE_MENU.1 >= GRAPH.1);
    }

    #[test]
    fn graph_axes_round_trip_and_clamp_at_edges() {
        for frequency in [20.0, 100.0, 1000.0, 10_000.0, 20_000.0] {
            assert!((x_frequency(frequency_x(frequency)) / frequency - 1.0).abs() < 0.0001);
        }
        for gain in [-24.0, -12.0, 0.0, 12.0, 24.0] {
            assert!((y_gain(gain_y(gain)) - gain).abs() < 0.001);
        }
        assert_eq!(x_frequency(GRAPH.0 - 100.0), 20.0);
        assert_eq!(x_frequency(GRAPH.0 + GRAPH.2 + 100.0), 20_000.0);
        assert_eq!(y_gain(GRAPH.1 - 100.0), GAIN_DB);
        assert_eq!(y_gain(GRAPH.1 + GRAPH.3 + 100.0), -GAIN_DB);
    }

    #[test]
    fn drag_fields_are_snapshotted_before_automated_shape_changes() {
        for (initial, automated, gain_started) in [
            (EqShape::Bell, EqShape::LowCut, true),
            (EqShape::LowCut, EqShape::Bell, false),
        ] {
            let mut settings = BandSettings {
                shape: initial,
                ..BandSettings::default()
            };
            let drag = NodeDrag {
                node: Node::Band(0),
                fields: node_fields(Node::Band(0), settings.shape),
                origin: (0.0, 0.0),
                frequency: 1000.0,
                gain: 0.0,
                q: 1.0,
            };
            settings.shape = automated;
            assert_eq!(drag.fields.contains(&Field::Gain), gain_started);
            assert_ne!(drag.fields, node_fields(drag.node, settings.shape));
            assert!(drag.fields.contains(&Field::Frequency));
            assert!(drag.fields.contains(&Field::Q));
        }
    }

    #[test]
    fn response_sampling_stays_below_nyquist_at_low_sample_rates() {
        assert_eq!(
            response_frequency(frequency_x(20_000.0), 32_000.0),
            15_680.0
        );
        assert_eq!(
            response_frequency(frequency_x(20_000.0), 48_000.0),
            20_000.0
        );
    }

    #[test]
    fn footer_readouts_fit_the_view_and_clear_the_graph() {
        for fields in [
            vec![Field::Frequency],
            vec![Field::Frequency, Field::Q],
            vec![Field::Frequency, Field::Gain, Field::Q],
        ] {
            for &field in &fields {
                let (x, y, w, h) = field_rect(field, &fields);
                assert!(x >= 0.0 && x + w <= WIDTH);
                assert_eq!(y, ACTIONS_Y);
                assert!(y + h <= HEIGHT);
                assert!(x >= SHAPE_RECT.0 + SHAPE_RECT.2 + 6.0);
                assert!(x + w <= REMOVE_RECT.0 - 6.0);
                assert!(GRAPH.1 + GRAPH.3 + 18.0 < y);
            }
            assert_eq!(
                ACTIONS_Y + super::super::appearance::PLEASANT_GRAPH_Y
                    - 12.0
                    - super::super::graph_pages::CURVE_HEADER_H,
                super::super::appearance::PLEASANT_FOOTER_Y
            );
            assert!(LISTEN_RECT.1 > 22.0 && LISTEN_RECT.1 + LISTEN_RECT.3 < GRAPH.1);
            assert!(SIDECHAIN_RECT.0 + SIDECHAIN_RECT.2 <= WIDTH);
        }
    }
}
