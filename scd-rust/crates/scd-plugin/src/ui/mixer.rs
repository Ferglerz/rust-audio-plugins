//! Mixer parameter interaction and painting.
use super::*;

impl ScdEditorView {
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
        let mixer_x = Self::mixer_x();
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
            self.icon_font.get(),
            lock.0,
            lock.1,
            lock.2,
            self.params.fader_lock.value(),
            self.hover_lock,
        );

        draw.text(SUB_KICK.0, SUB_KICK.1 + 14.0, "Sub Kick", 12.0, THEME);

        for (idx, &kit_piece) in KitPieceId::ALL.iter().enumerate() {
            let sx = mixer_x + idx as f32 * STRIP_W;
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

            draw_chan_meter(draw, sx, kit_vu[idx]);
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

            let name = kit_piece.name();
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
