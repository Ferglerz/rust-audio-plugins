mod midi;
mod performance;
mod types;
use crate::harmony::{self, Notes};
pub use types::*;

pub struct Engine {
    pub host_config: Config,
    pub memory: Option<harmony::SavedChord>,
    pub recalled: bool,
    pub saved: Option<(usize, u64)>,
    pub full_notes: Notes,
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
    pub inversion: u8,
    pub pedal: bool,
    pub down: [bool; 2048 + KEY_TOKEN_COUNT],
    pub x: f32,
    pub y: f32,
    pub y_active: bool,
    pub gate_open: bool,
    pub x_primed: bool,
    pub last_string: i16,
    pub strikes: [u32; 12],
    pub learn: u8,
    pub learned: Option<(usize, Mapping)>,
    pub learned_base: Option<u8>,
    pub input_upper: bool,
    pub input_members: u8,
    pub input_bend: [f32; 16],
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
            host_config: Config::default(),
            memory: None,
            recalled: false,
            saved: None,
            full_notes: Notes::default(),
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
            inversion: 0,
            pedal: false,
            down: [false; 2048 + KEY_TOKEN_COUNT],
            x: 0.0,
            y: 0.8,
            y_active: false,
            gate_open: true,
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
        let old = self.host_config;
        self.host_config = config;
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
        if self.recalled {
            if let Some(memory) = self.memory {
                if old.spread == config.spread {
                    self.config.spread = memory.spread;
                }
                if old.transpose == config.transpose {
                    self.config.transpose = memory.transpose;
                }
            }
        }
        if old.quality != config.quality {
            self.quality = config.quality;
        }
        if old.inversion != config.inversion {
            self.inversion = config.inversion;
        }
        if !config.latch && !self.pedal && self.root.is_none() {
            self.notes = Notes::default();
            self.stop_voices(0.0, out);
            self.scheduled.fill(None);
        }
        if old.control_base != config.control_base {
            self.panic(out);
        }
        if old.mappings != config.mappings {
            self.x_primed = false;
            self.gate_open = config.mappings[2].kind == 0;
        }
        let revoice = old.quality != config.quality
            || old.inversion != config.inversion
            || old.transpose != config.transpose
            || old.spread != config.spread
            || old.filter != config.filter;
        if revoice {
            self.rebuild(out);
        } else if route || mode {
            self.start_mode(out);
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
        if self.root.is_some() && self.second.is_some() {
            return;
        }
        let slot = Slot {
            id,
            note,
            channel,
            held: true,
            velocity,
        };
        if self.root.is_none() {
            self.stop_voices(0.0, out);
            self.scheduled.fill(None);
            self.root = Some(slot);
            self.quality = quality.unwrap_or(self.quality);
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
        self.rebuild(out);
    }
    pub fn note_off(&mut self, id: u16, velocity: f32, out: &mut impl FnMut(Out)) {
        if id as usize >= self.down.len() {
            return;
        }
        self.down[id as usize] = false;
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
                self.stop_voices(velocity, out);
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
                if had_keyboard && self.root.is_none() {
                    self.notes = Notes::default();
                    self.stop_voices(0.0, out);
                    self.scheduled.fill(None);
                }
            }
            Command::SetQuality(quality) => {
                if self.quality != quality.min(11) {
                    self.quality = quality.min(11);
                    self.rebuild(out);
                }
            }
            Command::SetInversion(inversion) => {
                let value = inversion % self.full_notes.len.max(1) as u8;
                if self.inversion != value {
                    self.inversion = value;
                    self.rebuild(out);
                }
            }
            Command::Inversion(step) => {
                let n = self.full_notes.len.max(1) as i16;
                self.inversion = (self.inversion as i16 + step as i16).rem_euclid(n) as u8;
                self.rebuild(out);
            }
            Command::Capture(index) => {
                if let Some(chord) = self.memory {
                    self.saved = Some((index.min(7), chord.encode()));
                }
            }
            Command::Recall(word) => {
                if let Some(chord) = harmony::SavedChord::decode(word) {
                    self.stop_voices(0.0, out);
                    self.scheduled.fill(None);
                    self.down.fill(false);
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
                    self.inversion = chord.inversion;
                    self.config.spread = chord.spread;
                    self.config.transpose = chord.transpose;
                    self.recalled = true;
                    self.rebuild(out);
                }
            }
            Command::BeginGesture(x, y) => {
                self.position(y, 1, out);
                self.x_primed = false;
                self.position(x, 0, out);
                if self.config.mode == MANUAL && self.gate_open {
                    self.pluck_string(self.last_string.max(0) as usize, out);
                }
            }
            Command::Panic => self.panic(out),
            Command::Learn(target) => self.learn = target,
            Command::X(x) => self.position(x, 0, out),
            Command::Y(y) => self.position(y, 1, out),
            Command::Gate(g) => {
                self.gate_open = g;
                self.x_primed = false;
            }
            Command::EndGesture => {
                self.y_active = false;
                self.fan_expression(out);
            }
        }
    }
    pub fn panic(&mut self, out: &mut impl FnMut(Out)) {
        self.stop_voices(0.0, out);
        self.scheduled.fill(None);
        self.root = None;
        self.second = None;
        self.notes = Notes::default();
        self.full_notes = Notes::default();
        self.memory = None;
        self.recalled = false;
        self.down.fill(false);
        self.pedal = false;
        self.x_primed = false;
        self.y_active = false;
        self.expression = Expression::default();
        self.channels.fill(Expression::default());
        self.needs_setup = true;
        let master = self.master();
        out(Out::Cc(master, 64, 0.0));
        out(Out::Bend(master, 0.5));
        out(Out::Pressure(master, 0.0));
        out(Out::Cc(master, 74, 0.5));
    }
    fn rebuild(&mut self, out: &mut impl FnMut(Out)) {
        let Some(root_note) = self.root.map(|s| s.note).or(self.memory.map(|m| m.root)) else {
            return;
        };
        let second = if self.root.is_some() {
            self.second.map(|s| s.note)
        } else {
            self.memory.and_then(|m| m.second)
        };
        self.full_notes = harmony::voice(
            root_note,
            self.quality,
            second,
            self.config.transpose,
            self.inversion,
            self.config.spread,
        );
        self.notes = harmony::filter_notes(self.full_notes, self.config.filter);
        self.memory = Some(harmony::SavedChord {
            root: root_note,
            second,
            quality: self.quality,
            inversion: self.inversion,
            spread: self.config.spread,
            transpose: self.config.transpose,
        });
        let count = harmony::intervals(
            self.quality,
            second.map(|n| (n as i16 - root_note as i16).rem_euclid(12) as u8),
        )
        .len;
        self.inversion %= count.max(1) as u8;
        self.scheduled.fill(None);
        self.start_mode(out);
    }
    fn start_mode(&mut self, out: &mut impl FnMut(Out)) {
        if self.notes.len == 0 {
            return;
        }
        match self.config.mode {
            CHORD => {
                for i in 0..15 {
                    if self.voices[i].is_some_and(|v| !self.notes.as_slice().contains(&v.note)) {
                        self.end_voice(i, 0.0, out);
                    }
                }
                for i in 0..self.notes.len {
                    let note = self.notes.values[i];
                    if !self.voices.iter().flatten().any(|v| v.note == note) {
                        self.strike(note, self.strike_velocity(), u64::MAX, out);
                    }
                }
            }
            AUTO => {
                self.stop_voices(0.0, out);
                let reverse =
                    self.config.direction == 1 || (self.config.direction == 2 && self.auto_reverse);
                self.auto_reverse = !self.auto_reverse;
                for i in 0..self.notes.len {
                    let idx = if reverse { self.notes.len - 1 - i } else { i };
                    let ratio = i as f32 / (self.notes.len - 1).max(1) as f32;
                    self.schedule(Scheduled {
                        at: self.now + self.ms(self.config.strum_ms * ratio),
                        note: self.notes.values[idx],
                        velocity: (self.strike_velocity()
                            * (1.0 + self.config.contour * (ratio - 0.5)))
                            .clamp(0.01, 1.0),
                        duration: self.duration(),
                    });
                }
            }
            MANUAL => {
                self.stop_voices(0.0, out);
                self.x_primed = false;
            }
            ARP => {
                self.stop_voices(0.0, out);
                self.arp_next = self.now;
            }
            _ => {}
        }
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
        Snapshot {
            captured: self.memory.map_or(0, |m| m.encode()),
            full_notes: self.full_notes,
            notes: self.notes,
            root: self.root.map_or(-1, |s| s.note as i16),
            second: self.second.map_or(-1, |s| s.note as i16),
            quality: self.quality,
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
