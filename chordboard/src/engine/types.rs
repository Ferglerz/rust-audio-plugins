use super::routing::{Route, ROUTE_COUNT, SOURCE_COUNT, SOURCE_DEFAULTS, TARGET_COUNT};
use crate::harmony::Notes;
pub const POINTER_KEY_OFFSET: u8 = 64;
pub const KEY_TOKEN_COUNT: usize = 128;
pub const MEMORY_COUNT: usize = 10;
pub const CHORD: u8 = 0;
pub const AUTO: u8 = 1;
pub const MANUAL: u8 = 2;
pub const ARP: u8 = 3;
pub const SEQUENCER: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub strum_enabled: bool,
    pub root_on_select: bool,
    pub routes: [Route; ROUTE_COUNT],
    pub filter: u8,
    pub bass_channel: u8,
    pub upper_channel: u8,
    pub mode: u8,
    pub seq_pages: [u8; 4],
    pub quality: u8,
    pub voice_leading: u8,
    /// Arp strikes that land on a chord strike: 0 off, 1 avoid chords, 2 match chords.
    pub interlock: u8,
    pub inversion: u8,
    pub transpose: i8,
    pub spread: u8,
    pub latch: bool,
    pub always_bass: bool,
    pub always_chord: bool,
    pub affect_chords: bool,
    pub key_split: bool,
    pub split_note: u8,
    pub bass_split: i16,
    pub bass_enabled: bool,
    pub strings: u8,
    pub strings_played: u8,
    pub routed_x: Option<f32>,
    pub legacy_direct_x: bool,
    pub velocity: f32,
    pub length_ms: f32,
    pub strum_ms: f32,
    pub strum_sync: bool,
    pub strum_beats: f32,
    pub strum_hold: bool,
    pub direction: u8,
    pub contour: f32,
    pub contour_curve: f32,
    pub arp_pattern: u8,
    pub rate: f32,
    pub gate: f32,
    pub swing: f32,
    pub octaves: u8,
    pub humanize: f32,
    pub mpe: bool,
    pub upper: bool,
    pub members: u8,
    pub bend_range: f32,
    pub master_range: f32,
    pub output_channel: u8,
    pub control_base: i16,
    pub x_reverse: bool,
    pub y_reverse: bool,
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
    pub y_target: u8,
    pub y_cc: u8,
    pub mappings: [Mapping; 2],
}
impl Config {
    pub fn bass_boundary(&self) -> i16 {
        if !self.bass_enabled {
            return -1;
        }
        if self.bass_split >= 0 {
            self.bass_split
        } else {
            self.control_base
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            strum_enabled: true,
            root_on_select: false,
            routes: super::routing::default_routes(),
            filter: 0,
            bass_channel: 0,
            upper_channel: 1,
            mode: CHORD,
            seq_pages: [0; 4],
            quality: 0,
            voice_leading: 2,
            interlock: 0,
            inversion: 0,
            transpose: 0,
            spread: 0,
            latch: false,
            always_bass: false,
            always_chord: false,
            affect_chords: false,
            key_split: false,
            split_note: 60,
            bass_split: -1,
            bass_enabled: true,
            strings: 8,
            strings_played: 12,
            routed_x: None,
            legacy_direct_x: false,
            velocity: 0.8,
            length_ms: 350.0,
            strum_ms: 125.0,
            strum_sync: true,
            strum_beats: 0.25,
            strum_hold: false,
            direction: 0,
            contour: 0.0,
            contour_curve: 0.0,
            arp_pattern: 0,
            rate: 0.25,
            gate: 0.65,
            swing: 0.0,
            octaves: 1,
            humanize: 0.0,
            mpe: false,
            upper: false,
            members: 15,
            bend_range: 48.0,
            master_range: 2.0,
            output_channel: 0,
            control_base: -1,
            x_reverse: false,
            y_reverse: false,
            x_min: 0.0,
            x_max: 1.0,
            y_min: 0.0,
            y_max: 1.0,
            y_target: 0,
            y_cc: 11,
            mappings: [Mapping::cc(1, 16), Mapping::default()],
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mapping {
    pub kind: u8,
    pub number: u8,
    pub channel: u8,
}
impl Mapping {
    pub const fn cc(number: u8, channel: u8) -> Self {
        Self {
            kind: 1,
            number,
            channel,
        }
    }
    pub fn encode(self) -> u32 {
        self.kind as u32 | (self.number as u32) << 8 | (self.channel as u32) << 16
    }
    pub fn decode(n: u32) -> Self {
        Self {
            kind: (n & 255).min(3) as u8,
            number: ((n >> 8) & 127) as u8,
            channel: ((n >> 16) & 255).min(16) as u8,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Expression {
    pub pressure: f32,
    pub timbre: f32,
    pub bend: f32,
}
impl Default for Expression {
    fn default() -> Self {
        Self {
            pressure: 0.0,
            timbre: 0.5,
            bend: 0.5,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub id: u16,
    pub note: u8,
    pub channel: u8,
    pub held: bool,
    pub velocity: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Voice {
    pub note: u8,
    pub channel: u8,
    pub started: u64,
    pub off: u64,
    pub layer: bool,
    pub owner: u8,
    pub seq_expression: Option<super::sequencer::StepExpression>,
}
#[derive(Clone, Copy, Debug)]
pub struct Scheduled {
    pub at: u64,
    pub note: u8,
    pub velocity: f32,
    pub duration: u64,
    pub gate_step: Option<u64>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Out {
    On(u8, u8, f32),
    Off(u8, u8, f32),
    Pressure(u8, f32),
    Bend(u8, f32),
    Cc(u8, u8, f32),
}
#[derive(Clone, Copy, Debug)]
pub enum Command {
    KeyDown(u8, u8, u8),
    KeyUp(u8),
    ReleaseKeyboard,
    Inversion(i8),
    SetQuality(u8),
    SetControlChord(u8),
    SetInversion(u8),
    Panic,
    Learn(u8),
    X(f32),
    Y(f32),
    EndGesture,
    BeginGesture(f32, f32),
    Capture(usize),
    Recall(u64),
    RecallMemory(usize, u64),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snapshot {
    pub held_notes: [bool; 128],
    pub sounding_notes: [bool; 128],
    /// Lowest note owned by the independent bass layer, including sustained notes.
    pub bass_note: Option<u8>,
    pub sources: [f32; SOURCE_COUNT],
    pub input_velocity: f32,
    pub routed: [Option<f32>; TARGET_COUNT],
    pub captured: u64,
    pub full_notes: Notes,
    pub leading_from: Notes,
    pub leading_to: Notes,
    pub leading_serial: u64,
    pub notes: Notes,
    pub root: i16,
    pub second: i16,
    pub quality: u8,
    pub control_alteration: Option<u8>,
    pub inversion: u8,
    pub accepted: u64,
    pub ignored: u64,
    pub pressure: f32,
    pub timbre: f32,
    pub bend: f32,
    pub x: f32,
    pub y: f32,
    pub strikes: [u32; 12],
    pub note_strikes: [u32; 128],
    pub voices: u8,
    pub learning: u8,
    pub output_mpe: bool,
    pub tempo: f32,
    pub seq_pages: [u8; 4],
    pub seq_base: [u8; 4],
    pub seq_pending: [u8; 4],
    pub seq_steps: [u8; 4],
    pub seq_running: bool,
    pub time_sig_num: u8,
    pub time_sig_den: u8,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            held_notes: [false; 128],
            sounding_notes: [false; 128],
            bass_note: None,
            sources: SOURCE_DEFAULTS,
            input_velocity: 0.8,
            routed: [None; TARGET_COUNT],
            captured: 0,
            full_notes: Notes::default(),
            leading_from: Notes::default(),
            leading_to: Notes::default(),
            leading_serial: 0,
            notes: Notes::default(),
            root: -1,
            second: -1,
            quality: 0,
            control_alteration: None,
            inversion: 0,
            accepted: 0,
            ignored: 0,
            pressure: 0.0,
            timbre: 0.5,
            bend: 0.5,
            x: 0.0,
            y: 0.8,
            strikes: [0; 12],
            note_strikes: [0; 128],
            voices: 0,
            learning: 0,
            output_mpe: false,
            tempo: 120.0,
            seq_pages: [0; 4],
            seq_base: [0; 4],
            seq_pending: [0; 4],
            seq_steps: [0; 4],
            seq_running: false,
            time_sig_num: 4,
            time_sig_den: 4,
        }
    }
}

// Quadratic bend: fixed endpoints, monotonic throughout the supported range.
pub fn contour_factor(position: f32, tilt: f32, curve: f32) -> f32 {
    let t = position.clamp(0.0, 1.0);
    1.0 + tilt * (t - 0.5) + tilt.abs() * curve * t * (1.0 - t)
}

/// Keep one playable chord key between the bass and melody regions.
pub fn ordered_splits(a: u8, b: u8) -> (u8, u8) {
    let bass = a.min(b).min(126);
    (bass, a.max(b).max(bass + 1).min(127))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LearnedSplit {
    pub target: u8,
    pub bass: u8,
    pub melody: u8,
}
impl LearnedSplit {
    pub fn encode(self) -> u32 {
        ((self.target as u32) << 16) | ((self.bass as u32) << 8) | self.melody as u32
    }
    pub fn decode(value: u32) -> Option<Self> {
        let target = (value >> 16) as u8;
        let bass = ((value >> 8) & 255) as u8;
        let melody = (value & 255) as u8;
        (matches!(target, 5 | 6) && bass < melody && melody < 128).then_some(Self {
            target,
            bass,
            melody,
        })
    }
    pub fn apply(self, config: &mut Config) {
        config.bass_split = self.bass as i16;
        config.split_note = self.melody;
        if self.target == 5 {
            config.bass_enabled = true;
        } else {
            config.key_split = true;
        }
    }
}
