use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) struct PianoNoteState {
    pub string_idx: usize,
    pub note: u8,
    pub is_in_loop: bool,
    pub is_muted: bool,
    pub is_skipped: bool,
    pub volume: f32,
}

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

    pub(super) fn piano_display_notes(&self, mode: i32) -> Vec<PianoNoteState> {
        if matches!(mode, 1 | 3) {
            let base_notes = if self.snapshot.notes.len > 0 {
                self.snapshot.notes
            } else {
                self.spread_preview_notes()
            };
            if base_notes.len == 0 {
                return Vec::new();
            }
            let octaves = self
                .routed_plain("octaves")
                .unwrap_or(self.params.octaves.value() as f32) as usize;
            let base_count = (base_notes.len * octaves).min(32);
            let loop_start = self.params.loop_start.value() as usize;
            let loop_end = self.params.loop_end.value() as usize;
            let count = if mode == 3 {
                base_count.max(loop_end).max(loop_start)
            } else {
                base_count
            };
            let effective_end = if loop_end > 0 { loop_end } else { base_count };
            let notes_len = base_notes.len.max(1);

            let mut list = Vec::with_capacity(count);
            for i in 0..count {
                let octave = i / notes_len;
                let string = i % notes_len;
                if let Some(note) = base_notes.string(string) {
                    let pitch = (note as usize + octave * 12).min(127) as u8;
                    let str_1 = i + 1;
                    let is_in_loop = if mode == 3 {
                        str_1 >= loop_start && str_1 <= effective_end
                    } else {
                        true
                    };
                    let is_muted = mode == 3 && self.params.is_string_muted(i);
                    let is_skipped = mode == 3 && self.params.is_string_skipped(i);
                    let volume = if mode == 3 {
                        self.params.string_volume(i)
                    } else {
                        1.0
                    };
                    list.push(PianoNoteState {
                        string_idx: i,
                        note: pitch,
                        is_in_loop,
                        is_muted,
                        is_skipped,
                        volume,
                    });
                }
            }
            list
        } else {
            let voicing = self.spread_preview_notes();
            voicing
                .as_slice()
                .iter()
                .enumerate()
                .map(|(i, &note)| PianoNoteState {
                    string_idx: i,
                    note,
                    is_in_loop: true,
                    is_muted: false,
                    is_skipped: false,
                    volume: 1.0,
                })
                .collect()
        }
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
        let mpe = self.params.mpe_enabled();
        if hit(routing_preset_rect(mpe), x, y) {
            let current = self.params.routing_preset.value();
            let next = if current == 0 { 1 } else { 0 };
            Self::emit(
                cx,
                self.params.routing_preset.as_ptr(),
                self.params.routing_preset.preview_normalized(next),
            );
            if next == 0 {
                Self::emit(cx, self.params.output_channel.as_ptr(), self.params.output_channel.preview_normalized(1));
                Self::emit(cx, self.params.comp_channel.as_ptr(), self.params.comp_channel.preview_normalized(1));
                Self::emit(cx, self.params.bass_channel.as_ptr(), self.params.bass_channel.preview_normalized(1));
            } else {
                Self::emit(cx, self.params.output_channel.as_ptr(), self.params.output_channel.preview_normalized(1));
                Self::emit(cx, self.params.comp_channel.as_ptr(), self.params.comp_channel.preview_normalized(2));
                Self::emit(cx, self.params.bass_channel.as_ptr(), self.params.bass_channel.preview_normalized(3));
            }
            cx.needs_redraw();
            return true;
        }
        if hit(bass_octave_rect(), x, y) {
            let cur = self.params.bass_octave.value();
            let next = match cur {
                -1 => -2,
                -2 => 0,
                _ => -1,
            };
            Self::emit(
                cx,
                self.params.bass_octave.as_ptr(),
                self.params.bass_octave.preview_normalized(next),
            );
            cx.needs_redraw();
            return true;
        }
        if hit(ALWAYS_BASS, x, y) {
            let cur = self.params.bass_mode.value();
            let next = (cur + 1) % 5;
            Self::emit(
                cx,
                self.params.bass_mode.as_ptr(),
                self.params.bass_mode.preview_normalized(next),
            );
            Self::emit(
                cx,
                self.params.always_bass.as_ptr(),
                if next == 1 { 1.0 } else { 0.0 },
            );
            cx.needs_redraw();
            return true;
        }
        if hit(melody_split_rect(self.params.mpe_enabled()), x, y) {
            if self.params.key_split.value() || self.params.comp_mode.value() != 2 {
                self.request_learn(if self.learning() == 6 { 0 } else { 6 });
                return true;
            }
        }
        if !self.params.key_split.value() {
            if hit(comp_mode_rect(mpe), x, y) {
                let cur = self.params.comp_mode.value();
                let next = (cur + 1) % 3;
                Self::emit(
                    cx,
                    self.params.comp_mode.as_ptr(),
                    self.params.comp_mode.preview_normalized(next),
                );
                cx.needs_redraw();
                return true;
            }
            if hit(comp_octave_rect(mpe), x, y) {
                let cur = self.params.comp_octave.value();
                let next = match cur {
                    0 => 1,
                    1 => -1,
                    _ => 0,
                };
                Self::emit(
                    cx,
                    self.params.comp_octave.as_ptr(),
                    self.params.comp_octave.preview_normalized(next),
                );
                cx.needs_redraw();
                return true;
            }
            if self.params.comp_mode.value() == 2 {
                if hit(comp_rhythm_rect(mpe), x, y) {
                    self.open_menu(Menu::CompRhythm);
                    return true;
                }
                if hit(comp_lag_rect(mpe), x, y) {
                    let cur = self.params.comp_lag_ms.value();
                    let next = if (cur - 0.0).abs() < 1.0 {
                        12.0
                    } else if (cur - 12.0).abs() < 1.0 {
                        25.0
                    } else if (cur - 25.0).abs() < 1.0 {
                        -10.0
                    } else {
                        0.0
                    };
                    Self::emit(cx, self.params.comp_lag_ms.as_ptr(), self.params.comp_lag_ms.preview_normalized(next));
                    cx.needs_redraw();
                    return true;
                }
                if hit(comp_interlock_rect(mpe), x, y) {
                    let cur = self.params.comp_interlock.value();
                    let next = (cur + 1) % 3;
                    Self::emit(cx, self.params.comp_interlock.as_ptr(), self.params.comp_interlock.preview_normalized(next));
                    cx.needs_redraw();
                    return true;
                }
            }
            if hit(keyboard_control_rect("comp_channel", mpe), x, y) {
                self.open_menu(Menu::KeyboardParam("comp_channel"));
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
        if self.params.mode.value() == 3 {
            let base_notes = if self.snapshot.notes.len > 0 {
                self.snapshot.notes
            } else {
                self.spread_preview_notes()
            };
            if base_notes.len > 0 {
                let octaves = self
                    .routed_plain("octaves")
                    .unwrap_or(self.params.octaves.value() as f32) as usize;
                let base_count = (base_notes.len * octaves).min(32);
                let loop_start_val = self.params.loop_start.value() as usize;
                let loop_end_val = self.params.loop_end.value() as usize;
                let count = base_count.max(loop_end_val).max(loop_start_val);
                let effective_end = if loop_end_val > 0 { loop_end_val } else { base_count };

                if hit(PIANO_ARP_LOOP_RAIL, x, y) {
                    let start_idx = loop_start_val.saturating_sub(1).min(count - 1);
                    let end_idx = effective_end.saturating_sub(1).min(count - 1);
                    let p_start = (base_notes.string(start_idx % base_notes.len).unwrap_or(60) as usize + (start_idx / base_notes.len) * 12).min(127) as u8;
                    let p_end = (base_notes.string(end_idx % base_notes.len).unwrap_or(60) as usize + (end_idx / base_notes.len) * 12).min(127) as u8;
                    let x_start = piano_note_x(p_start);
                    let x_end = piano_note_x(p_end);
                    let (xs, xe) = if (x_start - x_end).abs() < 16.0 {
                        (x_start - 8.0, x_end + 8.0)
                    } else {
                        (x_start, x_end)
                    };

                    if (x - xs).abs() < 14.0 || (x - xe).abs() < 14.0 {
                        let is_end = (x - xe).abs() < (x - xs).abs();
                        self.drag = Some(Drag::PianoArpLoopBound(is_end));
                        cx.capture();
                        let ptr = if is_end {
                            self.params.loop_end.as_ptr()
                        } else {
                            self.params.loop_start.as_ptr()
                        };
                        cx.emit(RawParamEvent::BeginSetParameter(ptr));
                        return true;
                    }

                    if let Some(origin_string) = piano_nearest_chord_string(x, base_notes.as_slice(), count) {
                        self.drag = Some(Drag::PianoArpLoopDraw { origin_string });
                        cx.capture();
                        cx.emit(RawParamEvent::SetParameterNormalized(
                            self.params.loop_start.as_ptr(),
                            self.params.loop_start.preview_normalized((origin_string + 1) as i32),
                        ));
                        cx.emit(RawParamEvent::SetParameterNormalized(
                            self.params.loop_end.as_ptr(),
                            self.params.loop_end.preview_normalized((origin_string + 1) as i32),
                        ));
                        cx.needs_redraw();
                        return true;
                    }
                }

                if let Some(note) = piano_note_at(x, y) {
                    let note_states = self.piano_display_notes(3);
                    if let Some(st) = note_states.iter().find(|ns| ns.note == note) {
                        let badge_r = piano_key_badge_rect(note);
                        if hit(badge_r, x, y) {
                            if cx.modifiers().alt() {
                                self.params.toggle_string_skipped(st.string_idx);
                            } else {
                                self.params.toggle_string_muted(st.string_idx);
                            }
                            cx.needs_redraw();
                            return true;
                        }

                        let r = piano_key_rect(note);
                        let body_h = (badge_r.1 - r.1).max(10.0);
                        let vol = 1.0 - ((y - r.1) / body_h).clamp(0.0, 1.0);
                        let is_ramp = cx.modifiers().shift();
                        self.params.set_string_volume(st.string_idx, vol);
                        self.drag = Some(Drag::PianoArpVolumeSweep {
                            origin_string: st.string_idx,
                            origin_vol: vol,
                            origin_y: y,
                            last_string: st.string_idx,
                            is_ramp,
                        });
                        cx.capture();
                        cx.needs_redraw();
                        return true;
                    }
                }
            }
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
        let mpe = self.params.mpe_enabled();
        if hit(routing_preset_rect(mpe), x, y) {
            let preset = if self.params.routing_preset.value() == 0 {
                "Single Track (Wurli on Ch 1)"
            } else {
                "Split Tracks (Ch 1/2/3)"
            };
            return Some(format!("Routing preset · Current: {preset} · Click to toggle between 1-track and multi-track routing"));
        }
        if hit(bass_octave_rect(), x, y) {
            return Some(format!("Bass octave · Current: {:+} · Click to cycle (-1, -2, 0)", self.params.bass_octave.value()));
        }
        if hit(ALWAYS_BASS, x, y) {
            let mode = match self.params.bass_mode.value() {
                1 => "Hold root note",
                2 => "Pulse on every arp step",
                3 => "Root-5th alternating pulse",
                4 => "Play in sync with comp hits",
                _ => "Off",
            };
            return Some(format!("Bass mode · Current: {mode} · Click to cycle modes"));
        }
        if !self.params.key_split.value() {
            if hit(comp_mode_rect(mpe), x, y) {
                let mode = match self.params.comp_mode.value() {
                    1 => "Sustained Pad (holds chords in background)",
                    2 => "Rhythmic Comp (chops chords in sync with groove)",
                    _ => "Off",
                };
                return Some(format!("Comping mode · Current: {mode} · Click to cycle (Off, Pad, Rhythm)"));
            }
            if hit(comp_octave_rect(mpe), x, y) {
                return Some(format!("Comp octave · Current: {:+} · Click to cycle (0, +1, -1)", self.params.comp_octave.value()));
            }
            if hit(comp_rhythm_rect(mpe), x, y) {
                let hits = self.params.comp_hits.value();
                let steps = self.params.comp_steps.value();
                let desc = crate::ui::menu::COMP_RHYTHMS
                    .iter()
                    .find(|&&(_, h, s, _)| h == hits && s == steps)
                    .map(|&(name, _, _, desc)| format!("{name}: {desc}"))
                    .unwrap_or_else(|| format!("{hits} hits across {steps} steps"));
                return Some(format!("Comp groove · {desc} · Click to choose rhythm"));
            }
            if hit(comp_lag_rect(mpe), x, y) {
                return Some(format!("Comp pocket microtiming · Current: {:+.0} ms · Click to cycle Dilla pocket lag (0ms, +12ms, +25ms, -10ms)", self.params.comp_lag_ms.value()));
            }
            if hit(comp_interlock_rect(mpe), x, y) {
                let inter = match self.params.comp_interlock.value() {
                    1 => "Duck Arp on comp hits (mixes arp down when chord stabs)",
                    2 => "Lock Arp accent to comp hits (accents arp on chord stabs)",
                    _ => "Independent (arp and comp play freely)",
                };
                return Some(format!("Conversational interlocking · Current: {inter} · Click to cycle"));
            }
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
        let base = self.params.control_base.load(Ordering::Relaxed);
        let base_note = if base < 0 { 0 } else { (base as u8).min(116) };
        let first = piano_key_rect(base_note);
        let last = piano_key_rect(base_note + 11);
        let badge_w = 148.0;
        let badge_h = 16.0;
        let badge_cx = ((first.0 + last.0 + last.2) * 0.5).clamp(
            PIANO_KEYS.0 + badge_w * 0.5,
            PIANO_KEYS.0 + PIANO_KEYS.2 - badge_w * 0.5,
        );
        let badge_rect = (badge_cx - badge_w * 0.5, PIANO_SPLITS.1 + 2.0, badge_w, badge_h);
        if hit(badge_rect, x, y) {
            return Some(if self.learning() == 4 {
                "Learning chord octave · Click any key to relocate".into()
            } else {
                format!("Chord trigger octave ({}–{}) · Click to relocate", self.note_label(base_note), self.note_label(base_note + 11))
            });
        }
        if self.params.mode.value() == 3 && hit(PIANO_ARP_LOOP_RAIL, x, y) {
            return Some("Arp loop rail · Drag [S] / [E] to adjust loop bounds · Drag across to brush loop · Double-click to reset".into());
        }
        if let Some(note) = piano_note_at(x, y) {
            let mut parts = vec![format!("{} · MIDI {}", self.note_label(note), note)];
            if self.params.mode.value() == 3 {
                let note_states = self.piano_display_notes(3);
                if let Some(st) = note_states.iter().find(|ns| ns.note == note) {
                    parts.push(format!(
                        "Arp String #{}{}{}",
                        st.string_idx + 1,
                        if st.is_muted { " [MUTED]" } else if st.is_skipped { " [SKIPPED]" } else { "" },
                        if !st.is_in_loop { " (out of loop)" } else { "" }
                    ));
                    parts.push(format!("Vol: {:.0}%", st.volume * 100.0));
                    parts.push("Click apron: Mute · Alt/Right-click: Skip · Drag body: Volume · Shift-drag: Ramp".into());
                }
            }
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

    pub(super) fn comp_rhythm_label(hits: i32, steps: i32) -> String {
        match (hits, steps) {
            (4, 16) => "Quarter ▾".into(),
            (8, 16) => "Eighths ▾".into(),
            (3, 8) => "Tresillo ▾".into(),
            (6, 16) => "Charleston ▾".into(),
            (5, 16) => "Cinquillo ▾".into(),
            (2, 16) => "Half Note ▾".into(),
            (12, 16) => "Shuffle ▾".into(),
            (7, 16) => "Afro 7 ▾".into(),
            _ => format!("{hits}/{steps} ▾"),
        }
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
        let bass_mode = self.params.bass_mode.value();
        let bass_mode_label = match bass_mode {
            1 => "Hold",
            2 => "Pulse",
            3 => "Root-5th",
            4 => "With Comp",
            _ => "Bass: Off",
        };
        self.button(
            d,
            ALWAYS_BASS,
            bass_mode_label,
            bass_mode > 0 || self.params.always_bass.value(),
            GOLD,
        );
        let bass_oct = self.params.bass_octave.value();
        self.button(
            d,
            bass_octave_rect(),
            &format!("Oct {:+}", bass_oct),
            bass_oct != 0,
            GOLD,
        );
        let mpe = self.params.mpe_enabled();
        let preset_single = self.params.routing_preset.value() == 0;
        self.button(
            d,
            routing_preset_rect(mpe),
            if preset_single { "Wurli" } else { "Split" },
            preset_single,
            GOLD,
        );
        if self.params.key_split.value() {
            self.button(
                d,
                affect_chords_rect(self.params.mpe_enabled()),
                "Affect Chords",
                self.params.affect_chords.value(),
                TEAL,
            );
            self.button(
                d,
                melody_split_rect(self.params.mpe_enabled()),
                if self.learning() == 6 {
                    "Cancel Learn"
                } else {
                    "Learn Split"
                },
                self.learning() == 6,
                TEAL,
            );
        }
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
            let comp_mode = self.params.comp_mode.value();
            let comp_label = match comp_mode {
                1 => "Comp: Pad",
                2 => "Comp: Rhythm",
                _ => "Comp: Off",
            };
            self.button(
                d,
                comp_mode_rect(mpe),
                comp_label,
                comp_mode > 0,
                TEAL,
            );
            let comp_oct = self.params.comp_octave.value();
            self.button(
                d,
                comp_octave_rect(mpe),
                &format!("Oct {:+}", comp_oct),
                comp_oct != 0,
                TEAL,
            );
            if comp_mode == 2 {
                let hits = self.params.comp_hits.value();
                let steps = self.params.comp_steps.value();
                let r = comp_rhythm_rect(mpe);
                let label = Self::comp_rhythm_label(hits, steps);
                self.button(
                    d,
                    r,
                    &label,
                    self.menu == Some(Menu::CompRhythm),
                    TEAL,
                );
                // Mini illuminated rhythm step dots
                let s = steps.clamp(1, 16) as usize;
                let pip_w = (r.2 - 16.0) / s as f32;
                let pip_y = r.1 + r.3 - 4.5;
                let cur_step = self.snapshot.arp_cycle_step % s;
                let is_active = self.snapshot.voices > 0;
                for i in 0..s {
                    let is_hit = ((i * hits as usize) % s) < (hits as usize);
                    let px = r.0 + 8.0 + i as f32 * pip_w + pip_w * 0.5;
                    let pip_col = if is_active && cur_step == i {
                        GOLD
                    } else if is_hit {
                        TEAL
                    } else {
                        alpha(MUTED, 0.35)
                    };
                    d.circle(px, pip_y, if is_hit { 1.6 } else { 1.0 }, pip_col, true);
                }
                let lag = self.params.comp_lag_ms.value();
                self.button(
                    d,
                    comp_lag_rect(mpe),
                    &format!("{lag:+.0}ms"),
                    lag != 0.0,
                    GOLD,
                );
                let interlock = self.params.comp_interlock.value();
                let inter_label = match interlock {
                    1 => "Duck",
                    2 => "Lock",
                    _ => "Indep",
                };
                self.button(
                    d,
                    comp_interlock_rect(mpe),
                    inter_label,
                    interlock > 0,
                    TEAL,
                );
            } else if comp_mode == 1 {
                d.text(
                    melody_container(mpe).0 + 172.0,
                    KEYBOARD_CONTROL_Y + 18.0,
                    "Sustained Pad · Backing chords",
                    TEXT_SMALL,
                    MUTED,
                );
            } else {
                d.text(
                    melody_container(mpe).0 + 172.0,
                    KEYBOARD_CONTROL_Y + 18.0,
                    "Accompaniment & groove stabs",
                    TEXT_SMALL,
                    alpha(MUTED, 0.6),
                );
            }
        }
        d.bypass_button(
            key_split_rect(self.params.mpe_enabled()),
            !self.params.key_split.value(),
            TEAL,
            self.hover_amount(key_split_rect(self.params.mpe_enabled())) > 0.01,
            0.0,
        );
        let mode = self.params.mode.value();
        let note_states = self.piano_display_notes(mode);
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

                let matching_state = note_states
                    .iter()
                    .find(|ns| ns.note == note && ns.is_in_loop)
                    .or_else(|| note_states.iter().find(|ns| ns.note == note));

                if let Some(st) = matching_state {
                    let full_h = r.3 - 4.0;
                    if st.is_in_loop {
                        if st.is_skipped {
                            d.outline_rounded(
                                r.0 + 1.5,
                                r.1 + 2.0,
                                (r.2 - 3.0).max(1.0),
                                full_h,
                                1.0,
                                alpha(MUTED, 0.4),
                                1.0,
                            );
                        } else if st.is_muted {
                            d.outline_rounded(
                                r.0 + 1.5,
                                r.1 + 2.0,
                                (r.2 - 3.0).max(1.0),
                                full_h,
                                1.0,
                                alpha(COLORS[1], 0.8),
                                1.5,
                            );
                            d.rounded_rect(
                                r.0 + 1.5,
                                r.1 + 2.0,
                                (r.2 - 3.0).max(1.0),
                                full_h,
                                1.0,
                                alpha(COLORS[1], 0.15),
                            );
                        } else {
                            let vol_h = full_h * st.volume.clamp(0.0, 1.0);
                            let vol_y = r.1 + 2.0 + (full_h - vol_h);
                            d.rounded_rect(
                                r.0 + 1.5,
                                vol_y,
                                (r.2 - 3.0).max(1.0),
                                vol_h,
                                1.0,
                                alpha(GOLD, if black { 0.7 } else { 0.6 }),
                            );
                            d.outline_rounded(
                                r.0 + 1.5,
                                r.1 + 2.0,
                                (r.2 - 3.0).max(1.0),
                                full_h,
                                1.0,
                                GOLD,
                                1.5,
                            );
                        }
                    } else {
                        // Ghost note outside the loop
                        d.rounded_rect(
                            r.0 + 1.5,
                            r.1 + 2.0,
                            (r.2 - 3.0).max(1.0),
                            full_h,
                            1.0,
                            alpha(GOLD, 0.08),
                        );
                        d.outline_rounded(
                            r.0 + 1.5,
                            r.1 + 2.0,
                            (r.2 - 3.0).max(1.0),
                            full_h,
                            1.0,
                            alpha(GOLD, 0.35),
                            1.0,
                        );
                    }

                    // Lower apron badges for mode 3
                    if mode == 3 && st.is_in_loop {
                        let badge_r = piano_key_badge_rect(note);
                        let badge_cx = badge_r.0 + badge_r.2 * 0.5;
                        let badge_cy = badge_r.1 + badge_r.3 * 0.5;
                        if st.is_muted {
                            d.rounded_rect(badge_r.0, badge_r.1, badge_r.2, badge_r.3, 2.0, alpha(COLORS[1], 0.85));
                            d.text_centered(badge_cx, badge_cy + 3.0, "M", 9.0, BG);
                        } else if st.is_skipped {
                            d.rounded_rect(badge_r.0, badge_r.1, badge_r.2, badge_r.3, 2.0, alpha(MUTED, 0.8));
                            d.text_centered(badge_cx, badge_cy + 3.0, "⊘", 9.0, BG);
                        } else {
                            d.rounded_rect(badge_r.0, badge_r.1, badge_r.2, badge_r.3, 2.0, alpha(BG, 0.75));
                            d.outline_rounded(badge_r.0, badge_r.1, badge_r.2, badge_r.3, 2.0, alpha(GOLD, 0.5), 1.0);
                            d.text_centered(badge_cx, badge_cy + 3.0, &format!("#{}", st.string_idx + 1), 8.5, GOLD);
                        }
                    }
                }
                let center = r.0 + r.2 / 2.0;
                if self.snapshot.sounding_notes[note as usize] {
                    d.circle(center, r.1 + r.3 - 20.0, 3.8, BG, true);
                    d.circle(center, r.1 + r.3 - 20.0, 3.0, GOLD, true);
                    if mode == 3 {
                        d.rounded_rect(
                            r.0 + 1.5,
                            r.1 + 2.0,
                            (r.2 - 3.0).max(1.0),
                            r.3 - 4.0,
                            1.0,
                            alpha(GOLD, 0.35),
                        );
                    }
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
        let split_note = self.params.split_note.value() as u8;
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
        let base_note = if base < 0 { 0 } else { (base as u8).min(116) };
        let first = piano_key_rect(base_note);
        let last = piano_key_rect(base_note + 11);
        if base >= 0 {
            d.rect(first.0, stripe_y, last.0 + last.2 - first.0, 4.0, COLORS[4]);
        }
        let start_name = self.note_label(base_note);
        let end_name = self.note_label(base_note + 11);
        let badge_text = if self.learning() == 4 {
            "LEARN: CLICK KEY TO MOVE".to_string()
        } else {
            format!("CHORD KEYS: {start_name}–{end_name}")
        };
        let badge_w = 148.0;
        let badge_h = 16.0;
        let badge_cx = ((first.0 + last.0 + last.2) * 0.5).clamp(
            PIANO_KEYS.0 + badge_w * 0.5,
            PIANO_KEYS.0 + PIANO_KEYS.2 - badge_w * 0.5,
        );
        let badge_y = PIANO_SPLITS.1 + 2.0;
        let badge_rect = (badge_cx - badge_w * 0.5, badge_y, badge_w, badge_h);
        let is_hover = self
            .hover_pointer()
            .is_some_and(|(px, py)| hit(badge_rect, px, py));
        d.rounded_rect(badge_rect.0, badge_rect.1, badge_rect.2, badge_rect.3, 3.0, BG);
        d.outline_rounded(
            badge_rect.0,
            badge_rect.1,
            badge_rect.2,
            badge_rect.3,
            3.0,
            if self.learning() == 4 || is_hover {
                COLORS[4]
            } else {
                alpha(COLORS[4], 0.7)
            },
            1.2,
        );
        d.text_centered(badge_cx, badge_y + 11.5, &badge_text, 9.5, COLORS[4]);

        self.draw_piano_arp_loop_rail(d);
    }

    fn draw_piano_arp_loop_rail(&self, d: &mut Draw) {
        if self.params.mode.value() != 3 {
            return;
        }
        let base_notes = if self.snapshot.notes.len > 0 {
            self.snapshot.notes
        } else {
            self.spread_preview_notes()
        };
        if base_notes.len == 0 {
            return;
        }
        let octaves = self
            .routed_plain("octaves")
            .unwrap_or(self.params.octaves.value() as f32) as usize;
        let base_count = (base_notes.len * octaves).min(32);
        let loop_start_val = self.params.loop_start.value() as usize;
        let loop_end_val = self.params.loop_end.value() as usize;
        let count = base_count.max(loop_end_val).max(loop_start_val);
        let effective_end = if loop_end_val > 0 { loop_end_val } else { base_count };
        let start_idx = loop_start_val.saturating_sub(1).min(count - 1);
        let end_idx = effective_end.saturating_sub(1).min(count - 1);

        let p_start = (base_notes.string(start_idx % base_notes.len).unwrap_or(60) as usize + (start_idx / base_notes.len) * 12).min(127) as u8;
        let p_end = (base_notes.string(end_idx % base_notes.len).unwrap_or(60) as usize + (end_idx / base_notes.len) * 12).min(127) as u8;

        let x_start = piano_note_x(p_start);
        let x_end = piano_note_x(p_end);
        let (xs, xe) = if (x_start - x_end).abs() < 16.0 {
            (x_start - 8.0, x_end + 8.0)
        } else {
            (x_start, x_end)
        };

        let rail_y = PIANO_ARP_LOOP_RAIL.1 + PIANO_ARP_LOOP_RAIL.3 * 0.5;

        // Subtle groove across the active chord span
        let first_p = (base_notes.string(0).unwrap_or(60) as usize).min(127) as u8;
        let last_p = (base_notes.string((count - 1) % base_notes.len).unwrap_or(60) as usize + ((count - 1) / base_notes.len) * 12).min(127) as u8;
        let x_min = piano_note_x(first_p).min(xs);
        let x_max = piano_note_x(last_p).max(xe);
        d.line(x_min, rail_y, x_max, rail_y, alpha(MUTED, 0.25), 2.0);

        // Active loop span line
        let (min_x, max_x) = if xs <= xe { (xs, xe) } else { (xe, xs) };
        d.line(min_x, rail_y, max_x, rail_y, alpha(GOLD, 0.85), 3.0);

        // Tags [S] and [E]
        let tag_w = 16.0;
        let tag_h = 13.0;
        let tag_y = rail_y - tag_h * 0.5;

        let dragging_start = matches!(self.drag, Some(Drag::PianoArpLoopBound(false)));
        let dragging_end = matches!(self.drag, Some(Drag::PianoArpLoopBound(true)));

        // Start tag [S]
        let s_rect = (xs - tag_w * 0.5, tag_y, tag_w, tag_h);
        d.rounded_rect(s_rect.0, s_rect.1, s_rect.2, s_rect.3, 2.5, BG);
        d.outline_rounded(s_rect.0, s_rect.1, s_rect.2, s_rect.3, 2.5, GOLD, if dragging_start { 2.0 } else { 1.2 });
        d.text_centered(xs, tag_y + 9.5, "S", 8.5, GOLD);

        // End tag [E]
        let e_rect = (xe - tag_w * 0.5, tag_y, tag_w, tag_h);
        d.rounded_rect(e_rect.0, e_rect.1, e_rect.2, e_rect.3, 2.5, BG);
        d.outline_rounded(e_rect.0, e_rect.1, e_rect.2, e_rect.3, 2.5, GOLD, if dragging_end { 2.0 } else { 1.2 });
        d.text_centered(xe, tag_y + 9.5, "E", 8.5, GOLD);
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
                if self.params.key_split.value() {
                    "MELODY"
                } else {
                    "COMP"
                },
                if self.params.key_split.value() {
                    TEAL
                } else {
                    TEAL
                },
            ),
        ] {
            self.surface(d, r);
            let melody = label == "MELODY" || label == "COMP";
            d.text(
                r.0 + 10.0 + if melody || label == "BASS" { 30.0 } else { 0.0 },
                module_title_y(r.1, MODULE_TITLE_SIZE),
                label,
                MODULE_TITLE_SIZE,
                if (melody && !self.params.key_split.value() && self.params.comp_mode.value() == 0)
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
            if !self.params.key_split.value() && c.id == "upper_channel" {
                let ch_r = keyboard_control_rect("comp_channel", mpe);
                self.button(
                    d,
                    ch_r,
                    &format!("Ch {} ▾", self.params.comp_channel.value()),
                    self.menu == Some(Menu::KeyboardParam("comp_channel")),
                    TEAL,
                );
                continue;
            }
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
