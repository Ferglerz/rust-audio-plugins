use crate::harmony::Notes;
pub const POINTER_KEY_OFFSET: u8 = 64;
pub const KEY_TOKEN_COUNT: usize = 128;
pub const CHORD: u8 = 0;
pub const AUTO: u8 = 1;
pub const MANUAL: u8 = 2;
pub const ARP: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub filter: u8,
    pub split_channels: bool,
    pub bass_channel: u8,
    pub upper_channel: u8,
    pub mode: u8,
    pub quality: u8,
    pub inversion: u8,
    pub transpose: i8,
    pub spread: u8,
    pub latch: bool,
    pub strings: u8,
    pub velocity: f32,
    pub length_ms: f32,
    pub strum_ms: f32,
    pub strum_sync: bool,
    pub strum_beats: f32,
    pub direction: u8,
    pub contour: f32,
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
impl Default for Config {
    fn default() -> Self {
        Self {
            filter: 0,
            split_channels: false,
            bass_channel: 0,
            upper_channel: 1,
            mode: CHORD,
            quality: 0,
            inversion: 0,
            transpose: 0,
            spread: 0,
            latch: false,
            strings: 8,
            velocity: 0.8,
            length_ms: 350.0,
            strum_ms: 120.0,
            strum_sync: false,
            strum_beats: 0.25,
            direction: 0,
            contour: 0.0,
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
            mappings: [Mapping::cc(1, 16), Mapping::cc(11, 16)],
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
}
#[derive(Clone, Copy, Debug)]
pub struct Scheduled {
    pub at: u64,
    pub note: u8,
    pub velocity: f32,
    pub duration: u64,
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
    SetInversion(u8),
    Panic,
    Learn(u8),
    X(f32),
    Y(f32),
    EndGesture,
    BeginGesture(f32, f32),
    Capture(usize),
    Recall(u64),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snapshot {
    pub captured: u64,
    pub full_notes: Notes,
    pub notes: Notes,
    pub root: i16,
    pub second: i16,
    pub quality: u8,
    pub inversion: u8,
    pub accepted: u64,
    pub ignored: u64,
    pub pressure: f32,
    pub timbre: f32,
    pub bend: f32,
    pub x: f32,
    pub y: f32,
    pub strikes: [u32; 12],
    pub voices: u8,
    pub learning: u8,
    pub output_mpe: bool,
    pub tempo: f32,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            captured: 0,
            full_notes: Notes::default(),
            notes: Notes::default(),
            root: -1,
            second: -1,
            quality: 0,
            inversion: 0,
            accepted: 0,
            ignored: 0,
            pressure: 0.0,
            timbre: 0.5,
            bend: 0.5,
            x: 0.0,
            y: 0.8,
            strikes: [0; 12],
            voices: 0,
            learning: 0,
            output_mpe: false,
            tempo: 120.0,
        }
    }
}
