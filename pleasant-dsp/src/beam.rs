//! Modal observations of a struck beam, in physical displacement coordinates.
use crate::instrument::rotate;
use std::f64::consts::TAU;
pub struct BeamMode {
    omega: f64,
    sin: f64,
    cos: f64,
    q: f64,
    quadrature: f64,
    hit: f64,
    impulse_to_quadrature: f64,
    sensed: f64,
    other: f64,
}

impl BeamMode {
    pub fn new(f: f64, rate: f64, t60: f64, hit: f64, sensed: f64, other: f64) -> Self {
        let omega = TAU * f;
        let gain = 10_f64.powf(-3. / (t60 * rate));
        let (s, c) = (omega / rate).sin_cos();
        Self {
            omega,
            sin: s * gain,
            cos: c * gain,
            q: 0.,
            quadrature: 0.,
            hit,
            impulse_to_quadrature: hit / omega,
            sensed,
            other,
        }
    }
    pub fn step(&mut self, impulse: f64, damping: f64) {
        self.quadrature += self.impulse_to_quadrature * impulse;
        let (q, v) = rotate(self.q, self.quadrature, self.sin, self.cos);
        self.q = damping * q;
        self.quadrature = damping * v;
    }
    #[inline]
    pub fn hit_displacement(&self) -> f64 {
        self.q * self.hit
    }
    #[inline]
    pub fn sensed_displacement(&self) -> f64 {
        self.q * self.sensed
    }
    #[inline]
    pub fn return_displacement(&self) -> f64 {
        self.q * self.other
    }
    pub fn speed_bound(&self) -> f64 {
        (self.q.abs() + self.quadrature.abs()) * self.omega * (self.sensed.abs() + self.other.abs())
    }
    pub fn clear(&mut self) {
        self.q = 0.;
        self.quadrature = 0.;
    }
}

pub fn beam_shape(beta: f64, x: f64) -> f64 {
    let sigma = (beta.cosh() + beta.cos()) / (beta.sinh() + beta.sin());
    let shape = |r: f64| r.cosh() - r.cos() - sigma * (r.sinh() - r.sin());
    shape(beta * x) / shape(beta)
}
