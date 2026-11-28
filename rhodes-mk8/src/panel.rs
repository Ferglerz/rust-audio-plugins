//! MK8 S documented signal order, diode drive, resonant equalizer and stereo Vari-Pan.
use pleasant_dsp::{
    circuit::{Circuit, Filter},
    filters::{BiquadCoefficients as Coeff, BiquadKind, TransposedDirectForm2 as Biquad},
};
use std::f64::consts::TAU;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Wave {
    #[default]
    Square,
    Ramp,
    Triangle,
    Sine,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub volume: f64,
    pub drive: f64,
    pub envelope: f64,
    pub low_db: f64,
    pub mid_db: f64,
    pub mid_hz: f64,
    pub high_db: f64,
    pub resonance: f64,
    pub pan_enabled: bool,
    pub pan_rate_hz: f64,
    pub pan_depth: f64,
    pub wave: Wave,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            volume: 1.,
            drive: 0.,
            envelope: 0.,
            low_db: 0.,
            mid_db: 0.,
            mid_hz: 480.,
            high_db: 0.,
            resonance: 0.35,
            pan_enabled: false,
            pan_rate_hz: 4.,
            pan_depth: 0.75,
            wave: Wave::Square,
        }
    }
}
impl Controls {
    fn bounded(self, old: Self) -> Self {
        let d = |x: f64, lo: f64, hi: f64, prev: f64| {
            if x.is_finite() {
                x.clamp(lo, hi)
            } else {
                prev
            }
        };
        Self {
            volume: d(self.volume, 0., 1., old.volume),
            drive: d(self.drive, 0., 1., old.drive),
            envelope: d(self.envelope, 0., 1., old.envelope),
            low_db: d(self.low_db, -15., 15., old.low_db),
            mid_db: d(self.mid_db, -15., 15., old.mid_db),
            mid_hz: d(self.mid_hz, 100., 2000., old.mid_hz),
            high_db: d(self.high_db, -15., 15., old.high_db),
            resonance: d(self.resonance, 0., 1., old.resonance),
            pan_rate_hz: d(self.pan_rate_hz, 0.1, 3000., old.pan_rate_hz),
            pan_depth: d(self.pan_depth, 0., 1., old.pan_depth),
            pan_enabled: self.pan_enabled,
            wave: self.wave,
        }
    }
}
#[derive(Clone, Copy, Default)]
pub struct Outputs {
    pub send_v: f64,
    pub left_v: f64,
    pub right_v: f64,
}

pub struct Panel {
    c: Controls,
    target: Controls,
    ramp: pleasant_dsp::interpolation::ControlRamp,
    rate: f64,
    source: Filter<2>,
    low: Coeff,
    mid: Coeff,
    high: Coeff,
    eq: [Biquad; 3],
    envelope: f64,
    attack: f64,
    recovery: f64,
    phase: f64,
    clock: u8,
    diode: Vec<f64>,
    fx: crate::fx::Board,
}
impl Panel {
    /// Supply is 15 V externally. The published manual specifies the functional
    /// blocks; actual rail conversion, component Q values and coil impedance need measurement.
    pub fn new(rate: f64, c: Controls) -> Self {
        let mut src = Circuit::<2>::new(2, rate);
        src.input(1, 1. / 1500.);
        src.r(1, 0, 1500.);
        src.l(1, 2, 0.65);
        src.c(2, 0, 470e-12);
        src.r(2, 0, 1e6);
        let mut source = Filter::default();
        src.compile(2, &mut source);
        // Positive half of the implicit anti-parallel diode circuit. R=4.7k,
        // Is=2nA, nVt=44mV are assumed. Large drives are handled by bisection.
        let diode = (0..=4096)
            .map(|i| {
                let vin = 64. * i as f64 / 4096.;
                let (mut lo, mut hi) = (0., vin.min(1.3));
                for _ in 0..40 {
                    let v = (lo + hi) * 0.5;
                    let e = v + 4700. * 4e-9 * (v / 0.044).sinh() - vin;
                    if e > 0. {
                        hi = v
                    } else {
                        lo = v
                    }
                }
                0.5 * (lo + hi)
            })
            .collect();
        let mut p = Self {
            c: Controls::default(),
            target: Controls::default(),
            ramp: pleasant_dsp::interpolation::ControlRamp::new(rate),
            rate,
            source,
            low: Coeff::IDENTITY,
            mid: Coeff::IDENTITY,
            high: Coeff::IDENTITY,
            eq: [Biquad::default(); 3],
            envelope: 0.,
            attack: 1. - (-1. / (0.004 * rate)).exp(),
            recovery: 1. - (-1. / (0.082 * rate)).exp(),
            phase: 0.,
            clock: 0,
            diode,
            fx: crate::fx::Board::new(rate),
        };
        p.set_controls(c);
        p
    }
    pub fn set_fx_controls(&mut self, c: crate::fx::Controls) {
        self.fx.set(c);
    }
    pub fn effects_sounding(&self) -> bool {
        self.fx.tail_active()
    }
    pub fn set_controls(&mut self, c: Controls) {
        let c = c.bounded(self.target);
        if self.target == c {
            return;
        }
        self.target = c;
        self.c.pan_enabled = c.pan_enabled;
        self.c.wave = c.wave;
        if self.ramp.retarget() {
            self.apply(c);
        }
    }
    fn apply(&mut self, c: Controls) {
        if self.c.low_db != c.low_db {
            self.low = Coeff::design(BiquadKind::Bell, 75., c.low_db, 0.75, self.rate);
        }
        if self.c.high_db != c.high_db {
            self.high = Coeff::design(BiquadKind::Bell, 3000., c.high_db, 0.78, self.rate);
        }
        let changed = (
            self.c.envelope,
            self.c.mid_hz,
            self.c.mid_db,
            self.c.resonance,
        ) != (c.envelope, c.mid_hz, c.mid_db, c.resonance);
        self.c = c;
        if changed {
            self.update_mid();
        }
    }
    fn update_mid(&mut self) {
        // An envelope scales the control voltage on the resonant mid circuit.
        let sweep = 1. + self.c.envelope * (self.envelope / 0.20).min(1.) * 7.;
        let f = (self.c.mid_hz * sweep).min(2000.);
        self.mid = Coeff::design(
            BiquadKind::Bell,
            f,
            self.c.mid_db,
            0.5 + 4. * self.c.resonance.powi(2),
            self.rate,
        );
    }
    pub fn reset(&mut self) {
        self.fx.clear();
        self.ramp.reset();
        self.apply(self.target);
        self.envelope = 0.;
        self.clock = 0;
        self.phase = 0.;
        self.source.reset();
        for f in &mut self.eq {
            f.reset();
        }
        self.update_mid();
    }
    pub fn send(&mut self, emf: f64) -> f64 {
        self.source.tick(emf)
    }
    /// The switched return feeds the panel independently from the high-impedance pickup tap.
    pub fn process_return(&mut self, input: f64) -> [f64; 2] {
        if let Some(step) = self.ramp.step() {
            let mut c = self.c;
            c.volume += step * (self.target.volume - c.volume);
            c.drive += step * (self.target.drive - c.drive);
            c.envelope += step * (self.target.envelope - c.envelope);
            c.low_db += step * (self.target.low_db - c.low_db);
            c.mid_db += step * (self.target.mid_db - c.mid_db);
            c.mid_hz += step * (self.target.mid_hz - c.mid_hz);
            c.high_db += step * (self.target.high_db - c.high_db);
            c.resonance += step * (self.target.resonance - c.resonance);
            c.pan_rate_hz += step * (self.target.pan_rate_hz - c.pan_rate_hz);
            c.pan_depth += step * (self.target.pan_depth - c.pan_depth);
            self.apply(c);
        }
        let c = self.c;
        let input = self.fx.compress(input);
        let a = input.abs();
        self.envelope += (if a > self.envelope {
            self.attack
        } else {
            self.recovery
        }) * (a - self.envelope);
        let mut x = input;
        if c.drive > 0. {
            let vin = input * (1. + 35. * c.drive * c.drive);
            let ix = (vin.abs() * 64.).min(4095.999);
            let i = ix as usize;
            x = (self.diode[i] + (ix - i as f64) * (self.diode[i + 1] - self.diode[i]))
                .copysign(vin);
        }
        x *= self.fx.drive_out;
        if c.envelope > 0. && c.mid_db != 0. && self.clock == 0 {
            self.update_mid();
        }
        self.clock = (self.clock + 1) & 15;
        for (section, coef) in self.eq.iter_mut().zip([self.low, self.mid, self.high]) {
            x = section.process(x, coef);
        }
        self.phase = (self.phase + c.pan_rate_hz / self.rate).fract();
        let u = self.phase;
        let v = if c.pan_enabled {
            (match c.wave {
                Wave::Square => {
                    if u < 0.5 {
                        1.
                    } else {
                        -1.
                    }
                }
                Wave::Triangle => 1. - 4. * (u - 0.5).abs(),
                Wave::Ramp => 2. * u - 1.,
                Wave::Sine => (TAU * u).sin(),
            }) * c.pan_depth
        } else {
            0.
        };
        // Estimated usable jack swing on the published 15 V external supply.
        // Drive and overlapping EQ bands can otherwise exceed the output stage.
        let stereo = self.fx.effects(x);
        let output = |x: f64, gain: f64| c.volume * c.volume * (x * gain).clamp(-6.5, 6.5);
        [output(stereo[0], 1. + v), output(stereo[1], 1. - v)]
    }
    pub fn tick(&mut self, emf: f64) -> Outputs {
        let send_v = self.send(emf);
        let [left_v, right_v] = self.process_return(send_v);
        Outputs {
            send_v,
            left_v,
            right_v,
        }
    }
}
