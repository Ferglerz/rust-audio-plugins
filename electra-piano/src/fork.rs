//! Two coupled prong-tip coordinates and two higher bending modes of the
//! lighter prong. Suspension flexibility enters through the coupling spring.
use crate::circuit::winding_sign;
use pleasant_dsp::{
    beam::{beam_shape, BeamMode},
    filters::{BiquadCoefficients, BiquadKind, TransposedDirectForm2},
};
use std::f64::consts::TAU;

#[derive(Clone, Copy, Debug)]
pub struct ForkSpec {
    pub note: u8,
    pub tip_mass: f64,
    pub return_mass: f64,
    pub return_ratio: f64,
    pub coupling: f64,
    pub hammer_mass: f64,
    pub contact_k: f64,
    pub field_width: f64,
    pub magnet_polarity: f64,
    pub ring_t60: f64,
    pub damped_t60: f64,
}

pub const KEYS: [ForkSpec; 3] = [
    ForkSpec {
        note: 48,
        tip_mass: 0.0040,
        return_mass: 0.020,
        return_ratio: 1.8,
        coupling: 0.20,
        hammer_mass: 0.0060,
        contact_k: 2.0e6,
        field_width: 0.00082,
        magnet_polarity: 1.,
        ring_t60: 6.8,
        damped_t60: 0.15,
    },
    ForkSpec {
        note: 65,
        tip_mass: 0.0018,
        return_mass: 0.010,
        return_ratio: 2.2,
        coupling: 0.18,
        hammer_mass: 0.0045,
        contact_k: 3.4e6,
        field_width: 0.00064,
        magnet_polarity: 1.,
        ring_t60: 5.0,
        damped_t60: 0.11,
    },
    ForkSpec {
        note: 80,
        tip_mass: 0.0005,
        return_mass: 0.004,
        return_ratio: 2.5,
        coupling: 0.15,
        hammer_mass: 0.0022,
        contact_k: 7.0e6,
        field_width: 0.00071,
        magnet_polarity: 1.,
        ring_t60: 1.7,
        damped_t60: 0.080,
    },
];

#[derive(Clone, Copy, Default, Debug)]
pub struct Motion {
    pub sensed_m: f64,
    pub return_m: f64,
    pub contact_n: f64,
    pub pickup_v: f64,
}

pub struct Fork {
    spec: ForkSpec,
    modes: [BeamMode; 4],
    held: bool,
    sustain: bool,
    hammer: Option<(f64, f64)>,
    damper: f64,
    damper_step: f64,
    damper_loss: f64,
    dt: f64,
    internal_rate: f64,
    inverse_width: f64,
    gap_squared: f64,
    substeps: usize,
    antialias_coefficients: [BiquadCoefficients; 3],
    antialias: [TransposedDirectForm2; 3],
    previous_flux: f64,
}

impl Fork {
    pub fn new(spec: ForkSpec, rate: f64) -> Self {
        assert!(rate.is_finite() && rate >= 8000.);
        let substeps = (176400. / rate).ceil().max(4.) as usize;
        let subrate = substeps as f64 * rate;
        let fundamental = 440. * 2_f64.powf((f64::from(spec.note) - 69.) / 12.);
        // Mass-normalized stiffness of both offset arms and a compliant root.
        // The common scale is fixed by the *lower coupled* eigenfrequency.
        let m = spec.tip_mass;
        let r = spec.return_mass;
        let a = 1. + spec.coupling;
        let d = spec.return_ratio.powi(2) + spec.coupling * m / r;
        let b = -spec.coupling * (m / r).sqrt();
        let delta = (d - a).hypot(2. * b);
        let lo = (a + d - delta) * 0.5;
        let hi = (a + d + delta) * 0.5;
        let lower = [b, lo - a];
        let norm = lower[0].hypot(lower[1]);
        let (u, v) = (lower[0] / norm, lower[1] / norm);
        let hit = 0.55;
        let sense = 0.91;
        let prong = |ratio: f64, t60: f64, u: f64, v: f64| {
            BeamMode::new(
                fundamental * ratio,
                subrate,
                t60,
                beam_shape(1.875104, hit) * u / m.sqrt(),
                beam_shape(1.875104, sense) * u / m.sqrt(),
                v / r.sqrt(),
            )
        };
        let bending = |beta: f64, loss: f64| {
            let f = fundamental * (beta / 1.875104).powi(2) / lo.sqrt();
            let pass = if f < 0.44 * rate { 1. } else { 0. };
            BeamMode::new(
                f,
                subrate,
                spec.ring_t60 * loss,
                pass * beam_shape(beta, hit) / m.sqrt(),
                beam_shape(beta, sense) / m.sqrt(),
                0.,
            )
        };
        let modes = [
            prong(1., spec.ring_t60, u, v),
            prong((hi / lo).sqrt(), spec.ring_t60 * 0.24, -v, u),
            bending(4.694091, 0.10),
            bending(7.854757, 0.041),
        ];
        Self {
            spec,
            modes,
            held: false,
            sustain: false,
            hammer: None,
            damper: 1.,
            damper_step: 1. / (subrate * 0.006),
            damper_loss: 1. - 10_f64.powf(-3. / (subrate * spec.damped_t60)),
            dt: 1. / subrate,
            internal_rate: subrate,
            inverse_width: 1. / spec.field_width,
            gap_squared: 1.,
            substeps,
            antialias_coefficients: [1., 3., 5.].map(|pole| {
                let q = 0.5 / (pole * TAU / 24.).cos();
                BiquadCoefficients::design(BiquadKind::HighCut, 0.39 * rate, 0., q, subrate)
            }),
            antialias: [TransposedDirectForm2::default(); 3],
            previous_flux: 0.,
        }
    }

    /// Hammer already at first contact. Existing fork displacement and
    /// momentum survive a second strike. Impact speed is measured in m/s.
    pub fn strike(&mut self, speed: f64) {
        if !speed.is_finite() || speed <= 0. {
            return;
        }
        self.held = true;
        self.damper = 0.;
        self.hammer = Some((
            self.modes.iter().map(|m| m.hit_displacement()).sum(),
            speed.min(3.),
        ));
    }

    pub fn release(&mut self) {
        self.held = false;
    }
    pub fn sustain(&mut self, down: bool) {
        self.sustain = down;
    }

    pub(crate) fn clear(&mut self) {
        for mode in &mut self.modes {
            mode.clear();
        }
        self.previous_flux = 0.;
        self.antialias.fill(TransposedDirectForm2::default());
        self.held = false;
        self.sustain = false;
        self.damper = 1.;
        self.hammer = None;
    }

    /// Conservative envelope of observable prong speed. Inspecting both
    /// quadratures prevents retirement at a zero crossing.
    pub(crate) fn at_rest(&self) -> bool {
        self.hammer.is_none() && self.modes.iter().all(|m| m.speed_bound() < 1e-10)
    }

    /// Radial pickup setback relative to the fitted nominal field width.
    /// Rebase linked flux at the current tip position when moving the virtual
    /// pickup, so parameter automation does not create an artificial impulse.
    pub fn set_pickup_gap_mm(&mut self, offset: f64) {
        if !offset.is_finite() {
            return;
        }
        let gap = 1. + offset.clamp(-0.25, 2.) * 0.001 * self.inverse_width;
        let squared = gap * gap;
        if self.gap_squared != squared {
            self.gap_squared = squared;
            let x = self.modes.iter().map(|m| m.sensed_displacement()).sum();
            self.previous_flux = self.flux(x);
        }
    }

    fn flux(&self, x: f64) -> f64 {
        let field =
            |v: f64| 1. / (self.gap_squared + (0.65 - v * self.inverse_width).powi(2)).sqrt();
        self.spec.magnet_polarity * 0.018 * self.spec.field_width * (field(x) - field(0.))
    }

    pub fn tick(&mut self) -> Motion {
        let mut peak: f64 = 0.;
        let mut emf = 0.;
        let mut sensed_m = 0.;
        for _ in 0..self.substeps {
            let force = if let Some((x, v)) = self.hammer {
                let hit: f64 = self.modes.iter().map(|m| m.hit_displacement()).sum();
                let penetration = x - hit;
                if penetration < 0. {
                    self.hammer = None;
                    0.
                } else {
                    let force = self.spec.contact_k * penetration * penetration.sqrt();
                    let next_v = v - force * self.dt / self.spec.hammer_mass;
                    self.hammer = Some((x + next_v * self.dt, next_v));
                    force
                }
            } else {
                0.
            };
            peak = peak.max(force);
            if self.held || self.sustain {
                self.damper = (self.damper - self.damper_step).max(0.);
            } else {
                self.damper = (self.damper + self.damper_step).min(1.);
            }
            let damping = 1. - self.damper * self.damper_loss;
            for m in &mut self.modes {
                m.step(force * self.dt, damping);
            }
            sensed_m = self
                .modes
                .iter()
                .map(|m| m.sensed_displacement())
                .sum::<f64>();
            // Bounded flux linkage with an offset pole. Both the magnetic
            // nonlinearity and its anti-alias filter run at the mechanical rate.
            let flux = self.flux(sensed_m);
            emf = -(flux - self.previous_flux) * self.internal_rate;
            self.previous_flux = flux;
            for (section, coeff) in self.antialias.iter_mut().zip(self.antialias_coefficients) {
                emf = section.process(emf, coeff);
            }
        }
        let return_m = self.modes.iter().map(|m| m.return_displacement()).sum();
        Motion {
            sensed_m,
            return_m,
            contact_n: peak,
            pickup_v: winding_sign(self.spec.note.clamp(crate::live::LOW, crate::live::HIGH)) * emf,
        }
    }
}
