use std::sync::Arc;

use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;

use crate::params::ComposureParams;

use super::param_widget_ext;

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
        if super::appearance::draw_control(
            cx,
            canvas,
            &self.param_base,
            super::appearance::Control::Switch,
            self.param_base.modulated_normalized_value(),
        ) {
            return;
        }
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let params = super::EditorData::params.get(cx);
        if super::appearance::harmonic_control_inactive(&params, self.param_base.name()) {
            return;
        }
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
        if super::appearance::draw_control(
            cx,
            canvas,
            &self.param_base,
            super::appearance::Control::Switch,
            self.param_base.modulated_normalized_value(),
        ) {
            return;
        }
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

pub struct ProgramEnableButton {
    param: ParamWidgetBase,
    input: bool,
}

impl ProgramEnableButton {
    pub fn new(cx: &mut Context, input: bool) -> Handle<'_, Self> {
        Self {
            param: ParamWidgetBase::new(cx, super::EditorData::params, move |p| {
                if input {
                    &p.program_input_enable
                } else {
                    &p.program_gr_enable
                }
            }),
            input,
        }
        .build(cx, |_| {})
    }

    fn enabled(&self, params: &ComposureParams) -> bool {
        if self.input {
            params.program_input_enabled()
        } else {
            params.program_gr_enabled()
        }
    }
}

impl View for ProgramEnableButton {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let params = super::EditorData::params.get(cx);
        let width = cx.bounds().w / cx.scale_factor();
        let mut d = super::appearance::painter(cx, canvas, width);
        super::appearance::button(
            &mut d,
            (0.0, 0.0, width, 28.0),
            if self.input { "INPUT DEP" } else { "GR DEP" },
            self.enabled(&params),
            if params.prog_release_blend.value() == 0.0 {
                pleasant_ui::MUTED
            } else {
                pleasant_ui::TEAL
            },
        );
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, meta| {
            let params = super::EditorData::params.get(cx);
            if params.prog_release_blend.value() == 0.0 {
                return;
            }
            if matches!(e, WindowEvent::MouseDown(MouseButton::Left)) {
                param_widget_ext::toggle_bool_param(cx, &self.param);
                cx.needs_redraw();
                meta.consume();
            }
        });
    }
}
