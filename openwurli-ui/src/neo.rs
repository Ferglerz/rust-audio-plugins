//! Controls on the Neo-Bechstein keyboard and the Siemens amplifier.
use neo_bechstein::amp::Controls;
use nih_plug::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Params)]
pub struct NeoTone {
    #[id = "master"]
    pub master: IntParam,
    #[id = "swell"]
    pub swell: FloatParam,
    #[id = "felt"]
    pub felt: BoolParam,
    #[id = "sustain"]
    pub sustain: BoolParam,
    midi_swell: AtomicU32,
    midi_sustain: AtomicBool,
}
impl Default for NeoTone {
    fn default() -> Self {
        let initial = Controls::default();
        Self {
            master: IntParam::new(
                "Neo-Bechstein Amplifier Step",
                i32::from(initial.master),
                IntRange::Linear { min: 1, max: 12 },
            ),
            swell: FloatParam::new(
                "Neo-Bechstein Swell Pedal",
                initial.swell as f32,
                FloatRange::Linear { min: 0., max: 1. },
            )
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            felt: BoolParam::new("Neo-Bechstein Felt Rail", false),
            sustain: BoolParam::new("Neo-Bechstein Damper Pedal", false),
            midi_swell: AtomicU32::new(0),
            midi_sustain: AtomicBool::new(false),
        }
    }
}
impl NeoTone {
    pub fn controls(&self, midi_swell: f32) -> Controls {
        self.midi_swell
            .store(midi_swell.to_bits(), Ordering::Relaxed);
        Controls {
            master: self.master.value() as u8,
            swell: f64::from(self.swell.value().max(midi_swell)),
        }
    }
    pub fn pedals(&self, midi_sustain: bool) -> (bool, bool) {
        self.midi_sustain.store(midi_sustain, Ordering::Relaxed);
        (self.felt.value(), self.sustain.value() || midi_sustain)
    }
    pub fn display(&self) -> (f32, bool, bool) {
        (
            self.swell
                .value()
                .max(f32::from_bits(self.midi_swell.load(Ordering::Relaxed))),
            self.felt.value(),
            self.sustain.value() || self.midi_sustain.load(Ordering::Relaxed),
        )
    }
}
