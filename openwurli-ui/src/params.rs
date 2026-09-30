use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::Arc;

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum CpuMode {
    #[id = "fast"]
    #[name = "Fast"]
    Fast,
    #[id = "heavy"]
    #[name = "Heavy"]
    Heavy,
}

#[derive(Params)]
pub struct OpenWurliUiParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    #[id = "volume"]
    pub volume: FloatParam,

    #[id = "trem_depth"]
    pub tremolo_depth: FloatParam,

    #[id = "speaker"]
    pub speaker_character: FloatParam,

    #[id = "cpu_mode"]
    pub cpu_mode: EnumParam<CpuMode>,

    #[id = "mlp"]
    pub mlp_enabled: BoolParam,

    #[id = "reed_decay"]
    pub reed_decay: FloatParam,

    #[id = "hammer_hardness"]
    pub hammer_hardness: FloatParam,

    #[id = "pickup_drive"]
    pub pickup_drive: FloatParam,

    #[id = "trem_response"]
    pub tremolo_response: FloatParam,

    #[id = "noise_enable"]
    pub noise_enabled: BoolParam,

    #[id = "noise_gain"]
    pub noise_gain: FloatParam,

    #[id = "rail_sag"]
    pub rail_sag: BoolParam,
}

impl Default for OpenWurliUiParams {
    fn default() -> Self {
        Self {
            editor_state: ViziaState::new_screen_sized("OpenWurli UI", || (720, 340)),
            volume: percent_param("Volume", 0.5, 5.0),
            tremolo_depth: percent_param("Tremolo Depth", 0.5, 50.0),
            speaker_character: percent_param("Speaker Character", 0.0, 50.0),
            // Current upstream (v0.7.0) defaults this off. Its README's older
            // parameter table still says on.
            mlp_enabled: BoolParam::new("MLP Corrections", false),
            // Old presets omit this parameter and retain the previous Fast sound.
            cpu_mode: EnumParam::new("CPU Mode", CpuMode::Fast),
            reed_decay: multiplier_param("Reed Decay"),
            hammer_hardness: multiplier_param("Hammer Hardness"),
            pickup_drive: multiplier_param("Pickup Drive"),
            tremolo_response: multiplier_param("Tremolo Response"),
            noise_enabled: BoolParam::new("Hiss", false),
            noise_gain: FloatParam::new(
                "Noise Level",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 30.0,
                },
            )
            .with_unit("×")
            .with_value_to_string(Arc::new(|value| format!("{value:.1}")))
            .with_string_to_value(Arc::new(|text| {
                text.trim().trim_end_matches('×').parse().ok()
            })),
            rail_sag: BoolParam::new("Extra Sag", false),
        }
    }
}

fn multiplier_param(name: &'static str) -> FloatParam {
    FloatParam::new(name, 1.0, FloatRange::Linear { min: 0.5, max: 2.0 })
        .with_unit("×")
        .with_value_to_string(Arc::new(|value| format!("{value:.2}")))
        .with_string_to_value(Arc::new(|text| {
            text.trim().trim_end_matches('×').parse().ok()
        }))
}

fn percent_param(name: &'static str, default: f32, smooth_ms: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_smoother(SmoothingStyle::Linear(smooth_ms))
        .with_unit(" %")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}
