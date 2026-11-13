use super::*;

impl DereverbView {
    pub(super) fn end_drag(&mut self, cx: &mut EventContext) {
        if self.drag.take().is_some() {
            param::end(cx, self.params.strength.as_ptr());
        }
        if let Some((index, _, _)) = self.curve_drag.take() {
            param::end(cx, self.params.weights()[index].as_ptr());
        }
        if self.guard_drag.take().is_some() {
            param::end(cx, self.params.transient_guard.as_ptr());
        }
        cx.release();
    }

    fn commit(&mut self, cx: &mut EventContext) {
        if let Some(edit) = self.edit.take() {
            if let Some(value) =
                pleasant_ui::value_edit::parse_number_with_units(&edit.text, &[("%", 1.0)])
            {
                if value.is_finite() {
                    param::set_normalized_once(
                        cx,
                        self.params.strength.as_ptr(),
                        value as f32 / 100.0,
                    );
                }
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
                    param::begin(cx, self.params.strength.as_ptr());
                    self.drag = Some((
                        press.origin.0,
                        press.origin.1,
                        self.params.strength.unmodulated_normalized_value(),
                    ));
                }
                PressAction::BeginEdit(_) => {
                    cx.release();
                    self.edit = Some(ValueEdit::new(
                        (),
                        value_rect(),
                        format!("{:.1}", self.params.strength.value()),
                    ));
                    nih_plug_vizia::consume_window_event(cx, window, meta);
                    cx.needs_redraw();
                    return;
                }
                PressAction::Released | PressAction::Cancelled => {
                    cx.release();
                    return;
                }
            }
            if let Some(edit) = &mut self.edit {
                let action = readout::handle_edit(edit, cx, window, point);
                match action {
                    EditAction::Commit | EditAction::CommitAndContinue => self.commit(cx),
                    EditAction::Cancel => self.edit = None,
                    EditAction::Blocked => return,
                    EditAction::Handled => {}
                }
                if action != EditAction::CommitAndContinue {
                    nih_plug_vizia::consume_window_event(cx, window, meta);
                    cx.needs_redraw();
                    return;
                }
            }
            match window {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    cx.focus();
                    if let Some(index) = self.node_at(point) {
                        let parameter = self.params.weights()[index];
                        param::begin(cx, parameter.as_ptr());
                        self.curve_drag =
                            Some((index, point.1, parameter.unmodulated_normalized_value()));
                        cx.capture();
                    } else if point_in_rect(point.0, point.1, RELATIVE)
                        || point_in_rect(point.0, point.1, ABSOLUTE)
                    {
                        param::set_normalized_once(
                            cx,
                            self.params.level_mode.as_ptr(),
                            if point_in_rect(point.0, point.1, RELATIVE) {
                                0.0
                            } else {
                                1.0
                            },
                        );
                    } else if point_in_rect(point.0, point.1, FLAT) {
                        self.set_curve(cx, [1.0; NODES]);
                    } else if point_in_rect(point.0, point.1, LOUDER_BUTTON) {
                        self.set_curve(cx, LOUDER);
                    } else if point_in_rect(point.0, point.1, TRANSIENT_GUARD) {
                        self.end_drag(cx);
                        let guard = &self.params.transient_guard;
                        param::begin(cx, guard.as_ptr());
                        let value = if cx.modifiers().shift() {
                            guard.unmodulated_normalized_value()
                        } else {
                            let value =
                                ((point.0 - GUARD_TRACK_LEFT) / GUARD_TRACK_WIDTH).clamp(0.0, 1.0);
                            param::set_normalized(cx, guard.as_ptr(), value);
                            value
                        };
                        self.guard_drag = Some((point.0, value));
                        cx.capture();
                    } else if point_in_rect(point.0, point.1, THEME) {
                        prefs().toggle();
                    } else if let Some((_, _, engine)) = ENGINE_BUTTONS
                        .iter()
                        .find(|(rect, _, _)| point_in_rect(point.0, point.1, *rect))
                    {
                        self.end_drag(cx);
                        param::set_normalized_once(
                            cx,
                            self.params.engine.as_ptr(),
                            self.params.engine.preview_normalized(*engine),
                        );
                    } else if self.params.engine.value().has_quality()
                        && (point_in_rect(point.0, point.1, LIVE)
                            || point_in_rect(point.0, point.1, HIGH_QUALITY))
                    {
                        self.end_drag(cx);
                        let quality = if point_in_rect(point.0, point.1, LIVE) {
                            Quality::Live
                        } else {
                            Quality::HighQuality
                        };
                        param::set_normalized_once(
                            cx,
                            self.params.quality.as_ptr(),
                            self.params.quality.preview_normalized(quality),
                        );
                    } else if point_in_rect(point.0, point.1, value_rect()) {
                        self.press = Some(ValuePress::new((), value_rect(), point));
                        cx.capture();
                    } else if point_in_rect(point.0, point.1, KNOB) {
                        param::begin(cx, self.params.strength.as_ptr());
                        self.drag = Some((
                            point.0,
                            point.1,
                            self.params.strength.unmodulated_normalized_value(),
                        ));
                        cx.capture();
                    } else {
                        return;
                    }
                }
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some(point);
                    if let Some((index, y, value)) = self.curve_drag {
                        let scale = if cx.modifiers().shift() { 0.1 } else { 1.0 };
                        param::set_normalized(
                            cx,
                            self.params.weights()[index].as_ptr(),
                            value + (y - point.1) / CURVE.3 * scale,
                        );
                    }
                    if let Some((x, value)) = self.guard_drag {
                        let scale = if cx.modifiers().shift() { 0.1 } else { 1.0 };
                        param::set_normalized(
                            cx,
                            self.params.transient_guard.as_ptr(),
                            value + (point.0 - x) / GUARD_TRACK_WIDTH * scale,
                        );
                    }
                    if let Some((x, y, value)) = self.drag {
                        let sensitivity = if cx.modifiers().shift() {
                            0.0005
                        } else {
                            0.005
                        };
                        param::set_normalized(
                            cx,
                            self.params.strength.as_ptr(),
                            value + (point.0 - x + y - point.1) * sensitivity,
                        );
                    }
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left)
                    if self.node_at(point).is_some() =>
                {
                    let index = self.node_at(point).unwrap();
                    self.end_drag(cx);
                    param::set_normalized_once(cx, self.params.weights()[index].as_ptr(), 1.0);
                }
                WindowEvent::MouseScroll(_, dy)
                    if self.node_at(point).is_some() && self.curve_drag.is_none() =>
                {
                    let parameter = self.params.weights()[self.node_at(point).unwrap()];
                    let step = if cx.modifiers().shift() { 0.001 } else { 0.01 };
                    param::set_normalized_once(
                        cx,
                        parameter.as_ptr(),
                        parameter.unmodulated_normalized_value() + dy * step,
                    );
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left)
                    if point_in_rect(point.0, point.1, TRANSIENT_GUARD) =>
                {
                    self.end_drag(cx);
                    let guard = &self.params.transient_guard;
                    param::set_normalized_once(
                        cx,
                        guard.as_ptr(),
                        guard.default_normalized_value(),
                    );
                }
                WindowEvent::MouseScroll(_, dy)
                    if point_in_rect(point.0, point.1, TRANSIENT_GUARD)
                        && self.guard_drag.is_none() =>
                {
                    let guard = &self.params.transient_guard;
                    let step = if cx.modifiers().shift() { 0.001 } else { 0.01 };
                    param::set_normalized_once(
                        cx,
                        guard.as_ptr(),
                        guard.unmodulated_normalized_value() + dy * step,
                    );
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left)
                    if point_in_rect(point.0, point.1, KNOB) =>
                {
                    self.end_drag(cx);
                    param::set_normalized_once(cx, self.params.strength.as_ptr(), 1.0);
                }
                WindowEvent::MouseScroll(_, dy)
                    if point_in_rect(point.0, point.1, KNOB) && self.drag.is_none() =>
                {
                    let step = if cx.modifiers().shift() { 0.001 } else { 0.01 };
                    param::set_normalized_once(
                        cx,
                        self.params.strength.as_ptr(),
                        self.params.strength.unmodulated_normalized_value() + dy * step,
                    );
                }
                WindowEvent::MouseUp(MouseButton::Left)
                | WindowEvent::FocusOut
                | WindowEvent::KeyDown(Code::Escape, _)
                | WindowEvent::MouseDown(MouseButton::Right) => self.end_drag(cx),
                WindowEvent::MouseLeave => self.hover = None,
                _ => return,
            }
            nih_plug_vizia::consume_window_event(cx, window, meta);
            cx.needs_redraw();
        });
    }
}
