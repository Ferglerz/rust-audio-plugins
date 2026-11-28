//! Passive mono volume, bass cut and treble cut. Component values are estimates.
use pleasant_dsp::circuit::{Circuit, CrossfadingFilter};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub volume: f64,
    pub bass: f64,
    pub treble: f64,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            volume: 1.,
            bass: 1.,
            treble: 1.,
        }
    }
}
#[derive(Clone, Copy, Default, Debug)]
pub struct Output {
    pub pickup_v: f64,
    pub harp_v: f64,
    pub jack_v: f64,
}

pub struct Panel {
    n: CrossfadingFilter<5, 1, 2>,
    controls: Controls,
    rate: f64,
}
impl Panel {
    pub fn new(rate: f64, controls: Controls) -> Self {
        let mut p = Self {
            n: CrossfadingFilter::new(rate),
            controls: Controls::default(),
            rate,
        };
        p.compile();
        p.set_controls(controls);
        p
    }
    fn compile(&mut self) {
        let mut n = Circuit::<5>::new(5, self.rate);
        // Equivalent series winding R and L, plus winding capacitance.
        n.input(1, 1. / 2500.);
        n.r(1, 0, 2500.);
        n.l(1, 2, 0.90);
        n.c(2, 0, 150e-12);
        // Finite contacts retain each capacitor state during pot movement.
        n.r(2, 3, 1. + 500e3 * (1. - self.controls.bass).powi(2));
        n.c(2, 3, 2.2e-9);
        n.r(3, 4, 1. + 500e3 * self.controls.treble.powi(2));
        n.c(4, 0, 22e-9);
        let vol = self.controls.volume.powf(2.5);
        n.r(3, 5, 1. + 250e3 * (1. - vol));
        n.r(5, 0, 1. + 250e3 * vol);
        n.r(5, 0, 1e6);
        n.c(5, 0, 470e-12);
        n.compile_ports([2, if vol == 0. { 0 } else { 5 }], self.n.prepare());
    }
    pub fn set_controls(&mut self, c: Controls) {
        let bound = |v: f64, old: f64| if v.is_finite() { v.clamp(0., 1.) } else { old };
        let c = Controls {
            volume: bound(c.volume, self.controls.volume),
            bass: bound(c.bass, self.controls.bass),
            treble: bound(c.treble, self.controls.treble),
        };
        if c != self.controls {
            self.controls = c;
            self.compile();
        }
    }
    pub fn tick(&mut self, v: f64) -> Output {
        let [harp_v, jack_v] = self.n.tick_ports([v]);
        Output {
            pickup_v: v,
            harp_v,
            jack_v,
        }
    }
    pub fn reset(&mut self) {
        self.n.reset();
    }
    pub(crate) fn quiet(&self) -> bool {
        self.n.history_below(1e-9)
    }
}
