mod layers;
mod midi;
mod performance;
mod ordering;
pub mod routing;
pub mod arpeggiator;
mod delay;
mod delay_output;
mod types;
use crate::harmony::{self, Notes};
pub use types::*;

pub struct Engine {
    pub arp: arpeggiator::Runtime,
    delay: delay::Runtime,
    next_token: u64,
    beat: f64,
    host_beat_available: bool,
    host_tempo_available: bool,
    resetting: bool,
    promoting: bool,
    delay_power: bool,
    delay_page: u8,
    delay_bend_ranges: [f32;16],
    pub sources: [f32; routing::SOURCE_COUNT],
    route_smoothers: [routing::RouteSmoother; routing::ROUTE_COUNT],
    pub(super) source_overrides: [Option<f32>; routing::SOURCE_COUNT],
    input_velocity: f32,
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
    pub additions: u16,
    addition_inputs: [bool; 2048 + KEY_TOKEN_COUNT],
    pub notes: Notes,
    pub expression: Expression,
    pub channels: [Expression; 16],
    pub voices: [Option<Voice>; 48],
    pub scheduled: [Option<Scheduled>; 128],
    pub quality: u8,
    control_alteration: Option<u8>,
    pub inversion: u8,
    pub pedal: bool,
    pub down: [bool; 2048 + KEY_TOKEN_COUNT],
    held_inputs: [Option<(Slot, Option<u8>)>; 2048 + KEY_TOKEN_COUNT],
    melody: [[bool; 128]; 16],
    bass: [[Option<u8>; 128]; 16],
    bass_sustain: [bool; 16],
    melody_sustain: [bool; 16],
    layer_latched: bool,
    pub x: f32,
    pub y: f32,
    pub y_active: bool,
    pub x_primed: bool,
    pub last_string: i16,
    pub strikes: [u32; 12],
    pub note_strikes: [u32; 128],
    pub learn: u8,
    pub learned: Option<(usize, Mapping)>,
    pub learned_base: Option<u8>,
    pub learned_split: Option<LearnedSplit>,
    learned_note_held: [bool; 2048],
    pub input_upper: bool,
    pub input_members: u8,
    pub input_bend: [f32; 16],
    input_bend_values: [f32; 16],
    pub rpn: [[u8; 2]; 16],
    pub cc: [[u8; 128]; 16],
    pub cc_known: [[bool; 128]; 16],
    cc_lsb_seen: [u32; 16],
    pub needs_setup: bool,
    pub rng: u32,
    pub tempo: f64,
    pub playing: bool,
    pub time_sig_host: bool,
    pub time_sig_num: u8,
    pub time_sig_den: u8,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            arp: arpeggiator::Runtime::default(),
            delay: delay::Runtime::default(),
            next_token: 1,
            beat: 0.0,
            host_beat_available: false,
            host_tempo_available: false,
            resetting: false,
            promoting: false,
            delay_power: false,
            delay_page: 0,
            delay_bend_ranges: [0.0;16],
            sources: routing::SOURCE_DEFAULTS,
            route_smoothers: [routing::RouteSmoother::default(); routing::ROUTE_COUNT],
            source_overrides: [None; routing::SOURCE_COUNT],
            input_velocity: 0.8,
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
            additions: 0,
            addition_inputs: [false; 2048 + KEY_TOKEN_COUNT],
            notes: Notes::default(),
            expression: Expression::default(),
            channels: [Expression::default(); 16],
            voices: [None; 48],
            scheduled: [None; 128],
            quality: 0,
            control_alteration: None,
            inversion: 0,
            pedal: false,
            down: [false; 2048 + KEY_TOKEN_COUNT],
            held_inputs: [None; 2048 + KEY_TOKEN_COUNT],
            melody: [[false; 128]; 16],
            bass: [[None; 128]; 16],
            bass_sustain: [false; 16],
            melody_sustain: [false; 16],
            layer_latched: false,
            x: 0.0,
            y: 0.8,
            y_active: false,
            x_primed: false,
            last_string: -1,
            strikes: [0; 12],
            note_strikes: [0; 128],
            learn: 0,
            learned: None,
            learned_base: None,
            learned_split: None,
            learned_note_held: [false; 2048],
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
            cc_known: [[false; 128]; 16],
            cc_lsb_seen: [0; 16],
            needs_setup: true,
            rng: 0x43484f52,
            tempo: 120.0,
            playing: false,
            time_sig_host: false,
            time_sig_num: 4,
            time_sig_den: 4,
        }
    }
}
impl Engine {
    pub fn configure(&mut self, config: Config, out: &mut impl FnMut(Out)) {
        self.configure_inner(config, true, out);
    }
    fn configure_inner(&mut self, config: Config, restart: bool, out: &mut impl FnMut(Out)) {
        let previous_sources = self.source_overrides;
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
        if !config.arp_enabled { self.arp.active=config.arp_page.min(7); }
        let settings = config.arp_settings[self.arp.active as usize];
        if config.arp_enabled {
        config.arp_mode = settings.mode; config.arp_pattern = settings.pattern;
        config.arp_octaves = settings.octaves; config.arp_sync = settings.sync;
        config.arp_free_ms = settings.free_ms; config.rate = settings.rate;
        config.swing = settings.swing; config.humanize = settings.humanize;
        }
        let unrouted_config = config;
        self.prepare_route_smoothing();
        self.apply_routes(&mut config);
        self.source_overrides = self.resolved_source_overrides();
        if config.bass_enabled && config.key_split && config.bass_boundary() >= 0 {
            let (bass, melody) = ordered_splits(config.bass_boundary() as u8, config.split_note);
            if bass as i16 != config.bass_boundary() {
                config.bass_split = bass as i16;
            }
            config.split_note = melody;
        }
        let route = old.mpe != config.mpe
            || old.upper != config.upper
            || old.members != config.members
            || old.output_channel != config.output_channel
            || old.bass_channel != config.bass_channel
            || old.upper_channel != config.upper_channel;
        let mode = old.mode != config.mode;
        if mode || route || old.arp_enabled != config.arp_enabled {
            self.reset_automatic_arp(out);
            config = unrouted_config;
            self.prepare_route_smoothing(); self.apply_routes(&mut config);
            self.source_overrides = self.resolved_source_overrides();
        }
        if route { self.reset_delay(out); self.delay_bend_ranges.fill(0.0); }
        if route {
            self.stop_bass(out);
        }
        if route || mode {
            self.stop_voices(0.0, out);
            self.scheduled.fill(None);
            self.x_primed = false;
        }
        if route || old.bend_range != config.bend_range || old.master_range != config.master_range {
            self.needs_setup = true;
        }
        self.config = config;
        let page_changed=self.arp.applied_page!=self.arp.active;
        self.arp.applied_page=self.arp.active;
        if old.arp_mode != config.arp_mode && !page_changed { self.restart_automatic_arp(); }
        self.update_delay_power(out);
        if old.strum_enabled && !config.strum_enabled && config.mode == MANUAL {
            self.x_primed = false;
            for i in 0..self.voices.len() {
                if self.voices[i].is_some_and(|voice| voice.owner == 0 && !voice.layer) {
                    self.end_voice(i, 0.0, out);
                }
            }
        }
        if old.quality != config.quality {
            self.additions = 0;
            self.refresh_additions();
            self.quality = config.quality;
            self.control_alteration = None;
        }
        if old.inversion != config.inversion {
            self.inversion = config.inversion;
        }
        if old.strum_hold && !config.strum_hold && self.config.mode == AUTO {
            let duration = self.duration();
            for i in 0..self.voices.len() {
                if let Some(voice) = self.voices[i].as_mut() {
                    if !voice.layer && voice.off == u64::MAX {
                        voice.off = self.now.saturating_add(duration);
                    }
                }
            }
        }
        if old.latch && !config.latch && !config.key_split {
            self.additions = 0;
            self.refresh_additions();
            // Return released, retained inputs to ordinary note-off semantics.
            for slot in [self.second, self.root].into_iter().flatten() {
                if !slot.held {
                    self.release_harmony_input(slot.id, 0.0, out);
                }
            }
        }
        if !config.latch && !config.key_split && !self.pedal && self.root.is_none() && !(self.playing && config.arp_enabled && self.memory.is_some()) {
            self.notes = Notes::default();
            if self.config.mode == CHORD {
                self.stop_voices(0.0, out);
            }
            self.scheduled.fill(None);
        }
        if old.bass_enabled != config.bass_enabled
            || old.bass_split != config.bass_split
            || old.control_base != config.control_base
            || old.key_split != config.key_split
            || (config.key_split && old.split_note != config.split_note)
        {
            self.panic(out);
        }
        if old.mappings != config.mappings || old.octaves != config.octaves {
            self.x_primed = false;
        }
        if old.voice_leading != config.voice_leading
            || old.inversion != config.inversion
            || old.transpose != config.transpose
            || old.spread != config.spread
            || mode
        {
            self.leading_history = None;
            self.leading_from = Notes::default();
            self.leading_to = Notes::default();
        }
        let revoice = old.affect_chords != config.affect_chords
            || old.voice_leading != config.voice_leading
            || old.quality != config.quality
            || old.inversion != config.inversion
            || old.transpose != config.transpose
            || old.spread != config.spread
            || old.filter != config.filter
            || old.root_on_select != config.root_on_select;
        if restart {
            if revoice || mode {
                self.rebuild_harmony(mode || self.config.mode != ARP, out);
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
        if previous_sources != self.source_overrides {
            // Restore a removed controller route even if its live baseline is unknown.
            for channel in 0..16 {
                if self.newest_voice(channel).is_none_or(|v|v.owner==delay_output::DELAY_OWNER) || self.melody_channel(channel) { continue; }
                for (index, cc) in [(1, 1), (2, 11), (3, 2)] {
                    if previous_sources[index].is_some() && self.source_overrides[index].is_none() {
                        out(Out::Cc(channel, cc, self.controller_baseline(cc) as f32 / 127.0));
                    }
                }
            }
            if self.config.routed_x.is_none() && (self.source_overrides[7].is_some() || previous_sources[7].is_some()) {
                self.position_value(self.source_overrides[7].unwrap_or(self.sources[7]), 0, out);
            }
            if self.source_overrides[8].is_some() || previous_sources[8].is_some() {
                self.position_value(self.source_overrides[8].unwrap_or(self.sources[8]), 1, out);
            }
            self.fan_expression(out);
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
    /// Recompute contributions by identity; released latched pitches retain their recipe.
    fn refresh_additions(&mut self) {
        let Some(root_note) = self.root.map(|s| s.note).or(self.memory.map(|m| m.root)) else { return; };
        if !self.config.latch {
            self.additions = 0;
        }
        for (slot, _) in self.held_inputs.iter().flatten() {
            if !self.addition_inputs[slot.id as usize] || self.root.is_some_and(|r| slot.id == r.id) || self.second.is_some_and(|s| s.id == slot.id) { continue; }
            let pc = (slot.note as i16 - root_note as i16).rem_euclid(12) as usize;
            self.additions |= 1 << pc;
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
        self.input_velocity = velocity.clamp(0.0, 1.0);
        self.sources[6] = self.input_velocity;
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
            self.addition_inputs[id as usize] = true;
            self.refresh_additions();
            self.configure_inner(self.host_config, false, out);
            self.rebuild_harmony(self.config.mode == CHORD, out);
            self.sync_layers(out);
            return;
        }
        if self.root.is_none() {
            self.addition_inputs[id as usize] = false;
            self.additions = 0;
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
            self.restart_arp();
        } else {
            self.second = Some(slot);
        }
        self.refresh_additions();
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
        self.addition_inputs[id as usize] = false;
        self.held_inputs[id as usize] = None;
        if self.config.latch || self.config.key_split {
            let old_additions = self.additions;
            self.refresh_additions();
            // Releases never change held harmony or restart strums/arp timing.
            // Retain a released alteration until a new input replaces it.
            for slot in [&mut self.root, &mut self.second].into_iter().flatten() {
                if slot.id == id {
                    slot.held = false;
                }
            }
            if self.root.is_none_or(|s| !s.held) && self.second.is_none_or(|s| !s.held)
                && !self.held_inputs.iter().flatten().any(|(s,_)| self.addition_inputs[s.id as usize]) {
                self.root = None;
                self.second = None;
            }
            if old_additions != self.additions { self.rebuild_harmony(self.config.mode == CHORD, out); }
            self.sync_layers(out);
            return;
        }
        let extra = self.root.is_some_and(|r| r.id != id) && self.second.is_none_or(|s| s.id != id);
        self.refresh_additions();
        self.release_harmony_input(id, velocity, out);
        if extra && self.root.is_some() { self.rebuild_harmony(self.config.mode == CHORD, out); }

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
                // Releasing a key may promote an already-held chord input, but
                // must not establish a new rhythmic origin.
                self.promoting = true;
                self.note_on(
                    slot.id,
                    slot.note,
                    slot.channel,
                    slot.velocity,
                    quality,
                    out,
                );
                self.promoting = false;
                return;
            }
        }
        if self.second.is_some_and(|s| s.id == id) {
            self.second = None;
            self.refresh_additions();
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
            if self.held_inputs.iter().flatten().count() >= 2 {
                // Keep the original register while several admitted contributors remain.
                self.rebuild_harmony(self.config.mode == CHORD, out);
                return;
            }
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
                self.additions = 0;
                self.refresh_additions();
                let (quality, alteration) = harmony::CONTROL_CHORDS[interval.min(11) as usize];
                self.quality = quality;
                self.control_alteration = alteration;
                if self.root.is_none() {
                    if let Some(memory) = self.memory.as_mut() {
                        memory.second = None;
                    }
                }
                self.rebuild_harmony(self.config.mode != ARP, out);
            }
            Command::SetQuality(quality) => {
                if self.quality != quality.min(11) || self.control_alteration.is_some() || self.additions != 0 {
                    self.additions = 0;
                    self.refresh_additions();
                    self.quality = quality.min(11);
                    self.control_alteration = None;
                    self.rebuild_harmony(self.config.mode != ARP, out);
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
                    self.saved = Some((index.min(MEMORY_COUNT - 1), chord.encode()));
                }
            }
            Command::RecallMemory(index, word) => {
                if harmony::SavedChord::decode(word).is_some() && index < MEMORY_COUNT {
                    self.command(Command::Recall(word), out);
                }
            }
            Command::Recall(word) => {
                if let Some(chord) = harmony::SavedChord::decode(word) {
                    self.scheduled.fill(None);
                    self.down.fill(false);
                    self.held_inputs.fill(None);
                    self.addition_inputs.fill(false);
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
                    self.additions = chord.additions;
                    self.quality = chord.quality;
                    self.control_alteration = None;
                    self.inversion = chord.inversion;
                    self.config.spread = chord.spread;
                    self.config.transpose = chord.transpose;
                    self.restart_arp();
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
                if self.manual_active() {
                    self.pluck_string(self.last_string.max(0) as usize, out);
                }
            }
            Command::Panic => self.panic(out),
            Command::Learn(target) => {
                self.learn = if matches!(target, 0..=2 | 4..=6) {
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
        self.resetting = true;
        self.reset_automatic_arp(out);
        self.reset_delay(out);
        self.learned_note_held.fill(false);
        self.stop_voices(0.0, out);
        self.stop_melody(out);
        self.stop_bass(out);
        self.scheduled.fill(None);
        self.root = None;
        self.second = None;
        self.additions = 0;
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
        self.addition_inputs.fill(false);
        self.pedal = false;
        self.x_primed = false;
        self.y_active = false;
        self.expression = Expression::default();
        self.channels.fill(Expression::default());
        self.input_bend_values.fill(0.5);
        self.resetting = false;
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
        let previous_notes = self.notes;
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
            additions: self.additions,
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
        let base = harmony::voice_recipe(recipe, spread);
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
        if self.config.affect_chords {
            self.full_notes = harmony::melody_harmony(self.full_notes, &self.melody);
            for i in 0..self.voices.len() {
                if self.voices[i]
                    .is_some_and(|v| self.melody.iter().any(|notes| notes[v.note as usize]))
                {
                    self.end_voice(i, 0.0, out);
                }
            }
        }
        self.notes = harmony::filter_notes(self.full_notes, self.config.filter);
        self.memory = Some(recipe);

        let count = harmony::recipe_intervals(recipe).len;
        self.inversion %= count.max(1) as u8;
        if play { self.scheduled.fill(None); }
        else {
            for event in &mut self.scheduled {
                if let Some(scheduled) = event.as_mut() {
                    if let Some(&note) = self.notes.as_slice().iter().find(|n| **n % 12 == scheduled.note % 12) {
                        let old = previous_notes.as_slice().iter().find(|n| **n % 12 == scheduled.note % 12).copied().unwrap_or(scheduled.note);
                        let remapped = scheduled.note as i16 + note as i16 - old as i16;
                        if (0..=127).contains(&remapped) { scheduled.note = remapped as u8; }
                        else { *event = None; }
                    } else { *event = None; }
                }
            }
        }
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
                for i in 0..self.voices.len() {
                    if self.voices[i].is_some_and(|v| v.owner==0 && !self.notes.as_slice().contains(&v.note)) {
                        let end = self.now.saturating_add(self.duration());
                        if let Some(voice) = self.voices[i].as_mut() {
                            voice.off = voice.off.min(end);
                        }
                    }
                }
                for i in 0..self.notes.len {
                    let note = self.notes.values[i];
                    if let Some(voice) = self.voices.iter_mut().flatten().find(|v| v.owner==0 && v.note == note) {
                        voice.off = u64::MAX;
                    } else {
                        self.strike(note, self.strike_velocity(), u64::MAX, out);
                    }
                }
            }
            AUTO => {
                if self.config.strum_hold {
                    for i in 0..self.voices.len() {
                        if self.voices[i].is_some_and(|v| v.owner==0 && !v.layer) {
                            self.end_voice(i, 0.0, out);
                        }
                    }
                }
                let count = self.string_count();
                let played = (self.config.strings_played as usize).clamp(1, count);
                let pattern = self.config.arp_pattern;
                let strikes = Self::pattern_length(pattern, played);
                let offset = if matches!(pattern, 1 | 4) {
                    count - played
                } else {
                    0
                };
                let base = self.rate_samples();
                let mut at = 0.0;
                for step in 0..strikes {
                    let string = offset + self.arp_index(pattern, step, played);
                    let ratio = step as f32 / (strikes - 1).max(1) as f32;
                    // Same per-strike RNG advance as the looping arp.
                    let _ = self.random();
                    let swing = if step.is_multiple_of(2) {
                        1.0 + self.config.swing as f64
                    } else {
                        1.0 - self.config.swing as f64
                    };
                    let spacing = base * swing;
                    let strike_at = at;
                    at += spacing;
                    let Some(note) = self.notes.string(string) else {
                        continue;
                    };
                    let strike_base = self.now.saturating_add(strike_at as u64);
                    let at = self.humanized_at(strike_base, spacing, self.now);
                    let human_vel = self.humanize_velocity_factor();
                    self.schedule(Scheduled {
                        gate_step: None,
                        at,
                        note,
                        velocity: (self.strike_velocity()
                            * contour_factor(
                                ratio,
                                self.config.contour,
                                self.config.contour_curve,
                            )
                            * human_vel)
                            .clamp(0.01, 1.0),
                        duration: if self.config.strum_hold {
                            u64::MAX
                        } else {
                            self.duration()
                        },
                    });
                }
            }

            MANUAL => {
                self.x_primed = false;
            }
            ARP => {}
            _ => {}
        }
        if self.config.root_on_select {
            if let Some(root) = self.selected_root() {
                for slot in &mut self.scheduled {
                    if slot.is_some_and(|e| e.note == root) {
                        *slot = None;
                    }
                }
                if !self
                    .voices
                    .iter()
                    .flatten()
                    .any(|v| v.note == root && v.owner == SELECT_ROOT_OWNER)
                {
                    self.strike_selection_root(root, out);
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
                        || self.addition_inputs[id as usize]
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
        let (arp_notes, arp_len) = arpeggiator::cycle_notes(
            self.arp_cycle_chord(), self.arp_live_settings(), self.arp.seed,
        );
        Snapshot {
            held_notes,
            sounding_notes,
            bass_note: (0..128)
                .find(|&note| self.bass.iter().any(|input| input[note].is_some()))
                .map(|note| note as u8),
            sources: self.effective_sources(),
            input_velocity: self.input_velocity,
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
            additions: self.additions,
            inversion: self.inversion,
            accepted,
            ignored,
            pressure: self.expression.pressure,
            timbre: self.expression.timbre,
            bend: self.expression.bend,
            x: self.x,
            y: self.y,
            strikes: self.strikes,
            note_strikes: self.note_strikes,
            voices: self.voices.iter().flatten().count() as u8,
            learning: self.learn,
            output_mpe: self.config.mpe,
            tempo: self.tempo as f32,
            arp_notes,
            arp_len: arp_len as u8,
            arp_page: self.arp.active,
            arp_pending: if self.config.arp_page != self.arp.active { self.config.arp_page } else { 255 },
            arp_step: self.arp.step,
            arp_phase: self.arp.display_phase,
            arp_running: self.arp.running && !self.arp.finished,
            arp_aligned: self.arp_alignment_available(),
            time_sig_host: self.time_sig_host,
            time_sig_num: self.time_sig_num,
            time_sig_den: self.time_sig_den,
        }
    }
    pub fn set_time_signature(&mut self, numerator: Option<i32>, denominator: Option<i32>) {
        self.time_sig_num = numerator.unwrap_or(4).clamp(1, 32) as u8;
        let denominator = denominator.unwrap_or(4);
        self.time_sig_den = if denominator > 0 {
            denominator.clamp(1, 32) as u8
        } else {
            4
        };
    }
    pub(super) fn selected_root(&self) -> Option<u8> {
        let root = self.root.map(|s| s.note).or(self.memory.map(|m| m.root))? as i16
            + self.config.transpose as i16;
        (0..=127).contains(&root).then_some(root as u8)
    }
    fn rate_samples(&self) -> f64 {
        if self.config.strum_sync {
            self.sample_rate as f64 * 60.0 / self.tempo * self.config.rate as f64
        } else {
            self.sample_rate as f64 * self.config.strum_ms as f64 / 1000.0
        }
    }
    /// Humanize timing: bipolar offset up to ±25% of `span` samples at 100%.
    /// Draws no randomness when humanize is off.
    fn humanize_timing(&mut self, span: f64) -> f64 {
        let h = self.config.humanize.clamp(0.0, 1.0) as f64;
        if h <= 0.0 {
            return 0.0;
        }
        let r = self.random() as f64 / u32::MAX as f64;
        span * 0.25 * h * (r * 2.0 - 1.0)
    }
    /// Humanize velocity: bipolar gain factor, 1 ± 30% at 100%.
    fn humanize_velocity_factor(&mut self) -> f32 {
        let h = self.config.humanize.clamp(0.0, 1.0);
        if h <= 0.0 {
            return 1.0;
        }
        let r = self.random() as f32 / u32::MAX as f32;
        1.0 + 0.3 * h * (r * 2.0 - 1.0)
    }
    /// `base` plus humanize jitter, never earlier than `floor`.
    fn humanized_at(&mut self, base: u64, span: f64, floor: u64) -> u64 {
        let off = self.humanize_timing(span);
        (base as i128 + off as i128).clamp(floor as i128, u64::MAX as i128) as u64
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
mod advance_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod arpeggiator_tests;
