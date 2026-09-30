mod layers;
mod midi;
mod performance;
pub mod routing;
mod types;
use crate::harmony::{self, Notes};
pub use types::*;

pub struct Engine {
    pub sources: [f32; routing::SOURCE_COUNT],
    pub host_config: Config,
    pub memory: Option<harmony::SavedChord>,
    pub recalled: bool,
    recall_voicing: Option<(u8, i8)>,
    pub saved: Option<(usize, u64)>,
    pub full_notes: Notes,
    pub leading_from: Notes,
    pub leading_to: Notes,
    pub leading_serial: u64,
    leading_history: Option<(harmony::SavedChord, Notes)>,
    pub config: Config,
    pub sample_rate: f32,
    pub now: u64,
    pub root: Option<Slot>,
    pub second: Option<Slot>,
    pub notes: Notes,
    pub expression: Expression,
    pub channels: [Expression; 16],
    pub voices: [Option<Voice>; 15],
    pub scheduled: [Option<Scheduled>; 64],
    pub quality: u8,
    control_alteration: Option<u8>,
    pub inversion: u8,
    pub pedal: bool,
    pub down: [bool; 2048 + KEY_TOKEN_COUNT],
    held_inputs: [Option<(Slot, Option<u8>)>; 2048 + KEY_TOKEN_COUNT],
    melody: [[bool; 128]; 16],
    melody_sustain: [bool; 16],
    layer_latched: bool,
    pub x: f32,
    pub y: f32,
    pub y_active: bool,
    pub x_primed: bool,
    pub last_string: i16,
    pub strikes: [u32; 12],
    pub learn: u8,
    pub learned: Option<(usize, Mapping)>,
    pub learned_base: Option<u8>,
    pub input_upper: bool,
    pub input_members: u8,
    pub input_bend: [f32; 16],
    input_bend_values: [f32; 16],
    pub rpn: [[u8; 2]; 16],
    pub cc: [[u8; 128]; 16],
    pub needs_setup: bool,
    pub arp_next: u64,
    pub arp_step: usize,
    pub rng: u32,
    pub tempo: f64,
    pub playing: bool,
    pub auto_reverse: bool,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            sources: routing::SOURCE_DEFAULTS,
            host_config: Config::default(),
            memory: None,
            recalled: false,
            recall_voicing: None,
            saved: None,
            full_notes: Notes::default(),
            leading_from: Notes::default(),
            leading_to: Notes::default(),
            leading_serial: 0,
            leading_history: None,
            config: Config::default(),
            sample_rate: 44100.0,
            now: 0,
            root: None,
            second: None,
            notes: Notes::default(),
            expression: Expression::default(),
            channels: [Expression::default(); 16],
            voices: [None; 15],
            scheduled: [None; 64],
            quality: 0,
            control_alteration: None,
            inversion: 0,
            pedal: false,
            down: [false; 2048 + KEY_TOKEN_COUNT],
            held_inputs: [None; 2048 + KEY_TOKEN_COUNT],
            melody: [[false; 128]; 16],
            melody_sustain: [false; 16],
            layer_latched: false,
            x: 0.0,
            y: 0.8,
            y_active: false,
            x_primed: false,
            last_string: -1,
            strikes: [0; 12],
            learn: 0,
            learned: None,
            learned_base: None,
            input_upper: false,
            input_members: 15,
            input_bend: {
                let mut ranges = [48.0; 16];
                ranges[0] = 2.0;
                ranges
            },
            input_bend_values: [0.5; 16],
            rpn: [[127; 2]; 16],
            cc: [[0; 128]; 16],
            needs_setup: true,
            arp_next: 0,
            arp_step: 0,
            rng: 0x43484f52,
            tempo: 120.0,
            playing: false,
            auto_reverse: false,
        }
    }
}
impl Engine {
    pub fn configure(&mut self, config: Config, out: &mut impl FnMut(Out)) {
        self.configure_inner(config, true, out);
    }
    fn configure_inner(&mut self, config: Config, restart: bool, out: &mut impl FnMut(Out)) {
        let previous_host = self.host_config;
        let old = self.config;
        self.host_config = config;
        let mut config = config;
        if self.recalled {
            if let Some((spread, transpose)) = self.recall_voicing.as_mut() {
                if previous_host.spread != config.spread {
                    *spread = config.spread;
                }
                if previous_host.transpose != config.transpose {
                    *transpose = config.transpose;
                }
                config.spread = *spread;
                config.transpose = *transpose;
            }
        }
        self.apply_routes(&mut config);
        let route = old.mpe != config.mpe
            || old.upper != config.upper
            || old.members != config.members
            || old.output_channel != config.output_channel
            || old.split_channels != config.split_channels
            || old.bass_channel != config.bass_channel
            || old.upper_channel != config.upper_channel;
        let mode = old.mode != config.mode;
        if route || mode {
            self.stop_voices(0.0, out);
            self.scheduled.fill(None);
            self.x_primed = false;
            self.arp_step = 0;
            self.arp_next = self.now;
        }
        if route || old.bend_range != config.bend_range || old.master_range != config.master_range {
            self.needs_setup = true;
        }
        self.config = config;
        if old.quality != config.quality {
            self.quality = config.quality;
            self.control_alteration = None;
        }
        if old.inversion != config.inversion {
            self.inversion = config.inversion;
        }
        if old.latch && !config.latch && !config.key_split {
            // Return released, retained inputs to ordinary note-off semantics.
            for slot in [self.second, self.root].into_iter().flatten() {
                if !slot.held {
                    self.release_harmony_input(slot.id, 0.0, out);
                }
            }
        }
        if !config.latch && !config.key_split && !self.pedal && self.root.is_none() {
            self.notes = Notes::default();
            if self.config.mode == CHORD {
                self.stop_voices(0.0, out);
            }
            self.scheduled.fill(None);
        }
        if old.control_base != config.control_base
            || old.key_split != config.key_split
            || (config.key_split && old.split_note != config.split_note)
        {
            self.panic(out);
        }
        if old.mappings != config.mappings || old.strings != config.strings {
            self.x_primed = false;
        }
        if old.voice_leading != config.voice_leading
            || old.inversion != config.inversion
            || old.transpose != config.transpose
            || old.spread != config.spread
            || mode
        {
            self.leading_history = None;
        }
        let revoice = old.voice_leading != config.voice_leading
            || old.quality != config.quality
            || old.inversion != config.inversion
            || old.transpose != config.transpose
            || old.spread != config.spread
            || old.filter != config.filter
            || old.root_on_select != config.root_on_select;
        if restart {
            if revoice || mode {
                self.rebuild(out);
            } else if route {
                self.start_mode(out);
            }
        }
        if old.routed_x != config.routed_x || mode {
            if let Some(x) = config.routed_x {
                self.position_value(x, 0, out);
            }
        }
        if restart
            && (old.always_bass != config.always_bass
                || old.always_chord != config.always_chord
                || old.latch != config.latch
                || route
                || mode
                || revoice)
        {
            self.sync_layers(out);
        }
        if route
            && self
                .melody
                .iter()
                .any(|notes| notes.iter().any(|&held| held))
        {
            self.setup(out);
            for channel in 0..16 {
                if self.melody_channel(channel) {
                    self.emit_melody_expression(channel, out);
                }
            }
        }
    }
    pub fn note_on(
        &mut self,
        id: u16,
        note: u8,
        channel: u8,
        velocity: f32,
        quality: Option<u8>,
        out: &mut impl FnMut(Out),
    ) {
        if id as usize >= self.down.len() || self.down[id as usize] {
            return;
        }
        self.down[id as usize] = true;
        if self.recalled {
            self.root = None;
            self.second = None;
            self.recalled = false;
        }
        if self.config.latch || self.config.key_split {
            if self.root.is_some_and(|s| !s.held) {
                self.root = None;
                self.second = None;
            } else if self.second.is_some_and(|s| !s.held) {
                self.second = None;
            }
        }
        let slot = Slot {
            id,
            note,
            channel,
            held: true,
            velocity,
        };
        self.held_inputs[id as usize] = Some((slot, quality));
        if self.root.is_some() && self.second.is_some() {
            return;
        }
        if self.root.is_none() {
            self.scheduled.fill(None);
            self.root = Some(slot);
            self.quality = quality.unwrap_or(self.quality);
            if quality.is_some() {
                self.control_alteration = None;
            }
            self.expression = if channel < 16 {
                self.channels[channel as usize]
            } else {
                Expression::default()
            };
            self.arp_step = 0;
            self.arp_next = self.now;
        } else {
            self.second = Some(slot);
        }
        self.sources[6] = velocity.clamp(0.0, 1.0);
        self.configure_inner(self.host_config, false, out);
        self.rebuild(out);
    }
    pub fn note_off(&mut self, id: u16, velocity: f32, out: &mut impl FnMut(Out)) {
        if id as usize >= self.down.len() {
            return;
        }
        if !self.down[id as usize] {
            return;
        }
        self.down[id as usize] = false;
        self.held_inputs[id as usize] = None;
        if self.config.latch || self.config.key_split {
            // Releases never change held harmony or restart strums/arp timing.
            // Retain a released alteration until a new input replaces it.
            for slot in [&mut self.root, &mut self.second].into_iter().flatten() {
                if slot.id == id {
                    slot.held = false;
                }
            }
            if self.root.is_none_or(|s| !s.held) && self.second.is_none_or(|s| !s.held) {
                self.root = None;
                self.second = None;
            }
            self.sync_layers(out);
            return;
        }
        self.release_harmony_input(id, velocity, out);
        self.sync_layers(out);
    }
    fn release_harmony_input(&mut self, id: u16, velocity: f32, out: &mut impl FnMut(Out)) {
        // A single remaining key starts a fresh chord, even if admission was
        // deferred while two earlier inputs owned the harmony.
        let mut held = self.held_inputs.iter().flatten();
        if let Some(&(slot, quality)) = held.next() {
            if held.next().is_none() && !self.root.is_some_and(|root| root.id == slot.id) {
                self.root = None;
                self.second = None;
                self.down[slot.id as usize] = false;
                self.note_on(
                    slot.id,
                    slot.note,
                    slot.channel,
                    slot.velocity,
                    quality,
                    out,
                );
                return;
            }
        }
        if self.second.is_some_and(|s| s.id == id) {
            self.second = None;
            if self.root.is_some_and(|s| s.held) {
                self.rebuild(out);
                return;
            }
        } else if let Some(root) = self.root.as_mut().filter(|s| s.id == id) {
            root.held = false;
        } else {
            return;
        }
        if self.root.is_some_and(|s| !s.held) && self.second.is_none() {
            self.root = None;
            if !self.config.latch && !self.pedal {
                self.notes = Notes::default();
                if self.config.mode == CHORD {
                    self.stop_voices(velocity, out);
                }
                self.scheduled.fill(None);
            }
        }
    }
    pub fn command(&mut self, cmd: Command, out: &mut impl FnMut(Out)) {
        match cmd {
            Command::KeyDown(key, note, quality) => self.note_on(
                2048 + key as u16,
                note,
                16,
                self.config.velocity,
                Some(quality),
                out,
            ),
            Command::KeyUp(key) => self.note_off(2048 + key as u16, 0.0, out),
            Command::ReleaseKeyboard => {
                let had_keyboard = self.down[2048..].iter().any(|&v| v);
                for key in 0..KEY_TOKEN_COUNT {
                    if self.down[2048 + key] {
                        self.note_off((2048 + key) as u16, 0.0, out);
                    }
                }
                if had_keyboard && self.root.is_none() && !self.config.key_split {
                    self.notes = Notes::default();
                    self.stop_voices(0.0, out);
                    self.scheduled.fill(None);
                }
            }
            Command::SetControlChord(interval) => {
                let (quality, alteration) = harmony::CONTROL_CHORDS[interval.min(11) as usize];
                self.quality = quality;
                self.control_alteration = alteration;
                if self.root.is_none() {
                    if let Some(memory) = self.memory.as_mut() {
                        memory.second = None;
                    }
                }
                self.rebuild(out);
            }
            Command::SetQuality(quality) => {
                if self.quality != quality.min(11) || self.control_alteration.is_some() {
                    self.quality = quality.min(11);
                    self.control_alteration = None;
                    self.rebuild(out);
                }
            }
            Command::SetInversion(inversion) => {
                let value = inversion % self.full_notes.len.max(1) as u8;
                if self.inversion != value {
                    self.inversion = value;
                    self.leading_history = None;
                    self.rebuild(out);
                }
            }
            Command::Inversion(step) => {
                let n = self.full_notes.len.max(1) as i16;
                self.inversion = (self.inversion as i16 + step as i16).rem_euclid(n) as u8;
                self.leading_history = None;
                self.rebuild(out);
            }
            Command::Capture(index) => {
                if let Some(chord) = self.memory {
                    self.saved = Some((index.min(7), chord.encode()));
                }
            }
            Command::Recall(word) => {
                if let Some(chord) = harmony::SavedChord::decode(word) {
                    self.scheduled.fill(None);
                    self.down.fill(false);
                    self.held_inputs.fill(None);
                    for channel in 0..16 {
                        for note in 0..128 {
                            self.down[channel * 128 + note] = self.melody[channel][note];
                        }
                    }
                    self.root = Some(Slot {
                        id: 2048 + KEY_TOKEN_COUNT as u16 - 1,
                        note: chord.root,
                        channel: 16,
                        held: true,
                        velocity: self.config.velocity,
                    });
                    self.second = chord.second.map(|note| Slot {
                        id: 2048 + KEY_TOKEN_COUNT as u16 - 2,
                        note,
                        channel: 16,
                        held: false,
                        velocity: self.config.velocity,
                    });
                    self.quality = chord.quality;
                    self.control_alteration = None;
                    self.inversion = chord.inversion;
                    self.config.spread = chord.spread;
                    self.config.transpose = chord.transpose;
                    self.recalled = true;
                    self.recall_voicing = Some((chord.spread, chord.transpose));
                    self.configure_inner(self.host_config, false, out);
                    self.rebuild(out);
                }
            }
            Command::BeginGesture(x, y) => {
                self.position(y, 1, out);
                self.x_primed = false;
                self.position(x, 0, out);
                if self.config.mode == MANUAL {
                    self.pluck_string(self.last_string.max(0) as usize, out);
                }
            }
            Command::Panic => self.panic(out),
            Command::Learn(target) => {
                self.learn = if matches!(target, 0..=2 | 4) {
                    target
                } else {
                    0
                }
            }
            Command::X(x) => self.position(x, 0, out),
            Command::Y(y) => self.position(y, 1, out),
            Command::EndGesture => {
                self.y_active = false;
                self.fan_expression(out);
            }
        }
    }
    pub fn panic(&mut self, out: &mut impl FnMut(Out)) {
        self.stop_voices(0.0, out);
        self.stop_melody(out);
        self.scheduled.fill(None);
        self.root = None;
        self.second = None;
        self.notes = Notes::default();
        self.full_notes = Notes::default();
        self.leading_from = Notes::default();
        self.leading_to = Notes::default();
        self.memory = None;
        self.leading_history = None;
        self.recalled = false;
        self.recall_voicing = None;
        self.down.fill(false);
        self.held_inputs.fill(None);
        self.pedal = false;
        self.x_primed = false;
        self.y_active = false;
        self.expression = Expression::default();
        self.channels.fill(Expression::default());
        self.input_bend_values.fill(0.5);
        self.needs_setup = true;
        let master = self.master();
        out(Out::Cc(master, 64, 0.0));
        out(Out::Bend(master, 0.5));
        out(Out::Pressure(master, 0.0));
        out(Out::Cc(master, 74, 0.5));
    }
    fn rebuild(&mut self, out: &mut impl FnMut(Out)) {
        self.rebuild_harmony(true, out);
    }
    fn rebuild_harmony(&mut self, play: bool, out: &mut impl FnMut(Out)) {
        let Some(root_note) = self.root.map(|s| s.note).or(self.memory.map(|m| m.root)) else {
            return;
        };
        let second = if self.root.is_some() {
            self.second.map(|s| s.note).or_else(|| {
                self.control_alteration
                    .map(|interval| (root_note % 12 + interval) % 12)
            })
        } else {
            self.control_alteration
                .map(|interval| (root_note % 12 + interval) % 12)
                .or(self.memory.and_then(|m| m.second))
        };
        let recipe = harmony::SavedChord {
            root: root_note,
            second,
            quality: self.quality,
            inversion: self.inversion,
            spread: self.config.spread,
            transpose: self.config.transpose,
        };
        let spread = if self.config.mode == CHORD {
            0
        } else {
            recipe.spread
        };
        let base = harmony::voice(
            root_note,
            recipe.quality,
            second,
            recipe.transpose,
            recipe.inversion,
            spread,
        );
        self.full_notes = match self.leading_history {
            Some((previous, notes)) if self.config.voice_leading < 2 => {
                if previous == recipe {
                    notes
                } else {
                    harmony::lead(
                        recipe,
                        spread,
                        previous,
                        notes,
                        base,
                        self.config.voice_leading == 1,
                    )
                }
            }
            _ => base,
        };
        if let Some((previous, notes)) = self.leading_history {
            if previous != recipe
                && self.config.voice_leading < 2
                && notes.len > 0
                && self.full_notes.len > 0
            {
                self.leading_from = notes;
                self.leading_to = self.full_notes;
                self.leading_serial = self.leading_serial.wrapping_add(1);
            }
        }
        self.leading_history = Some((recipe, self.full_notes));
        self.notes = harmony::filter_notes(self.full_notes, self.config.filter);
        self.memory = Some(recipe);
        let count = harmony::intervals(
            self.quality,
            second.map(|n| (n as i16 - root_note as i16).rem_euclid(12) as u8),
        )
        .len;
        self.inversion %= count.max(1) as u8;
        self.scheduled.fill(None);
        if self.root.is_some() || self.config.latch || self.config.key_split || self.pedal {
            if play {
                self.start_mode(out);
            }
        } else {
            self.notes = Notes::default();
        }
    }
    fn start_mode(&mut self, out: &mut impl FnMut(Out)) {
        if self.notes.len == 0 {
            return;
        }
        match self.config.mode {
            CHORD => {
                for i in 0..15 {
                    if self.voices[i].is_some_and(|v| !self.notes.as_slice().contains(&v.note)) {
                        let end = self.now.saturating_add(self.duration());
                        if let Some(voice) = self.voices[i].as_mut() {
                            voice.off = voice.off.min(end);
                        }
                    }
                }
                for i in 0..self.notes.len {
                    let note = self.notes.values[i];
                    if let Some(voice) = self.voices.iter_mut().flatten().find(|v| v.note == note) {
                        voice.off = u64::MAX;
                    } else {
                        self.strike(note, self.strike_velocity(), u64::MAX, out);
                    }
                }
            }
            AUTO => {
                let reverse =
                    self.config.direction == 1 || (self.config.direction == 2 && self.auto_reverse);
                self.auto_reverse = !self.auto_reverse;
                let count = self.config.strings.clamp(3, 12) as usize;
                let played = (self.config.strings_played as usize).clamp(1, count);
                for i in 0..played {
                    let idx = if reverse { count - 1 - i } else { i };
                    let Some(note) = self.notes.string(idx) else {
                        continue;
                    };
                    let ratio = i as f32 / (played - 1).max(1) as f32;
                    self.schedule(Scheduled {
                        at: self.now + self.ms(self.sweep_ms() * ratio),
                        note,
                        velocity: (self.strike_velocity()
                            * (1.0 + self.config.contour * (ratio - 0.5)))
                            .clamp(0.01, 1.0),
                        duration: self.duration(),
                    });
                }
            }
            MANUAL => {
                self.x_primed = false;
            }
            ARP => {
                self.arp_next = self.now;
            }
            _ => {}
        }
        if self.config.root_on_select {
            if let Some(root) = self.selected_root() {
                for slot in &mut self.scheduled {
                    if slot.is_some_and(|e| e.note == root) {
                        *slot = None;
                    }
                }
                if !self.voices.iter().flatten().any(|v| v.note == root) {
                    self.strike(root, self.strike_velocity(), self.duration(), out);
                }
            }
        }
        self.sync_layers(out);
    }
    pub fn snapshot(&self) -> Snapshot {
        let mut accepted = 0;
        let mut ignored = 0;
        for key in 0..harmony::KEY_COUNT {
            for token in [key, key + POINTER_KEY_OFFSET as usize] {
                let id = 2048 + token as u16;
                if self.down[id as usize] {
                    if self.root.is_some_and(|s| s.id == id)
                        || self.second.is_some_and(|s| s.id == id)
                    {
                        accepted |= 1 << key;
                    } else {
                        ignored |= 1 << key;
                    }
                }
            }
        }
        let mut held_notes = [false; 128];
        for (id, &held) in self.down[..2048].iter().enumerate() {
            if held {
                held_notes[id % 128] = true;
            }
        }
        for (slot, _) in self.held_inputs[2048..].iter().flatten() {
            held_notes[slot.note as usize] = true;
        }
        let mut sounding_notes = [false; 128];
        for channel in self.owned_output_notes() {
            for (note, sounding) in channel.into_iter().enumerate() {
                sounding_notes[note] |= sounding;
            }
        }
        Snapshot {
            held_notes,
            sounding_notes,
            sources: self.sources,
            routed: self.routed_values(),
            captured: self.memory.map_or(0, |m| m.encode()),
            full_notes: self.full_notes,
            leading_from: self.leading_from,
            leading_to: self.leading_to,
            leading_serial: self.leading_serial,
            notes: self.notes,
            root: self.root.map_or(-1, |s| s.note as i16),
            second: self.second.map_or(-1, |s| s.note as i16),
            quality: self.quality,
            control_alteration: self.control_alteration,
            inversion: self.inversion,
            accepted,
            ignored,
            pressure: self.expression.pressure,
            timbre: self.expression.timbre,
            bend: self.expression.bend,
            x: self.x,
            y: self.y,
            strikes: self.strikes,
            voices: self.voices.iter().flatten().count() as u8,
            learning: self.learn,
            output_mpe: self.config.mpe,
            tempo: self.tempo as f32,
        }
    }
    pub(super) fn selected_root(&self) -> Option<u8> {
        let root = self.root.map(|s| s.note).or(self.memory.map(|m| m.root))? as i16
            + self.config.transpose as i16;
        (0..=127).contains(&root).then_some(root as u8)
    }
    fn sweep_ms(&self) -> f32 {
        if self.config.strum_sync {
            self.config.strum_beats * 60_000.0 / self.tempo as f32
        } else {
            self.config.strum_ms
        }
    }
    fn ms(&self, ms: f32) -> u64 {
        (ms.max(0.0) * self.sample_rate / 1000.0) as u64
    }
    fn master(&self) -> u8 {
        if !self.config.mpe {
            self.config.output_channel
        } else if self.config.upper {
            15
        } else {
            0
        }
    }
    fn member(&self, index: usize) -> u8 {
        if !self.config.mpe {
            self.config.output_channel
        } else if self.config.upper {
            14 - index as u8
        } else {
            1 + index as u8
        }
    }
}

#[cfg(test)]
mod tests;
