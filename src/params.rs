use crate::{
    band::Band,
    lift::LiftBand,
    processing::{Config, ProcessingMode, Resolution},
};
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::{Arc, Mutex};
#[derive(Params)]
pub struct StripParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,
    #[persist = "eq-bands"]
    pub bands: Arc<Mutex<Vec<Band>>>,
    #[persist = "eq2-bands"]
    pub eq2_bands: Arc<Mutex<Vec<Band>>>,
    #[persist = "lift-bands"]
    pub lift_bands: Arc<Mutex<Vec<LiftBand>>>,
    #[persist = "eq-graph-range"]
    pub graph_range: Arc<Mutex<f64>>,
    #[id = "processing_mode"]
    pub processing_mode: EnumParam<ProcessingMode>,
    #[id = "linear_resolution"]
    pub linear_resolution: EnumParam<Resolution>,
    #[id = "bypass"]
    pub bypass: BoolParam,
    #[id = "eq_on"]
    pub eq_on: BoolParam,
    #[id = "eq2_on"]
    pub eq2_on: BoolParam,
    #[id = "comp_on"]
    pub comp_on: BoolParam,
    #[id = "comp_pre"]
    pub comp_pre: BoolParam,
    #[id = "compression"]
    // Keep the original positive depth and parameter ID for saved automation.
    // The editor and host display this depth as a negative threshold.
    pub compression: FloatParam,
    #[id = "comp_ratio"]
    pub comp_ratio: FloatParam,
    #[id = "comp_attack"]
    pub comp_attack: FloatParam,
    #[id = "comp_release"]
    pub comp_release: FloatParam,
    #[id = "soft_knee"]
    pub soft_knee: BoolParam,
    #[id = "auto_makeup"]
    pub auto_makeup: BoolParam,
    #[id = "stereo_link"]
    pub stereo_link: BoolParam,
    #[id = "gate"]
    pub gate: FloatParam,
    #[id = "pse_depth"]
    pub pse_depth: FloatParam,
    #[id = "pse_hysteresis"]
    pub pse_hysteresis: FloatParam,
    #[id = "pse_knee"]
    pub pse_knee: FloatParam,
    #[id = "pse_peak"]
    pub pse_peak: BoolParam,
    #[id = "pse_time"]
    pub pse_time: FloatParam,
    #[id = "pse_listen"]
    pub pse_listen: BoolParam,
    #[id = "sc_hpf"]
    pub sc_hpf: FloatParam,
    #[id = "dry"]
    pub dry: FloatParam,
    #[id = "wet"]
    pub wet: FloatParam,
}
impl Default for StripParams {
    fn default() -> Self {
        fn param(name: &str, v: f32, min: f32, max: f32, unit: &'static str) -> FloatParam {
            FloatParam::new(name, v, FloatRange::Linear { min, max })
                .with_unit(unit)
                .with_step_size(0.1)
                .with_smoother(SmoothingStyle::Linear(20.0))
        }
        Self {
            editor_state: ViziaState::new(|| (1120, 800)),
            bands: Arc::new(Mutex::new(Vec::new())),
            eq2_bands: Arc::new(Mutex::new(Vec::new())),
            lift_bands: Arc::new(Mutex::new(Vec::new())),
            graph_range: Arc::new(Mutex::new(24.0)),
            processing_mode: EnumParam::new("EQ processing mode", ProcessingMode::ZeroLatency),
            linear_resolution: EnumParam::new("Linear phase resolution", Resolution::Medium),
            bypass: BoolParam::new("Bypass", false).make_bypass(),
            eq_on: BoolParam::new("EQ enabled", true),
            eq2_on: BoolParam::new("EQ 2 enabled", true),
            comp_on: BoolParam::new("Dynamics enabled", true),
            comp_pre: BoolParam::new("Dynamics routing", false),
            compression: param("Threshold", 0.0, 0.0, 48.0, " dB")
                .with_value_to_string(Arc::new(|depth| format!("{:.1}", -depth)))
                .with_string_to_value(Arc::new(|text| {
                    text.trim()
                        .trim_end_matches("dB")
                        .trim()
                        .parse::<f32>()
                        .ok()
                        .filter(|v| v.is_finite())
                        .map(|v| -v)
                })),
            comp_ratio: param("Ratio", 4.0, 1.0, 20.0, ":1"),
            comp_attack: FloatParam::new(
                "Attack",
                2.0,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-2.5),
                },
            )
            .with_unit(" ms")
            .with_step_size(0.1)
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_value_to_string(Arc::new(|v| format!("{:.1} ms", v)))
            .with_string_to_value(Arc::new(|text| {
                text.trim()
                    .trim_end_matches("ms")
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .filter(|v| v.is_finite())
            })),
            comp_release: FloatParam::new(
                "Release",
                120.0,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_step_size(1.0)
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_value_to_string(Arc::new(|v| {
                if v >= 1000.0 {
                    format!("{:.2} s", v / 1000.0)
                } else {
                    format!("{:.0} ms", v)
                }
            }))
            .with_string_to_value(Arc::new(|text| {
                let text = text.trim().to_ascii_lowercase();
                if let Some(s) = text.strip_suffix("ms") {
                    s.trim().parse::<f32>().ok().filter(|v| v.is_finite())
                } else if let Some(s) = text.strip_suffix('s') {
                    s.trim()
                        .parse::<f32>()
                        .ok()
                        .filter(|v| v.is_finite())
                        .map(|v| v * 1000.0)
                } else if let Ok(v) = text.parse::<f32>() {
                    if v.is_finite() {
                        if v <= 5.0 {
                            Some(v * 1000.0)
                        } else {
                            Some(v)
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })),
            soft_knee: BoolParam::new("Soft knee", true),
            auto_makeup: BoolParam::new("Auto makeup", true),
            stereo_link: BoolParam::new("Linked stereo", true),
            gate: param("PSE threshold", -80.0, -80.0, -20.0, " dB"),
            pse_depth: param("PSE depth", 10.0, 0.0, 20.0, " dB"),
            pse_hysteresis: param("PSE hysteresis", 3.0, 0.0, 9.0, " dB"),
            pse_knee: param("PSE knee", 6.0, 0.0, 18.0, " dB"),
            pse_peak: BoolParam::new("PSE peak detection", false),
            pse_time: FloatParam::new(
                "PSE time",
                2.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            )
            .with_step_size(0.01)
            .with_smoother(SmoothingStyle::Linear(20.0))
            .with_value_to_string(Arc::new(|v| {
                let (_, release) = crate::dsp::pse_time_to_times(v as f64, false);
                if release >= 10.0 {
                    format!("{:.1} s", release)
                } else if release >= 1.0 {
                    if (release * 10.0).fract().abs() < 1e-3 {
                        format!("{:.1} s", release)
                    } else {
                        format!("{:.2} s", release)
                    }
                } else {
                    format!("{:.0} ms", release * 1000.0)
                }
            }))
            .with_string_to_value(Arc::new(|text| {
                let text = text.trim().to_ascii_lowercase();
                match text.as_str() {
                    "a" => return Some(0.0),
                    "b" => return Some(1.0),
                    "c" => return Some(2.0),
                    "d" => return Some(3.0),
                    "e" => return Some(4.0),
                    "f" => return Some(5.0),
                    _ => {}
                }
                if let Some(s) = text.strip_suffix("ms") {
                    if let Ok(ms) = s.trim().parse::<f64>() {
                        return Some(crate::dsp::seconds_to_pse_time_pos(ms * 0.001) as f32);
                    }
                }
                if let Some(s) = text.strip_suffix("s") {
                    if let Ok(sec) = s.trim().parse::<f64>() {
                        return Some(crate::dsp::seconds_to_pse_time_pos(sec) as f32);
                    }
                }
                if let Ok(val) = text.parse::<f32>() {
                    if (0.0..=5.0).contains(&val) {
                        return Some(val);
                    }
                    if val > 5.0 {
                        return Some(crate::dsp::seconds_to_pse_time_pos(val as f64 * 0.001) as f32);
                    }
                }
                None
            })),
            pse_listen: BoolParam::new("PSE listen sidechain", false),
            sc_hpf: FloatParam::new(
                "Sidechain high pass",
                80.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 500.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz")
            .with_step_size(1.0)
            .with_smoother(SmoothingStyle::Linear(20.0)),
            dry: param("Dry level", 0.0, 0.0, 100.0, " %"),
            wet: param("Wet level", 100.0, 0.0, 100.0, " %"),
        }
    }
}
impl StripParams {
    pub fn processing_config(&self) -> Config {
        Config {
            mode: self.processing_mode.value(),
            resolution: self.linear_resolution.value(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eq_bands_round_trip_in_host_state() {
        let params = StripParams::default();
        params.bands.lock().unwrap().push(Band {
            id: 51,
            freq: 1234.0,
            dynamic: true,
            threshold: -33.0,
            ..Band::default()
        });
        *params.graph_range.lock().unwrap() = 48.0;
        let fields = params.serialize_fields();
        let restored = StripParams::default();
        restored.deserialize_fields(&fields);
        assert_eq!(*restored.graph_range.lock().unwrap(), 48.0);
        assert_eq!(
            *params.bands.lock().unwrap(),
            *restored.bands.lock().unwrap()
        );
    }
    #[test]
    fn lift_bands_round_trip_in_host_state() {
        let params = StripParams::default();
        params.lift_bands.lock().unwrap().push(LiftBand {
            id: 100_005,
            freq: 14000.0,
            gain: 8.5,
            threshold: -20.0,
            ..LiftBand::default()
        });
        let fields = params.serialize_fields();
        let restored = StripParams::default();
        restored.deserialize_fields(&fields);
        assert_eq!(
            *params.lift_bands.lock().unwrap(),
            *restored.lift_bands.lock().unwrap()
        );
    }
    #[test]
    fn eq2_bands_round_trip_in_host_state() {
        let params = StripParams::default();
        params.eq2_bands.lock().unwrap().push(Band {
            id: 10_001,
            freq: 3456.0,
            gain: -4.5,
            ..Band::default()
        });
        let fields = params.serialize_fields();
        let restored = StripParams::default();
        restored.deserialize_fields(&fields);
        assert_eq!(
            *params.eq2_bands.lock().unwrap(),
            *restored.eq2_bands.lock().unwrap()
        );
    }
}
