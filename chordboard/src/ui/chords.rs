use super::*;

impl ChordboardView {
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
        d.text(
            325.0,
            110.0,
            &format!("ROOT {}", slot_label(self.snapshot.root)),
            TEXT_SMALL,
            TEAL,
        );
        d.text(
            325.0,
            132.0,
            &format!("COLOR {}", slot_label(self.snapshot.second)),
            TEXT_SMALL,
            GOLD,
        );
        d.button(inversion_rect(0), "↓", false, TEAL);
        d.button(inversion_rect(1), "↑", false, TEAL);
        d.text(
            124.0,
            173.0,
            &format!(
                "INVERSION {}/{}",
                self.snapshot.inversion,
                self.snapshot.full_notes.len.saturating_sub(1)
            ),
            TEXT_LABEL,
            TEXT,
        );
        d.button(LATCH, "LATCH", self.params.latch.value(), GOLD);
        d.button(
            (32.0, 190.0, 154.0, 28.0),
            if self.params.fifths.value() {
                "FIFTHS"
            } else {
                "CHROMATIC"
            },
            true,
            TEAL,
        );
        d.text_right(564.0, 208.0, "12 ROOTS · MAJ / MIN / 7", TEXT_SMALL, MUTED);
        for i in 0..KEY_COUNT {
            let (x, y, w, h) = key_rect(i);
            let amt = self.key_anim[i];
            let y = y + 2.0 * amt;
            let row = i / KEY_COLUMNS;
            let color = [TEAL, COLORS[1], GOLD][row];
            let root = harmony::keyboard_root(self.params.fifths.value(), i % KEY_COLUMNS);
            let compatible = root.is_some_and(|root| {
                harmony::in_key(
                    root,
                    harmony::row_quality(row),
                    self.params.key.value() as u8,
                    self.params.scale.value() as u8,
                )
            });
            d.rounded_rect(x, y + 3.0, w, h, 7.0, alpha(LINE, 0.6));
            d.rounded_rect(x, y, w, h, 7.0, PANEL);
            if compatible && self.params.highlight.value() {
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
                if self.params.highlight.value() {
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
                }
            } else {
                d.text(x + 4.0, y + 25.0, "--", 15.0, MUTED);
            }
            d.text_right(
                x + w - 4.0,
                y + h - 4.0,
                harmony::HINTS[i],
                TEXT_SMALL,
                if root.is_some() { MUTED } else { LINE },
            );
        }
        d.button(
            (32.0, 441.0, 158.0, 28.0),
            &format!(
                "KEY {} ▾",
                harmony::NOTE_NAMES[self.params.key.value() as usize]
            ),
            true,
            TEAL,
        );
        d.button(
            (198.0, 441.0, 228.0, 28.0),
            &format!(
                "{} ▾",
                harmony::SCALE_NAMES[self.params.scale.value() as usize]
            ),
            true,
            TEAL,
        );
        d.button(
            (434.0, 441.0, 130.0, 28.0),
            "HIGHLIGHT",
            self.params.highlight.value(),
            TEAL,
        );
        self.draw_memories(d);
        let base = self.params.control_base.load(Ordering::Relaxed);
        if base < 0 {
            d.text(
                32.0,
                570.0,
                "Set your control octave in Setup → Mapping",
                TEXT_SMALL,
                MUTED,
            );
        } else {
            for i in 0..12 {
                let x = 32.0 + i as f32 * 44.5;
                let color = if self.snapshot.quality == i as u8 {
                    GOLD
                } else {
                    MUTED
                };
                d.rect(x, 553.0, 41.0, 27.0, PANEL);
                d.outline((x, 553.0, 41.0, 27.0), LINE);
                d.text(
                    x + 3.0,
                    565.0,
                    [
                        "M", "m", "7", "M7", "m7", "dim", "aug", "6", "m6", "o7", "h7", "5",
                    ][i],
                    TEXT_SMALL,
                    color,
                );
                d.text_right(
                    x + 38.0,
                    578.0,
                    &(base + i as i32).to_string(),
                    TEXT_SMALL,
                    MUTED,
                );
            }
        }
    }
}
