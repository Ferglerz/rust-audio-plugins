use super::{controls::*, *};

impl TapeStopView {
    pub(super) fn handle_event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| {
            let bounds = cx.bounds();
            let scale = bounds.w / UI_W;
            let mouse_x = (cx.mouse().cursorx - bounds.x) / scale;
            let mouse_y = (cx.mouse().cursory - bounds.y) / scale;

            if let WindowEvent::MouseScroll(_, dy) = window_event {
                if *dy != 0.0 {
                    if let Some(id) = self.readout_at(mouse_x, mouse_y) {
                        if self.edit.is_some() {
                            self.commit_edit(cx);
                        }
                        let step = if matches!(id, KnobId::OverrideCc | KnobId::OverrideNote) {
                            dy.signum() / 127.0
                        } else {
                            *dy * if cx.modifiers().shift() { 0.001 } else { 0.01 }
                        };
                        self.emit_knob_norm(
                            cx,
                            id,
                            (self.get_knob_norm(id) + step).clamp(0.0, 1.0),
                        );
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        cx.needs_redraw();
                        return;
                    }
                }
            }

            if let Some(mut press) = self.value_press {
                match window_event {
                    WindowEvent::MouseMove(_, _) => {
                        if press.update(mouse_x, mouse_y) {
                            self.value_press = None;
                            self.drag = Some(DragState::Value {
                                id: press.target,
                                start_x: press.origin.0,
                                start_y: press.origin.1,
                                start_norm: self.get_knob_norm(press.target),
                            });
                        } else {
                            self.value_press = Some(press);
                            return;
                        }
                    }
                    WindowEvent::MouseUp(MouseButton::Left) => {
                        self.value_press = None;
                        cx.release();
                        if press.released_as_click(mouse_x, mouse_y) {
                            let (_, value, _) = self.knob_info(press.target);
                            self.start_edit(cx, press.target, press.rect, value);
                        }
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::FocusOut
                    | WindowEvent::KeyDown(Code::Escape, _)
                    | WindowEvent::MouseDown(MouseButton::Right) => {
                        self.value_press = None;
                        cx.release();
                        cx.needs_redraw();
                        return;
                    }
                    _ => return,
                }
            }
            if self.edit.is_some() {
                match window_event {
                    WindowEvent::CharInput(c) => {
                        if !cx.modifiers().command() && c.is_ascii() && !c.is_control() {
                            if let Some(edit) = self.edit.as_mut() {
                                edit.insert(&c.to_string());
                            }
                        }
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::KeyDown(code, key) => {
                        match code {
                            Code::Enter | Code::NumpadEnter => {
                                self.commit_edit(cx);
                            }
                            Code::Escape => {
                                self.edit = None;
                            }
                            _ => {
                                self.edit.as_mut().unwrap().handle_key(cx, *code);
                                let has_char = matches!(key, Some(Key::Character(_)));
                                if !has_char && !cx.modifiers().command() {
                                    if let Some(c) = typed_char(*code, cx.modifiers().shift()) {
                                        self.edit.as_mut().unwrap().insert(&c.to_string());
                                    }
                                }
                            }
                        }
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::MouseDown(MouseButton::Left) => {
                        let edit_rect = self.edit.as_ref().unwrap().rect;
                        if Self::inside(mouse_x, mouse_y, edit_rect) {
                            self.edit.as_mut().unwrap().handle_mouse_down(mouse_x);
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            cx.needs_redraw();
                            return;
                        }
                        self.commit_edit(cx);
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                        self.edit.as_mut().unwrap().select_all();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::FocusOut => {
                        self.commit_edit(cx);
                    }
                    _ => return,
                }
            }

            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    // Circular STOP / REW overlay at the bottom-left of the graph
                    let (sx, sy, sw, sh) = STOP_BUTTON;
                    let scx = sx + sw * 0.5;
                    let scy = sy + sh * 0.5;
                    let dx = mouse_x - scx;
                    let dy = mouse_y - scy;
                    if dx * dx + dy * dy <= (sw * 0.5) * (sw * 0.5) {
                        let is_braking = self.telemetry.is_braking.load(Ordering::Relaxed);
                        self.telemetry.set_manual_trigger(!is_braking);
                        self.stop_click.set(1.0);
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, THEME_BUTTON) {
                        prefs().toggle();
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, cog_rect()) {
                        self.show_axis_controls = !self.show_axis_controls;
                        cx.needs_redraw();
                        return;
                    }

                    if self.show_axis_controls
                        && Self::inside(mouse_x, mouse_y, trigger_bypass_rect())
                    {
                        let next = if self.params.auto_restart.value() {
                            0.0
                        } else {
                            1.0
                        };
                        self.emit_param_norm(cx, self.params.auto_restart.as_ptr(), next);
                        self.auto_restart_anim.trigger_click();
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, DROP_TIME_READOUT) {
                        self.press_value(
                            cx,
                            KnobId::DropTime,
                            DROP_TIME_READOUT,
                            (mouse_x, mouse_y),
                        );
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, DROP_TIME_SLIDER) {
                        let bar_r = Self::drop_bar_rect();
                        let new_norm = (1.0 - (mouse_y - bar_r.1) / bar_r.3).clamp(0.0, 1.0);
                        self.emit_knob_norm(cx, KnobId::DropTime, new_norm);
                        self.drag = Some(DragState::Slider {
                            id: KnobId::DropTime,
                            start_x: mouse_x,
                            start_y: mouse_y,
                            start_norm: new_norm,
                            vertical: true,
                        });
                        cx.needs_redraw();
                        return;
                    }

                    if self.show_axis_controls {
                        if Self::inside(mouse_x, mouse_y, midi_label_rect()) {
                            let next = match self.midi_assign() {
                                MidiAssign::Cc => 1.0,
                                MidiAssign::Note => 0.0,
                            };
                            self.emit_param_norm(cx, self.params.midi_assign.as_ptr(), next);
                            cx.needs_redraw();
                            return;
                        }
                        if Self::inside(mouse_x, mouse_y, midi_value_rect()) {
                            let id = self.midi_number_id();
                            self.press_value(cx, id, midi_value_rect(), (mouse_x, mouse_y));
                            cx.needs_redraw();
                            return;
                        }
                        for &id in &[KnobId::Return, KnobId::Xfade, KnobId::StereoDiv] {
                            let r = Self::slider_rect(id);
                            let val_r = axis_value_rect(r);
                            let bar_r = axis_bar_rect(r);
                            if Self::inside(mouse_x, mouse_y, val_r) {
                                self.press_value(cx, id, val_r, (mouse_x, mouse_y));
                                cx.needs_redraw();
                                return;
                            }
                            if Self::inside(mouse_x, mouse_y, r) {
                                let new_norm = ((mouse_x - bar_r.0) / bar_r.2).clamp(0.0, 1.0);
                                self.emit_knob_norm(cx, id, new_norm);
                                self.drag = Some(DragState::Slider {
                                    id,
                                    start_x: mouse_x,
                                    start_y: mouse_y,
                                    start_norm: new_norm,
                                    vertical: false,
                                });
                                cx.needs_redraw();
                                return;
                            }
                        }
                    }

                    if self.show_axis_controls && self.knob_enabled(KnobId::RestartThresh) {
                        let val_r = trigger_value_rect();
                        if Self::inside(mouse_x, mouse_y, val_r) {
                            self.press_value(cx, KnobId::RestartThresh, val_r, (mouse_x, mouse_y));
                            cx.needs_redraw();
                            return;
                        }
                        if Self::inside(mouse_x, mouse_y, trigger_column())
                            && !Self::inside(mouse_x, mouse_y, trigger_bypass_rect())
                        {
                            let bar_r = trigger_bar_rect();
                            let new_norm = ((mouse_x - bar_r.0) / bar_r.2).clamp(0.0, 1.0);
                            self.emit_knob_norm(cx, KnobId::RestartThresh, new_norm);
                            self.drag = Some(DragState::Slider {
                                id: KnobId::RestartThresh,
                                start_x: mouse_x,
                                start_y: mouse_y,
                                start_norm: new_norm,
                                vertical: false,
                            });
                            cx.needs_redraw();
                            return;
                        }
                    }

                    if Self::hit_curve_node(mouse_x, mouse_y) {
                        self.drag = Some(DragState::Curve {
                            start_x: mouse_x,
                            start_y: mouse_y,
                            start_norm: self.get_knob_norm(KnobId::Curve),
                        });
                        cx.needs_redraw();
                    }
                }

                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    if Self::hit_curve_node(mouse_x, mouse_y) {
                        self.reset_knob(cx, KnobId::Curve);
                        self.drag = None;
                        cx.needs_redraw();
                        return;
                    }
                    if Self::inside(mouse_x, mouse_y, DROP_TIME_SLIDER) {
                        self.reset_knob(cx, KnobId::DropTime);
                        self.drag = None;
                        cx.needs_redraw();
                        return;
                    }
                    if self.show_axis_controls {
                        for &id in &[KnobId::Return, KnobId::Xfade, KnobId::StereoDiv] {
                            let r = Self::slider_rect(id);
                            let val_r = axis_value_rect(r);
                            if Self::inside(mouse_x, mouse_y, val_r) {
                                continue;
                            }
                            if Self::inside(mouse_x, mouse_y, r) {
                                self.reset_knob(cx, id);
                                self.drag = None;
                                cx.needs_redraw();
                                return;
                            }
                        }
                    }
                    if self.show_axis_controls
                        && self.knob_enabled(KnobId::RestartThresh)
                        && Self::inside(mouse_x, mouse_y, trigger_column())
                        && !Self::inside(mouse_x, mouse_y, trigger_value_rect())
                        && !Self::inside(mouse_x, mouse_y, trigger_bypass_rect())
                    {
                        self.reset_knob(cx, KnobId::RestartThresh);
                        self.drag = None;
                        cx.needs_redraw();
                    }
                }

                WindowEvent::MouseUp(MouseButton::Left) | WindowEvent::FocusOut => {
                    self.drag = None;
                    cx.release();
                    cx.needs_redraw();
                }

                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some((mouse_x, mouse_y));
                    cx.needs_redraw();

                    if self.drag.is_none() {
                        let (sx, sy, sw, sh) = STOP_BUTTON;
                        let scx = sx + sw * 0.5;
                        let scy = sy + sh * 0.5;
                        let dx = mouse_x - scx;
                        let dy = mouse_y - scy;
                        let hover_stop = dx * dx + dy * dy <= (sw * 0.5 + 4.0) * (sw * 0.5 + 4.0);
                        if hover_stop != self.hover_stop.get() {
                            self.hover_stop.set(hover_stop);
                            cx.needs_redraw();
                        }
                    }

                    match self.drag {
                        Some(DragState::Value {
                            id,
                            start_x,
                            start_y,
                            start_norm,
                        }) => {
                            let delta = pleasant_ui::pointer::readout_drag_delta(
                                mouse_x - start_x,
                                mouse_y - start_y,
                                cx.modifiers().shift(),
                            );
                            self.emit_knob_norm(cx, id, (start_norm + delta).clamp(0.0, 1.0));
                            cx.needs_redraw();
                        }
                        Some(DragState::Slider {
                            id,
                            start_x,
                            start_y,
                            start_norm,
                            vertical,
                        }) => {
                            let bar_r = match id {
                                KnobId::DropTime => Self::drop_bar_rect(),
                                KnobId::RestartThresh => trigger_bar_rect(),
                                _ => axis_bar_rect(Self::slider_rect(id)),
                            };
                            let (delta, span) = if vertical {
                                (start_y - mouse_y, bar_r.3)
                            } else {
                                (mouse_x - start_x, bar_r.2)
                            };
                            let step = if cx.modifiers().shift() {
                                0.001
                            } else {
                                1.0 / span.max(1.0)
                            };
                            let new_norm = (start_norm + delta * step).clamp(0.0, 1.0);
                            self.emit_knob_norm(cx, id, new_norm);
                        }
                        Some(DragState::Curve {
                            start_x,
                            start_y,
                            start_norm,
                        }) => {
                            let dx = mouse_x - start_x;
                            let dy = start_y - mouse_y;
                            let step = if cx.modifiers().shift() { 0.001 } else { 0.004 };
                            let new_norm = (start_norm + (dx + dy) * step).clamp(0.0, 1.0);
                            self.emit_knob_norm(cx, KnobId::Curve, new_norm);
                            cx.needs_redraw();
                        }
                        None => {}
                    }
                }

                WindowEvent::MouseLeave => {
                    self.hover = None;
                    self.hover_stop.set(false);
                    cx.needs_redraw();
                }

                _ => {}
            }
        });
    }
}
