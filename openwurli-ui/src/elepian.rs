//! EP-61C’s passive panel and mechanical sustain pedal.
use nih_plug::prelude::*;
use pianet_n::elepian::Controls;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Params)]
pub struct ElepianTone {
    #[id = "volume"]
    pub volume: FloatParam,
    #[id = "tone"]
    pub tone: FloatParam,
    #[id = "align"]
    pub alignment: FloatParam,
    #[id = "hold"]
    pub sustain_pedal: BoolParam,
    midi_hold: AtomicBool,
}
impl Default for ElepianTone {
    fn default() -> Self {
        let knob = |name| {
            FloatParam::new(name, 1., FloatRange::Linear { min: 0., max: 1. })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
        };
        Self {
            volume: knob("Elepian Volume"),
            tone: knob("Elepian Tone"),
            alignment: FloatParam::new(
                "Elepian Pole Alignment",
                0.,
                FloatRange::Linear { min: -1., max: 1. },
            )
            .with_smoother(SmoothingStyle::Linear(5.))
            .with_unit(" mm"),
            sustain_pedal: BoolParam::new("Elepian Damper Pedal", false),
            midi_hold: AtomicBool::new(false),
        }
    }
}
impl ElepianTone {
    pub fn controls(&self) -> Controls {
        Controls {
            volume: self.volume.value().into(),
            tone: self.tone.value().into(),
            alignment_mm: self.alignment.value().into(),
        }
    }
    pub fn sustain(&self, midi: bool) -> bool {
        self.midi_hold.store(midi, Ordering::Relaxed);
        self.sustain_pedal.value() || midi
    }
    pub fn display_state(&self) -> (Controls, bool) {
        (
            self.controls(),
            self.sustain_pedal.value() || self.midi_hold.load(Ordering::Relaxed),
        )
    }
}
