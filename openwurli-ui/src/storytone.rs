//! Storytone keyboard-channel controls and the three cabinet pedals.
use ::storytone::amp::Controls;
use nih_plug::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Params)]
pub struct StorytoneTone {
    #[id = "optimum"]
    pub optimum: FloatParam,
    #[id = "manual"]
    pub manual: FloatParam,
    #[id = "swell"]
    pub swell: FloatParam,
    #[id = "soft"]
    pub soft: BoolParam,
    #[id = "sustain"]
    pub sustain: BoolParam,
    midi_swell: AtomicU32,
    midi_soft: AtomicBool,
    midi_sustain: AtomicBool,
}

impl Default for StorytoneTone {
    fn default() -> Self {
        let base = Controls::default();
        let level = |label, value| {
            FloatParam::new(label, value, FloatRange::Linear { min: 0., max: 1. })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
        };
        Self {
            optimum: level("Storytone Optimum Volume", base.optimum as f32),
            manual: level("Storytone Manual Volume", base.manual as f32),
            swell: level("Storytone Swell Pedal", base.swell as f32),
            soft: BoolParam::new("Storytone Soft Pedal", false),
            sustain: BoolParam::new("Storytone Damper Pedal", false),
            midi_swell: AtomicU32::new(0),
            midi_soft: AtomicBool::new(false),
            midi_sustain: AtomicBool::new(false),
        }
    }
}

impl StorytoneTone {
    pub fn controls(&self, midi_swell: f32) -> Controls {
        let ext = midi_swell.clamp(0., 1.);
        self.midi_swell.store(ext.to_bits(), Ordering::Relaxed);
        Controls {
            optimum: self.optimum.value().into(),
            manual: self.manual.value().into(),
            swell: self.swell.value().max(ext).into(),
        }
    }
    pub fn pedals(&self, midi_soft: bool, midi_sustain: bool) -> (bool, bool) {
        self.midi_soft.store(midi_soft, Ordering::Relaxed);
        self.midi_sustain.store(midi_sustain, Ordering::Relaxed);
        (
            self.soft.value() || midi_soft,
            self.sustain.value() || midi_sustain,
        )
    }
    pub fn display(&self) -> (f32, bool, bool) {
        (
            self.swell
                .value()
                .max(f32::from_bits(self.midi_swell.load(Ordering::Relaxed))),
            self.soft.value() || self.midi_soft.load(Ordering::Relaxed),
            self.sustain.value() || self.midi_sustain.load(Ordering::Relaxed),
        )
    }
}
