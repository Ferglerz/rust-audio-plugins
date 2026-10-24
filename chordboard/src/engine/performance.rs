use super::*;
impl Engine {
    pub(super) fn manual_active(&self) -> bool {
        self.config.strum_enabled && self.config.mode == MANUAL
    }
    pub(super) fn controller_baseline(&self, cc: u8) -> u8 {
        let index = match cc {
            1 => Some(1),
            11 => Some(2),
            2 => Some(3),
            74 => Some(5),
            _ => None,
        };
        if let Some(value) = index.and_then(|i| self.source_overrides[i]) {
            return (value * 127.0).round() as u8;
        }
        let source =
            self.root
                .filter(|s| s.channel < 16)
                .map_or(if self.input_upper { 15 } else { 0 }, |s| s.channel) as usize;
        if self.cc_known[source][cc as usize] {
            self.cc[source][cc as usize]
        } else {
            match cc {
                7 | 11 => 127,
                10 => 64,
                _ => 0,
            }
        }
    }
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
            midi::emit_bend_range(channel, range, out);
        }
    }
    pub(super) fn end_voice(&mut self, i: usize, release: f32, out: &mut impl FnMut(Out)) {
        if let Some(v) = self.voices[i].take() {
            if !self.output_owned(v.channel, v.note) {
                out(Out::Off(v.channel, v.note, release));
            }
            if v.owner == delay_output::DELAY_OWNER {
                if !self.channel_owned(v.channel) {
                    out(Out::Cc(v.channel, 10, 0.5));
                    out(Out::Bend(v.channel, 0.5));
                }
            } else {
                self.release_delay_source(v.token);
            }
        }
    }
    pub(super) fn stop_voices(&mut self, release: f32, out: &mut impl FnMut(Out)) {
        for i in 0..self.voices.len() {
            if self.resetting
                || self.voices[i].is_some_and(|v| {
                    v.owner != delay_output::DELAY_OWNER && v.owner != arpeggiator::ARP_OWNER
                })
            {
                self.end_voice(i, release, out);
            }
        }
    }
    pub(super) fn newest_voice(&self, channel: u8) -> Option<Voice> {
        self.voices
            .iter()
            .enumerate()
            .filter_map(|(i, v)| v.filter(|v| v.channel == channel).map(|v| (i, v)))
            .max_by_key(|(i, v)| (v.started, *i))
            .map(|(_, v)| v)
    }
    pub(super) fn fan_expression(&self, out: &mut impl FnMut(Out)) {
        for channel in 0..16 {
            if self.melody_channel(channel) {
                continue;
            }
            if let Some(v) = self.newest_voice(channel) {
                if v.owner != delay_output::DELAY_OWNER {
                    self.emit_expression(channel, out);
                }
            }
        }
    }
    pub(super) fn attack_expression(&self) -> Expression {
        let mut e = self.routed_expression();
        if self.y_active {
            match self.config.y_target {
                2 => e.pressure = self.y,
                3 => e.timbre = self.y,
                4 => e.bend = self.y,
                _ => {}
            }
        }
        e
    }
    fn emit_expression(&self, channel: u8, out: &mut impl FnMut(Out)) {
        let e = self.attack_expression();
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
        self.emit_routed_controllers(channel, out);
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
        self.strike_with_gate(note, velocity, duration, None, out);
    }
    fn strike_with_gate(
        &mut self,
        note: u8,
        velocity: f32,
        duration: u64,
        gate_step: Option<u64>,
        out: &mut impl FnMut(Out),
    ) {
        self.strike_owned_with_gate(note, velocity, duration, gate_step, 0, out);
    }
    pub(super) fn strike_selection_root(&mut self, note: u8, out: &mut impl FnMut(Out)) {
        self.strike_owned_with_gate(
            note,
            self.strike_velocity(),
            self.duration(),
            None,
            SELECT_ROOT_OWNER,
            out,
        );
    }
    #[allow(clippy::too_many_arguments)]
    fn strike_owned_with_gate(
        &mut self,
        note: u8,
        velocity: f32,
        duration: u64,
        _gate_step: Option<u64>,
        owner: u8,
        out: &mut impl FnMut(Out),
    ) {
        self.strike_prepared(note, velocity.clamp(0.01, 1.0), duration, owner, out);
    }
    pub(super) fn strike_prepared(
        &mut self,
        note: u8,
        velocity: f32,
        duration: u64,
        owner: u8,
        out: &mut impl FnMut(Out),
    ) {
        self.setup(out);
        let off = self.now.saturating_add(duration);
        if let Some(voice) = self
            .voices
            .iter_mut()
            .flatten()
            .find(|v| v.note == note && v.layer && v.owner == owner)
        {
            // Rearticulate the sustained string without discarding its layer
            // ownership. Its strum tail still survives release of the layer gate.
            voice.off = off;
            let channel = voice.channel;
            self.emit_expression(channel, out);
            out(Out::Off(channel, note, 0.0));
            out(Out::On(channel, note, velocity));
            let token = self.new_note_token();
            self.capture_delay_source(
                crate::arpeggiator::source::STRUM,
                token,
                note,
                velocity,
                self.attack_expression(),
                (off != u64::MAX).then_some(off),
            );
            self.mark_strike(note);
            return;
        }
        for i in 0..self.voices.len() {
            if self.voices[i].is_some_and(|v| v.note == note && v.owner == owner) {
                self.end_voice(i, 0.0, out);
            }
        }
        self.allocate_owned_voice(note, velocity, off, false, owner, None, out);
        self.mark_strike(note);
    }
    pub(super) fn allocate_voice(
        &mut self,
        note: u8,
        velocity: f32,
        off: u64,
        layer: bool,
        out: &mut impl FnMut(Out),
    ) {
        self.allocate_owned_voice(note, velocity, off, layer, 0, None, out);
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn allocate_owned_voice(
        &mut self,
        note: u8,
        velocity: f32,
        off: u64,
        layer: bool,
        owner: u8,
        attack_expression: Option<Expression>,
        out: &mut impl FnMut(Out),
    ) {
        if self.config.affect_chords && self.melody.iter().any(|notes| notes[note as usize]) {
            return;
        }
        self.setup(out);
        let capacity = if self.config.mpe {
            self.config.members.clamp(1, 15) as usize
        } else if self.config.arp_enabled {
            self.voices.len()
        } else {
            15
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
                (0..capacity)
                    .filter(available)
                    .filter(|&i| self.voices[i].is_some_and(|v| !v.layer))
                    .min_by_key(|&i| {
                        self.voices[i].map_or((false, 0), |v| {
                            (v.owner != delay_output::DELAY_OWNER, v.started)
                        })
                    })
            });
        let Some(index) = index else {
            // Melody and sustained accompaniment have priority over new tails.
            return;
        };
        self.end_voice(index, 0.0, out);
        let channel = if !self.config.mpe && owner == 3 {
            self.config.bass_channel
        } else {
            self.member(index)
        };
        self.reclaim_delay_channel(channel, out);
        if !self.melody_channel(channel) {
            self.emit_expression(channel, out);
        }
        let owned = self.output_owned(channel, note);
        // A selection root may share this channel and pitch with a live strum.
        // Keep both owners, but give the gesture an audible attack.
        let restrike = owner == 0
            && !layer
            && self.voices.iter().flatten().any(|voice| {
                voice.channel == channel && voice.note == note && voice.owner == SELECT_ROOT_OWNER
            });
        if restrike {
            out(Out::Off(channel, note, 0.0));
        }
        if !owned || restrike {
            out(Out::On(channel, note, velocity.clamp(0.01, 1.0)));
        }
        let token = self.new_note_token();
        self.voices[index] = Some(Voice {
            note,
            channel,
            started: self.now,
            off,
            layer,
            owner,
            attack_expression,
            token,
        });
        let source = if owner == arpeggiator::ARP_OWNER {
            crate::arpeggiator::source::ARP
        } else if layer {
            let bass = self.config.always_bass && self.selected_root() == Some(note);
            let chord = self.config.always_chord || self.config.mode == CHORD;
            (if bass {
                crate::arpeggiator::source::BASS
            } else {
                0
            }) | (if chord || !bass {
                crate::arpeggiator::source::CHORDS
            } else {
                0
            })
        } else if owner == SELECT_ROOT_OWNER || self.config.mode == CHORD {
            crate::arpeggiator::source::CHORDS
        } else {
            crate::arpeggiator::source::STRUM
        };
        self.capture_delay_source(
            source,
            token,
            note,
            velocity,
            attack_expression.unwrap_or_else(|| self.attack_expression()),
            (!layer && off != u64::MAX).then_some(off),
        );
    }
    fn mark_strike(&mut self, note: u8) {
        self.note_strikes[note as usize] = self.note_strikes[note as usize].wrapping_add(1);
        for i in 0..self.string_count() {
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
        let base = self.base_strike_velocity();
        if self.y_active
            && self.config.y_target == 0
            && !self
                .host_config
                .routes
                .iter()
                .any(|r| r.active() && routing::TARGETS[r.target as usize].id == "velocity")
        {
            (base * self.y / 0.8).clamp(0.01, 1.0)
        } else {
            base
        }
    }
    pub(super) fn base_strike_velocity(&self) -> f32 {
        if self
            .host_config
            .routes
            .iter()
            .any(|r| r.active() && routing::TARGETS[r.target as usize].id == "velocity")
        {
            return self.config.velocity.clamp(0.01, 1.0);
        }
        if let Some(velocity) = self.source_overrides[6] {
            return (velocity * self.config.velocity / 0.8).clamp(0.01, 1.0);
        }
        let base = self.root.map_or(self.config.velocity, |s| {
            if s.channel == 16 {
                self.config.velocity
            } else {
                s.velocity * self.config.velocity / 0.8
            }
        });
        base.clamp(0.01, 1.0)
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
                self.position_value(self.source_overrides[7].unwrap_or(value), axis, out);
            }
        } else {
            self.position_value(self.source_overrides[8].unwrap_or(value), axis, out);
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
        let count = self.string_count() as i16;
        let location = value * (count - 1) as f32;
        self.last_string = self.last_string.min(count - 1);
        if !self.x_primed || !self.manual_active() || self.notes.len == 0 {
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
    pub(super) fn string_count(&self) -> usize {
        harmony::string_count(self.notes.len, self.config.octaves)
    }
    pub(super) fn pluck_string(&mut self, index: usize, out: &mut impl FnMut(Out)) {
        if let Some(note) = self.notes.string(index) {
            let ratio = index as f32 / self.string_count().saturating_sub(1).max(1) as f32;
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
        let previous_tempo = self.tempo;
        self.tempo = if tempo.is_finite() {
            tempo.clamp(20.0, 400.0)
        } else {
            120.0
        };
        self.retime_arp(previous_tempo, self.tempo);
        if discontinuity || (self.playing && !playing) {
            let selected = (self.memory, self.notes, self.full_notes, self.expression);
            let retain = discontinuity
                && playing
                && self.config.arp_mode == crate::arpeggiator::ArpMode::AlignedLoop;
            self.panic(out);
            if retain {
                self.memory = selected.0;
                self.notes = selected.1;
                self.full_notes = selected.2;
                self.expression = selected.3;
            }
        }
        self.playing = playing;
    }
    // Only a newly played/recalled chord establishes a new rhythmic origin.
    pub(super) fn restart_arp(&mut self) {
        if !self.promoting
            && matches!(
                self.config.arp_mode,
                crate::arpeggiator::ArpMode::Once | crate::arpeggiator::ArpMode::Loop
            )
        {
            self.restart_automatic_arp();
        }
    }

    /// Advance a span without scanning every voice and pending strike per sample.
    /// Output offsets are relative to this span. Inputs at an offset must be
    /// applied before advancing the sample at that offset.
    pub fn advance(&mut self, samples: usize, out: &mut impl FnMut(usize, Out)) {
        self.refresh_arp_events();
        let mut offset = 0;
        while offset < samples {
            self.refresh_arp_events();
            let deadline = self
                .voices
                .iter()
                .flatten()
                .filter(|voice| !voice.layer)
                .map(|voice| voice.off)
                .chain(self.scheduled.iter().flatten().map(|event| event.at))
                // The arp clock and RNG also advance in the other modes.
                .chain(self.arp_deadline())
                .chain(self.delay.deadline(
                    self.now,
                    self.beat,
                    self.sample_rate as f64 * 60.0 / self.tempo,
                ))
                .chain(self.smoothing_active().then_some(self.now))
                .min()
                .unwrap_or(u64::MAX);
            // Stop at MAX even when idle: tick owns the wrapping clock and
            // processes deadlines there before moving to zero.
            let quiet = deadline.saturating_sub(self.now).min(u64::MAX - self.now);
            let skip = quiet.min((samples - offset) as u64) as usize;
            self.now += skip as u64;
            self.beat += skip as f64 / (self.sample_rate as f64 * 60.0 / self.tempo);
            offset += skip;
            if offset < samples {
                // Retain voice-off, arp, and scheduled-slot ordering exactly.
                self.tick(&mut |event| out(offset, event));
                offset += 1;
            }
        }
    }

    pub fn tick(&mut self, out: &mut impl FnMut(Out)) {
        self.tick_route_smoothing(out);
        for i in 0..self.voices.len() {
            if self.voices[i].is_some_and(|v| !v.layer && v.off <= self.now) {
                self.end_voice(i, 0.0, out);
            }
        }
        self.tick_automatic_arp(out);
        self.tick_delay(out);
        for i in 0..self.scheduled.len() {
            if self.scheduled[i].is_some_and(|e| e.at <= self.now) {
                if let Some(e) = self.scheduled[i].take() {
                    self.strike_with_gate(e.note, e.velocity, e.duration, e.gate_step, out);
                }
            }
        }
        self.now = self.now.wrapping_add(1);
        self.beat += 1.0 / (self.sample_rate as f64 * 60.0 / self.tempo);
    }
    pub(super) fn arp_index(&mut self, pattern: u8, step: usize, total: usize) -> usize {
        match pattern {
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
            _ => ordering::index(pattern, step, total, &mut self.rng),
        }
    }
    pub(super) fn pattern_length(pattern: u8, notes: usize) -> usize {
        match pattern {
            2 => (notes * 2).saturating_sub(2).max(1),
            3 | 4 => (notes * 2).max(1),
            _ => notes.max(1),
        }
    }
    pub(super) fn random(&mut self) -> u32 {
        ordering::random(&mut self.rng)
    }
}
