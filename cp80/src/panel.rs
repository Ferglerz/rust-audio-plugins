//! Small-signal reductions of the Yamaha CP-80 and CP-70B TR boards.
//! The three stages meet at high-impedance FET gates. See the two research decks.
use crate::network::{Circuit, Filter};
use pleasant_dsp::circuit::CrossfadingFilter;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Controls {
    pub volume: f64,
    pub bass: f64,
    pub middle: f64,
    pub treble: f64,
    pub brilliance: u8,
    pub tremolo: bool,
    pub frame_ring: bool,
    pub speed: f64,
    pub depth: f64,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            volume: 1.,
            bass: 0.5,
            middle: 0.5,
            treble: 0.5,
            brilliance: 1,
            tremolo: false,
            frame_ring: false,
            speed: 0.75,
            depth: 0.6,
        }
    }
}
fn parallel(a: f64, b: f64) -> f64 {
    a * b / (a + b)
}

// Midband load seen by one 22 kΩ tremolo feed. FET6/7 and FET9/8
// each have a 180k/220k bias divider. VR1 feeds the latter at its wiper.
pub(crate) fn optical_load(volume: f64) -> f64 {
    let gate = parallel(180e3, 220e3);
    let lower = 100e3 * volume.clamp(0., 1.).powi(3);
    1. / gate + 1. / (100e3 - lower + parallel(lower, gate))
}

fn taper(x: f64) -> f64 {
    x.clamp(0., 1.).powf(-0.30_f64.log2())
}

enum FrontEnd {
    Cp80 {
        tone: Box<CrossfadingFilter<7>>,
        amplifier: Filter<8>,
    },
    Cp70B {
        tone: CrossfadingFilter<4>,
        amplifier: Filter<7>,
    },
}

pub struct Panel {
    front: FrontEnd,
    jack: Filter<5>,
    old_jack: Filter<5>,
    jack_volume: f64,
    fade_remaining: u32,
    fade_length: u32,
    started: bool,
    rate: f64,
    controls: Controls,
    input_z: f64,
    hp: f64,
}
impl Panel {
    pub fn new(rate: f64) -> Self {
        Self::with_front(
            rate,
            FrontEnd::Cp80 {
                tone: Box::new(CrossfadingFilter::new(rate)),
                amplifier: Filter::default(),
            },
        )
    }
    pub fn cp70b(rate: f64) -> Self {
        Self::with_front(
            rate,
            FrontEnd::Cp70B {
                tone: CrossfadingFilter::new(rate),
                amplifier: Filter::default(),
            },
        )
    }
    fn with_front(rate: f64, front: FrontEnd) -> Self {
        let total = 6e-9 + parallel(82e-9, 22e-9);
        let series = parallel(total, 100e-9);
        let mut p = Self {
            front,
            jack: Default::default(),
            old_jack: Default::default(),
            jack_volume: 1.,
            fade_remaining: 0,
            fade_length: (rate * 0.005).round().max(1.) as u32,
            started: false,
            rate,
            controls: Controls::default(),
            input_z: 0.,
            hp: 1. / (1. + 2. * rate * 470e3 * series),
        };
        p.build_tone();
        p.build_amplifier();
        p.build_jack();
        p
    }
    pub fn set(&mut self, c: Controls) {
        let old = self.controls;
        self.controls = c;
        if (old.bass, old.middle, old.treble, old.brilliance)
            != (c.bass, c.middle, c.treble, c.brilliance)
        {
            self.build_tone();
        }
        if self.fade_remaining == 0 {
            self.update_volume();
        }
    }
    fn update_volume(&mut self) {
        if self.jack_volume != self.controls.volume {
            if self.started {
                self.old_jack.clone_from(&self.jack);
                self.fade_remaining = self.fade_length;
            }
            self.jack_volume = self.controls.volume;
            self.build_jack();
        }
    }
    pub fn reset(&mut self) {
        self.started = false;
        self.fade_remaining = 0;
        self.update_volume();
        self.input_z = 0.;
        match &mut self.front {
            FrontEnd::Cp80 { tone, amplifier } => {
                tone.reset();
                amplifier.reset();
            }
            FrontEnd::Cp70B { tone, amplifier } => {
                tone.reset();
                amplifier.reset();
            }
        }
        self.jack.reset();
    }
    fn build_tone(&mut self) {
        let c = self.controls;
        let tone = match &mut self.front {
            FrontEnd::Cp80 { tone, .. } => tone,
            FrontEnd::Cp70B { tone, .. } => {
                let mut n = Circuit::<4>::new(8, self.rate);
                Self::tone_input(&mut n, c);
                n.c(1, 5, 2.2e-9);
                n.r(5, 6, 3300.);
                n.r(6, 7, 0.1 + 10e3 * (1. - c.treble));
                n.r(7, 3, 0.1 + 10e3 * c.treble);
                n.c(7, 8, 100e-9);
                n.r(8, 0, 470e3 + parallel(47e3, 5600.));
                n.compile(8, tone.prepare());
                return;
            }
        };
        let mut n = Circuit::<7>::new(11, self.rate);
        Self::tone_input(&mut n, c);
        let mid = taper(c.middle);
        n.r(5, 0, 0.1 + 1e3 * (1. - mid));
        n.c(1, 6, 2.2e-9);
        n.r(6, 7, 3300.);
        n.c(7, 0, 470e-12);
        n.r(7, 8, 0.1 + 10e3 * (1. - c.treble));
        n.r(8, 3, 0.1 + 10e3 * c.treble);
        n.r(8, 9, 47e3);
        n.r(9, 3, 4700.);
        let selected = [9, 8, 7][usize::from(c.brilliance.min(2))];
        n.r(selected, 10, 0.01);
        if c.brilliance == 0 {
            n.r(5, 4, 0.01);
        } else if c.brilliance == 1 {
            n.r(5, 0, 0.01);
        }
        n.c(10, 0, 100e-12);
        n.c(10, 11, 100e-9);
        n.c(11, 0, 330e-12);
        n.r(11, 0, 470e3 + parallel(47e3, 5600.));
        n.compile(11, tone.prepare());
    }
    fn tone_input<const N: usize>(n: &mut Circuit<N>, c: Controls) {
        // FET1 with 1k source degeneration driving R104 and the passive network.
        let gm = 0.001640590;
        let gds = 0.000009387152;
        let deg = 1. + 1000. * (gm + gds);
        n.r(1, 0, 1. / (0.0001 + gds / deg));
        n.input(1, -gm / deg);
        n.r(1, 2, 10e3);
        n.c(2, 3, 470e-9);
        n.c(2, 4, 470e-9);
        n.r(4, 0, 3900.);
        n.r(3, 4, 0.1 + 50e3 * taper(c.bass));
        let mid = taper(c.middle);
        n.r(4, 0, 0.1 + 1e3 * mid);
    }
    fn build_amplifier(&mut self) {
        match &mut self.front {
            FrontEnd::Cp80 { amplifier, .. } => Self::amplifier(self.rate, amplifier, false),
            FrontEnd::Cp70B { amplifier, .. } => Self::amplifier(self.rate, amplifier, true),
        }
    }
    fn amplifier<const N: usize>(rate: f64, output: &mut Filter<N>, cp70b: bool) {
        let mut n = Circuit::<N>::new(11, rate);
        // FET2 common-source stage, followed by FET3 and the active RC feedback loop.
        let gm = 0.001739630;
        let gds = 0.00001071098;
        n.gm(1, 2, 0, gm);
        n.input(1, -gm);
        n.input(2, gm);
        n.r(1, 2, 1. / gds);
        n.r(1, 0, 8200.);
        n.r(2, 0, 2200.);
        n.c(2, 0, 220e-6);
        n.c(1, 3, 47e-9);
        n.r(3, 4, 39e3);
        n.r(4, 5, 39e3);
        n.c(4, 6, 2.2e-9);
        n.c(5, 0, if cp70b { 1e-9 } else { 470e-12 });
        n.r(5, 0, parallel(180e3, 220e3));
        n.gm(0, 6, 5, 0.000986167);
        n.r(6, 0, parallel(27e3, 1. / 0.000003306382));
        if !cp70b {
            n.c(6, 0, 330e-12);
        }
        n.c(6, 7, 1e-6);
        let trim = if cp70b { 0.883563 } else { 0.89488 };
        n.r(7, 8, 0.1 + 10e3 * (1. - trim));
        n.r(8, 0, 0.1 + 10e3 * trim);
        n.c(8, 9, 1e-6);
        n.r(9, 0, 100e3);
        n.r(9, 10, 660.);
        n.r(10, 0, 100e3);
        n.c(10, 11, 100e-9);
        n.r(11, 0, 470e3 + parallel(47e3, 2200.));
        n.compile(11, output);
    }
    fn build_jack(&mut self) {
        let mut n = Circuit::<5>::new(11, self.rate);
        let gm = 0.001977556;
        let gds = 0.00001325144;
        let deg = 1. + 1000. * (gm + gds);
        n.r(1, 0, 1. / (1. / 5600. + gds / deg));
        n.input(1, -gm / deg);
        n.c(1, 2, 100e-9);
        n.r(2, 0, parallel(180e3, 220e3));
        n.gm(0, 3, 2, 0.000986167);
        n.r(3, 0, parallel(27e3, 1. / 0.000003306382));
        n.c(3, 4, 1e-6);
        n.r(4, 0, 100e3);
        // Both branches see the same signal at rest. Fold them into one by
        // doubling admittances up to their non-loading buffer gates.
        n.r(4, 5, 11e3);
        n.r(5, 0, 11343. / 2.);
        n.c(5, 6, 200e-9);
        n.r(6, 0, parallel(180e3, 220e3) / 2.);
        let v = self.controls.volume.clamp(0., 1.).powi(3);
        n.r(5, 7, (0.1 + 100e3 * (1. - v)) / 2.);
        n.r(7, 0, (0.1 + 100e3 * v) / 2.);
        n.c(7, 8, 200e-9);
        n.r(8, 0, parallel(180e3, 220e3) / 2.);
        n.gm(0, 9, 8, 0.000986167);
        n.r(9, 0, parallel(27e3, 1. / 0.000003306382));
        n.c(9, 10, 1e-6);
        n.r(10, 0, 100e3);
        n.r(10, 11, 330.);
        n.r(11, 0, 100e3);
        n.compile(11, &mut self.jack);
    }
    /// Inputs use the open-circuit pickup voltage associated with each force.
    pub fn tick(&mut self, lower: f64, upper: f64) -> f64 {
        let signal = match &mut self.front {
            FrontEnd::Cp80 { tone, amplifier } => {
                let split = 22. / 104.;
                let x = (lower * split + upper) / (6. + 82. * split);
                let low = (x - self.input_z) * self.hp;
                let high = x - (low + self.input_z);
                self.input_z += 2. * low;
                amplifier.tick(tone.tick(high))
            }
            // The CP-70B's three-branch bank has already solved the FET1 gate voltage.
            FrontEnd::Cp70B { tone, amplifier } => amplifier.tick(tone.tick(lower)),
        };
        self.started = true;
        let jack = self.jack.tick(signal);
        if self.fade_remaining == 0 {
            return jack;
        }
        let previous = self.old_jack.tick(signal);
        let mix = f64::from(self.fade_remaining) / f64::from(self.fade_length);
        self.fade_remaining -= 1;
        if self.fade_remaining == 0 {
            // Coalesce faster automation until both capacitor histories have
            // completed this transition, then move to the most recent wiper.
            self.update_volume();
        }
        jack + mix * (previous - jack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_jack_wipers_starts_from_the_playing_signal_and_applies_latest_position() {
        let mut moving = Panel::new(192000.);
        let mut fixed = Panel::new(192000.);
        let tone = |i: usize| (std::f64::consts::TAU * 500. * i as f64 / 192000.).sin();
        for i in 0..12000 {
            moving.tick(tone(i), 0.);
            fixed.tick(tone(i), 0.);
        }
        moving.set(Controls {
            volume: 0.4,
            ..Default::default()
        });
        assert!((moving.tick(tone(12000), 0.) - fixed.tick(tone(12000), 0.)).abs() < 1e-12);
        moving.set(Controls {
            volume: 0.,
            ..Default::default()
        });
        for i in 12001..60000 {
            let sample = moving.tick(tone(i), 0.);
            assert!(sample.is_finite());
            if i > 58000 {
                assert!(sample.abs() < 1e-6);
            }
        }
        assert_eq!(moving.jack_volume, 0.);
        assert_eq!(moving.fade_remaining, 0);
    }
}
