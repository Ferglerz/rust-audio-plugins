//! Preset menu state, actions, layout rows, and painting.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PresetPick {
    Factory(FactoryPreset),
    User(usize),
}

#[derive(Clone, Copy)]
pub(super) enum PresetRow {
    Resets,
    Rule,
    Factory(FactoryPreset),
    User(usize),
    Add,
}

impl ScdEditorView {
    pub(super) fn preset_menu_rows(&self) -> Vec<(PresetRow, (f32, f32, f32, f32))> {
        let users = self.params.user_presets.list();
        let mut y = PRESET_Y + PRESET_H + 8.0;
        let mut rows = Vec::new();
        rows.push((
            PresetRow::Resets,
            (PRESET_X, y, PRESET_MENU_W, PRESET_ITEM_H),
        ));
        y += PRESET_ITEM_H;
        rows.push((
            PresetRow::Rule,
            (
                PRESET_X + 10.0,
                y + PRESET_RULE_H * 0.5 - 0.5,
                PRESET_MENU_W - 20.0,
                1.0,
            ),
        ));
        y += PRESET_RULE_H;
        for preset in FactoryPreset::SNAPSHOTS {
            rows.push((
                PresetRow::Factory(preset),
                (PRESET_X, y, PRESET_MENU_W, PRESET_ITEM_H),
            ));
            y += PRESET_ITEM_H;
        }
        for i in 0..users.len() {
            rows.push((
                PresetRow::User(i),
                (PRESET_X, y, PRESET_MENU_W, PRESET_ITEM_H),
            ));
            y += PRESET_ITEM_H;
        }
        rows.push((
            PresetRow::Rule,
            (
                PRESET_X + 10.0,
                y + PRESET_RULE_H * 0.5 - 0.5,
                PRESET_MENU_W - 20.0,
                1.0,
            ),
        ));
        y += PRESET_RULE_H;
        rows.push((PresetRow::Add, (PRESET_X, y, PRESET_MENU_W, PRESET_ITEM_H)));
        rows
    }

    pub(super) fn preset_menu_rect(&self) -> (f32, f32, f32, f32) {
        let rows = self.preset_menu_rows();
        let top = PRESET_Y + PRESET_H + 4.0;
        let bottom = rows.last().map(|(_, r)| r.1 + r.3).unwrap_or(top) + 4.0;
        (
            PRESET_X,
            top,
            PRESET_MENU_W,
            (bottom - top).max(PRESET_ITEM_H),
        )
    }

    pub(super) fn close_preset_menus(&mut self) {
        self.preset_open = false;
        self.resets_open = false;
    }

    pub(super) fn open_add_preset(&mut self, cx: &mut EventContext) {
        self.prepare_modal(cx);
        self.add_preset_open = true;
        self.add_name = next_preset_name(&self.params.user_presets.list());
        self.add_scope = PresetScope::all();
        self.add_name_focus = true;
        self.blur_dirty.set(true);
        cx.focus();
        cx.needs_redraw();
    }

    pub(super) fn save_user_preset(&mut self, cx: &mut EventContext) {
        let name = self.add_name.trim();
        if name.is_empty() || !self.add_scope.any() {
            return;
        }
        if self.params.user_presets.list().len() >= MAX_USER_PRESETS {
            return;
        }
        let preset = UserPreset::capture(name.to_string(), self.add_scope, &self.params);
        if let Some(idx) = self.params.user_presets.push(preset) {
            self.current_preset = PresetPick::User(idx);
        }
        self.add_preset_open = false;
        self.add_name_focus = false;
        cx.needs_redraw();
    }

    pub(super) fn apply_user_preset(&mut self, cx: &mut EventContext, index: usize) {
        let Some(preset) = self.params.user_presets.get(index) else {
            return;
        };
        for (ptr, norm) in preset.apply_values(&self.params) {
            self.emit_norm(cx, ptr, norm);
        }
        if let Some(bank) = preset.vel_maps {
            self.params.vel_maps.load_bank(bank);
        }
        if let Some(bank) = preset.note_maps {
            self.params.note_maps.load_bank(bank);
        }
        self.current_preset = PresetPick::User(index);
    }

    pub(super) fn apply_preset(&self, cx: &mut EventContext, preset: FactoryPreset) {
        match preset {
            FactoryPreset::Init => {
                self.emit_default(cx, &self.params.master_gain);
                self.emit_default(cx, &self.params.sub_kick.vol);
                self.emit_default(cx, &self.params.sub_kick.length);
                self.emit_default(cx, &self.params.sub_kick.dive);
                self.emit_default(cx, &self.params.sub_kick.speed);
                self.emit_default(cx, &self.params.sub_kick.offset);
                for kit_piece in KitPieceId::ALL {
                    let strip = self.params.get_strip(kit_piece);
                    self.emit_default(cx, &strip.gain);
                    self.emit_default(cx, &strip.pan);
                    self.emit_default(cx, &strip.pitch);
                    self.emit_default(cx, &strip.punch);
                    for mic in MicChannel::ALL {
                        self.emit_default(cx, strip.send_for(mic));
                    }
                }
                self.params.vel_maps.reset_all();
                self.params.note_maps.reset_all();
            }
            FactoryPreset::ResetPitch => {
                for kit_piece in KitPieceId::ALL {
                    self.emit_plain(cx, &self.params.get_strip(kit_piece).pitch, 0.0);
                }
            }
            FactoryPreset::ResetPan => {
                for kit_piece in KitPieceId::ALL {
                    self.emit_plain(cx, &self.params.get_strip(kit_piece).pan, 0.0);
                }
            }
            FactoryPreset::ResetPunch => {
                for kit_piece in KitPieceId::ALL {
                    self.emit_plain(cx, &self.params.get_strip(kit_piece).punch, 0.0);
                }
            }
            FactoryPreset::ResetVolumes => {
                let unity = util::db_to_gain(0.0);
                self.emit_plain(cx, &self.params.master_gain, unity);
                for kit_piece in KitPieceId::ALL {
                    let strip = self.params.get_strip(kit_piece);
                    self.emit_plain(cx, &strip.gain, unity);
                    for mic in MicChannel::ALL {
                        self.emit_plain(
                            cx,
                            strip.send_for(mic),
                            util::db_to_gain(default_send_db(mic)),
                        );
                    }
                }
            }
            FactoryPreset::DrummerPerspective => {
                for (kit_piece, pan) in DRUMMER_PANS {
                    self.emit_plain(cx, &self.params.get_strip(kit_piece).pan, pan);
                }
            }
        }
    }

    pub(super) fn draw_preset_header(&self, draw: &mut Draw<'_>) {
        let (px, py, pw, ph) = Self::preset_combo_rect();
        if self.preset_open {
            draw.rounded_rect(px, py + 3.0, pw, ph - 6.0, 4.0, THEME_DIM);
        }
        draw.text(px + 8.0, HEADER_H * 0.5 + 4.0, "Presets", 12.0, THEME);
        draw_spinner(draw, px + pw - 11.0, HEADER_H * 0.5, THEME);
    }

    pub(super) fn draw_preset_menu(&self, draw: &mut Draw<'_>) {
        if !self.preset_open {
            return;
        }
        let (mx, my, mw, mh) = self.preset_menu_rect();
        draw.rounded_rect(mx, my, mw, mh, 4.0, HEADER);
        draw.outline_rounded(mx, my, mw, mh, 4.0, THEME_DIM, 1.0);
        let users = self.params.user_presets.list();
        for (row, (ix, iy, iw, ih)) in self.preset_menu_rows() {
            match row {
                PresetRow::Rule => {
                    draw.rect(ix, iy, iw, ih, THEME_DIM);
                }
                PresetRow::Resets => {
                    if self.resets_open {
                        draw.rounded_rect(ix + 4.0, iy, iw - 8.0, ih, 3.0, THEME_DIM);
                    }
                    draw.text(ix + 12.0, iy + 15.0, "Resets", 11.0, THEME);
                    draw_chevron_right(draw, ix + iw - 14.0, iy + ih * 0.5, THEME);
                }
                PresetRow::Factory(preset) => {
                    if self.current_preset == PresetPick::Factory(preset) {
                        draw.rounded_rect(ix + 4.0, iy, iw - 8.0, ih, 3.0, THEME_DIM);
                    }
                    draw.text(ix + 12.0, iy + 15.0, preset.name(), 11.0, THEME);
                }
                PresetRow::User(i) => {
                    if self.current_preset == PresetPick::User(i) {
                        draw.rounded_rect(ix + 4.0, iy, iw - 8.0, ih, 3.0, THEME_DIM);
                    }
                    let label = users.get(i).map(|p| p.name.as_str()).unwrap_or("Preset");
                    draw.text(ix + 12.0, iy + 15.0, label, 11.0, THEME);
                }
                PresetRow::Add => {
                    draw.text(ix + 12.0, iy + 15.0, "+ Add Preset", 11.0, THEME);
                }
            }
        }
        if self.resets_open {
            let (fx, fy, fw, fh) = Self::reset_flyout_rect();
            draw.rounded_rect(fx, fy, fw, fh, 4.0, HEADER);
            draw.outline_rounded(fx, fy, fw, fh, 4.0, THEME_DIM, 1.0);
            for (i, preset) in FactoryPreset::RESETS.iter().enumerate() {
                let (ix, iy, _, _) = Self::reset_flyout_item(i);
                draw.text(ix + 12.0, iy + 15.0, preset.name(), 11.0, THEME);
            }
        }
    }

    pub(super) fn draw_add_preset(&self, draw: &mut Draw<'_>, kit_vu: &[f32; KitPieceId::COUNT]) {
        if !self.add_preset_open {
            return;
        }
        let (mx, my, mw, mh) = Self::add_preset_modal();
        paint_glass_modal(
            draw,
            &self.blur_shot,
            &self.blur_src,
            &self.blur_dst,
            &self.blur_dirty,
            mx,
            my,
            mw,
            mh,
        );
        draw_all_chan_meters(draw, kit_vu);
        draw.text(mx + 20.0, my + 28.0, "Add Preset", 14.0, THEME);
        let name_r = Self::add_name_rect((mx, my, mw, mh));
        draw.rounded_rect(name_r.0, name_r.1, name_r.2, name_r.3, 4.0, THEME_DIM);
        if self.add_name_focus {
            draw.outline_rounded(name_r.0, name_r.1, name_r.2, name_r.3, 4.0, THEME, 1.0);
        }
        let name_label = if self.add_name.is_empty() {
            "Name"
        } else {
            self.add_name.as_str()
        };
        draw.text(
            name_r.0 + 8.0,
            name_r.1 + 19.0,
            name_label,
            12.0,
            if self.add_name.is_empty() {
                THEME_DIM
            } else {
                THEME
            },
        );
        for i in 0..PresetScope::names().len() {
            let r = Self::add_scope_rect((mx, my, mw, mh), i);
            let on = self.add_scope.get(i);
            draw.rounded_rect(
                r.0,
                r.1 + 4.0,
                16.0,
                16.0,
                3.0,
                if on { THEME } else { THEME_DIM },
            );
            if on {
                draw.rounded_rect(r.0 + 4.0, r.1 + 8.0, 8.0, 8.0, 2.0, FADER_TEXT);
            }
            draw.text(r.0 + 24.0, r.1 + 17.0, PresetScope::names()[i], 12.0, THEME);
        }
        let save = Self::add_save_rect((mx, my, mw, mh));
        let cancel = Self::add_cancel_rect((mx, my, mw, mh));
        let can_save = !self.add_name.trim().is_empty() && self.add_scope.any();
        draw.rounded_rect(
            save.0,
            save.1,
            save.2,
            save.3,
            4.0,
            if can_save { THEME_DIM } else { SOF_OFF },
        );
        draw.text_centered(save.0 + save.2 * 0.5, save.1 + 18.0, "Save", 12.0, THEME);
        draw.rounded_rect(cancel.0, cancel.1, cancel.2, cancel.3, 4.0, THEME_DIM);
        draw.text_centered(
            cancel.0 + cancel.2 * 0.5,
            cancel.1 + 18.0,
            "Cancel",
            12.0,
            THEME,
        );
    }
}

fn next_preset_name(existing: &[UserPreset]) -> String {
    for i in 1..=MAX_USER_PRESETS {
        let name = format!("Preset {i}");
        if !existing.iter().any(|p| p.name == name) {
            return name;
        }
    }
    "Preset".to_string()
}
