use super::*;

impl ChordboardView {
    pub(super) fn display_input_notes(&self) -> (i16, i16) {
        if self.params.latch.value() && self.snapshot.root < 0 && self.snapshot.full_notes.len > 0 {
            if let Some(chord) = SavedChord::decode(self.snapshot.captured) {
                return (chord.root as i16, chord.second.map_or(-1, i16::from));
            }
        }
        (self.snapshot.root, self.snapshot.second)
    }

    pub(super) fn chord_state_label(&self) -> &'static str {
        if self.snapshot.root >= 0 {
            "Held"
        } else if self.params.latch.value() && self.snapshot.full_notes.len > 0 {
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
        d.text(32.0, 116.0, "CHORD", 11.0, MUTED);
        let state = self.chord_state_label();
        if state != "Ready" {
            d.text(117.0, 116.0, state, 11.0, MUTED);
        }
        let name = SavedChord::decode(self.snapshot.captured)
            .map_or_else(|| "—".into(), harmony::chord_name);
        d.font = self.bold_font.get();
        let size = (33.0 * 284.0 / self.text_width(d, &name, 33.0).max(1.0)).clamp(24.0, 33.0);
        let title = self.fit_text(d, &name, 284.0, size);
        d.text(32.0, 153.0, &title, size, TEXT);
        d.font = self.font.get();
        let tones = self
            .snapshot
            .full_notes
            .as_slice()
            .iter()
            .map(|n| {
                format!(
                    "{}{}",
                    harmony::NOTE_NAMES[*n as usize % 12],
                    *n as i16 / 12 - 1
                )
            })
            .collect::<Vec<_>>()
            .join("  ");
        let tones = self.fit_text(d, &tones, 284.0, 11.0);
        d.text(32.0, 174.0, &tones, 11.0, MUTED);
        d.font = self.ui_font.get();
        let base = self.params.control_base.load(Ordering::Relaxed);
        let range_label = if base >= 0 {
            format!(
                "Control {}{}–{}{}",
                harmony::NOTE_NAMES[base as usize % 12],
                base / 12 - 1,
                harmony::NOTE_NAMES[(base as usize + 11) % 12],
                (base + 11) / 12 - 1
            )
        } else {
            "Set control octave".into()
        };
        self.button(
            d,
            LEARN_OCTAVE,
            if self.learning() == 4 {
                "Listening… Cancel learn"
            } else {
                &range_label
            },
            self.learning() == 4,
            TEAL,
        );
        let note = |n: i16| {
            if n < 0 {
                "—".into()
            } else {
                format!("{}{}", harmony::NOTE_NAMES[n as usize % 12], n / 12 - 1)
            }
        };
        let (root, second) = self.display_input_notes();
        d.text(352.0, 150.0, "Root", 11.0, MUTED);
        d.text(352.0, 173.0, "Second", 11.0, MUTED);
        d.font = self.font.get();
        d.text(
            401.0,
            150.0,
            &note(root),
            12.0,
            if root >= 0 { TEAL } else { MUTED },
        );
        d.text(
            401.0,
            173.0,
            &note(second),
            12.0,
            if second >= 0 { GOLD } else { MUTED },
        );
        d.font = self.ui_font.get();
        self.button(d, LATCH, "Hold chord", self.params.latch.value(), GOLD);
        self.button(
            d,
            Menu::Key.trigger_rect(),
            &format!(
                "Key {} ▾",
                harmony::NOTE_NAMES[self.params.key.value() as usize]
            ),
            self.menu == Some(Menu::Key),
            TEAL,
        );
        self.button(
            d,
            Menu::Scale.trigger_rect(),
            &format!(
                "{} ▾",
                harmony::SCALE_NAMES[self.params.scale.value() as usize]
            ),
            self.menu == Some(Menu::Scale),
            TEAL,
        );
        self.button(
            d,
            ORDER,
            if self.params.fifths.value() {
                "Order: fifths"
            } else {
                "Chromatic"
            },
            self.params.fifths.value(),
            TEAL,
        );
        self.button(
            d,
            QWERTY,
            if !self.params.keyboard.value() {
                "Keys off"
            } else if self.focused
                && self.menu.is_none()
                && self.panel.is_none()
                && self.edit.is_none()
            {
                "Keys active"
            } else {
                "Keys enabled"
            },
            self.params.keyboard.value(),
            TEAL,
        );

        for i in 0..KEY_COUNT {
            let r = key_rect(i);
            let (x, mut y, w, h) = r;
            let amt = if self.key_active(i) {
                1.0
            } else {
                self.key_anim[i]
            };
            y += 2.0 * amt;
            let color = [TEAL, COLORS[1], GOLD][i / KEY_COLUMNS];
            let root = self.keyboard_root(i % KEY_COLUMNS);
            let compatible = root.is_some_and(|root| {
                harmony::in_key(
                    root,
                    harmony::row_quality(i / KEY_COLUMNS),
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
            if let Some(root) = root {
                d.font = self.bold_font.get();
                let chord = format!(
                    "{}{}",
                    harmony::NOTE_NAMES[root as usize],
                    harmony::QUALITY_SUFFIX[harmony::row_quality(i / KEY_COLUMNS) as usize]
                );
                let key_size =
                    (16.0 * (w - 8.0) / self.text_width(d, &chord, 16.0).max(1.0)).min(16.0);
                d.text(
                    x + 4.0,
                    y + 24.0,
                    &chord,
                    key_size,
                    if amt > 0.1 { color } else { TEXT },
                );
                d.font = self.ui_font.get();
                let roman = harmony::roman(
                    root,
                    harmony::row_quality(i / KEY_COLUMNS),
                    self.params.key.value() as u8,
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
            if self.params.keyboard.value() {
                d.font = self.font.get();
                d.text_right(x + w - 4.0, y + h - 4.0, harmony::HINTS[i], 11.0, MUTED);
            }
        }
        d.font = self.ui_font.get();
        self.draw_memories(d);
        self.draw_voicing(d);
    }

    fn draw_voicing(&self, d: &mut Draw) {
        d.text(32.0, 556.0, "Chord quality", 12.0, MUTED);
        d.text(218.0, 556.0, "Voice leading", 12.0, MUTED);
        d.text(400.0, 556.0, "Spread", 12.0, MUTED);
        d.text(32.0, 622.0, "Inversion", 11.0, MUTED);
        self.button(d, inversion_rect(0), "↓", false, TEAL);
        self.button(d, inversion_rect(1), "↑", false, TEAL);
        let label = match self.snapshot.inversion {
            0 => "Root position".into(),
            n => format!("Inversion {n}"),
        };
        d.text(110.0, 649.0, &label, 12.0, TEXT);
        d.text(
            326.0,
            622.0,
            &format!("Transpose  {:+} st", self.params.transpose.value()),
            11.0,
            MUTED,
        );
        for (i, label) in ["−12", "−1", "Reset", "+1", "+12"].iter().enumerate() {
            self.button(d, transpose_rect(i), label, false, TEAL);
        }
    }
}
