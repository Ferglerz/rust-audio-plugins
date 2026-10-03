//! Mixer parameter interaction and painting.
use super::*;

impl ScdEditorView {
    pub(super) fn reset_target_at(x: f32, y: f32, sub_kick_open: bool) -> Option<DragTarget> {
        if sub_kick_open {
            let (mx, my, mw, mh) = Self::sub_kick_modal();
            if !Self::hit(x, y, mx, my, mw, mh) {
                return None;
            }
            let knob_y = my + 55.0;
            return (0..5).find_map(|i| {
                let knob_x = Self::sub_kick_knob_x(mx, i);
                ((x - knob_x).powi(2) + (y - knob_y).powi(2) <= 30.0_f32.powi(2)).then_some(match i
                {
                    0 => DragTarget::SubKickVol,
                    1 => DragTarget::SubKickLength,
                    2 => DragTarget::SubKickDive,
                    3 => DragTarget::SubKickSpeed,
                    _ => DragTarget::SubKickOffset,
                })
            });
        }
        for piece in KitPieceId::ALL
            .into_iter()
            .filter(|p| *p != KitPieceId::OpenSnare)
        {
            let sx = Self::strip_x(piece);
            if !(sx..=sx + STRIP_W).contains(&x) {
                continue;
            }
            if (PITCH_Y..=PITCH_Y + SLIDER_H).contains(&y) {
                return Some(DragTarget::Pitch(piece));
            }
            if (PAN_Y..=PAN_Y + SLIDER_H).contains(&y) {
                return Some(DragTarget::Pan(piece));
            }
            if (FADER_Y..=FADER_Y + FADER_H).contains(&y) {
                return Some(DragTarget::Fader(piece));
            }
            if (PUNCH_Y..=PUNCH_Y + PUNCH_SIZE).contains(&y) {
                return Some(DragTarget::Punch(piece));
            }
        }
        None
    }

    pub(super) fn reset_control_at(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        if self.add_preset_open || self.vel_map_open.is_some() {
            return false;
        }
        if self.samples_open {
            let (mx, my, mw, mh) = Self::mapping_menu_rect();
            if Self::hit(x, y, mx, my, mw, mh) {
                return false;
            }
        }
        if self.preset_open {
            let (mx, my, mw, mh) = self.preset_menu_rect();
            if Self::hit(x, y, mx, my, mw, mh) {
                return false;
            }
        }
        let Some(mut target) = Self::reset_target_at(x, y, self.sub_kick_open) else {
            return false;
        };
        if self.params.snare_wires_off.value() {
            target = match target {
                DragTarget::Pitch(KitPieceId::Snare) => DragTarget::Pitch(KitPieceId::OpenSnare),
                DragTarget::Pan(KitPieceId::Snare) => DragTarget::Pan(KitPieceId::OpenSnare),
                DragTarget::Fader(KitPieceId::Snare) => DragTarget::Fader(KitPieceId::OpenSnare),
                DragTarget::Punch(KitPieceId::Snare) => DragTarget::Punch(KitPieceId::OpenSnare),
                other => other,
            };
        }
        if matches!(target, DragTarget::Fader(piece) if self.omitted(piece)) {
            return false;
        }
        if self.drag.take().is_some() {
            self.end_gesture(cx);
            cx.release();
        }
        let param = match target {
            DragTarget::Pitch(piece) => &self.params.get_strip(piece).pitch,
            DragTarget::Pan(piece) => &self.params.get_strip(piece).pan,
            DragTarget::Fader(piece) => self.params.get_strip(piece).fader_param(self.active_sof),
            DragTarget::Punch(piece) => &self.params.get_strip(piece).punch,
            DragTarget::SubKickVol => &self.params.sub_kick.vol,
            DragTarget::SubKickLength => &self.params.sub_kick.length,
            DragTarget::SubKickDive => &self.params.sub_kick.dive,
            DragTarget::SubKickSpeed => &self.params.sub_kick.speed,
            DragTarget::SubKickOffset => &self.params.sub_kick.offset,
            _ => return false,
        };
        self.emit_default(cx, param);
        true
    }

    pub(super) fn omitted(&self, kit_piece: KitPieceId) -> bool {
        self.active_sof
            .map(|mic| kit_piece.omits_mic(mic))
            .unwrap_or(false)
    }

    pub(super) fn apply_fader(&self, cx: &mut EventContext, dragged: KitPieceId, hise_pos: f32) {
        if self.omitted(dragged) {
            return;
        }
        let sof = self.active_sof;
        let dragged_param = self.params.get_strip(dragged).fader_param(sof);
        let new_db = fader_law::pos_to_db(hise_pos);
        if self.params.fader_lock.value() {
            let old_db = util::gain_to_db(dragged_param.unmodulated_plain_value());
            let delta = new_db - old_db;
            for kit_piece in KitPieceId::ALL {
                if self.omitted(kit_piece) {
                    continue;
                }
                let param = self.params.get_strip(kit_piece).fader_param(sof);
                let db = (util::gain_to_db(param.unmodulated_plain_value()) + delta)
                    .clamp(FADER_MIN_DB, FADER_MAX_DB);
                self.set_live(
                    cx,
                    param.as_ptr(),
                    param.preview_normalized(util::db_to_gain(db)),
                );
            }
        } else {
            self.set_live(
                cx,
                dragged_param.as_ptr(),
                dragged_param.preview_normalized(util::db_to_gain(new_db)),
            );
        }
    }

    pub(super) fn begin_fader_gesture(&mut self, cx: &mut EventContext, dragged: KitPieceId) {
        let sof = self.active_sof;
        if self.params.fader_lock.value() {
            for kit_piece in KitPieceId::ALL {
                if self.omitted(kit_piece) {
                    continue;
                }
                let ptr = self.params.get_strip(kit_piece).fader_param(sof).as_ptr();
                self.begin_ptr(cx, ptr, kit_piece as usize);
            }
        } else {
            let ptr = self.params.get_strip(dragged).fader_param(sof).as_ptr();
            self.begin_ptr(cx, ptr, dragged as usize);
        }
    }

    pub(super) fn draw_mixer(&self, draw: &mut Draw<'_>, kit_vu: &[f32; KitPieceId::COUNT]) {
        draw.text(LABEL_X, MIXER_Y + 12.0, "PITCH:", 12.0, THEME);
        draw.text(LABEL_X + 12.0, MIXER_Y + 32.0, "PAN:", 12.0, THEME);
        draw.text(LABEL_X, MIXER_Y + 352.0, "PUNCH:", 12.0, THEME);

        let sc = Self::logo_sc_rect();
        if let Some(id) = self.logo_img.get() {
            blit(draw, id, sc.0, sc.1, sc.2, sc.3);
        }
        let sh = Self::logo_sh_rect();
        if let Some(id) = self.stone_img.get() {
            blit(draw, id, sh.0, sh.1, sh.2, sh.3);
        }

        let lock = Self::lock_rect();
        draw_lock(
            draw,
            lock.0,
            lock.1,
            lock.2,
            self.params.fader_lock.value(),
            self.hover_lock,
        );

        draw.text(SUB_KICK.0, SUB_KICK.1 + 14.0, "Sub Kick", 12.0, THEME);

        for kit_piece in self.visible_pieces() {
            let idx = kit_piece as usize;
            let sx = Self::strip_x(kit_piece);
            if let Some(id) = self.icons[idx].get() {
                if let Ok((w, h)) = draw.c.image_size(id) {
                    let scale = ((STRIP_W - 16.0) / w as f32).min(40.0 / h as f32);
                    let (w, h) = (w as f32 * scale, h as f32 * scale);
                    blit(
                        draw,
                        id,
                        sx + (STRIP_W - w) * 0.5,
                        34.0 + (40.0 - h) * 0.5,
                        w,
                        h,
                    );
                }
            }
            if matches!(kit_piece, KitPieceId::Snare | KitPieceId::OpenSnare) {
                for mixed in [false, true] {
                    let (x, y, w, h) = Self::snare_toggle_rect(mixed);
                    let on = if mixed {
                        self.params.snare_mixed.value()
                    } else {
                        self.params.snare_wires_off.value()
                    };
                    draw.rounded_rect(x, y, w, h, 4.0, if on { STAGE_ON } else { SOF_OFF });
                    let label = if mixed {
                        "Mixed"
                    } else if on {
                        "Wires Off"
                    } else {
                        "Wires On"
                    };
                    draw.text_centered(x + w * 0.5, y + 13.0, label, 10.0, THEME);
                }
            }
            let strip = self.params.get_strip(kit_piece);
            let omitted = self.omitted(kit_piece);
            let slider_x = sx + 5.0;
            let slider_w = STRIP_W - 10.0;

            bipolar_slider(
                draw,
                slider_x,
                PITCH_Y,
                slider_w,
                SLIDER_H,
                strip.pitch.unmodulated_normalized_value(),
            );
            bipolar_slider(
                draw,
                slider_x,
                PAN_Y,
                slider_w,
                SLIDER_H,
                strip.pan.unmodulated_normalized_value(),
            );

            draw_chan_meter(
                draw,
                sx,
                if matches!(kit_piece, KitPieceId::Snare | KitPieceId::OpenSnare)
                    && self.params.snare_mixed.value()
                {
                    kit_vu[KitPieceId::Snare as usize].max(kit_vu[KitPieceId::OpenSnare as usize])
                } else {
                    kit_vu[idx]
                },
            );
            draw.rect(sx + STRIP_W * 0.5 - 1.0, FADER_Y, 2.0, FADER_H, FADER_LINE);

            let gain_db =
                util::gain_to_db(strip.fader_param(self.active_sof).unmodulated_plain_value());
            let gain_pos = fader_law::db_to_pos(gain_db);
            let handle_w = STRIP_W - FADER_HANDLE_INSET * 2.0;
            let handle_x = sx + FADER_HANDLE_INSET;
            let handle_y = FADER_Y + (1.0 - gain_pos) * (FADER_H - FADER_HANDLE_H);
            let handle_color = if omitted {
                THEME_DIM
            } else {
                match self.active_sof {
                    None => FADER_MASTER,
                    Some(mic) => MIC_COLORS[mic as usize],
                }
            };
            if !omitted {
                draw.rounded_rect(
                    handle_x + 1.5,
                    handle_y + 3.0,
                    handle_w - 3.0,
                    FADER_HANDLE_H,
                    5.0,
                    FADER_SHADOW,
                );
            }
            draw.rounded_rect(
                handle_x,
                handle_y,
                handle_w,
                FADER_HANDLE_H,
                5.0,
                handle_color,
            );

            let name = if kit_piece == KitPieceId::OpenSnare {
                "Snare"
            } else {
                kit_piece.name()
            };
            let split = name.split_once(' ').filter(|_| name.len() > 6);
            if let Some((a, b)) = split {
                draw.text_centered(sx + STRIP_W * 0.5, handle_y + 11.0, a, 10.0, FADER_TEXT);
                draw.text_centered(sx + STRIP_W * 0.5, handle_y + 21.0, b, 10.0, FADER_TEXT);
            } else {
                draw.text_centered(sx + STRIP_W * 0.5, handle_y + 17.0, name, 11.0, FADER_TEXT);
            }

            hise_knob(
                draw,
                sx + STRIP_W * 0.5,
                PUNCH_Y + PUNCH_SIZE * 0.5,
                PUNCH_SIZE,
                strip.punch.unmodulated_normalized_value(),
                true,
                None,
            );
        }

        match self.drag {
            Some(DragTarget::Pitch(kp)) => {
                let text = format!(
                    "{:.1} st",
                    self.params.get_strip(kp).pitch.unmodulated_plain_value()
                );
                draw_value_popup(
                    draw,
                    Self::strip_x(kp) + STRIP_W * 0.5,
                    PITCH_Y,
                    &text,
                    true,
                );
            }
            Some(DragTarget::Pan(kp)) => {
                let text = format!(
                    "{:.0}%",
                    self.params.get_strip(kp).pan.unmodulated_plain_value()
                );
                draw_value_popup(
                    draw,
                    Self::strip_x(kp) + STRIP_W * 0.5,
                    PAN_Y + SLIDER_H,
                    &text,
                    false,
                );
            }
            Some(DragTarget::Fader(kp)) => {
                let param = self.params.get_strip(kp).fader_param(self.active_sof);
                let pos = fader_law::db_to_pos(util::gain_to_db(param.unmodulated_plain_value()));
                let handle_y = FADER_Y + (1.0 - pos) * (FADER_H - FADER_HANDLE_H);
                draw_value_popup(
                    draw,
                    Self::strip_x(kp) + STRIP_W * 0.5,
                    handle_y,
                    &format_db(param.unmodulated_plain_value()),
                    true,
                );
            }
            _ => {}
        }

        for (i, mic) in MicChannel::ALL.iter().enumerate() {
            let (bx, by, bw, bh) = Self::sof_rect(i);
            let is_active = self.active_sof == Some(*mic);
            let bg = if is_active {
                MIC_COLORS[*mic as usize]
            } else if self.hover_sof == Some(*mic) {
                SOF_HOVER
            } else {
                SOF_OFF
            };
            draw.rounded_rect(bx, by, bw, bh, 5.0, bg);
            draw.text_centered(
                bx + bw * 0.5,
                by + bh * 0.5 + 4.0,
                mic.name(),
                11.0,
                if is_active { FADER_TEXT } else { THEME },
            );
        }
    }
}
