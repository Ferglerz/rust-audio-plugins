use super::routing::{Route, ROUTE_COUNT, SOURCE_COUNT, SOURCE_DEFAULTS, TARGET_COUNT};
use crate::harmony::Notes;
pub const POINTER_KEY_OFFSET: u8 = 64;
pub const KEY_TOKEN_COUNT: usize = 128;
pub const MEMORY_COUNT: usize = 10;
pub const CHORD: u8 = 0;
pub const AUTO: u8 = 1;
pub const MANUAL: u8 = 2;
pub const ARP: u8 = 3;

pub const VOICE_CAPACITY: usize = 48;
pub const KIND_ARP: u8 = 0;
pub const KIND_COMP: u8 = 1;
pub const KIND_BASS: u8 = 2;
pub const KIND_LEAD: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lane {
    Bass = 0,
    Comp = 1,
    Arp = 2,
    Lead = 3,
}

impl Lane {
    pub const ALL: [Lane; 4] = [Lane::Bass, Lane::Comp, Lane::Arp, Lane::Lead];
    pub const COUNT: usize = 4;

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Lane::Bass,
            1 => Lane::Comp,
            2 => Lane::Arp,
            3 => Lane::Lead,
            _ => Lane::Arp,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Lane::Bass => "Bass",
            Lane::Comp => "Comp",
            Lane::Arp => "Arp",
            Lane::Lead => "Lead",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LaneEvent {
    On {
        lane: Lane,
        channel: u8,
        note: u8,
        velocity: f32,
    },
    Off {
        lane: Lane,
        channel: u8,
        note: u8,
        velocity: f32,
    },
}

impl LaneEvent {
    pub fn lane(self) -> Lane {
        match self {
            LaneEvent::On { lane, .. } | LaneEvent::Off { lane, .. } => lane,
        }
    }

    pub fn channel(self) -> u8 {
        match self {
            LaneEvent::On { channel, .. } | LaneEvent::Off { channel, .. } => channel,
        }
    }

    pub fn note(self) -> u8 {
        match self {
            LaneEvent::On { note, .. } | LaneEvent::Off { note, .. } => note,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaneTapEvent {
    pub at: u64,
    pub event: LaneEvent,
}

pub struct LaneTap {
    pub events: Box<[Option<LaneTapEvent>; 1024]>,
    pub len: usize,
    pub overflow: bool,
}

impl Default for LaneTap {
    fn default() -> Self {
        Self {
            events: Box::new([None; 1024]),
            len: 0,
            overflow: false,
        }
    }
}

impl LaneTap {
    pub fn push(&mut self, at: u64, event: LaneEvent) {
        if self.len >= self.events.len() {
            self.overflow = true;
        } else {
            self.events[self.len] = Some(LaneTapEvent { at, event });
            self.len += 1;
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.overflow = false;
    }

    pub fn drain(&mut self) -> impl Iterator<Item = LaneEvent> + '_ {
        let count = self.len;
        self.len = 0;
        self.overflow = false;
        self.events[..count]
            .iter_mut()
            .filter_map(|opt| opt.take().map(|tap| tap.event))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub root_on_select: bool,
    pub routes: [Route; ROUTE_COUNT],
    pub filter: u8,
    pub bass_channel: u8,
    pub upper_channel: u8,
    pub mode: u8,
    pub quality: u8,
    pub voice_leading: u8,
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
    pub loop_start: u8,
    pub loop_end: u8,
    pub arp_volume: [u8; 32],
    pub arp_mute: u32,
    pub arp_skip: u32,
    pub arp_accents: [u8; 32],
    pub comp_mode: u8,
    pub comp_channel: u8,
    pub comp_octave: i8,
    pub comp_velocity: f32,
    pub comp_euclidean_steps: u8,
    pub comp_euclidean_hits: u8,
    pub comp_lag_ms: f32,
    pub comp_interlock: u8,
    pub bass_mode: u8,
    pub bass_octave: i8,
    pub bass_velocity: f32,
    pub routing_preset: u8,
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
    pub harmonization_mode: u8,
    pub chromatic_flavor: u8,
    pub key: u8,
    pub scale: u8,
    pub extensions: f32,
    pub legato_retention: bool,
    pub comp_guide_tone: bool,
    pub pad_swell_source: u8,
    pub lane_auto_mute: [bool; 4],
    pub bass_pad_trigger: u8,
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
    #[inline]
    pub fn is_string_muted(&self, index: usize) -> bool {
        index < 32 && (self.arp_mute & (1 << index)) != 0
    }
    #[inline]
    pub fn is_string_skipped(&self, index: usize) -> bool {
        index < 32 && (self.arp_skip & (1 << index)) != 0
    }
    #[inline]
    pub fn string_volume(&self, index: usize) -> f32 {
        if index < 32 {
            self.arp_volume[index] as f32 / 100.0
        } else {
            1.0
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            root_on_select: false,
            routes: super::routing::default_routes(),
            filter: 0,
            bass_channel: 0,
            upper_channel: 1,
            mode: CHORD,
            quality: 0,
            voice_leading: 2,
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
            loop_start: 1,
            loop_end: 0,
            arp_volume: [100; 32],
            arp_mute: 0,
            arp_skip: 0,
            arp_accents: [0; 32],
            comp_mode: 0,
            comp_channel: 0,
            comp_octave: 0,
            comp_velocity: 0.75,
            comp_euclidean_steps: 16,
            comp_euclidean_hits: 4,
            comp_lag_ms: 0.0,
            comp_interlock: 0,
            bass_mode: 0,
            bass_octave: -1,
            bass_velocity: 0.85,
            routing_preset: 0,
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
            harmonization_mode: 0,
            chromatic_flavor: 0,
            key: 0,
            scale: 0,
            extensions: 0.5,
            legato_retention: true,
            comp_guide_tone: false,
            pad_swell_source: 0,
            lane_auto_mute: [false; 4],
            bass_pad_trigger: 0,
        }
    }
}
impl Config {
    #[inline]
    pub fn step_accent(&self, step: usize) -> u8 {
        if step < 32 {
            self.arp_accents[step]
        } else {
            0
        }
    }
    #[inline]
    pub fn comp_is_hit(&self, step: usize) -> bool {
        let s = self.comp_euclidean_steps.clamp(1, 32) as usize;
        let k = self.comp_euclidean_hits.clamp(0, s as u8) as usize;
        if k == 0 {
            return false;
        }
        if k >= s {
            return true;
        }
        ((step % s) * k) % s < k
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
    pub kind: u8,
}
#[derive(Clone, Copy, Debug)]
pub struct Scheduled {
    pub at: u64,
    pub note: u8,
    pub velocity: f32,
    pub duration: u64,
    pub gate_step: Option<u64>,
    pub channel: u8,
    pub kind: u8,
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
    pub arp_cycle_step: usize,
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
            arp_cycle_step: 0,
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

pub type NoteMask = u128;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VoiceModule {
    Keys,
    Bass,
    Pad,
    Arp,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VoiceState {
    Inactive,
    Active,
    SustainedLegato,
    Releasing,
}

#[derive(Copy, Clone, Debug)]
pub struct RealtimeVoice {
    pub note_number: u8,
    pub target_bus: VoiceModule,
    pub state: VoiceState,
    pub current_gain: f32,
    pub target_gain: f32,
    pub velocity: u8,
    pub age_samples: u64,
}

impl Default for RealtimeVoice {
    fn default() -> Self {
        Self {
            note_number: 0,
            target_bus: VoiceModule::Keys,
            state: VoiceState::Inactive,
            current_gain: 0.0,
            target_gain: 0.0,
            velocity: 0,
            age_samples: 0,
        }
    }
}

pub const MAX_VOICES: usize = 32;

pub struct VoicePool {
    pub voices: [RealtimeVoice; MAX_VOICES],
    pub active_mask: NoteMask,
}

impl VoicePool {
    pub const fn new() -> Self {
        Self {
            voices: [RealtimeVoice {
                note_number: 0,
                target_bus: VoiceModule::Keys,
                state: VoiceState::Inactive,
                current_gain: 0.0,
                target_gain: 0.0,
                velocity: 0,
                age_samples: 0,
            }; MAX_VOICES],
            active_mask: 0,
        }
    }

    #[inline(always)]
    pub fn is_note_active(&self, note: u8) -> bool {
        (self.active_mask & (1u128 << note)) != 0
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TransitionPlan {
    pub notes_to_release: [u8; 16],
    pub release_count: usize,
    pub notes_to_attack: [u8; 16],
    pub attack_count: usize,
    pub notes_to_sustain: [u8; 16],
    pub sustain_count: usize,
}

#[inline]
pub fn plan_legato_transition(current: NoteMask, target: NoteMask) -> TransitionPlan {
    let mut plan = TransitionPlan {
        notes_to_release: [0; 16],
        release_count: 0,
        notes_to_attack: [0; 16],
        attack_count: 0,
        notes_to_sustain: [0; 16],
        sustain_count: 0,
    };

    let common_mask = current & target;
    let release_mask = current & !target;
    let attack_mask = target & !current;

    populate_plan_array(release_mask, &mut plan.notes_to_release, &mut plan.release_count);
    populate_plan_array(attack_mask, &mut plan.notes_to_attack, &mut plan.attack_count);
    populate_plan_array(common_mask, &mut plan.notes_to_sustain, &mut plan.sustain_count);

    plan
}

#[inline(always)]
fn populate_plan_array(mut mask: NoteMask, destination: &mut [u8; 16], count: &mut usize) {
    while mask != 0 && *count < 16 {
        let lsb_index = mask.trailing_zeros() as u8;
        destination[*count] = lsb_index;
        *count += 1;
        mask &= mask - 1;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtensionHysteresisTracker {
    pub current_tier: u8,
    pub smoothed_alpha: f32,
    pub filter_coeff: f32,
}

impl ExtensionHysteresisTracker {
    pub const HYSTERESIS_EPSILON: f32 = 0.05;

    pub fn new(sample_rate: f32) -> Self {
        let tau = 0.020;
        let filter_coeff = (-1.0 / (tau * sample_rate.max(1.0))).exp();
        Self {
            current_tier: 0,
            smoothed_alpha: 0.0,
            filter_coeff,
        }
    }

    #[inline]
    pub fn process_parameter(&mut self, raw_alpha: f32) -> (u8, f32) {
        self.smoothed_alpha = (raw_alpha * (1.0 - self.filter_coeff))
            + (self.smoothed_alpha * self.filter_coeff);

        let alpha = self.smoothed_alpha;
        let mut tier = self.current_tier;

        match tier {
            0 => {
                if alpha >= (1.0 + Self::HYSTERESIS_EPSILON) {
                    tier = 1;
                }
            }
            1 => {
                if alpha < (1.0 - Self::HYSTERESIS_EPSILON) {
                    tier = 0;
                } else if alpha >= (2.0 + Self::HYSTERESIS_EPSILON) {
                    tier = 2;
                }
            }
            2 => {
                if alpha < (2.0 - Self::HYSTERESIS_EPSILON) {
                    tier = 1;
                } else if alpha >= (3.0 + Self::HYSTERESIS_EPSILON) {
                    tier = 3;
                }
            }
            3 => {
                if alpha < (3.0 - Self::HYSTERESIS_EPSILON) {
                    tier = 2;
                } else if alpha >= (4.0 - Self::HYSTERESIS_EPSILON) {
                    tier = 4;
                }
            }
            4 => {
                if alpha < (4.0 - Self::HYSTERESIS_EPSILON) {
                    tier = 3;
                }
            }
            _ => tier = 0,
        }

        self.current_tier = tier;
        (tier, alpha)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PendingNote {
    pub channel: u8,
    pub note: u8,
    pub velocity: u8,
    pub timestamp_samples: u64,
}

impl Default for PendingNote {
    fn default() -> Self {
        Self {
            channel: 0,
            note: 0,
            velocity: 0,
            timestamp_samples: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookaheadBuffer {
    pub queue: [Option<PendingNote>; 8],
    pub count: usize,
    pub window_samples: u64,
}

impl LookaheadBuffer {
    pub fn new(sample_rate: f32) -> Self {
        let window_samples = (0.035 * sample_rate.max(1.0)) as u64;
        Self {
            queue: [None; 8],
            count: 0,
            window_samples,
        }
    }

    #[inline]
    pub fn ingest_zone_b(&mut self, channel: u8, note: u8, velocity: u8, current_time: u64) {
        let pending = PendingNote {
            channel,
            note,
            velocity,
            timestamp_samples: current_time,
        };
        // Update existing note if already present
        for slot in self.queue.iter_mut() {
            if let Some(p) = slot {
                if p.channel == channel && p.note == note {
                    *p = pending;
                    return;
                }
            }
        }
        // Insert into first empty slot
        for slot in self.queue.iter_mut() {
            if slot.is_none() {
                *slot = Some(pending);
                self.count += 1;
                return;
            }
        }
        // If buffer full (8 notes), overwrite oldest
        let mut oldest_idx = 0;
        let mut oldest_time = u64::MAX;
        for (i, slot) in self.queue.iter().enumerate() {
            if let Some(p) = slot {
                if p.timestamp_samples < oldest_time {
                    oldest_time = p.timestamp_samples;
                    oldest_idx = i;
                }
            }
        }
        self.queue[oldest_idx] = Some(pending);
    }

    /// Fast Staccato Slap Cancellation:
    /// If note-off arrives in Zone B before 35ms lookahead timer expires,
    /// cancel the pending note so it never rings orphaned.
    #[inline]
    pub fn cancel_note(&mut self, channel: u8, note: u8) -> bool {
        for slot in self.queue.iter_mut() {
            if let Some(p) = slot {
                if p.channel == channel && p.note == note {
                    *slot = None;
                    self.count = self.count.saturating_sub(1);
                    return true;
                }
            }
        }
        false
    }

    /// Drain all pending notes in arrival order when Zone A chord lands
    #[inline]
    pub fn drain_all(&mut self, out: &mut [PendingNote; 8]) -> usize {
        let mut n = 0;
        for slot in self.queue.iter_mut() {
            if let Some(p) = slot.take() {
                out[n] = p;
                n += 1;
            }
        }
        if n > 1 {
            out[..n].sort_unstable_by_key(|p| p.timestamp_samples);
        }
        self.count = 0;
        n
    }

    /// Drain expired notes whose 35ms window has elapsed without Zone A landing
    #[inline]
    pub fn drain_expired(&mut self, current_time: u64, out: &mut [PendingNote; 8]) -> usize {
        let mut n = 0;
        for slot in self.queue.iter_mut() {
            if let Some(p) = slot {
                if current_time.saturating_sub(p.timestamp_samples) >= self.window_samples {
                    out[n] = *p;
                    n += 1;
                    *slot = None;
                }
            }
        }
        if n > 0 {
            self.count = self.count.saturating_sub(n);
            if n > 1 {
                out[..n].sort_unstable_by_key(|p| p.timestamp_samples);
            }
        }
        n
    }

    #[inline]
    pub fn clear(&mut self) {
        self.queue = [None; 8];
        self.count = 0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DualCoreTopology {
    pub zone_a_min: u8,
    pub zone_a_max: u8,
    pub zone_b_min: u8,
    pub zone_b_max: u8,
}

impl DualCoreTopology {
    pub const KEYS_61: Self = Self {
        zone_a_min: 36, // C2
        zone_a_max: 59, // B3
        zone_b_min: 60, // C4
        zone_b_max: 84, // C6
    };

    pub const KEYS_49: Self = Self {
        zone_a_min: 36, // C2
        zone_a_max: 59, // B3
        zone_b_min: 60, // C4
        zone_b_max: 72, // C5
    };

    pub const KEYS_25: Self = Self {
        zone_a_min: 36, // C2
        zone_a_max: 47, // B2
        zone_b_min: 48, // C3
        zone_b_max: 60, // C4
    };

    pub const KEYS_88: Self = Self {
        zone_a_min: 36, // C2
        zone_a_max: 59, // B3
        zone_b_min: 60, // C4
        zone_b_max: 108, // C8
    };

    #[inline]
    pub fn is_zone_a(&self, note: u8) -> bool {
        note >= self.zone_a_min && note <= self.zone_a_max
    }

    #[inline]
    pub fn is_zone_b(&self, note: u8) -> bool {
        note >= self.zone_b_min && note <= self.zone_b_max
    }
}

