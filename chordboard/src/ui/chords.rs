use super::*;

impl ChordboardView {
    pub(super) fn current_chord_key(&self) -> Option<usize> {
        if self.snapshot.root < 0 && self.snapshot.notes.len == 0 {
            return None;
        }
        let current = SavedChord::decode(self.snapshot.captured)?;
        let matches_root = |chord: harmony::KeyboardChord| chord.root == current.root % 12;
        (0..KEY_COUNT)
            .find(|&i| {
                self.keyboard_chord(i)
                    .is_some_and(|c| matches_root(c) && c.quality == current.quality)
            })
            .or_else(|| (0..KEY_COUNT).find(|&i| self.keyboard_chord(i).is_some_and(matches_root)))
    }

    pub(super) fn chord_readout(&self) -> String {
        let Some(chord) = SavedChord::decode(self.snapshot.captured) else {
            return "—".into();
        };
        let mut name = self.named_chord(chord);
        let root = (chord.root as i16 + chord.transpose as i16).rem_euclid(12) as u8;
        let bass = self
            .snapshot
            .bass_note
            .or_else(|| self.snapshot.full_notes.as_slice().iter().copied().min());
        if let Some(bass) = bass.filter(|note| note % 12 != root) {
            name.push('/');
            name.push_str(&self.note_name(bass));
        }
        name
    }

    #[cfg(test)]
    pub(super) fn display_input_notes(&self) -> (i16, i16) {
        if self
            .routed_plain("latch")
            .map_or(self.params.latch.value(), |v| v >= 0.5)
            && self.snapshot.root < 0
            && self.snapshot.full_notes.len > 0
        {
            if let Some(chord) = SavedChord::decode(self.snapshot.captured) {
                return (chord.root as i16, chord.second.map_or(-1, i16::from));
            }
        }
        (self.snapshot.root, self.snapshot.second)
    }

    pub(super) fn chord_state_label(&self) -> &'static str {
        if self.snapshot.root >= 0 {
            "Held"
        } else if self
            .routed_plain("latch")
            .map_or(self.params.latch.value(), |v| v >= 0.5)
            && self.snapshot.full_notes.len > 0
        {
            "Latched"
        } else if self.params.key_split.value() && self.snapshot.full_notes.len > 0 {
            "Selected"
        } else if self.snapshot.notes.len > 0 {
            "Sustained"
        } else if self.can_capture() {
            "Last chord"
        } else {
            "Ready"
        }
    }

    pub(super) fn draw_chords(&self, d: &mut Draw) {
        d.font = self.ui_font.get();
        d.text(
            CHORDS_SURFACE.0 + 10.0,
            module_title_y(CHORDS_SURFACE.1, MODULE_TITLE_SIZE),
            "CHORDS",
            MODULE_TITLE_SIZE,
            TEXT,
        );
        d.rect(TRANSPOSE.0, TRANSPOSE.1, TRANSPOSE.2, TRANSPOSE.3, LINE);
        let transpose = self
            .routed_plain("transpose")
            .unwrap_or(self.params.transpose.value() as f32) as i32;
        for (i, label) in ["−12", "−1", &format!("{transpose:+} Reset"), "+1", "+12"]
            .iter()
            .enumerate()
        {
            let r = transpose_control_rect(self.params.mpe_enabled(), i);
            let flash = self
                .action_flash
                .filter(|(rect, _)| *rect == r)
                .map_or(0.0, |(_, amount)| amount);
            d.rounded_rect(
                r.0 + 1.0,
                r.1 + 1.0,
                r.2 - 2.0,
                r.3 - 2.0,
                3.0,
                alpha(TEXT, self.hover_amount(r) * 0.08 + flash * 0.07),
            );
            if i > 0 {
                d.line(r.0, r.1 + 6.0, r.0, r.1 + r.3 - 6.0, alpha(MUTED, 0.3), 1.0);
            }
            d.text_centered(
                r.0 + r.2 * 0.5,
                r.1 + r.3 * 0.5 + 4.0,
                label,
                11.0,
                if i == 2 && transpose != 0 { GOLD } else { TEXT },
            );
            if i == 2 && self.routed_plain("transpose").is_some() {
                self.live_highlight(d, r);
            }
        }
        let readout = chord_readout_rect();
        let state = self.chord_state_label();
        if state != "Ready" {
            d.text(readout.0, readout.1 + 68.0, state, 11.0, MUTED);
        }
        let name = self.chord_readout();
        d.font = self.bold_font.get();
        let title = self.fit_text(d, &name, readout.2, 22.0);
        d.text(readout.0, readout.1 + 46.0, &title, 22.0, TEXT);
        d.font = self.ui_font.get();
        let live_latch = self.routed_plain("latch");
        let latched = live_latch.map_or(self.params.latch.value(), |value| value >= 0.5);
        d.rect(LATCH.0, LATCH.1, LATCH.2, LATCH.3, BG);
        d.rect(
            LATCH.0,
            LATCH.1,
            LATCH.2,
            LATCH.3,
            alpha(
                GOLD,
                if latched { 0.22 } else { 0.06 } + self.hover_amount(LATCH) * 0.08,
            ),
        );
        d.outline_rounded(
            LATCH.0,
            LATCH.1,
            LATCH.2,
            LATCH.3,
            5.0,
            alpha(GOLD, if latched { 1.0 } else { 0.7 }),
            1.5,
        );
        d.font = self.bold_font.get();
        d.text_centered(LATCH.0 + LATCH.2 * 0.5, LATCH.1 + 36.0, "Latch", 16.0, GOLD);
        d.font = self.ui_font.get();
        d.text_centered(
            LATCH.0 + LATCH.2 * 0.5,
            LATCH.1 + 56.0,
            if latched { "ON" } else { "OFF" },
            11.0,
            if latched { GOLD } else { MUTED },
        );
        if live_latch.is_some() {
            self.live_highlight(d, LATCH);
        }
        self.button_tinted(
            d,
            Menu::Key.trigger_rect(),
            &format!("Key {} ▾", self.key_name()),
            true,
            GOLD,
        );
        self.button_tinted(
            d,
            Menu::Scale.trigger_rect(),
            &format!(
                "{} ▾",
                harmony::SCALE_NAMES[self.params.scale.value() as usize]
            ),
            true,
            GOLD,
        );
        self.button_tinted(
            d,
            ORDER,
            harmony::KEYBOARD_LAYOUTS[self.keyboard_layout() as usize],
            true,
            GOLD,
        );

        let current_key = self.current_chord_key();
        for i in 0..KEY_COUNT {
            let r = self.keyboard_key_rect(i);
            if r.2 <= 0.0 {
                continue;
            }
            let (x, mut y, w, h) = r;
            let amt = if self.key_active(i) {
                1.0
            } else {
                self.key_anim[i]
            };
            y += 2.0 * amt;
            let color = [TEAL, COLORS[1], GOLD][i / KEY_COLUMNS];
            let chord = self.keyboard_chord(i);
            let compatible = chord.is_some_and(|chord| {
                harmony::in_key(
                    chord.root,
                    chord.quality,
                    self.params.key.value() as u8,
                    self.params.scale.value() as u8,
                )
            });
            let ignored = self.snapshot.ignored & (1 << i) != 0;
            let hover = self.hover_amount(r);
            d.rounded_rect(x, y + 3.0, w, h, 7.0, alpha(LINE, 0.7));
            d.rounded_rect(x, y, w, h, 7.0, BG);
            d.rounded_rect(
                x,
                y,
                w,
                h,
                7.0,
                alpha(color, (if compatible { 0.08 } else { 0.0 }) + hover * 0.07),
            );
            let current = current_key == Some(i);
            if current {
                let pulse = 0.5 + 0.5 * self.voicing_pulse.sin();
                d.rounded_rect(x, y, w, h, 7.0, alpha(color, 0.20 + pulse * 0.13));
                d.outline_rounded(
                    x - 1.5,
                    y - 1.5,
                    w + 3.0,
                    h + 3.0,
                    8.0,
                    alpha(color, 0.65 + pulse * 0.35),
                    2.5,
                );
                d.circle(x + w - 8.0, y + 8.0, 2.5 + pulse, color, true);
            }
            if amt > 0.005 {
                d.rounded_rect(
                    x,
                    y,
                    w,
                    h,
                    7.0,
                    alpha(color, amt * (0.16 + self.meters[0] * 0.13)),
                );
                d.outline_rounded(
                    x - 1.5,
                    y - 1.5,
                    w + 3.0,
                    h + 3.0,
                    8.0,
                    alpha(color, amt * 0.45),
                    1.0,
                );
            }
            d.outline_rounded(x, y, w, h, 7.0, if ignored { COLORS[4] } else { LINE }, 1.0);
            d.outline_rounded(
                x,
                y,
                w,
                h,
                7.0,
                alpha(color, (hover * 0.4 + amt * 0.7).min(1.0)),
                1.0,
            );
            if compatible {
                d.rounded_rect(x + 8.0, y, w - 16.0, 2.0, 1.0, alpha(color, 0.65));
            }
            if let Some(recipe) = chord {
                d.font = self.bold_font.get();
                let chord = format!(
                    "{}{}",
                    self.note_name(recipe.root),
                    harmony::QUALITY_SUFFIX[recipe.quality as usize]
                );
                let key_size =
                    (18.0 * (w - 8.0) / self.text_width(d, &chord, 18.0).max(1.0)).min(18.0);
                d.text(
                    x + 4.0,
                    y + 24.0,
                    &chord,
                    key_size,
                    if current || amt > 0.1 { color } else { TEXT },
                );
                d.font = self.ui_font.get();
                let roman = harmony::roman_in_key(
                    recipe.root,
                    recipe.quality,
                    self.params.key.value() as u8,
                    self.params.scale.value() as u8,
                    self.params.key_spelling.value(),
                );
                let roman_size =
                    (11.0 * (w - 8.0) / self.text_width(d, &roman, 11.0).max(1.0)).min(11.0);
                d.text(
                    x + 4.0,
                    y + 40.0,
                    &roman,
                    roman_size,
                    if compatible { color } else { MUTED },
                );
            }
            if ignored {
                d.text(x + w - 8.0, y + 13.0, "!", 11.0, COLORS[4]);
            }
            d.font = self.font.get();
            d.text_right(x + w - 4.0, y + h - 4.0, harmony::HINTS[i], 11.0, MUTED);
        }
        d.font = self.ui_font.get();
        self.draw_memories(d);
    }
}
