use std::sync::Arc;

use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;

use crate::params::{ComposureParams, ProgramReleaseMode};

use super::ui_assets as assets;
use super::param_widget_ext;
use super::theme;
use super::texture_cache::draw_tex;

pub fn draw_switch(canvas: &mut Canvas, bounds: BoundingBox, on: bool, opacity: f32) {
    let png = if on { assets::SWITCH_UP } else { assets::SWITCH_DN };
    let key = if on { "switch_up" } else { "switch_dn" };
    // Widget is placed at JSFX_x - SWITCH_LEFT_OVERHANG so the 83px art is not clipped.
    draw_tex(
        canvas,
        key,
        png,
        bounds.x,
        bounds.y,
        theme::SWITCH_W,
        theme::SWITCH_H,
        opacity,
    );
}

// ── Image switch ─────────────────────────────────────────────────────────────

pub struct ImageSwitch {
    param_base: ParamWidgetBase,
}

impl ImageSwitch {
    pub fn new<L, P, F>(cx: &mut Context, params: L, map: F) -> Handle<'_, ImageSwitch>
    where
        L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
        P: Param + 'static,
        F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
    {
        ImageSwitch {
            param_base: ParamWidgetBase::new(cx, params, map),
        }
        .build(cx, |_| {})
    }
}

impl View for ImageSwitch {
    fn element(&self) -> Option<&'static str> {
        Some("image-switch")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let on = param_widget_ext::bool_param_on(&self.param_base);
        draw_switch(canvas, cx.bounds(), on, cx.opacity());
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

// ── Program release mode (3-state, click cycles) ─────────────────────────────

pub struct ProgModeSwitch {
    param_base: ParamWidgetBase,
}

impl ProgModeSwitch {
    pub fn new<L, P, F>(cx: &mut Context, params: L, map: F) -> Handle<'_, ProgModeSwitch>
    where
        L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
        P: Param + 'static,
        F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
    {
        let mode_label = params.map(|p: &Arc<ComposureParams>| {
            p.prog_release_mode.value().short_label().to_string()
        });

        ProgModeSwitch {
            param_base: ParamWidgetBase::new(cx, params, map),
        }
        .build(cx, |cx| {
            Label::new(cx, mode_label)
                .class("prog-mode-label")
                .bottom(Pixels(2.0))
                .left(Stretch(1.0))
                .right(Stretch(1.0))
                .height(Pixels(14.0));
        })
    }
}

impl View for ProgModeSwitch {
    fn element(&self) -> Option<&'static str> {
        Some("image-switch")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let mode = match self.param_base.modulated_plain_value().round() as i32 {
            0 => ProgramReleaseMode::InputDependent,
            1 => ProgramReleaseMode::GrDependent,
            _ => ProgramReleaseMode::RateOfChange,
        };
        let on = mode.switch_on();
        draw_switch(canvas, cx.bounds(), on, cx.opacity());
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                param_widget_ext::cycle_enum_param(cx, &self.param_base, 3);
                cx.needs_redraw();
                meta.consume();
            }
            _ => {}
        });
    }
}

// ── Detection feed / feedforward ─────────────────────────────────────────────

pub struct DetectionModeButton {
    param_base: ParamWidgetBase,
}

impl DetectionModeButton {
    pub fn new<L, P, F>(cx: &mut Context, params: L, map: F) -> Handle<'_, DetectionModeButton>
    where
        L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
        P: Param + 'static,
        F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
    {
        DetectionModeButton {
            param_base: ParamWidgetBase::new(cx, params, map),
        }
        .build(cx, |_| {})
    }
}

impl View for DetectionModeButton {
    fn element(&self) -> Option<&'static str> {
        Some("detection-mode-button")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let opacity = cx.opacity();
        let feedforward = self.param_base.modulated_plain_value() >= 0.5;
        let (key, png) = if feedforward {
            ("feedfwrd_off", assets::FEEDFWRD_OFF)
        } else {
            ("feedback_on", assets::FEEDBACK_ON)
        };
        draw_tex(canvas, key, png, bounds.x, bounds.y, bounds.w, bounds.h, opacity);
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

pub type HarmonicTypeSwitch = ImageSwitch;
