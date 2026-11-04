//! Global MIDI effects, independent of tracker rows.
use serde::{Deserialize, Serialize};

pub const TAPS: usize = 8;
pub mod source {
    pub const ARP: u8 = 1;
    pub const STRUM: u8 = 2;
    pub const CHORDS: u8 = 4;
    pub const BASS: u8 = 8;
    pub const MELODY: u8 = 16;
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
fn finite(value: f32, default: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        default
    }
}

/// This order is both the module display order and the processing order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MidiFxId {
    Delay,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MidiFxState {
    pub delay: DelaySettings,
    pub order: Vec<MidiFxId>,
}
impl Default for MidiFxState {
    fn default() -> Self {
        Self {
            delay: DelaySettings::default(),
            order: vec![MidiFxId::Delay],
        }
    }
}
impl MidiFxState {
    pub fn sanitize(&mut self) {
        self.delay.sanitize();
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
        for id in [MidiFxId::Delay] {
            if !self.order.contains(&id) {
                self.order.push(id);
            }
        }
    }
    pub fn move_effect(&mut self, from: usize, to: usize) {
        move_item(&mut self.order, from, to);
    }
}

pub(crate) fn move_item<T>(items: &mut Vec<T>, from: usize, to: usize) {
    if from < items.len() && to < items.len() && from != to {
        let item = items.remove(from);
        items.insert(to, item);
    }
}
