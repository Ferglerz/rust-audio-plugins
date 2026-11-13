use super::*;

impl ScdEditorView {
    pub(super) fn handle_cc_menu_mouse_down(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        x: f32,
        y: f32,
    ) -> bool {
        if !self.cc_menu_open {
            return false;
        }
        let (mx, my, mw, mh) = Self::cc_menu_rect();
        if Self::hit(x, y, mx, my, mw, mh) {
            if let Some(n) = Self::cc_index_at(x, y, (mx, my, mw, mh)) {
                self.set_cc_number(cx, n);
            }
            self.cc_menu_open = false;
            consume_redraw(cx, window_event, meta);
            return true;
        }
        self.cc_menu_open = false;
        cx.needs_redraw();
        false
    }

    pub(super) fn handle_preset_menu_mouse_down(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        x: f32,
        y: f32,
    ) -> bool {
        if !self.preset_open {
            return false;
        }
        if self.resets_open {
            let (fx, fy, fw, fh) = Self::reset_flyout_rect();
            if Self::hit(x, y, fx, fy, fw, fh) {
                for (i, preset) in FactoryPreset::RESETS.iter().enumerate() {
                    let (ix, iy, iw, ih) = Self::reset_flyout_item(i);
                    if Self::hit(x, y, ix, iy, iw, ih) {
                        self.apply_preset(cx, *preset);
                        self.close_preset_menus();
                        consume_redraw(cx, window_event, meta);
                        return true;
                    }
                }
                nih_plug_vizia::consume_window_event(cx, window_event, meta);
                return true;
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
            return true;
        }
        false
    }

    pub(super) fn handle_samples_menu_mouse_down(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        x: f32,
        y: f32,
    ) -> bool {
        if !self.samples_open {
            return false;
        }
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
                    consume_redraw(cx, window_event, meta);
                    return true;
                }
            }
            let (ix, iy, iw, ih) = Self::mapping_name_rect(i);
            if Self::hit(x, y, ix, iy, iw, ih) {
                self.commit_note_edit();
                self.open_vel_map(cx, *kit_piece);
                consume_redraw(cx, window_event, meta);
                return true;
            }
        }
        let (mx, my, mw, mh) = Self::mapping_menu_rect();
        if Self::hit(x, y, mx, my, mw, mh) {
            nih_plug_vizia::consume_window_event(cx, window_event, meta);
            return true;
        }
        false
    }

    pub(super) fn handle_header_dropdown_mouse_down(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        x: f32,
        y: f32,
    ) -> bool {
        let (px, py, pw, ph) = Self::preset_combo_rect();
        if Self::hit(x, y, px, py, pw, ph) {
            self.preset_open = !self.preset_open;
            self.resets_open = false;
            self.samples_open = false;
            self.cc_menu_open = false;
            consume_redraw(cx, window_event, meta);
            return true;
        } else if self.preset_open {
            self.close_preset_menus();
            cx.needs_redraw();
        }

        let (sx, sy, sw, sh) = Self::samples_combo_rect();
        if Self::hit(x, y, sx, sy, sw, sh) {
            self.samples_open = !self.samples_open;
            self.close_preset_menus();
            self.cc_menu_open = false;
            consume_redraw(cx, window_event, meta);
            return true;
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
            consume_redraw(cx, window_event, meta);
            return true;
        }

        if Self::hit(x, y, CC_SELECT.0, CC_SELECT.1, CC_SELECT.2, CC_SELECT.3) {
            self.cc_menu_open = !self.cc_menu_open;
            self.close_preset_menus();
            self.samples_open = false;
            consume_redraw(cx, window_event, meta);
            return true;
        }
        false
    }
}
