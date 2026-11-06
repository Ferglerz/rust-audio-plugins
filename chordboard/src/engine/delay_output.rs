//! Echo allocation never steals a dry voice or changes an occupied dry channel.
use super::*;
use crate::harmony::Notes;
use crate::midi_fx::source;
pub(super) use super::DELAY_OWNER;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct DelayMapKey {
    tracker: bool,
    follow: bool,
    leading: bool,
    voice_leading: u8,
    mode: u8,
    notes_empty: bool,
    full_empty: bool,
    chord: Notes,
    full: Notes,
    memory: Option<crate::harmony::SavedChord>,
}
impl Engine {
    pub(super) fn new_note_token(&mut self) -> u64 {
        let token = self.next_token;
        self.next_token = self.next_token.wrapping_add(1).max(1);
        token
    }
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn capture_delay_source(&mut self, source: u8, token: u64, note: u8,
        velocity: f32, expression: Expression, off: Option<u64>) {
        self.capture_fx_source(source, token, self.config.output_channel, note, velocity, expression, off, false);
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn capture_fx_source(
        &mut self,
        source: u8,
        token: u64,
        channel: u8,
        note: u8,
        velocity: f32,
        expression: Expression,
        off: Option<u64>,
        retrigger: bool,
    ) {
        if self.resetting {
            return;
        }
        let ctx = self.fx_context();
        self.fx.capture(fx::Input { token, source, channel, note, velocity, expression, off, retrigger }, ctx);
        if !self.fx.active() {
            self.delay.capture(self.arp.data.midi_fx.delay, source, token, note, velocity,
                expression, off, self.now, self.beat, self.sample_rate as f64, self.tempo);
        }
    }
    pub(super) fn release_fx_source(&mut self, token: u64, out: &mut impl FnMut(Out)) {
        if !self.resetting {
            let ctx = self.fx_context();
            self.fx.release(token, ctx, out);
            if !self.fx.active() { self.delay.release(token, self.now, self.beat); }
        }
    }

    pub(super) fn update_delay_power(&mut self, out: &mut impl FnMut(Out)) {
        let power = self.arp.data.midi_fx.delay.enabled;
        if self.delay_power && !power {
            self.reset_delay(out);
        }
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
    pub(super) fn delay_note(&self, mut note: u8) -> u8 {
        let settings = self.arp.data.midi_fx.delay;
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
            note = nearest_chord_pitch(note, chord_notes.as_slice());
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
    pub(super) fn cached_delay_map(&mut self) -> [u8; 128] {
        let key = self.delay_map_key();
        if let Some(cached) = &self.delay_map_cache {
            if cached.0 == key {
                return cached.1;
            }
        }
        let map = std::array::from_fn(|note| self.delay_note(note as u8));
        let result = map;
        self.delay_map_cache = Some(Box::new((key, map)));
        result
    }
    fn delay_map_key(&self) -> DelayMapKey {
        let settings = self.arp.data.midi_fx.delay;
        let chord = if self.config.tracker_enabled {
            self.arp_cycle_chord()
        } else {
            self.full_notes
        };
        DelayMapKey {
            tracker: self.config.tracker_enabled,
            follow: settings.follow_chords,
            leading: settings.voice_leading,
            voice_leading: self.config.voice_leading,
            mode: self.config.mode,
            notes_empty: self.notes.len == 0,
            full_empty: self.full_notes.len == 0,
            chord,
            full: self.full_notes,
            memory: self.memory,
        }
    }
    pub(super) fn tick_delay(&mut self, out: &mut impl FnMut(Out)) {
        if self.fx.active() { return; }
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
                    ..
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
                    fx::expression_out(channel, expression, out);
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
    pub(super) fn capture_bass_fx(
        &mut self,
        input: usize,
        channel: u8,
        note: u8,
        velocity: f32,
    ) {
        self.capture_fx_source(
            source::BASS,
            Self::bass_delay_token(input, note as usize),
            channel,
            note,
            velocity,
            self.channels[input],
            None,
            false,
        );
    }
}

/// Equal distances resolve downward, matching a full 0..=127 scan.
fn nearest_chord_pitch(note: u8, chord: &[u8]) -> u8 {
    let mut pcs = [false; 12];
    for &tone in chord {
        pcs[(tone % 12) as usize] = true;
    }
    if !pcs.iter().any(|&on| on) {
        return note;
    }
    for dist in 0..=127u8 {
        if let Some(lower) = note.checked_sub(dist) {
            if pcs[(lower % 12) as usize] {
                return lower;
            }
        }
        if dist > 0 {
            let upper = note as u16 + dist as u16;
            if upper <= 127 && pcs[(upper as u8 % 12) as usize] {
                return upper as u8;
            }
        }
    }
    note
}
