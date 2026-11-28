//! Vintage Vibe 64 Classic’s original passive panel and mechanical sustain pedal.
use nih_plug::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use vintage_vibe_64::panel::Controls;

#[derive(Params)]
pub struct Vv64Tone {
    #[id = "volume"]
    pub volume: FloatParam,
    #[id = "bass"]
    pub bass: FloatParam,
    #[id = "treble"]
    pub treble: FloatParam,
    #[id = "hold"]
    pub sustain_pedal: BoolParam,
    midi_hold: AtomicBool,
}
impl Default for Vv64Tone {
    fn default() -> Self {
        let knob = |name| {
            FloatParam::new(name, 1., FloatRange::Linear { min: 0., max: 1. })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
        };
        Self {
            volume: knob("Vintage Vibe Volume"),
            bass: knob("Vintage Vibe Bass"),
            treble: knob("Vintage Vibe Treble"),
            sustain_pedal: BoolParam::new("Vintage Vibe Damper Pedal", false),
            midi_hold: AtomicBool::new(false),
        }
    }
}
impl Vv64Tone {
    pub fn controls(&self) -> Controls {
        Controls {
            volume: self.volume.value().into(),
            bass: self.bass.value().into(),
            treble: self.treble.value().into(),
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
