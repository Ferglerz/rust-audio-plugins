//! Listening paths and original mechanical damper pedal.
use nih_plug::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
#[derive(Params)]
pub struct RoadTone {
    #[id = "board"]
    pub board: BoolParam,
    #[id = "hold"]
    pub sustain_pedal: BoolParam,
    midi: AtomicBool,
}
impl Default for RoadTone {
    fn default() -> Self {
        Self {
            board: BoolParam::new("Roadmaster Soundboard Monitor", false),
            sustain_pedal: BoolParam::new("Roadmaster Damper Pedal", false),
            midi: AtomicBool::new(false),
        }
    }
}
impl RoadTone {
    pub fn sustain(&self, midi: bool) -> bool {
        self.midi.store(midi, Ordering::Relaxed);
        self.sustain_pedal.value() || midi
    }
    pub fn display_state(&self) -> (bool, bool) {
        (
            self.board.value(),
            self.sustain_pedal.value() || self.midi.load(Ordering::Relaxed),
        )
    }
}
