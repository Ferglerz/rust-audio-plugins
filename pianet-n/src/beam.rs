//! Uniform clamped-free beam, exact free rotation and constant-force response.
use crate::scale::Section;
use pleasant_dsp::instrument::rotate;
use std::f64::consts::TAU;

// Cantilever roots, clamped at x=0 and free at x=1.
pub(crate) const ROOTS: [f64; 8] = [
    1.87510407,
    4.69409113,
    7.85475744,
    10.99554073,
    14.13716839,
    17.27875953,
    20.42035225,
    23.56194490,
];

pub(crate) fn midi_hz(note: u8) -> f64 {
    440. * 2f64.powf((f64::from(note) - 69.) / 12.)
}

pub(crate) fn shape(beta: f64, x: f64) -> f64 {
    let s = (beta.cosh() + beta.cos()) / (beta.sinh() + beta.sin());
    let y = |u: f64| u.cosh() - u.cos() - s * (u.sinh() - u.sin());
    y(beta * x) / y(beta)
}

#[derive(Clone)]
pub(crate) struct Mode {
    pub(crate) position: f64,
    // Quadrature coordinate (velocity + loss * position) / damped angular frequency.
    pub(crate) quadrature: f64,
    pub(crate) sin: f64,
    pub(crate) cos: f64,
    pub(crate) omega: f64,
    pub(crate) loss: f64,
    pub(crate) pad: f64,
    pub(crate) dx_per_force: f64,
    pub(crate) dq_per_force: f64,
    pub(crate) compliance: f64,
}

/// Reuse the native cross section and modal mass while retuning its natural
/// frequencies. Extended virtual keys keep the nearest actual reed geometry.
pub(crate) fn modes_at_pitch(
    section: &Section,
    fundamental: f64,
    solver_rate: f64,
    contact: f64,
    loss_at: impl Fn(f64) -> f64,
) -> (f64, f64, Vec<Mode>) {
    let Section {
        hz, length, width, ..
    } = *section;
    let step = 1.0 / solver_rate;
    let thickness =
        TAU * hz * length.powi(2) / ROOTS[0].powi(2) * (12.0_f64 * 7850.0 / 200e9).sqrt();
    let mass = 0.25 * 7850.0 * width * thickness * length;
    let modes: Vec<_> = ROOTS
        .iter()
        .enumerate()
        .filter_map(|(index, &root)| {
            let frequency = fundamental * (root / ROOTS[0]).powi(2);
            if index != 0 && frequency > 20_000.0_f64.min(solver_rate * 0.43) {
                return None;
            }
            Some(driven_mode(
                frequency,
                loss_at(frequency),
                shape(root, contact),
                mass,
                step,
            ))
        })
        .collect();
    (thickness, mass, modes)
}

pub(crate) fn driven_mode(frequency: f64, loss: f64, pad: f64, mass: f64, step: f64) -> Mode {
    let natural = TAU * frequency;
    let omega = (natural.powi(2) - loss.powi(2)).sqrt();
    let decay = (-loss * step).exp();
    let (sin, cos) = (omega * step).sin_cos();
    let (sin, cos) = (sin * decay, cos * decay);
    let compliance = pad / (mass * natural.powi(2));
    // Exact response to a constant contact force over the time step.
    let equilibrium_q = loss / omega;
    let (x, q) = rotate(1.0, equilibrium_q, sin, cos);
    Mode {
        position: 0.0,
        quadrature: 0.0,
        sin,
        cos,
        omega,
        loss,
        pad,
        dx_per_force: compliance * (1.0 - x),
        dq_per_force: compliance * (equilibrium_q - q),
        compliance,
    }
}

impl Mode {
    pub(crate) fn free(&mut self) {
        (self.position, self.quadrature) =
            rotate(self.position, self.quadrature, self.sin, self.cos);
    }
    pub(crate) fn velocity(&self) -> f64 {
        self.omega * self.quadrature - self.loss * self.position
    }
}
pub(crate) fn damper_shape(index: usize, x: f64) -> f64 {
    shape(ROOTS[index], x)
}
