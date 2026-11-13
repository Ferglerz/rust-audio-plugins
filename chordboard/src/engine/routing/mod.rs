//! Fixed-capacity routing with bounded source-to-source resolution.
use super::{Config, Engine, Out};

mod smooth;
mod targets;
#[cfg(test)]
mod tests;

pub(super) use smooth::RouteSmoother;
pub use targets::{
    available_targets, default_routes, maximum_for_routes, Target, TARGETS, TARGET_COUNT,
};

pub const LOOP_OCTAVE_IDS: [&str; 4] = [
    "arp_loop_1_octave",
    "arp_loop_2_octave",
    "arp_loop_3_octave",
    "arp_loop_4_octave",
];
pub const ROUTE_COUNT: usize = 16;
pub const SOURCE_COUNT: usize = 14;
pub const ARP_MOD_SOURCES: std::ops::Range<usize> = 9..11;
pub const ARP_POSITION_SOURCE: usize = 11;
pub const PERF_MOD_SOURCES: std::ops::Range<usize> = 12..14;
// Saved route source numbers are one-based. Append new sources at the end.
pub const SOURCES: [&str; SOURCE_COUNT + 1] = [
    "Off",
    "Pitch wheel",
    "Modwheel",
    "Expression",
    "Breath",
    "Pressure",
    "Timbre",
    "Input Vel",
    "Strum X",
    "Strum Y",
    "Tracker Mod 1",
    "Tracker Mod 2",
    "Arp Position",
    "Perf Mod 1",
    "Perf Mod 2",
];
pub const SOURCE_SHORT: [&str; SOURCE_COUNT] = [
    "Pitch",
    "Modwheel",
    "Expr",
    "Breath",
    "Press",
    "Timbre",
    "Input Vel",
    "X",
    "Y",
    "Mod 1",
    "Mod 2",
    "Arp Pos",
    "Perf Mod 1",
    "Perf Mod 2",
];
pub const SOURCE_DEFAULTS: [f32; SOURCE_COUNT] =
    [0.5, 0.0, 0.0, 0.0, 0.0, 0.5, 0.8, 0.0, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0];

pub fn bipolar_source(index: usize) -> bool {
    ARP_MOD_SOURCES.contains(&index) || PERF_MOD_SOURCES.contains(&index)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Route {
    pub enabled: bool,
    pub source: u8,
    pub target: u8,
    pub min: f32,
    pub max: f32,
    pub curve: f32,
    pub smoothing_ms: f32,
}
impl Default for Route {
    fn default() -> Self {
        Self {
            enabled: true,
            source: 0,
            target: 0,
            min: 0.0,
            max: 1.0,
            curve: 0.0,
            smoothing_ms: 0.0,
        }
    }
}
impl Route {
    pub fn additive(self) -> bool {
        self.source
            .checked_sub(1)
            .is_some_and(|index| bipolar_source(index as usize))
    }
    /// Bipolar routes preserve zero and use the contour's span as their depth.
    pub fn output(self, input: f32) -> f32 {
        if self.additive() {
            let input = input.clamp(-1.0, 1.0);
            input.signum()
                * (self.max - self.min)
                * input
                    .abs()
                    .powf(2.0_f32.powf(self.curve.clamp(-1.0, 1.0) * 3.0))
        } else {
            self.value(input)
        }
    }
    pub fn active(self) -> bool {
        self.enabled
            && (1..=SOURCE_COUNT as u8).contains(&self.source)
            && TARGETS
                .get(self.target as usize)
                .is_some_and(|target| target.accepts_source(self.source))
    }
    pub fn value(self, input: f32) -> f32 {
        let shape = input
            .clamp(0.0, 1.0)
            .powf(2.0_f32.powf(self.curve.clamp(-1.0, 1.0) * 3.0));
        (self.min + (self.max - self.min) * shape).clamp(0.0, 1.0)
    }
}
