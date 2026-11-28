//! Serviced StereoVibe Rev 4.1 front-panel controls and input-stage DIP settings.
use nih_plug::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use vintage_vibe_64::stereo::{Controls, Input};

#[derive(Params)]
pub struct StereoVibeTone {
    #[id = "volume"]
    pub volume: FloatParam,
    #[id = "bass"]
    pub bass: FloatParam,
    #[id = "treble"]
    pub treble: FloatParam,
    #[id = "rate"]
    pub rate: FloatParam,
    #[id = "depth"]
    pub depth: FloatParam,
    #[id = "tremolo"]
    pub tremolo: BoolParam,
    #[id = "half"]
    pub half_speed: BoolParam,
    #[id = "input"]
    pub input: IntParam,
    #[id = "hold"]
    pub sustain_pedal: BoolParam,
    midi_hold: AtomicBool,
}
impl Default for StereoVibeTone {
    fn default() -> Self {
        let c = Controls::default();
        let knob = |name, v: f64| {
            FloatParam::new(name, v as f32, FloatRange::Linear { min: 0., max: 1. })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
        };
        Self {
            volume: knob("StereoVibe Volume", c.volume),
            bass: knob("StereoVibe Bass", c.bass),
            treble: knob("StereoVibe Treble", c.treble),
            rate: knob("StereoVibe Speed", c.rate),
            depth: knob("StereoVibe Depth", c.depth),
            tremolo: BoolParam::new("StereoVibe Tremolo", c.tremolo),
            half_speed: BoolParam::new("StereoVibe Half Speed", c.half_speed),
            input: IntParam::new("StereoVibe Input", 0, IntRange::Linear { min: 0, max: 2 })
                .with_value_to_string(std::sync::Arc::new(|x| {
                    ["Classic", "FET", "Parallel"][x.clamp(0, 2) as usize].into()
                })),
            sustain_pedal: BoolParam::new("StereoVibe Damper Pedal", false),
            midi_hold: AtomicBool::new(false),
        }
    }
}
impl StereoVibeTone {
    pub fn controls(&self) -> Controls {
        Controls {
            volume: self.volume.value().into(),
            bass: self.bass.value().into(),
            treble: self.treble.value().into(),
            rate: self.rate.value().into(),
            depth: self.depth.value().into(),
            tremolo: self.tremolo.value(),
            half_speed: self.half_speed.value(),
            input: [Input::Classic, Input::Fet, Input::Parallel][self.input.value() as usize],
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
