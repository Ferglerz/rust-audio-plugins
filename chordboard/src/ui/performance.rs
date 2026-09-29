use super::*;

impl ChordboardView {
    pub(super) fn draw_performance_header(&self, d: &mut Draw) {
        d.font = self.bold_font.get();
        d.text(600.0, 111.0, "Performance", 15.0, TEXT);
        d.font = self.ui_font.get();
        d.text_right(
            778.0,
            109.0,
            &format!("{} voices", self.snapshot.voices),
            11.0,
            MUTED,
        );
        let protocol = match self.params.output_mode.value() {
            0 if self.params.mpe_enabled() => "Auto · MPE ▾",
            0 => "Auto · MIDI ▾",
            1 => "MPE ▾",
            _ => "Regular MIDI ▾",
        };
        self.button(d, MPE, protocol, self.menu == Some(Menu::Protocol), TEAL);
        self.button(
            d,
            OUTPUT,
            "Output ▾",
            self.panel == Some(Panel::Output),
            TEAL,
        );
        let synced = self.params.tempo_sync.value();
        self.button(
            d,
            TEMPO_SYNC,
            if synced { "Host tempo" } else { "Manual tempo" },
            synced,
            TEAL,
        );
        if synced {
            d.text(
                TEMPO_CONTROL.0 + 16.0,
                41.0,
                &format!("{:.1} bpm", self.snapshot.tempo),
                13.0,
                TEXT,
            );
        }
        for (i, label) in MODE_LABELS.iter().enumerate() {
            self.button(
                d,
                mode_rect(i),
                label,
                self.params.mode.value().max(1) == i as i32 + 1,
                TEAL,
            );
        }
    }

    pub(super) fn draw_direction(&self, d: &mut Draw) {
        d.font = self.ui_font.get();
        d.text(616.0, 355.0, "Direction", 11.0, MUTED);
        for (i, label) in ["Up", "Down", "Alternate"].iter().enumerate() {
            let r = direction_rect(i);
            let selected = self.params.direction.value() == i as i32;
            self.button(d, r, "", selected, TEAL);
            d.text_centered(
                r.0 + r.2 / 2.0,
                r.1 + r.3 / 2.0 + 4.0,
                label,
                12.0,
                if selected { TEAL } else { TEXT },
            );
        }
    }
    pub(super) fn draw_arp(&self, d: &mut Draw) {
        d.rounded_rect(PAD.0, PAD.1, PAD.2, PAD.3, 8.0, BG);
        d.font = self.bold_font.get();
        d.text(618.0, 183.0, "Arpeggiator", 14.0, GOLD);
        d.font = self.ui_font.get();
        let count = self.snapshot.notes.len.min(6);
        for i in 0..count {
            if let Some(note) = self.snapshot.notes.string(i) {
                let x = 840.0 + i as f32 * 40.0;
                let pulse = self.string_anim[i];
                d.circle(x, 179.0, 10.0, alpha(GOLD, pulse * 0.25), true);
                d.text_centered(
                    x,
                    183.0,
                    harmony::NOTE_NAMES[note as usize % 12],
                    TEXT_SMALL,
                    if pulse > 0.1 { GOLD } else { MUTED },
                );
            }
        }
        // Glyphs illustrate each ordering rule; they are not a playback position.
        let contours = [
            [0, 1, 2, 3, 4],
            [4, 3, 2, 1, 0],
            [0, 2, 4, 2, 0],
            [0, 3, 1, 2, 4],
            [2, 4, 0, 3, 1],
        ];
        for (i, label) in ["Up", "Down", "Up / Down", "As played", "Random"]
            .iter()
            .enumerate()
        {
            let r = pattern_rect(i);
            let selected = self.params.arp_pattern.value() == i as i32;
            let color = if selected { GOLD } else { MUTED };
            let hover = self.hover_amount(r);
            d.rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                5.0,
                if selected {
                    alpha(GOLD, 0.14 + hover * 0.05)
                } else {
                    alpha(TEXT, 0.025 + hover * 0.05)
                },
            );
            if selected {
                d.rounded_rect(r.0 + 15.0, r.1 + r.3 - 2.0, r.2 - 30.0, 2.0, 1.0, GOLD);
            }
            let points: Vec<_> = contours[i]
                .iter()
                .enumerate()
                .map(|(j, n)| {
                    (
                        r.0 + 9.0 + j as f32 * (r.2 - 18.0) / 4.0,
                        r.1 + 21.0 - *n as f32 * 3.4,
                    )
                })
                .collect();
            d.poly(&points, color, 1.2);
            for (x, y) in points {
                d.circle(x, y, 1.7, color, true);
            }
            d.text_centered(r.0 + r.2 / 2.0, r.1 + 35.0, label, 11.0, color);
        }
        d.text(618.0, 319.0, "Octaves", 11.0, MUTED);
        for i in 0..4 {
            self.button(
                d,
                octave_rect(i),
                &(i + 1).to_string(),
                self.params.octaves.value() == i as i32 + 1,
                GOLD,
            );
        }
        let header = RATE_HEADER;
        d.text(header.0, header.1 + 15.0, "Rate", 11.0, MUTED);
        d.text_right(
            header.0 + header.2,
            header.1 + 15.0,
            &format!("{:.3} beats", self.params.rate.value()),
            TEXT_SMALL,
            MUTED,
        );
        for (i, (label, beats)) in ARP_RATES.iter().enumerate() {
            self.button(
                d,
                rate_rect(i),
                label,
                (self.params.rate.value() - beats).abs() < 0.0001,
                GOLD,
            );
        }
        for (id, r) in arp_controls().into_iter().skip(1) {
            let gate = self.params.gate.value();
            let swing = self.params.swing.value();
            let is_gate = id == "gate";
            d.rounded_rect(r.0, r.1, r.2, r.3, 6.0, alpha(TEXT, 0.025));
            d.text(
                r.0 + 12.0,
                r.1 + 17.0,
                if is_gate { "Gate" } else { "Swing" },
                TEXT_SMALL,
                MUTED,
            );
            d.text_right(
                r.0 + r.2 - 12.0,
                r.1 + 17.0,
                &format!("{:.0}%", if is_gate { gate * 100.0 } else { swing * 100.0 }),
                TEXT_SMALL,
                GOLD,
            );
            let step = (r.2 - 24.0) / 8.0;
            for i in 0..8 {
                let start = if i % 2 == 0 {
                    i as f32
                } else {
                    i as f32 + swing
                };
                let duration = if i % 2 == 0 { 1.0 + swing } else { 1.0 - swing };
                let x = r.0 + 12.0 + start * step;
                d.line(
                    r.0 + 12.0 + i as f32 * step,
                    r.1 + 26.0,
                    r.0 + 12.0 + i as f32 * step,
                    r.1 + 46.0,
                    LINE,
                    1.0,
                );
                d.rounded_rect(
                    x,
                    r.1 + 29.0,
                    (step * duration * if is_gate { gate } else { 0.8 }).max(2.0),
                    13.0,
                    3.0,
                    if i % 2 == 0 { GOLD } else { alpha(GOLD, 0.5) },
                );
            }
        }
    }
    pub(super) fn draw_pad(&self, d: &mut Draw, mode: i32) {
        d.rounded_rect(PAD.0, PAD.1, PAD.2, PAD.3, 8.0, BG);
        let manual = mode == 2;
        let field = if manual {
            PLAY_PAD
        } else {
            (620.0, 203.0, 448.0, 115.0)
        };
        let (min, max) = if manual {
            strum_bounds(self.params.x_min.value(), self.params.x_max.value())
        } else {
            (0.0, 1.0)
        };
        let (y_min, y_max) =
            expression_bounds(self.params.y_min.value(), self.params.y_max.value());
        let expression_position = |value: f32| {
            y_min
                + if self.params.y_reverse.value() {
                    1.0 - value
                } else {
                    value
                } * (y_max - y_min)
        };
        let n = self.params.strings.value() as usize;
        for i in 0..n {
            let x = field.0 + (min + i as f32 * (max - min) / (n - 1) as f32) * field.2;
            let string = if manual && self.params.x_reverse.value() {
                n - 1 - i
            } else {
                i
            };
            let pulse = self.string_anim[string];
            let color = if self.snapshot.notes.string(string).is_some() {
                TEAL
            } else {
                MUTED
            };
            if pulse > 0.01 {
                d.rect(x - 5.0, field.1, 10.0, field.3, alpha(color, pulse * 0.10));
            }
            let displacement = (pulse * 24.0).sin() * pulse * 3.0;
            d.poly(
                &[
                    (x, field.1),
                    (x + displacement, field.1 + field.3 / 2.0),
                    (x, field.1 + field.3),
                ],
                alpha(color, 0.15 + pulse * 0.7),
                1.0 + pulse,
            );
            if let Some(note) = self.snapshot.notes.string(string) {
                d.text_centered(
                    x,
                    field.1 + field.3 + 14.0,
                    harmony::NOTE_NAMES[note as usize % 12],
                    TEXT_SMALL,
                    if pulse > 0.1 { color } else { MUTED },
                );
            }
        }
        d.text(
            PAD.0 + 18.0,
            PAD.1 + 25.0,
            if manual { "STRUM FIELD" } else { "AUTO STRUM" },
            TEXT_LABEL,
            TEAL,
        );
        if manual {
            d.text_right(
                PAD.0 + PAD.2 - 18.0,
                PAD.1 + 25.0,
                "X STRUM · Y EXPRESSION",
                TEXT_SMALL,
                MUTED,
            );
        } else {
            d.button(
                STRUM_SYNC,
                "SWEEP SYNC",
                self.params.strum_sync.value(),
                TEAL,
            );
            if self.params.strum_sync.value() {
                let label = ARP_RATES
                    .iter()
                    .find(|(_, beats)| (*beats - self.params.strum_beats.value()).abs() < 0.0001)
                    .map_or_else(
                        || format!("{:.3} beats", self.params.strum_beats.value()),
                        |(label, _)| label.to_string(),
                    );
                d.button(
                    Menu::StrumRate.trigger_rect(),
                    &format!("Sweep: {label} ▾"),
                    self.menu == Some(Menu::StrumRate),
                    GOLD,
                );
            }
        }
        if !manual {
            return;
        }
        {
            for &(x, y, time) in &self.trail {
                let age = time.elapsed().as_secs_f32();
                let strength = (1.0 - age / 0.35).max(0.0);
                d.circle(
                    field.0
                        + (min
                            + if self.params.x_reverse.value() {
                                1.0 - x
                            } else {
                                x
                            } * (max - min))
                            * field.2,
                    field.1 + (1.0 - expression_position(y)) * field.3,
                    3.0 + strength * 3.0,
                    alpha(TEAL, strength * 0.17),
                    true,
                );
            }
        }
        for (y_axis, low, high) in [(false, min, max), (true, y_min, y_max)] {
            for (maximum, value) in [(false, low), (true, high)] {
                let (ax, ay, pointer) = strum_bound_anchor(y_axis, value);
                if y_axis {
                    d.line(field.0, ay, field.0 + field.2, ay, alpha(GOLD, 0.35), 1.0);
                } else {
                    d.line(ax, field.1, ax, field.1 + field.3, alpha(TEAL, 0.35), 1.0);
                }
                let color = if y_axis { GOLD } else { TEAL };
                d.tag_handle(ax, ay, pointer, color);
                let dragging = matches!(self.drag, Some(Drag::StrumBound(axis, end)) if axis == y_axis && end == maximum);
                let label = if dragging {
                    format!("{value:.2}")
                } else if maximum {
                    "MAX".into()
                } else {
                    "MIN".into()
                };
                if y_axis {
                    d.text_centered(ax - 22.0, ay + 22.0, &label, TEXT_SMALL, color);
                } else if maximum {
                    d.text_right(ax - 14.0, ay - 17.0, &label, TEXT_SMALL, color);
                } else {
                    d.text(ax + 14.0, ay - 17.0, &label, TEXT_SMALL, color);
                }
            }
        }
        let position = if self.params.x_reverse.value() {
            1.0 - self.snapshot.x
        } else {
            self.snapshot.x
        };
        let x = field.0 + (min + position * (max - min)) * field.2;
        let y = field.1 + (1.0 - expression_position(self.snapshot.y)) * field.3;
        d.circle(
            x.clamp(field.0 + 8.0, field.0 + field.2 - 8.0),
            y.clamp(field.1 + 8.0, field.1 + field.3 - 8.0),
            7.0,
            TEAL,
            false,
        );
    }
    pub(super) fn spread_preview_notes(&self) -> harmony::Notes {
        if self.snapshot.full_notes.len > 0 {
            self.snapshot.full_notes
        } else {
            harmony::voice(
                (self.params.keyboard_octave.value() + self.params.key.value()).clamp(0, 127) as u8,
                self.params.quality.value() as u8,
                None,
                self.params.transpose.value() as i8,
                self.params.inversion.value() as u8,
                self.params.spread.value() as u8,
            )
        }
    }

    pub(super) fn draw_spread_keyboard(&self, d: &mut Draw, r: Rect, label: &str) {
        let notes = self.spread_preview_notes();
        let tones = notes.as_slice();
        let first = tones.first().copied().unwrap_or(48) / 12 * 12;
        let last = ((tones.last().copied().unwrap_or(60) as u16 / 12 + 1) * 12 - 1).min(127) as u8;
        let black = |note: u8| matches!(note % 12, 1 | 3 | 6 | 8 | 10);
        let whites = (first..=last).filter(|&n| !black(n)).count().max(1);
        let key_w = r.2 / whites as f32;
        let top = r.1 + 23.0;
        let height = r.3 - 25.0;
        let hovered = self.pointer.is_some_and(|(x, y)| hit(r, x, y));
        d.text(r.0, r.1 + 15.0, "Spread", TEXT_SMALL, MUTED);
        d.text_right(
            r.0 + r.2,
            r.1 + 15.0,
            &format!("{label} · click to change"),
            TEXT_SMALL,
            GOLD,
        );
        let white_key = if d.light { BG } else { alpha(TEXT, 0.65) };
        let black_key = if d.light { TEXT } else { BG };
        let mut white_index = 0;
        for note in first..=last {
            if black(note) {
                continue;
            }
            let x = r.0 + white_index as f32 * key_w;
            d.rect(
                x,
                top,
                key_w,
                height,
                if tones.contains(&note) {
                    GOLD
                } else {
                    white_key
                },
            );
            d.outline((x, top, key_w, height), BG);
            white_index += 1;
        }
        white_index = 0;
        for note in first..=last {
            if !black(note) {
                white_index += 1;
                continue;
            }
            let x = r.0 + white_index as f32 * key_w - key_w * 0.3;
            d.rect(
                x,
                top,
                key_w * 0.6,
                height * 0.62,
                if tones.contains(&note) {
                    GOLD
                } else {
                    black_key
                },
            );
            d.outline((x, top, key_w * 0.6, height * 0.62), LINE);
        }
        d.outline((r.0, top, r.2, height), if hovered { GOLD } else { LINE });
    }

    pub(super) fn draw_meters(&self, d: &mut Draw) {
        d.font = self.ui_font.get();
        for (i, label) in ["Pressure", "Timbre", "Bend"].iter().enumerate() {
            let r = meter_rect(i);
            let x = r.0 + r.2 / 2.0;
            let top = r.1 + 7.0;
            let height = 52.0;
            let bottom = top + height;
            let end = bottom - self.meters[i].clamp(0.0, 1.0) * height;
            d.rounded_rect(
                x - 5.0,
                top - 1.0,
                10.0,
                height + 2.0,
                5.0,
                alpha(TEXT, 0.055),
            );
            d.rounded_rect(x - 3.0, top, 6.0, height, 3.0, BG);
            let origin = if i == 0 { bottom } else { top + height / 2.0 };
            let amount = (end - origin).abs();
            if amount > 0.0 {
                d.rounded_rect(x - 3.0, end.min(origin), 6.0, amount, 2.5, TEAL);
            }
            for tick in 0..=4 {
                let y = top + tick as f32 * height / 4.0;
                d.line(x + 9.0, y, x + 12.0, y, alpha(MUTED, 0.35), 1.0);
            }
            if i > 0 {
                d.line(x - 8.0, origin, x + 8.0, origin, alpha(MUTED, 0.7), 1.0);
            }
            d.text_centered(x, r.1 + r.3 - 2.0, label, 11.0, MUTED);
        }
    }
}
