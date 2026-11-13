//! Pleasant-only attack / hold / release editor.

use std::sync::Arc;

use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::widgets::util::ModifiersExt;
use pleasant_ui::envelope::{
    curve_from_mid_norm, ease, ms_to_rate, path, rate_to_ms, EnvelopeHit as StageHit,
    EnvelopeLayout, EnvelopeValues,
};
use pleasant_ui::graph::Viewport;
use pleasant_ui::{pointer::ValuePress, ValueEdit, GOLD, LINE, MUTED, PANEL, TEAL, TEXT};

use super::appearance;
use super::display::UiDisplay;
use super::step_points::snap_time_ms;
use crate::params::{quantize_lookahead_ms, ComposureParams};

mod draw;
mod events;
mod geometry;
mod hit;
#[cfg(test)]
mod tests;

use geometry::*;
use hit::lookahead_line_hit;

struct GraphDrag {
    hit: EnvelopeHit,
    start: EnvelopeValues,
    origin: (f32, f32),
    start_norm: f32,
    lookahead_on_graph: bool,
    axis: pleasant_ui::pointer::AxisLock,
}

struct LabelPress {
    press: ValuePress<EnvelopeHit>,
    start_norm: f32,
    active: bool,
}

pub struct EnvelopeView {
    display: Arc<UiDisplay>,
    lookahead: ParamWidgetBase,
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
            lookahead: ParamWidgetBase::new(cx, params.clone(), |p| &p.lookahead_ms),
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
            EnvelopeHit::Lookahead => &self.lookahead,
            EnvelopeHit::AttackLength => &self.attack,
            EnvelopeHit::HoldLength => &self.hold,
            EnvelopeHit::ReleaseLength => &self.release,
            EnvelopeHit::AttackCurve => &self.attack_curve,
            EnvelopeHit::ReleaseCurve => &self.release_curve,
        }
    }

    fn snapped_plain(hit: EnvelopeHit, plain: f32) -> f32 {
        match hit {
            EnvelopeHit::AttackLength | EnvelopeHit::ReleaseLength => {
                ms_to_rate(snap_time_ms(rate_to_ms(plain)))
            }
            EnvelopeHit::HoldLength => snap_time_ms(plain),
            EnvelopeHit::Lookahead => quantize_lookahead_ms(plain),
            _ => plain,
        }
    }

    fn set_time_norm(&self, cx: &mut EventContext, hit: EnvelopeHit, norm: f32, fine: bool) {
        let param = self.param(hit);
        let plain = param.preview_plain(norm);
        self.set_plain(
            cx,
            hit,
            if fine {
                plain
            } else {
                Self::snapped_plain(hit, plain)
            },
        );
    }

    fn time_drag_delta(hit: EnvelopeHit, origin: (f32, f32), point: (f32, f32), fine: bool) -> f32 {
        let delta =
            pleasant_ui::pointer::readout_drag_delta(point.0 - origin.0, point.1 - origin.1, fine);
        // Attack/release store rates: a larger time requires a smaller rate.
        match hit {
            EnvelopeHit::AttackLength | EnvelopeHit::ReleaseLength => -delta,
            _ => delta,
        }
    }

    fn set_plain(&self, cx: &mut EventContext, hit: EnvelopeHit, plain: f32) {
        let param = self.param(hit);
        param.set_normalized_value(cx, param.preview_normalized(plain));
        self.display.set_param_readout(
            param.normalized_value_to_string(param.preview_normalized(plain), true),
        );
    }

    fn begin_graph(&mut self, cx: &mut EventContext, hit: EnvelopeHit) {
        cx.capture();
        cx.focus();
        self.param(hit).begin_set_parameter(cx);
        let lookahead_on_graph = hit == EnvelopeHit::Lookahead && Self::point(cx).1 >= GRAPH.1;
        let start_norm = if hit == EnvelopeHit::Lookahead && !lookahead_on_graph {
            let ms = quantize_lookahead_ms(lookahead_bar_ms(Self::point(cx).0));
            self.set_plain(cx, hit, ms);
            self.lookahead.preview_normalized(ms)
        } else {
            self.param(hit).unmodulated_normalized_value()
        };
        self.drag = Some(GraphDrag {
            hit,
            start: self.values(),
            origin: Self::point(cx),
            start_norm,
            lookahead_on_graph,
            axis: pleasant_ui::pointer::AxisLock::default(),
        });
    }

    fn finish_gesture(&mut self, cx: &mut EventContext) {
        let captured = self.drag.is_some() || self.press.is_some();
        if let Some(drag) = self.drag.take() {
            let param = self.param(drag.hit);
            param.end_set_parameter(cx);
        }
        if let Some(press) = self.press.take() {
            if press.active {
                let param = self.param(press.press.target);
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
        self.draw_view(cx, canvas);
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        self.handle_event(cx, event);
    }
}
