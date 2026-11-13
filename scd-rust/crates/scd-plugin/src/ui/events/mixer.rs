use super::*;

impl ScdEditorView {
    pub(super) fn handle_mixer_mouse_down(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        x: f32,
        y: f32,
    ) {
        if let Some(mic) = Self::sof_at(x, y) {
            if self.active_sof == Some(mic) {
                self.active_sof = None;
            } else {
                self.active_sof = Some(mic);
            }
            consume_redraw(cx, window_event, meta);
            return;
        }

        if Self::hit(x, y, SUB_KICK.0, SUB_KICK.1, SUB_KICK.2, SUB_KICK.3) {
            self.prepare_modal(cx);
            self.sub_kick_open = true;
            self.blur_dirty.set(true);
            consume_redraw(cx, window_event, meta);
            return;
        }

        if cx.modifiers().alt() && self.reset_control_at(cx, x, y) {
            consume_redraw(cx, window_event, meta);
            return;
        }

        let lock = Self::lock_rect();
        if Self::hit(x, y, lock.0, lock.1, lock.2, lock.3) {
            let cur = self.params.fader_lock.value();
            self.emit_norm(
                cx,
                self.params.fader_lock.as_ptr(),
                if !cur { 1.0 } else { 0.0 },
            );
            consume_redraw(cx, window_event, meta);
            return;
        }

        for mixed in [false, true] {
            let (bx, by, bw, bh) = Self::snare_toggle_rect(mixed);
            if Self::hit(x, y, bx, by, bw, bh) {
                let param = if mixed {
                    &self.params.snare_mixed
                } else {
                    &self.params.snare_wires_off
                };
                self.emit_norm(
                    cx,
                    param.as_ptr(),
                    if param.value() { 0.0 } else { 1.0 },
                );
                consume_redraw(cx, window_event, meta);
                return;
            }
        }
        let pieces: Vec<_> = self.visible_pieces().collect();
        for kit_piece in pieces {
            let sx = Self::strip_x(kit_piece);
            if !(sx..=sx + STRIP_W).contains(&x) {
                continue;
            }
            if (PITCH_Y..=PITCH_Y + SLIDER_H).contains(&y) {
                let ptr = self.params.get_strip(kit_piece).pitch.as_ptr();
                self.drag = Some(DragTarget::Pitch(kit_piece));
                self.begin_one(cx, ptr);
                self.set_live(cx, ptr, Self::slider_norm_at(x, sx));
                cx.capture();
                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                return;
            }
            if (PAN_Y..=PAN_Y + SLIDER_H).contains(&y) {
                let ptr = self.params.get_strip(kit_piece).pan.as_ptr();
                self.drag = Some(DragTarget::Pan(kit_piece));
                self.begin_one(cx, ptr);
                self.set_live(cx, ptr, Self::slider_norm_at(x, sx));
                cx.capture();
                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                return;
            }
            if (FADER_Y..=FADER_Y + FADER_H).contains(&y) {
                if self.omitted(kit_piece) {
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                    return;
                }
                self.drag = Some(DragTarget::Fader(kit_piece));
                self.begin_fader_gesture(cx, kit_piece);
                self.apply_fader(cx, kit_piece, Self::fader_pos_at(y));
                cx.capture();
                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                return;
            }
            if (PUNCH_Y..=PUNCH_Y + PUNCH_SIZE).contains(&y) {
                let ptr = self.params.get_strip(kit_piece).punch.as_ptr();
                self.drag = Some(DragTarget::Punch(kit_piece));
                self.begin_one(cx, ptr);
                cx.capture();
                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                return;
            }
        }
    }

    pub(super) fn handle_mouse_up(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        mouse_x: f32,
        mouse_y: f32,
    ) {
        if self.press_invert {
            self.press_invert = false;
            if Self::hit(
                mouse_x,
                mouse_y,
                CC_INVERT.0,
                CC_INVERT.1,
                CC_INVERT.2,
                CC_INVERT.3,
            ) {
                let on = self.params.invert_cc.unmodulated_plain_value();
                cx.emit(ParamEvent::BeginSetParameter(&self.params.invert_cc).upcast());
                cx.emit(ParamEvent::SetParameter(&self.params.invert_cc, !on).upcast());
                cx.emit(ParamEvent::EndSetParameter(&self.params.invert_cc).upcast());
            }
            consume_redraw(cx, window_event, meta);
        }
        if self.press_note.take().is_some() {
            consume_redraw(cx, window_event, meta);
        }
        let was_vel_drag = matches!(
            self.drag,
            Some(
                DragTarget::VelMapNode
                    | DragTarget::VelMapHandleIn
                    | DragTarget::VelMapHandleOut
            )
        );
        if self.drag.is_some() {
            self.drag = None;
            self.end_gesture(cx);
            cx.release();
            consume_redraw(cx, window_event, meta);
        }
        self.handle_vel_map_mouse_up(cx, window_event, meta, mouse_x, mouse_y, was_vel_drag);
    }

    pub(super) fn handle_drag_move(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        mouse_x: f32,
        mouse_y: f32,
        prev_y: f32,
    ) {
        if let Some(drag) = self.drag {
            let vdelta = -(mouse_y - prev_y) * 0.005;
            match drag {
                DragTarget::Pitch(kp) => {
                    let p = &self.params.get_strip(kp).pitch;
                    self.set_live(
                        cx,
                        p.as_ptr(),
                        Self::slider_norm_at(mouse_x, Self::strip_x(kp)),
                    );
                }
                DragTarget::Pan(kp) => {
                    let p = &self.params.get_strip(kp).pan;
                    self.set_live(
                        cx,
                        p.as_ptr(),
                        Self::slider_norm_at(mouse_x, Self::strip_x(kp)),
                    );
                }
                DragTarget::Punch(kp) => {
                    let p = &self.params.get_strip(kp).punch;
                    self.set_live(
                        cx,
                        p.as_ptr(),
                        p.unmodulated_normalized_value() + vdelta,
                    );
                }
                DragTarget::Fader(kp) => {
                    self.apply_fader(cx, kp, Self::fader_pos_at(mouse_y));
                }
                DragTarget::SubKickVol => {
                    let p = &self.params.sub_kick.vol;
                    self.set_live(
                        cx,
                        p.as_ptr(),
                        p.unmodulated_normalized_value() + vdelta,
                    );
                }
                DragTarget::SubKickLength => {
                    let p = &self.params.sub_kick.length;
                    self.set_live(
                        cx,
                        p.as_ptr(),
                        p.unmodulated_normalized_value() + vdelta,
                    );
                }
                DragTarget::SubKickDive => {
                    let p = &self.params.sub_kick.dive;
                    self.set_live(
                        cx,
                        p.as_ptr(),
                        p.unmodulated_normalized_value() + vdelta,
                    );
                }
                DragTarget::SubKickSpeed => {
                    let p = &self.params.sub_kick.speed;
                    self.set_live(
                        cx,
                        p.as_ptr(),
                        p.unmodulated_normalized_value() + vdelta,
                    );
                }
                DragTarget::SubKickOffset => {
                    let p = &self.params.sub_kick.offset;
                    self.set_live(
                        cx,
                        p.as_ptr(),
                        p.unmodulated_normalized_value() + vdelta,
                    );
                }
                DragTarget::VelMapNode => {
                    if let Some(i) = self.vel_selected_node {
                        if let Some(mut curve) = self.vel_curve() {
                            let (nx, ny) = Self::graph_to_norm(mouse_x, mouse_y);
                            curve.move_node(i, nx, ny);
                            self.commit_vel_curve(curve);
                        }
                    }
                }
                DragTarget::VelMapHandleOut => {
                    if let Some(i) = self.vel_selected_node {
                        if let Some(mut curve) = self.vel_curve() {
                            let (nx, ny) = Self::graph_to_xy(mouse_x, mouse_y);
                            curve.drag_out_handle(i, nx, ny);
                            self.commit_vel_curve(curve);
                        }
                    }
                }
                DragTarget::VelMapHandleIn => {
                    if let Some(i) = self.vel_selected_node {
                        if let Some(mut curve) = self.vel_curve() {
                            let (nx, ny) = Self::graph_to_xy(mouse_x, mouse_y);
                            curve.drag_in_handle(i, nx, ny);
                            self.commit_vel_curve(curve);
                        }
                    }
                }
            }
            consume_redraw(cx, window_event, meta);
        } else {
            self.update_hover(cx);
        }
    }
}
