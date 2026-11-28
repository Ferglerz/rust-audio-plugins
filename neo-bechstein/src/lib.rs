//! Soundboardless grand with an escapement-driven microhammer and shared magnetic heads.
pub mod action;
pub mod amp;
pub mod keyboard;
pub mod pickup;

use amp::{Amplifier, Controls};
use keyboard::{Keyboard, Terminals};

/// The keyboard's loaded pickup terminals feed the ELV413 piano input.
pub struct Engine {
    pub keyboard: Keyboard,
    amp: Amplifier,
    quiet_samples: u32,
    tail: u32,
}

impl Engine {
    pub fn new(rate: f64) -> Self {
        let tail = (0.12 * rate).ceil() as u32;
        Self {
            keyboard: Keyboard::new(rate),
            amp: Amplifier::new(rate, Controls::default()),
            quiet_samples: tail,
            tail,
        }
    }
    pub fn set_controls(&mut self, controls: Controls) {
        self.amp.set_controls(controls);
    }
    pub fn reset(&mut self) {
        self.keyboard.reset();
        self.amp.reset();
        self.quiet_samples = self.tail;
    }
    pub fn sounding(&self) -> bool {
        self.keyboard.sounding() || self.quiet_samples < self.tail
    }
    /// Isolated head EMFs, loaded input voltage, and 6-ohm speaker terminal voltage.
    pub fn sample_with_taps(&mut self) -> (Terminals, f64) {
        if !self.sounding() {
            return (Terminals::default(), 0.);
        }
        let taps = self.keyboard.sample();
        let speaker = self.amp.tick(taps.input_v);
        if !self.keyboard.sounding() && speaker.abs() < 1e-8 && taps.input_v.abs() < 1e-9 {
            self.quiet_samples = self.quiet_samples.saturating_add(1);
            if self.quiet_samples == self.tail {
                self.amp.reset();
                self.keyboard.clear_pickup_tail();
            }
        } else {
            self.quiet_samples = 0;
        }
        (taps, speaker)
    }
    pub fn render(&mut self, output: &mut [f32]) {
        for s in output {
            *s = self.sample_with_taps().1 as f32;
        }
    }
}
