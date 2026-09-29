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
    pub(super) fn draw_chords(&self, d: &mut Draw) {
        let captured = SavedChord::decode(self.snapshot.captured);
        let chord_name = captured.map_or_else(|| "PLAY A CHORD".into(), harmony::chord_name);
        d.text(
            32.0,
            128.0,
            &chord_name,
            (278.0 / (chord_name.chars().count().max(1) as f32 * 0.61)).min(29.0),
            TEXT,
        );
        let tones = self
            .snapshot
            .full_notes
            .as_slice()
            .iter()
            .map(|n| {
                format!(
                    "{}{}",
                    harmony::NOTE_NAMES[*n as usize % 12],
                    *n as i16 / 12 - 1,
                )
            })
            .collect::<Vec<_>>()
            .join("  ");
        d.text(32.0, 144.0, &tones, TEXT_SMALL, MUTED);
        let slot_label = |n: i16| {
            if n < 0 {
                "--".to_string()
            } else {
                format!("{}{}", harmony::NOTE_NAMES[n as usize % 12], n / 12 - 1)
            }
        };
        let (root, second) = self.display_input_notes();
        d.text(
            325.0,
            110.0,
            &format!("ROOT {}", slot_label(root)),
            TEXT_SMALL,
            MUTED,
        );
        d.text(
            325.0,
            132.0,
            &format!("COLOR {}", slot_label(second)),
            TEXT_SMALL,
            MUTED,
        );
        d.text_right(
            564.0,
            142.0,
            &format!("TRANSPOSE {:+} st", self.params.transpose.value()),
            TEXT_LABEL,
            TEAL,
        );
        d.button(inversion_rect(0), "↓", false, TEAL);
        d.button(inversion_rect(1), "↑", false, TEAL);
        d.text(
            124.0,
            173.0,
            &format!(
                "INV {}/{}",
                self.snapshot.inversion,
                self.snapshot.full_notes.len.saturating_sub(1)
            ),
            TEXT_LABEL,
            TEXT,
        );
        for (i, label) in ["-12", "-1", "Reset", "+1", "+12"].iter().enumerate() {
            let r = transpose_rect(i);
            let hovered = self.pointer.is_some_and(|(x, y)| hit(r, x, y));
            d.rounded_rect(r.0, r.1 + 2.0, r.2, r.3, 4.0, BG);
            d.rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                4.0,
                if hovered {
                    alpha(TEAL, 0.16)
                } else {
                    alpha(TEAL, 0.07)
                },
            );
            d.outline_rounded(
                r.0,
                r.1,
                r.2,
                r.3,
                4.0,
                if hovered { TEAL } else { LINE },
                1.0,
            );
            d.text_centered(r.0 + r.2 / 2.0, r.1 + 19.0, label, TEXT_SMALL, TEXT);
        }
        d.button(LATCH, "LATCH", self.params.latch.value(), GOLD);
        d.button(
            (32.0, 190.0, 154.0, 28.0),
            if self.params.fifths.value() {
                "FIFTHS"
            } else {
                "CHROMATIC"
            },
            false,
            TEAL,
        );
        d.button(
            LEARN_OCTAVE,
            if self.learning() == 4 {
                "CANCEL LEARN"
            } else {
                "LEARN CONTROL OCTAVE"
            },
            self.learning() == 4,
            TEAL,
        );
        d.button(QWERTY, "QWERTY", self.params.keyboard.value(), TEAL);
        for i in 0..KEY_COUNT {
            let (x, y, w, h) = key_rect(i);
            let amt = if self.key_active(i) {
                1.0
            } else {
                self.key_anim[i]
            };
            let y = y + 2.0 * amt;
            let row = i / KEY_COLUMNS;
            let color = [TEAL, COLORS[1], GOLD][row];
            let root = self.keyboard_root(i % KEY_COLUMNS);
            let compatible = root.is_some_and(|root| {
                harmony::in_key(
                    root,
                    harmony::row_quality(row),
                    self.params.key.value() as u8,
                    self.params.scale.value() as u8,
                )
            });
            d.rounded_rect(x, y + 3.0, w, h, 7.0, alpha(LINE, 0.6));
            d.rounded_rect(x, y, w, h, 7.0, BG);
            if compatible {
                d.rounded_rect(x, y, w, h, 7.0, alpha(color, 0.09));
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
            d.outline(
                (x, y, w, h),
                if self.snapshot.ignored & (1 << i) != 0 {
                    COLORS[4]
                } else if amt > 0.01 {
                    alpha(color, 0.3 + amt * 0.7)
                } else {
                    LINE
                },
            );
            if let Some(root) = root {
                let text = if amt > 0.1 { color } else { TEXT };
                d.text(
                    x + 4.0,
                    y + 24.0,
                    &format!(
                        "{}{}",
                        harmony::NOTE_NAMES[root as usize],
                        harmony::QUALITY_SUFFIX[harmony::row_quality(row) as usize]
                    ),
                    15.0,
                    text,
                );
                d.text(
                    x + 4.0,
                    y + 39.0,
                    &harmony::roman(
                        root,
                        harmony::row_quality(row),
                        self.params.key.value() as u8,
                    ),
                    TEXT_SMALL,
                    if compatible { color } else { MUTED },
                );
            } else {
                d.text(x + 4.0, y + 25.0, "--", 15.0, MUTED);
            }
            if self.params.keyboard.value() {
                d.text_right(
                    x + w - 4.0,
                    y + h - 4.0,
                    harmony::HINTS[i],
                    TEXT_SMALL,
                    if root.is_some() { MUTED } else { LINE },
                );
            }
        }
        d.button(
            (32.0, 510.0, 158.0, 28.0),
            &format!(
                "KEY {} ▾",
                harmony::NOTE_NAMES[self.params.key.value() as usize]
            ),
            false,
            TEAL,
        );
        d.button(
            (198.0, 510.0, 228.0, 28.0),
            &format!(
                "{} ▾",
                harmony::SCALE_NAMES[self.params.scale.value() as usize]
            ),
            false,
            TEAL,
        );
        self.draw_memories(d);
    }
}
