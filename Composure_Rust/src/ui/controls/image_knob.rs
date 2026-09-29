use std::sync::Arc;

use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;

use crate::params::ComposureParams;

use super::display::UiDisplay;
use super::param_widget_ext::{self, ParamDragSession};
use super::step_points::StepSet;

const KNOB_DRAG_SENS: f32 = 0.004;
const KNOB_FINE_SENS: f32 = 0.0008;

// ── Image knob ───────────────────────────────────────────────────────────────

pub struct ImageKnob {
    param_base: ParamWidgetBase,
    display: Arc<UiDisplay>,
    drag: ParamDragSession,
    value_edit: param_widget_ext::ParamValueEdit,
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
            value_edit: param_widget_ext::ParamValueEdit::default(),
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
        let params = super::EditorData::params.get(cx);
        let inactive = super::appearance::program_control_inactive(&params, self.param_base.name())
            || super::appearance::harmonic_control_inactive(&params, self.param_base.name());
        let norm = param_widget_ext::ui_normalized(&self.param_base, self.step_set);
        if super::appearance::draw_control(
            cx,
            canvas,
            &self.param_base,
            super::appearance::Control::Knob,
            norm,
        ) {
            super::appearance::draw_knob_activity(
                cx,
                canvas,
                self.param_base.name(),
                &self.display,
            );
            if !inactive {
                self.value_edit
                    .draw(cx, canvas, super::appearance::Control::Knob);
            }
            return;
        }
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let params = super::EditorData::params.get(cx);
        if !self.drag.active
            && (super::appearance::program_control_inactive(&params, self.param_base.name())
                || super::appearance::harmonic_control_inactive(&params, self.param_base.name()))
        {
            return;
        }
        event.map(|window_event, meta| {
            if !self.drag.active {
                let handled = self.value_edit.event(
                    cx,
                    window_event,
                    &self.param_base,
                    super::appearance::Control::Knob,
                );
                if handled {
                    meta.consume();
                    return;
                }
            }
            match window_event {
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
                WindowEvent::MouseUp(MouseButton::Left) | WindowEvent::FocusOut => {
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
            }
        });
    }
}
