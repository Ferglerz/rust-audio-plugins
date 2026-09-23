//! Note mapping editor state, actions, and painting.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct NoteTarget {
    pub(super) kit_piece: KitPieceId,
    pub(super) art: usize,
    pub(super) note2: bool,
}

impl ScdEditorView {
    pub(super) fn mapping_art(kit_piece: KitPieceId) -> usize {
        stonehouse()
            .arts(kit_piece)
            .iter()
            .enumerate()
            .max_by_key(|(_, art)| art.layers)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    pub(super) fn set_cc_number(&self, cx: &mut EventContext, value: i32) {
        let param = &self.params.cc_number;
        let value = value.clamp(0, 127);
        self.emit_norm(cx, param.as_ptr(), param.preview_normalized(value));
    }

    pub(super) fn note_target_at(&self, x: f32, y: f32) -> Option<NoteTarget> {
        if self.samples_open {
            for (i, kit_piece) in KitPieceId::ALL.iter().enumerate() {
                for note2 in [false, true] {
                    let (nx, ny, nw, nh) = Self::mapping_note_rect(i, note2);
                    if Self::hit(x, y, nx, ny, nw, nh) {
                        return Some(NoteTarget {
                            kit_piece: *kit_piece,
                            art: Self::mapping_art(*kit_piece),
                            note2,
                        });
                    }
                }
            }
        }
        None
    }

    pub(super) fn note_label(&self, kit_piece: KitPieceId, art: usize, note2: bool) -> String {
        if let Some(edit) = &self.note_edit {
            if edit.target.kit_piece == kit_piece
                && edit.target.art == art
                && edit.target.note2 == note2
            {
                return if edit.text.is_empty() {
                    "_".to_string()
                } else {
                    edit.text.clone()
                };
            }
        }
        let over = self.params.note_maps.art_notes(kit_piece, art);
        if note2 {
            NoteMapStateDisplay::n2(kit_piece, art, over)
        } else {
            format!("{}", NoteMapStateDisplay::n1(kit_piece, art, over))
        }
    }

    pub(super) fn note_altered(&self, kit_piece: KitPieceId, art: usize, note2: bool) -> bool {
        let over = self.params.note_maps.art_notes(kit_piece, art);
        if note2 {
            crate::note_map::NoteMapState::n2_altered(kit_piece, art, over)
        } else {
            crate::note_map::NoteMapState::n1_altered(kit_piece, art, over)
        }
    }

    pub(super) fn begin_note_edit(&mut self, cx: &mut EventContext, target: NoteTarget) {
        cx.focus();
        let over = self
            .params
            .note_maps
            .art_notes(target.kit_piece, target.art);
        let buf = if target.note2 {
            NoteMapStateDisplay::n2(target.kit_piece, target.art, over)
        } else {
            format!(
                "{}",
                NoteMapStateDisplay::n1(target.kit_piece, target.art, over)
            )
        };
        let buf = if buf == "-" { String::new() } else { buf };
        let rect = self.note_rect(target);
        self.params.note_maps.arm_learn();
        self.note_edit = Some(ValueEdit::new(target, rect, buf));
    }

    pub(super) fn activate_note(&mut self, cx: &mut EventContext, target: NoteTarget) {
        if cx.modifiers().alt() {
            self.cancel_note_edit();
            self.params
                .note_maps
                .reset_slot(target.kit_piece, target.art, target.note2);
        } else {
            self.commit_note_edit();
            self.begin_note_edit(cx, target);
        }
        self.press_note = Some(target);
    }

    pub(super) fn note_edit_hits(edit: &ValueEdit<NoteTarget>, x: f32, y: f32) -> bool {
        Self::hit(x, y, edit.rect.0, edit.rect.1, edit.rect.2, edit.rect.3)
    }

    pub(super) fn note_rect(&self, target: NoteTarget) -> (f32, f32, f32, f32) {
        KitPieceId::ALL
            .iter()
            .position(|p| *p == target.kit_piece)
            .map(|i| Self::mapping_note_rect(i, target.note2))
            .unwrap_or(Self::mapping_note_rect(0, target.note2))
    }

    pub(super) fn apply_note_value(&self, target: NoteTarget, raw: &str) {
        if target.note2 {
            if raw.trim().is_empty() {
                let has_factory = stonehouse()
                    .arts(target.kit_piece)
                    .get(target.art)
                    .and_then(|a| a.note2)
                    .is_some();
                self.params
                    .note_maps
                    .set_n2(target.kit_piece, target.art, None, has_factory);
            } else if let Ok(n) = raw.trim().parse::<u8>() {
                if n <= 127 {
                    self.params
                        .note_maps
                        .set_n2(target.kit_piece, target.art, Some(n), false);
                }
            }
        } else if let Ok(n) = raw.trim().parse::<u8>() {
            if n <= 127 {
                let def = stonehouse()
                    .arts(target.kit_piece)
                    .get(target.art)
                    .map(|a| a.note1)
                    .unwrap_or(0);
                if n == def {
                    self.params
                        .note_maps
                        .set_n1(target.kit_piece, target.art, None);
                } else {
                    self.params
                        .note_maps
                        .set_n1(target.kit_piece, target.art, Some(n));
                }
            }
        }
    }

    pub(super) fn commit_note_edit(&mut self) {
        let Some(edit) = self.note_edit.take() else {
            return;
        };
        self.params.note_maps.disarm_learn();
        self.apply_note_value(edit.target, &edit.text);
    }

    pub(super) fn cancel_note_edit(&mut self) {
        self.note_edit = None;
        self.params.note_maps.disarm_learn();
    }

    pub(super) fn apply_note_learn(&mut self) -> bool {
        let Some(edit) = &self.note_edit else {
            return false;
        };
        let Some(n) = self.params.note_maps.take_learn() else {
            return false;
        };
        let target = edit.target;
        self.note_edit = None;
        self.apply_note_value(target, &n.to_string());
        true
    }

    pub(super) fn note_stage(&self, target: NoteTarget) -> (bool, bool, bool) {
        let on = self.note_edit.as_ref().is_some_and(|e| e.target == target);
        let hover = self.hover_note == Some(target);
        let press = self.press_note == Some(target);
        (on, hover, press)
    }

    pub(super) fn draw_mapping_header(&self, draw: &mut Draw<'_>) {
        let (sx, sy, sw, sh) = Self::samples_combo_rect();
        if self.samples_open {
            draw.rounded_rect(sx, sy + 3.0, sw, sh - 6.0, 4.0, THEME_DIM);
        }
        draw.text(sx + 8.0, HEADER_H * 0.5 + 4.0, "Mapping", 12.0, THEME);
        draw_spinner(draw, sx + sw - 11.0, HEADER_H * 0.5, THEME);
    }

    pub(super) fn draw_cc_header(&self, draw: &mut Draw<'_>) {
        let invert_on = self.params.invert_cc.value();
        draw_five_stage(
            draw,
            (CC_INVERT.0, 3.0, CC_INVERT.2, HEADER_H - 6.0),
            4.0,
            invert_on,
            self.hover_invert,
            self.press_invert,
        );
        draw.text_centered(
            CC_INVERT.0 + CC_INVERT.2 * 0.5,
            HEADER_H * 0.5 + 4.0,
            "Invert",
            12.0,
            THEME,
        );
        draw.text(CC_LABEL.0, HEADER_H * 0.5 + 4.0, "Hihat CC:", 12.0, THEME);
        if self.cc_menu_open {
            draw.rounded_rect(
                CC_SELECT.0,
                3.0,
                CC_SELECT.2,
                HEADER_H - 6.0,
                4.0,
                THEME_DIM,
            );
        }
        let cc_text = format!("{}", self.params.cc_number.value());
        draw.text(
            CC_SELECT.0 + 4.0,
            HEADER_H * 0.5 + 4.0,
            &cc_text,
            12.0,
            THEME,
        );
        draw_spinner(
            draw,
            CC_SELECT.0 + CC_SELECT.2 - 10.0,
            HEADER_H * 0.5,
            THEME,
        );
        let cc_shown = self.params.cc_display.load(Ordering::Relaxed);
        let cc_live = if cc_shown == 255 {
            "–".to_string()
        } else {
            format!("{cc_shown}")
        };
        draw.text_centered(
            CC_DISPLAY.0 + CC_DISPLAY.2 * 0.5,
            HEADER_H * 0.5 + 4.0,
            &cc_live,
            12.0,
            THEME,
        );
    }

    pub(super) fn draw_mapping_menu(&self, draw: &mut Draw<'_>) {
        if !self.samples_open {
            return;
        }
        let (_, menu_y, _, _) = Self::mapping_header_rect();
        let menu_y = menu_y - 4.0;
        let menu_h = (KitPieceId::COUNT + 1) as f32 * PRESET_ITEM_H + 8.0;
        draw.rounded_rect(SAMPLES_X, menu_y, SAMPLES_MENU_W, menu_h, 4.0, HEADER);
        draw.outline_rounded(
            SAMPLES_X,
            menu_y,
            SAMPLES_MENU_W,
            menu_h,
            4.0,
            THEME_DIM,
            1.0,
        );
        let (hx, hy, _, _) = Self::mapping_header_rect();
        draw.text(hx + 8.0, hy + 15.0, "N1", 10.0, THEME);
        draw.text(hx + 8.0 + MAP_NOTE_W, hy + 15.0, "N2", 10.0, THEME);
        draw.text(hx + 8.0 + MAP_NOTE_W * 2.0, hy + 15.0, "Piece", 10.0, THEME);
        for (i, kit_piece) in KitPieceId::ALL.iter().enumerate() {
            let (ix, iy, iw, ih) = Self::samples_item_rect(i);
            if self.vel_map_open == Some(*kit_piece) {
                draw.rounded_rect(ix + 4.0, iy, iw - 8.0, ih, 3.0, THEME_DIM);
            }
            for note2 in [false, true] {
                let (nx, ny, nw, nh) = Self::mapping_note_rect(i, note2);
                let art = Self::mapping_art(*kit_piece);
                let target = NoteTarget {
                    kit_piece: *kit_piece,
                    art,
                    note2,
                };
                let color = if self.note_altered(*kit_piece, art, note2) {
                    NOTE_YELLOW
                } else {
                    THEME
                };
                if let Some(edit) = self.note_edit.as_ref().filter(|e| e.target == target) {
                    draw.value_edit(edit, color);
                    continue;
                }
                let (on, hover, press) = self.note_stage(target);
                draw_five_stage(draw, (nx, ny, nw, nh), 5.0, on, hover, press);
                let label = self.note_label(*kit_piece, art, note2);
                draw.text_centered(nx + nw * 0.5, ny + nh * 0.5 + 4.0, &label, 10.0, color);
            }
            draw.text(
                ix + 8.0 + MAP_NOTE_W * 2.0,
                iy + 15.0,
                kit_piece.name(),
                11.0,
                THEME,
            );
        }
    }

    pub(super) fn draw_cc_menu(&self, draw: &mut Draw<'_>) {
        if !self.cc_menu_open {
            return;
        }
        let (mx, my, mw, mh) = Self::cc_menu_rect();
        draw.rounded_rect(mx, my, mw, mh, 4.0, HEADER);
        draw.outline_rounded(mx, my, mw, mh, 4.0, THEME_DIM, 1.0);
        let current = self.params.cc_number.value();
        for n in 0..128 {
            let (cx_cell, cy_cell, cw, ch) = Self::cc_cell_rect((mx, my, mw, mh), n);
            if n as i32 == current {
                draw.rounded_rect(cx_cell, cy_cell, cw - 2.0, ch - 2.0, 2.0, THEME_DIM);
            }
            draw.text_centered(
                cx_cell + cw * 0.5 - 1.0,
                cy_cell + ch * 0.5 + 4.0,
                &format!("{n}"),
                9.0,
                THEME,
            );
        }
    }
}

struct NoteMapStateDisplay;

impl NoteMapStateDisplay {
    fn n1(kit_piece: KitPieceId, art: usize, over: crate::note_map::ArtNotes) -> u8 {
        crate::note_map::NoteMapState::effective_n1(kit_piece, art, over)
    }

    fn n2(kit_piece: KitPieceId, art: usize, over: crate::note_map::ArtNotes) -> String {
        crate::note_map::NoteMapState::effective_n2(kit_piece, art, over)
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string())
    }
}

pub(super) fn insert_note_digit(edit: &mut ValueEdit<NoteTarget>, ch: char) -> bool {
    if !ch.is_ascii_digit() {
        return false;
    }
    if edit.text.len().saturating_sub(edit.selection().len()) >= 3 {
        return false;
    }
    edit.insert(&ch.to_string());
    true
}
