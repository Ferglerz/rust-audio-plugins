//! Global MIDI effects, independent of tracker rows.
use serde::{Deserialize, Serialize};

pub const TAPS: usize = 8;
pub mod source {
    pub const ARP: u8 = 1;
    pub const STRUM: u8 = 2;
    pub const CHORDS: u8 = 4;
    pub const BASS: u8 = 8;
    pub const MELODY: u8 = 16;
    pub const ALL: u8 = ARP | STRUM | CHORDS | BASS | MELODY;
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DelayTap {
    pub enabled: bool,
    /// Beat interval in sync mode, milliseconds otherwise.
    pub time: f32,
    pub velocity: f32,
    pub feedback: f32,
    pub pitch: i8,
    pub fine: f32,
    /// Bipolar MIDI pan. Ping pong replaces this with alternating hard pan.
    pub pan: f32,
    /// Zero = automatic, 1..16 = explicit MIDI channel.
    pub channel: u8,
}
impl Default for DelayTap {
    fn default() -> Self {
        Self {
            enabled: true,
            time: 0.5,
            velocity: 0.75,
            feedback: 0.35,
            pitch: 0,
            fine: 0.0,
            pan: 0.0,
            channel: 0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DelaySettings {
    pub enabled: bool,
    pub sync: bool,
    pub ping_pong: bool,
    /// Join the main voice-leading mode, including sounding echo references.
    pub voice_leading: bool,
    /// Resolve each new echo against the chord active at playback time.
    pub follow_chords: bool,
    pub sources: u8,
    pub count: u8,
    pub taps: [DelayTap; TAPS],
}
impl Default for DelaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            sync: true,
            ping_pong: false,
            voice_leading: false,
            follow_chords: false,
            sources: source::ARP,
            count: 1,
            taps: [DelayTap::default(); TAPS],
        }
    }
}
impl DelaySettings {
    pub fn sanitize(&mut self) {
        self.count = self.count.clamp(1, TAPS as u8);
        self.sources &= 31;
        for tap in &mut self.taps {
            tap.time = finite(tap.time, if self.sync { 0.5 } else { 250.0 }).clamp(
                if self.sync { 1.0 / 64.0 } else { 1.0 },
                if self.sync { 16.0 } else { 8000.0 },
            );
            tap.velocity = finite(tap.velocity, 0.75).clamp(0.0, 2.0);
            tap.feedback = finite(tap.feedback, 0.35).clamp(0.0, 0.95);
            tap.pitch = tap.pitch.clamp(-24, 24);
            tap.fine = finite(tap.fine, 0.0).clamp(-100.0, 100.0);
            tap.pan = finite(tap.pan, 0.0).clamp(-1.0, 1.0);
            tap.channel = tap.channel.min(16);
        }
    }
}
use crate::finite;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HarmonizerMode {
    Fixed,
    #[default]
    Scale,
    Chord,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HarmonyVoice {
    pub enabled: bool,
    /// Semitones in Fixed mode, signed scale/chord-tone steps otherwise.
    pub interval: i8,
    pub velocity: f32,
}
impl Default for HarmonyVoice {
    fn default() -> Self {
        Self {
            enabled: false,
            interval: 0,
            velocity: 0.8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HarmonizerSettings {
    pub enabled: bool,
    pub sources: u8,
    pub original: bool,
    pub mode: HarmonizerMode,
    pub voices: [HarmonyVoice; 3],
}
impl Default for HarmonizerSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            sources: source::MELODY,
            original: true,
            mode: HarmonizerMode::Scale,
            voices: [
                HarmonyVoice {
                    enabled: true,
                    interval: 2,
                    velocity: 0.8,
                },
                HarmonyVoice {
                    enabled: true,
                    interval: 4,
                    velocity: 0.6,
                },
                HarmonyVoice::default(),
            ],
        }
    }
}
impl HarmonizerSettings {
    pub fn sanitize(&mut self) {
        self.sources &= source::ALL;
        for voice in &mut self.voices {
            voice.interval = voice.interval.clamp(-24, 24);
            voice.velocity = finite(voice.velocity, 0.8).clamp(0.0, 2.0);
        }
    }
}

pub const GATE_STEPS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RhythmGateSettings {
    pub enabled: bool,
    pub sources: u8,
    pub steps: u8,
    /// Zero is a rest; positive values multiply the input velocity.
    pub pattern: [f32; GATE_STEPS],
    /// Quarter-note beats per step.
    pub rate: f32,
    /// Fraction of one step for which each pulse sounds.
    pub gate: f32,
    /// Euclidean fill controls do not alter the pattern until Fill is invoked.
    pub pulses: u8,
    pub rotation: u8,
}
impl Default for RhythmGateSettings {
    fn default() -> Self {
        let mut settings = Self {
            enabled: false,
            sources: source::CHORDS,
            steps: 16,
            pattern: [0.0; GATE_STEPS],
            rate: 0.25,
            gate: 0.5,
            pulses: 4,
            rotation: 0,
        };
        settings.fill_euclidean();
        settings
    }
}
impl RhythmGateSettings {
    pub fn sanitize(&mut self) {
        self.sources &= source::ALL;
        self.steps = self.steps.clamp(1, GATE_STEPS as u8);
        self.pulses = self.pulses.min(self.steps);
        self.rotation %= self.steps;
        self.rate = finite(self.rate, 0.25).clamp(1.0 / 64.0, 16.0);
        self.gate = finite(self.gate, 0.5).clamp(0.01, 1.0);
        for velocity in &mut self.pattern {
            *velocity = finite(*velocity, 0.0).clamp(0.0, 2.0);
        }
    }

    /// Distribute pulses evenly, beginning at step zero before rotation.
    pub fn fill_euclidean(&mut self) {
        self.sanitize();
        self.pattern.fill(0.0);
        let steps = self.steps as usize;
        let pulses = self.pulses as usize;
        for step in 0..steps {
            if (step * pulses) % steps < pulses {
                self.pattern[(step + self.rotation as usize) % steps] = 1.0;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VoiceSplitterSettings {
    pub enabled: bool,
    pub sources: u8,
    /// One-based standard MIDI channels for Low, Inner and High.
    pub channels: [u8; 3],
    /// Member-channel counts for Low, Inner and High; remaining members are dry.
    pub pools: [u8; 3],
}
impl Default for VoiceSplitterSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            sources: source::CHORDS,
            channels: [2, 3, 4],
            pools: [2, 6, 3],
        }
    }
}
impl VoiceSplitterSettings {
    pub fn sanitize(&mut self) {
        self.sources &= source::ALL;
        for channel in &mut self.channels {
            *channel = (*channel).clamp(1, 16);
        }
        for size in &mut self.pools {
            *size = (*size).clamp(1, 15);
        }
    }

    /// Zero-based channel masks for Low, Inner, High and unselected notes.
    /// Invalid layouts remain visible in settings, but cannot allocate voices.
    pub fn pool_ranges(&self, members: u8, upper: bool) -> Option<[u16; 4]> {
        if !(4..=15).contains(&members) || self.pools.contains(&0) {
            return None;
        }
        let assigned: u16 = self.pools.iter().map(|&size| u16::from(size)).sum();
        if assigned >= u16::from(members) {
            return None;
        }
        let sizes = [
            self.pools[0],
            self.pools[1],
            self.pools[2],
            members - assigned as u8,
        ];
        let mut ranges = [0; 4];
        let mut ordinal = 0;
        for (mask, size) in ranges.iter_mut().zip(sizes) {
            for _ in 0..size {
                let channel = if upper { 14 - ordinal } else { 1 + ordinal };
                *mask |= 1 << channel;
                ordinal += 1;
            }
        }
        Some(ranges)
    }
}

/// This order is both the module display order and the processing order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MidiFxId {
    Harmonizer,
    RhythmGate,
    VoiceSplitter,
    Delay,
}

pub const EFFECTS: [MidiFxId; 4] = [
    MidiFxId::Harmonizer,
    MidiFxId::RhythmGate,
    MidiFxId::VoiceSplitter,
    MidiFxId::Delay,
];

fn legacy_order() -> Vec<MidiFxId> {
    vec![MidiFxId::Delay]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MidiFxState {
    pub delay: DelaySettings,
    pub harmonizer: HarmonizerSettings,
    pub rhythm_gate: RhythmGateSettings,
    pub voice_splitter: VoiceSplitterSettings,
    #[serde(default = "legacy_order")]
    pub order: Vec<MidiFxId>,
}
impl Default for MidiFxState {
    fn default() -> Self {
        Self {
            delay: DelaySettings::default(),
            harmonizer: HarmonizerSettings::default(),
            rhythm_gate: RhythmGateSettings::default(),
            voice_splitter: VoiceSplitterSettings::default(),
            order: EFFECTS.to_vec(),
        }
    }
}
impl MidiFxState {
    pub fn sanitize(&mut self) {
        self.delay.sanitize();
        self.harmonizer.sanitize();
        self.rhythm_gate.sanitize();
        self.voice_splitter.sanitize();
        // Exactly one instance of each supported effect. Keep valid saved order.
        let mut seen = Vec::new();
        self.order.retain(|id| {
            if seen.contains(id) {
                false
            } else {
                seen.push(*id);
                true
            }
        });
        for id in EFFECTS {
            if !self.order.contains(&id) {
                self.order.push(id);
            }
        }
    }
    pub fn move_effect(&mut self, from: usize, to: usize) {
        move_item(&mut self.order, from, to);
    }

    pub fn sources(&self, id: MidiFxId) -> u8 {
        match id {
            MidiFxId::Harmonizer => self.harmonizer.sources,
            MidiFxId::RhythmGate => self.rhythm_gate.sources,
            MidiFxId::VoiceSplitter => self.voice_splitter.sources,
            MidiFxId::Delay => self.delay.sources,
        }
    }

    pub fn set_sources(&mut self, id: MidiFxId, sources: u8) {
        let sources = sources & source::ALL;
        match id {
            MidiFxId::Harmonizer => self.harmonizer.sources = sources,
            MidiFxId::RhythmGate => self.rhythm_gate.sources = sources,
            MidiFxId::VoiceSplitter => self.voice_splitter.sources = sources,
            MidiFxId::Delay => self.delay.sources = sources,
        }
    }

    pub fn enabled(&self, id: MidiFxId) -> bool {
        match id {
            MidiFxId::Harmonizer => self.harmonizer.enabled,
            MidiFxId::RhythmGate => self.rhythm_gate.enabled,
            MidiFxId::VoiceSplitter => self.voice_splitter.enabled,
            MidiFxId::Delay => self.delay.enabled,
        }
    }

    pub fn set_enabled(&mut self, id: MidiFxId, enabled: bool) {
        match id {
            MidiFxId::Harmonizer => self.harmonizer.enabled = enabled,
            MidiFxId::RhythmGate => self.rhythm_gate.enabled = enabled,
            MidiFxId::VoiceSplitter => self.voice_splitter.enabled = enabled,
            MidiFxId::Delay => self.delay.enabled = enabled,
        }
    }
}

pub(crate) fn move_item<T>(items: &mut Vec<T>, from: usize, to: usize) {
    if from < items.len() && to < items.len() && from != to {
        let item = items.remove(from);
        items.insert(to, item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_fx_legacy_delay_is_preserved_and_new_modules_are_bypassed() {
        let mut state: MidiFxState = serde_json::from_str(
            r#"{"delay":{"enabled":true,"sources":24},"order":["Delay","Delay"]}"#,
        )
        .unwrap();
        state.sanitize();
        assert_eq!(
            state.order,
            vec![
                MidiFxId::Delay,
                MidiFxId::Harmonizer,
                MidiFxId::RhythmGate,
                MidiFxId::VoiceSplitter
            ]
        );
        assert!(state.delay.enabled);
        assert_eq!(state.delay.sources, source::BASS | source::MELODY);
        assert!(
            !state.harmonizer.enabled
                && !state.rhythm_gate.enabled
                && !state.voice_splitter.enabled
        );
        assert_eq!(state.harmonizer.sources, source::MELODY);
        assert_eq!(state.rhythm_gate.sources, source::CHORDS);
        assert_eq!(state.voice_splitter.sources, source::CHORDS);
        let mut missing: MidiFxState = serde_json::from_str("{}").unwrap();
        missing.sanitize();
        assert_eq!(missing.order, state.order);
        assert_eq!(MidiFxState::default().order, EFFECTS);
        let restored: MidiFxState =
            serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        assert_eq!(restored, state);
    }

    #[test]
    fn midi_fx_sanitize_bounds_values_without_replacing_manual_patterns() {
        let mut state = MidiFxState::default();
        state.harmonizer.voices[0].velocity = f32::NAN;
        state.harmonizer.voices[1].interval = i8::MIN;
        state.rhythm_gate.rate = f32::INFINITY;
        state.rhythm_gate.gate = f32::NAN;
        state.rhythm_gate.steps = 0;
        state.rhythm_gate.rotation = 255;
        state.rhythm_gate.pattern[0] = 0.37;
        state.rhythm_gate.pattern[1] = f32::NEG_INFINITY;
        state.voice_splitter.channels = [0, 255, 4];
        state.voice_splitter.pools = [0, 255, 3];
        state.sanitize();
        assert_eq!(state.harmonizer.voices[0].velocity, 0.8);
        assert_eq!(state.harmonizer.voices[1].interval, -24);
        assert_eq!(state.rhythm_gate.rate, 0.25);
        assert_eq!(state.rhythm_gate.gate, 0.5);
        assert_eq!(state.rhythm_gate.steps, 1);
        assert_eq!(state.rhythm_gate.rotation, 0);
        assert_eq!(state.rhythm_gate.pattern[0], 0.37);
        assert_eq!(state.rhythm_gate.pattern[1], 0.0);
        assert_eq!(state.voice_splitter.channels, [1, 16, 4]);
        assert!(state.voice_splitter.pool_ranges(15, false).is_none());
        for id in EFFECTS {
            state.set_sources(id, 255);
            assert_eq!(state.sources(id), source::ALL);
            state.set_sources(id, 0);
            state.set_enabled(id, true);
            assert_eq!(state.sources(id), 0);
            assert!(state.enabled(id));
        }
    }

    #[test]
    fn midi_fx_euclidean_fill_has_exact_pulse_count_and_rotation() {
        let mut gate = RhythmGateSettings::default();
        let active: Vec<_> = gate
            .pattern
            .iter()
            .enumerate()
            .filter_map(|(i, v)| (*v > 0.0).then_some(i))
            .collect();
        assert_eq!(active, [0, 4, 8, 12]);
        for steps in 1..=32 {
            for pulses in 0..=steps {
                gate.steps = steps;
                gate.pulses = pulses;
                gate.rotation = steps - 1;
                gate.fill_euclidean();
                assert_eq!(
                    gate.pattern.iter().filter(|&&v| v > 0.0).count(),
                    pulses as usize
                );
                assert!(gate.pattern[steps as usize..].iter().all(|&v| v == 0.0));
            }
        }
        gate.steps = 8;
        gate.pulses = 2;
        gate.rotation = 1;
        gate.fill_euclidean();
        assert_eq!(gate.pattern[1], 1.0);
        assert_eq!(gate.pattern[5], 1.0);
    }

    #[test]
    fn midi_fx_mpe_pools_are_disjoint_and_exclude_master() {
        let splitter = VoiceSplitterSettings::default();
        for upper in [false, true] {
            let masks = splitter.pool_ranges(15, upper).unwrap();
            assert_eq!(masks.map(u16::count_ones), [2, 6, 3, 4]);
            let mut union = 0;
            for mask in masks {
                assert_eq!(mask & union, 0);
                union |= mask;
            }
            assert_eq!(union, if upper { 0x7fff } else { 0xfffe });
        }
        assert_eq!(
            splitter.pool_ranges(15, false).unwrap()[0],
            (1 << 1) | (1 << 2)
        );
        assert_eq!(
            splitter.pool_ranges(15, true).unwrap()[0],
            (1 << 14) | (1 << 13)
        );
        assert!(splitter.pool_ranges(11, false).is_none());
        assert!(splitter.pool_ranges(16, false).is_none());
        let small = VoiceSplitterSettings {
            pools: [1, 1, 1],
            ..splitter
        };
        assert_eq!(small.pool_ranges(4, false).unwrap(), [2, 4, 8, 16]);
    }
}
