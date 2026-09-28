use super::*;

impl ChordboardView {
    pub(super) fn draw_performance_header(&self, d: &mut Draw) {
        d.text(600.0, 105.0, "PLAYBACK", TEXT_SMALL, MUTED);
        d.text_right(
            1088.0,
            105.0,
            &format!(
                "{} VOICES  /  {}",
                self.snapshot.voices,
                if self.params.mpe.value() {
                    "MPE"
                } else {
                    "MIDI"
                }
            ),
            TEXT_SMALL,
            MUTED,
        );
        for (i, label) in MODE_LABELS.iter().enumerate() {
            d.tab_button(
                mode_rect(i),
                label,
                self.params.mode.value() == i as i32,
                TEAL,
            );
        }
    }
    pub(super) fn draw_arp(&self, d: &mut Draw) {
        let main = self.arp_main();
        if main {
            d.rect(PAD.0, PAD.1, PAD.2, PAD.3, PANEL);
            d.text(618.0, 215.0, "ARPEGGIATOR", TEXT_LABEL, GOLD);
            let count = self.snapshot.notes.len.min(6);
            for i in 0..count {
                if let Some(note) = self.snapshot.notes.string(i) {
                    let x = 840.0 + i as f32 * 40.0;
                    let pulse = self.string_anim[i];
                    d.circle(x, 211.0, 10.0, alpha(GOLD, pulse * 0.25), true);
                    d.text_centered(
                        x,
                        215.0,
                        harmony::NOTE_NAMES[note as usize % 12],
                        TEXT_SMALL,
                        if pulse > 0.1 { GOLD } else { MUTED },
                    );
                }
            }
        } else {
            d.rect(32.0, 624.0, 340.0, 118.0, PANEL);
            d.text(44.0, 640.0, "NOTE PATH", TEXT_SMALL, GOLD);
            d.text_right(360.0, 640.0, "CHOOSE A CONTOUR", TEXT_SMALL, MUTED);
        }
        // Glyphs illustrate each ordering rule; they are not a playback position.
        let contours = [
            [0, 1, 2, 3, 4],
            [4, 3, 2, 1, 0],
            [0, 2, 4, 2, 0],
            [0, 3, 1, 2, 4],
            [2, 4, 0, 3, 1],
        ];
        for (i, label) in ["UP", "DOWN", "UP/DN", "PLAYED", "RANDOM"]
            .iter()
            .enumerate()
        {
            let r = pattern_rect(i, main);
            let selected = self.params.arp_pattern.value() == i as i32;
            let color = if selected { GOLD } else { MUTED };
            d.rect(
                r.0,
                r.1,
                r.2,
                r.3,
                if selected { alpha(GOLD, 0.12) } else { BG },
            );
            if selected {
                d.outline(r, GOLD);
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
            d.text_centered(r.0 + r.2 / 2.0, r.1 + 35.0, label, TEXT_SMALL, color);
        }
        d.text(
            if main { 618.0 } else { 44.0 },
            if main { 351.0 } else { 728.0 },
            "OCTAVES",
            TEXT_SMALL,
            MUTED,
        );
        for i in 0..4 {
            d.tab_button(
                octave_rect(i, main),
                &(i + 1).to_string(),
                self.params.octaves.value() == i as i32 + 1,
                GOLD,
            );
        }
        let header = rate_header(main);
        d.text(header.0, header.1 + 15.0, "RATE", TEXT_SMALL, GOLD);
        d.text_right(
            header.0 + header.2,
            header.1 + 15.0,
            &format!("{:.3} BEATS", self.params.rate.value()),
            TEXT_SMALL,
            MUTED,
        );
        for (i, (label, beats)) in ARP_RATES.iter().enumerate() {
            d.tab_button(
                rate_rect(i, main),
                label,
                (self.params.rate.value() - beats).abs() < 0.0001,
                GOLD,
            );
        }
        for (id, r) in arp_controls(main).into_iter().skip(1) {
            let gate = self.params.gate.value();
            let swing = self.params.swing.value();
            let is_gate = id == "gate";
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            d.line(r.0, r.1, r.0 + r.2, r.1, LINE, 1.0);
            d.text(
                r.0 + 12.0,
                r.1 + 17.0,
                if is_gate {
                    "GATE / NOTE LENGTH"
                } else {
                    "SWING / LONG · SHORT"
                },
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
    pub(super) fn draw_pad(&self, d: &mut Draw) {
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
                    PLAY_PAD.3,
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
                    (x + displacement, PLAY_PAD.1 + PLAY_PAD.3 / 2.0),
                    (x, PLAY_PAD.1 + PLAY_PAD.3),
                ],
                alpha(color, 0.15 + pulse * 0.7),
                1.0 + pulse,
            );
            if let Some(note) = self.snapshot.notes.string(i) {
                d.text_centered(
                    x,
                    PLAY_PAD.1 + PLAY_PAD.3 + 14.0,
                    harmony::NOTE_NAMES[note as usize % 12],
                    TEXT_SMALL,
                    if pulse > 0.1 { color } else { MUTED },
                );
            }
        }
        d.text(PAD.0 + 18.0, PAD.1 + 25.0, "STRUM FIELD", TEXT_LABEL, TEAL);
        d.text_right(
            PAD.0 + PAD.2 - 18.0,
            PAD.1 + 25.0,
            if self.params.mode.value() == 2 {
                "CC1 ↔ STRINGS"
            } else {
                "SELECT MANUAL TO STRUM"
            },
            TEXT_SMALL,
            MUTED,
        );
        if !self.params.reduced_motion.value() {
            for &(x, y, time) in &self.trail {
                let age = time.elapsed().as_secs_f32();
                let strength = (1.0 - age / 0.35).max(0.0);
                d.circle(
                    PLAY_PAD.0 + x * PLAY_PAD.2,
                    PLAY_PAD.1 + (1.0 - y) * PLAY_PAD.3,
                    3.0 + strength * 3.0,
                    alpha(TEAL, strength * 0.17),
                    true,
                );
            }
        }
        let x = PLAY_PAD.0 + self.snapshot.x * PLAY_PAD.2;
        let y = PLAY_PAD.1 + (1.0 - self.snapshot.y) * PLAY_PAD.3;
        d.circle(
            x.clamp(PLAY_PAD.0 + 8.0, PLAY_PAD.0 + PLAY_PAD.2 - 8.0),
            y.clamp(PLAY_PAD.1 + 8.0, PLAY_PAD.1 + PLAY_PAD.3 - 8.0),
            7.0,
            TEAL,
            false,
        );
    }
    pub(super) fn draw_meters(&self, d: &mut Draw) {
        for (i, label) in ["PRESSURE", "TIMBRE", "BEND"].iter().enumerate() {
            let x = 600.0 + i as f32 * 164.0;
            d.text(x, 132.0, label, TEXT_SMALL, MUTED);
            if i > 0 {
                let value = if i == 1 {
                    self.snapshot.timbre
                } else {
                    self.snapshot.bend
                };
                let signed = (value.clamp(0.0, 1.0) - 0.5) * 200.0;
                let value = if signed.abs() < 0.5 {
                    "0%".into()
                } else {
                    format!("{signed:+.0}%")
                };
                d.text_right(x + 152.0, 119.0, &value, TEXT_SMALL, TEAL);
                d.text(x + 69.0, 143.0, "−", TEXT_SMALL, MUTED);
                d.text_centered(x + 112.0, 143.0, "0", TEXT_SMALL, MUTED);
                d.text_right(x + 155.0, 143.0, "+", TEXT_SMALL, MUTED);
            }
            d.rect(x + 72.0, 123.0, 80.0, 8.0, LINE);
            if i > 0 {
                let end = x + 72.0 + 80.0 * self.meters[i];
                d.rect(
                    (x + 112.0).min(end),
                    123.0,
                    (end - (x + 112.0)).abs(),
                    8.0,
                    TEAL,
                );
                d.line(x + 112.0, 121.0, x + 112.0, 133.0, TEXT, 1.0);
            } else {
                d.rect(x + 72.0, 123.0, 80.0 * self.meters[i], 8.0, TEAL);
            }
        }
    }
}
