use crate::dsp::{EngineSettings, Waveform, MAX_HARMONICS, MAX_VOICES};
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::Arc;

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum WaveformParam {
    Sine,
    Saw,
    Square,
    Triangle,
}

impl From<WaveformParam> for Waveform {
    fn from(value: WaveformParam) -> Self {
        match value {
            WaveformParam::Sine => Waveform::Sine,
            WaveformParam::Saw => Waveform::Saw,
            WaveformParam::Square => Waveform::Square,
            WaveformParam::Triangle => Waveform::Triangle,
        }
    }
}

fn corrected_high_hz(low_hz: f32, high_hz: f32) -> f32 {
    if low_hz >= high_hz {
        low_hz * 2.0
    } else {
        high_hz
    }
}

#[derive(Params)]
pub struct FundamentParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    #[id = "max_voices"]
    pub max_voices: IntParam,

    #[id = "harmonics"]
    pub harmonics: IntParam,

    #[id = "low_hz"]
    pub low_hz: FloatParam,

    #[id = "high_hz"]
    pub high_hz: FloatParam,

    #[id = "sensitivity_db"]
    pub sensitivity_db: FloatParam,

    #[id = "cut_depth_db"]
    pub cut_depth_db: FloatParam,

    #[id = "cut_q"]
    pub cut_q: FloatParam,

    #[id = "synth_level_db"]
    pub synth_level_db: FloatParam,

    #[id = "tone"]
    pub tone: FloatParam,

    #[id = "waveform"]
    pub waveform: EnumParam<WaveformParam>,

    #[id = "attack_ms"]
    pub attack_ms: FloatParam,

    #[id = "release_ms"]
    pub release_ms: FloatParam,

    #[id = "glide_ms"]
    pub glide_ms: FloatParam,

    #[id = "mix"]
    pub mix: FloatParam,

    #[id = "output_db"]
    pub output_db: FloatParam,

    #[id = "bypass"]
    pub bypass: BoolParam,
}

impl Default for FundamentParams {
    fn default() -> Self {
        Self {
            editor_state: crate::ui::default_editor_state(),

            max_voices: IntParam::new(
                "Voices",
                3,
                IntRange::Linear {
                    min: 1,
                    max: MAX_VOICES as i32,
                },
            ),

            harmonics: IntParam::new(
                "Harmonics",
                8,
                IntRange::Linear {
                    min: 1,
                    max: MAX_HARMONICS as i32,
                },
            ),

            low_hz: FloatParam::new(
                "Low",
                35.0,
                FloatRange::Skewed {
                    min: 25.0,
                    max: 400.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            high_hz: FloatParam::new(
                "High",
                1200.0,
                FloatRange::Skewed {
                    min: 100.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            sensitivity_db: FloatParam::new(
                "Sensitivity",
                -50.0,
                FloatRange::Linear {
                    min: -90.0,
                    max: -10.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            cut_depth_db: FloatParam::new(
                "Cut Depth",
                -12.0,
                FloatRange::Linear {
                    min: -36.0,
                    max: 0.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            cut_q: FloatParam::new(
                "Cut Width",
                8.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 18.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Q")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            synth_level_db: FloatParam::new(
                "Synth Level",
                0.0,
                FloatRange::Linear {
                    min: -36.0,
                    max: 12.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            tone: FloatParam::new("Tone", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            waveform: EnumParam::new("Waveform", WaveformParam::Sine),

            attack_ms: FloatParam::new(
                "Attack",
                20.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 500.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" ms")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            release_ms: FloatParam::new(
                "Release",
                120.0,
                FloatRange::Skewed {
                    min: 5.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" ms")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            glide_ms: FloatParam::new(
                "Glide",
                15.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 500.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" ms")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            output_db: FloatParam::new(
                "Output",
                0.0,
                FloatRange::Linear {
                    min: -24.0,
                    max: 12.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            bypass: BoolParam::new("Bypass", false),
        }
    }
}

impl FundamentParams {
    pub fn settings(&self) -> EngineSettings {
        let low_hz = self.low_hz.value();
        let high_hz = corrected_high_hz(low_hz, self.high_hz.value());
        EngineSettings {
            max_voices: self.max_voices.value() as usize,
            harmonics: self.harmonics.value() as usize,
            low_hz,
            high_hz,
            sensitivity_db: self.sensitivity_db.value(),
            cut_depth_db: self.cut_depth_db.value(),
            cut_q: self.cut_q.value(),
            synth_level_db: self.synth_level_db.value(),
            tone: self.tone.value(),
            waveform: self.waveform.value().into(),
            attack_ms: self.attack_ms.value(),
            release_ms: self.release_ms.value(),
            glide_ms: self.glide_ms.value(),
            mix: self.mix.value(),
            output_db: self.output_db.value(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_match_engine_defaults() {
        assert_eq!(
            FundamentParams::default().settings(),
            EngineSettings::default()
        );
    }

    #[test]
    fn waveform_param_maps_to_matching_waveform() {
        assert_eq!(Waveform::from(WaveformParam::Sine), Waveform::Sine);
        assert_eq!(Waveform::from(WaveformParam::Saw), Waveform::Saw);
        assert_eq!(Waveform::from(WaveformParam::Square), Waveform::Square);
        assert_eq!(Waveform::from(WaveformParam::Triangle), Waveform::Triangle);
    }

    #[test]
    fn high_hz_correction_doubles_low_when_not_above() {
        assert_eq!(corrected_high_hz(35.0, 1200.0), 1200.0);
        assert_eq!(corrected_high_hz(400.0, 400.0), 800.0);
        assert_eq!(corrected_high_hz(400.0, 100.0), 800.0);
        assert_eq!(corrected_high_hz(100.0, 200.0), 200.0);
    }
}
