//! Passive series bank, volume, tone and output from the Japanese EP-61C drawing.
use pleasant_dsp::circuit::{Circuit, CrossfadingFilter};

#[derive(Clone, Copy, PartialEq)]
pub struct Controls {
    pub volume: f64,
    pub tone: f64,
    pub alignment_mm: f64,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            volume: 1.,
            tone: 1.,
            alignment_mm: 0.,
        }
    }
}
pub struct Panel {
    rate: f64,
    load: f64,
    pub(super) controls: Controls,
    filter: CrossfadingFilter<3>,
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
            tone: c.tone.clamp(0., 1.),
            alignment_mm: c.alignment_mm.clamp(-1., 1.),
        };
        let rebuild = c.volume != self.controls.volume || c.tone != self.controls.tone;
        self.controls = c;
        if rebuild {
            self.compile();
        }
    }
    fn compile(&mut self) {
        // Series copper and effective inductance are provisional. Cable C is
        // external. Both pot tracks and the 10 kΩ input resistor are drawn.
        // With the two 100 kΩ tracks loading the input, this effective
        // resistance yields the printed 15 kΩ at full controls and low frequency.
        let copper = 1. / (1. / 15000. - 2. / 100000.) - 10000.;
        let mut c = Circuit::<3>::new(5, self.rate);
        c.input(1, 1. / copper);
        c.r(1, 0, copper);
        c.l(1, 2, 61. * 0.002);
        c.r(2, 3, 10e3);
        let r = 100e3;
        let tone = self.controls.tone;
        let vol = self.controls.volume;
        // A milliohm represents endpoint contact. The capacitor stays on the
        // same physical wiper node when an automated control moves.
        c.r(3, 4, (r * tone).max(0.001));
        c.r(4, 0, (r * (1. - tone)).max(0.001));
        c.c(4, 0, 0.047e-6);
        c.r(3, 5, (r * (1. - vol)).max(0.001));
        c.r(5, 0, (r * vol).max(0.001));
        c.r(5, 0, self.load);
        c.c(5, 0, 470e-12);
        c.compile(
            if self.controls.volume == 0. { 0 } else { 5 },
            self.filter.prepare(),
        );
    }
    pub fn step(&mut self, emf: f64) -> f64 {
        self.filter.tick(emf)
    }
}
impl crate::keyboard::Board for Panel {
    fn reset(&mut self) {
        self.filter.reset();
    }
    fn step(&mut self, emf: f64) -> f64 {
        self.step(emf)
    }
}
