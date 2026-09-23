use std::sync::Arc;

use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::{Color as VgColor, Paint, Path};
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;

use crate::params::ComposureParams;

use super::display::UiDisplay;
use super::param_widget_ext::{self, ParamDragSession};
use super::step_points::StepSet;
use super::texture_cache::draw_tex;
use super::ui_assets as assets;

const KNOB_TEMPLATE_PX: f32 = 125.0;
const KNOB_IMAGE_W: f32 = 92.0;
const KNOB_IMAGE_H: f32 = 95.0;
const KNOB_LINE_OFFSET_REF: f32 = 24.0;
const KNOB_LINE_LENGTH_RATIO: f32 = 0.25;
const KNOB_DRAG_SENS: f32 = 0.004;
const KNOB_FINE_SENS: f32 = 0.0008;

fn knob_indicator(
    canvas: &mut Canvas,
    cxp: f32,
    cyp: f32,
    radius: f32,
    norm: f32,
    opacity: f32,
    line_offset: f32,
) {
    let angle_deg = 135.0 + norm * 270.0;
    let angle = angle_deg.to_radians();
    let indicator_radius = radius - line_offset;
    let line_length = radius * KNOB_LINE_LENGTH_RATIO;
    let line_start = indicator_radius - line_length;
    let x0 = cxp + angle.cos() * line_start;
    let y0 = cyp + angle.sin() * line_start;
    let x1 = cxp + angle.cos() * indicator_radius;
    let y1 = cyp + angle.sin() * indicator_radius;
    let mut line = Path::new();
    line.move_to(x0, y0);
    line.line_to(x1, y1);
    canvas.stroke_path(&line, &{
        let mut p = Paint::color(VgColor::rgbaf(0.067, 0.733, 1.0, opacity));
        p.set_line_width(1.5);
        p
    });
}

// ── Image knob ───────────────────────────────────────────────────────────────

pub struct ImageKnob {
    param_base: ParamWidgetBase,
    display: Arc<UiDisplay>,
    drag: ParamDragSession,
    step_set: StepSet,
}

impl ImageKnob {
    pub fn new<L, P, F>(
        cx: &mut Context,
        params: L,
        map: F,
        display: Arc<UiDisplay>,
        step_set: StepSet,
    ) -> Handle<'_, ImageKnob>
    where
        L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
        P: Param + 'static,
        F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
    {
        ImageKnob {
            param_base: ParamWidgetBase::new(cx, params, map),
            display,
            drag: ParamDragSession::INACTIVE,
            step_set,
        }
        .build(cx, |_| {})
    }
}

impl View for ImageKnob {
    fn element(&self) -> Option<&'static str> {
        Some("image-knob")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let opacity = cx.opacity();
        let norm = param_widget_ext::ui_normalized(&self.param_base, self.step_set);
        if super::appearance::draw_control(
            cx,
            canvas,
            &self.param_base,
            super::appearance::Control::Knob,
            norm,
        ) {
            return;
        }
        let cxp = bounds.x + bounds.w * 0.5;
        let cyp = bounds.y + bounds.h * 0.5;
        let scale = bounds.w / KNOB_TEMPLATE_PX;
        let scaled_w = KNOB_TEMPLATE_PX * scale;
        let scaled_h = scaled_w * (KNOB_IMAGE_H / KNOB_IMAGE_W);
        let line_offset = KNOB_LINE_OFFSET_REF * scale;
        draw_tex(
            canvas,
            "knob",
            assets::KNOB,
            cxp - scaled_w * 0.5,
            cyp - scaled_h * 0.5,
            scaled_w,
            scaled_h,
            opacity,
        );
        knob_indicator(canvas, cxp, cyp, bounds.w * 0.5, norm, opacity, line_offset);
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                if param_widget_ext::reset_to_default_on_command_click(cx, &self.param_base) {
                } else {
                    param_widget_ext::param_drag_begin_vertical(
                        &mut self.drag,
                        cx,
                        &self.param_base,
                        &self.display,
                        self.step_set,
                        cx.mouse().cursory,
                    );
                }
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if param_widget_ext::param_drag_end(
                    &mut self.drag,
                    cx,
                    &self.param_base,
                    &self.display,
                    self.step_set,
                    false,
                ) {
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(_, y) => {
                if self.drag.active {
                    param_widget_ext::param_drag_move_vertical(
                        &self.drag,
                        cx,
                        &self.param_base,
                        &self.display,
                        self.step_set,
                        *y,
                        KNOB_DRAG_SENS,
                        KNOB_FINE_SENS,
                    );
                    meta.consume();
                }
            }
            _ => {}
        });
    }
}
