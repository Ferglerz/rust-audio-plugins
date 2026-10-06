use super::*;

impl Engine {
    pub fn melody_note_held(&self, channel: u8, note: u8) -> bool {
        channel < 16 && note < 128 && self.melody[channel as usize][note as usize]
    }

    /// Every note currently owned by this processor, including transparent melody.
    pub fn owned_output_notes(&self) -> [[bool; 128]; 16] {
        let mut notes = self.melody;
        for input in &self.bass {
            for (note, channel) in input.iter().enumerate() {
                if let Some(channel) = channel {
                    notes[*channel as usize][note] = true;
                }
            }
        }
        for voice in self.voices.iter().flatten() {
            notes[voice.channel as usize][voice.note as usize] = true;
        }
        notes
    }

    pub(super) fn output_owned(&self, channel: u8, note: u8) -> bool {
        self.melody[channel as usize][note as usize]
            || self
                .bass
                .iter()
                .any(|input| input[note as usize] == Some(channel))
            || self
                .voices
                .iter()
                .flatten()
                .any(|v| v.channel == channel && v.note == note)
    }

    fn release_bass(
        &mut self,
        input: usize,
        note: usize,
        velocity: f32,
        out: &mut impl FnMut(Out),
    ) {
        if let Some(channel) = self.bass[input][note].take() {
            self.tap.push(
                self.now,
                LaneEvent::Off {
                    lane: Lane::Bass,
                    channel,
                    note: note as u8,
                    velocity,
                },
            );
            if !self.output_owned(channel, note as u8) {
                out(Out::Off(channel, note as u8, velocity));
            }
        }
    }

    pub(super) fn bass_note(
        &mut self,
        pressed: bool,
        input: u8,
        note: u8,
        velocity: f32,
        out: &mut impl FnMut(Out),
    ) {
        let input = input as usize;
        let note_index = note as usize;
        let id = input * 128 + note_index;
        if self.down[id] == pressed {
            return;
        }
        self.down[id] = pressed;
        if pressed {
            let channel = self.config.bass_channel;
            let owned = self.output_owned(channel, note);
            self.bass[input][note_index] = Some(channel);
            self.tap.push(
                self.now,
                LaneEvent::On {
                    lane: Lane::Bass,
                    channel,
                    note,
                    velocity: velocity.clamp(0.01, 1.0),
                },
            );
            if !owned {
                out(Out::On(channel, note, velocity.clamp(0.01, 1.0)));
            }
        } else if !self.bass_sustain[input] {
            self.release_bass(input, note_index, velocity, out);
        }
    }

    pub(super) fn bass_pedal(&mut self, input: u8, held: bool, out: &mut impl FnMut(Out)) {
        let input = input as usize;
        self.bass_sustain[input] = held;
        if !held {
            for note in 0..128 {
                if !self.down[input * 128 + note] {
                    self.release_bass(input, note, 0.0, out);
                }
            }
        }
    }

    pub(super) fn stop_bass(&mut self, out: &mut impl FnMut(Out)) {
        for input in 0..16 {
            for note in 0..128 {
                if self.bass[input][note].is_some() {
                    self.down[input * 128 + note] = false;
                    self.release_bass(input, note, 0.0, out);
                }
            }
        }
        self.bass_sustain.fill(false);
    }

    pub(super) fn melody_channel(&self, channel: u8) -> bool {
        self.melody[channel as usize].iter().any(|&held| held)
    }

    pub(super) fn melody_bend(&self, channel: u8) -> f32 {
        let member = self.config.mpe
            && (0..self.config.members.clamp(1, 15) as usize).any(|i| self.member(i) == channel);
        let range = if member {
            self.config.bend_range
        } else {
            self.config.master_range
        };
        ((self.input_bend_values[channel as usize] - 0.5) * self.input_bend[channel as usize]
            / range.max(1.0)
            + 0.5)
            .clamp(0.0, 1.0)
    }

    pub(super) fn emit_melody_expression(&self, channel: u8, out: &mut impl FnMut(Out)) {
        let expression = self.channels[channel as usize];
        out(Out::Pressure(channel, expression.pressure));
        out(Out::Cc(channel, 74, expression.timbre));
        out(Out::Bend(channel, self.melody_bend(channel)));
    }

    pub(super) fn melody_note(
        &mut self,
        pressed: bool,
        channel: u8,
        note: u8,
        velocity: f32,
        out: &mut impl FnMut(Out),
    ) {
        let id = channel as usize * 128 + note as usize;
        if pressed == self.melody[channel as usize][note as usize] {
            return;
        }
        let already_owned = self.output_owned(channel, note);
        self.melody[channel as usize][note as usize] = pressed;
        self.down[id] = pressed;
        if pressed {
            self.setup(out);
            // A transparent MPE melody owns its original member channel. Move
            // generated voices away before forwarding its expression or attack.
            if self.config.mpe {
                let displaced = self.voices;
                for (i, voice) in displaced.into_iter().enumerate() {
                    if let Some(voice) = voice.filter(|v| v.channel == channel) {
                        self.end_voice(i, 0.0, out);
                        self.allocate_voice(
                            voice.note,
                            self.strike_velocity(),
                            voice.off,
                            voice.layer,
                            voice.kind,
                            out,
                        );
                    }
                }
            }
            self.emit_melody_expression(channel, out);
            self.tap.push(
                self.now,
                LaneEvent::On {
                    lane: Lane::Lead,
                    channel,
                    note,
                    velocity: velocity.clamp(0.01, 1.0),
                },
            );
            if !already_owned {
                out(Out::On(channel, note, velocity.clamp(0.01, 1.0)));
            }
        } else {
            self.tap.push(
                self.now,
                LaneEvent::Off {
                    lane: Lane::Lead,
                    channel,
                    note,
                    velocity,
                },
            );
            if !self.output_owned(channel, note) {
                out(Out::Off(channel, note, velocity));
            }
        }
        if self.config.affect_chords {
            // Rebuild from the saved recipe, not the previous melody additions.
            // Keep a sweep's remaining attacks and timing while updating pitches.
            let pending = self.scheduled;
            self.rebuild_harmony(self.config.mode == CHORD, out);
            if self.config.mode != CHORD {
                for (slot, event) in self.scheduled.iter_mut().zip(pending) {
                    *slot = event.and_then(|mut event| {
                        let note = self
                            .notes
                            .as_slice()
                            .iter()
                            .copied()
                            .filter(|n| n % 12 == event.note % 12)
                            .min_by_key(|&n| (n as i16 - event.note as i16).abs())?;
                        event.note = note;
                        Some(event)
                    });
                }
            }
        }
        self.sync_layers(out);
    }

    pub(super) fn stop_melody(&mut self, out: &mut impl FnMut(Out)) {
        for channel in 0..16 {
            for note in 0..128 {
                if self.melody[channel][note] {
                    self.melody[channel][note] = false;
                    self.down[channel * 128 + note] = false;
                    self.tap.push(
                        self.now,
                        LaneEvent::Off {
                            lane: Lane::Lead,
                            channel: channel as u8,
                            note: note as u8,
                            velocity: 0.0,
                        },
                    );
                    if !self.output_owned(channel as u8, note as u8) {
                        out(Out::Off(channel as u8, note as u8, 0.0));
                    }
                }
            }
            if self.melody_sustain[channel] {
                self.melody_sustain[channel] = false;
                out(Out::Cc(channel as u8, 64, 0.0));
            }
        }
        self.layer_latched = false;
    }

    pub(super) fn sync_layers(&mut self, out: &mut impl FnMut(Out)) {
        let comp_pad = self.config.comp_mode == 1;
        let bass_hold = (self.config.bass_mode == 1 || self.config.always_bass) && self.config.bass_enabled;
        let enabled =
            bass_hold
                || self.config.always_chord
                || comp_pad;
        let held = if self.config.key_split {
            self.melody.iter().any(|channel| channel.iter().any(|&v| v))
        } else {
            self.held_inputs.iter().any(Option::is_some)
        };
        if !self.config.latch || !enabled {
            self.layer_latched = false;
        }
        if enabled && held && self.config.latch {
            self.layer_latched = true;
        }
        let mut desired: [Option<(u8, u8, u8)>; 32] = [None; 32];
        let mut desired_count = 0;
        if enabled && (held || self.layer_latched) {
            if self.config.always_chord {
                for &note in self.full_notes.as_slice() {
                    if desired_count < 32 {
                        desired[desired_count] = Some((note, self.config.output_channel, KIND_COMP));
                        desired_count += 1;
                    }
                }
            }
            if comp_pad {
                if self.config.comp_guide_tone {
                    if let Some(guide) = self.selected_root().and_then(|r| crate::harmony::extract_guide_tone(&self.notes, r)) {
                        let trans = (guide as i16 + self.config.comp_octave as i16 * 12).clamp(0, 127) as u8;
                        if desired_count < 32 {
                            desired[desired_count] = Some((trans, self.config.comp_channel, KIND_COMP));
                            desired_count += 1;
                        }
                    }
                } else {
                    for &note in self.notes.as_slice() {
                        let trans = (note as i16 + self.config.comp_octave as i16 * 12).clamp(0, 127) as u8;
                        if desired_count < 32 {
                            desired[desired_count] = Some((trans, self.config.comp_channel, KIND_COMP));
                            desired_count += 1;
                        }
                    }
                }
            }
            if self.config.bass_mode == 1 && self.config.bass_enabled {
                if let Some(note) = self.selected_root() {
                    let alt_offset = if self.config.bass_pad_trigger == 1 { 7 } else { 0 };
                    let trans = (note as i16 + alt_offset + self.config.bass_octave as i16 * 12).clamp(0, 127) as u8;
                    if desired_count < 32 {
                        desired[desired_count] = Some((trans, self.config.bass_channel, KIND_BASS));
                        desired_count += 1;
                    }
                }
            } else if self.config.always_bass && self.config.bass_enabled {
                if let Some(note) = self.selected_root() {
                    if desired_count < 32 {
                        desired[desired_count] = Some((note, self.config.output_channel, KIND_BASS));
                        desired_count += 1;
                    }
                }
            }
        }
        // Layer ownership is independent of a strum's deadline. Removing a
        // layer never cuts off an overlapping strum that is still ringing.
        for i in 0..self.voices.len() {
            if let Some(voice) = self.voices[i].as_mut() {
                let was_layer = voice.layer;
                let still_desired = desired[..desired_count]
                    .iter()
                    .any(|d| d.is_some_and(|(n, ch, k)| n == voice.note && ch == voice.channel && k == voice.kind));
                voice.layer = still_desired;
                if was_layer && !voice.layer && voice.off <= self.now {
                    self.end_voice(i, 0.0, out);
                }
            }
        }
        for d in desired[..desired_count].iter().flatten() {
            let (note, channel, kind) = *d;
            if !self.voices.iter().flatten().any(|v| v.note == note && v.channel == channel && v.kind == kind) {
                let vel = if kind == KIND_BASS {
                    self.config.bass_velocity
                } else if kind == KIND_COMP {
                    self.config.comp_velocity
                } else {
                    self.strike_velocity()
                };
                self.allocate_voice(note, vel, self.now, true, kind, out);
            }
        }
    }
}
