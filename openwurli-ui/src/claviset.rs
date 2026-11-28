//! Saved controls for the Weltmeister 200 and Selmer no. 8701.
use nih_plug::prelude::*;
use pianet_n::claviset::{Controls, Instrument, Modulation, Register};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToneTab {
    #[id = "bass"]
    Bass,
    #[id = "guitar"]
    Guitar,
    #[id = "tutti"]
    Tutti,
    #[id = "piano"]
    #[name = "Piano / Vibraphone"]
    Piano,
    #[id = "harp"]
    Harp,
    #[id = "celesta"]
    Celesta,
    #[id = "music_box"]
    #[name = "Music Box"]
    MusicBox,
    #[id = "metallophone"]
    Metallophone,
    #[id = "xylophone"]
    Xylophone,
}
impl ToneTab {
    pub const ALL: [Self; 9] = [
        Self::Bass,
        Self::Guitar,
        Self::Tutti,
        Self::Piano,
        Self::Harp,
        Self::Celesta,
        Self::MusicBox,
        Self::Metallophone,
        Self::Xylophone,
    ];
}
#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateTab {
    #[id = "off"]
    Off,
    #[id = "slow"]
    Slow,
    #[id = "medium"]
    Medium,
    #[id = "fast"]
    Fast,
}
impl RateTab {
    pub const ALL: [Self; 4] = [Self::Off, Self::Slow, Self::Medium, Self::Fast];
}

#[derive(Params)]
pub struct ClavisetTone {
    #[id = "register"]
    pub register: EnumParam<ToneTab>,
    #[id = "vibrato"]
    pub modulation: EnumParam<RateTab>,
    #[id = "foot"]
    pub foot_swell: FloatParam,
    #[id = "hold"]
    pub sustain_pedal: BoolParam,
    #[id = "power"]
    pub power: BoolParam,
    #[id = "knee"]
    pub knee_swell: FloatParam,
    #[id = "tone"]
    pub tone: FloatParam,
    #[id = "depth"]
    pub depth: FloatParam,
    #[id = "rate"]
    pub rate: FloatParam,
    midi_swell: AtomicU32,
    midi_sustain: AtomicBool,
}
impl Default for ClavisetTone {
    fn default() -> Self {
        let level = |name, v| {
            FloatParam::new(name, v, FloatRange::Linear { min: 0., max: 1. })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
        };
        Self {
            register: EnumParam::new("Claviset Register", ToneTab::Piano),
            modulation: EnumParam::new("Claviset Vibrato", RateTab::Off),
            foot_swell: level("Claviset Foot Swell", 1.),
            sustain_pedal: BoolParam::new("Claviset Sustain", false),
            power: BoolParam::new("Claviset Power", true),
            knee_swell: level("Pianotron Knee Swell", 1.),
            tone: level("Pianotron Tone", 0.5),
            depth: level("Pianotron Vibrato Depth", 0.),
            rate: level("Pianotron Vibrato Speed", 0.5),
            midi_swell: AtomicU32::new(u32::MAX),
            midi_sustain: AtomicBool::new(false),
        }
    }
}
impl ClavisetTone {
    fn settings(&self, which: Instrument, midi_swell: Option<f32>) -> Controls {
        let volume = match which {
            Instrument::Pianotron => self.knee_swell.value(),
            Instrument::Claviset200 => self.foot_swell.value(),
        };
        Controls {
            power: which == Instrument::Pianotron || self.power.value(),
            swell: midi_swell.unwrap_or(volume).clamp(0., 1.).into(),
            register: Register::ALL[self.register.value().to_index()],
            modulation: match self.modulation.value() {
                RateTab::Off => Modulation::Off,
                RateTab::Slow => Modulation::Slow,
                RateTab::Medium => Modulation::Medium,
                RateTab::Fast => Modulation::Fast,
            },
            tone: self.tone.value().into(),
            depth: self.depth.value().into(),
            rate: self.rate.value().into(),
        }
    }
    pub fn controls(&self, which: Instrument, midi_swell: Option<f32>) -> Controls {
        self.midi_swell
            .store(midi_swell.map_or(u32::MAX, f32::to_bits), Ordering::Relaxed);
        self.settings(which, midi_swell)
    }
    pub fn sustain(&self, midi: bool) -> bool {
        self.midi_sustain.store(midi, Ordering::Relaxed);
        self.sustain_pedal.value() || midi
    }
    pub fn display_state(&self, which: Instrument) -> (Controls, bool) {
        let raw = self.midi_swell.load(Ordering::Relaxed);
        (
            self.settings(which, (raw != u32::MAX).then(|| f32::from_bits(raw))),
            which == Instrument::Claviset200
                && (self.sustain_pedal.value() || self.midi_sustain.load(Ordering::Relaxed)),
        )
    }
}
