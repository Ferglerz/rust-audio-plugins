//! Gain-reduction meters — thresholds, trails, reflection strip.

use std::sync::Arc;

use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::{Color as VgColor, Paint, Path};
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::widgets::util::ModifiersExt;

use crate::params::ComposureParams;

use super::display::{METER_TRAIL_FADE_PEAK, TRAIL_MAX_AGE_SECS, UiDisplay};
use super::draw_helpers::{self, gr_vg_color};
use super::theme;
use super::threshold_lines::{self, ThresholdLine};

pub struct GrMeterView<L, R, P> {
    gr_db: L,
    range_db: R,
    params: P,
    display: Arc<UiDisplay>,
    handles: MeterThresholdHandles,
    dragging_threshold: Option<ThresholdLine>,
    hovered_threshold: Option<ThresholdLine>,
    dragging_knee: bool,
    last_drag_y: f32,
    meter_captured: bool,
}

impl GrMeterView<(), (), ()> {
    pub fn new<L, R, P>(
        cx: &mut Context,
        gr_db: L,
        range_db: R,
        params: P,
        display: Arc<UiDisplay>,
    ) -> Handle<'_, GrMeterView<L, R, P>>
    where
        L: Lens<Target = f32>,
        R: Lens<Target = f32>,
        P: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    {
        let handles = MeterThresholdHandles::new(cx, params.clone());

        GrMeterView {
            gr_db,
            range_db,
            params,
            display,
            handles,
            dragging_threshold: None,
            hovered_threshold: None,
            dragging_knee: false,
            last_drag_y: 0.0,
            meter_captured: false,
        }
        .build(cx, |_| {})
    }
}

impl<L, R, P> GrMeterView<L, R, P>
where
    L: Lens<Target = f32>,
    R: Lens<Target = f32>,
    P: Lens<Target = Arc<ComposureParams>> + Clone,
{
    fn meter_bounds(&self, cx: &EventContext) -> BoundingBox {
        cx.bounds()
    }

    fn active_threshold(&self) -> Option<ThresholdLine> {
        self.dragging_threshold.or(self.hovered_threshold)
    }

    fn is_gr_line(line: ThresholdLine) -> bool {
        matches!(
            line,
            ThresholdLine::GrBlendReduction | ThresholdLine::GrBlendAddition
        )
    }

    fn begin_threshold_drag(
        &mut self,
        cx: &mut EventContext,
        line: ThresholdLine,
        knee_mode: bool,
        mouse_y: f32,
    ) {
        self.dragging_threshold = Some(line);
        self.dragging_knee = knee_mode;
        self.last_drag_y = mouse_y;
        self.meter_captured = true;
        self.handles.begin_drag(cx, line, knee_mode);
        cx.capture();
    }

    fn end_threshold_drag(&mut self, cx: &mut EventContext) {
        if let Some(line) = self.dragging_threshold.take() {
            self.handles.end_drag(cx, line, self.dragging_knee);
        }
        self.dragging_knee = false;
        if self.meter_captured {
            cx.release();
            self.meter_captured = false;
        }
    }

    fn apply_knee_drag(&self, cx: &mut EventContext, line: ThresholdLine, dy: f32) {
        let bounds = self.meter_bounds(cx);
        let range = self.range_db.get(cx).max(1.0);
        let knee_span = (range * 2.0).max(1.0);
        let delta_db = -(dy / bounds.h.max(1.0)) * knee_span;
        self.handles.apply_knee_drag(cx, line, delta_db);
    }

    fn apply_threshold_drag(&self, cx: &mut EventContext, line: ThresholdLine, mouse_y: f32) {
        let bounds = self.meter_bounds(cx);
        let range = self.range_db.get(cx).max(1.0);
        self.handles.apply_threshold_drag(cx, line, mouse_y, bounds, range);
    }
}

impl<L, R, P> View for GrMeterView<L, R, P>
where
    L: Lens<Target = f32>,
    R: Lens<Target = f32>,
    P: Lens<Target = Arc<ComposureParams>> + Clone,
{
    fn element(&self) -> Option<&'static str> {
        Some("composure-gr-meter")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                let cursor_y = cx.mouse().cursory;
                let cursor_x = cx.mouse().cursorx;
                let bounds = self.meter_bounds(cx);
                let range = self.range_db.get(cx).max(1.0);
                let params = self.params.get(cx);
                if let Some(line) =
                    threshold_lines::find_meter_threshold(&params, cursor_x, cursor_y, bounds, range)
                {
                    let knee = cx.modifiers().command() && Self::is_gr_line(line);
                    self.begin_threshold_drag(cx, line, knee, cursor_y);
                    meta.consume();
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.dragging_threshold.is_some() {
                    self.end_threshold_drag(cx);
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(_, _) => {
                let cursor_y = cx.mouse().cursory;
                let cursor_x = cx.mouse().cursorx;
                let bounds = self.meter_bounds(cx);
                let range = self.range_db.get(cx).max(1.0);
                let params = self.params.get(cx);
                if let Some(line) = self.dragging_threshold {
                    let knee_now =
                        cx.modifiers().command() && Self::is_gr_line(line);
                    if knee_now != self.dragging_knee && Self::is_gr_line(line) {
                        self.end_threshold_drag(cx);
                        self.begin_threshold_drag(cx, line, knee_now, cursor_y);
                    } else if self.dragging_knee {
                        let dy = cursor_y - self.last_drag_y;
                        if dy.abs() > 0.01 {
                            self.apply_knee_drag(cx, line, dy);
                            self.last_drag_y = cursor_y;
                        }
                    } else {
                        self.apply_threshold_drag(cx, line, cursor_y);
                    }
                    meta.consume();
                } else {
                    self.hovered_threshold =
                        threshold_lines::find_meter_threshold(
                            &params, cursor_x, cursor_y, bounds, range,
                        );
                }
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        self.display.age_dot_trails_one_frame();

        let bounds = cx.bounds();
        if bounds.w < 1.0 || bounds.h < 1.0 {
            return;
        }

        let opacity = cx.opacity();
        let gr = self.gr_db.get(cx);
        let range = self.range_db.get(cx).max(1.0);
        let meter_bounds = bounds;
        let meter_h = meter_bounds.h;
        let px_per_db = meter_h / range;
        let params = self.params.get(cx);
        let center_y = meter_bounds.y + meter_h * 0.5;
        let active = self.active_threshold();

        let mut bg = Path::new();
        bg.rect(meter_bounds.x, meter_bounds.y, meter_bounds.w, meter_h);
        let bg_paint = Paint::color(VgColor::rgbaf(0.12, 0.12, 0.12, 0.35 * opacity));
        canvas.fill_path(&bg, &bg_paint);

        draw_trail_lines(
            canvas,
            meter_bounds,
            &self.display,
            px_per_db,
            center_y,
            opacity,
        );

        threshold_lines::draw_gr_blend_thresholds(
            canvas,
            meter_bounds,
            &params,
            range,
            opacity,
            active,
        );
        threshold_lines::draw_rate_change_threshold(
            canvas,
            meter_bounds,
            &params,
            opacity,
            active,
        );

        if let Some((x, y, w, h)) = draw_helpers::gr_bar_rect(gr, meter_bounds, px_per_db) {
            draw_helpers::gr_fill_bar(canvas, gr, x, y, w, h, opacity);
        }

        draw_meter_reflection(canvas, meter_bounds, gr, range, opacity);
    }
}

/// History lines never reach full meter opacity — keeps them below the live readout.
const METER_REFLECTION_PEAK: f32 = 0.28125;

fn draw_trail_lines(
    canvas: &mut Canvas,
    bounds: BoundingBox,
    display: &UiDisplay,
    px_per_db: f32,
    center_y: f32,
    opacity: f32,
) {
    display.read_trails(|lines| {
        for line in lines {
            let alpha = super::display::fade_by_age(
                line.age_secs,
                TRAIL_MAX_AGE_SECS,
                METER_TRAIL_FADE_PEAK,
                true,
            ) * opacity;
            if alpha < 0.01 {
                continue;
            }
            let Some(y) = draw_helpers::gr_trail_line_y(line.gr_db, bounds, px_per_db, center_y) else {
                continue;
            };
            let color = gr_vg_color(line.gr_db, alpha);
            let mut path = Path::new();
            path.rect(bounds.x, y - 1.0, bounds.w, 3.0);
            canvas.fill_path(&path, &Paint::color(color));
        }
    });
}

struct MeterThresholdHandles {
    gr_cut: ParamWidgetBase,
    gr_boost: ParamWidgetBase,
    gr_cut_knee: ParamWidgetBase,
    gr_boost_knee: ParamWidgetBase,
    rate_mod: ParamWidgetBase,
}

impl MeterThresholdHandles {
    fn new<P>(cx: &mut Context, params: P) -> Self
    where
        P: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    {
        Self {
            gr_cut: ParamWidgetBase::new(cx, params.clone(), |p| {
                &p.gr_blend_threshold_reduction_db
            }),
            gr_boost: ParamWidgetBase::new(cx, params.clone(), |p| {
                &p.gr_blend_threshold_addition_db
            }),
            rate_mod: ParamWidgetBase::new(cx, params.clone(), |p| {
                &p.rate_change_threshold_modifier
            }),
            gr_cut_knee: ParamWidgetBase::new(cx, params.clone(), |p| {
                &p.gr_blend_threshold_reduction_knee_db
            }),
            gr_boost_knee: ParamWidgetBase::new(cx, params, |p| {
                &p.gr_blend_threshold_addition_knee_db
            }),
        }
    }

    fn param_for(&self, line: ThresholdLine, knee: bool) -> Option<&ParamWidgetBase> {
        match (line, knee) {
            (ThresholdLine::GrBlendReduction, false) => Some(&self.gr_cut),
            (ThresholdLine::GrBlendAddition, false) => Some(&self.gr_boost),
            (ThresholdLine::RateChangeModifier, false) => Some(&self.rate_mod),
            (ThresholdLine::GrBlendReduction, true) => Some(&self.gr_cut_knee),
            (ThresholdLine::GrBlendAddition, true) => Some(&self.gr_boost_knee),
            _ => None,
        }
    }

    fn begin_drag(&self, cx: &mut EventContext, line: ThresholdLine, knee: bool) {
        if let Some(base) = self.param_for(line, knee) {
            base.begin_set_parameter(cx);
        }
    }

    fn end_drag(&self, cx: &mut EventContext, line: ThresholdLine, knee: bool) {
        if let Some(base) = self.param_for(line, knee) {
            base.end_set_parameter(cx);
        }
    }

    fn apply_knee_drag(&self, cx: &mut EventContext, line: ThresholdLine, delta_db: f32) {
        let Some(base) = self.param_for(line, true) else {
            return;
        };
        let current = base.unmodulated_plain_value();
        let v = (current - delta_db).clamp(0.0, 12.0);
        base.set_normalized_value(cx, base.preview_normalized(v));
    }

    fn apply_threshold_drag(
        &self,
        cx: &mut EventContext,
        line: ThresholdLine,
        mouse_y: f32,
        bounds: BoundingBox,
        range: f32,
    ) {
        let Some(base) = self.param_for(line, false) else {
            return;
        };
        let Some(v) =
            threshold_lines::meter_threshold_value_from_y(line, mouse_y, bounds, range)
        else {
            return;
        };
        base.set_normalized_value(cx, base.preview_normalized(v));
    }
}

fn draw_meter_reflection(
    canvas: &mut Canvas,
    meter_bounds: BoundingBox,
    gr_db: f32,
    range_db: f32,
    opacity: f32,
) {
    let meter_weight = draw_helpers::reflection_weight_from_level(gr_db, range_db);
    let Some(mut meter_alpha) =
        draw_helpers::reflection_alpha(meter_weight, METER_REFLECTION_PEAK, opacity, 0.001)
    else {
        return;
    };

    let (r, g, b) = if gr_db < 0.0 {
        let blue_weight = 1.0 - meter_weight * 0.7;
        let orange_weight = 1.0 - blue_weight;
        let blue_opacity = meter_alpha * blue_weight * 1.5;
        let orange_opacity = meter_alpha * orange_weight;
        let total = (blue_opacity + orange_opacity).max(0.001);
        meter_alpha = blue_opacity + orange_opacity;
        (
            (1.0 * orange_opacity + 0.2 * blue_opacity) / total,
            (0.5 * orange_opacity + 0.5 * blue_opacity) / total,
            (0.0 * orange_opacity + 1.0 * blue_opacity) / total,
        )
    } else if gr_db > 0.0 {
        (0.2, 0.5, 1.0)
    } else {
        return;
    };

    let r = r * 0.7 + 0.3;
    let g = g * 0.7 + 0.3;
    let b = b * 0.7 + 0.3;

    let ref_y = meter_bounds.y + meter_bounds.h + theme::METER_REFLECTION_GAP;
    let levels = [0.5, 0.75, 1.0, 1.0, 1.0, 0.75, 0.5];
    draw_helpers::draw_reflection_strip(
        canvas,
        meter_bounds.x,
        ref_y,
        meter_bounds.w,
        r,
        g,
        b,
        meter_alpha,
        &levels,
    );
}
