//! ABS hammer and four tip zones striking a spring-loaded, tapered steel tine.
use crate::{field::FlatPole, modes::MODES};
use pleasant_dsp::beam::BeamMode;
pub const LOW: u8 = 28;
pub const HIGH: u8 = 100;

pub fn zone(n: u8) -> usize {
    match n {
        0..=45 => 0,
        46..=61 => 1,
        62..=80 => 2,
        _ => 3,
    }
}
#[derive(Clone, Copy, Default)]
pub struct Motion {
    pub tine_mm: f64,
    pub lateral_mm: f64,
    pub contact_n: f64,
    pub emf_v: f64,
}
pub struct Action {
    vertical: [BeamMode; 5],
    side: BeamMode,
    pole: &'static FlatPole,
    dt: f64,
    height: f64,
    gap: f64,
    offset: f64,
    last_flux: f64,
    hammer: Option<(f64, f64)>,
    mass: f64,
    contact: f64,
    held: bool,
    pedal: bool,
    damper: f64,
    damper_step: f64,
    damper_loss: f64,
}
impl Action {
    pub fn new(n: u8, rate: f64, host_rate: f64) -> Self {
        let edge = n.clamp(LOW, HIGH);
        let reg = f64::from(edge - LOW) / 72.;
        let f0 = 440. * 2f64.powf((f64::from(n) - 69.) / 12.);
        let specs = MODES[usize::from(edge - LOW)];
        let t60 = 12. * 0.25_f64.powf(reg);
        let make = |f: f64, t: f64, h: f64, tine: f64, ret: f64| {
            BeamMode::new(
                f,
                rate,
                t,
                if f < 0.46 * host_rate { h } else { 0. },
                tine,
                ret,
            )
        };
        // A much heavier tonebar near unison couples to the primary through
        // the tone block. The symmetric pair shares the tine mobility. The
        // weaker branch stores energy and returns it during a long note.
        let angle = 0.27_f64;
        let (s, c) = angle.sin_cos();
        let [_, h, v] = specs[0];
        let bar = v / (18. + reg * 12.).sqrt();
        let bending = |i: usize, loss: f64| {
            let [q, h, v] = specs[i];
            make(f0 * q, t60 * loss, h, v, 0.)
        };
        let vertical = [
            make(f0, t60, h * c, v * c, bar * s),
            make(f0 + 0.46 + 0.55 * reg, t60 * 0.74, -h * s, -v * s, bar * c),
            bending(1, 0.072),
            bending(2, 0.040),
            bending(3, 0.022),
        ];
        let side = make(f0 * 1.0007, t60 * 0.70, h * 0.06, v, 0.);
        Self {
            vertical,
            side,
            pole: FlatPole::shared(),
            dt: 1. / rate,
            height: 1.78 - 0.72 * reg,
            gap: 1.87 - 0.32 * reg,
            offset: 0.,
            last_flux: 0.,
            hammer: None,
            mass: 0.007 * 0.55_f64.powf(reg),
            contact: [2.6e6, 4.0e6, 7.6e6, 1.4e7][zone(edge)],
            held: false,
            pedal: false,
            damper: 1.,
            damper_step: 1. / (rate * 0.004),
            damper_loss: 1. - 10_f64.powf(-3. / (rate * (0.080 - 0.052 * reg))),
        }
    }
    fn at_hammer(&self) -> f64 {
        self.vertical
            .iter()
            .map(BeamMode::hit_displacement)
            .sum::<f64>()
            + self.side.hit_displacement()
    }
    fn motion(&self) -> (f64, f64) {
        (
            1000.
                * self
                    .vertical
                    .iter()
                    .map(BeamMode::sensed_displacement)
                    .sum::<f64>(),
            1000. * self.side.sensed_displacement(),
        )
    }
    fn flux(&self, x: f64, y: f64) -> f64 {
        let g = self.gap + self.offset;
        let r = (self.height + x).hypot(y);
        self.pole.linkage(r, g) - self.pole.linkage(self.height, g)
    }
    pub fn set_pickup_gap_mm(&mut self, offset: f64) {
        if !offset.is_finite() {
            return;
        }
        let mm = offset.clamp(-0.25, 2.);
        if self.offset != mm {
            self.offset = mm;
            let (x, y) = self.motion();
            self.last_flux = self.flux(x, y);
        }
    }
    pub fn strike(&mut self, v: f64) {
        if v.is_finite() && v > 0. {
            self.held = true;
            self.damper = 0.;
            self.hammer = Some((self.at_hammer(), v.min(4.)));
        }
    }
    pub fn release(&mut self) {
        self.held = false;
    }
    pub fn sustain(&mut self, down: bool) {
        self.pedal = down;
    }
    pub fn clear(&mut self) {
        for m in &mut self.vertical {
            m.clear();
        }
        self.side.clear();
        self.held = false;
        self.pedal = false;
        self.hammer = None;
        self.last_flux = 0.;
        self.damper = 1.;
    }
    pub fn quiet(&self) -> bool {
        self.hammer.is_none()
            && self.vertical.iter().all(|m| m.speed_bound() < 1e-8)
            && self.side.speed_bound() < 1e-8
    }
    pub fn step(&mut self) -> Motion {
        let mut force = 0.;
        if let Some((x, v)) = self.hammer {
            let dx = x - self.at_hammer();
            if dx < 0. {
                self.hammer = None;
            } else {
                force = self.contact * dx * dx.sqrt();
                let v = v - self.dt * force / self.mass;
                self.hammer = Some((x + self.dt * v, v));
            }
        }
        self.damper = if self.held || self.pedal {
            (self.damper - self.damper_step).max(0.)
        } else {
            (self.damper + self.damper_step).min(1.)
        };
        let damping = 1. - self.damper_loss * self.damper;
        for m in &mut self.vertical {
            m.step(force * self.dt, damping);
        }
        self.side.step(force * self.dt, damping);
        let (x, y) = self.motion();
        let flux = self.flux(x, y);
        let emf_v = -6.0e-6 * (flux - self.last_flux) / self.dt;
        self.last_flux = flux;
        Motion {
            tine_mm: x,
            lateral_mm: y,
            contact_n: force,
            emf_v,
        }
    }
}
