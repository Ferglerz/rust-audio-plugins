use super::*;
impl Engine {
    pub(super) fn setup(&mut self, out: &mut impl FnMut(Out)) {
        if !self.needs_setup {
            return;
        }
        self.needs_setup = false;
        if !self.config.mpe {
            return;
        }
        let master = self.master();
        // RPN 6 establishes the output zone; RPN 0 establishes bend sensitivity.
        for (cc, value) in [
            (101, 0),
            (100, 6),
            (6, self.config.members.clamp(1, 15)),
            (101, 127),
            (100, 127),
        ] {
            out(Out::Cc(master, cc, value as f32 / 127.0));
        }
        for i in 0..=self.config.members as usize {
            let channel = if i == 0 { master } else { self.member(i - 1) };
            let range = if i == 0 {
                self.config.master_range
            } else {
                self.config.bend_range
            };
            for (cc, value) in [
                (101, 0),
                (100, 0),
                (6, range as u8),
                (38, 0),
                (101, 127),
                (100, 127),
            ] {
                out(Out::Cc(channel, cc, value as f32 / 127.0));
            }
        }
    }
    pub(super) fn end_voice(&mut self, i: usize, release: f32, out: &mut impl FnMut(Out)) {
        if let Some(v) = self.voices[i].take() {
            out(Out::Off(v.channel, v.note, release));
        }
    }
    pub(super) fn stop_voices(&mut self, release: f32, out: &mut impl FnMut(Out)) {
        for i in 0..15 {
            self.end_voice(i, release, out);
        }
    }
    pub(super) fn fan_expression(&self, out: &mut impl FnMut(Out)) {
        let mut sent = [false; 16];
        for v in self.voices.iter().flatten() {
            if !sent[v.channel as usize] {
                self.emit_expression(v.channel, out);
                sent[v.channel as usize] = true;
            }
        }
    }
    fn emit_expression(&self, channel: u8, out: &mut impl FnMut(Out)) {
        let e = self.expression;
        let pressure = if self.y_active && self.config.y_target == 2 {
            self.y
        } else {
            e.pressure
        };
        let timbre = if self.y_active && self.config.y_target == 3 {
            self.y
        } else {
            e.timbre
        };
        let bend = if self.y_active && self.config.y_target == 4 {
            self.y
        } else {
            e.bend
        };
        out(Out::Pressure(channel, pressure));
        out(Out::Cc(channel, 74, timbre));
        out(Out::Bend(channel, bend));
        if self.y_active && self.config.y_target == 5 {
            out(Out::Cc(channel, self.config.y_cc, self.y));
        }
    }
    pub(super) fn strike(
        &mut self,
        note: u8,
        velocity: f32,
        duration: u64,
        out: &mut impl FnMut(Out),
    ) {
        self.setup(out);
        for i in 0..15 {
            if self.voices[i].is_some_and(|v| v.note == note) {
                self.end_voice(i, 0.0, out);
            }
        }
        let capacity = if self.config.mpe {
            self.config.members.clamp(1, 15) as usize
        } else {
            15
        };
        let index = self.voices[..capacity]
            .iter()
            .position(Option::is_none)
            .unwrap_or_else(|| {
                self.voices[..capacity]
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, v)| v.map_or(0, |v| v.started))
                    .map_or(0, |(i, _)| i)
            });
        self.end_voice(index, 0.0, out);
        let channel = if !self.config.mpe && self.config.split_channels {
            if self.full_notes.len > 0 && note == self.full_notes.values[0] {
                self.config.bass_channel
            } else {
                self.config.upper_channel
            }
        } else {
            self.member(index)
        };
        self.emit_expression(channel, out);
        out(Out::On(channel, note, velocity.clamp(0.01, 1.0)));
        self.voices[index] = Some(Voice {
            note,
            channel,
            started: self.now,
            off: self.now.saturating_add(duration),
        });
        for i in 0..self.config.strings as usize {
            if self.notes.string(i) == Some(note) {
                self.strikes[i] = self.strikes[i].wrapping_add(1);
            }
        }
    }
    pub(super) fn schedule(&mut self, event: Scheduled) {
        if let Some(slot) = self.scheduled.iter_mut().find(|v| v.is_none()) {
            *slot = Some(event);
        }
        // A full scheduler drops the new strike, never a pending Note Off.
    }
    pub(super) fn strike_velocity(&self) -> f32 {
        let base = self.root.map_or(self.config.velocity, |s| {
            if s.channel == 16 {
                self.config.velocity
            } else {
                s.velocity * self.config.velocity / 0.8
            }
        });
        if self.y_active && self.config.y_target == 0 {
            (base * self.y / 0.8).clamp(0.01, 1.0)
        } else {
            base.clamp(0.01, 1.0)
        }
    }
    pub(super) fn duration(&self) -> u64 {
        self.ms(self.config.length_ms
            * if self.y_active && self.config.y_target == 1 {
                0.1 + 1.9 * self.y
            } else {
                1.0
            })
    }
    pub fn position(&mut self, value: f32, axis: usize, out: &mut impl FnMut(Out)) {
        if !value.is_finite() {
            return;
        }
        let (min, max, reverse) = if axis == 0 {
            let min = self.config.x_min.clamp(0.0, 0.75);
            (
                min,
                self.config.x_max.clamp(min + 0.25, 1.0),
                self.config.x_reverse,
            )
        } else {
            (self.config.y_min, self.config.y_max, self.config.y_reverse)
        };
        let mut value = ((value - min) / (max - min).max(0.001)).clamp(0.0, 1.0);
        if reverse {
            value = 1.0 - value;
        }
        if axis == 1 {
            self.y = value;
            self.y_active = true;
            self.fan_expression(out);
            return;
        }
        self.x = value;
        let count = self.config.strings.clamp(3, 12) as i16;
        let location = value * (count - 1) as f32;
        if !self.x_primed || self.config.mode != MANUAL || self.notes.len == 0 {
            self.last_string = location.round() as i16;
            self.x_primed = true;
            return;
        }
        // Schmitt boundaries around the halfway point between adjacent strings.
        while self.last_string < count - 1 && location > self.last_string as f32 + 0.55 {
            self.last_string += 1;
            self.pluck_string(self.last_string as usize, out);
        }
        while self.last_string > 0 && location < self.last_string as f32 - 0.55 {
            self.last_string -= 1;
            self.pluck_string(self.last_string as usize, out);
        }
    }
    pub(super) fn pluck_string(&mut self, index: usize, out: &mut impl FnMut(Out)) {
        if let Some(note) = self.notes.string(index) {
            self.strike(note, self.strike_velocity(), self.duration(), out);
        }
    }
    pub fn transport(
        &mut self,
        playing: bool,
        tempo: f64,
        discontinuity: bool,
        out: &mut impl FnMut(Out),
    ) {
        self.tempo = if tempo.is_finite() {
            tempo.clamp(20.0, 400.0)
        } else {
            120.0
        };
        if discontinuity || (self.playing && !playing) {
            self.panic(out);
        }
        if !self.playing && playing {
            self.arp_next = self.now;
            self.arp_step = 0;
        }
        self.playing = playing;
    }
    pub fn tick(&mut self, out: &mut impl FnMut(Out)) {
        for i in 0..15 {
            if self.voices[i].is_some_and(|v| v.off <= self.now) {
                self.end_voice(i, 0.0, out);
            }
        }
        if self.config.mode == ARP && self.notes.len > 0 && self.now >= self.arp_next {
            let total = self.notes.len * self.config.octaves.clamp(1, 4) as usize;
            let index = match self.config.arp_pattern {
                1 => total - 1 - self.arp_step % total,
                2 => {
                    let span = (total * 2).saturating_sub(2).max(1);
                    let p = self.arp_step % span;
                    if p < total {
                        p
                    } else {
                        span - p
                    }
                }
                3 => {
                    // Put the explicitly played alteration immediately after the root when available.
                    let p = self.arp_step % total;
                    let wanted = self.second.map(|s| s.note % 12);
                    let second = self
                        .notes
                        .as_slice()
                        .iter()
                        .position(|n| Some(n % 12) == wanted)
                        .unwrap_or(1.min(self.notes.len - 1));
                    let local = p % self.notes.len;
                    let local = if local == 1 {
                        second
                    } else if local == second {
                        1
                    } else {
                        local
                    };
                    p / self.notes.len * self.notes.len + local
                }
                4 => self.random() as usize % total,
                _ => self.arp_step % total,
            };
            let base = self.sample_rate as f64 * 60.0 / self.tempo * self.config.rate as f64;
            let swing = if self.arp_step.is_multiple_of(2) {
                1.0 + self.config.swing as f64
            } else {
                1.0 - self.config.swing as f64
            };
            let step = (base * swing).max(1.0) as u64;
            let random = self.random() as f32 / u32::MAX as f32;
            if let Some(note) = self.notes.string(index) {
                self.schedule(Scheduled {
                    at: self.now + (step as f32 * 0.1 * self.config.humanize * random) as u64,
                    note,
                    velocity: (self.strike_velocity()
                        * (1.0 - self.config.humanize * 0.2 * random))
                        .max(0.01),
                    duration: (step as f32 * self.config.gate).max(1.0) as u64,
                });
            }
            self.arp_step = self.arp_step.wrapping_add(1);
            self.arp_next = self.now + step;
        }
        for i in 0..64 {
            if self.scheduled[i].is_some_and(|e| e.at <= self.now) {
                if let Some(e) = self.scheduled[i].take() {
                    self.strike(e.note, e.velocity, e.duration, out);
                }
            }
        }
        self.now = self.now.wrapping_add(1);
    }
    fn random(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }
}
