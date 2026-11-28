//! Nameboard volume and three pedals of the museum's C 1004.
use nih_plug::prelude::*;
use pianet_n::vivi::Controls;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
#[derive(Params)]
pub struct ViviTone {
    #[id = "volume"]
    pub volume: FloatParam,
    #[id = "swell"]
    pub expression: FloatParam,
    #[id = "hold"]
    pub sustain_pedal: BoolParam,
    #[id = "bass"]
    pub bass_pedal: BoolParam,
    midi_expression: AtomicU32,
    midi_sustain: AtomicBool,
    midi_bass: AtomicBool,
}
impl Default for ViviTone {
    fn default() -> Self {
        let f = |name| {
            FloatParam::new(name, 1., FloatRange::Linear { min: 0., max: 1. })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
        };
        Self {
            volume: f("Vivi-Tone Nameboard Volume"),
            expression: f("Vivi-Tone Volume Pedal"),
            sustain_pedal: BoolParam::new("Vivi-Tone Full Sustain", false),
            bass_pedal: BoolParam::new("Vivi-Tone Bass Sustain", false),
            midi_expression: AtomicU32::new(f32::NAN.to_bits()),
            midi_sustain: AtomicBool::new(false),
            midi_bass: AtomicBool::new(false),
        }
    }
}
impl ViviTone {
    fn values(&self, v: Option<f32>) -> Controls {
        Controls {
            volume: self.volume.value().into(),
            expression: v.unwrap_or(self.expression.value()).into(),
        }
    }
    pub fn controls(&self, midi: Option<f32>) -> Controls {
        self.midi_expression
            .store(midi.unwrap_or(f32::NAN).to_bits(), Ordering::Relaxed);
        self.values(midi)
    }
    pub fn sustain(&self, down: bool) -> bool {
        self.midi_sustain.store(down, Ordering::Relaxed);
        self.sustain_pedal.value() || down
    }
    pub fn bass_sustain(&self, down: bool) -> bool {
        self.midi_bass.store(down, Ordering::Relaxed);
        self.bass_pedal.value() || down
    }
    pub fn display_state(&self) -> (Controls, bool, bool) {
        let x = f32::from_bits(self.midi_expression.load(Ordering::Relaxed));
        (
            self.values(x.is_finite().then_some(x)),
            self.sustain_pedal.value() || self.midi_sustain.load(Ordering::Relaxed),
            self.bass_pedal.value() || self.midi_bass.load(Ordering::Relaxed),
        )
    }
}
