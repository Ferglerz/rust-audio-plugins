//! JSFX-styled sliders + text buttons (HP/LP, curves, L/SC, etc.).

use std::sync::Arc;

use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::{Color as VgColor, Paint, Path};
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::widgets::util;

use crate::params::ComposureParams;

use super::display::UiDisplay;
use super::param_widget_ext::{self, ParamDragSession};
use super::step_points::StepSet;

const SLIDER_BG_OPACITY: f32 = 0.165;
const FILL_R: f32 = 0.8;
const FILL_G: f32 = 0.6;
const FILL_B: f32 = 0.2;

#[derive(Clone, Copy)]
pub enum SliderFill {
    LeftToRight,
    RightToLeft,
    CenterOut,
}

pub struct ReadoutSlider {
    param_base: ParamWidgetBase,
    display: Arc<UiDisplay>,
    drag: ParamDragSession,
    on_filter_drag: bool,
    fill: SliderFill,
    step_set: StepSet,
}

impl ReadoutSlider {
    pub fn new<L, P, F>(
        cx: &mut Context,
        params: L,
        map: F,
        display: Arc<UiDisplay>,
        fill: SliderFill,
        on_filter_drag: bool,
        step_set: StepSet,
    ) -> Handle<'_, ReadoutSlider>
    where
        L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
        P: Param + 'static,
        F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
    {
        ReadoutSlider {
            param_base: ParamWidgetBase::new(cx, params, map),
            display,
            drag: ParamDragSession::INACTIVE,
            on_filter_drag,
            fill,
            step_set,
        }
        .build(cx, |_| {})
    }

    fn draw_fill(&self, canvas: &mut Canvas, bounds: BoundingBox, norm: f32, opacity: f32) {
        let fill_paint = Paint::color(VgColor::rgbaf(FILL_R, FILL_G, FILL_B, opacity));
        match self.fill {
            SliderFill::LeftToRight => {
                let fill_w = bounds.w * norm;
                if fill_w > 0.5 {
                    let mut fill = Path::new();
                    fill.rect(bounds.x, bounds.y, fill_w, bounds.h);
                    canvas.fill_path(&fill, &fill_paint);
                }
            }
            SliderFill::RightToLeft => {
                let fill_start = bounds.x + bounds.w * norm;
                let fill_w = bounds.w - bounds.w * norm;
                if fill_w > 0.5 {
                    let mut fill = Path::new();
                    fill.rect(fill_start, bounds.y, fill_w, bounds.h);
                    canvas.fill_path(&fill, &fill_paint);
                }
            }
            SliderFill::CenterOut => {
                let center_x = bounds.x + bounds.w * 0.5;
                let fader_x = bounds.x + bounds.w * norm;
                if fader_x > center_x + 0.5 {
                    let mut fill = Path::new();
                    fill.rect(center_x, bounds.y, fader_x - center_x, bounds.h);
                    canvas.fill_path(&fill, &fill_paint);
                } else if fader_x < center_x - 0.5 {
                    let mut fill = Path::new();
                    fill.rect(fader_x, bounds.y, center_x - fader_x, bounds.h);
                    canvas.fill_path(&fill, &fill_paint);
                }
            }
        }

        let handle_x = bounds.x + bounds.w * norm - 3.0;
        let mut handle = Path::new();
        handle.rect(handle_x, bounds.y - 2.0, 6.0, bounds.h + 4.0);
        canvas.fill_path(
            &handle,
            &Paint::color(VgColor::rgbaf(0.92, 0.92, 0.92, opacity)),
        );
    }
}

impl View for ReadoutSlider {
    fn element(&self) -> Option<&'static str> {
        Some("readout-slider")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let opacity = cx.opacity();
        let norm = param_widget_ext::ui_normalized(&self.param_base, self.step_set);
        if super::appearance::draw_control(
            cx,
            canvas,
            &self.param_base,
            super::appearance::Control::Slider(self.fill),
            norm,
        ) {
            return;
        }

        let mut track = Path::new();
        track.rect(bounds.x, bounds.y, bounds.w, bounds.h);
        canvas.fill_path(
            &track,
            &Paint::color(VgColor::rgbaf(0.3, 0.3, 0.3, SLIDER_BG_OPACITY * opacity)),
        );

        self.draw_fill(canvas, bounds, norm, opacity);
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                if param_widget_ext::reset_to_default_on_command_click(cx, &self.param_base) {
                } else {
                    let ui_norm = util::remap_current_entity_x_coordinate(cx, cx.mouse().cursorx);
                    param_widget_ext::param_drag_begin_horizontal(
                        &mut self.drag,
                        cx,
                        &self.param_base,
                        &self.display,
                        self.step_set,
                        ui_norm,
                        self.on_filter_drag,
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
                    self.on_filter_drag,
                ) {
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(_, _) => {
                if self.drag.active {
                    let ui_norm = util::remap_current_entity_x_coordinate(cx, cx.mouse().cursorx);
                    param_widget_ext::param_drag_move_horizontal(
                        cx,
                        &self.param_base,
                        &self.display,
                        self.step_set,
                        ui_norm,
                    );
                    meta.consume();
                }
            }
            _ => {}
        });
    }
}

#[derive(Clone, Copy)]
pub enum JsfxButtonLabel {
    Listen,
    Sidechain,
    Brickwall,
    Inverse,
}

impl JsfxButtonLabel {
    const fn text(self) -> &'static str {
        match self {
            Self::Listen => "L",
            Self::Sidechain => "SC",
            Self::Brickwall => "Brickwall",
            Self::Inverse => "Inverse",
        }
    }
}

/// Yellow JSFX generic button (L, SC, Brickwall, Inverse).
pub struct JsfxParamButton {
    param_base: ParamWidgetBase,
}

impl JsfxParamButton {
    pub fn new<L, P, F>(
        cx: &mut Context,
        params: L,
        map: F,
        label: JsfxButtonLabel,
    ) -> Handle<'_, JsfxParamButton>
    where
        L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
        P: Param + 'static,
        F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
    {
        JsfxParamButton {
            param_base: ParamWidgetBase::new(cx, params, map),
        }
        .build(cx, |cx| {
            Label::new(cx, label.text())
                .class("jsfx-button-label")
                .width(Stretch(1.0))
                .height(Stretch(1.0));
        })
    }
}

impl View for JsfxParamButton {
    fn element(&self) -> Option<&'static str> {
        Some("jsfx-button")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let opacity = cx.opacity();
        let on = param_widget_ext::bool_param_on(&self.param_base);
        if super::appearance::draw_control(
            cx,
            canvas,
            &self.param_base,
            super::appearance::Control::Button,
            if on { 1.0 } else { 0.0 },
        ) {
            return;
        }
        let (r, g, b) = if on {
            (0.528, 0.396, 0.132)
        } else {
            (0.2, 0.15, 0.05)
        };

        let mut bg = Path::new();
        bg.rect(bounds.x, bounds.y, bounds.w, bounds.h);
        canvas.fill_path(&bg, &Paint::color(VgColor::rgbaf(r, g, b, opacity)));

        let mut border = Path::new();
        border.rect(bounds.x, bounds.y, bounds.w, bounds.h);
        canvas.stroke_path(&border, &{
            let mut p = Paint::color(VgColor::rgbaf(0.6, 0.6, 0.6, opacity));
            p.set_line_width(1.0);
            p
        });
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                param_widget_ext::toggle_bool_param(cx, &self.param_base);
                cx.needs_redraw();
                meta.consume();
            }
            _ => {}
        });
    }
}
