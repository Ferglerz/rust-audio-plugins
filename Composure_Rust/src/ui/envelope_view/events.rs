use super::*;

impl EnvelopeView {
    pub(super) fn handle_event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let was_editing = self.edit.is_some();
        event.map(|window_event, meta| {
            let point = Self::point(cx);
            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    if let Some(edit) = self.edit.as_mut() {
                        if point.0 >= edit.rect.0
                            && point.0 <= edit.rect.0 + edit.rect.2
                            && point.1 >= edit.rect.1
                            && point.1 <= edit.rect.1 + edit.rect.3
                        {
                            edit.handle_mouse_down(point.0);
                            cx.needs_redraw();
                            meta.consume();
                            return;
                        }
                        if !self.commit_edit(cx) {
                            meta.consume();
                            return;
                        }
                    }
                    let laid = self.laid();
                    if let Some(hit) = self.interactive_hit(&laid, point) {
                        if cx.modifiers().command() && hit != EnvelopeHit::Lookahead {
                            self.reset(cx, hit);
                        } else {
                            self.begin_graph(cx, hit, false);
                        }
                        cx.needs_redraw();
                        meta.consume();
                    } else if let Some(hit) = Self::label_hit(point) {
                        if cx.modifiers().command() {
                            self.reset(cx, hit);
                        } else if Viewport::from_tuple(Self::timing_track(hit))
                            .contains(point.0, point.1)
                        {
                            self.begin_graph(cx, hit, true);
                        } else if Viewport::from_tuple(Self::timing_value_rect(hit))
                            .contains(point.0, point.1)
                        {
                            self.press = Some(LabelPress {
                                press: ValuePress::new(hit, Self::timing_value_rect(hit), point),
                                start_norm: self.param(hit).unmodulated_normalized_value(),
                                active: false,
                            });
                            cx.capture();
                            cx.focus();
                        }
                        cx.needs_redraw();
                        meta.consume();
                    }
                }
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some(point);
                    let axis_delta = self.drag.as_mut().map_or(0.0, |drag| {
                        drag.axis
                            .delta(point.0 - drag.origin.0, point.1 - drag.origin.1, 1.0)
                    });
                    if let Some(drag) = &self.drag {
                        let values = drag.start;
                        let laid = layout(Viewport::from_tuple(GRAPH), values);
                        match drag.hit {
                            EnvelopeHit::Lookahead => {
                                let fine = cx.modifiers().shift();
                                let start = self.lookahead.preview_plain(drag.start_norm);
                                let dx = axis_delta * if fine { 0.2 } else { 1.0 };
                                let ms = if drag.lookahead_on_graph {
                                    if dx.abs() < f32::EPSILON {
                                        start
                                    } else {
                                        let x = lookahead_marker_x(start, &laid, values) + dx;
                                        lookahead_drag_ms(
                                            x,
                                            &laid,
                                            values,
                                            fine || cx.modifiers().command(),
                                        )
                                    }
                                } else {
                                    quantize_lookahead_ms(lookahead_bar_ms(
                                        lookahead_bar_x(start) + dx,
                                    ))
                                };
                                self.set_plain(cx, drag.hit, ms);
                            }
                            EnvelopeHit::AttackCurve | EnvelopeHit::ReleaseCurve => {
                                let handle_y = match drag.hit {
                                    EnvelopeHit::AttackCurve => laid.attack_curve.1,
                                    _ => laid.release_curve.1,
                                };
                                let mut curve = curve_drag(
                                    if drag.hit == EnvelopeHit::AttackCurve {
                                        StageHit::AttackCurve
                                    } else {
                                        StageHit::ReleaseCurve
                                    },
                                    &laid,
                                    handle_y + point.1 - drag.origin.1,
                                );
                                if cx.modifiers().shift() {
                                    let current = match drag.hit {
                                        EnvelopeHit::AttackCurve => values.attack_curve,
                                        _ => values.release_curve,
                                    };
                                    curve = current + (curve - current) * 0.2;
                                }
                                self.set_plain(cx, drag.hit, curve);
                            }
                            hit => {
                                let fine = cx.modifiers().shift();
                                let sensitivity = if fine { 0.2 } else { 1.0 };
                                let plain = if drag.time_slider {
                                    let dx = (point.0 - drag.origin.0) * sensitivity;
                                    if dx.abs() < f32::EPSILON {
                                        self.param(hit).unmodulated_plain_value()
                                    } else {
                                        let norm = drag.start_norm
                                            + dx / (Self::timing_track(hit).2 - 4.0);
                                        timing_slider_plain(hit, norm, fine)
                                    }
                                } else {
                                    let plain =
                                        stage_time_drag(hit, values, axis_delta * sensitivity);
                                    if fine || axis_delta == 0.0 {
                                        plain
                                    } else {
                                        Self::snapped_plain(hit, plain)
                                    }
                                };
                                self.set_plain(cx, hit, plain);
                            }
                        }
                        meta.consume();
                    }
                    if let Some(mut press) = self.press.take() {
                        if press.press.update(point.0, point.1) {
                            let param = self.param(press.press.target);
                            if !press.active {
                                param.begin_set_parameter(cx);
                                press.active = true;
                            }
                            let delta = Self::time_drag_delta(
                                press.press.target,
                                press.press.origin,
                                point,
                                cx.modifiers().shift(),
                            );
                            self.set_time_norm(
                                cx,
                                press.press.target,
                                (press.start_norm + delta).clamp(0.0, 1.0),
                                cx.modifiers().shift(),
                            );
                        }
                        self.press = Some(press);
                        meta.consume();
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    let clicked = self.press.as_mut().and_then(|press| {
                        press
                            .press
                            .released_as_click(point.0, point.1)
                            .then_some(press.press.target)
                    });
                    self.finish_gesture(cx);
                    if let Some(hit) = clicked {
                        let param = self.param(hit);
                        self.edit = Some(ValueEdit::new(
                            hit,
                            Self::timing_value_rect(hit),
                            param
                                .normalized_value_to_string(
                                    param.unmodulated_normalized_value(),
                                    false,
                                )
                                .replace('µ', "u"),
                        ));
                    }
                    cx.needs_redraw();
                    meta.consume();
                }
                WindowEvent::MouseScroll(_, dy) => {
                    if self.drag.is_none() && self.press.is_none() {
                        let laid = self.laid();
                        if let Some(hit) = self
                            .interactive_hit(&laid, point)
                            .or_else(|| Self::label_hit(point))
                        {
                            let param = self.param(hit);
                            let step = if cx.modifiers().shift() { 0.004 } else { 0.02 };
                            param.begin_set_parameter(cx);
                            if hit == EnvelopeHit::Lookahead {
                                let ms = self.lookahead.unmodulated_plain_value();
                                let step = if ms < 1.0 || (*dy < 0.0 && ms <= 1.0) {
                                    0.1
                                } else {
                                    1.0
                                };
                                self.set_plain(
                                    cx,
                                    hit,
                                    quantize_lookahead_ms(ms + dy.signum() * step),
                                );
                            } else if cx.modifiers().shift()
                                || matches!(
                                    hit,
                                    EnvelopeHit::AttackCurve | EnvelopeHit::ReleaseCurve
                                )
                            {
                                let direction = if matches!(
                                    hit,
                                    EnvelopeHit::AttackLength | EnvelopeHit::ReleaseLength
                                ) {
                                    -1.0
                                } else {
                                    1.0
                                };
                                self.set_time_norm(
                                    cx,
                                    hit,
                                    (param.unmodulated_normalized_value() + *dy * step * direction)
                                        .clamp(0.0, 1.0),
                                    true,
                                );
                            } else if *dy != 0.0 {
                                let rate = matches!(
                                    hit,
                                    EnvelopeHit::AttackLength | EnvelopeHit::ReleaseLength
                                );
                                let plain = param.unmodulated_plain_value();
                                let ms = if rate { rate_to_ms(plain) } else { plain };
                                let next = if !rate && ms <= 1.0 {
                                    (ms + dy.signum()).max(0.0)
                                } else {
                                    super::super::step_points::next_time_ms(ms, *dy)
                                };
                                self.set_plain(cx, hit, if rate { ms_to_rate(next) } else { next });
                            }
                            param.end_set_parameter(cx);
                            cx.needs_redraw();
                            meta.consume();
                        }
                    }
                }
                WindowEvent::KeyDown(code, key) => {
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
                        meta.consume();
                        nih_plug_vizia::report_handled_key(cx, window_event);
                    }
                }
                WindowEvent::CharInput(c) => {
                    if let Some(edit) = self.edit.as_mut() {
                        if !cx.modifiers().command() && !c.is_control() {
                            edit.insert(&c.to_string());
                        }
                        cx.needs_redraw();
                        meta.consume();
                        nih_plug_vizia::report_handled_key(cx, window_event);
                    }
                }
                WindowEvent::MouseLeave => {
                    self.hover = None;
                    cx.needs_redraw();
                }
                WindowEvent::FocusOut => {
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
