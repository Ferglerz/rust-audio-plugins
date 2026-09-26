use super::*;
impl ChordboardView {
    pub(super) fn render(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }
        if self.font.get().is_none() {
            self.font.set(canvas.add_font_mem(FONT_JETBRAINS_MONO).ok());
        }
        let mut d = Draw::new(
            canvas,
            prefs().light(),
            bounds.w / W,
            bounds.x,
            bounds.y,
            self.font.get(),
        );
        d.rect(0.0, 0.0, W, H, BG);
        d.rect(0.0, 0.0, W, 76.0, PANEL);
        d.line(0.0, 75.0, W, 75.0, LINE, 1.0);
        d.text(32.0, 44.0, "CHORDBOARD", 23.0, GOLD);
        d.text(34.0, 62.0, "HARMONY IN YOUR HANDS", 8.0, MUTED);
        d.button(
            (280.0, 24.0, 194.0, 30.0),
            PRESETS[self.preset],
            false,
            TEAL,
        );
        d.button((482.0, 24.0, 62.0, 30.0), "LOAD", false, TEAL);
        d.button((552.0, 24.0, 62.0, 30.0), "SAVE", false, TEAL);
        d.text(648.0, 44.0, "PLEASANT UI", 10.0, MUTED);
        d.button(
            (814.0, 24.0, 112.0, 30.0),
            "QWERTY",
            self.params.keyboard.value(),
            TEAL,
        );
        d.button(
            (936.0, 24.0, 70.0, 30.0),
            if prefs().light() { "LIGHT" } else { "DARK" },
            false,
            TEAL,
        );
        d.button((1016.0, 24.0, 72.0, 30.0), "PANIC", false, COLORS[4]);
        let pulse = self.pulse.step();
        if pulse > 0.0 {
            d.outline((1016.0, 24.0, 72.0, 30.0), alpha(COLORS[4], pulse));
        }
        for (i, label) in ["CHORD", "AUTO STRUM", "MANUAL STRUM", "ARPEGGIATOR"]
            .iter()
            .enumerate()
        {
            d.tab_button(
                (32.0 + i as f32 * 142.0, 91.0, 134.0, 34.0),
                label,
                self.params.mode.value() == i as i32,
                TEAL,
            );
        }
        d.button(
            (632.0, 91.0, 144.0, 34.0),
            if self.snapshot.learning == 4 {
                "PLAY LOWEST KEY"
            } else {
                "LEARN LOW OCTAVE"
            },
            self.snapshot.learning == 4,
            GOLD,
        );
        d.button(
            (784.0, 91.0, 144.0, 34.0),
            "LATCH",
            self.params.latch.value(),
            GOLD,
        );
        d.button(
            (936.0, 91.0, 152.0, 34.0),
            "MPE OUTPUT",
            self.params.mpe.value(),
            TEAL,
        );
        let captured = SavedChord::decode(self.snapshot.captured);
        let chord_name = captured.map_or_else(|| "PLAY A CHORD".into(), harmony::chord_name);
        d.text(
            32.0,
            169.0,
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
                    "{}{} [{}]",
                    harmony::NOTE_NAMES[*n as usize % 12],
                    *n as i16 / 12 - 1,
                    n
                )
            })
            .collect::<Vec<_>>()
            .join("  ");
        d.text(32.0, 184.0, &tones, 8.0, MUTED);
        let slot_label = |n: i16| {
            if n < 0 {
                "--".to_string()
            } else {
                format!("{}{}", harmony::NOTE_NAMES[n as usize % 12], n / 12 - 1)
            }
        };
        d.text(
            325.0,
            151.0,
            &format!("ROOT {}", slot_label(self.snapshot.root)),
            10.0,
            TEAL,
        );
        d.text(
            325.0,
            173.0,
            &format!("COLOR {}", slot_label(self.snapshot.second)),
            10.0,
            GOLD,
        );
        d.text_right(
            1088.0,
            153.0,
            &format!(
                "{} VOICES  /  {}",
                self.snapshot.voices,
                if self.params.mpe.value() {
                    "MPE"
                } else {
                    "MIDI"
                }
            ),
            10.0,
            MUTED,
        );
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
        for i in 0..2 {
            d.button(
                (194.0 + i as f32 * 76.0, 190.0, 68.0, 28.0),
                &format!("{} BANK", i + 1),
                self.params.bank.value() == i,
                TEAL,
            );
        }
        d.text_right(564.0, 208.0, "MAJ / MIN / 7", 10.0, MUTED);
        for i in 0..21 {
            let (x, y, w, h) = key_rect(i);
            let amt = self.key_anim[i];
            let y = y + 2.0 * amt;
            let row = i / 7;
            let color = [TEAL, COLORS[1], GOLD][row];
            let root = harmony::keyboard_root(
                self.params.fifths.value(),
                self.params.bank.value() as u8,
                i % 7,
            );
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
                    x + 9.0,
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
                        x + 9.0,
                        y + 45.0,
                        &harmony::roman(
                            root,
                            harmony::row_quality(row),
                            self.params.key.value() as u8,
                        ),
                        8.0,
                        if compatible { color } else { MUTED },
                    );
                }
            } else {
                d.text(x + 9.0, y + 25.0, "--", 15.0, MUTED);
            }
            d.text_right(
                x + w - 7.0,
                y + h - 8.0,
                harmony::HINTS[i],
                10.0,
                if root.is_some() { MUTED } else { LINE },
            );
        }
        d.button(
            (32.0, 441.0, 158.0, 28.0),
            &format!(
                "KEY {}",
                harmony::NOTE_NAMES[self.params.key.value() as usize]
            ),
            true,
            TEAL,
        );
        d.button(
            (198.0, 441.0, 228.0, 28.0),
            harmony::SCALE_NAMES[self.params.scale.value() as usize],
            true,
            TEAL,
        );
        d.button(
            (434.0, 441.0, 130.0, 28.0),
            "HIGHLIGHT",
            self.params.highlight.value(),
            TEAL,
        );
        d.text(32.0, 492.0, "CHORD MEMORIES", 10.0, MUTED);
        d.text_right(564.0, 492.0, "SHIFT + CLICK TO CAPTURE", 8.0, MUTED);
        for i in 0..8 {
            let rect = (32.0 + i as f32 * 67.0, 504.0, 61.0, 46.0);
            let chord = SavedChord::decode(self.params.slot(i).load(Ordering::Relaxed));
            d.rect(rect.0, rect.1, rect.2, rect.3, PANEL);
            d.outline(
                rect,
                if chord.is_some() {
                    alpha(GOLD, 0.45)
                } else {
                    LINE
                },
            );
            d.text(
                rect.0 + 7.0,
                rect.1 + 13.0,
                &(i + 1).to_string(),
                8.0,
                MUTED,
            );
            let label = chord.map_or_else(|| "--".into(), harmony::chord_name);
            d.text_centered(
                rect.0 + rect.2 / 2.0,
                rect.1 + 33.0,
                &label,
                12.0,
                if chord.is_some() { GOLD } else { MUTED },
            );
        }
        let base = self.params.control_base.load(Ordering::Relaxed);
        if base < 0 {
            d.text(
                32.0,
                570.0,
                "SETUP: Learn your keyboard's lowest key to reserve its control octave.",
                8.0,
                GOLD,
            );
        } else {
            for i in 0..12 {
                let x = 32.0 + i as f32 * 44.5;
                let color = if self.snapshot.quality == i as u8 {
                    GOLD
                } else {
                    MUTED
                };
                d.rect(x, 556.0, 41.0, 22.0, PANEL);
                d.outline((x, 556.0, 41.0, 22.0), LINE);
                d.text(
                    x + 3.0,
                    570.0,
                    [
                        "M", "m", "7", "M7", "m7", "dim", "aug", "6", "m6", "o7", "h7", "5",
                    ][i],
                    8.0,
                    color,
                );
                d.text_right(x + 38.0, 574.0, &(base + i as i32).to_string(), 6.0, MUTED);
            }
        }
        self.draw_pad(&mut d);
        d.button((600.0, 516.0, 40.0, 34.0), "↓", false, TEAL);
        d.button((648.0, 516.0, 40.0, 34.0), "↑", false, TEAL);
        d.text(
            700.0,
            538.0,
            &format!(
                "INVERSION {}/{}",
                self.snapshot.inversion,
                self.snapshot.full_notes.len.saturating_sub(1)
            ),
            11.0,
            TEXT,
        );
        d.button(
            (856.0, 516.0, 112.0, 34.0),
            ["MAP X", "MAP Y", "MAP GATE"][self.mapping_axis],
            false,
            TEAL,
        );
        d.button(
            (976.0, 516.0, 112.0, 34.0),
            "MIDI LEARN",
            self.snapshot.learning == self.mapping_axis as u8 + 1,
            GOLD,
        );
        let m = crate::engine::Mapping::decode(
            self.params
                .mapping(self.mapping_axis)
                .load(Ordering::Relaxed),
        );
        d.text(
            600.0,
            571.0,
            &format!(
                "INPUT: {} (CLICK TO CHANGE)",
                ["OFF", "CC 7-BIT", "CC 14-BIT", "PITCH BEND"][m.kind as usize]
            ),
            8.0,
            MUTED,
        );
        d.text(
            856.0,
            571.0,
            &format!(
                "CH {}",
                if m.channel == 16 {
                    "ANY".into()
                } else {
                    (m.channel + 1).to_string()
                }
            ),
            9.0,
            TEAL,
        );
        d.text(976.0, 571.0, &format!("CC {}", m.number), 9.0, TEAL);
        for (i, label) in GROUPS.iter().enumerate() {
            d.tab_button(
                (32.0 + i as f32 * 157.0, 585.0, 149.0, 28.0),
                label,
                self.group == i,
                GOLD,
            );
        }
        let pages = self
            .params
            .controls()
            .iter()
            .filter(|c| c.group == self.group)
            .count()
            .div_ceil(8);
        d.button(
            (982.0, 585.0, 106.0, 28.0),
            &format!("{}/{}  >", self.page + 1, pages),
            false,
            TEAL,
        );
        for (i, c) in self.shown_controls().iter().enumerate() {
            d.control(
                control_rect(i),
                &c.name,
                &c.value,
                c.norm,
                if self.group == 2 { TEAL } else { GOLD },
            );
        }
        if let Some(edit) = &self.edit {
            d.value_edit(edit, GOLD);
        }
        let footer = if self.status.is_empty() {
            "Click the keyboard to focus • ↑ / ↓ inversions • CC1 strums in Manual mode • Double-click values to type"
        } else {
            &self.status
        };
        d.text(
            32.0,
            767.0,
            &footer.chars().take(145).collect::<String>(),
            8.0,
            MUTED,
        );
    }
    fn draw_pad(&self, d: &mut Draw) {
        d.rounded_rect(PAD.0, PAD.1, PAD.2, PAD.3, 9.0, PANEL);
        d.outline(PAD, LINE);
        let n = self.params.strings.value() as usize;
        for i in 0..n {
            let x = PAD.0 + 20.0 + i as f32 * (PAD.2 - 40.0) / (n - 1) as f32;
            let pulse = self.string_anim[i];
            let color = if self.snapshot.notes.string(i).is_some() {
                TEAL
            } else {
                MUTED
            };
            if pulse > 0.01 {
                d.rect(
                    x - 5.0,
                    PAD.1 + 45.0,
                    10.0,
                    PAD.3 - 82.0,
                    alpha(color, pulse * 0.10),
                );
            }
            let displacement = if self.params.reduced_motion.value() {
                0.0
            } else {
                (pulse * 24.0).sin() * pulse * 3.0
            };
            d.poly(
                &[
                    (x, PAD.1 + 45.0),
                    (x + displacement, PAD.1 + PAD.3 / 2.0),
                    (x, PAD.1 + PAD.3 - 38.0),
                ],
                alpha(color, 0.15 + pulse * 0.7),
                1.0 + pulse,
            );
            if let Some(note) = self.snapshot.notes.string(i) {
                d.text_centered(
                    x,
                    PAD.1 + PAD.3 - 15.0,
                    harmony::NOTE_NAMES[note as usize % 12],
                    9.0,
                    if pulse > 0.1 { color } else { MUTED },
                );
            }
        }
        d.text(PAD.0 + 18.0, PAD.1 + 25.0, "STRUM FIELD", 11.0, TEAL);
        d.text_right(
            PAD.0 + PAD.2 - 18.0,
            PAD.1 + 25.0,
            if self.params.mode.value() == 2 {
                "CC1 ↔ STRINGS"
            } else {
                "SELECT MANUAL TO STRUM"
            },
            9.0,
            MUTED,
        );
        if !self.params.reduced_motion.value() {
            for &(x, y, time) in &self.trail {
                let age = time.elapsed().as_secs_f32();
                let strength = (1.0 - age / 0.35).max(0.0);
                d.circle(
                    PAD.0 + x * PAD.2,
                    PAD.1 + (1.0 - y) * PAD.3,
                    3.0 + strength * 3.0,
                    alpha(TEAL, strength * 0.17),
                    true,
                );
            }
        }
        let x = PAD.0 + self.snapshot.x * PAD.2;
        let y = PAD.1 + (1.0 - self.snapshot.y) * PAD.3;
        d.circle(
            x.clamp(PAD.0 + 8.0, PAD.0 + PAD.2 - 8.0),
            y.clamp(PAD.1 + 8.0, PAD.1 + PAD.3 - 8.0),
            7.0,
            TEAL,
            false,
        );
        // Engine-owned voices, with positions animated independently from MIDI timing.
        for i in 0..128 {
            if self.note_alpha[i] > 0.01 {
                let px = (600.0 + (self.note_positions[i] - 36.0) * 5.0).clamp(600.0, 930.0);
                d.circle(px, 171.0, 3.0, alpha(GOLD, self.note_alpha[i]), true);
            }
        }
        for (i, label) in ["P", "T", "B"].iter().enumerate() {
            let x = 950.0 + i as f32 * 46.0;
            d.text(x, 178.0, label, 8.0, MUTED);
            d.rect(x + 12.0, 169.0, 25.0, 6.0, LINE);
            d.rect(x + 12.0, 169.0, 25.0 * self.meters[i], 6.0, TEAL);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyboard_is_staggered_and_hit_boxes_do_not_overlap() {
        assert_eq!(key_rect(7).0 - key_rect(0).0, 17.0);
        assert_eq!(key_rect(14).0 - key_rect(0).0, 51.0);
        for i in 0..21 {
            let r = key_rect(i);
            assert!(hit(r, r.0 + r.2 / 2.0, r.1 + r.3 / 2.0));
            for j in 0..21 {
                if i != j {
                    assert!(!hit(key_rect(j), r.0 + r.2 / 2.0, r.1 + r.3 / 2.0));
                }
            }
        }
    }
}
