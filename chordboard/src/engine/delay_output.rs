//! Echo allocation never steals a dry voice or changes an occupied dry channel.
use super::*;
use crate::arpeggiator::source;
pub(super) const DELAY_OWNER: u8 = 6;
impl Engine {
    pub(super) fn new_note_token(&mut self) -> u64 {
        let token = self.next_token;
        self.next_token = self.next_token.wrapping_add(1).max(1);
        token
    }
    pub(super) fn capture_delay_source(
        &mut self,
        source: u8,
        token: u64,
        note: u8,
        velocity: f32,
        expression: Expression,
        off: Option<u64>,
    ) {
        if self.resetting {
            return;
        }
        let settings = self.active_arp_page().delay;
        self.delay.capture(
            settings,
            source,
            token,
            note,
            velocity,
            expression,
            off,
            self.now,
            self.beat,
            self.sample_rate as f64,
            self.tempo,
        );
    }
    pub(super) fn release_delay_source(&mut self, token: u64) {
        if !self.resetting {
            self.delay.release(token, self.now, self.beat);
        }
    }
    pub(super) fn update_delay_power(&mut self, out: &mut impl FnMut(Out)) {
        let power = self.active_arp_page().delay.enabled;
        if self.delay_page == self.arp.active && self.delay_power && !power {
            self.reset_delay(out);
        }
        self.delay_page = self.arp.active;
        self.delay_power = power;
    }
    pub(super) fn reset_delay(&mut self, out: &mut impl FnMut(Out)) {
        self.delay.clear();
        for i in 0..self.voices.len() {
            if self.voices[i].is_some_and(|v| v.owner == DELAY_OWNER) {
                self.end_voice(i, 0.0, out);
            }
        }
    }
    pub(super) fn reclaim_delay_channel(&mut self, channel: u8, out: &mut impl FnMut(Out)) {
        for i in 0..self.voices.len() {
            if self.voices[i].is_some_and(|v| v.owner == DELAY_OWNER && v.channel == channel) {
                self.end_voice(i, 0.0, out);
            }
        }
    }
    pub(super) fn channel_owned(&self, channel: u8) -> bool {
        self.melody_channel(channel)
            || self.bass.iter().any(|notes| notes.contains(&Some(channel)))
            || self.voices.iter().flatten().any(|v| v.channel == channel)
    }
    /// Resolve only new attacks. Existing echo tokens retain their actual pitch
    /// until release, so a harmony change cannot misdirect a pending note-off.
    fn delay_note(&self, mut note: u8) -> u8 {
        let settings = self.active_arp_page().delay;
        let chord_notes = if self.config.tracker_enabled {
            self.arp_cycle_chord()
        } else {
            self.full_notes
        };
        if self.config.tracker_enabled && chord_notes.len == 0
            || !self.config.tracker_enabled && (self.notes.len == 0 || self.full_notes.len == 0)
        {
            return note;
        }
        if settings.follow_chords {
            // Search MIDI pitches rather than clipping octaves near 0 or 127.
            // Equal distances resolve downward, deterministically.
            note = (0..=127u8)
                .filter(|n| {
                    chord_notes
                        .as_slice()
                        .iter()
                        .any(|tone| tone % 12 == n % 12)
                })
                .min_by_key(|n| n.abs_diff(note))
                .unwrap_or(note);
        }
        if settings.voice_leading && self.config.voice_leading < 2 {
            if let Some(recipe) = self.memory {
                let spread = if self.config.mode == CHORD {
                    0
                } else {
                    recipe.spread
                };
                let base = harmony::voice_recipe(recipe, spread);
                let original = base
                    .as_slice()
                    .iter()
                    .copied()
                    .filter(|n| n % 12 == note % 12)
                    .min_by_key(|n| n.abs_diff(note));
                let resolved = self
                    .full_notes
                    .as_slice()
                    .iter()
                    .copied()
                    .filter(|n| n % 12 == note % 12)
                    .min_by_key(|n| n.abs_diff(note));
                if let (Some(original), Some(resolved)) = (original, resolved) {
                    let shifted = i16::from(note) + i16::from(resolved) - i16::from(original);
                    if (0..=127).contains(&shifted) {
                        note = shifted as u8;
                    }
                }
            }
        }
        note
    }
    pub(super) fn tick_delay(&mut self, out: &mut impl FnMut(Out)) {
        while let Some(action) = self.delay.pop_due(self.now, self.beat) {
            match action {
                delay::Action::Off { token } => {
                    for i in 0..self.voices.len() {
                        if self.voices[i]
                            .is_some_and(|v| v.owner == DELAY_OWNER && v.token == token)
                        {
                            self.end_voice(i, 0.0, out);
                        }
                    }
                }
                delay::Action::On {
                    token,
                    note,
                    velocity,
                    mut expression,
                    fine,
                    pan,
                    channel: requested,
                } => {
                    let note = self.delay_note(note);
                    let allowed = |channel: u8| {
                        if self.config.mpe {
                            (0..self.config.members.clamp(1, 15) as usize)
                                .any(|i| self.member(i) == channel)
                        } else {
                            channel != self.config.output_channel
                                && channel != self.config.bass_channel
                                && channel != self.config.upper_channel
                        }
                    };
                    let channel = if requested > 0 {
                        let c = requested - 1;
                        ((!self.config.mpe || allowed(c)) && !self.channel_owned(c)).then_some(c)
                    } else {
                        (0..16).find(|&c| allowed(c) && !self.channel_owned(c))
                    };
                    let Some(channel) = channel else {
                        continue;
                    };
                    let Some(index) = self.voices.iter().position(Option::is_none) else {
                        continue;
                    };
                    let range = if self.config.mpe {
                        self.config.bend_range
                    } else {
                        self.config.master_range
                    }
                    .max(1.0);
                    // RPN pitch sensitivity is needed for fine pitch on regular MIDI too.
                    if self.delay_bend_ranges[channel as usize] != range {
                        midi::emit_bend_range(channel, range, out);
                        self.delay_bend_ranges[channel as usize] = range;
                    }
                    expression.bend = (expression.bend + fine / (200.0 * range)).clamp(0.0, 1.0);
                    out(Out::Bend(channel, expression.bend));
                    out(Out::Pressure(channel, expression.pressure));
                    out(Out::Cc(channel, 74, expression.timbre));
                    out(Out::Cc(channel, 10, (pan + 1.0) * 0.5));
                    out(Out::On(channel, note, velocity));
                    self.voices[index] = Some(Voice {
                        note,
                        channel,
                        started: self.now,
                        off: u64::MAX,
                        layer: false,
                        owner: DELAY_OWNER,
                        attack_expression: Some(expression),
                        token,
                    });
                }
            }
        }
    }
    pub(super) fn bass_delay_token(input: usize, note: usize) -> u64 {
        (1 << 63) | (input * 128 + note) as u64
    }
    pub(super) fn melody_delay_token(channel: u8, note: u8) -> u64 {
        (1 << 62) | (channel as u64 * 128 + note as u64)
    }
    pub(super) fn capture_bass_delay(
        &mut self,
        input: usize,
        channel: u8,
        note: u8,
        velocity: f32,
    ) {
        self.capture_delay_source(
            source::BASS,
            Self::bass_delay_token(input, note as usize),
            note,
            velocity,
            self.channels[channel as usize],
            None,
        );
    }
}
