//! Parallel coil bank and the two electrical volume controls on C 1004.
use crate::keyboard::Board;
use pleasant_dsp::circuit::{Circuit, CrossfadingFilter};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub volume: f64,
    pub expression: f64,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            volume: 1.,
            expression: 1.,
        }
    }
}
pub struct Panel {
    rate: f64,
    load: f64,
    c: Controls,
    network: CrossfadingFilter<3>,
}
impl Panel {
    pub fn new(rate: f64, load: f64) -> Self {
        assert!(rate.is_finite() && rate > 0. && load.is_finite() && load > 0.);
        let mut p = Self {
            rate,
            load,
            c: Controls::default(),
            network: CrossfadingFilter::new(rate),
        };
        p.compile();
        p
    }
    pub fn set(&mut self, c: Controls) {
        let c = Controls {
            volume: c.volume.clamp(0., 1.),
            expression: c.expression.clamp(0., 1.),
        };
        if self.c != c {
            self.c = c;
            self.compile();
        }
    }
    fn compile(&mut self) {
        // 73 armatures / 3 per pole face -> 25 parallel sources in the Fig. 6
        // equivalent. Coil impedance, pot tracks and tone capacitance are estimates.
        let mut n = Circuit::<3>::new(4, self.rate);
        let coils = 25.;
        let r = 9200. / coils;
        n.input(1, 1. / r / coils);
        n.r(1, 0, r);
        n.l(1, 2, 0.72 / coils);
        n.c(2, 0, 33e-9);
        n.r(2, 3, (100e3 * (1. - self.c.volume)).max(0.001));
        n.r(3, 0, (100e3 * self.c.volume).max(0.001));
        n.r(3, 4, (100e3 * (1. - self.c.expression)).max(0.001));
        n.r(4, 0, (100e3 * self.c.expression).max(0.001));
        n.r(4, 0, self.load);
        n.c(4, 0, 470e-12);
        n.compile(
            if self.c.volume == 0. || self.c.expression == 0. {
                0
            } else {
                4
            },
            self.network.prepare(),
        );
    }
    pub fn step(&mut self, emf: f64) -> f64 {
        self.network.tick(emf)
    }
}
impl Board for Panel {
    fn step(&mut self, emf: f64) -> f64 {
        self.step(emf)
    }
    fn reset(&mut self) {
        self.network.reset();
    }
}
