//! Optional independently saved Pianet latch and its panel pedal.
use crate::models::Instrument;
use nih_plug::prelude::*;
use pianet_n::Model;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[derive(Params)]
pub struct Pedal {
    #[id = "enable"]
    pub enabled: BoolParam,
    #[id = "pedal"]
    pub pedal: BoolParam,
}
impl Pedal {
    fn new(name: &str) -> Self {
        Self {
            enabled: BoolParam::new(format!("{name} Sustain Mod"), false),
            pedal: BoolParam::new(format!("{name} Sustain Pedal"), false),
        }
    }
    pub fn state(&self, midi: bool) -> (bool, bool) {
        (
            self.enabled.value(),
            self.enabled.value() && (midi || self.pedal.value()),
        )
    }
}
#[derive(Params)]
pub struct Settings {
    #[nested(id_prefix = "n")]
    pub n: Pedal,
    #[nested(id_prefix = "c")]
    pub c: Pedal,
    #[nested(id_prefix = "l")]
    pub l: Pedal,
    #[nested(id_prefix = "t")]
    pub t: Pedal,
    #[nested(id_prefix = "duo")]
    pub duo: Pedal,
    midi: AtomicBool,
    caught_pads: [AtomicU64; 2],
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            n: Pedal::new("Pianet N"),
            c: Pedal::new("Pianet C"),
            l: Pedal::new("Pianet L"),
            t: Pedal::new("Pianet T"),
            duo: Pedal::new("Duo Pianet"),
            midi: AtomicBool::new(false),
            caught_pads: Default::default(),
        }
    }
}
impl Settings {
    pub fn publish_catches(&self, notes: u128) {
        self.caught_pads[0].store(notes as u64, Ordering::Relaxed);
        self.caught_pads[1].store((notes >> 64) as u64, Ordering::Relaxed);
    }
    pub fn pad_is_caught(&self, note: u8) -> bool {
        self.caught_pads[usize::from(note / 64)].load(Ordering::Relaxed) & (1 << (note % 64)) != 0
    }
    pub fn update_midi(&self, down: bool) {
        self.midi.store(down, Ordering::Relaxed);
    }
    pub(crate) fn for_model(&self, model: Instrument) -> Option<&Pedal> {
        Some(match model {
            Instrument::Reed(Model::N) => &self.n,
            Instrument::Reed(Model::C) => &self.c,
            Instrument::Reed(Model::L) => &self.l,
            Instrument::Reed(Model::T) => &self.t,
            Instrument::Duo => &self.duo,
            _ => return None,
        })
    }
    pub(crate) fn display(&self, model: Instrument) -> Option<(bool, bool)> {
        self.for_model(model)
            .map(|m| m.state(self.midi.load(Ordering::Relaxed)))
    }
    pub fn restore_missing(state: &mut nih_plug::wrapper::state::PluginState) {
        use nih_plug::wrapper::state::ParamValue;
        for model in ["n", "c", "l", "t", "duo"] {
            for attr in ["enable", "pedal"] {
                state
                    .params
                    .entry(format!("smod_{model}_{attr}"))
                    .or_insert(ParamValue::Bool(false));
            }
        }
    }
}
