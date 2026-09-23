//! Shared parameter-widget helpers for stepped/continuous drag and bool toggles.

use nih_plug_vizia::vizia::prelude::EventContext;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::widgets::util::ModifiersExt;

use super::display::UiDisplay;
use super::step_points::{self, StepSet};

pub struct ParamDragSession {
    pub active: bool,
    pub start_y: f32,
    pub start_norm: f32,
}

impl ParamDragSession {
    pub const INACTIVE: Self = Self {
        active: false,
        start_y: 0.0,
        start_norm: 0.0,
    };
}

pub fn bool_param_on(param_base: &ParamWidgetBase) -> bool {
    param_base.unmodulated_normalized_value() >= 0.5
}

pub fn toggle_bool_param(cx: &mut EventContext, param_base: &ParamWidgetBase) {
    param_base.begin_set_parameter(cx);
    let on = bool_param_on(param_base);
    param_base.set_normalized_value(cx, if on { 0.0 } else { 1.0 });
    param_base.end_set_parameter(cx);
}

pub fn cycle_enum_param(cx: &mut EventContext, param_base: &ParamWidgetBase, count: u32) {
    param_base.begin_set_parameter(cx);
    let current = param_base.modulated_plain_value().round() as i32;
    let next_plain = ((current + 1).rem_euclid(count as i32)) as f32;
    let norm = param_base.preview_normalized(next_plain);
    param_base.set_normalized_value(cx, norm);
    param_base.end_set_parameter(cx);
}

pub fn reset_to_default_on_command_click(
    cx: &mut EventContext,
    param_base: &ParamWidgetBase,
) -> bool {
    if cx.modifiers().command() {
        param_base.begin_set_parameter(cx);
        param_base.set_normalized_value(cx, param_base.default_normalized_value());
        param_base.end_set_parameter(cx);
        true
    } else {
        false
    }
}

pub fn ui_normalized(param_base: &ParamWidgetBase, step_set: StepSet) -> f32 {
    if let Some(steps) = step_set.steps() {
        step_points::value_to_step_norm(
            param_base.unmodulated_plain_value() as f64,
            steps,
        )
    } else {
        param_base.unmodulated_normalized_value().clamp(0.0, 1.0)
    }
}

fn ui_norm_to_normalized(param_base: &ParamWidgetBase, step_set: StepSet, ui_norm: f32) -> f32 {
    if let Some(steps) = step_set.steps() {
        let plain = step_points::step_norm_to_value(ui_norm, steps);
        param_base.preview_normalized(plain as f32)
    } else {
        ui_norm
    }
}

pub fn set_from_ui_norm(cx: &mut EventContext, param_base: &ParamWidgetBase, step_set: StepSet, ui_norm: f32) {
    let norm = ui_norm_to_normalized(param_base, step_set, ui_norm);
    param_base.set_normalized_value(cx, norm);
}

pub fn set_readout_from_ui_norm(
    display: &UiDisplay,
    param_base: &ParamWidgetBase,
    step_set: StepSet,
    ui_norm: f32,
) {
    let norm = ui_norm_to_normalized(param_base, step_set, ui_norm);
    display.set_param_readout(param_base.normalized_value_to_string(norm, true));
}

pub fn apply_drag_ui_norm(
    cx: &mut EventContext,
    param_base: &ParamWidgetBase,
    display: &UiDisplay,
    step_set: StepSet,
    ui_norm: f32,
) {
    set_from_ui_norm(cx, param_base, step_set, ui_norm);
    set_readout_from_ui_norm(display, param_base, step_set, ui_norm);
}

pub fn param_drag_begin_horizontal(
    session: &mut ParamDragSession,
    cx: &mut EventContext,
    param_base: &ParamWidgetBase,
    display: &UiDisplay,
    step_set: StepSet,
    ui_norm: f32,
    filter_preview: bool,
) {
    session.active = true;
    if filter_preview {
        display.set_filter_preview_drag(true);
    }
    cx.capture();
    cx.focus();
    param_base.begin_set_parameter(cx);
    apply_drag_ui_norm(cx, param_base, display, step_set, ui_norm);
}

pub fn param_drag_begin_vertical(
    session: &mut ParamDragSession,
    cx: &mut EventContext,
    param_base: &ParamWidgetBase,
    display: &UiDisplay,
    step_set: StepSet,
    start_y: f32,
) {
    session.active = true;
    session.start_y = start_y;
    session.start_norm = ui_normalized(param_base, step_set);
    cx.capture();
    cx.focus();
    param_base.begin_set_parameter(cx);
    set_readout_from_ui_norm(display, param_base, step_set, session.start_norm);
}

pub fn param_drag_end(
    session: &mut ParamDragSession,
    cx: &mut EventContext,
    param_base: &ParamWidgetBase,
    display: &UiDisplay,
    step_set: StepSet,
    filter_preview: bool,
) -> bool {
    if !session.active {
        return false;
    }
    session.active = false;
    if filter_preview {
        display.set_filter_preview_drag(false);
    }
    if step_set != StepSet::None {
        step_points::snap_param(cx, param_base, step_set);
    }
    display.clear_param_readout();
    cx.release();
    param_base.end_set_parameter(cx);
    cx.needs_redraw();
    true
}

pub fn param_drag_move_horizontal(
    cx: &mut EventContext,
    param_base: &ParamWidgetBase,
    display: &UiDisplay,
    step_set: StepSet,
    ui_norm: f32,
) {
    apply_drag_ui_norm(cx, param_base, display, step_set, ui_norm);
    cx.needs_redraw();
}

pub fn param_drag_move_vertical(
    session: &ParamDragSession,
    cx: &mut EventContext,
    param_base: &ParamWidgetBase,
    display: &UiDisplay,
    step_set: StepSet,
    y: f32,
    sensitivity: f32,
    fine_sensitivity: f32,
) {
    let sens = if cx.modifiers().shift() {
        fine_sensitivity
    } else {
        sensitivity
    };
    let ui_norm = (session.start_norm + (session.start_y - y) * sens).clamp(0.0, 1.0);
    apply_drag_ui_norm(cx, param_base, display, step_set, ui_norm);
    cx.needs_redraw();
}
