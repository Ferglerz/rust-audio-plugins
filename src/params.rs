use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::Arc;

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum MidiAssign {
    #[name = "CC"]
    Cc,
    #[name = "Note"]
    Note,
}

#[derive(Params)]
pub struct TapeStopParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    /// Drop Time in seconds [0.05s - 16.0s]
    #[id = "drop_time"]
    pub drop_time: FloatParam,

    /// Release Crossfade in milliseconds [5.0ms - 500.0ms]
    #[id = "xfade_ms"]
    pub xfade_ms: FloatParam,

    /// Deceleration Curve Exponent [0.1 - 4.0]
    #[id = "drop_curve"]
    pub drop_curve: FloatParam,

    /// Stereo Divergence in percentage [-100.0% to +100.0%]
    #[id = "stereo_div"]
    pub stereo_div: FloatParam,

    /// MIDI assignment type for the compact CC / Note control
    #[id = "midi_assign"]
    pub midi_assign: EnumParam<MidiAssign>,

    /// 14-bit Continuous Speed MIDI CC # [0 to 31]
    #[id = "override_cc"]
    pub override_cc: IntParam,

    /// MIDI note that triggers the tape brake when assigned to Note [0 to 127]
    #[id = "override_note"]
    pub override_note: IntParam,

    /// Auto Restart Enabled on Transient / Envelope
    #[id = "auto_restart"]
    pub auto_restart: BoolParam,

    /// Auto Restart Transient Threshold in dB [-60.0 dB to 0.0 dB]
    #[id = "auto_restart_thresh"]
    pub auto_restart_thresh: FloatParam,

    /// Master Power (Engaged / Off)
    #[id = "power"]
    pub power: BoolParam,

    /// Master Bypass
    #[id = "bypass"]
    pub bypass: BoolParam,
}

impl Default for TapeStopParams {
    fn default() -> Self {
        Self {
            editor_state: ViziaState::new_screen_sized(|| (1040, 520)),

            drop_time: FloatParam::new(
                "Drop Time",
                0.50,
                FloatRange::Skewed {
                    min: 0.05,
                    max: 16.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" s")
            .with_step_size(0.05)
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            xfade_ms: FloatParam::new(
                "Crossfade",
                50.0,
                FloatRange::Skewed {
                    min: 5.0,
                    max: 500.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" ms")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            drop_curve: FloatParam::new("Curve", 1.0, FloatRange::Linear { min: 0.1, max: 4.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),

            stereo_div: FloatParam::new(
                "Stereo Divergence",
                0.0,
                FloatRange::Linear {
                    min: -100.0,
                    max: 100.0,
                },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            midi_assign: EnumParam::new("MIDI Assign", MidiAssign::Cc),

            override_cc: IntParam::new("CC #", 3, IntRange::Linear { min: 0, max: 31 }),

            override_note: IntParam::new("Note #", 60, IntRange::Linear { min: 0, max: 127 }),

            auto_restart: BoolParam::new("Auto Restart", true),

            auto_restart_thresh: FloatParam::new(
                "Restart Threshold",
                -18.0,
                FloatRange::Linear {
                    min: -60.0,
                    max: 0.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            power: BoolParam::new("Power", true),

            bypass: BoolParam::new("Bypass", false),
        }
    }
}
