//! Valente's passive two-control panel and damper pull.
use nih_plug::prelude::*;
use pianet_n::valente::Controls;
use std::sync::atomic::{AtomicBool, Ordering};
#[derive(Params)]
pub struct ValenteTone {
    #[id = "volume"]
    pub volume: FloatParam,
    #[id = "bright"]
    pub brilliance: FloatParam,
    #[id = "hold"]
    pub sustain_pedal: BoolParam,
    midi_hold: AtomicBool,
}
impl Default for ValenteTone {
    fn default() -> Self {
        let knob = |name| {
            FloatParam::new(name, 1., FloatRange::Linear { min: 0., max: 1. })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
        };
        Self {
            volume: knob("Valente Volume"),
            brilliance: knob("Valente Brilliance"),
            sustain_pedal: BoolParam::new("Valente Damper Pedal", false),
            midi_hold: AtomicBool::new(false),
        }
    }
}
impl ValenteTone {
    pub fn controls(&self) -> Controls {
        Controls {
            volume: self.volume.value().into(),
            brilliance: self.brilliance.value().into(),
        }
    }
    pub fn sustain(&self, midi: bool) -> bool {
        self.midi_hold.store(midi, Ordering::Relaxed);
        midi || self.sustain_pedal.value()
    }
    pub fn display_state(&self) -> (Controls, bool) {
        (
            self.controls(),
            self.midi_hold.load(Ordering::Relaxed) || self.sustain_pedal.value(),
        )
    }
}
