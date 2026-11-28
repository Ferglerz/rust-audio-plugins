//! Four-zone MK8 S harp, standard active panel and stereo outputs.
pub use crate::action::{HIGH, LOW};
use crate::{
    action::Action,
    panel::{Controls, Outputs as Output, Panel},
};
use pleasant_dsp::instrument::Decimator;

pub const LATENCY: usize = 12;
pub fn hammer_speed(velocity: f32) -> f64 {
    2.65 * f64::from(velocity.clamp(0., 1.)).powf(1.4)
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
        self.set_pickup_gap_mm(gap * 1000.);
    }
    fn step_output(&mut self, _gap: f64) -> f64 {
        self.step().emf_v
    }
}
pub struct Engine {
    keys: StrikeKeys<Action>,
    steps: usize,
    filter: Decimator<3>,
    panel: Panel,
    tail: u32,
}
impl Engine {
    pub fn new(rate: f64) -> Self {
        assert!(rate.is_finite() && rate >= 8000.);
        let steps = (176400. / rate).ceil().max(2.) as usize;
        Self {
            keys: StrikeKeys::new(rate, LOW, HIGH, |n| {
                Action::new(n, rate * steps as f64, rate)
            }),
            steps,
            filter: Decimator::new(steps, rate, LATENCY),
            panel: Panel::new(rate * steps as f64, Controls::default()),
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
    pub fn set_controls(&mut self, c: Controls) {
        self.panel.set_controls(c);
    }
    pub fn set_fx_controls(&mut self, c: crate::fx::Controls) {
        self.panel.set_fx_controls(c);
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
    pub fn next_outputs(&mut self) -> Output {
        if !self.sounding() {
            return Output::default();
        }
        self.keys.begin_frame();
        for _ in 0..self.steps {
            let x = self.panel.tick(self.keys.step());
            self.filter.push([x.send_v, x.left_v, x.right_v]);
        }
        let [send_v, left_v, right_v] = self.filter.read();
        let output = Output {
            send_v,
            left_v,
            right_v,
        };
        self.keys.finish_frame();
        let magnitude = send_v.abs().max(left_v.abs()).max(right_v.abs());
        if self.keys.active() || self.panel.effects_sounding() || magnitude >= 1e-10 {
            self.tail = (2 * LATENCY + 64) as u32;
        } else {
            self.tail = self.tail.saturating_sub(1);
            if self.tail == 0 {
                self.panel.reset();
                self.filter.reset();
            }
        }
        output
    }
    pub fn render_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let x = self.next_outputs();
            *l = x.left_v as f32;
            *r = x.right_v as f32;
        }
    }
    pub fn render(&mut self, out: &mut [f32]) {
        for y in out {
            let x = self.next_outputs();
            *y = ((x.left_v + x.right_v) * 0.5) as f32;
        }
    }
}
