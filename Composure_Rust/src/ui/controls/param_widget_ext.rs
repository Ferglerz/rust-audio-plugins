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
        step_points::value_to_step_norm(param_base.unmodulated_plain_value() as f64, steps)
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

pub fn set_from_ui_norm(
    cx: &mut EventContext,
    param_base: &ParamWidgetBase,
    step_set: StepSet,
    ui_norm: f32,
) {
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

/// Pleasant readouts share the same text editing and hover affordance as the other plugins.
#[derive(Default)]
pub struct ParamValueEdit {
    edit: Option<pleasant_ui::value_edit::ValueEdit<()>>,
    hovered: bool,
    press: Option<pleasant_ui::pointer::ValuePress<()>>,
    readout_drag: Option<((f32, f32), f32)>,
}

impl ParamValueEdit {
    pub fn draw(
        &self,
        cx: &mut nih_plug_vizia::vizia::prelude::DrawContext,
        canvas: &mut nih_plug_vizia::vizia::prelude::Canvas,
        control: super::appearance::Control,
    ) {
        let b = cx.bounds();
        let scale = cx.scale_factor();
        let rect = super::appearance::value_rect(control, b.w / scale, b.h / scale);
        let mut d = super::appearance::painter(cx, canvas, b.w / scale);
        if let Some(edit) = &self.edit {
            d.value_edit(edit, pleasant_ui::GOLD);
        } else if self.hovered {
            let baseline = match control {
                super::appearance::Control::Knob => {
                    pleasant_ui::draw::KnobLayout::new((0.0, 0.0, b.w / scale, b.h / scale))
                        .with_text_sizes(13.0, 15.0)
                        .value_y
                }
                _ => 14.0,
            };
            d.value_underline(rect, baseline, pleasant_ui::GOLD);
        }
    }

    fn commit(&mut self, cx: &mut EventContext, param: &ParamWidgetBase) {
        let Some(edit) = &mut self.edit else { return };
        if let Some(norm) = param
            .string_to_normalized_value(&edit.text)
            .filter(|n| n.is_finite())
        {
            param.begin_set_parameter(cx);
            param.set_normalized_value(cx, norm.clamp(0.0, 1.0));
            param.end_set_parameter(cx);
            self.edit = None;
        } else {
            edit.invalid = true;
        }
        cx.needs_redraw();
    }

    pub fn event(
        &mut self,
        cx: &mut EventContext,
        event: &nih_plug_vizia::vizia::prelude::WindowEvent,
        param: &ParamWidgetBase,
        control: super::appearance::Control,
    ) -> bool {
        let was_editing = self.edit.is_some();
        let handled = self.handle_event(cx, event, param, control);
        pleasant_ui::value_edit::sync_text_input(cx, was_editing, self.edit.is_some());
        if handled {
            nih_plug_vizia::report_handled_key(cx, event);
        }
        handled
    }

    fn handle_event(
        &mut self,
        cx: &mut EventContext,
        event: &nih_plug_vizia::vizia::prelude::WindowEvent,
        param: &ParamWidgetBase,
        control: super::appearance::Control,
    ) -> bool {
        use nih_plug_vizia::vizia::prelude::*;
        use pleasant_ui::value_edit::{typed_char, ValueEdit};

        let b = cx.bounds();
        let scale = cx.scale_factor();
        let rect = super::appearance::value_rect(control, b.w / scale, b.h / scale);
        let x = (cx.mouse().cursorx - b.x) / scale;
        let y = (cx.mouse().cursory - b.y) / scale;
        let inside = x >= rect.0 && x <= rect.0 + rect.2 && y >= rect.1 && y <= rect.1 + rect.3;
        if let Some((origin, norm)) = self.readout_drag {
            match event {
                WindowEvent::MouseMove(_, _) => {
                    let delta = pleasant_ui::pointer::readout_drag_delta(
                        x - origin.0,
                        y - origin.1,
                        cx.modifiers().shift(),
                    );
                    param.set_normalized_value(cx, (norm + delta).clamp(0.0, 1.0));
                    cx.needs_redraw();
                }
                WindowEvent::MouseUp(MouseButton::Left) | WindowEvent::FocusOut => {
                    self.readout_drag = None;
                    param.end_set_parameter(cx);
                    cx.release();
                    cx.needs_redraw();
                }
                _ => {}
            }
            return true;
        }
        if let WindowEvent::MouseScroll(_, dy) = event {
            if inside && *dy != 0.0 {
                if self.edit.is_some() {
                    self.commit(cx, param);
                    if self.edit.is_some() {
                        return true;
                    }
                }
                let delta = *dy * if cx.modifiers().shift() { 0.001 } else { 0.01 };
                let norm = (param.unmodulated_normalized_value() + delta).clamp(0.0, 1.0);
                param.begin_set_parameter(cx);
                param.set_normalized_value(cx, norm);
                param.end_set_parameter(cx);
                cx.needs_redraw();
                return true;
            }
        }
        if let Some(mut press) = self.press {
            match event {
                WindowEvent::MouseMove(_, _) => {
                    if press.update(x, y) {
                        self.press = None;
                        let norm = param.unmodulated_normalized_value();
                        self.readout_drag = Some((press.origin, norm));
                        param.begin_set_parameter(cx);
                        let delta = pleasant_ui::pointer::readout_drag_delta(
                            x - press.origin.0,
                            y - press.origin.1,
                            cx.modifiers().shift(),
                        );
                        param.set_normalized_value(cx, (norm + delta).clamp(0.0, 1.0));
                        cx.needs_redraw();
                        return true;
                    }
                    self.press = Some(press);
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    self.press = None;
                    cx.release();
                    if press.released_as_click(x, y) {
                        self.edit = Some(ValueEdit::new(
                            (),
                            rect,
                            param.normalized_value_to_string(
                                param.unmodulated_normalized_value(),
                                true,
                            ),
                        ));
                    }
                    cx.needs_redraw();
                }
                WindowEvent::FocusOut
                | WindowEvent::KeyDown(Code::Escape, _)
                | WindowEvent::MouseDown(MouseButton::Right) => {
                    self.press = None;
                    cx.release();
                    cx.needs_redraw();
                }
                _ => {}
            }
            return true;
        }
        match event {
            WindowEvent::MouseMove(_, _) | WindowEvent::MouseLeave => {
                let hovered = inside && !matches!(event, WindowEvent::MouseLeave);
                if self.hovered != hovered {
                    self.hovered = hovered;
                    cx.needs_redraw();
                }
            }
            WindowEvent::MouseDown(MouseButton::Left) if cx.modifiers().command() => {
                self.edit = None;
                return false;
            }
            WindowEvent::MouseDown(MouseButton::Left) if inside => {
                cx.focus();
                if let Some(edit) = &mut self.edit {
                    edit.handle_mouse_down(x);
                } else {
                    self.press = Some(pleasant_ui::pointer::ValuePress::new((), rect, (x, y)));
                    cx.capture();
                }
                cx.needs_redraw();
                return true;
            }
            WindowEvent::MouseDown(MouseButton::Left) if self.edit.is_some() => {
                self.commit(cx, param);
                return true;
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) if self.edit.is_some() => {
                if let Some(edit) = &mut self.edit {
                    edit.select_all();
                }
                cx.needs_redraw();
                return true;
            }
            WindowEvent::CharInput(c) if self.edit.is_some() => {
                if !cx.modifiers().command() && c.is_ascii() && !c.is_control() {
                    if let Some(edit) = &mut self.edit {
                        edit.insert(&c.to_string());
                    }
                }
                cx.needs_redraw();
                return true;
            }
            WindowEvent::KeyDown(code, key) if self.edit.is_some() => {
                match code {
                    Code::Enter | Code::NumpadEnter => self.commit(cx, param),
                    Code::Escape => self.edit = None,
                    _ => {
                        if let Some(edit) = &mut self.edit {
                            edit.handle_key(cx, *code);
                            if !matches!(key, Some(Key::Character(_))) && !cx.modifiers().command()
                            {
                                if let Some(c) = typed_char(*code, cx.modifiers().shift()) {
                                    edit.insert(&c.to_string());
                                }
                            }
                        }
                    }
                }
                cx.needs_redraw();
                return true;
            }
            WindowEvent::FocusOut => {
                self.commit(cx, param);
                self.edit = None;
            }
            _ => {}
        }
        false
    }
}
