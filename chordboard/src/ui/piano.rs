use super::*;

impl ChordboardView {
    pub(super) fn learn_highlight_color(&self) -> Color {
        match self.learning() {
            5 => GOLD,
            6 => TEAL,
            _ => COLORS[4],
        }
    }

    fn note_label(&self, note: u8) -> String {
        harmony::midi_note_name_in_key(
            note,
            self.params.key.value() as u8,
            self.params.scale.value() as u8,
            self.params.key_spelling.value(),
        )
    }

    fn set_learned_splits(&mut self, cx: &mut EventContext, learned: crate::engine::LearnedSplit) {
        for (param, value) in [
            (&self.params.bass_split, learned.bass as i32),
            (&self.params.split_note, learned.melody as i32),
        ] {
            if param.value() != value {
                Self::emit(cx, param.as_ptr(), param.preview_normalized(value));
            }
        }
        let enabled = if learned.target == 5 {
            &self.params.bass_enabled
        } else {
            &self.params.key_split
        };
        if !enabled.value() {
            Self::emit(cx, enabled.as_ptr(), 1.0);
        }
        self.pending_learn = None;
        self.snapshot.learning = 0;
    }
    pub(super) fn receive_learned_split(&mut self, cx: &mut EventContext) {
        if let Some(learned) =
            crate::engine::LearnedSplit::decode(self.bridge.learned_split.load(Ordering::Acquire))
        {
            self.set_learned_splits(cx, learned);
        }
    }
    fn learn_split_at(&mut self, cx: &mut EventContext, target: u8, note: u8) {
        let (bass, melody) = if target == 5 {
            crate::engine::ordered_splits(note, self.params.split_note.value() as u8)
        } else {
            crate::engine::ordered_splits(self.bass_boundary().unwrap_or(36), note)
        };
        self.set_learned_splits(
            cx,
            crate::engine::LearnedSplit {
                target,
                bass,
                melody,
            },
        );
        self.request_learn(0);
    }

    pub(super) fn bass_boundary(&self) -> Option<u8> {
        let split = self.params.bass_split.value();
        let note = if split >= 0 {
            split
        } else {
            let base = self.params.control_base.load(Ordering::Relaxed);
            if base >= 0 {
                base
            } else {
                36
            }
        };
        Some(note.clamp(0, 127) as u8)
    }

    pub(super) fn bass_marker(&self) -> Option<Rect> {
        if !self.show_bass_split {
            return None;
        }
        self.bass_boundary().map(|note| {
            (
                piano_note_x(note) - 8.0,
                PIANO_SPLITS.1,
                16.0,
                PIANO_SPLITS.3,
            )
        })
    }

    pub(super) fn move_bass_split(&self, cx: &mut EventContext, x: f32, y: f32) {
        if !(PIANO_SPLITS.1..=PIANO_SPLITS.1 + PIANO_SPLITS.3).contains(&y) {
            return;
        }
        let melody = self.params.split_note.value().max(1);
        if self.params.split_note.value() < 1 {
            Self::emit(
                cx,
                self.params.split_note.as_ptr(),
                self.params.split_note.preview_normalized(melody),
            );
        }
        let note = piano_nearest_note(x).min(melody as u8 - 1);
        cx.emit(RawParamEvent::SetParameterNormalized(
            self.params.bass_split.as_ptr(),
            self.params.bass_split.preview_normalized(note as i32),
        ));
    }

    pub(super) fn split_marker(&self) -> Rect {
        let x = piano_note_x(self.params.split_note.value() as u8);
        (x - 8.0, PIANO_SPLITS.1, 16.0, PIANO_SPLITS.3)
    }

    fn set_split_at(&self, cx: &mut EventContext, x: f32, y: f32) {
        if !(PIANO_SPLITS.1..=PIANO_SPLITS.1 + PIANO_SPLITS.3).contains(&y) {
            return;
        }
        let bass = self.bass_boundary().unwrap_or(36).min(126);
        if self.bass_boundary() == Some(127) {
            Self::emit(
                cx,
                self.params.bass_split.as_ptr(),
                self.params.bass_split.preview_normalized(126),
            );
        }
        let note = piano_nearest_note(x).max(bass + 1);
        cx.emit(RawParamEvent::SetParameterNormalized(
            self.params.split_note.as_ptr(),
            self.params.split_note.preview_normalized(note as i32),
        ));
    }

    pub(super) fn control_octave_bounds(&self) -> Rect {
        let base = self.params.control_base.load(Ordering::Relaxed);
        let base = if base < 0 { 12 } else { base.min(116) } as u8;
        let first = piano_key_rect(base);
        let last = piano_key_rect(base + 11);
        (
            first.0 - 2.0,
            PIANO_KEYS.1 - 2.0,
            last.0 + last.2 - first.0 + 4.0,
            PIANO_KEYS.3 + 22.0,
        )
    }

    pub(super) fn press_piano(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        let control_octave = self.control_octave_bounds();
        if hit(control_octave, x, y)
            && (self.learning() == 0 || (self.learning() == 4 && y > PIANO_KEYS.1 + PIANO_KEYS.3))
        {
            self.request_learn(if self.learning() == 4 { 0 } else { 4 });
            return true;
        }
        if hit(inversion_control_rect(self.params.mpe_enabled(), 0), x, y) {
            self.inversion(cx, -1);
            return true;
        }
        if hit(inversion_control_rect(self.params.mpe_enabled(), 1), x, y) {
            self.inversion(cx, 1);
            return true;
        }
        for (i, step) in TRANSPOSE_STEPS.iter().enumerate() {
            if hit(transpose_control_rect(self.params.mpe_enabled(), i), x, y) {
                let value = if *step == 0 {
                    0
                } else {
                    (self.params.transpose.value() + step).clamp(-24, 24)
                };
                Self::emit(
                    cx,
                    self.params.transpose.as_ptr(),
                    self.params.transpose.preview_normalized(value),
                );
                return true;
            }
        }
        if matches!(self.learning(), 5 | 6) {
            if let Some(note) = piano_note_at(x, y) {
                self.learn_split_at(cx, self.learning(), note);
                return true;
            }
        }
        if self.learning() == 4 {
            if let Some(note) =
                piano_note_at(x, y).or_else(|| hit(PIANO_KEYS, x, y).then(|| piano_nearest_note(x)))
            {
                let base = note.saturating_sub(note % 12).min(116);
                self.params
                    .control_base
                    .store(base as i32, Ordering::Relaxed);
                self.request_learn(0);
                cx.needs_redraw();
                return true;
            }
        }
        for i in 0..3 {
            if hit(output_protocol_rect(self.params.mpe_enabled(), i), x, y) {
                Self::emit(
                    cx,
                    self.params.output_mode.as_ptr(),
                    self.params.output_mode.preview_normalized(i as i32),
                );
                return true;
            }
        }
        for (c, r) in self.keyboard_controls() {
            if hit(r, x, y) {
                if c.id == "bass_split" {
                    self.request_learn(if self.learning() == 5 { 0 } else { 5 });
                    cx.needs_redraw();
                    return true;
                }
                if keyboard_menu_range(c.id).is_some() {
                    self.open_menu(Menu::KeyboardParam(c.id));
                } else {
                    self.press_control(cx, x, y);
                }
                return true;
            }
        }
        for (r, ptr, value) in [
            (
                BASS_BYPASS,
                self.params.bass_enabled.as_ptr(),
                self.params.bass_enabled.value(),
            ),
            (
                affect_chords_rect(self.params.mpe_enabled()),
                self.params.affect_chords.as_ptr(),
                self.params.affect_chords.value(),
            ),
            (
                ALWAYS_BASS,
                self.params.always_bass.as_ptr(),
                self.params.always_bass.value(),
            ),
            (
                key_split_rect(self.params.mpe_enabled()),
                self.params.key_split.as_ptr(),
                self.params.key_split.value(),
            ),
        ] {
            if hit(r, x, y) {
                Self::emit(cx, ptr, if value { 0.0 } else { 1.0 });
                return true;
            }
        }
        if hit(melody_split_rect(self.params.mpe_enabled()), x, y) {
            self.request_learn(if self.learning() == 6 { 0 } else { 6 });
            return true;
        }
        if self.bass_marker().is_some_and(|r| hit(r, x, y)) {
            if !self.params.bass_enabled.value() {
                Self::emit(cx, self.params.bass_enabled.as_ptr(), 1.0);
            }
            self.drag = Some(Drag::BassSplit);
            cx.capture();
            cx.emit(RawParamEvent::BeginSetParameter(
                self.params.bass_split.as_ptr(),
            ));
            self.move_bass_split(cx, x, y);
            return true;
        }
        if self.voicing_bounds().is_some_and(|r| hit(r, x, y)) {
            if self.explain_modulation("spread") {
                return true;
            }
            let next_spread =
                (self.params.spread.value() + 1) % harmony::VOICING_NAMES.len() as i32;
            Self::emit(
                cx,
                self.params.spread.as_ptr(),
                self.params.spread.preview_normalized(next_spread),
            );
            return true;
        }
        if hit(self.split_marker(), x, y) {
            if !self.params.key_split.value() {
                Self::emit(cx, self.params.key_split.as_ptr(), 1.0);
            }
            self.drag = Some(Drag::Split);
            cx.capture();
            cx.emit(RawParamEvent::BeginSetParameter(
                self.params.split_note.as_ptr(),
            ));
            self.set_split_at(cx, x, y);
            return true;
        }
        if hit(PIANO_LEADING, x, y) {
            let param = &self.params.voice_leading;
            Self::emit(
                cx,
                param.as_ptr(),
                param.preview_normalized((param.value() + 1) % 3),
            );
            return true;
        }
        hit(PIANO_SURFACE, x, y)
    }

    pub(super) fn move_split(&self, cx: &mut EventContext, x: f32, y: f32) {
        self.set_split_at(cx, x, y);
    }

    pub(super) fn piano_hint(&self, x: f32, y: f32) -> Option<String> {
        if (0..2).any(|i| hit(inversion_control_rect(self.params.mpe_enabled(), i), x, y)) {
            return Some(format!(
                "Inversion {} · Move the chord down or up",
                self.snapshot.inversion
            ));
        }
        if hit(key_split_rect(self.params.mpe_enabled()), x, y) {
            return Some(
                if self.params.key_split.value() {
                    "Bypass Melody · Disable the right-hand melody split"
                } else {
                    "Enable Melody · Route right-hand notes through the melody split"
                }
                .into(),
            );
        }
        if hit(BASS_BYPASS, x, y) {
            return Some("Enable or bypass the Bass module".into());
        }
        if hit(melody_split_rect(self.params.mpe_enabled()), x, y) {
            return Some(
                "Learn Melody split from the next MIDI note or click a keyboard key".into(),
            );
        }
        if hit(affect_chords_rect(self.params.mpe_enabled()), x, y) {
            return Some("Affect Chords · Include held melody pitch classes in the chord; use other octaves to avoid exact note doubling, or omit when no octave fits".into());
        }
        if self.learning() == 4 {
            if let Some(note) = piano_note_at(x, y) {
                let base = note.saturating_sub(note % 12).min(116);
                return Some(format!(
                    "Click to set control octave to {} ({}–{})",
                    self.note_label(base),
                    self.note_label(base),
                    self.note_label(base + 11)
                ));
            }
        }
        if hit(self.control_octave_bounds(), x, y) && piano_note_at(x, y).is_none() {
            return Some("Learn the control octave from a MIDI key or click an onscreen key · Coral keys select chord qualities silently".into());
        }
        if hit(
            keyboard_control_rect("bass_split", self.params.mpe_enabled()),
            x,
            y,
        ) {
            return Some("Learn Bass split from the next MIDI note or click a keyboard key".into());
        }
        if self.bass_marker().is_some_and(|r| hit(r, x, y)) {
            return Some("Drag the gold handle above the keys to set the bass boundary · Control keys always select quality · Set Bass below to Auto to follow the control octave".into());
        }
        if hit(ALWAYS_BASS, x, y) {
            return Some("Always play bass · Sustain the chord’s bass while right-hand notes are held · Latch keeps it until the next chord".into());
        }
        if hit(key_split_rect(self.params.mpe_enabled()), x, y)
            || hit(melody_split_rect(self.params.mpe_enabled()), x, y)
            || hit(self.split_marker(), x, y)
        {
            let note = self.params.split_note.value() as u8;
            if self.params.key_split.value() {
                return Some(format!("Key split · Below {} selects chords · {} and above plays melody · MIDI {} · Drag the teal handle above the keys", self.note_label(note), self.note_label(note), note));
            } else {
                return Some(format!("Key split (off) · At {} · Drag the teal handle above the keys to turn key split on", self.note_label(note)));
            }
        }
        if let Some(r) = self.voicing_bounds() {
            if hit(r, x, y) {
                let mode_name = harmony::VOICING_NAMES[self.params.spread.value() as usize];
                return Some(format!("{mode_name} voicing · Click to cycle"));
            }
        }
        if let Some(note) = piano_note_at(x, y) {
            let base = self.params.control_base.load(Ordering::Relaxed);
            let mut parts = vec![format!("{} · MIDI {}", self.note_label(note), note)];
            if base >= 0 && (base..base + 12).contains(&(note as i32)) {
                parts.push(format!(
                    "Control {} · silent · Click to learn octave",
                    harmony::CONTROL_LABELS[(note as i32 - base) as usize]
                ));
            } else if self.params.bass_enabled.value()
                && self.bass_boundary().is_some_and(|boundary| note < boundary)
            {
                parts.push("Bass only".into());
            } else if self.params.key_split.value() {
                parts.push(if note as i32 >= self.params.split_note.value() {
                    "Right hand · melody".into()
                } else {
                    "Left hand · chord selection".into()
                });
            }
            if self.snapshot.held_notes[note as usize] {
                parts.push("Held input".into());
            }
            if self.snapshot.sounding_notes[note as usize] {
                parts.push("Sounding output".into());
            }
            if self.snapshot.full_notes.as_slice().contains(&note) {
                parts.push("Chord voicing".into());
            }
            return Some(parts.join(" · "));
        }
        if hit(PIANO_LEADING, x, y) {
            let from = self.snapshot.leading_from;
            let to = self.snapshot.leading_to;
            if self.params.voice_leading.value() != 2 && from.len > 0 && to.len > 0 {
                let count = from.len.max(to.len);
                let moves = (0..count)
                    .map(|i| {
                        let a = from.values[i * from.len / count];
                        let b = to.values[i * to.len / count];
                        format!(
                            "{} → {} ({:+})",
                            self.note_label(a),
                            self.note_label(b),
                            b as i16 - a as i16
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                return Some(format!(
                    "Voice leading: {moves} · Click to cycle Nearest / Furthest / Off"
                ));
            }
            return Some(
                "Click the paths to cycle Nearest / Furthest / Off · Paths show how each chord voice moves"
                    .into(),
            );
        }
        None
    }

    pub(super) fn draw_piano(&self, d: &mut Draw) {
        self.surface(d, PIANO_SURFACE);
        self.draw_piano_keyboard_containers(d);
        d.font = self.ui_font.get();
        self.button(
            d,
            inversion_control_rect(self.params.mpe_enabled(), 0),
            "↓",
            false,
            TEAL,
        );
        self.button(
            d,
            inversion_control_rect(self.params.mpe_enabled(), 1),
            "↑",
            false,
            TEAL,
        );
        // Row 1 additions: Always-play buttons (positioned after VL buttons via constants)
        self.button(
            d,
            ALWAYS_BASS,
            "Play Always",
            self.params.always_bass.value(),
            GOLD,
        );
        self.button(
            d,
            affect_chords_rect(self.params.mpe_enabled()),
            "Affect Chords",
            self.params.affect_chords.value(),
            if self.params.key_split.value() {
                TEAL
            } else {
                MUTED
            },
        );
        let split_note = self.params.split_note.value() as u8;
        self.button(
            d,
            melody_split_rect(self.params.mpe_enabled()),
            if self.learning() == 6 {
                "Cancel Learn"
            } else {
                "Learn Split"
            },
            self.learning() == 6,
            if self.params.key_split.value() {
                TEAL
            } else {
                MUTED
            },
        );
        let base = self.params.control_base.load(Ordering::Relaxed);
        if !self.params.bass_enabled.value() {
            let r = BASS_CONTAINER;
            d.rounded_rect(r.0, r.1, r.2, r.3, 6.0, alpha(BG, 0.45));
        }
        d.bypass_button(
            BASS_BYPASS,
            !self.params.bass_enabled.value(),
            GOLD,
            self.hover_amount(BASS_BYPASS) > 0.01,
            0.0,
        );
        if !self.params.key_split.value() {
            let r = melody_container(self.params.mpe_enabled());
            d.rounded_rect(r.0, r.1, r.2, r.3, 6.0, alpha(BG, 0.45));
        }
        d.bypass_button(
            key_split_rect(self.params.mpe_enabled()),
            !self.params.key_split.value(),
            TEAL,
            self.hover_amount(key_split_rect(self.params.mpe_enabled())) > 0.01,
            0.0,
        );
        let voicing = self.spread_preview_notes();
        let learn_hover = if matches!(self.learning(), 4..=6) {
            self.hover_pointer()
                .and_then(|(px, py)| piano_note_at(px, py))
        } else {
            None
        };
        for black in [false, true] {
            for note in (0..128u8).filter(|&n| piano_black(n) == black) {
                let r = piano_key_rect(note);
                let control = base >= 0 && (base..base + 12).contains(&(note as i32));
                let fill = if black {
                    if d.light {
                        TEXT
                    } else {
                        BG
                    }
                } else if d.light {
                    BG
                } else {
                    alpha(TEXT, 0.65)
                };
                d.rect(r.0, r.1, r.2, r.3, fill);
                if learn_hover == Some(note) {
                    d.rect(r.0 + 0.5, r.1, r.2 - 1.0, r.3, self.learn_highlight_color());
                } else if control {
                    d.rect(
                        r.0 + 0.5,
                        r.1,
                        r.2 - 1.0,
                        r.3,
                        alpha(COLORS[4], if black { 0.5 } else { 0.35 }),
                    );
                }
                d.outline(r, if black || d.light { LINE } else { BG });
                if voicing.as_slice().contains(&note) {
                    d.rounded_rect(
                        r.0 + 1.5,
                        r.1 + 2.0,
                        (r.2 - 3.0).max(1.0),
                        r.3 - 4.0,
                        1.0,
                        alpha(GOLD, if black { 0.7 } else { 0.6 }),
                    );
                    d.outline_rounded(
                        r.0 + 1.5,
                        r.1 + 2.0,
                        (r.2 - 3.0).max(1.0),
                        r.3 - 4.0,
                        1.0,
                        GOLD,
                        1.5,
                    );
                }
                let center = r.0 + r.2 / 2.0;
                if self.snapshot.sounding_notes[note as usize] {
                    d.circle(center, r.1 + r.3 - 20.0, 3.1, BG, true);
                    d.circle(center, r.1 + r.3 - 20.0, 2.4, GOLD, true);
                }
                if self.snapshot.held_notes[note as usize] {
                    d.rounded_rect(r.0 + 2.0, r.1 + 4.0, r.2 - 4.0, 5.0, 1.5, TEAL);
                }
                if note % 12 == 0 {
                    d.text_centered(
                        center,
                        r.1 + r.3 - 4.0,
                        &self.note_label(note),
                        10.0,
                        if d.light { TEXT } else { BG },
                    );
                }
            }
        }
        // Paths land on the keys; split handles and their rails paint over them.
        self.draw_piano_leading(d);
        let control_rect = self.control_octave_bounds();
        let control_hover = self
            .hover_pointer()
            .is_some_and(|(x, y)| hit(control_rect, x, y));
        if control_hover || self.learning() == 4 {
            let pulse = self.voicing_pulse.sin() * 0.5 + 0.5;
            let keys = (
                control_rect.0,
                PIANO_KEYS.1 - 2.0,
                control_rect.2,
                PIANO_KEYS.3 + 4.0,
            );
            d.rounded_rect(
                keys.0,
                keys.1,
                keys.2,
                keys.3,
                4.0,
                alpha(COLORS[4], 0.06 + 0.05 * pulse),
            );
            d.outline_rounded(
                keys.0,
                keys.1,
                keys.2,
                keys.3,
                4.0,
                alpha(COLORS[4], 0.45 + 0.45 * pulse),
                2.0,
            );
            let width = 108.0;
            let cx = (control_rect.0 + control_rect.2 * 0.5).clamp(
                PIANO_KEYS.0 + width * 0.5,
                PIANO_KEYS.0 + PIANO_KEYS.2 - width * 0.5,
            );
            let y = PIANO_KEYS.1 + PIANO_KEYS.3 + 2.0;
            d.rounded_rect(cx - width * 0.5, y, width, 16.0, 3.0, BG);
            d.outline_rounded(
                cx - width * 0.5,
                y,
                width,
                16.0,
                3.0,
                alpha(COLORS[4], 0.8),
                1.0,
            );
            d.text_centered(
                cx,
                y + 11.5,
                if self.learning() == 4 {
                    "Cancel learn"
                } else {
                    "Learn octave"
                },
                TEXT_SMALL,
                COLORS[4],
            );
        }
        let voicing_hover = self
            .hover_pointer()
            .is_some_and(|(px, py)| self.voicing_bounds().is_some_and(|r| hit(r, px, py)));
        if voicing_hover && !control_hover && self.learning() != 4 {
            if let Some(r) = self.voicing_bounds() {
                let pulse = self.voicing_pulse.sin() * 0.5 + 0.5;
                let keys_rect = (r.0, PIANO_KEYS.1 - 2.0, r.2, PIANO_KEYS.3 + 4.0);
                d.rounded_rect(
                    keys_rect.0,
                    keys_rect.1,
                    keys_rect.2,
                    keys_rect.3,
                    4.0,
                    alpha(GOLD, 0.06 + 0.05 * pulse),
                );
                d.outline_rounded(
                    keys_rect.0,
                    keys_rect.1,
                    keys_rect.2,
                    keys_rect.3,
                    4.0,
                    alpha(GOLD, 0.45 + 0.45 * pulse),
                    2.0,
                );

                let mode_name = harmony::VOICING_NAMES[self.params.spread.value() as usize];
                let pill_w = 146.0;
                let pill_h = 16.0;
                let cx = (r.0 + r.2 / 2.0).clamp(
                    PIANO_KEYS.0 + pill_w / 2.0,
                    PIANO_KEYS.0 + PIANO_KEYS.2 - pill_w / 2.0,
                );
                let pill_x = cx - pill_w / 2.0;
                let pill_y = PIANO_KEYS.1 + PIANO_KEYS.3 + 2.0;
                d.rounded_rect(pill_x, pill_y, pill_w, pill_h, 3.0, BG);
                d.outline_rounded(pill_x, pill_y, pill_w, pill_h, 3.0, alpha(GOLD, 0.8), 1.0);
                d.font = self.ui_font.get();
                d.text_centered(
                    cx,
                    pill_y + 11.5,
                    &format!("{mode_name} voicing"),
                    TEXT_SMALL,
                    GOLD,
                );
            }
        }
        for (r, color) in [
            (
                Some(self.split_marker()),
                if self.params.key_split.value() {
                    TEAL
                } else {
                    MUTED
                },
            ),
            (
                self.bass_marker(),
                if self.params.bass_enabled.value() {
                    GOLD
                } else {
                    MUTED
                },
            ),
        ] {
            if let Some(r) = r {
                let dragging = matches!(self.drag, Some(Drag::Split)) && color == TEAL
                    || matches!(self.drag, Some(Drag::BassSplit)) && color == GOLD;
                d.eq_node(
                    r.0 + r.2 / 2.0,
                    r.1 + r.3 / 2.0,
                    color,
                    dragging || self.hover_amount(r) > 0.01,
                );
            }
        }
        let stripe_y = PIANO_KEYS.1 - 7.0;
        let split_x = piano_note_x(split_note);
        if self.params.key_split.value() {
            let boundary = if !self.params.bass_enabled.value() {
                -1
            } else if self.params.bass_split.value() >= 0 {
                self.params.bass_split.value()
            } else {
                base
            };
            let bass_x = if boundary >= 0 {
                piano_note_x(boundary.min(127) as u8).min(split_x)
            } else {
                PIANO_KEYS.0
            };
            d.rect(
                PIANO_KEYS.0,
                stripe_y,
                (bass_x - PIANO_KEYS.0).max(0.0),
                4.0,
                alpha(GOLD, 0.7),
            );
            d.rect(
                bass_x,
                stripe_y,
                (split_x - bass_x).max(0.0),
                4.0,
                alpha(COLORS[1], 0.7),
            );
            d.rect(
                split_x,
                stripe_y,
                PIANO_KEYS.0 + PIANO_KEYS.2 - split_x,
                4.0,
                alpha(TEAL, 0.7),
            );
            d.line(
                split_x,
                PIANO_SPLITS.1 + PIANO_SPLITS.3 / 2.0,
                split_x,
                PIANO_KEYS.1 + PIANO_KEYS.3,
                alpha(TEAL, 0.85),
                1.0,
            );
        } else {
            let grey = alpha(MUTED, 0.35);
            d.rect(
                PIANO_KEYS.0,
                stripe_y,
                split_x - PIANO_KEYS.0,
                3.0,
                alpha(MUTED, 0.12),
            );
            d.rect(
                split_x,
                stripe_y,
                PIANO_KEYS.0 + PIANO_KEYS.2 - split_x,
                3.0,
                alpha(MUTED, 0.12),
            );
            d.line(
                split_x,
                PIANO_SPLITS.1 + PIANO_SPLITS.3 / 2.0,
                split_x,
                PIANO_KEYS.1 + PIANO_KEYS.3,
                grey,
                1.0,
            );
        }
        if self.show_bass_split && self.params.bass_enabled.value() {
            if let Some(note) = self.bass_boundary() {
                let x = piano_note_x(note);
                let bass_stripe_y = PIANO_KEYS.1 - 4.0;
                d.rect(
                    PIANO_KEYS.0,
                    bass_stripe_y,
                    (x - PIANO_KEYS.0).max(0.0),
                    3.0,
                    GOLD,
                );
                d.line(
                    x,
                    PIANO_SPLITS.1 + PIANO_SPLITS.3 / 2.0,
                    x,
                    PIANO_KEYS.1 + PIANO_KEYS.3 + 2.0,
                    alpha(GOLD, 0.85),
                    1.5,
                );
            }
        }
        if base >= 0 {
            let first = piano_key_rect(base.clamp(0, 127) as u8);
            let last = piano_key_rect((base + 11).clamp(0, 127) as u8);
            d.rect(first.0, stripe_y, last.0 + last.2 - first.0, 4.0, COLORS[4]);
        }
    }

    fn draw_piano_leading(&self, d: &mut Draw) {
        let leading_y = PIANO_LEADING.1 + PIANO_LEADING.3 * 0.5;
        let from = self.snapshot.leading_from;
        let to = self.snapshot.leading_to;
        let hover = self.hover_amount(PIANO_LEADING);
        let notes = self.snapshot.full_notes.as_slice();
        let low = from
            .as_slice()
            .iter()
            .chain(to.as_slice())
            .chain(notes)
            .copied()
            .min()
            .unwrap_or(48);
        let high = from
            .as_slice()
            .iter()
            .chain(to.as_slice())
            .chain(notes)
            .copied()
            .max()
            .unwrap_or(67);
        let x = piano_note_x(low) - 10.0;
        let width = (piano_note_x(high) - x + 10.0).max(40.0);
        let mode = self
            .routed_plain("voice_leading")
            .unwrap_or(self.params.voice_leading.value() as f32) as i32;
        let active = mode < 2;
        if active || hover > 0.005 {
            let pulse = self.voicing_pulse.sin() * 0.5 + 0.5;
            d.outline_rounded(
                x,
                PIANO_LEADING.1 - 2.0,
                width,
                PIANO_LEADING.3 + 4.0,
                4.0,
                alpha(
                    COLORS[1],
                    if active {
                        0.65 + hover * 0.25
                    } else {
                        hover * (0.45 + 0.45 * pulse)
                    },
                ),
                2.0,
            );

            let label = ["Nearest", "Furthest", "Off"][mode.clamp(0, 2) as usize];
            let cx = (x + width + 64.0).min(PIANO_LEADING.0 + PIANO_LEADING.2 - 54.0);
            d.rounded_rect(cx - 54.0, PIANO_LEADING.1 + 6.0, 108.0, 16.0, 3.0, BG);
            d.outline_rounded(
                cx - 54.0,
                PIANO_LEADING.1 + 6.0,
                108.0,
                16.0,
                3.0,
                alpha(COLORS[1], if active { 0.8 } else { hover * 0.8 }),
                1.0,
            );
            d.text_centered(
                cx,
                PIANO_LEADING.1 + 17.5,
                label,
                TEXT_SMALL,
                if active { COLORS[1] } else { MUTED },
            );
        }
        if mode == 2 || from.len == 0 || to.len == 0 {
            let notes = self.snapshot.full_notes.as_slice();
            let fallback = [48, 55, 60, 64, 67];
            for &note in if notes.is_empty() {
                &fallback[..]
            } else {
                notes
            } {
                let x = piano_note_x(note);
                let color = alpha(MUTED, 0.45 + hover * 0.4);
                d.line(x, leading_y, x, piano_key_rect(note).1 + 3.0, color, 1.5);
                d.circle(x, leading_y, 2.5, color, false);
            }
            return;
        }
        let count = from.len.max(to.len);
        let t = self.leading_progress;
        let eased = t * t * (3.0 - 2.0 * t);
        for i in 0..count {
            let a = from.values[i * from.len / count];
            let b = to.values[i * to.len / count];
            let start = (piano_note_x(a), leading_y);
            let end = (piano_note_x(b), piano_key_rect(b).1 + 3.0);
            let color = if a == b {
                TEAL
            } else if b > a {
                GOLD
            } else {
                COLORS[4]
            };
            d.line(start.0, start.1, end.0, end.1, alpha(BG, 0.9), 3.5);
            d.line(start.0, start.1, end.0, end.1, alpha(color, 0.65), 1.5);
            d.circle(start.0, start.1, 2.5, color, false);
            if t < 1.0 {
                d.circle(
                    start.0 + (end.0 - start.0) * eased,
                    start.1 + (end.1 - start.1) * eased,
                    3.0,
                    color,
                    true,
                );
            }
        }
    }

    /// Draws the three labelled containers below the keyboard:
    /// Bass (GOLD, left) · Chord (neutral, center) · Melody (TEAL, right)
    fn draw_piano_keyboard_containers(&self, d: &mut Draw) {
        let mpe = self.params.mpe_enabled();
        for (r, label, _color) in [
            (BASS_CONTAINER, "BASS", GOLD),
            (chord_container(mpe), "CHORDS", COLORS[1]),
            (
                melody_container(mpe),
                "MELODY",
                if self.params.key_split.value() {
                    TEAL
                } else {
                    MUTED
                },
            ),
        ] {
            self.surface(d, r);
            let melody = label == "MELODY";
            d.text(
                r.0 + 10.0 + if melody || label == "BASS" { 30.0 } else { 0.0 },
                module_title_y(r.1, MODULE_TITLE_SIZE),
                label,
                MODULE_TITLE_SIZE,
                if (melody && !self.params.key_split.value())
                    || (label == "BASS" && !self.params.bass_enabled.value())
                {
                    MUTED
                } else {
                    TEXT
                },
            );
        }

        // ── Controls inside containers ─────────────────────────────────────────
        // Protocol / MPE button goes in the Chord container
        for (i, label) in ["Auto", "MPE", "MIDI"].iter().enumerate() {
            self.button(
                d,
                output_protocol_rect(self.params.mpe_enabled(), i),
                label,
                self.params.output_mode.value() == i as i32,
                COLORS[1],
            );
        }

        // Channel/mode controls from keyboard_controls()
        for (c, r) in self.keyboard_controls() {
            let color = if c.id.starts_with("bass") {
                GOLD
            } else if c.id == "output_channel" {
                COLORS[1]
            } else if !self.params.key_split.value() {
                MUTED
            } else {
                TEAL
            };
            if c.id == "bass_split" {
                self.button(
                    d,
                    r,
                    if self.learning() == 5 {
                        "Cancel Learn"
                    } else {
                        "Learn Split"
                    },
                    self.learning() == 5,
                    GOLD,
                );
                continue;
            }
            let label = match c.id {
                "bass_channel" => "Ch",
                "output_channel" => "Ch",
                "upper_channel" => "Ch",
                "members" => "N",
                _ => &c.name,
            };
            if keyboard_menu_range(c.id).is_some() {
                self.button(
                    d,
                    r,
                    &format!("{label} {} ▾", self.display_value(&c)),
                    self.menu == Some(Menu::KeyboardParam(c.id)),
                    color,
                );
                if self.routed_control(&c).is_some() {
                    self.live_highlight(d, r);
                }
            } else {
                self.draw_control(d, &c, r, color);
            }
        }
    }
}
