use nih_plug::prelude::Enum;

pub(super) const HOP: usize = 1024;

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ProcessingMode {
    #[default]
    #[id = "zero-latency"]
    #[name = "Zero Latency"]
    ZeroLatency,
    #[id = "natural-phase"]
    #[name = "Natural Phase (4x)"]
    NaturalPhase,
    #[id = "linear-phase"]
    #[name = "Linear Phase"]
    LinearPhase,
}

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Resolution {
    #[id = "low"]
    Low,
    #[default]
    #[id = "medium"]
    Medium,
    #[id = "high"]
    High,
    #[id = "very-high"]
    #[name = "Very High"]
    VeryHigh,
    #[id = "maximum"]
    Maximum,
}

pub const MODES: [ProcessingMode; 3] = [
    ProcessingMode::ZeroLatency,
    ProcessingMode::NaturalPhase,
    ProcessingMode::LinearPhase,
];
pub const RESOLUTIONS: [Resolution; 5] = [
    Resolution::Low,
    Resolution::Medium,
    Resolution::High,
    Resolution::VeryHigh,
    Resolution::Maximum,
];

impl ProcessingMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::ZeroLatency => "ZERO LATENCY",
            Self::NaturalPhase => "NATURAL PHASE (4x)",
            Self::LinearPhase => "LINEAR PHASE",
        }
    }

    pub fn rate(self, sr: f64) -> f64 {
        if self == Self::NaturalPhase {
            sr * 4.0
        } else {
            sr
        }
    }
}

impl Resolution {
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::VeryHigh => "VERY HIGH",
            Self::Maximum => "MAXIMUM",
        }
    }

    /// Round to partitions, preserving approximately constant resolution in Hz across rates.
    pub(super) fn order(self, sr: f64) -> usize {
        let base = [4096, 8192, 16384, 32768, 131072][self as usize];
        ((base as f64 * sr / 44100.0 / HOP as f64).ceil() as usize).max(2) * HOP
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub mode: ProcessingMode,
    pub resolution: Resolution,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: ProcessingMode::ZeroLatency,
            resolution: Resolution::Medium,
        }
    }
}

impl Config {
    pub fn encode(self) -> u32 {
        self.mode as u32 * 8 + self.resolution as u32
    }

    pub fn decode(value: u32) -> Self {
        Self {
            mode: MODES.get((value / 8) as usize).copied().unwrap_or_default(),
            resolution: RESOLUTIONS
                .get((value % 8) as usize)
                .copied()
                .unwrap_or_default(),
        }
    }

    pub fn latency(self, sr: f64) -> usize {
        match self.mode {
            ProcessingMode::ZeroLatency => 0,
            ProcessingMode::NaturalPhase => 64,
            ProcessingMode::LinearPhase => self.resolution.order(sr) / 2 + HOP,
        }
    }
}
