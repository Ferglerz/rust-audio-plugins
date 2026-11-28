//! Classroom Electropiano electrical soft pedal and mechanical damper pedal.
use nih_plug::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Params)]
pub struct BaldwinTone {
    #[id = "volume"]
    pub volume: FloatParam,
    #[id = "soft"]
    pub soft: BoolParam,
    #[id = "hold"]
    pub sustain: BoolParam,
    midi_soft: AtomicBool,
    midi_sustain: AtomicBool,
}
impl Default for BaldwinTone {
    fn default() -> Self {
        Self {
            volume: FloatParam::new(
                "Baldwin Volume",
                1.,
                FloatRange::Linear { min: 0., max: 1. },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            soft: BoolParam::new("Baldwin Electronic Soft Pedal", false),
            sustain: BoolParam::new("Baldwin Damper Pedal", false),
            midi_soft: AtomicBool::new(false),
            midi_sustain: AtomicBool::new(false),
        }
    }
}
impl BaldwinTone {
    pub fn pedals(&self, soft: bool, sustain: bool) -> (bool, bool) {
        self.midi_soft.store(soft, Ordering::Relaxed);
        self.midi_sustain.store(sustain, Ordering::Relaxed);
        (self.soft.value() || soft, self.sustain.value() || sustain)
    }
    pub fn display(&self) -> (bool, bool) {
        (
            self.soft.value() || self.midi_soft.load(Ordering::Relaxed),
            self.sustain.value() || self.midi_sustain.load(Ordering::Relaxed),
        )
    }
}
