use super::{
    controls::{self, hit, value_rect, Hit},
    prefs, Arm, ControlId, Drag, FundamentView, VIEW_H, VIEW_W,
};
use crate::params::WaveformParam;
use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::util::ModifiersExt;
use pleasant_ui::draw::EditorViewport;
use pleasant_ui::readout::{self, EditAction, PressAction};

impl FundamentView {
    pub(super) fn handle_event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| {
            let bounds = cx.bounds();
            let Some(viewport) =
                EditorViewport::fit((bounds.x, bounds.y, bounds.w, bounds.h), (VIEW_W, VIEW_H))
            else {
                return;
            };
            let (mouse_x, mouse_y) = viewport.to_local(cx.mouse().cursorx, cx.mouse().cursory);

            if let WindowEvent::MouseScroll(_, dy) = window_event {
                if *dy != 0.0 {
                    if let Some(id) = controls::control_at(mouse_x, mouse_y) {
                        if self.edit.is_some() {
                            self.commit_edit(cx);
                        }
                        self.end_gesture(cx);
                        let norm = self.wheel_norm(id, *dy, cx.modifiers().shift());
                        self.emit_once(cx, id, norm);
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        cx.needs_redraw();
                        return;
                    }
                }
            }

            match readout::handle_press(&mut self.value_press, window_event, (mouse_x, mouse_y)) {
                PressAction::Inactive => {}
                PressAction::Blocked => return,
                PressAction::BeginDrag(press) => {
                    self.begin_drag(cx, press.target, press.origin.1);
                }
                PressAction::BeginEdit(press) => {
                    cx.release();
                    let value = self.control_value(press.target);
                    self.start_edit(cx, press.target, press.rect, value);
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                    cx.needs_redraw();
                    return;
                }
                PressAction::Released => {
                    cx.release();
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                    cx.needs_redraw();
                    return;
                }
                PressAction::Cancelled => {
                    cx.release();
                    cx.needs_redraw();
                    return;
                }
            }

            if let Some(edit) = self.edit.as_mut() {
                let action = readout::handle_edit(edit, cx, window_event, (mouse_x, mouse_y));
                match action {
                    EditAction::Blocked => return,
                    EditAction::Handled => {}
                    EditAction::Commit | EditAction::CommitAndContinue => self.commit_edit(cx),
                    EditAction::Cancel => self.edit = None,
                }
                if action != EditAction::CommitAndContinue {
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                    cx.needs_redraw();
                    return;
                }
            }

            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    self.press(cx, mouse_x, mouse_y);
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                    cx.needs_redraw();
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    self.arm = None;
                    self.end_gesture(cx);
                    cx.release();
                    if let Some(Hit::Body(id)) = hit(mouse_x, mouse_y) {
                        let norm = self.control_default_norm(id);
                        self.emit_once(cx, id, norm);
                    }
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                    cx.needs_redraw();
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    self.finish_pointer(cx, true);
                    cx.needs_redraw();
                }
                WindowEvent::FocusOut => {
                    self.finish_pointer(cx, false);
                    cx.needs_redraw();
                }
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some((mouse_x, mouse_y));
                    if let Some(arm) = self.arm {
                        if (mouse_y - arm.origin_y).abs() >= 4.0 {
                            self.begin_drag(cx, arm.id, arm.origin_y);
                        }
                    }
                    if let Some(drag) = self.drag {
                        let speed = if cx.modifiers().shift() {
                            0.0004
                        } else {
                            0.004
                        };
                        let norm =
                            (drag.start_norm + (drag.start_y - mouse_y) * speed).clamp(0.0, 1.0);
                        if let Some(ptr) = self.gesture {
                            pleasant_ui::param::set_normalized(cx, ptr, norm);
                        }
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseLeave => {
                    self.hover = None;
                    cx.needs_redraw();
                }
                _ => {}
            }
        });
    }

    fn press(&mut self, cx: &mut EventContext, x: f32, y: f32) {
        match hit(x, y) {
            Some(Hit::Theme) => {
                prefs().toggle();
            }
            Some(Hit::Bypass) => {
                let next = if self.params.bypass.value() { 0.0 } else { 1.0 };
                pleasant_ui::param::set_normalized_once(cx, self.params.bypass.as_ptr(), next);
                self.bypass_anim.trigger_click();
            }
            Some(Hit::Value(id)) => {
                let slot = controls::control_slot(
                    controls::CONTROLS
                        .iter()
                        .position(|item| *item == id)
                        .unwrap_or(0),
                );
                self.value_press = Some(pleasant_ui::pointer::ValuePress::new(
                    id,
                    value_rect(slot),
                    (x, y),
                ));
                cx.focus();
                cx.capture();
            }
            Some(Hit::Body(id)) => {
                self.arm = Some(Arm { id, origin_y: y });
                cx.focus();
                cx.capture();
            }
            None => {}
        }
    }

    fn start_edit(
        &mut self,
        cx: &mut EventContext,
        id: ControlId,
        rect: (f32, f32, f32, f32),
        value: String,
    ) {
        cx.focus();
        self.edit = Some(pleasant_ui::value_edit::ValueEdit::new(id, rect, value));
    }

    fn commit_edit(&mut self, cx: &mut EventContext) {
        if let Some(edit) = self.edit.take() {
            if let Some(norm) = self.control_from_text(edit.target, &edit.text) {
                self.emit_once(cx, edit.target, norm);
            }
        }
    }

    fn begin_drag(&mut self, cx: &mut EventContext, id: ControlId, start_y: f32) {
        self.end_gesture(cx);
        let ptr = self.control_ptr(id);
        pleasant_ui::param::begin(cx, ptr);
        self.gesture = Some(ptr);
        self.drag = Some(Drag {
            start_y,
            start_norm: self.control_norm(id),
        });
    }

    fn end_gesture(&mut self, cx: &mut EventContext) {
        self.drag = None;
        self.arm = None;
        if let Some(ptr) = self.gesture.take() {
            pleasant_ui::param::end(cx, ptr);
        }
    }

    fn finish_pointer(&mut self, cx: &mut EventContext, allow_cycle: bool) {
        let cycle = allow_cycle && self.arm.is_some_and(|arm| arm.id == ControlId::Waveform);
        self.arm = None;
        self.end_gesture(cx);
        cx.release();
        if cycle {
            self.cycle_waveform(cx);
        }
    }

    fn emit_once(&mut self, cx: &mut EventContext, id: ControlId, norm: f32) {
        self.end_gesture(cx);
        pleasant_ui::param::set_normalized_once(cx, self.control_ptr(id), norm);
    }

    fn cycle_waveform(&mut self, cx: &mut EventContext) {
        let next = match self.params.waveform.value() {
            WaveformParam::Sine => WaveformParam::Saw,
            WaveformParam::Saw => WaveformParam::Square,
            WaveformParam::Square => WaveformParam::Triangle,
            WaveformParam::Triangle => WaveformParam::Sine,
        };
        let norm = self.params.waveform.preview_normalized(next);
        self.emit_once(cx, ControlId::Waveform, norm);
    }

    fn wheel_norm(&self, id: ControlId, dy: f32, fine: bool) -> f32 {
        let up = dy > 0.0;
        match id {
            ControlId::Voices => Self::step_int(&self.params.max_voices, up, fine),
            ControlId::Harmonics => Self::step_int(&self.params.harmonics, up, fine),
            ControlId::Waveform => {
                let param = &self.params.waveform;
                let plain = param.value();
                let next = if up {
                    param.next_step(plain, fine)
                } else {
                    param.previous_step(plain, fine)
                };
                param.preview_normalized(next)
            }
            _ => {
                let step = if fine { 0.001 } else { 0.01 };
                (self.control_norm(id) + dy * step).clamp(0.0, 1.0)
            }
        }
    }
}
