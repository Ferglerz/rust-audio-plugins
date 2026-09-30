use super::*;

fn note_label(note: u8) -> String {
    format!(
        "{}{}",
        harmony::NOTE_NAMES[note as usize % 12],
        note as i16 / 12 - 1
    )
}

impl ChordboardView {
    pub(super) fn split_marker(&self) -> Rect {
        let x = piano_note_x(self.params.split_note.value() as u8);
        (
            x - 8.0,
            PIANO_LEADING.1,
            16.0,
            PIANO_KEYS.1 - PIANO_LEADING.1,
        )
    }

    fn set_split_at(&self, cx: &mut EventContext, x: f32, y: f32) {
        let note = piano_note_at(x, y).unwrap_or_else(|| piano_nearest_note(x));
        cx.emit(RawParamEvent::SetParameterNormalized(
            self.params.split_note.as_ptr(),
            self.params.split_note.preview_normalized(note as i32),
        ));
    }

    pub(super) fn press_piano(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        for (r, ptr, value) in [
            (
                ALWAYS_BASS,
                self.params.always_bass.as_ptr(),
                self.params.always_bass.value(),
            ),
            (
                ALWAYS_CHORD,
                self.params.always_chord.as_ptr(),
                self.params.always_chord.value(),
            ),
            (
                KEY_SPLIT,
                self.params.key_split.as_ptr(),
                self.params.key_split.value(),
            ),
        ] {
            if hit(r, x, y) {
                Self::emit(cx, ptr, if value { 0.0 } else { 1.0 });
                return true;
            }
        }
        if hit(SPLIT_NOTE, x, y) {
            if let Some(c) = self.control("split_note") {
                self.drag = Some(Drag::Control(ControlDrag {
                    press: pleasant_ui::pointer::ValuePress::new(c.id, SPLIT_NOTE, (x, y)),
                    ptr: c.ptr,
                    norm: c.norm,
                    editable: true,
                    started: false,
                }));
                cx.capture();
            }
            return true;
        }
        if self.params.key_split.value()
            && (hit(self.split_marker(), x, y) || hit(PIANO_KEYS, x, y))
        {
            self.drag = Some(Drag::Split);
            cx.capture();
            cx.emit(RawParamEvent::BeginSetParameter(
                self.params.split_note.as_ptr(),
            ));
            self.set_split_at(cx, x, y);
            return true;
        }
        hit(PIANO_SURFACE, x, y)
    }

    pub(super) fn move_split(&self, cx: &mut EventContext, x: f32, y: f32) {
        self.set_split_at(cx, x, y);
    }

    pub(super) fn piano_hint(&self, x: f32, y: f32) -> Option<String> {
        if hit(ALWAYS_BASS, x, y) {
            return Some("Always play bass · Sustain the chord’s bass while right-hand notes are held · Hold chord keeps it until the next chord".into());
        }
        if hit(ALWAYS_CHORD, x, y) {
            return Some("Always play full chord · Sustain every chord tone while right-hand notes are held · Hold chord keeps them until the next chord".into());
        }
        if hit(KEY_SPLIT, x, y)
            || hit(SPLIT_NOTE, x, y)
            || (self.params.key_split.value() && hit(self.split_marker(), x, y))
        {
            let note = self.params.split_note.value() as u8;
            return Some(format!("Key split · Below {} selects chords · {} and above plays melody · MIDI {} · Drag marker or click a piano key; click the value to type", note_label(note), note_label(note), note));
        }
        if let Some(note) = piano_note_at(x, y) {
            let base = self.params.control_base.load(Ordering::Relaxed);
            let mut parts = vec![format!("{} · MIDI {}", note_label(note), note)];
            if base >= 0 && (base..base + 12).contains(&(note as i32)) {
                parts.push(format!(
                    "Control {} · silent",
                    harmony::CONTROL_LABELS[(note as i32 - base) as usize]
                ));
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
            if self.params.key_split.value() {
                parts.push("Click to set split".into());
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
                            note_label(a),
                            note_label(b),
                            b as i16 - a as i16
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                return Some(format!("Voice leading: {moves}"));
            }
            return Some(
                "Voice-leading paths animate above the keyboard whenever the chord voicing changes"
                    .into(),
            );
        }
        None
    }

    pub(super) fn draw_piano(&self, d: &mut Draw) {
        self.surface(d, PIANO_SURFACE);
        self.button(
            d,
            ALWAYS_BASS,
            "Always play bass",
            self.params.always_bass.value(),
            GOLD,
        );
        self.button(
            d,
            ALWAYS_CHORD,
            "Always play full chord",
            self.params.always_chord.value(),
            GOLD,
        );
        self.button(
            d,
            KEY_SPLIT,
            "Key split",
            self.params.key_split.value(),
            TEAL,
        );
        let split_note = self.params.split_note.value() as u8;
        self.button(
            d,
            SPLIT_NOTE,
            &format!("Right hand ≥ {}", note_label(split_note)),
            self.params.key_split.value(),
            TEAL,
        );
        for (x, label, color) in [
            (738.0, "Input", TEAL),
            (838.0, "Output", GOLD),
            (946.0, "Control", COLORS[4]),
        ] {
            d.circle(x, 712.0, 3.0, color, true);
            d.text(x + 10.0, 716.0, label, TEXT_SMALL, MUTED);
        }
        let base = self.params.control_base.load(Ordering::Relaxed);
        let voicing = self.spread_preview_notes();
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
                if control {
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
                        &note_label(note),
                        10.0,
                        if d.light { TEXT } else { BG },
                    );
                }
            }
        }
        let stripe_y = PIANO_KEYS.1 - 7.0;
        let split_x = piano_note_x(split_note);
        if self.params.key_split.value() {
            d.rect(
                PIANO_KEYS.0,
                stripe_y,
                split_x - PIANO_KEYS.0,
                4.0,
                alpha(GOLD, 0.7),
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
                PIANO_LEADING.1,
                split_x,
                PIANO_KEYS.1 + PIANO_KEYS.3,
                alpha(TEAL, 0.85),
                1.0,
            );
            d.circle(split_x, PIANO_LEADING.1 + 5.0, 4.0, TEAL, true);
        }
        if base >= 0 {
            let first = piano_key_rect(base.clamp(0, 127) as u8);
            let last = piano_key_rect((base + 11).clamp(0, 127) as u8);
            d.rect(first.0, stripe_y, last.0 + last.2 - first.0, 4.0, COLORS[4]);
        }
        self.draw_piano_leading(d);
    }

    fn draw_piano_leading(&self, d: &mut Draw) {
        let from = self.snapshot.leading_from;
        let to = self.snapshot.leading_to;
        if self.params.voice_leading.value() == 2 || from.len == 0 || to.len == 0 {
            d.text(
                32.0,
                747.0,
                "KEYBOARD  ·  Voicing outlines / live input & output",
                TEXT_SMALL,
                MUTED,
            );
            return;
        }
        let count = from.len.max(to.len);
        let t = self.leading_progress;
        let eased = t * t * (3.0 - 2.0 * t);
        for i in 0..count {
            let a = from.values[i * from.len / count];
            let b = to.values[i * to.len / count];
            let start = (piano_note_x(a), PIANO_LEADING.1 + 3.0);
            let end = (piano_note_x(b), PIANO_LEADING.1 + PIANO_LEADING.3 - 3.0);
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
