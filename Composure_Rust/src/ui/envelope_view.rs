//! Pleasant-only attack / hold / release editor.

use std::sync::Arc;

use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::widgets::util::ModifiersExt;
use pleasant_ui::envelope::{
    drag_curve, drag_length, hit_test, layout, ms_to_rate, path, rate_to_ms, EnvelopeHit,
    EnvelopeLayout, EnvelopeValues,
};
use pleasant_ui::graph::Viewport;
use pleasant_ui::handles::TagPointer;
use pleasant_ui::{pointer::ValuePress, ValueEdit, GOLD, LINE, MUTED, PANEL, TEAL, TEXT};

use super::appearance;
use super::display::UiDisplay;
use super::step_points::{self, StepSet};
use crate::params::ComposureParams;

const WIDTH: f32 = appearance::ENVELOPE_GRAPH_W;
const HEIGHT: f32 = appearance::ENVELOPE_GRAPH_H;
const GRAPH: (f32, f32, f32, f32) = (16.0, 12.0, WIDTH - 32.0, HEIGHT - 64.0);
const LABEL_H: f32 = 36.0;

struct GraphDrag {
    hit: EnvelopeHit,
    start: EnvelopeValues,
}

struct LabelPress {
    press: ValuePress<EnvelopeHit>,
    start_norm: f32,
    active: bool,
}

pub struct EnvelopeView {
    display: Arc<UiDisplay>,
    attack: ParamWidgetBase,
    hold: ParamWidgetBase,
    release: ParamWidgetBase,
    attack_curve: ParamWidgetBase,
    release_curve: ParamWidgetBase,
    drag: Option<GraphDrag>,
    press: Option<LabelPress>,
    edit: Option<ValueEdit<EnvelopeHit>>,
    hover: Option<(f32, f32)>,
}

impl EnvelopeView {
    pub fn new<L>(cx: &mut Context, params: L, display: Arc<UiDisplay>) -> Handle<'_, EnvelopeView>
    where
        L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    {
        EnvelopeView {
            display,
            attack: ParamWidgetBase::new(cx, params.clone(), |p| &p.attack),
            hold: ParamWidgetBase::new(cx, params.clone(), |p| &p.hold_ms),
            release: ParamWidgetBase::new(cx, params.clone(), |p| &p.release),
            attack_curve: ParamWidgetBase::new(cx, params.clone(), |p| &p.attack_curve),
            release_curve: ParamWidgetBase::new(cx, params, |p| &p.release_curve),
            drag: None,
            press: None,
            edit: None,
            hover: None,
        }
        .build(cx, |_| {})
    }

    fn point(cx: &EventContext) -> (f32, f32) {
        let b = cx.bounds();
        pleasant_ui::local_xy(b.x, b.y, b.w, WIDTH, cx.mouse().cursorx, cx.mouse().cursory)
            .unwrap_or((-1.0, -1.0))
    }

    fn values(&self) -> EnvelopeValues {
        EnvelopeValues {
            attack_ms: rate_to_ms(self.attack.unmodulated_plain_value()),
            hold_ms: self.hold.unmodulated_plain_value(),
            release_ms: rate_to_ms(self.release.unmodulated_plain_value()),
            attack_curve: self.attack_curve.unmodulated_plain_value(),
            release_curve: self.release_curve.unmodulated_plain_value(),
        }
    }

    fn laid(&self) -> EnvelopeLayout {
        layout(Viewport::from_tuple(GRAPH), self.values())
    }

    fn param(&self, hit: EnvelopeHit) -> &ParamWidgetBase {
        match hit {
            EnvelopeHit::AttackLength => &self.attack,
            EnvelopeHit::HoldLength => &self.hold,
            EnvelopeHit::ReleaseLength => &self.release,
            EnvelopeHit::AttackCurve => &self.attack_curve,
            EnvelopeHit::ReleaseCurve => &self.release_curve,
        }
    }

    fn step_set(hit: EnvelopeHit) -> StepSet {
        match hit {
            EnvelopeHit::AttackLength => StepSet::Attack,
            EnvelopeHit::ReleaseLength => StepSet::Release,
            _ => StepSet::None,
        }
    }

    fn label_rect(hit: EnvelopeHit) -> Option<(f32, f32, f32, f32)> {
        let index = match hit {
            EnvelopeHit::AttackLength => 0,
            EnvelopeHit::HoldLength => 1,
            EnvelopeHit::ReleaseLength => 2,
            _ => return None,
        };
        let width = (WIDTH - 20.0) / 3.0;
        Some((
            10.0 + width * index as f32,
            HEIGHT - LABEL_H - 8.0,
            width,
            LABEL_H,
        ))
    }

    fn label_hit(p: (f32, f32)) -> Option<EnvelopeHit> {
        [
            EnvelopeHit::AttackLength,
            EnvelopeHit::HoldLength,
            EnvelopeHit::ReleaseLength,
        ]
        .into_iter()
        .find(|hit| {
            Self::label_rect(*hit).is_some_and(|rect| {
                p.0 >= rect.0 && p.0 <= rect.0 + rect.2 && p.1 >= rect.1 && p.1 <= rect.1 + rect.3
            })
        })
    }

    fn set_plain(&self, cx: &mut EventContext, hit: EnvelopeHit, plain: f32) {
        let param = self.param(hit);
        param.set_normalized_value(cx, param.preview_normalized(plain));
        self.display.set_param_readout(
            param.normalized_value_to_string(param.preview_normalized(plain), true),
        );
    }

    fn apply_length(&self, cx: &mut EventContext, hit: EnvelopeHit, ms: f32) {
        match hit {
            EnvelopeHit::AttackLength | EnvelopeHit::ReleaseLength => {
                self.set_plain(cx, hit, ms_to_rate(ms));
            }
            EnvelopeHit::HoldLength => self.set_plain(cx, hit, ms),
            _ => {}
        }
    }

    fn begin_graph(&mut self, cx: &mut EventContext, hit: EnvelopeHit) {
        cx.capture();
        cx.focus();
        self.param(hit).begin_set_parameter(cx);
        self.drag = Some(GraphDrag {
            hit,
            start: self.values(),
        });
    }

    fn finish_gesture(&mut self, cx: &mut EventContext) {
        let captured = self.drag.is_some() || self.press.is_some();
        if let Some(drag) = self.drag.take() {
            let param = self.param(drag.hit);
            if Self::step_set(drag.hit) != StepSet::None {
                step_points::snap_param(cx, param, Self::step_set(drag.hit));
            }
            param.end_set_parameter(cx);
        }
        if let Some(press) = self.press.take() {
            if press.active {
                let param = self.param(press.press.target);
                if Self::step_set(press.press.target) != StepSet::None {
                    step_points::snap_param(cx, param, Self::step_set(press.press.target));
                }
                param.end_set_parameter(cx);
            }
        }
        if captured {
            self.display.clear_param_readout();
            cx.release();
        }
    }

    fn commit_edit(&mut self, cx: &mut EventContext) -> bool {
        let Some(edit) = self.edit.as_ref() else {
            return true;
        };
        let param = self.param(edit.target);
        if let Some(norm) = param
            .string_to_normalized_value(&edit.text)
            .filter(|value| value.is_finite())
        {
            param.begin_set_parameter(cx);
            param.set_normalized_value(cx, norm.clamp(0.0, 1.0));
            param.end_set_parameter(cx);
            self.edit = None;
            true
        } else {
            if let Some(edit) = self.edit.as_mut() {
                edit.invalid = true;
            }
            false
        }
    }

    fn reset(&self, cx: &mut EventContext, hit: EnvelopeHit) {
        let param = self.param(hit);
        param.begin_set_parameter(cx);
        param.set_normalized_value(cx, param.default_normalized_value());
        param.end_set_parameter(cx);
    }
}

impl View for EnvelopeView {
    fn element(&self) -> Option<&'static str> {
        Some("envelope-graph")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let mut d = appearance::painter(cx, canvas, WIDTH);
        let values = self.values();
        let laid = layout(Viewport::from_tuple(GRAPH), values);
        let points = path(&laid, values);
        let hover =
            pleasant_ui::idle_hover(self.hover, self.drag.is_some() || self.press.is_some());
        let active = self.drag.as_ref().map(|drag| drag.hit).or_else(|| {
            hover.and_then(|(x, y)| hit_test(&laid, x, y).or_else(|| Self::label_hit((x, y))))
        });

        d.rect(GRAPH.0, GRAPH.1, GRAPH.2, GRAPH.3, PANEL);
        d.outline(GRAPH, LINE);
        d.line(
            laid.start.0,
            laid.rest_y,
            laid.release_end.0.max(laid.start.0 + 1.0),
            laid.rest_y,
            LINE,
            1.0,
        );

        if points.len() >= 2 {
            let mut fill = points.clone();
            fill.push((laid.release_end.0, laid.hold_y));
            fill.push((laid.start.0, laid.hold_y));
            let mut well = TEAL;
            well.a = 0.14;
            d.fill_poly(&fill, well);
            d.poly(&points, TEAL, 2.0);
        }

        for (hit, point, pointer) in [
            (EnvelopeHit::AttackLength, laid.attack_end, TagPointer::Down),
            (EnvelopeHit::HoldLength, laid.hold_end, TagPointer::Down),
            (
                EnvelopeHit::ReleaseLength,
                laid.release_end,
                TagPointer::None,
            ),
        ] {
            let color = if active == Some(hit) { GOLD } else { TEAL };
            d.tag_handle(point.0, point.1, pointer, color);
        }
        for (hit, point) in [
            (EnvelopeHit::AttackCurve, laid.attack_curve),
            (EnvelopeHit::ReleaseCurve, laid.release_curve),
        ] {
            d.eq_node(
                point.0,
                point.1,
                if active == Some(hit) { GOLD } else { TEAL },
                active == Some(hit),
            );
        }

        for hit in [
            EnvelopeHit::AttackLength,
            EnvelopeHit::HoldLength,
            EnvelopeHit::ReleaseLength,
        ] {
            let Some(rect) = Self::label_rect(hit) else {
                continue;
            };
            let param = self.param(hit);
            let name = match hit {
                EnvelopeHit::AttackLength => "ATT",
                EnvelopeHit::HoldLength => "HOLD",
                EnvelopeHit::ReleaseLength => "REL",
                _ => "",
            };
            let value =
                param.normalized_value_to_string(param.unmodulated_normalized_value(), true);
            let accent = if active == Some(hit) { GOLD } else { TEXT };
            d.text(rect.0 + 4.0, rect.1 + 14.0, name, 11.0, MUTED);
            if self.edit.as_ref().is_some_and(|edit| edit.target == hit) {
                continue;
            }
            d.text(rect.0 + 4.0, rect.1 + 30.0, &value, 13.0, accent);
            if hover.is_some_and(|p| Self::label_hit(p) == Some(hit)) {
                d.value_underline((rect.0, rect.1 + 14.0, rect.2 - 8.0, 18.0), GOLD);
            }
        }
        if let Some(edit) = &self.edit {
            d.value_edit(edit, GOLD);
        }
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| {
            let point = Self::point(cx);
            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    if let Some(edit) = self.edit.as_mut() {
                        if point.0 >= edit.rect.0
                            && point.0 <= edit.rect.0 + edit.rect.2
                            && point.1 >= edit.rect.1
                            && point.1 <= edit.rect.1 + edit.rect.3
                        {
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
                    let laid = self.laid();
                    if let Some(hit) = hit_test(&laid, point.0, point.1) {
                        if cx.modifiers().command() {
                            self.reset(cx, hit);
                        } else {
                            self.begin_graph(cx, hit);
                        }
                        cx.needs_redraw();
                        meta.consume();
                    } else if let Some(hit) = Self::label_hit(point) {
                        if cx.modifiers().command() {
                            self.reset(cx, hit);
                        } else {
                            self.press = Some(LabelPress {
                                press: ValuePress::new(hit, Self::label_rect(hit).unwrap(), point),
                                start_norm: self.param(hit).unmodulated_normalized_value(),
                                active: false,
                            });
                            cx.capture();
                            cx.focus();
                        }
                        cx.needs_redraw();
                        meta.consume();
                    }
                }
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some(point);
                    if let Some(drag) = &self.drag {
                        let values = drag.start;
                        let laid = layout(Viewport::from_tuple(GRAPH), values);
                        match drag.hit {
                            EnvelopeHit::AttackCurve | EnvelopeHit::ReleaseCurve => {
                                let mut curve = drag_curve(drag.hit, &laid, point.1);
                                if cx.modifiers().shift() {
                                    let current = match drag.hit {
                                        EnvelopeHit::AttackCurve => values.attack_curve,
                                        _ => values.release_curve,
                                    };
                                    curve = current + (curve - current) * 0.2;
                                }
                                self.set_plain(cx, drag.hit, curve);
                            }
                            hit => {
                                let mut ms =
                                    drag_length(hit, values, Viewport::from_tuple(GRAPH), point.0);
                                if cx.modifiers().shift() {
                                    let current = match hit {
                                        EnvelopeHit::AttackLength => values.attack_ms,
                                        EnvelopeHit::HoldLength => values.hold_ms,
                                        _ => values.release_ms,
                                    };
                                    ms = current + (ms - current) * 0.2;
                                }
                                self.apply_length(cx, hit, ms);
                            }
                        }
                        meta.consume();
                    }
                    if let Some(mut press) = self.press.take() {
                        if press.press.update(point.0, point.1) {
                            let param = self.param(press.press.target);
                            if !press.active {
                                param.begin_set_parameter(cx);
                                press.active = true;
                            }
                            let delta = pleasant_ui::pointer::readout_drag_delta(
                                point.0 - press.press.origin.0,
                                point.1 - press.press.origin.1,
                                cx.modifiers().shift(),
                            );
                            param.set_normalized_value(
                                cx,
                                (press.start_norm + delta).clamp(0.0, 1.0),
                            );
                        }
                        self.press = Some(press);
                        meta.consume();
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    let clicked = self.press.as_mut().and_then(|press| {
                        press
                            .press
                            .released_as_click(point.0, point.1)
                            .then_some(press.press.target)
                    });
                    self.finish_gesture(cx);
                    if let Some(hit) = clicked {
                        if let Some(rect) = Self::label_rect(hit) {
                            let param = self.param(hit);
                            self.edit = Some(ValueEdit::new(
                                hit,
                                (rect.0, rect.1 + 14.0, rect.2 - 6.0, 20.0),
                                param.normalized_value_to_string(
                                    param.unmodulated_normalized_value(),
                                    false,
                                ),
                            ));
                        }
                    }
                    cx.needs_redraw();
                    meta.consume();
                }
                WindowEvent::MouseScroll(_, dy) => {
                    if self.drag.is_none() && self.press.is_none() {
                        let laid = self.laid();
                        if let Some(hit) =
                            hit_test(&laid, point.0, point.1).or_else(|| Self::label_hit(point))
                        {
                            let param = self.param(hit);
                            let step = if cx.modifiers().shift() { 0.004 } else { 0.02 };
                            param.begin_set_parameter(cx);
                            param.set_normalized_value(
                                cx,
                                (param.unmodulated_normalized_value() + *dy * step).clamp(0.0, 1.0),
                            );
                            if Self::step_set(hit) != StepSet::None {
                                step_points::snap_param(cx, param, Self::step_set(hit));
                            }
                            param.end_set_parameter(cx);
                            cx.needs_redraw();
                            meta.consume();
                        }
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
    fn labels_sit_under_the_graph_without_overlap() {
        let attack = EnvelopeView::label_rect(EnvelopeHit::AttackLength).unwrap();
        let hold = EnvelopeView::label_rect(EnvelopeHit::HoldLength).unwrap();
        let release = EnvelopeView::label_rect(EnvelopeHit::ReleaseLength).unwrap();
        assert!(attack.1 >= GRAPH.1 + GRAPH.3);
        assert!(attack.0 + attack.2 <= hold.0 + 0.01);
        assert!(hold.0 + hold.2 <= release.0 + 0.01);
        assert!(release.0 + release.2 <= WIDTH);
        assert_eq!(
            EnvelopeView::label_hit((attack.0 + 8.0, attack.1 + 8.0)),
            Some(EnvelopeHit::AttackLength)
        );
        assert_eq!(EnvelopeView::label_hit((0.0, 0.0)), None);
    }
}
