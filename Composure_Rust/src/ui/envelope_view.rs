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

const WIDTH: f32 = appearance::ENVELOPE_GRAPH_W;
const HEIGHT: f32 = appearance::ENVELOPE_GRAPH_H;
const LOOK_BAR: (f32, f32, f32, f32) = (12.0, 8.0, WIDTH - 24.0, 30.0);
const GRAPH: (f32, f32, f32, f32) = (12.0, 50.0, WIDTH - 24.0, HEIGHT - 96.0);
const LABEL_H: f32 = 36.0;

// After normalization this permits a 1/14 share, half the previous 1/7 minimum.
const MIN_STAGE_WEIGHT: f32 = 1.0 / 12.0;

fn dashed_line(
    d: &mut pleasant_ui::Draw<'_>,
    x: f32,
    top: f32,
    bottom: f32,
    color: nih_plug_vizia::vizia::vg::Color,
) {
    let mut y = top;
    while y < bottom {
        d.line(x, y, x, (y + 4.0).min(bottom), color, 1.0);
        y += 8.0;
    }
}

// Lookahead belongs to this editor; shared envelope geometry stays attack/hold/release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnvelopeHit {
    Lookahead,
    AttackLength,
    HoldLength,
    ReleaseLength,
    AttackCurve,
    ReleaseCurve,
}

// Duration shares with a smaller touchability floor, then normalize.
// Even a zero hold stays visible and can be dragged open.
fn layout(viewport: Viewport, values: EnvelopeValues) -> EnvelopeLayout {
    let values = values.clamped();
    let total = values.attack_ms + values.hold_ms + values.release_ms;
    let weights = [values.attack_ms, values.hold_ms, values.release_ms]
        .map(|ms| (ms / total).max(MIN_STAGE_WEIGHT));
    let sum = weights.iter().sum::<f32>();
    let rest_y = viewport.y + viewport.height - 24.0;
    let hold_y = viewport.y + 24.0;
    let start = (viewport.x, rest_y);
    let attack_end = (viewport.x + viewport.width * weights[0] / sum, hold_y);
    let hold_end = (attack_end.0 + viewport.width * weights[1] / sum, hold_y);
    let release_end = (viewport.x + viewport.width, rest_y);
    EnvelopeLayout {
        viewport,
        rest_y,
        hold_y,
        start,
        attack_end,
        hold_end,
        release_end,
        attack_curve: (
            (start.0 + attack_end.0) * 0.5,
            rest_y + (hold_y - rest_y) * ease(0.5, values.attack_curve),
        ),
        release_curve: (
            (hold_end.0 + release_end.0) * 0.5,
            hold_y + (rest_y - hold_y) * ease(0.5, values.release_curve),
        ),
    }
}

fn lookahead_marker_x(ms: f32, laid: &EnvelopeLayout, values: EnvelopeValues) -> f32 {
    let values = values.clamped();
    let time = ms.max(0.0);
    if time <= values.attack_ms {
        return laid.start.0 + time / values.attack_ms * (laid.attack_end.0 - laid.start.0);
    }
    let time = time - values.attack_ms;
    // The zero-time hold is an editing affordance, not a range of lookahead times.
    // Spread release time across that empty space so dragging does not stick at attack.
    if values.hold_ms == 0.0 {
        return laid.attack_end.0
            + (time / values.release_ms).clamp(0.0, 1.0)
                * (laid.release_end.0 - laid.attack_end.0);
    }
    if values.hold_ms > 0.0 && time <= values.hold_ms {
        return laid.attack_end.0 + time / values.hold_ms * (laid.hold_end.0 - laid.attack_end.0);
    }
    let time = time - values.hold_ms;
    laid.hold_end.0
        + (time / values.release_ms).clamp(0.0, 1.0) * (laid.release_end.0 - laid.hold_end.0)
}

// Inverse of the graph's piecewise time scale; the slider keeps its own cubic scale.
fn lookahead_graph_ms(x: f32, laid: &EnvelopeLayout, values: EnvelopeValues) -> f32 {
    let values = values.clamped();
    let x = x.clamp(laid.start.0, laid.release_end.0);
    let ms = if x <= laid.attack_end.0 {
        (x - laid.start.0) / (laid.attack_end.0 - laid.start.0) * values.attack_ms
    } else if values.hold_ms == 0.0 {
        values.attack_ms
            + (x - laid.attack_end.0) / (laid.release_end.0 - laid.attack_end.0) * values.release_ms
    } else if x <= laid.hold_end.0 {
        values.attack_ms
            + (x - laid.attack_end.0) / (laid.hold_end.0 - laid.attack_end.0) * values.hold_ms
    } else {
        values.attack_ms
            + values.hold_ms
            + (x - laid.hold_end.0) / (laid.release_end.0 - laid.hold_end.0) * values.release_ms
    };
    ms.clamp(0.0, 2000.0)
}

fn lookahead_drag_ms(
    x: f32,
    laid: &EnvelopeLayout,
    values: EnvelopeValues,
    bypass_snap: bool,
) -> f32 {
    let ms = if !bypass_snap && (x - laid.attack_end.0).abs() <= 0.75 {
        values.attack_ms
    } else {
        lookahead_graph_ms(x, laid, values)
    };
    quantize_lookahead_ms(ms)
}

fn lookahead_line_hit(laid: &EnvelopeLayout, point: (f32, f32), x: f32) -> bool {
    point.1 >= laid.viewport.y
        && point.1 <= laid.viewport.y + laid.viewport.height
        && point.0 >= laid.start.0
        && point.0 <= laid.release_end.0
        && (point.0 - x).abs() <= 6.0
}

fn lookahead_bar_x(ms: f32) -> f32 {
    LOOK_BAR.0 + LOOK_BAR.2 * (ms.clamp(0.0, 2000.0) / 2000.0).cbrt()
}

fn lookahead_bar_ms(x: f32) -> f32 {
    2000.0 * ((x - LOOK_BAR.0) / LOOK_BAR.2).clamp(0.0, 1.0).powi(3)
}

fn curve_drag(hit: StageHit, laid: &EnvelopeLayout, y: f32) -> f32 {
    let (start, end) = match hit {
        StageHit::AttackCurve => (laid.rest_y, laid.hold_y),
        _ => (laid.hold_y, laid.rest_y),
    };
    curve_from_mid_norm(((y - start) / (end - start)).clamp(0.02, 0.98))
}

fn stage_time_drag(hit: EnvelopeHit, values: EnvelopeValues, dx: f32) -> f32 {
    let (start_ms, offset) = match hit {
        EnvelopeHit::AttackLength => (values.attack_ms, 0.0),
        EnvelopeHit::ReleaseLength => (values.release_ms, 0.0),
        _ => (values.hold_ms, 1.0),
    };
    let ms = ((start_ms + offset) * 1.03_f32.powf(dx) - offset).max(0.0);
    match hit {
        EnvelopeHit::AttackLength | EnvelopeHit::ReleaseLength => ms_to_rate(ms),
        _ => ms,
    }
}

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

    fn zone_rect(laid: &EnvelopeLayout, hit: EnvelopeHit) -> (f32, f32, f32, f32) {
        if hit == EnvelopeHit::Lookahead {
            return LOOK_BAR;
        }
        let (left, right) = match hit {
            EnvelopeHit::Lookahead => unreachable!(),
            EnvelopeHit::AttackLength | EnvelopeHit::AttackCurve => {
                (laid.start.0, laid.attack_end.0)
            }
            EnvelopeHit::HoldLength => (laid.attack_end.0, laid.hold_end.0),
            EnvelopeHit::ReleaseLength | EnvelopeHit::ReleaseCurve => {
                (laid.hold_end.0, laid.viewport.x + laid.viewport.width)
            }
        };
        (left, laid.viewport.y, right - left, laid.viewport.height)
    }

    fn graph_hit(laid: &EnvelopeLayout, point: (f32, f32)) -> Option<EnvelopeHit> {
        if point.0 >= LOOK_BAR.0
            && point.0 <= LOOK_BAR.0 + LOOK_BAR.2
            && point.1 >= LOOK_BAR.1
            && point.1 <= LOOK_BAR.1 + LOOK_BAR.3
        {
            return Some(EnvelopeHit::Lookahead);
        }
        let rect = laid.graph_rect();
        if point.0 < rect.0
            || point.0 > rect.0 + rect.2
            || point.1 < rect.1
            || point.1 > rect.1 + rect.3
        {
            return None;
        }
        // Curve handles take priority over the stage's drag zone.
        let curve = [
            (EnvelopeHit::AttackCurve, laid.attack_curve),
            (EnvelopeHit::ReleaseCurve, laid.release_curve),
        ]
        .into_iter()
        .filter_map(|(hit, center)| {
            let distance = (center.0 - point.0).hypot(center.1 - point.1);
            (distance <= 14.0).then_some((hit, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((hit, _)) = curve {
            return Some(hit);
        }
        Some(if point.0 < laid.attack_end.0 {
            EnvelopeHit::AttackLength
        } else if point.0 < laid.hold_end.0 {
            EnvelopeHit::HoldLength
        } else {
            EnvelopeHit::ReleaseLength
        })
    }

    fn interactive_hit(&self, laid: &EnvelopeLayout, point: (f32, f32)) -> Option<EnvelopeHit> {
        let hit = Self::graph_hit(laid, point);
        // Keep the explicit curve nodes usable when the line crosses one.
        if matches!(
            hit,
            Some(EnvelopeHit::AttackCurve | EnvelopeHit::ReleaseCurve)
        ) {
            return hit;
        }
        let x = lookahead_marker_x(
            self.lookahead.unmodulated_plain_value(),
            laid,
            self.values(),
        );
        if lookahead_line_hit(laid, point, x) {
            Some(EnvelopeHit::Lookahead)
        } else {
            hit
        }
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
        let mut d = appearance::painter(cx, canvas, WIDTH);
        let values = self.values();
        let laid = layout(Viewport::from_tuple(GRAPH), values);
        let points = path(&laid, values);
        let hover =
            pleasant_ui::idle_hover(self.hover, self.drag.is_some() || self.press.is_some());
        let active = self
            .drag
            .as_ref()
            .map(|drag| drag.hit)
            .or_else(|| self.press.as_ref().map(|press| press.press.target))
            .or_else(|| self.edit.as_ref().map(|edit| edit.target))
            .or_else(|| {
                hover.and_then(|(x, y)| {
                    self.interactive_hit(&laid, (x, y))
                        .or_else(|| Self::label_hit((x, y)))
                })
            });

        let lookahead_ms = self.lookahead.unmodulated_plain_value();
        let lookahead_x = lookahead_marker_x(lookahead_ms, &laid, values);
        let total = values.attack_ms + values.hold_ms + values.release_ms;
        let exceeds = lookahead_ms > total;
        let marker_color = GOLD;
        d.rect(GRAPH.0, GRAPH.1, GRAPH.2, GRAPH.3, PANEL);
        if let Some(hit) = active {
            let rect = Self::zone_rect(&laid, hit);
            let mut highlight = TEAL;
            highlight.a = 0.10;
            d.rect(rect.0, rect.1, rect.2, rect.3, highlight);
        }
        let mut anticipation = marker_color;
        anticipation.a = 0.10;
        d.rect(
            GRAPH.0,
            GRAPH.1,
            lookahead_x - GRAPH.0,
            GRAPH.3,
            anticipation,
        );
        for x in [
            laid.start.0,
            laid.attack_end.0,
            laid.hold_end.0,
            laid.release_end.0,
        ] {
            d.line(x, GRAPH.1, x, GRAPH.1 + GRAPH.3, LINE, 1.0);
        }
        d.outline(GRAPH, LINE);
        let attack_active = matches!(
            active,
            Some(EnvelopeHit::AttackLength | EnvelopeHit::AttackCurve)
        );
        let release_active = matches!(
            active,
            Some(EnvelopeHit::ReleaseLength | EnvelopeHit::ReleaseCurve)
        );
        // Draw stage paths separately so the active stage can brighten independently.
        for (left, right, stage_active) in [
            (laid.start.0, laid.attack_end.0, attack_active),
            (
                laid.attack_end.0,
                laid.hold_end.0,
                active == Some(EnvelopeHit::HoldLength),
            ),
            (laid.hold_end.0, laid.release_end.0, release_active),
        ] {
            let stage: Vec<_> = points
                .iter()
                .copied()
                .filter(|p| p.0 >= left && p.0 <= right)
                .collect();
            d.poly(
                &stage,
                if stage_active { GOLD } else { TEAL },
                if stage_active { 3.0 } else { 2.0 },
            );
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
        for (left, right, hit) in [
            (laid.start.0, laid.attack_end.0, EnvelopeHit::AttackLength),
            (laid.attack_end.0, laid.hold_end.0, EnvelopeHit::HoldLength),
            (
                laid.hold_end.0,
                laid.release_end.0,
                EnvelopeHit::ReleaseLength,
            ),
        ] {
            if !hover.is_some_and(|point| self.interactive_hit(&laid, point) == Some(hit)) {
                continue;
            }
            let mut hint = MUTED;
            hint.a = 0.20;
            let center = (left + right) * 0.5;
            let y = GRAPH.1 + GRAPH.3 - 15.0;
            d.line(center - 7.0, y, center + 7.0, y, hint, 3.5);
            for direction in [-1.0, 1.0] {
                let tip = center + 7.0 * direction;
                let shoulder = center + 3.0 * direction;
                d.poly(
                    &[(shoulder, y - 3.5), (tip, y), (shoulder, y + 3.5)],
                    hint,
                    2.5,
                );
            }
        }
        // True duration proportions remain visible beneath the enlarged stage zones.
        let mut x = GRAPH.0;
        for (ms, color) in [
            (values.attack_ms, TEAL),
            (values.hold_ms, GOLD),
            (values.release_ms, TEAL),
        ] {
            let width = GRAPH.2 * ms / total;
            d.rect(x, GRAPH.1 + GRAPH.3 - 5.0, width, 3.0, color);
            x += width;
        }
        dashed_line(
            &mut d,
            lookahead_x,
            GRAPH.1,
            GRAPH.1 + GRAPH.3 - 8.0,
            marker_color,
        );
        let line_active = self
            .drag
            .as_ref()
            .is_some_and(|drag| drag.lookahead_on_graph)
            || hover.is_some_and(|point| {
                lookahead_line_hit(&laid, point, lookahead_x)
                    && self.interactive_hit(&laid, point) == Some(EnvelopeHit::Lookahead)
            });
        if line_active {
            let mut guide = GOLD;
            guide.a = 0.65;
            for x in [lookahead_x - 4.0, lookahead_x + 4.0] {
                d.line(x, GRAPH.1, x, GRAPH.1 + GRAPH.3 - 8.0, guide, 1.0);
            }
        }
        if exceeds {
            d.text_right(GRAPH.0 + GRAPH.2 - 5.0, GRAPH.1 + 14.0, ">>>", 10.0, MUTED);
        }
        let slider_active = active == Some(EnvelopeHit::Lookahead);
        d.rect(LOOK_BAR.0, LOOK_BAR.1, LOOK_BAR.2, LOOK_BAR.3, PANEL);
        let mut slider_highlight = GOLD;
        slider_highlight.a = if slider_active { 0.12 } else { 0.04 };
        d.rect(
            LOOK_BAR.0,
            LOOK_BAR.1,
            LOOK_BAR.2,
            LOOK_BAR.3,
            slider_highlight,
        );
        let mut slider_fill = GOLD;
        slider_fill.a = if slider_active { 0.28 } else { 0.18 };
        let bar_x = lookahead_bar_x(lookahead_ms);
        d.rect(
            LOOK_BAR.0,
            LOOK_BAR.1,
            bar_x - LOOK_BAR.0,
            LOOK_BAR.3,
            slider_fill,
        );
        for (ms, color) in [
            (values.attack_ms, MUTED),
            (values.attack_ms + values.hold_ms, MUTED),
            (total, MUTED),
        ] {
            if ms <= 2000.0 {
                dashed_line(
                    &mut d,
                    lookahead_bar_x(ms),
                    LOOK_BAR.1,
                    LOOK_BAR.1 + LOOK_BAR.3,
                    color,
                );
            }
        }
        let mut slider_border = GOLD;
        slider_border.a = if slider_active { 0.8 } else { 0.35 };
        d.outline(LOOK_BAR, slider_border);
        let thumb_x = bar_x.clamp(LOOK_BAR.0 + 3.0, LOOK_BAR.0 + LOOK_BAR.2 - 3.0);
        d.rect(thumb_x - 3.0, LOOK_BAR.1 + 4.0, 6.0, LOOK_BAR.3 - 8.0, GOLD);
        let text = format!(
            "LOOKAHEAD {}",
            self.lookahead
                .normalized_value_to_string(self.lookahead.unmodulated_normalized_value(), true)
        );
        if bar_x < LOOK_BAR.0 + LOOK_BAR.2 * 0.5 {
            d.text(bar_x + 10.0, LOOK_BAR.1 + 20.0, &text, 11.0, GOLD);
        } else {
            d.text_right(bar_x - 10.0, LOOK_BAR.1 + 20.0, &text, 11.0, GOLD);
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
            d.rect(rect.0 + 2.0, rect.1 + 17.0, rect.2 - 6.0, 18.0, PANEL);
            d.text(rect.0 + 4.0, rect.1 + 30.0, &value, 13.0, accent);
            if hover.is_some_and(|p| Self::label_hit(p) == Some(hit)) {
                d.value_underline(
                    (rect.0, rect.1 + 14.0, rect.2 - 8.0, 18.0),
                    rect.1 + 30.0,
                    GOLD,
                );
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
                    if let Some(hit) = self.interactive_hit(&laid, point) {
                        if cx.modifiers().command() && hit != EnvelopeHit::Lookahead {
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
                    let axis_delta = self.drag.as_mut().map_or(0.0, |drag| {
                        drag.axis
                            .delta(point.0 - drag.origin.0, point.1 - drag.origin.1, 1.0)
                    });
                    if let Some(drag) = &self.drag {
                        let values = drag.start;
                        let laid = layout(Viewport::from_tuple(GRAPH), values);
                        match drag.hit {
                            EnvelopeHit::Lookahead => {
                                let fine = cx.modifiers().shift();
                                let start = self.lookahead.preview_plain(drag.start_norm);
                                let dx = axis_delta * if fine { 0.2 } else { 1.0 };
                                let ms = if drag.lookahead_on_graph {
                                    if dx.abs() < f32::EPSILON {
                                        start
                                    } else {
                                        let x = lookahead_marker_x(start, &laid, values) + dx;
                                        lookahead_drag_ms(
                                            x,
                                            &laid,
                                            values,
                                            fine || cx.modifiers().command(),
                                        )
                                    }
                                } else {
                                    quantize_lookahead_ms(lookahead_bar_ms(
                                        lookahead_bar_x(start) + dx,
                                    ))
                                };
                                self.set_plain(cx, drag.hit, ms);
                            }
                            EnvelopeHit::AttackCurve | EnvelopeHit::ReleaseCurve => {
                                let handle_y = match drag.hit {
                                    EnvelopeHit::AttackCurve => laid.attack_curve.1,
                                    _ => laid.release_curve.1,
                                };
                                let mut curve = curve_drag(
                                    if drag.hit == EnvelopeHit::AttackCurve {
                                        StageHit::AttackCurve
                                    } else {
                                        StageHit::ReleaseCurve
                                    },
                                    &laid,
                                    handle_y + point.1 - drag.origin.1,
                                );
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
                                let fine = cx.modifiers().shift();
                                let dx = axis_delta * if fine { 0.2 } else { 1.0 };
                                let plain = stage_time_drag(hit, values, dx);
                                self.set_plain(
                                    cx,
                                    hit,
                                    if fine || axis_delta == 0.0 {
                                        plain
                                    } else {
                                        Self::snapped_plain(hit, plain)
                                    },
                                );
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
                            let delta = Self::time_drag_delta(
                                press.press.target,
                                press.press.origin,
                                point,
                                cx.modifiers().shift(),
                            );
                            self.set_time_norm(
                                cx,
                                press.press.target,
                                (press.start_norm + delta).clamp(0.0, 1.0),
                                cx.modifiers().shift(),
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
                        if let Some(hit) = self
                            .interactive_hit(&laid, point)
                            .or_else(|| Self::label_hit(point))
                        {
                            let param = self.param(hit);
                            let step = if cx.modifiers().shift() { 0.004 } else { 0.02 };
                            param.begin_set_parameter(cx);
                            if hit == EnvelopeHit::Lookahead {
                                let ms = self.lookahead.unmodulated_plain_value();
                                let step = if ms < 1.0 || (*dy < 0.0 && ms <= 1.0) {
                                    0.1
                                } else {
                                    1.0
                                };
                                self.set_plain(
                                    cx,
                                    hit,
                                    quantize_lookahead_ms(ms + dy.signum() * step),
                                );
                            } else if cx.modifiers().shift()
                                || matches!(
                                    hit,
                                    EnvelopeHit::AttackCurve | EnvelopeHit::ReleaseCurve
                                )
                            {
                                let direction = if matches!(
                                    hit,
                                    EnvelopeHit::AttackLength | EnvelopeHit::ReleaseLength
                                ) {
                                    -1.0
                                } else {
                                    1.0
                                };
                                self.set_time_norm(
                                    cx,
                                    hit,
                                    (param.unmodulated_normalized_value() + *dy * step * direction)
                                        .clamp(0.0, 1.0),
                                    true,
                                );
                            } else if *dy != 0.0 {
                                let rate = matches!(
                                    hit,
                                    EnvelopeHit::AttackLength | EnvelopeHit::ReleaseLength
                                );
                                let plain = param.unmodulated_plain_value();
                                let ms = if rate { rate_to_ms(plain) } else { plain };
                                let next = if !rate && ms <= 1.0 {
                                    (ms + dy.signum()).max(0.0)
                                } else {
                                    super::step_points::next_time_ms(ms, *dy)
                                };
                                self.set_plain(cx, hit, if rate { ms_to_rate(next) } else { next });
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

    fn test_values() -> EnvelopeValues {
        EnvelopeValues {
            attack_ms: 10.0,
            hold_ms: 0.0,
            release_ms: 100.0,
            attack_curve: 0.0,
            release_curve: 0.0,
        }
    }

    #[test]
    fn attack_snap_is_narrow_and_modifiers_bypass_it() {
        let values = EnvelopeValues {
            attack_ms: 1000.0,
            hold_ms: 100.0,
            release_ms: 1000.0,
            ..test_values()
        };
        let laid = layout(Viewport::from_tuple(GRAPH), values);
        let x = laid.attack_end.0 + 0.5;
        assert_eq!(lookahead_drag_ms(x, &laid, values, false), 1000.0);
        assert!(lookahead_drag_ms(x, &laid, values, true) > 1000.0);
        assert!(lookahead_drag_ms(laid.attack_end.0 + 1.0, &laid, values, false) > 1000.0);
    }

    #[test]
    fn graph_lookahead_drag_round_trips_visually_through_scaled_stages() {
        for values in [
            EnvelopeValues {
                attack_ms: 0.1,
                hold_ms: 500.0,
                release_ms: 1500.0,
                ..test_values()
            },
            EnvelopeValues {
                attack_ms: 500.0,
                hold_ms: 10.0,
                release_ms: 100.0,
                ..test_values()
            },
            test_values(),
        ] {
            let laid = layout(Viewport::from_tuple(GRAPH), values);
            let max_x = lookahead_marker_x(2000.0, &laid, values);
            for i in 0..=100 {
                let x = laid.start.0 + (max_x - laid.start.0) * i as f32 / 100.0;
                let ms = lookahead_graph_ms(x, &laid, values);
                assert!((lookahead_marker_x(ms, &laid, values) - x).abs() < 0.001);
            }
        }
        let values = test_values();
        let laid = layout(Viewport::from_tuple(GRAPH), values);
        assert!(
            lookahead_graph_ms((laid.attack_end.0 + laid.hold_end.0) * 0.5, &laid, values)
                > values.attack_ms
        );
        assert_eq!(lookahead_graph_ms(laid.start.0 - 20.0, &laid, values), 0.0);
    }

    #[test]
    fn lookahead_line_has_a_small_hover_buffer_and_no_numeric_entry() {
        let laid = envelope_layout();
        let x = laid.attack_end.0;
        let y = laid.viewport.y + 10.0;
        assert!(lookahead_line_hit(&laid, (x - 5.0, y), x));
        assert!(lookahead_line_hit(&laid, (x + 5.0, y), x));
        assert!(!lookahead_line_hit(&laid, (x + 7.0, y), x));
        assert!(!lookahead_line_hit(&laid, (x, laid.viewport.y - 1.0), x));
        assert_eq!(EnvelopeView::label_rect(EnvelopeHit::Lookahead), None);
        let attack = EnvelopeView::label_rect(EnvelopeHit::AttackLength).unwrap();
        let release = EnvelopeView::label_rect(EnvelopeHit::ReleaseLength).unwrap();
        assert_eq!(attack.0, 10.0);
        assert!((release.0 + release.2 - (WIDTH - 10.0)).abs() < 0.001);
    }

    #[test]
    fn autoscale_keeps_extreme_stages_usable_and_tracks_duration_shares() {
        for values in [
            test_values(),
            EnvelopeValues {
                attack_ms: 0.1,
                hold_ms: 0.0,
                release_ms: 5000.0,
                ..test_values()
            },
            EnvelopeValues {
                attack_ms: 5000.0,
                hold_ms: 1000.0,
                release_ms: 40.0,
                ..test_values()
            },
        ] {
            let laid = layout(Viewport::from_tuple(GRAPH), values);
            for width in [
                laid.attack_end.0 - laid.start.0,
                laid.hold_end.0 - laid.attack_end.0,
                laid.release_end.0 - laid.hold_end.0,
            ] {
                assert!(width >= GRAPH.2 / 14.0 - 0.001);
            }
            assert!((laid.release_end.0 - GRAPH.0 - GRAPH.2).abs() < 0.001);
        }
        let long_attack = layout(
            Viewport::from_tuple(GRAPH),
            EnvelopeValues {
                attack_ms: 500.0,
                ..test_values()
            },
        );
        assert!(long_attack.attack_end.0 > envelope_layout().attack_end.0);
    }

    #[test]
    fn lookahead_tracks_each_stage_and_bar_round_trips_the_parameter_range() {
        let values = EnvelopeValues {
            hold_ms: 50.0,
            ..test_values()
        };
        let laid = layout(Viewport::from_tuple(GRAPH), values);
        for (time, x) in [
            (5.0, (laid.start.0 + laid.attack_end.0) * 0.5),
            (10.0, laid.attack_end.0),
            (35.0, (laid.attack_end.0 + laid.hold_end.0) * 0.5),
            (60.0, laid.hold_end.0),
            (110.0, (laid.hold_end.0 + laid.release_end.0) * 0.5),
            (160.0, laid.release_end.0),
            (2000.0, laid.release_end.0),
        ] {
            assert!((lookahead_marker_x(time, &laid, values) - x).abs() < 0.001);
        }
        for ms in [0.0, 1.0, 10.0, 500.0, 2000.0] {
            assert!((lookahead_bar_ms(lookahead_bar_x(ms)) - ms).abs() < 0.001);
        }
    }

    #[test]
    fn curve_nodes_round_trip_and_follow_the_pointer_in_both_directions() {
        for curve in [-2.0, -1.0, 0.0, 1.0, 2.0] {
            let values = EnvelopeValues {
                attack_curve: curve,
                release_curve: curve,
                ..test_values()
            };
            let laid = layout(Viewport::from_tuple(GRAPH), values);
            for (hit, y) in [
                (StageHit::AttackCurve, laid.attack_curve.1),
                (StageHit::ReleaseCurve, laid.release_curve.1),
            ] {
                assert!((curve_drag(hit, &laid, y) - curve).abs() < 0.001);
                let up = curve_drag(hit, &laid, y - 5.0);
                let down = curve_drag(hit, &laid, y + 5.0);
                assert!(if hit == StageHit::AttackCurve {
                    up <= curve && down >= curve
                } else {
                    up >= curve && down <= curve
                });
            }
        }
    }

    #[test]
    fn horizontal_time_drag_grows_each_stage_including_zero_hold() {
        for hit in [
            EnvelopeHit::AttackLength,
            EnvelopeHit::HoldLength,
            EnvelopeHit::ReleaseLength,
        ] {
            let plain = stage_time_drag(hit, test_values(), 10.0);
            let time = if hit == EnvelopeHit::HoldLength {
                plain
            } else {
                rate_to_ms(plain)
            };
            let start = match hit {
                EnvelopeHit::AttackLength => 10.0,
                EnvelopeHit::ReleaseLength => 100.0,
                _ => 0.0,
            };
            assert!(time > start);
        }
    }

    #[test]
    fn vertical_time_drag_increases_up_and_keeps_its_axis() {
        for hit in [
            EnvelopeHit::AttackLength,
            EnvelopeHit::HoldLength,
            EnvelopeHit::ReleaseLength,
        ] {
            let mut axis = pleasant_ui::pointer::AxisLock::default();
            assert_eq!(axis.delta(1.0, -2.0, 1.0), 0.0);
            let up_delta = axis.delta(1.0, -10.0, 1.0);
            assert_eq!(axis.delta(100.0, -10.0, 1.0), up_delta);
            let up = stage_time_drag(hit, test_values(), up_delta);
            let down = stage_time_drag(hit, test_values(), axis.delta(100.0, 10.0, 1.0));
            if hit == EnvelopeHit::HoldLength {
                assert!(up > down);
            } else {
                assert!(rate_to_ms(up) > rate_to_ms(down));
            }
        }
    }

    fn envelope_layout() -> EnvelopeLayout {
        layout(
            Viewport::from_tuple(GRAPH),
            EnvelopeValues {
                attack_ms: 10.0,
                hold_ms: 0.0,
                release_ms: 100.0,
                attack_curve: 0.0,
                release_curve: 0.0,
            },
        )
    }

    #[test]
    fn lookahead_has_a_graph_zone_and_a_monotonic_marker() {
        let laid = envelope_layout();
        let rect = EnvelopeView::zone_rect(&laid, EnvelopeHit::Lookahead);
        assert_eq!(
            EnvelopeView::graph_hit(&laid, (rect.0 + rect.2 * 0.5, rect.1 + 12.0)),
            Some(EnvelopeHit::Lookahead)
        );
        let values = test_values();
        assert_eq!(lookahead_marker_x(0.0, &laid, values), GRAPH.0);
        assert_eq!(
            lookahead_marker_x(values.attack_ms, &laid, values),
            laid.attack_end.0
        );
        assert!(lookahead_marker_x(10.0, &laid, values) < lookahead_marker_x(100.0, &laid, values));
        assert_eq!(
            lookahead_marker_x(2000.0, &laid, values),
            laid.release_end.0
        );
    }

    #[test]
    fn rate_controls_snap_in_milliseconds() {
        let rate = EnvelopeView::snapped_plain(EnvelopeHit::AttackLength, ms_to_rate(247.0));
        assert!((rate_to_ms(rate) - 250.0).abs() < 0.001);
        let rate = EnvelopeView::snapped_plain(EnvelopeHit::ReleaseLength, ms_to_rate(4480.0));
        assert!((rate_to_ms(rate) - 4500.0).abs() < 0.001);
        assert_eq!(
            EnvelopeView::snapped_plain(EnvelopeHit::HoldLength, 78.0),
            80.0
        );
    }

    #[test]
    fn stage_zones_cover_graph_even_with_zero_hold() {
        let laid = envelope_layout();
        for hit in [
            EnvelopeHit::AttackLength,
            EnvelopeHit::HoldLength,
            EnvelopeHit::ReleaseLength,
        ] {
            let rect = EnvelopeView::zone_rect(&laid, hit);
            assert!(rect.2 > 0.0);
            for y in [rect.1 + 2.0, rect.1 + rect.3 - 2.0] {
                assert_eq!(
                    EnvelopeView::graph_hit(&laid, (rect.0 + rect.2 * 0.5, y)),
                    Some(hit)
                );
            }
        }
        assert_eq!(
            EnvelopeView::graph_hit(&laid, (GRAPH.0 - 1.0, GRAPH.1)),
            None
        );
        assert_eq!(
            EnvelopeView::graph_hit(&laid, (GRAPH.0, GRAPH.1 + GRAPH.3 + 1.0)),
            None
        );
    }

    #[test]
    fn curve_handles_take_priority_over_time_zones() {
        let laid = envelope_layout();
        assert_eq!(
            EnvelopeView::graph_hit(&laid, laid.attack_curve),
            Some(EnvelopeHit::AttackCurve)
        );
        assert_eq!(
            EnvelopeView::graph_hit(&laid, laid.release_curve),
            Some(EnvelopeHit::ReleaseCurve)
        );
    }

    #[test]
    fn time_drags_are_relative_and_increase_displayed_time() {
        let origin = (100.0, 100.0);
        for hit in [
            EnvelopeHit::AttackLength,
            EnvelopeHit::HoldLength,
            EnvelopeHit::ReleaseLength,
        ] {
            assert_eq!(
                EnvelopeView::time_drag_delta(hit, origin, origin, false),
                0.0
            );
            let right = EnvelopeView::time_drag_delta(hit, origin, (110.0, 100.0), false);
            let up = EnvelopeView::time_drag_delta(hit, origin, (100.0, 90.0), false);
            assert_eq!(right, up);
            assert_eq!(right > 0.0, hit == EnvelopeHit::HoldLength);
            let fine = EnvelopeView::time_drag_delta(hit, origin, (110.0, 100.0), true);
            assert!(fine.abs() < right.abs());
        }
    }

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
