//! Detector equalizer, using the same response model and node artwork as Damian.
use std::sync::{atomic::Ordering, Arc};

use nih_plug::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::{util::ModifiersExt, RawParamEvent};
use pleasant_eq::{BandCoefficients, BandSettings, EqShape};
use pleasant_ui::theme::EQ_COLORS as COLORS;
use pleasant_ui::{pointer::ValuePress, Draw, ValueEdit, GOLD, LINE, MUTED};

use super::display::UiDisplay;
use crate::{detector_eq::DetectorShape, params::ComposureParams};

const SIZE: f32 = 290.0;
const GRAPH: (f32, f32, f32, f32) = (10.0, 20.0, 270.0, 194.0);
const GAIN_DB: f32 = 24.0;
const FIELDS_Y: f32 = 252.0;

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
fn field_rect(field: Field) -> (f32, f32, f32, f32) {
    let index = match field {
        Field::Frequency => 0,
        Field::Gain => 1,
        Field::Q => 2,
    };
    (10.0 + index as f32 * 92.0, FIELDS_Y, 86.0, 30.0)
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
        }
        .build(cx, |_| {})
    }
    fn point(cx: &EventContext) -> (f32, f32) {
        let b = cx.bounds();
        pleasant_ui::local_xy(b.x, b.y, b.w, SIZE, cx.mouse().cursorx, cx.mouse().cursory)
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
            .find(|field| contains(field_rect(*field), p))
            .map(|field| Target { node, field })
    }
    fn begin_node(&mut self, cx: &mut EventContext, node: Node, p: (f32, f32)) {
        let b = self.settings(node);
        let fields = self.fields(node);
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
        let points: [(f32, f32); 271] = std::array::from_fn(|i| {
            let x = GRAPH.0 + i as f32;
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
        let mut d = super::appearance::painter(cx, canvas, SIZE);
        d.text(10.0, 12.0, "DETECTOR EQ", 11.0, MUTED);
        for (frequency, label) in [(100.0, "100"), (1000.0, "1k"), (10000.0, "10k")] {
            let x = frequency_x(frequency);
            d.line(x, GRAPH.1, x, GRAPH.1 + GRAPH.3, LINE, 0.7);
            d.text_centered(x, GRAPH.1 + GRAPH.3 + 12.0, label, 9.0, MUTED);
        }
        for gain in [-24.0, -12.0, 0.0, 12.0, 24.0] {
            let y = gain_y(gain);
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
                    d.button(
                        (10.0, 230.0, 106.0, 20.0),
                        &b.shape.value().label().to_uppercase(),
                        true,
                        color,
                    );
                    d.button(
                        (122.0, 230.0, 106.0, 20.0),
                        "BYPASS",
                        !b.enabled.value(),
                        color,
                    );
                }
                Node::HighPass => d.text(10.0, 244.0, "LOW CUT", 11.0, color),
                Node::LowPass => d.text(10.0, 244.0, "HIGH CUT", 11.0, color),
            }
            d.button((244.0, 230.0, 36.0, 20.0), "×", false, MUTED);
            for field in self.fields(node) {
                let target = Target { node, field };
                let r = field_rect(field);
                let highlighted = self.hover.is_some_and(|p| contains(r, p))
                    || self.press.as_ref().is_some_and(|p| p.press.rect == r);
                d.outline(r, if highlighted { color } else { LINE });
                d.text(
                    r.0 + 4.0,
                    r.1 + 11.0,
                    match field {
                        Field::Frequency => "FREQ",
                        Field::Gain => "GAIN",
                        Field::Q => "Q",
                    },
                    8.0,
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
                d.text_right(r.0 + r.2 - 4.0, r.1 + 26.0, &value, 11.0, color);
            }
        } else {
            d.text_centered(SIZE * 0.5, 250.0, "CLICK TO ADD A BAND", 10.0, MUTED);
        }
        if let Some(edit) = &self.edit {
            d.value_edit(edit, node_color(edit.target.node));
        }
    }
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        if !super::graph_pages::detector_active(cx) {
            self.finish_gesture(cx);
            self.edit = None;
            return;
        }
        event.map(|e: &WindowEvent, meta| {
            let point = Self::point(cx);
            match e {
                WindowEvent::MouseDown(MouseButton::Left) => {
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
                    if let Some(target) = self.value_hit(point) {
                        self.press = Some(ReadoutPress {
                            press: ValuePress::new(target, field_rect(target.field), point),
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
                        .filter(|_| point.1 >= 230.0 && point.1 <= 250.0)
                    {
                        if contains((244.0, 230.0, 36.0, 20.0), point) {
                            self.remove(cx, node);
                        } else if let Node::Band(i) = node {
                            let b = &self.params.detector_eq[i];
                            if contains((10.0, 230.0, 106.0, 20.0), point) {
                                let at = DetectorShape::ALL
                                    .iter()
                                    .position(|s| *s == b.shape.value())
                                    .unwrap_or(0);
                                let next = DetectorShape::ALL[(at + 1) % DetectorShape::ALL.len()];
                                set_once(cx, &b.shape, b.shape.preview_normalized(next));
                            } else if contains((122.0, 230.0, 106.0, 20.0), point) {
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
                        self.edit = Some(ValueEdit::new(target, field_rect(target.field), value));
                    }
                    cx.needs_redraw();
                    meta.consume();
                }
                WindowEvent::MouseScroll(_, dy) => {
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
                    if let Some(node) = self.hit(point) {
                        self.remove(cx, node);
                        cx.needs_redraw();
                        meta.consume();
                    }
                }
                WindowEvent::KeyDown(code, key) => {
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
    fn footer_readouts_fit_the_square_and_clear_the_graph() {
        for field in [Field::Frequency, Field::Gain, Field::Q] {
            let (x, y, w, h) = field_rect(field);
            assert!(x >= 0.0 && x + w <= SIZE);
            assert!(y > GRAPH.1 + GRAPH.3 && y + h <= SIZE);
        }
    }
}
