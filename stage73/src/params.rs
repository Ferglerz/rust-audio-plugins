use crate::{engine::Controls, era::Era};
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::Arc;

#[derive(Params)]
pub struct StageParams {
    pub(crate) keyboard: Arc<pleasant_ui::playable_keyboard::KeyboardLink>,
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,
    #[id = "volume"]
    pub volume: FloatParam,
    #[nested]
    pub tone: StageTone,
}
impl std::ops::Deref for StageParams {
    type Target = StageTone;
    fn deref(&self) -> &Self::Target {
        &self.tone
    }
}
impl Default for StageParams {
    fn default() -> Self {
        Self {
            keyboard: Arc::default(),
            editor_state: ViziaState::new_screen_sized("Stage 73", || {
                (crate::ui::SIZE.0 as u32, crate::ui::SIZE.1 as u32)
            }),
            volume: knob("Volume", 1.),
            tone: Default::default(),
        }
    }
}
impl StageParams {
    pub fn controls(&self) -> Controls {
        Controls {
            volume: self.volume.value() as f64,
            ..self.tone.controls()
        }
    }
}
fn knob(name: &'static str, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_unit(" %")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}
#[derive(Params)]
pub struct StageTone {
    #[id = "era"]
    pub era: EnumParam<Era>,
    #[id = "peterson"]
    pub peterson: BoolParam,
    #[id = "peterson_hz"]
    pub peterson_speed: FloatParam,
    #[id = "peterson_depth"]
    pub intensity: FloatParam,
    #[id = "jordan_volume"]
    pub jordan_volume: FloatParam,
    #[id = "jordan_bass"]
    pub jordan_bass: FloatParam,
    #[id = "jordan_treble"]
    pub treble: FloatParam,
    #[id = "trem_rate"]
    pub rate: FloatParam,
    #[id = "trem_depth"]
    pub depth: FloatParam,
    #[id = "bass_boost"]
    pub bass: FloatParam,
    #[id = "tine_height"]
    pub height: FloatParam,
    #[id = "pickup_gap"]
    pub gap: FloatParam,
    #[id = "trem_enabled"]
    pub tremolo: BoolParam,
}
impl Default for StageTone {
    fn default() -> Self {
        let offset = |name, min, max| {
            FloatParam::new(name, 0.0, FloatRange::Linear { min, max })
                .with_unit(" mm")
                .with_value_to_string(Arc::new(|v| format!("{v:+.2}")))
        };

        Self {
            era: EnumParam::new("Era", Era::LateI),
            peterson: BoolParam::new("Peterson Suitcase", false),
            peterson_speed: FloatParam::new(
                "Vibrato Speed",
                5.,
                FloatRange::Linear {
                    min: crate::peterson::MIN_HZ as f32,
                    max: crate::peterson::MAX_HZ as f32,
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            intensity: knob("Vibrato Intensity", 0.7),
            jordan_volume: knob("Jordan Volume", 0.8),
            jordan_bass: knob("Jordan Bass", 0.5),
            treble: knob("Jordan Treble", 0.5),
            tremolo: BoolParam::new("Tremolo", false),
            depth: knob("Tremolo Depth", 0.5),
            rate: FloatParam::new(
                "Tremolo Rate",
                5.0,
                FloatRange::Linear {
                    min: crate::period_panel::MIN_TREM_HZ as f32,
                    max: crate::period_panel::MAX_TREM_HZ as f32,
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            bass: knob("Stage Tone", 1.0),
            height: offset("Tine Height", -3.0, 3.0),
            gap: offset("Pickup Gap", -1.0, 1.0),
        }
    }
}
impl StageTone {
    pub fn controls(&self) -> Controls {
        let era = self.era.value();
        let peterson = self.peterson.value();
        Controls {
            era,
            peterson,
            playback_gain: era.playback_gain(peterson),
            peterson_hz: self.peterson_speed.value() as f64,
            intensity: self.intensity.value() as f64,
            jordan_volume: self.jordan_volume.value() as f64,
            jordan_bass: self.jordan_bass.value() as f64,
            treble: self.treble.value() as f64,
            rate_hz: self.rate.value() as f64,
            depth: if self.tremolo.value() {
                self.depth.value() as f64
            } else {
                0.0
            },
            volume: 1.,
            cable_pf: crate::circuit::DEFAULT_CABLE_PF,
            bass: self.bass.value() as f64,
            height_mm: self.height.value() as f64,
            gap_mm: self.gap.value() as f64,
        }
    }
}
