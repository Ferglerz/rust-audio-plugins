use super::*;

impl ScdEditorView {
    pub(super) fn handle_note_edit_event(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        mouse_x: f32,
        mouse_y: f32,
    ) -> bool {
        if self.note_edit.is_none() {
            return false;
        }
        match window_event {
            WindowEvent::CharInput(ch) if ch.is_ascii_digit() && !cx.modifiers().command() => {
                if let Some(edit) = self.note_edit.as_mut() {
                    insert_note_digit(edit, *ch);
                }
                consume_redraw(cx, window_event, meta);
                return true;
            }
            WindowEvent::KeyDown(code, key) => {
                match *code {
                    Code::Enter | Code::NumpadEnter => self.commit_note_edit(),
                    Code::Escape => self.cancel_note_edit(),
                    _ => {
                        if let Some(edit) = self.note_edit.as_mut() {
                            edit.handle_key(cx, *code);
                            let has_char = matches!(key, Some(Key::Character(_)));
                            if !has_char && !cx.modifiers().command() {
                                if let Some(c) = typed_char(*code, cx.modifiers().shift()) {
                                    insert_note_digit(edit, c);
                                }
                            }
                        }
                    }
                }
                consume_redraw(cx, window_event, meta);
                return true;
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                if self
                    .note_edit
                    .as_ref()
                    .is_some_and(|edit| Self::note_edit_hits(edit, mouse_x, mouse_y))
                {
                    if let Some(edit) = self.note_edit.as_mut() {
                        edit.handle_mouse_down(mouse_x);
                    }
                    consume_redraw(cx, window_event, meta);
                    return true;
                }
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                if self
                    .note_edit
                    .as_ref()
                    .is_some_and(|edit| Self::note_edit_hits(edit, mouse_x, mouse_y))
                {
                    if let Some(edit) = self.note_edit.as_mut() {
                        edit.select_all();
                    }
                    consume_redraw(cx, window_event, meta);
                    return true;
                }
            }
            WindowEvent::FocusOut => {
                self.commit_note_edit();
                cx.needs_redraw();
            }
            _ => {}
        }
        false
    }

    pub(super) fn handle_add_preset_mouse_down(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        x: f32,
        y: f32,
    ) -> bool {
        if !self.add_preset_open {
            return false;
        }
        let (mx, my, mw, mh) = Self::add_preset_modal();
        if Self::hit(x, y, mx, my, mw, mh) {
            let name_r = Self::add_name_rect((mx, my, mw, mh));
            self.add_name_focus = Self::hit(x, y, name_r.0, name_r.1, name_r.2, name_r.3);
            for i in 0..PresetScope::names().len() {
                let r = Self::add_scope_rect((mx, my, mw, mh), i);
                if Self::hit(x, y, r.0, r.1, r.2, r.3) {
                    self.add_scope.toggle(i);
                    consume_redraw(cx, window_event, meta);
                    return true;
                }
            }
            let save = Self::add_save_rect((mx, my, mw, mh));
            if Self::hit(x, y, save.0, save.1, save.2, save.3) {
                self.save_user_preset(cx);
                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                return true;
            }
            let cancel = Self::add_cancel_rect((mx, my, mw, mh));
            if Self::hit(x, y, cancel.0, cancel.1, cancel.2, cancel.3) {
                self.add_preset_open = false;
                self.add_name_focus = false;
                consume_redraw(cx, window_event, meta);
                return true;
            }
            nih_plug_vizia::consume_window_event(cx, window_event, meta);
            true
        } else {
            self.add_preset_open = false;
            self.add_name_focus = false;
            consume_redraw(cx, window_event, meta);
            true
        }
    }

    pub(super) fn handle_sub_kick_mouse_down(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        x: f32,
        y: f32,
    ) -> bool {
        if !self.sub_kick_open {
            return false;
        }
        let (modal_x, modal_y, modal_w, modal_h) = Self::sub_kick_modal();
        if Self::hit(x, y, modal_x, modal_y, modal_w, modal_h) {
            if cx.modifiers().alt() && self.reset_control_at(cx, x, y) {
                consume_redraw(cx, window_event, meta);
                return true;
            }
            let knob_y = modal_y + 55.0;
            let knob_radius: f32 = 30.0;
            for k_idx in 0..5 {
                let kx = Self::sub_kick_knob_x(modal_x, k_idx);
                let dist_sq = (x - kx).powi(2) + (y - knob_y).powi(2);
                if dist_sq <= knob_radius.powi(2) {
                    let ptr = match k_idx {
                        0 => self.params.sub_kick.vol.as_ptr(),
                        1 => self.params.sub_kick.length.as_ptr(),
                        2 => self.params.sub_kick.dive.as_ptr(),
                        3 => self.params.sub_kick.speed.as_ptr(),
                        _ => self.params.sub_kick.offset.as_ptr(),
                    };
                    self.drag = Some(match k_idx {
                        0 => DragTarget::SubKickVol,
                        1 => DragTarget::SubKickLength,
                        2 => DragTarget::SubKickDive,
                        3 => DragTarget::SubKickSpeed,
                        _ => DragTarget::SubKickOffset,
                    });
                    self.begin_one(cx, ptr);
                    cx.capture();
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                    return true;
                }
            }
            // Empty modal space must not fall through to the mixer.
            nih_plug_vizia::consume_window_event(cx, window_event, meta);
            true
        } else {
            self.sub_kick_open = false;
            consume_redraw(cx, window_event, meta);
            true
        }
    }

    pub(super) fn handle_add_preset_key_event(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
    ) -> bool {
        if !self.add_preset_open {
            return false;
        }
        match window_event {
            WindowEvent::CharInput(ch) if self.add_name_focus && !ch.is_control() => {
                if self.add_name.len() < 32 {
                    self.add_name.push(*ch);
                    cx.needs_redraw();
                }
                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                true
            }
            WindowEvent::KeyDown(code, _) => {
                match *code {
                    Code::Backspace if self.add_name_focus => {
                        self.add_name.pop();
                        cx.needs_redraw();
                    }
                    Code::Enter | Code::NumpadEnter => self.save_user_preset(cx),
                    Code::Escape => {
                        self.add_preset_open = false;
                        self.add_name_focus = false;
                        cx.needs_redraw();
                    }
                    _ => {}
                }
                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                true
            }
            _ => false,
        }
    }
}
