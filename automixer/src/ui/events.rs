use super::*;

impl AutomixerView {
    fn end_drag(&mut self, cx: &mut EventContext) {
        if let Some((id, _, _)) = self.drag.take() {
            param::end(cx, self.parameter(id).ptr());
        }
        self.press = None;
        cx.release();
    }
    fn commit(&mut self, cx: &mut EventContext) {
        let Some(mut edit) = self.edit.take() else {
            return;
        };
        match edit.target {
            Target::Parameter(id) => {
                if let Some(value) = self
                    .parameter(id)
                    .parse(&edit.text)
                    .filter(|v| v.is_finite())
                {
                    self.prepare_edit(id, value);
                    param::set_normalized_once(cx, self.parameter(id).ptr(), value);
                } else {
                    edit.invalid = true;
                    self.edit = Some(edit);
                }
            }
            Target::Name(i) => {
                let name = edit.text.trim().chars().take(12).collect::<String>();
                *self.params.shared.mics[i]
                    .name
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = name;
            }
        }
    }
    pub(super) fn handle_event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window: &WindowEvent, meta| {
            let b = cx.bounds();
            let Some(viewport) = EditorViewport::fit((b.x, b.y, b.w, b.h), SIZE) else {
                return;
            };
            let point = viewport.to_local(cx.mouse().cursorx, cx.mouse().cursory);
            match readout::handle_press(&mut self.press, window, point) {
                PressAction::Inactive => {}
                PressAction::Blocked => return,
                PressAction::BeginDrag(press) => {
                    let initial = self.normalized(press.target);
                    param::begin(cx, self.parameter(press.target).ptr());
                    self.drag = Some((press.target, press.origin, initial));
                }
                PressAction::BeginEdit(press) => {
                    cx.release();
                    self.edit = Some(ValueEdit::new(
                        Target::Parameter(press.target),
                        press.rect,
                        self.parameter(press.target)
                            .text(self.normalized(press.target), false),
                    ));
                    nih_plug_vizia::consume_window_event(cx, window, meta);
                    cx.needs_redraw();
                    return;
                }
                PressAction::Released | PressAction::Cancelled => {
                    cx.release();
                }
            }
            if let Some(edit) = &mut self.edit {
                match readout::handle_edit(edit, cx, window, point) {
                    EditAction::Commit => {
                        self.commit(cx);
                    }
                    EditAction::CommitAndContinue => {
                        self.commit(cx);
                        self.end_drag(cx);
                    }
                    EditAction::Cancel => {
                        self.edit = None;
                    }
                    EditAction::Handled => {}
                    EditAction::Blocked => return,
                }
                nih_plug_vizia::consume_window_event(cx, window, meta);
                cx.needs_redraw();
                return;
            }
            match window {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    if self.menu.is_some() {
                        if let Some(index) = self.menu_index_at(point) {
                            self.choose_menu(cx, index);
                        }
                        self.menu = None;
                    } else if point_in_rect(point.0, point.1, THEME) {
                        prefs().toggle();
                    } else if self.toggle_bypass_at(point) {
                    } else if point_in_rect(point.0, point.1, PREVIOUS) {
                        self.change_page(false);
                    } else if point_in_rect(point.0, point.1, NEXT) {
                        self.change_page(true);
                    } else if point_in_rect(point.0, point.1, Self::channel_rect()) {
                        let mic = self.selected.min(CAPACITY - 1);
                        self.open_menu(MenuKind::Channel(mic));
                    } else if point_in_rect(point.0, point.1, Self::cc_rect()) {
                        let mic = self.selected.min(CAPACITY - 1);
                        self.open_menu(MenuKind::Cc(mic));
                    } else if let Some(mic) = self.group_at(point) {
                        self.selected = mic;
                        self.group_press = Some(mic);
                    } else {
                        if let Some(strip) = self.strip_at(point) {
                            self.selected = strip.mic;
                        }
                        if let Some((id, r)) = self.control_at(point) {
                            let value_rect = Self::value_rect(r);
                            if point_in_rect(point.0, point.1, value_rect) {
                                self.press = Some(ValuePress::new(id, value_rect, point));
                            } else {
                                let initial = self.normalized(id);
                                param::begin(cx, self.parameter(id).ptr());
                                self.drag = Some((id, point, initial));
                            }
                            cx.capture();
                        }
                    }
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    self.group_press = None;
                    self.menu = None;
                    self.end_drag(cx);
                    if let Some(strip) = self
                        .drawn_strips
                        .get()
                        .into_iter()
                        .flatten()
                        .find(|strip| point_in_rect(point.0, point.1, Self::name_rect(strip.rect)))
                    {
                        let mic = strip.mic;
                        let name = self.params.shared.mics[mic]
                            .name
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .clone();
                        self.edit = Some(ValueEdit::new(
                            Target::Name(mic),
                            Self::name_rect(strip.rect),
                            name,
                        ));
                    } else if let Some((id, _)) = self.control_at(point) {
                        self.prepare_edit(id, self.parameter(id).default());
                        param::set_normalized_once(
                            cx,
                            self.parameter(id).ptr(),
                            self.parameter(id).default(),
                        );
                    }
                }
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some(point);
                    if let Some((id, origin, initial)) = self.drag {
                        let scale = if cx.modifiers().shift() {
                            0.0005
                        } else {
                            0.005
                        };
                        let value = initial + (point.0 - origin.0 + origin.1 - point.1) * scale;
                        self.prepare_edit(id, value);
                        param::set_normalized(cx, self.parameter(id).ptr(), value);
                    }
                }
                WindowEvent::MouseScroll(_, dy) if self.menu.is_some() => self.scroll_menu(*dy),
                WindowEvent::MouseScroll(_, dy) if self.drag.is_none() => {
                    let mic = self.selected.min(CAPACITY - 1);
                    let fine = cx.modifiers().shift();
                    let list = if point_in_rect(point.0, point.1, Self::channel_rect()) {
                        Some(&self.params.shared.mics[mic].midi_channel)
                    } else if point_in_rect(point.0, point.1, Self::cc_rect()) {
                        Some(&self.params.shared.mics[mic].midi_cc)
                    } else {
                        None
                    };
                    if let Some(param) = list {
                        let current = param.unmodulated_normalized_value();
                        let next = if *dy > 0.0 {
                            param.next_normalized_step(current, fine)
                        } else {
                            param.previous_normalized_step(current, fine)
                        };
                        let ptr = param.as_ptr();
                        param::set_normalized_once(cx, ptr, next);
                    } else if let Some((id, _)) = self.control_at(point) {
                        let value = self.parameter(id).step(
                            self.normalized(id),
                            *dy > 0.0,
                            cx.modifiers().shift(),
                        );
                        self.prepare_edit(id, value);
                        param::set_normalized_once(cx, self.parameter(id).ptr(), value);
                    }
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    self.finish_group(cx, point);
                    self.end_drag(cx);
                }
                WindowEvent::FocusOut
                | WindowEvent::KeyDown(Code::Escape, _)
                | WindowEvent::MouseDown(MouseButton::Right) => {
                    self.menu = None;
                    self.group_press = None;
                    self.end_drag(cx);
                }
                WindowEvent::MouseLeave => {
                    self.hover = None;
                    self.menu = None;
                    self.group_press = None;
                }
                _ => return,
            }
            nih_plug_vizia::consume_window_event(cx, window, meta);
            cx.needs_redraw();
        });
    }
}
