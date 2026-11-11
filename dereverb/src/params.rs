use crate::weighting::{LevelMode, Weighting};
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::Arc;

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    #[name = "Live"]
    #[id = "live"]
    Live,
    #[name = "High Quality"]
    #[id = "hq"]
    HighQuality,
}

impl Quality {
    pub const fn index(self) -> usize {
        match self {
            Self::Live => 0,
            Self::HighQuality => 1,
        }
    }
    pub const fn delay_frames(self) -> usize {
        match self {
            Self::Live => 1,
            Self::HighQuality => 4,
        }
    }
    pub const fn model_latency(self) -> usize {
        960 + self.delay_frames() * 480
    }
}

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineKind {
    #[name = "DPDFNet"]
    #[id = "dpdfnet"]
    DpdfNet,
    #[name = "NARA-WPE"]
    #[id = "nara_wpe"]
    NaraWpe,
    #[name = "DTLN Denoise"]
    #[id = "dtln"]
    Dtln,
}

impl EngineKind {
    pub const MODES: [(Self, Quality); 5] = [
        (Self::DpdfNet, Quality::Live),
        (Self::DpdfNet, Quality::HighQuality),
        (Self::NaraWpe, Quality::Live),
        (Self::NaraWpe, Quality::HighQuality),
        (Self::Dtln, Quality::Live),
    ];

    pub const fn has_quality(self) -> bool {
        !matches!(self, Self::Dtln)
    }

    pub const fn index(self, quality: Quality) -> usize {
        match self {
            Self::DpdfNet => quality.index(),
            Self::NaraWpe => 2 + quality.index(),
            Self::Dtln => 4,
        }
    }
}

#[derive(Params)]
pub struct DereverbParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,
    #[id = "strength"]
    pub strength: FloatParam,
    #[id = "transient_guard"]
    pub transient_guard: FloatParam,
    #[id = "engine"]
    pub engine: EnumParam<EngineKind>,
    #[id = "quality"]
    pub quality: EnumParam<Quality>,
    #[id = "level_mode"]
    pub level_mode: EnumParam<LevelMode>,
    #[id = "weight_0"]
    pub weight_0: FloatParam,
    #[id = "weight_1"]
    pub weight_1: FloatParam,
    #[id = "weight_2"]
    pub weight_2: FloatParam,
    #[id = "weight_3"]
    pub weight_3: FloatParam,
    #[id = "weight_4"]
    pub weight_4: FloatParam,
}

impl DereverbParams {
    pub fn weights(&self) -> [&FloatParam; 5] {
        [
            &self.weight_0,
            &self.weight_1,
            &self.weight_2,
            &self.weight_3,
            &self.weight_4,
        ]
    }
    pub fn weighting(&self) -> Weighting {
        Weighting {
            mode: self.level_mode.value(),
            nodes: self.weights().map(|p| p.value() * 0.01),
        }
    }
}

impl Default for DereverbParams {
    fn default() -> Self {
        let weight = |name| {
            FloatParam::new(
                name,
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(1))
        };
        Self {
            editor_state: ViziaState::new_screen_sized("Dereverb", || (800, 620)),
            strength: FloatParam::new(
                "Strength",
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            transient_guard: weight("Transient Guard").non_automatable(),
            engine: EnumParam::new("Engine", EngineKind::DpdfNet).non_automatable(),
            quality: EnumParam::new("Quality", Quality::Live).non_automatable(),
            level_mode: EnumParam::new("Level reference", LevelMode::Relative),
            weight_0: weight("Weight: quietest"),
            weight_1: weight("Weight: quiet"),
            weight_2: weight("Weight: middle"),
            weight_3: weight("Weight: loud"),
            weight_4: weight("Weight: loudest"),
        }
    }
}
