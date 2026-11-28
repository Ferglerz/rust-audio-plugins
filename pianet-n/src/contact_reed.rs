//! Bending-mode contact compliance for a hammer and a separately located felt.
use crate::beam;
pub(crate) struct Bending {
    pub(crate) mode: beam::Mode,
    pub(crate) felt_to_hammer: f64,
}

pub(crate) struct ContactReed {
    modes: Vec<Bending>,
    felt_k: f64,
    felt_b: f64,
    hammer_response: f64,
    hammer_velocity_response: f64,
    felt_response: f64,
    felt_velocity_response: f64,
}
impl ContactReed {
    pub(crate) fn new(modes: Vec<Bending>, effective_mass: f64) -> Self {
        let velocity_response = |m: &beam::Mode| m.omega * m.dq_per_force - m.loss * m.dx_per_force;
        let sum = |felt: bool, speed: bool| {
            modes
                .iter()
                .map(|r| {
                    let m = &r.mode;
                    let ratio = if felt { r.felt_to_hammer.powi(2) } else { 1. };
                    m.pad
                        * ratio
                        * if speed {
                            velocity_response(m)
                        } else {
                            m.dx_per_force
                        }
                })
                .sum()
        };
        let hammer_response = sum(false, false);
        let hammer_velocity_response = sum(false, true);
        let felt_response = sum(true, false);
        let felt_velocity_response = sum(true, true);

        let felt_k = 1.7
            / modes
                .iter()
                .map(|r| r.mode.pad * r.mode.compliance * r.felt_to_hammer.powi(2))
                .sum::<f64>();
        let felt_b = 1.6 * (effective_mass * felt_k).sqrt();
        Self {
            modes,
            felt_k,
            felt_b,
            hammer_response,
            hammer_velocity_response,
            felt_response,
            felt_velocity_response,
        }
    }
    pub(crate) fn free(&mut self) -> (f64, f64, f64, f64) {
        let mut xhit = 0.;
        let mut vhit = 0.;
        let mut xfelt = 0.;
        let mut vfelt = 0.;
        for r in &mut self.modes {
            r.mode.free();
            let x = r.mode.position * r.mode.pad;
            let v = r.mode.velocity() * r.mode.pad;
            xhit += x;
            vhit += v;
            xfelt += x * r.felt_to_hammer;
            vfelt += v * r.felt_to_hammer;
        }
        (xhit, vhit, xfelt, vfelt)
    }
    pub(crate) fn impact(
        &self,
        compression: f64,
        relative_velocity: f64,
        stiffness: f64,
        dashpot: f64,
        dt: f64,
        mass: f64,
    ) -> f64 {
        ((stiffness * compression + dashpot * relative_velocity)
            / (1.
                + stiffness * (self.hammer_response + dt * dt / mass)
                + dashpot * (self.hammer_velocity_response + dt / mass)))
            .max(0.)
    }
    pub(crate) fn damper(&self, stop: f64, xfelt: f64, vfelt: f64) -> f64 {
        if xfelt > stop {
            ((self.felt_k * (stop - xfelt) - self.felt_b * vfelt)
                / (1.
                    + self.felt_k * self.felt_response
                    + self.felt_b * self.felt_velocity_response))
                .min(0.)
        } else {
            0.
        }
    }
    pub(crate) fn apply(&mut self, impact: f64, felt: f64) -> f64 {
        let mut tip = 0.;
        for r in &mut self.modes {
            let force = impact + r.felt_to_hammer * felt;
            r.mode.position += r.mode.dx_per_force * force;
            r.mode.quadrature += r.mode.dq_per_force * force;
            tip += r.mode.position;
        }
        tip
    }
    pub(crate) fn drive_position(&self) -> f64 {
        self.modes
            .iter()
            .map(|r| r.mode.position * r.mode.pad)
            .sum()
    }
    pub(crate) fn reset(&mut self) {
        for r in &mut self.modes {
            r.mode.position = 0.;
            r.mode.quadrature = 0.;
        }
    }
}
