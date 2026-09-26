use super::*;
impl Engine {
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
        let id = channel as u16 * 128 + note as u16;
        if on && velocity > 0.0 {
            if self.learn == 4 {
                self.learned_base = Some(note.min(116));
                self.learn = 0;
                return;
            }
            if self.config.control_base >= 0
                && (self.config.control_base..self.config.control_base + 12)
                    .contains(&(note as i16))
            {
                self.quality = (note as i16 - self.config.control_base) as u8;
                self.rebuild(out);
                return;
            }
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
        if channel >= 16 {
            return;
        }
        self.channels[channel as usize].pressure = value.clamp(0.0, 1.0);
        if self
            .root
            .is_some_and(|s| s.held && s.channel == channel && note.is_none_or(|n| n == s.note))
        {
            self.expression.pressure = value.clamp(0.0, 1.0);
            self.fan_expression(out);
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
        if self.learn > 0 && self.learn <= 3 && !self.is_member(channel) {
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
        for axis in 0..3 {
            let m = self.config.mappings[axis];
            if m.kind == 3 && m.channel == channel && !self.is_member(channel) {
                self.mapped(axis, value, out);
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
                self.channels[channel as usize] = Expression::default();
                if self.root.is_some_and(|s| s.held && s.channel == channel) {
                    self.expression = Expression::default();
                    self.fan_expression(out);
                }
                return;
            }
            64 => {
                self.pedal = raw >= 64;
                if !self.pedal && !self.config.latch && self.root.is_none() {
                    self.notes = Notes::default();
                    self.stop_voices(0.0, out);
                    self.scheduled.fill(None);
                }
                return;
            }
            _ => {}
        }
        // Never learn or intercept Osmose's member timbre dimension.
        if self.learn > 0 && self.learn <= 3 && !(cc == 74 && self.is_member(channel)) {
            self.learned = Some(((self.learn - 1) as usize, Mapping::cc(cc, channel)));
            self.learn = 0;
            return;
        }
        for axis in 0..3 {
            let m = self.config.mappings[axis];
            if m.channel != 16 && m.channel != channel {
                continue;
            }
            if cc == 74 && self.is_member(channel) {
                continue;
            }
            if m.kind == 1 && cc == m.number {
                self.mapped(axis, value, out);
                return;
            }
            if m.kind == 2 && m.number < 32 && (cc == m.number || cc == m.number + 32) {
                let data = &self.cc[channel as usize];
                let val =
                    ((data[m.number as usize] as u16) << 7) | data[m.number as usize + 32] as u16;
                self.mapped(axis, val as f32 / 16383.0, out);
                return;
            }
        }
        if cc == 74 {
            self.channels[channel as usize].timbre = value;
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
    fn mapped(&mut self, axis: usize, value: f32, out: &mut impl FnMut(Out)) {
        if axis == 2 {
            self.gate_open = value >= 0.5;
            self.x_primed = false;
        } else {
            self.position(value, axis, out);
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
