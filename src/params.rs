use crate::band::Band;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::{Arc, Mutex};
#[derive(Params)]
pub struct StripParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,
    #[persist = "eq-bands"]
    pub bands: Arc<Mutex<Vec<Band>>>,
    #[id = "bypass"]
    pub bypass: BoolParam,
    #[id = "eq_on"]
    pub eq_on: BoolParam,
    #[id = "comp_on"]
    pub comp_on: BoolParam,
    #[id = "compression"]
    pub compression: FloatParam,
    #[id = "gate"]
    pub gate: FloatParam,
    #[id = "output"]
    pub output: FloatParam,
    #[id = "sc_hpf"]
    pub sc_hpf: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
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
            bypass: BoolParam::new("Bypass", false).make_bypass(),
            eq_on: BoolParam::new("EQ enabled", true),
            comp_on: BoolParam::new("Compressor enabled", true),
            compression: param("Compression", 0.0, 0.0, 48.0, " dB"),
            gate: param("Gate threshold", -80.0, -80.0, -20.0, " dB"),
            output: param("Output", 0.0, -24.0, 24.0, " dB"),
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
            mix: param("Compressor mix", 100.0, 0.0, 100.0, " %"),
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
        let fields = params.serialize_fields();
        let restored = StripParams::default();
        restored.deserialize_fields(&fields);
        assert_eq!(
            *params.bands.lock().unwrap(),
            *restored.bands.lock().unwrap()
        );
    }
}
