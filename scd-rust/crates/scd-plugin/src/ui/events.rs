//! Input routing for the SCD editor.
use super::*;

impl ScdEditorView {
    pub(super) fn handle_event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|param_event: &RawParamEvent, _| {
            if matches!(param_event, RawParamEvent::ParametersChanged) {
                cx.needs_redraw();
            }
        });
        event.map(|_: &NoteLearnPoll, _| {
            let now = Instant::now();
            self.vel_hits
                .retain(|(_, _, _, at)| now.duration_since(*at).as_secs_f32() < 2.5);
            for piece in KitPieceId::ALL {
                for art in 0..crate::vel_map::MAX_ARTS {
                    let velocity =
                        self.params.midi_velocities[piece as usize][art].swap(0, Ordering::Relaxed);
                    if velocity != 0
                        && self.vel_map_open == Some(piece)
                        && Self::vel_art_members(piece, self.vel_art).contains(&art)
                    {
                        // One trail per velocity keeps memory bounded even under dense MIDI.
                        self.vel_hits
                            .retain(|(p, a, v, _)| (*p, *a, *v) != (piece, art, velocity));
                        self.vel_hits.push((piece, art, velocity, now));
                    }
                }
            }
            if self.apply_note_learn() {
                cx.needs_redraw();
            }
        });
        event.map(|window_event, meta| {
            let bounds = cx.bounds();
            if bounds.w <= 0.0 || bounds.h <= 0.0 {
                return;
            }
            let scale = artwork_scale(bounds.w);
            let mouse_x = (cx.mouse().cursorx - bounds.x) / scale;
            let mouse_y = (cx.mouse().cursory - bounds.y) / scale;
            let prev_y = self.mouse.1;
            self.mouse = (mouse_x, mouse_y);

            if self.note_edit.is_some() {
                match window_event {
                    WindowEvent::CharInput(ch)
                        if ch.is_ascii_digit() && !cx.modifiers().command() =>
                    {
                        if let Some(edit) = self.note_edit.as_mut() {
                            insert_note_digit(edit, *ch);
                        }
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        cx.needs_redraw();
                        return;
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
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        cx.needs_redraw();
                        return;
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
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            cx.needs_redraw();
                            return;
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
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            cx.needs_redraw();
                            return;
                        }
                    }
                    WindowEvent::FocusOut => {
                        self.commit_note_edit();
                        cx.needs_redraw();
                    }
                    _ => {}
                }
            }

            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    let x = mouse_x;
                    let y = mouse_y;

                    if self.add_preset_open {
                        let (mx, my, mw, mh) = Self::add_preset_modal();
                        if Self::hit(x, y, mx, my, mw, mh) {
                            let name_r = Self::add_name_rect((mx, my, mw, mh));
                            self.add_name_focus =
                                Self::hit(x, y, name_r.0, name_r.1, name_r.2, name_r.3);
                            for i in 0..PresetScope::names().len() {
                                let r = Self::add_scope_rect((mx, my, mw, mh), i);
                                if Self::hit(x, y, r.0, r.1, r.2, r.3) {
                                    self.add_scope.toggle(i);
                                    cx.needs_redraw();
                                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                                    return;
                                }
                            }
                            let save = Self::add_save_rect((mx, my, mw, mh));
                            if Self::hit(x, y, save.0, save.1, save.2, save.3) {
                                self.save_user_preset(cx);
                                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                                return;
                            }
                            let cancel = Self::add_cancel_rect((mx, my, mw, mh));
                            if Self::hit(x, y, cancel.0, cancel.1, cancel.2, cancel.3) {
                                self.add_preset_open = false;
                                self.add_name_focus = false;
                                cx.needs_redraw();
                                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                                return;
                            }
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            return;
                        } else {
                            self.add_preset_open = false;
                            self.add_name_focus = false;
                            cx.needs_redraw();
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            return;
                        }
                    }

                    if self.sub_kick_open {
                        let (modal_x, modal_y, modal_w, modal_h) = Self::sub_kick_modal();

                        if Self::hit(x, y, modal_x, modal_y, modal_w, modal_h) {
                            if cx.modifiers().alt() && self.reset_control_at(cx, x, y) {
                                cx.needs_redraw();
                                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                                return;
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
                                    return;
                                }
                            }
                            // Empty modal space must not fall through to the mixer.
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            return;
                        } else {
                            self.sub_kick_open = false;
                            cx.needs_redraw();
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            return;
                        }
                    }

                    if self.vel_map_open.is_some() {
                        let (modal_x, modal_y, modal_w, modal_h) = Self::vel_map_modal();
                        if Self::hit(x, y, modal_x, modal_y, modal_w, modal_h) {
                            if let Some(kit_piece) = self.vel_map_open {
                                let (dx, dy, dw, dh) = Self::vel_art_dropdown_rect();
                                if Self::hit(x, y, dx, dy, dw, dh) {
                                    self.vel_art_menu_open = !self.vel_art_menu_open;
                                    self.vel_ignore_up = true;
                                    cx.needs_redraw();
                                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                                    return;
                                }
                                if self.vel_art_menu_open {
                                    for (i, (art, _)) in
                                        Self::vel_art_options(kit_piece).iter().enumerate()
                                    {
                                        let (sx, sy, sw, sh) = Self::vel_art_item_rect(i);
                                        if Self::hit(x, y, sx, sy, sw, sh) {
                                            self.vel_art = *art;
                                            self.vel_selected_node = None;
                                            break;
                                        }
                                    }
                                    self.vel_art_menu_open = false;
                                    self.vel_ignore_up = true;
                                    cx.needs_redraw();
                                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                                    return;
                                }
                            }
                            if let Some(curve) = self.vel_curve() {
                                if let Some(i) = self.hit_vel_delete_x(&curve, x, y) {
                                    self.vel_ignore_up = true;
                                    let mut curve = curve;
                                    if curve.delete(i) {
                                        self.vel_selected_node = None;
                                        self.commit_vel_curve(curve);
                                    }
                                    cx.needs_redraw();
                                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                                    return;
                                }
                                if let Some((i, is_out)) = self.hit_vel_handle(&curve, x, y) {
                                    self.drag = Some(if is_out {
                                        DragTarget::VelMapHandleOut
                                    } else {
                                        DragTarget::VelMapHandleIn
                                    });
                                    self.vel_selected_node = Some(i);
                                    self.vel_just_inserted = false;
                                    cx.capture();
                                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                                    return;
                                }
                                if let Some(i) = self.hit_vel_node(&curve, x, y) {
                                    self.drag = Some(DragTarget::VelMapNode);
                                    self.vel_selected_node = Some(i);
                                    self.vel_just_inserted = false;
                                    cx.capture();
                                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                                    return;
                                }
                            }
                            self.commit_note_edit();
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            return;
                        } else {
                            self.commit_note_edit();
                            self.vel_map_open = None;
                            self.vel_selected_node = None;
                            cx.needs_redraw();
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            return;
                        }
                    }

                    if self.cc_menu_open {
                        let (mx, my, mw, mh) = Self::cc_menu_rect();
                        if Self::hit(x, y, mx, my, mw, mh) {
                            if let Some(n) = Self::cc_index_at(x, y, (mx, my, mw, mh)) {
                                self.set_cc_number(cx, n);
                            }
                            self.cc_menu_open = false;
                            cx.needs_redraw();
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            return;
                        }
                        self.cc_menu_open = false;
                        cx.needs_redraw();
                    }

                    if self.preset_open {
                        if self.resets_open {
                            let (fx, fy, fw, fh) = Self::reset_flyout_rect();
                            if Self::hit(x, y, fx, fy, fw, fh) {
                                for (i, preset) in FactoryPreset::RESETS.iter().enumerate() {
                                    let (ix, iy, iw, ih) = Self::reset_flyout_item(i);
                                    if Self::hit(x, y, ix, iy, iw, ih) {
                                        self.apply_preset(cx, *preset);
                                        self.close_preset_menus();
                                        cx.needs_redraw();
                                        nih_plug_vizia::consume_window_event(
                                            cx,
                                            window_event,
                                            meta,
                                        );
                                        return;
                                    }
                                }
                                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                                return;
                            }
                        }
                        for (row, (rx, ry, rw, rh)) in self.preset_menu_rows() {
                            if matches!(row, PresetRow::Rule) {
                                continue;
                            }
                            if !Self::hit(x, y, rx, ry, rw, rh) {
                                continue;
                            }
                            match row {
                                PresetRow::Resets => {
                                    self.resets_open = !self.resets_open;
                                    cx.needs_redraw();
                                }
                                PresetRow::Factory(preset) => {
                                    self.apply_preset(cx, preset);
                                    self.current_preset = PresetPick::Factory(preset);
                                    self.close_preset_menus();
                                    cx.needs_redraw();
                                }
                                PresetRow::User(i) => {
                                    self.apply_user_preset(cx, i);
                                    self.close_preset_menus();
                                    cx.needs_redraw();
                                }
                                PresetRow::Add => {
                                    self.open_add_preset(cx);
                                }
                                PresetRow::Rule => {}
                            }
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            return;
                        }
                    }

                    if self.samples_open {
                        for (i, kit_piece) in KitPieceId::ALL.iter().enumerate() {
                            for note2 in [false, true] {
                                let (nx, ny, nw, nh) = Self::mapping_note_rect(i, note2);
                                if Self::hit(x, y, nx, ny, nw, nh) {
                                    self.activate_note(
                                        cx,
                                        NoteTarget {
                                            kit_piece: *kit_piece,
                                            art: Self::mapping_art(*kit_piece),
                                            note2,
                                        },
                                    );
                                    cx.needs_redraw();
                                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                                    return;
                                }
                            }
                            let (ix, iy, iw, ih) = Self::mapping_name_rect(i);
                            if Self::hit(x, y, ix, iy, iw, ih) {
                                self.commit_note_edit();
                                self.open_vel_map(cx, *kit_piece);
                                cx.needs_redraw();
                                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                                return;
                            }
                        }
                        let (mx, my, mw, mh) = Self::mapping_menu_rect();
                        if Self::hit(x, y, mx, my, mw, mh) {
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            return;
                        }
                    }

                    let (px, py, pw, ph) = Self::preset_combo_rect();
                    if Self::hit(x, y, px, py, pw, ph) {
                        self.preset_open = !self.preset_open;
                        self.resets_open = false;
                        self.samples_open = false;
                        self.cc_menu_open = false;
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        return;
                    } else if self.preset_open {
                        self.close_preset_menus();
                        cx.needs_redraw();
                    }

                    let (sx, sy, sw, sh) = Self::samples_combo_rect();
                    if Self::hit(x, y, sx, sy, sw, sh) {
                        self.samples_open = !self.samples_open;
                        self.close_preset_menus();
                        self.cc_menu_open = false;
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        return;
                    } else if self.samples_open {
                        self.commit_note_edit();
                        self.samples_open = false;
                        cx.needs_redraw();
                    }

                    if Self::hit(x, y, CC_INVERT.0, CC_INVERT.1, CC_INVERT.2, CC_INVERT.3) {
                        self.cc_menu_open = false;
                        self.close_preset_menus();
                        self.samples_open = false;
                        self.press_invert = true;
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        return;
                    }

                    if Self::hit(x, y, CC_SELECT.0, CC_SELECT.1, CC_SELECT.2, CC_SELECT.3) {
                        self.cc_menu_open = !self.cc_menu_open;
                        self.close_preset_menus();
                        self.samples_open = false;
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        return;
                    }

                    if let Some(mic) = Self::sof_at(x, y) {
                        if self.active_sof == Some(mic) {
                            self.active_sof = None;
                        } else {
                            self.active_sof = Some(mic);
                        }
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        return;
                    }

                    if Self::hit(x, y, SUB_KICK.0, SUB_KICK.1, SUB_KICK.2, SUB_KICK.3) {
                        self.prepare_modal(cx);
                        self.sub_kick_open = true;
                        self.blur_dirty.set(true);
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        return;
                    }

                    if cx.modifiers().alt() && self.reset_control_at(cx, x, y) {
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
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
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
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
                            cx.needs_redraw();
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
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
                WindowEvent::MouseUp(MouseButton::Left) => {
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
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                    }
                    if self.press_note.take().is_some() {
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
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
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                    }
                    if self.vel_ignore_up {
                        self.vel_ignore_up = false;
                    } else if !was_vel_drag && self.vel_map_open.is_some() {
                        let (gx, gy, gw, gh) = Self::vel_map_graph();
                        if Self::hit(mouse_x, mouse_y, gx, gy, gw, gh) {
                            if let Some(mut curve) = self.vel_curve() {
                                if self.hit_vel_node(&curve, mouse_x, mouse_y).is_none() {
                                    let (nx, _) = Self::graph_to_norm(mouse_x, mouse_y);
                                    if let Some(idx) = curve.insert_at(nx) {
                                        self.vel_selected_node = Some(idx);
                                        self.vel_just_inserted = true;
                                        self.commit_vel_curve(curve);
                                        cx.needs_redraw();
                                        nih_plug_vizia::consume_window_event(
                                            cx,
                                            window_event,
                                            meta,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    if self.reset_control_at(cx, mouse_x, mouse_y) {
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        return;
                    }
                    if self.vel_map_open.is_some() {
                        let (modal_x, modal_y, modal_w, modal_h) = Self::vel_map_modal();
                        let (gx, gy, gw, gh) = Self::vel_map_graph();
                        if self.vel_art_menu_open || !Self::hit(mouse_x, mouse_y, gx, gy, gw, gh) {
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                            return;
                        }
                        if Self::hit(mouse_x, mouse_y, modal_x, modal_y, modal_w, modal_h) {
                            if let Some(kit_piece) = self.vel_map_open {
                                if let Some(mut curve) = self.vel_curve() {
                                    if let Some(i) = self.hit_vel_node(&curve, mouse_x, mouse_y) {
                                        if self.vel_just_inserted || !curve.delete(i) {
                                            self.reset_vel_art(kit_piece);
                                            self.vel_selected_node = None;
                                        } else {
                                            self.vel_selected_node = None;
                                            self.commit_vel_curve(curve);
                                        }
                                    } else {
                                        self.reset_vel_art(kit_piece);
                                        self.vel_selected_node = None;
                                    }
                                }
                            }
                            self.vel_just_inserted = false;
                            self.vel_ignore_up = true;
                            self.drag = None;
                            cx.needs_redraw();
                            nih_plug_vizia::consume_window_event(cx, window_event, meta);
                        }
                    }
                }
                WindowEvent::MouseMove(..) => {
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
                        cx.needs_redraw();
                        nih_plug_vizia::consume_window_event(cx, window_event, meta);
                    } else {
                        self.update_hover(cx);
                    }
                }
                WindowEvent::MouseOut => {
                    if self.hover_sof.is_some()
                        || self.hover_lock
                        || self.hover_invert
                        || self.hover_note.is_some()
                    {
                        self.hover_sof = None;
                        self.hover_lock = false;
                        self.hover_invert = false;
                        self.hover_note = None;
                        cx.needs_redraw();
                    }
                }
                WindowEvent::MouseScroll(..) if self.modal_open() => {
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                }
                WindowEvent::MouseScroll(_, dy)
                    if *dy != 0.0
                        && (Self::hit(
                            mouse_x,
                            mouse_y,
                            CC_SELECT.0,
                            CC_SELECT.1,
                            CC_SELECT.2,
                            CC_SELECT.3,
                        ) || self.cc_menu_open) =>
                {
                    let delta = if *dy > 0.0 { 1 } else { -1 };
                    self.set_cc_number(cx, self.params.cc_number.value() + delta);
                    cx.needs_redraw();
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                }
                WindowEvent::CharInput(ch)
                    if self.add_preset_open && self.add_name_focus && !ch.is_control() =>
                {
                    if self.add_name.len() < 32 {
                        self.add_name.push(*ch);
                        cx.needs_redraw();
                    }
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                }
                WindowEvent::KeyDown(code, _) if self.add_preset_open => {
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
                }
                _ => {}
            }
        });
    }
}
