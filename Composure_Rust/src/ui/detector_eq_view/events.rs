use super::*;

impl DetectorEqView {
    pub(super) fn handle_event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let was_editing = self.edit.is_some();
        if !super::super::graph_pages::detector_active(cx) {
            self.close_menu(cx);
            self.finish_gesture(cx);
            self.edit = None;
            pleasant_ui::value_edit::sync_text_input(cx, was_editing, false);
            return;
        }
        event.map(|e: &WindowEvent, meta| {
            let point = Self::point(cx);
            match e {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    if self.shape_menu {
                        if let (Some(Node::Band(i)), Some(shape)) =
                            (self.selected, menu_shape(point))
                        {
                            let param = &self.params.detector_eq[i].shape;
                            set_once(cx, param, param.preview_normalized(shape));
                        }
                        self.close_menu(cx);
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                        return;
                    }
                    if let Some(edit) = self.edit.as_mut() {
                        if point_in_rect(point.0, point.1, edit.rect) {
                            edit.handle_mouse_down(point.0);
                            cx.needs_redraw();
                            nih_plug_vizia::consume_window_event(cx, e, meta);
                            return;
                        }
                        if !self.commit_edit(cx) {
                            nih_plug_vizia::consume_window_event(cx, e, meta);
                            return;
                        }
                    }
                    if point_in_rect(point.0, point.1, LISTEN_RECT)
                        || point_in_rect(point.0, point.1, SIDECHAIN_RECT)
                    {
                        let param = if point_in_rect(point.0, point.1, LISTEN_RECT) {
                            &self.params.sc_adjust_preview
                        } else {
                            &self.params.use_sidechain
                        };
                        set_once(cx, param, if param.value() { 0.0 } else { 1.0 });
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                        return;
                    }
                    if let Some(target) = self.value_hit(point) {
                        self.press = Some(ReadoutPress {
                            press: ValuePress::new(
                                target,
                                field_rect(target.field, &self.fields(target.node)),
                                point,
                            ),
                            start: self.param(target).unmodulated_normalized_value(),
                            active: false,
                        });
                        cx.capture();
                        cx.focus();
                    } else if let Some(node) = self.hit(point) {
                        self.selected = Some(node);
                        self.begin_node(cx, node, point);
                    } else if let Some(node) = self
                        .selected
                        .filter(|_| point.1 >= ACTIONS_Y && point.1 <= ACTIONS_Y + 28.0)
                    {
                        if point_in_rect(point.0, point.1, REMOVE_RECT) {
                            self.remove(cx, node);
                        } else if let Node::Band(i) = node {
                            let b = &self.params.detector_eq[i];
                            if point_in_rect(point.0, point.1, SHAPE_RECT) {
                                self.shape_menu = true;
                                cx.focus();
                                cx.capture();
                            } else if point_in_rect(point.0, point.1, BYPASS_RECT) {
                                self.bypass_anim.trigger_click();
                                set_once(cx, &b.enabled, if b.enabled.value() { 0.0 } else { 1.0 });
                            }
                        }
                    } else if point_in_rect(point.0, point.1, GRAPH) {
                        self.create(cx, point);
                        cx.focus();
                    } else {
                        self.selected = None;
                    }
                    cx.needs_redraw();
                    nih_plug_vizia::consume_window_event(cx, e, meta);
                }
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some(point);
                    if let Some(drag) = &self.drag {
                        let node = drag.node;
                        if cx.modifiers().shift() && drag.fields.contains(&Field::Q) {
                            let p = self.param(Target {
                                node,
                                field: Field::Q,
                            });
                            let q = drag.q * 2.0_f32.powf((drag.origin.1 - point.1) / 60.0);
                            pleasant_ui::param::set_normalized(
                                cx,
                                p.as_ptr(),
                                p.preview_normalized(q),
                            );
                        } else {
                            let p = self.param(Target {
                                node,
                                field: Field::Frequency,
                            });
                            let frequency =
                                x_frequency(frequency_x(drag.frequency) + point.0 - drag.origin.0);
                            pleasant_ui::param::set_normalized(
                                cx,
                                p.as_ptr(),
                                p.preview_normalized(frequency),
                            );
                            if drag.fields.contains(&Field::Q) && self.settings(node).shape.is_cut()
                            {
                                let param = self.param(Target {
                                    node,
                                    field: Field::Q,
                                });
                                let q = drag.q * 2.0_f32.powf((point.1 - drag.origin.1) / 60.0);
                                pleasant_ui::param::set_normalized(
                                    cx,
                                    param.as_ptr(),
                                    param.preview_normalized(q),
                                );
                            }
                            if drag.fields.contains(&Field::Gain)
                                && self.settings(node).shape.has_gain()
                            {
                                let p = self.param(Target {
                                    node,
                                    field: Field::Gain,
                                });
                                let gain = y_gain(gain_y(drag.gain) + point.1 - drag.origin.1);
                                pleasant_ui::param::set_normalized(
                                    cx,
                                    p.as_ptr(),
                                    p.preview_normalized(gain),
                                );
                            }
                        }
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                    }
                    if let Some(mut press) = self.press.take() {
                        if press.press.update(point.0, point.1) {
                            let p = self.param(press.press.target);
                            if !press.active {
                                pleasant_ui::param::begin(cx, p.as_ptr());
                                press.active = true;
                                self.display.set_filter_preview_drag(true);
                            }
                            let delta = pleasant_ui::pointer::readout_drag_delta(
                                point.0 - press.press.origin.0,
                                point.1 - press.press.origin.1,
                                cx.modifiers().shift(),
                            );
                            pleasant_ui::param::set_normalized(
                                cx,
                                p.as_ptr(),
                                (press.start + delta).clamp(0.0, 1.0),
                            );
                        }
                        self.press = Some(press);
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    let clicked = self.press.as_mut().and_then(|p| {
                        p.press
                            .released_as_click(point.0, point.1)
                            .then_some(p.press.target)
                    });
                    self.finish_gesture(cx);
                    if let Some(target) = clicked {
                        let value = self.param(target).normalized_value_to_string(
                            self.param(target).unmodulated_normalized_value(),
                            false,
                        );
                        self.edit = Some(ValueEdit::new(
                            target,
                            field_rect(target.field, &self.fields(target.node)),
                            value,
                        ));
                    }
                    cx.needs_redraw();
                    nih_plug_vizia::consume_window_event(cx, e, meta);
                }
                WindowEvent::MouseScroll(_, dy) => {
                    if self.shape_menu {
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                        return;
                    }
                    if self.drag.is_none() && self.press.is_none() {
                        let target = self.value_hit(point).or_else(|| {
                            self.hit(point).map(|node| Target {
                                node,
                                field: if matches!(node, Node::Band(_)) {
                                    Field::Q
                                } else {
                                    Field::Frequency
                                },
                            })
                        });
                        if let Some(target) = target {
                            self.scroll(cx, target, *dy);
                            cx.needs_redraw();
                            nih_plug_vizia::consume_window_event(cx, e, meta);
                        }
                    }
                }
                WindowEvent::MouseDown(MouseButton::Right) => {
                    if self.shape_menu {
                        self.close_menu(cx);
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                        return;
                    }
                    if let Some(node) = self.hit(point) {
                        self.remove(cx, node);
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                    }
                }
                WindowEvent::KeyDown(code, key) => {
                    if self.shape_menu {
                        if *code == Code::Escape {
                            self.close_menu(cx);
                        }
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                        return;
                    }
                    if self.edit.is_some() {
                        match code {
                            Code::Enter | Code::NumpadEnter => {
                                self.commit_edit(cx);
                            }
                            Code::Escape => self.edit = None,
                            _ => {
                                let edit = self.edit.as_mut().unwrap();
                                edit.handle_key(cx, *code);
                                if !matches!(key, Some(Key::Character(_)))
                                    && !cx.modifiers().command()
                                {
                                    if let Some(c) =
                                        pleasant_ui::typed_char(*code, cx.modifiers().shift())
                                    {
                                        edit.insert(&c.to_string());
                                    }
                                }
                            }
                        }
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                    } else if matches!(code, Code::Delete | Code::Backspace) {
                        if let Some(node) = self.selected {
                            self.remove(cx, node);
                            cx.needs_redraw();
                            nih_plug_vizia::consume_window_event(cx, e, meta);
                        }
                    }
                }
                WindowEvent::CharInput(c) => {
                    if let Some(edit) = self.edit.as_mut() {
                        if !cx.modifiers().command() && !c.is_control() {
                            edit.insert(&c.to_string());
                        }
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                    }
                }
                WindowEvent::MouseLeave => {
                    self.hover = None;
                    cx.needs_redraw();
                }
                WindowEvent::FocusOut => {
                    self.close_menu(cx);
                    self.finish_gesture(cx);
                    self.commit_edit(cx);
                    cx.needs_redraw();
                }
                _ => {}
            }
        });
        pleasant_ui::value_edit::sync_text_input(cx, was_editing, self.edit.is_some());
    }
}
