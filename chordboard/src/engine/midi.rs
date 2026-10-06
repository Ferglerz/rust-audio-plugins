use super::*;
impl Engine {
    fn is_control_note(&self, note: u8) -> bool {
        self.config.control_base >= 0
            && (self.config.control_base..self.config.control_base + 12).contains(&(note as i16))
    }

    /// Infer MPE only from expression on an occupied member channel while
    /// multiple member channels each hold a single note. Notes alone are not evidence.
    pub fn member_expression_is_mpe(&self, channel: u8) -> bool {
        if channel >= 16 || !self.is_member(channel) {
            return false;
        }
        let held = |ch: u8| {
            self.down[ch as usize * 128..(ch as usize + 1) * 128]
                .iter()
                .filter(|&&v| v)
                .count()
        };
        held(channel) == 1
            && (0..16)
                .filter(|&ch| self.is_member(ch) && held(ch) == 1)
                .count()
                >= 2
    }

    pub fn midi_note(
        &mut self,
        on: bool,
        channel: u8,
        note: u8,
        velocity: f32,
        out: &mut impl FnMut(Out),
    ) {
        if channel >= 16 || note >= 128 {
            return;
        }
        let pressed = on && velocity > 0.0;
        let learn_id = channel as usize * 128 + note as usize;
        if self.learned_note_held[learn_id] {
            if !pressed {
                self.learned_note_held[learn_id] = false;
            }
            return;
        }
        if pressed && matches!(self.learn, 5 | 6) {
            let target = self.learn;
            let bass = if self.host_config.bass_split >= 0 {
                self.host_config.bass_split as u8
            } else {
                if self.host_config.control_base >= 0 {
                    self.host_config.control_base as u8
                } else {
                    36
                }
            };
            let (bass, melody) = if target == 5 {
                ordered_splits(note, self.host_config.split_note)
            } else {
                ordered_splits(bass, note)
            };
            let learned = LearnedSplit {
                target,
                bass,
                melody,
            };
            self.learn = 0;
            let mut config = self.host_config;
            learned.apply(&mut config);
            self.configure(config, out);
            self.learned_note_held[learn_id] = true;
            self.learned_split = Some(learned);
            return;
        }
        if pressed && self.learn == 4 {
            let base = note.min(116);
            self.learned_base = Some(base);
            self.learn = 0;
            // Activate before the next event in this block.
            let mut config = self.host_config;
            config.control_base = base as i16;
            self.configure(config, out);
            return;
        }
        if self.is_control_note(note) {
            if pressed {
                let interval = (note as i16 - self.config.control_base) as usize;
                let (quality, alteration) = harmony::CONTROL_CHORDS[interval];
                self.quality = quality;
                self.control_alteration = alteration;
                if self.root.is_none() {
                    if let Some(memory) = self.memory.as_mut() {
                        memory.second = None;
                    }
                }
                // Selection only: do not strike, sweep, or restart playback.
                self.rebuild_harmony(false, out);
            }
            // Consume releases and zero-velocity Note Ons on every channel too.
            return;
        }
        if self.config.harmonization_mode > 0 && self.config.key_split && self.topology.is_zone_b(note) {
            if pressed {
                if self.root.is_some() {
                    if let Some(white_idx) = harmony::white_key_degree(note % 12) {
                        let root_pc = self.selected_root().unwrap_or(60) % 12;
                        let rel_semi = (root_pc as i16 - self.config.key as i16).rem_euclid(12) as u8;
                        let tuple = harmony::matrix_interval_tuple(self.config.scale, rel_semi);
                        let octave_offset = (note.saturating_sub(self.topology.zone_b_min) / 12) as u8;
                        let root_pitch = (self.topology.zone_b_min + root_pc).min(127);
                        let mapped = harmony::map_zone_b_white_key(white_idx, octave_offset, root_pitch, &tuple, self.config.extensions * 4.0);
                        self.zone_b_mapped[channel as usize][note as usize] = Some(mapped);
                        self.melody_note(true, channel, mapped, velocity, out);
                    } else {
                        self.zone_b_mapped[channel as usize][note as usize] = Some(note);
                        self.melody_note(true, channel, note, velocity, out);
                    }
                } else {
                    self.lookahead_buffer.ingest_zone_b(channel, note, (velocity.clamp(0.01, 1.0) * 127.0) as u8, self.now);
                }
            } else {
                if self.lookahead_buffer.cancel_note(channel, note) {
                    return;
                }
                if let Some(mapped) = self.zone_b_mapped[channel as usize][note as usize].take() {
                    self.melody_note(false, channel, mapped, velocity, out);
                } else {
                    self.melody_note(false, channel, note, velocity, out);
                }
            }
            return;
        }
        if self.config.bass_boundary() >= 0 && (note as i16) < self.config.bass_boundary() {
            self.bass_note(pressed, channel, note, velocity, out);
            return;
        }
        if self.config.key_split && note >= self.config.split_note {
            self.melody_note(pressed, channel, note, velocity, out);
            return;
        }
        let id = channel as u16 * 128 + note as u16;
        if pressed {
            self.note_on(id, note, channel, velocity, None, out);
        } else {
            self.note_off(id, velocity, out);
        }
    }
    pub fn pressure(
        &mut self,
        channel: u8,
        note: Option<u8>,
        value: f32,
        out: &mut impl FnMut(Out),
    ) {
        if channel >= 16
            || !value.is_finite()
            || note.is_some_and(|n| n >= 128 || self.is_control_note(n))
        {
            return;
        }
        self.source(4, value, out);
        self.channels[channel as usize].pressure = value.clamp(0.0, 1.0);
        if self.config.key_split
            && note.map_or_else(
                || self.melody_channel(channel),
                |note| self.melody_note_held(channel, note),
            )
        {
            out(Out::Pressure(channel, value.clamp(0.0, 1.0)));
        }
        if self
            .root
            .is_some_and(|s| s.held && s.channel == channel && note.is_none_or(|n| n == s.note))
        {
            self.expression.pressure = value.clamp(0.0, 1.0);
            self.fan_expression(out);
            if self.config.harmonization_mode > 0 {
                let alpha = value.clamp(0.0, 1.0) * 4.0;
                let (_tier, smoothed_alpha) = self.hysteresis_tracker.process_parameter(alpha);
                self.config.extensions = (smoothed_alpha / 4.0).clamp(0.0, 1.0);
                self.rebuild_harmony(false, out);
            }
        } else if note.is_none()
            && channel == self.input_master()
            && self.root.is_some_and(|s| s.channel != channel)
        {
            out(Out::Pressure(self.master(), value));
        }
    }
    pub fn bend(&mut self, channel: u8, value: f32, out: &mut impl FnMut(Out)) {
        if channel >= 16 || !value.is_finite() {
            return;
        }
        self.input_bend_values[channel as usize] = value.clamp(0.0, 1.0);
        if self.config.key_split && self.melody_channel(channel) {
            out(Out::Bend(channel, self.melody_bend(channel)));
        }
        self.source(0, value, out);
        if self.learn > 0 && self.learn <= 2 && !self.is_member(channel) {
            self.learned = Some((
                (self.learn - 1) as usize,
                Mapping {
                    kind: 3,
                    number: 0,
                    channel,
                },
            ));
            self.learn = 0;
            return;
        }
        for axis in 0..2 {
            let m = self.config.mappings[axis];
            if m.kind == 3 && m.channel == channel && !self.is_member(channel) {
                self.position(value, axis, out);
                return;
            }
        }
        let mapped = ((value - 0.5) * self.input_bend[channel as usize]
            / (if self.config.mpe {
                self.config.bend_range
            } else {
                self.config.master_range
            })
            .max(1.0)
            + 0.5)
            .clamp(0.0, 1.0);
        self.channels[channel as usize].bend = mapped;
        if self.root.is_some_and(|s| s.held && s.channel == channel) {
            self.expression.bend = mapped;
            self.fan_expression(out);
        } else if channel == self.input_master() {
            let bend = ((value - 0.5) * self.input_bend[channel as usize]
                / self.config.master_range.max(1.0)
                + 0.5)
                .clamp(0.0, 1.0);
            out(Out::Bend(self.master(), bend));
        }
    }
    pub fn control(&mut self, channel: u8, cc: u8, value: f32, out: &mut impl FnMut(Out)) {
        if channel >= 16 || cc >= 128 || !value.is_finite() {
            return;
        }
        let raw = (value.clamp(0.0, 1.0) * 127.0).round() as u8;
        self.cc[channel as usize][cc as usize] = raw;
        if (32..64).contains(&cc) {
            self.cc_lsb_seen[channel as usize] |= 1 << (cc - 32);
        }
        match cc {
            1 => self.source(1, value, out),
            11 => {
                self.source(2, value, out);
                if self.config.harmonization_mode > 0 {
                    self.config.extensions = value.clamp(0.0, 1.0);
                    self.rebuild_harmony(false, out);
                }
            }
            2 => self.source(3, value, out),
            74 => self.source(5, value, out),
            _ => {}
        }
        match cc {
            101 => {
                self.rpn[channel as usize][0] = raw;
                return;
            }
            100 => {
                self.rpn[channel as usize][1] = raw;
                return;
            }
            6 | 38 => match self.rpn[channel as usize] {
                [0, 0] => {
                    self.input_bend[channel as usize] = self.cc[channel as usize][6] as f32
                        + self.cc[channel as usize][38] as f32 / 100.0;
                    return;
                }
                [0, 6] if cc == 6 && (channel == 0 || channel == 15) => {
                    self.input_upper = channel == 15;
                    self.input_members = raw.min(15);
                    return;
                }
                _ => {}
            },
            120 | 123 => {
                self.panic(out);
                return;
            }
            121 => {
                self.cc_lsb_seen[channel as usize] = 0;
                self.cc[channel as usize].fill(0);
                self.bass_pedal(channel, false, out);
                self.channels[channel as usize] = Expression::default();
                self.input_bend_values[channel as usize] = 0.5;
                if self.config.key_split {
                    self.melody_sustain[channel as usize] = false;
                    out(Out::Cc(channel, 121, value));
                    out(Out::Cc(channel, 64, 0.0));
                }
                if self.root.is_some_and(|s| s.held && s.channel == channel) {
                    self.expression = Expression::default();
                    self.fan_expression(out);
                }
                return;
            }
            64 => {
                self.bass_pedal(channel, raw >= 64, out);
                if self.config.key_split {
                    self.melody_sustain[channel as usize] = raw >= 64;
                    out(Out::Cc(channel, 64, value));
                }
                let prev_pedal = self.pedal;
                self.pedal = raw >= 64;
                if self.config.harmonization_mode > 0 && !prev_pedal && self.pedal {
                    let now_ms = (self.now as f64 * 1000.0 / self.sample_rate.max(1.0) as f64) as u64;
                    if self.last_pedal_tap_time > 0 && now_ms.saturating_sub(self.last_pedal_tap_time) < 400 {
                        if let Some(held_note) = self.root.map(|s| s.note).or_else(|| self.selected_root()) {
                            self.config.key = held_note % 12;
                            self.rebuild_harmony(true, out);
                        }
                        self.last_pedal_tap_time = 0;
                    } else {
                        self.last_pedal_tap_time = now_ms.max(1);
                    }
                }
                if !self.pedal
                    && !self.config.latch
                    && !self.config.key_split
                    && self.root.is_none()
                {
                    self.notes = Notes::default();
                    self.stop_voices(0.0, out);
                    self.scheduled.fill(None);
                }
                return;
            }
            _ => {}
        }
        // Do not automatically learn Osmose's member timbre dimension.
        if self.learn > 0 && self.learn <= 2 && !(cc == 74 && self.is_member(channel)) {
            self.learned = Some(((self.learn - 1) as usize, Mapping::cc(cc, channel)));
            self.learn = 0;
            return;
        }
        let mut mapped = false;
        for axis in 0..2 {
            let m = self.config.mappings[axis];
            if m.channel != 16 && m.channel != channel {
                continue;
            }
            let wide = m.number < 32
                && (m.kind == 2 || self.cc_lsb_seen[channel as usize] & (1 << m.number) != 0);
            if matches!(m.kind, 1 | 2) && (cc == m.number || (wide && cc == m.number + 32)) {
                let value = if wide {
                    let data = &self.cc[channel as usize];
                    let val = ((data[m.number as usize] as u16) << 7)
                        | data[m.number as usize + 32] as u16;
                    val as f32 / 16383.0
                } else {
                    value
                };
                self.position(value, axis, out);
                mapped = true;
            }
        }
        if mapped {
            return;
        }
        if cc == 74 {
            self.channels[channel as usize].timbre = value;
            if self.config.key_split && self.melody_channel(channel) {
                out(Out::Cc(channel, 74, value));
            }
            if self.root.is_some_and(|s| s.held && s.channel == channel) {
                self.expression.timbre = value;
                self.fan_expression(out);
            } else if channel == self.input_master()
                && self.root.is_some_and(|s| s.channel != channel)
            {
                out(Out::Cc(self.master(), 74, value));
            }
        } else if !self.is_member(channel) {
            out(Out::Cc(self.master(), cc, value));
        }
    }
    fn input_master(&self) -> u8 {
        if self.input_upper {
            15
        } else {
            0
        }
    }
    fn is_member(&self, channel: u8) -> bool {
        if self.input_upper {
            channel < 15 && channel >= 15 - self.input_members
        } else {
            channel > 0 && channel <= self.input_members
        }
    }
}
