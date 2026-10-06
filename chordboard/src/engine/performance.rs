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
            let lane = match v.kind {
                KIND_BASS => Lane::Bass,
                KIND_COMP => Lane::Comp,
                KIND_LEAD => Lane::Lead,
                _ => Lane::Arp,
            };
            self.tap.push(
                self.now,
                LaneEvent::Off {
                    lane,
                    channel: v.channel,
                    note: v.note,
                    velocity: release,
                },
            );
            if !self.output_owned(v.channel, v.note) {
                out(Out::Off(v.channel, v.note, release));
            }
        }
    }
    pub(super) fn stop_voices(&mut self, release: f32, out: &mut impl FnMut(Out)) {
        for i in 0..self.voices.len() {
            self.end_voice(i, release, out);
        }
    }
    pub(super) fn fan_expression(&self, out: &mut impl FnMut(Out)) {
        let mut sent = [false; 16];
        for v in self.voices.iter().flatten() {
            if !sent[v.channel as usize] && !self.melody_channel(v.channel) {
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
        self.strike_with_gate(note, velocity, duration, None, self.config.output_channel, KIND_ARP, out);
    }
    pub(super) fn strike_with_gate(
        &mut self,
        note: u8,
        velocity: f32,
        duration: u64,
        gate_step: Option<u64>,
        channel: u8,
        kind: u8,
        out: &mut impl FnMut(Out),
    ) {
        // Publish the final, contoured velocity before evaluating destinations
        // or emitting MIDI. Do not reconfigure here: that would revoice harmony
        // and discard other pending strum notes.
        let velocity = velocity.clamp(0.01, 1.0);
        self.sources[6] = velocity;
        let previous = self.config;
        let mut config = previous;
        self.apply_routes(&mut config);
        self.config = config;
        let has_velocity_route = |id: &str| {
            self.host_config.routes.iter().any(|r| {
                r.active() && r.source == 7 && routing::TARGETS[r.target as usize].id == id
            })
        };
        let duration = if duration == u64::MAX {
            duration
        } else if has_velocity_route("length_ms") {
            self.duration()
        } else if previous.mode == ARP && has_velocity_route("gate") {
            gate_step.map_or(duration, |step| (step as f32 * config.gate).max(1.0) as u64)
        } else {
            duration
        };
        self.strike_prepared(note, channel, velocity, duration, kind, out);
        self.config = previous;
    }
    fn strike_prepared(
        &mut self,
        note: u8,
        channel: u8,
        velocity: f32,
        duration: u64,
        kind: u8,
        out: &mut impl FnMut(Out),
    ) {
        self.setup(out);
        let off = self.now.saturating_add(duration);
        if let Some(voice) = self
            .voices
            .iter_mut()
            .flatten()
            .find(|v| v.note == note && v.channel == channel && v.layer && v.kind == kind)
        {
            // A shared sustained note on this channel must never receive a premature Note Off.
            voice.off = off;
            self.mark_strike(note);
            return;
        }
        for i in 0..self.voices.len() {
            if self.voices[i].is_some_and(|v| v.note == note && v.channel == channel && v.kind == kind) {
                self.end_voice(i, 0.0, out);
            }
        }
        self.allocate_voice(note, velocity, off, false, kind, out);
        self.mark_strike(note);
    }
    pub(super) fn allocate_voice(
        &mut self,
        note: u8,
        velocity: f32,
        off: u64,
        layer: bool,
        kind: u8,
        out: &mut impl FnMut(Out),
    ) {
        if self.config.affect_chords && self.melody.iter().any(|notes| notes[note as usize]) {
            return;
        }
        self.setup(out);
        let capacity = if self.config.mpe {
            self.config.members.clamp(1, 15) as usize
        } else {
            self.voices.len()
        };
        let available = |i: &usize| {
            !self.config.mpe
                || (!self.melody_channel(self.member(*i))
                    && !(self.config.control_base > 0
                        && self.member(*i) == self.config.bass_channel))
        };
        let index = (0..capacity)
            .filter(available)
            .find(|&i| self.voices[i].is_none())
            .or_else(|| {
                // If capacity is reached, prioritize stealing oldest Arp tail
                (0..capacity)
                    .filter(available)
                    .filter(|&i| self.voices[i].is_some_and(|v| !v.layer && v.kind == KIND_ARP))
                    .min_by_key(|&i| self.voices[i].map_or(0, |v| v.started))
            })
            .or_else(|| {
                // Otherwise steal any non-layer voice
                (0..capacity)
                    .filter(available)
                    .filter(|&i| self.voices[i].is_some_and(|v| !v.layer))
                    .min_by_key(|&i| self.voices[i].map_or(0, |v| v.started))
            });
        let Some(index) = index else {
            // Melody and sustained accompaniment have priority over new tails.
            return;
        };
        self.end_voice(index, 0.0, out);
        let channel = if self.config.mpe {
            self.member(index)
        } else {
            match kind {
                KIND_COMP => self.config.comp_channel,
                KIND_BASS => self.config.bass_channel,
                _ => self.config.output_channel,
            }
        };
        if !self.melody_channel(channel) {
            self.emit_expression(channel, out);
        }
        let lane = match kind {
            KIND_BASS => Lane::Bass,
            KIND_COMP => Lane::Comp,
            KIND_LEAD => Lane::Lead,
            _ => Lane::Arp,
        };
        self.tap.push(
            self.now,
            LaneEvent::On {
                lane,
                channel,
                note,
                velocity: velocity.clamp(0.01, 1.0),
            },
        );
        if !self.output_owned(channel, note) {
            out(Out::On(channel, note, velocity.clamp(0.01, 1.0)));
        }
        self.voices[index] = Some(Voice {
            note,
            channel,
            started: self.now,
            off,
            layer,
            kind,
        });
    }
    fn mark_strike(&mut self, note: u8) {
        self.note_strikes[note as usize] = self.note_strikes[note as usize].wrapping_add(1);
        let count = if self.config.mode == MANUAL {
            self.config.strings.clamp(3, 12) as usize
        } else {
            (self.notes.len * self.arp_octaves.max(1) as usize).min(self.strikes.len())
        };
        for i in 0..count {
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
        if self
            .host_config
            .routes
            .iter()
            .any(|r| r.active() && routing::TARGETS[r.target as usize].id == "velocity")
        {
            return self.config.velocity.clamp(0.01, 1.0);
        }
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
        if !value.is_finite() || axis > 1 {
            return;
        }
        self.source(7 + axis, value, out);
        if axis == 0 {
            // A route can shape X, but direct Strumfield input needs no route.
            if let Some(x) = self.config.routed_x {
                self.position_value(x, axis, out);
            } else {
                self.position_value(value, axis, out);
            }
        } else {
            self.position_value(value, axis, out);
        }
    }
    // Routed positions do not feed back into source modulation.
    pub(super) fn position_value(&mut self, value: f32, axis: usize, out: &mut impl FnMut(Out)) {
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
            let ratio = index as f32 / (self.config.strings.saturating_sub(1)).max(1) as f32;
            let velocity = self.strike_velocity()
                * contour_factor(ratio, self.config.contour, self.config.contour_curve);
            self.strike(note, velocity.clamp(0.01, 1.0), self.duration(), out);
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
        self.playing = playing;
    }
    // Only a newly played/recalled chord establishes a new rhythmic origin.
    pub(super) fn restart_arp(&mut self) {
        self.arp_step = 0;
        self.arp_cycle_step = 0;
        self.arp_next = self.now;
        self.arp_fraction = 0.0;
    }

    /// Advance a span without scanning every voice and pending strike per sample.
    /// Output offsets are relative to this span. Inputs at an offset must be
    /// applied before advancing the sample at that offset.
    pub fn advance(&mut self, samples: usize, out: &mut impl FnMut(usize, Out)) {
        let mut offset = 0;
        while offset < samples {
            let deadline = self
                .voices
                .iter()
                .flatten()
                .filter(|voice| !voice.layer)
                .map(|voice| voice.off)
                .chain(self.scheduled.iter().flatten().map(|event| event.at))
                // The arp clock and RNG also advance in the other modes.
                .chain((self.notes.len > 0).then_some(self.arp_next))
                .min()
                .unwrap_or(u64::MAX);
            // Stop at MAX even when idle: tick owns the wrapping clock and
            // processes deadlines there before moving to zero.
            let quiet = deadline.saturating_sub(self.now).min(u64::MAX - self.now);
            let skip = quiet.min((samples - offset) as u64) as usize;
            self.now += skip as u64;
            offset += skip;
            if offset < samples {
                // Retain voice-off, arp, and scheduled-slot ordering exactly.
                self.tick(&mut |event| out(offset, event));
                offset += 1;
            }
        }
    }

    pub fn tick(&mut self, out: &mut impl FnMut(Out)) {
        for i in 0..self.voices.len() {
            if self.voices[i].is_some_and(|v| !v.layer && v.off <= self.now) {
                self.end_voice(i, 0.0, out);
            }
        }
        if self.notes.len > 0 && self.now >= self.arp_next {
            // Keep the clock moving while another performance mode is selected.
            // A new range/pattern takes effect only after the old cycle completes.
            let total = self.notes.len * self.arp_octaves as usize;
            let cycle = self.current_loop_length(total);
            if self.arp_cycle_step >= cycle {
                self.arp_cycle_step = 0;
            }
            if self.arp_cycle_step == 0 {
                self.arp_octaves = self.config.octaves.clamp(1, 4);
                self.arp_pattern = self.config.arp_pattern;
            }
            let total = self.notes.len * self.arp_octaves as usize;
            let (start_idx, end_idx) = self.arp_range(total);
            let mut unskipped = [0usize; 32];
            let mut unskipped_len = 0;
            for s in start_idx..=end_idx {
                if !self.config.is_string_skipped(s) {
                    unskipped[unskipped_len] = s;
                    unskipped_len += 1;
                }
            }
            if self.arp_step.is_multiple_of(2) {
                self.arp_pair_swing = self.config.swing as f64;
                self.arp_pair_rate = self.rate_samples();
            }
            let base = self.arp_pair_rate;
            let swing = if self.arp_step.is_multiple_of(2) {
                1.0 + self.arp_pair_swing
            } else {
                1.0 - self.arp_pair_swing
            };
            let exact_step = (base * swing).max(1.0) + self.arp_fraction;
            let step = exact_step.floor() as u64;
            self.arp_fraction = exact_step - step as f64;
            let current_step = self.arp_step;
            self.arp_step = self.arp_step.wrapping_add(1);
            self.arp_next = self.now + step;

            // 1. Rhythmic Comping
            if self.config.comp_mode == 2 && self.config.comp_is_hit(current_step) {
                let comp_lag = if self.config.comp_lag_ms != 0.0 {
                    (self.config.comp_lag_ms / 1000.0 * self.sample_rate) as i64
                } else {
                    0
                };
                let comp_at = if comp_lag >= 0 {
                    self.now.saturating_add(comp_lag as u64)
                } else {
                    self.now.saturating_sub((-comp_lag) as u64)
                };
                let comp_duration = (step as f32 * 0.75).max(1.0) as u64;
                for i in 0..self.notes.len {
                    let base_note = self.notes.values[i];
                    let trans = (base_note as i16 + self.config.comp_octave as i16 * 12).clamp(0, 127) as u8;
                    self.schedule(Scheduled {
                        at: comp_at,
                        note: trans,
                        velocity: self.config.comp_velocity.clamp(0.01, 1.0),
                        duration: comp_duration,
                        gate_step: Some(step),
                        channel: self.config.comp_channel,
                        kind: KIND_COMP,
                    });
                }
            }

            // 2. Bass (Pulse, Root-5th, With Comp)
            let bass_hit = if self.config.bass_enabled {
                match self.config.bass_mode {
                    2 => true,
                    3 => current_step.is_multiple_of(2),
                    4 => self.config.comp_mode == 2 && self.config.comp_is_hit(current_step),
                    _ => false,
                }
            } else {
                false
            };
            if bass_hit {
                if let Some(root) = self.selected_root() {
                    let is_alt = (self.config.bass_mode == 3 && (current_step / 2).is_multiple_of(2)) || self.config.bass_pad_trigger == 1;
                    let alt_offset = if is_alt { 7 } else { 0 };
                    let bass_pitch = (root as i16 + alt_offset + self.config.bass_octave as i16 * 12).clamp(0, 127) as u8;
                    self.schedule(Scheduled {
                        at: self.now,
                        note: bass_pitch,
                        velocity: self.config.bass_velocity.clamp(0.01, 1.0),
                        duration: (step as f32 * 0.85).max(1.0) as u64,
                        gate_step: Some(step),
                        channel: self.config.bass_channel,
                        kind: KIND_BASS,
                    });
                }
            }

            // 3. Arp Step
            if unskipped_len > 0 {
                let cycle = Self::pattern_length(self.arp_pattern, unskipped_len);
                let pattern_idx = self.arp_index(self.arp_pattern, self.arp_cycle_step, unskipped_len);
                let index = unskipped[pattern_idx];
                let random = self.random() as f32 / u32::MAX as f32;
                let is_muted = self.config.is_string_muted(index);
                let string_vol = self.config.string_volume(index);
                if let Some(note) = self.notes.string(index) {
                    let already_started = self.config.root_on_select
                        && self.selected_root() == Some(note)
                        && self
                            .voices
                            .iter()
                            .flatten()
                            .any(|v| v.note == note && v.started == self.now);
                    if self.config.mode == ARP && !already_started && !is_muted && string_vol > 0.001 {
                        let step_in_cycle = self.arp_cycle_step % cycle;
                        let accent_state = self.config.arp_accents[step_in_cycle % 32];
                        let (mut vel_factor, mut gate_factor) = match accent_state {
                            1 => (1.35, 1.20),
                            2 => (0.60, 0.50),
                            _ => (1.00, 1.00),
                        };

                        let comp_hit_now = self.config.comp_mode == 2 && self.config.comp_is_hit(current_step);
                        if comp_hit_now {
                            if self.config.comp_interlock == 1 {
                                vel_factor = 0.50;
                                gate_factor = 0.60;
                            } else if self.config.comp_interlock == 2 {
                                vel_factor = 1.35;
                                gate_factor = 1.20;
                            }
                        }

                        self.schedule(Scheduled {
                            gate_step: Some(step),
                            at: self.now + (step as f32 * 0.1 * self.config.humanize * random) as u64,
                            note,
                            velocity: (self.strike_velocity()
                                * contour_factor(
                                    self.arp_contour_position(cycle),
                                    self.config.contour,
                                    self.config.contour_curve,
                                )
                                * string_vol
                                * vel_factor
                                * (1.0 - self.config.humanize * 0.2 * random))
                                .clamp(0.01, 1.0),
                            duration: (step as f32 * self.config.gate * gate_factor).max(1.0) as u64,
                            channel: self.config.output_channel,
                            kind: KIND_ARP,
                        });
                    }
                }
                self.arp_cycle_step = (self.arp_cycle_step + 1) % cycle;
            }
        }
        for i in 0..self.scheduled.len() {
            if self.scheduled[i].is_some_and(|e| e.at <= self.now) {
                if let Some(e) = self.scheduled[i].take() {
                    self.strike_with_gate(e.note, e.velocity, e.duration, e.gate_step, e.channel, e.kind, out);
                }
            }
        }
        self.now = self.now.wrapping_add(1);
    }
    pub(super) fn arp_index(&mut self, pattern: u8, step: usize, total: usize) -> usize {
        match pattern {
            1 => total - 1 - step % total,
            2 => {
                let span = (total * 2).saturating_sub(2).max(1);
                let p = step % span;
                if p < total {
                    p
                } else {
                    span - p
                }
            }
            3 => {
                let s = step % (total * 2).max(1);
                let k = s / 2;
                if s.is_multiple_of(2) {
                    k % total
                } else {
                    (k + 2) % total
                }
            }
            4 => {
                if total <= 1 {
                    0
                } else {
                    let s = step % (total * 2);
                    let k = s / 2;
                    let start = total - 1 - (k % total);
                    if s.is_multiple_of(2) {
                        start
                    } else {
                        (start + total - 2) % total
                    }
                }
            }
            5 => {
                // Put the explicitly played alteration immediately after the root when available.
                let p = step % total;
                let wanted = self.second.map(|s| s.note % 12);
                let group_start = p / self.notes.len * self.notes.len;
                let group_len = (total - group_start).min(self.notes.len);
                let second = self
                    .notes
                    .as_slice()
                    .iter()
                    .position(|n| Some(n % 12) == wanted)
                    .filter(|&index| index < group_len)
                    .unwrap_or(1.min(group_len - 1));
                let local = p % self.notes.len;
                let local = if group_len == 1 {
                    0
                } else if local == 1 {
                    second
                } else if local == second {
                    1
                } else {
                    local
                };
                p / self.notes.len * self.notes.len + local
            }
            6 => self.random() as usize % total,
            _ => step % total,
        }
    }
    pub fn pattern_length(pattern: u8, notes: usize) -> usize {
        match pattern {
            2 => (notes * 2).saturating_sub(2).max(1),
            3 | 4 => (notes * 2).max(1),
            _ => notes.max(1),
        }
    }
    pub(super) fn arp_range(&self, total: usize) -> (usize, usize) {
        let start_idx = (self.config.loop_start.clamp(1, 32) - 1) as usize;
        let end_idx = if self.config.loop_end > 0 {
            (self.config.loop_end.clamp(1, 32) - 1) as usize
        } else {
            total.saturating_sub(1)
        };
        if start_idx <= end_idx {
            (start_idx, end_idx)
        } else {
            (end_idx, start_idx)
        }
    }
    pub(super) fn current_loop_length(&self, total: usize) -> usize {
        let (start_idx, end_idx) = self.arp_range(total);
        let mut unskipped_len = 0;
        for s in start_idx..=end_idx {
            if !self.config.is_string_skipped(s) {
                unskipped_len += 1;
            }
        }
        let active_len = if unskipped_len == 0 {
            (end_idx - start_idx + 1).max(1)
        } else {
            unskipped_len
        };
        Self::pattern_length(self.arp_pattern, active_len)
    }

    fn arp_contour_position(&self, cycle: usize) -> f32 {
        (self.arp_cycle_step % cycle) as f32 / cycle.saturating_sub(1).max(1) as f32
    }
    pub(super) fn random(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }
}
