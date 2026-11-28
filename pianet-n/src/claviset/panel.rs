//! Magnetic pickup and battery preamplifier equivalents. All parts are estimates.
use super::{Controls, Instrument, Modulation, Register};
use pleasant_dsp::circuit::{Circuit, CrossfadingFilter};
use std::f64::consts::TAU;

pub struct Panel {
    instrument: Instrument,
    rate: f64,
    filter: CrossfadingFilter<6>,
    controls: Controls,
    phase: f64,
    depth: f64,
    swell: f64,
    approach: f64,
}

impl Panel {
    pub fn new(instrument: Instrument, rate: f64) -> Self {
        let mut p = Self {
            instrument,
            rate,
            filter: CrossfadingFilter::new(rate),
            controls: Controls::default(),
            phase: 0.,
            depth: 0.,
            swell: 1.,
            approach: -(-1. / (0.006 * rate)).exp_m1(),
        };
        p.retune();
        p
    }
    pub fn reset(&mut self) {
        self.filter.reset();
        self.phase = 0.;
        self.depth = self.modulation().1;
        self.swell = self.controls.swell;
    }
    pub fn set(&mut self, c: Controls) {
        let rebuild = match self.instrument {
            Instrument::Pianotron => self.controls.tone != c.tone,
            Instrument::Claviset200 => self.controls.register != c.register,
        };
        self.controls = c;
        if rebuild {
            self.retune();
        }
    }
    fn modulation(&self) -> (f64, f64) {
        match self.instrument {
            Instrument::Pianotron => (3. + 6.5 * self.controls.rate, 0.90 * self.controls.depth),
            Instrument::Claviset200 => match self.controls.modulation {
                Modulation::Off => (3.7, 0.),
                Modulation::Slow => (3.7, 0.70),
                Modulation::Medium => (6.2, 0.75),
                Modulation::Fast => (8.9, 0.82),
            },
        }
    }
    fn retune(&mut self) {
        // A small-signal equivalent only. The Selmer specimen documents a
        // transformer. Its magnetizing inductance and turns ratio are estimated.
        let mut c = Circuit::<6>::new(7, self.rate);
        match self.instrument {
            Instrument::Pianotron => {
                let turns = 4.;
                let copper = 620. * turns * turns;
                c.input(1, turns / copper);
                c.r(1, 0, copper);
                c.l(1, 0, 1.6 * turns * turns);
                c.r(1, 2, 2200.);
            }
            Instrument::Claviset200 => {
                c.input(1, 1. / 1550.);
                c.r(1, 0, 1550.);
                c.l(1, 2, 0.18);
            }
        }
        c.c(2, 3, 470e-9);
        c.r(3, 0, 39e3);
        c.gm(4, 0, 3, 0.0015);
        c.r(4, 0, 8200.);
        c.r(4, 3, 470e3);
        c.c(4, 3, 180e-12);
        // Select a bypassed series resistance and shunt capacitance. The lack of a
        // traceable factory board leaves these as reproducible voicing estimates.
        let (bypass, shunt, divider) = match self.instrument {
            Instrument::Pianotron => (
                470e-12 + 2.2e-9 * self.controls.tone,
                2.2e-9 * 16f64.powf(1. - self.controls.tone),
                0.90,
            ),
            Instrument::Claviset200 => match self.controls.register {
                Register::Bass => (220e-12, 33e-9, 0.52),
                Register::Guitar => (1.0e-9, 16e-9, 0.68),
                Register::Tutti => (1.8e-9, 12e-9, 1.0),
                Register::Piano => (680e-12, 10e-9, 0.72),
                Register::Harp => (1.5e-9, 8.2e-9, 0.88),
                Register::Celesta => (2.2e-9, 5.6e-9, 1.0),
                Register::MusicBox => (3.3e-9, 4.7e-9, 0.58),
                Register::Metallophone => (4.7e-9, 3.3e-9, 0.80),
                Register::Xylophone => (6.8e-9, 2.2e-9, 1.0),
            },
        };
        c.r(4, 5, 15e3);
        c.c(4, 5, bypass);
        c.c(5, 0, shunt);
        c.r(5, 6, 1000. + (1. - divider) * 47e3);
        c.r(6, 0, 47e3 * divider);
        c.c(6, 7, 220e-9);
        c.r(7, 0, 1e6);
        c.compile(7, self.filter.prepare());
    }
    #[inline]
    pub fn step(&mut self, emf: f64) -> f64 {
        let (rate, target) = self.modulation();
        self.phase = (self.phase + rate / self.rate).fract();
        self.depth += self.approach * (target - self.depth);
        self.swell += self.approach * (self.controls.swell - self.swell);
        let ac = self.filter.tick(emf);
        let gain = 1. - 0.5 * self.depth * (1. - (TAU * self.phase).sin());
        if self.instrument == Instrument::Claviset200 && !self.controls.power {
            0.
        } else {
            ac * gain * self.swell * self.swell
        }
    }
}
