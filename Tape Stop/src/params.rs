use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use pleasant_dsp::tempo::{self, DIVISIONS, QUARTER_NOTE};
use std::sync::Arc;

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum XAxis {
    Curve,
    Return,
}

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum PadMode {
    Absolute,
    Relative,
}

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

    #[id = "time_sync"]
    pub time_sync: BoolParam,
    #[id = "tempo_sync"]
    pub tempo_sync: BoolParam,
    #[id = "tempo"]
    pub tempo: FloatParam,
    #[id = "drop_division"]
    pub drop_division: IntParam,
    #[id = "return_division"]
    pub return_division: IntParam,
    #[id = "xy_x_axis"]
    pub xy_x_axis: EnumParam<XAxis>,
    #[id = "xy_mode"]
    pub xy_mode: EnumParam<PadMode>,

    /// Drop Time in seconds [0.05s - 16.0s]
    #[id = "drop_time"]
    pub drop_time: FloatParam,

    /// Release Crossfade in milliseconds [5.0ms - 500.0ms]
    #[id = "xfade_ms"]
    pub xfade_ms: FloatParam,

    /// Time to spin back up and meet the live write head [0.05s - 8.0s]
    #[id = "return_sec"]
    pub return_sec: FloatParam,

    /// Deceleration Curve Exponent [0.1 - 4.0]
    #[id = "drop_curve"]
    pub drop_curve: FloatParam,

    /// Stereo Divergence in percentage [-100.0% to +100.0%]
    #[id = "stereo_div"]
    pub stereo_div: FloatParam,

    /// MIDI assignment type for the compact CC / Note control
    #[id = "midi_assign"]
    pub midi_assign: EnumParam<MidiAssign>,

    /// Continuous Speed MIDI CC # [0 to 127]; CC 0-31 support 14-bit pairs
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
            editor_state: crate::ui::default_editor_state(),
            time_sync: BoolParam::new("Time sync", false),
            tempo_sync: BoolParam::new("Host sync", true),
            tempo: FloatParam::new(
                "Tempo BPM",
                120.0,
                FloatRange::Linear {
                    min: 20.0,
                    max: 400.0,
                },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            drop_division: division_param("Drop division"),
            return_division: division_param("Return division"),
            xy_x_axis: EnumParam::new("XY X axis", XAxis::Curve),
            xy_mode: EnumParam::new("XY mode", PadMode::Absolute),

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
            .with_value_to_string(formatters::v2s_f32_rounded(4)),

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

            return_sec: FloatParam::new(
                "Return",
                0.50,
                FloatRange::Skewed {
                    min: 0.05,
                    max: 8.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" s")
            .with_value_to_string(formatters::v2s_f32_rounded(4)),

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

            override_cc: IntParam::new("CC #", 3, IntRange::Linear { min: 0, max: 127 }),

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

fn division_param(name: &'static str) -> IntParam {
    IntParam::new(
        name,
        QUARTER_NOTE,
        IntRange::Linear {
            min: 0,
            max: DIVISIONS.len() as i32 - 1,
        },
    )
    .with_value_to_string(Arc::new(|i| tempo::division(i).label.to_owned()))
    .with_string_to_value(Arc::new(tempo::parse_division))
}

impl TapeStopParams {
    /// UI durations: nominal full-stop time in sync, legacy drop rate in free.
    pub fn timing(&self, bpm: f64) -> (f32, f32) {
        if self.time_sync.value() {
            (
                tempo::beats_to_seconds(tempo::division(self.drop_division.value()).beats, bpm)
                    as f32,
                tempo::beats_to_seconds(tempo::division(self.return_division.value()).beats, bpm)
                    as f32,
            )
        } else {
            (self.drop_time.value(), self.return_sec.value())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_and_synced_settings_remain_independent_with_compatible_defaults() {
        let mut params = TapeStopParams::default();
        assert!(!params.time_sync.value());
        assert!(params.tempo_sync.value());
        assert_eq!(params.xy_x_axis.value(), XAxis::Curve);
        assert_eq!(params.xy_mode.value(), PadMode::Absolute);
        let free = params.timing(87.0);
        params.time_sync = BoolParam::new("Sync", true);
        assert_eq!(params.timing(60.0), (1.0, 1.0));
        assert_eq!(params.timing(120.0), (0.5, 0.5));
        params.time_sync = BoolParam::new("Sync", false);
        assert_eq!(params.timing(87.0), free);
        let ids: Vec<_> = params.param_map().into_iter().map(|p| p.0).collect();
        for id in [
            "drop_time",
            "return_sec",
            "drop_curve",
            "time_sync",
            "tempo_sync",
            "tempo",
            "drop_division",
            "return_division",
            "xy_x_axis",
            "xy_mode",
        ] {
            assert!(ids.iter().any(|v| v == id));
        }
    }
}
