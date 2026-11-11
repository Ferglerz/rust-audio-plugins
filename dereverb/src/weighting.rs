//! Shared curve evaluation and level detection for processing and the editor.
use nih_plug::prelude::Enum;

pub const NODES: usize = 5;
pub const LOUDER: [f32; NODES] = [0.0, 0.1, 0.35, 0.8, 1.0];

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LevelMode {
    #[default]
    #[id = "relative"]
    Relative,
    #[id = "absolute"]
    Absolute,
}
impl LevelMode {
    pub fn range(self) -> (f32, f32) {
        match self {
            Self::Relative => (-36.0, 12.0),
            Self::Absolute => (-72.0, 0.0),
        }
    }
    pub fn position(self, db: f32) -> f32 {
        let (min, max) = self.range();
        ((db - min) / (max - min)).clamp(0.0, 1.0)
    }
    pub fn db(self, position: f32) -> f32 {
        let (min, max) = self.range();
        min + position * (max - min)
    }
}

#[derive(Clone, Copy)]
pub struct Weighting {
    pub mode: LevelMode,
    pub nodes: [f32; NODES],
}
impl Default for Weighting {
    fn default() -> Self {
        Self {
            mode: LevelMode::Relative,
            nodes: [1.0; NODES],
        }
    }
}
impl Weighting {
    /// Shape-preserving cubic interpolation, without overshoot between nodes.
    pub fn at(self, position: f32) -> f32 {
        let x = position.clamp(0.0, 1.0) * (NODES - 1) as f32;
        let i = (x as usize).min(NODES - 2);
        let t = x - i as f32;
        let slope = |n: usize| {
            if n == 0 {
                return self.nodes[1] - self.nodes[0];
            }
            if n == NODES - 1 {
                return self.nodes[n] - self.nodes[n - 1];
            }
            let a = self.nodes[n] - self.nodes[n - 1];
            let b = self.nodes[n + 1] - self.nodes[n];
            if a * b <= 0.0 {
                0.0
            } else {
                2.0 * a * b / (a + b)
            }
        };
        let t2 = t * t;
        let t3 = t2 * t;
        ((2.0 * t3 - 3.0 * t2 + 1.0) * self.nodes[i]
            + (t3 - 2.0 * t2 + t) * slope(i)
            + (-2.0 * t3 + 3.0 * t2) * self.nodes[i + 1]
            + (t3 - t2) * slope(i + 1))
        .clamp(0.0, 1.0)
    }
    pub fn amount(self, level: FrameLevel) -> f32 {
        self.at(self.mode.position(match self.mode {
            LevelMode::Relative => level.relative_db,
            LevelMode::Absolute => level.absolute_db,
        }))
    }
}

#[derive(Clone, Copy)]
pub struct FrameLevel {
    /// Multiplier restoring the original scale after normalized inference.
    /// Zero marks silence before the first observed input, including pre-roll.
    pub restore: f64,
    pub absolute_db: f32,
    pub relative_db: f32,
    pub active: bool,
}
impl Default for FrameLevel {
    fn default() -> Self {
        Self {
            restore: 0.0,
            absolute_db: -120.0,
            relative_db: -36.0,
            active: false,
        }
    }
}

pub struct LevelTracker {
    reference_power: f64,
    detector_power: f64,
    reference_rate: f64,
    attack_rate: f64,
    release_rate: f64,
}
impl Default for LevelTracker {
    fn default() -> Self {
        Self::new(0.01)
    }
}
impl LevelTracker {
    pub fn new(hop_seconds: f64) -> Self {
        Self {
            reference_power: 0.0,
            detector_power: 0.0,
            reference_rate: 1.0 - (-hop_seconds).exp(),
            attack_rate: 1.0 - (-hop_seconds / 0.03).exp(),
            release_rate: 1.0 - (-hop_seconds / 0.12).exp(),
        }
    }
    pub fn reset(&mut self) {
        self.reference_power = 0.0;
        self.detector_power = 0.0;
    }
    /// One call per configured hop. Initialize from the first nonzero frame so
    /// startup, too, is independent of a constant change in input gain.
    pub fn update(&mut self, power: f64) -> FrameLevel {
        if self.reference_power == 0.0 {
            self.reference_power = power;
            self.detector_power = power;
        } else {
            // One-second reference; 30 ms attack / 120 ms release detector.
            // No fixed input threshold or absolute gain cap.
            self.reference_power += self.reference_rate * (power - self.reference_power);
            let rate = if power > self.detector_power {
                self.attack_rate
            } else {
                self.release_rate
            };
            self.detector_power += rate * (power - self.detector_power);
        }
        if self.reference_power <= f64::MIN_POSITIVE {
            return FrameLevel::default();
        }
        FrameLevel {
            // Internal reference RMS of 0.1 (-20 dBFS), undone at the matching
            // delayed analysis frame, not at the current input frame.
            restore: self.reference_power.sqrt() / 0.1,
            absolute_db: (10.0 * self.detector_power.max(f64::MIN_POSITIVE).log10()) as f32,
            relative_db: (10.0
                * (self.detector_power / self.reference_power)
                    .max(f64::MIN_POSITIVE)
                    .log10()) as f32,
            active: power > 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_interpolates_nodes_without_overshoot_and_clamps_ends() {
        for nodes in [LOUDER, [1.0; NODES], [1.0, 0.2, 0.7, 0.0, 0.4]] {
            let curve = Weighting {
                nodes,
                ..Weighting::default()
            };
            for (i, &value) in nodes.iter().enumerate() {
                assert_eq!(curve.at(i as f32 / 4.0), value);
            }
            for segment in 0..4 {
                for step in 0..100 {
                    let value = curve.at((segment as f32 + step as f32 / 100.0) / 4.0);
                    assert!(value >= nodes[segment].min(nodes[segment + 1]) - 1e-6);
                    assert!(value <= nodes[segment].max(nodes[segment + 1]) + 1e-6);
                }
            }
            assert_eq!(curve.at(-1.0), nodes[0]);
            assert_eq!(curve.at(2.0), nodes[4]);
        }
    }

    #[test]
    fn detector_and_normalizer_scale_from_first_sound_through_silence() {
        let mut reference = LevelTracker::default();
        let mut quiet = LevelTracker::default();
        let relative = Weighting {
            nodes: LOUDER,
            ..Weighting::default()
        };
        let absolute = Weighting {
            mode: LevelMode::Absolute,
            nodes: LOUDER,
        };
        let mut absolute_difference = 0.0_f32;
        for frame in 0..800 {
            let power = if frame < 20 || (150..300).contains(&frame) {
                0.0
            } else {
                0.01 * (0.1 + (frame as f64 * 0.073).sin().abs())
            };
            let a = reference.update(power);
            let b = quiet.update(power * 1e-6);
            assert!(a.restore.is_finite() && b.restore.is_finite());
            if frame >= 20 {
                assert!((b.restore / a.restore - 0.001).abs() < 1e-10);
                assert!((a.relative_db - b.relative_db).abs() < 1e-5);
                assert!((a.absolute_db - b.absolute_db - 60.0).abs() < 1e-4);
                assert!((relative.amount(a) - relative.amount(b)).abs() < 1e-5);
                absolute_difference =
                    absolute_difference.max(absolute.amount(a) - absolute.amount(b));
            }
        }
        assert!(
            absolute_difference > 0.5,
            "absolute mode must respond to the original input level"
        );
    }
}
