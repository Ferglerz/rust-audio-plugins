//! Independent key actions, common harp and interchangeable output panels.
pub use crate::action::{HIGH, LOW};
use crate::{
    action::Action,
    panel::{Controls, Output, Panel},
    stereo,
};
use pleasant_dsp::instrument::Decimator;

pub const LATENCY: usize = 12;
pub fn hammer_speed(velocity: f32) -> f64 {
    2.0 * f64::from(velocity.clamp(0., 1.)).powf(1.4)
}

use pleasant_dsp::struck_keys::{StrikeKeys, StruckAction};

impl StruckAction for Action {
    fn strike_key(&mut self, v: f32) {
        self.strike(hammer_speed(v));
    }
    fn sustain(&mut self, down: bool) {
        self.sustain(down);
    }
    fn release(&mut self) {
        self.release();
    }
    fn clear(&mut self) {
        self.clear();
    }
    fn quiet(&self) -> bool {
        self.quiet()
    }
    fn reposition(&mut self, gap: f64) {
        self.reposition(gap);
    }
    fn step_output(&mut self, gap: f64) -> f64 {
        self.step(gap).pickup_v
    }
}
/// Output stage of the common electromechanical harp.
pub trait OutputStage {
    type Sample: Default;
    fn read(&mut self, source: f64) -> Self::Sample;
    fn quiet(&self) -> bool;
    fn reset(&mut self);
    fn park(&mut self) {
        self.reset();
    }
    fn idle(&mut self) {}
}
impl OutputStage for Panel {
    type Sample = Output;
    fn read(&mut self, source: f64) -> Output {
        self.tick(source)
    }
    fn quiet(&self) -> bool {
        self.quiet()
    }
    fn reset(&mut self) {
        self.reset();
    }
}
impl OutputStage for stereo::Panel {
    type Sample = stereo::Output;
    fn read(&mut self, source: f64) -> Self::Sample {
        self.tick(source)
    }
    fn quiet(&self) -> bool {
        self.quiet()
    }
    fn idle(&mut self) {
        self.optical_gains();
    }
    fn park(&mut self) {
        self.clear_audio();
    }
    fn reset(&mut self) {
        self.reset();
    }
}
pub type StereoEngine = Engine<stereo::Panel>;
pub struct Engine<P = Panel> {
    keys: StrikeKeys<Action>,
    steps: usize,
    filter: Decimator<1>,
    panel: P,
    tail: u32,
}
impl Engine {
    pub fn new(rate: f64) -> Self {
        Self::with_panel(rate, Panel::new(rate, Controls::default()))
    }
    pub fn set_controls(&mut self, c: Controls) {
        self.panel.set_controls(c);
    }
    pub fn render(&mut self, out: &mut [f32]) {
        for y in out {
            *y = self.next_outputs().jack_v as f32;
        }
    }
}
impl StereoEngine {
    pub fn active(rate: f64) -> Self {
        Self::with_panel(rate, stereo::Panel::new(rate, stereo::Controls::default()))
    }
    pub fn set_controls(&mut self, c: stereo::Controls) {
        self.panel.set_controls(c);
    }
    pub fn render_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        assert_eq!(left.len(), right.len());
        for (l, r) in left.iter_mut().zip(right) {
            let out = self.next_outputs();
            *l = out.left_v as f32;
            *r = out.right_v as f32;
        }
    }
}
impl<P: OutputStage> Engine<P> {
    fn with_panel(rate: f64, panel: P) -> Self {
        assert!(rate.is_finite() && rate >= 8000.);
        let steps = (176400. / rate).ceil().max(2.) as usize;
        Self {
            keys: StrikeKeys::new(rate, LOW, HIGH, |n| {
                Action::new(n, rate * steps as f64, rate)
            }),
            steps,
            filter: Decimator::new(steps, rate, LATENCY),
            panel,
            tail: 0,
        }
    }
    pub fn reset(&mut self) {
        self.keys.reset();
        self.filter.reset();
        self.panel.reset();
        self.tail = 0;
    }
    pub fn set_extended(&mut self, on: bool) {
        self.keys.set_extended(on);
    }
    pub fn set_pickup_gap_mm(&mut self, offset: f64) {
        self.keys.set_pickup_gap_mm(offset);
    }
    pub fn set_sustain(&mut self, down: bool) {
        self.keys.set_sustain(down);
    }
    pub fn note_on(&mut self, n: u8, v: f32) {
        self.keys.note_on(n, v);
    }
    pub fn note_off(&mut self, n: u8) {
        self.keys.note_off(n);
    }
    pub fn audition(&mut self, n: u8, v: f32) {
        self.keys.audition(n, v);
    }
    pub fn audition_off(&mut self) {
        self.keys.audition_off();
    }
    pub fn choke(&mut self, n: u8) {
        self.keys.choke(n);
    }
    pub fn held_notes(&self) -> u128 {
        self.keys.held_notes()
    }
    pub fn sounding(&self) -> bool {
        self.keys.active() || self.tail != 0
    }
    pub fn next_outputs(&mut self) -> P::Sample {
        if !self.sounding() {
            self.panel.idle();
            return P::Sample::default();
        }
        self.keys.begin_frame();
        for _ in 0..self.steps {
            self.filter.push([self.keys.step()]);
        }
        let [source] = self.filter.read();
        let output = self.panel.read(source);
        self.keys.finish_frame();
        if self.keys.active() {
            self.tail = (2 * LATENCY + 2) as u32;
        } else if self.tail > 1 {
            self.tail -= 1;
        } else if self.panel.quiet() {
            self.tail = 0;
            self.panel.park();
            self.filter.reset();
        }
        output
    }
}
