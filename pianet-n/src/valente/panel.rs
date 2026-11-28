//! Passive two-control terminal and the reed harp feeding it.
use super::{action, LOW};
use crate::keyboard::Board;
use pleasant_dsp::{
    circuit::{Circuit, CrossfadingFilter},
    sympathetic::{ReceiverSpec, Sympathetic},
};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Controls {
    pub volume: f64,
    pub brilliance: f64,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            volume: 1.,
            brilliance: 1.,
        }
    }
}

pub struct Panel {
    rate: f64,
    load: f64,
    controls: Controls,
    filter: CrossfadingFilter<4>,
}
impl Panel {
    pub fn new(rate: f64, load: f64) -> Self {
        assert!(rate.is_finite() && rate > 0. && load.is_finite() && load > 0.);
        let mut p = Self {
            rate,
            load,
            controls: Controls::default(),
            filter: CrossfadingFilter::new(rate),
        };
        p.compile();
        p
    }
    pub fn set(&mut self, c: Controls) {
        let c = Controls {
            volume: c.volume.clamp(0., 1.),
            brilliance: c.brilliance.clamp(0., 1.),
        };
        if self.controls != c {
            self.controls = c;
            self.compile();
        }
    }
    fn compile(&mut self) {
        // Effective component values. Two passive controls and the audible
        // treble bleed are documented, but a board trace has not surfaced.
        let mut c = Circuit::<4>::new(4, self.rate);
        c.input(1, 1. / 3400.);
        c.r(1, 0, 3400.);
        c.l(1, 2, 0.24);
        c.r(2, 3, 8200. + 100e3 * self.controls.brilliance);
        c.c(3, 0, 6.8e-9);
        c.r(2, 4, (250e3 * (1. - self.controls.volume)).max(0.001));
        c.c(2, 4, 470e-12);
        c.r(4, 0, (250e3 * self.controls.volume).max(0.001));
        c.r(4, 0, self.load);
        c.c(4, 0, 470e-12);
        c.compile(
            if self.controls.volume == 0. { 0 } else { 4 },
            self.filter.prepare(),
        );
    }
    pub fn step(&mut self, emf: f64) -> f64 {
        self.filter.tick(emf)
    }
}
pub struct Harp {
    pub(super) panel: Panel,
    receivers: Sympathetic<61>,
}
impl Harp {
    pub(super) fn new(rate: f64, load: f64) -> Self {
        let receivers = Sympathetic::new(rate, LOW, 0, 650., 240., 1.8, |i| {
            let n = LOW + i as u8;
            ReceiverSpec {
                hz: crate::beam::midi_hz(n),
                decay: action::loss(n),
                damped_decay: 95.,
                pickup: [Self::slope(n, 0.), 0.],
            }
        });
        Self {
            panel: Panel::new(rate, load),
            receivers,
        }
    }
    fn slope(n: u8, offset: f32) -> f64 {
        let native = action::clearance(n);
        let z = action::neutral_offset(n);
        let g = native + 0.001 * f64::from(offset.clamp(-0.15, 1.));
        -0.000105 * native * native * 2. * z / (g * g + z * z).powi(2)
    }
}
impl Board for Harp {
    fn step(&mut self, emf: f64) -> f64 {
        self.panel.step(emf + self.receivers.tick()[0])
    }
    fn reset(&mut self) {
        self.panel.filter.reset();
        self.receivers.clear();
    }
    fn hold(&mut self, n: u8, down: bool) {
        self.receivers.held(n, down);
    }
    fn pedal(&mut self, down: bool) {
        self.receivers.pedal(down);
    }
    fn choke(&mut self, n: u8) {
        self.receivers.choke(n);
    }
    fn drive(&mut self, n: u8, v: f64) {
        self.receivers.drive(n, v);
    }
    fn ringing(&self) -> bool {
        self.receivers.ringing()
    }
    fn set_pickup_distance_mm(&mut self, offset: f32) {
        for i in 0..61 {
            self.receivers
                .set_pickup(i, [Self::slope(LOW + i as u8, offset), 0.]);
        }
    }
}
