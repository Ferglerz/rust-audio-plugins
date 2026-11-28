//! Vertical reed, escaped hammer and independent rear damper on the EP-61C.
use crate::contact_reed::{Bending, ContactReed};
use crate::{beam, keyboard::Action, scale::interpolate};
use std::f64::consts::{PI, TAU};

#[derive(Default, Clone, Copy)]
pub struct Observation {
    pub key_fraction: f64,
    pub hammer_m: f64,
    pub hammer_force_n: f64,
    pub escaped: bool,
    pub caught: bool,
    pub damper_force_n: f64,
    pub reed_tip_m: f64,
    pub flux_wb_turns: f64,
    pub pickup_v: f64,
}

#[derive(Clone, Copy, PartialEq)]
enum Hammer {
    Jack,
    Flight,
    Backcheck,
}

/// The end weight contributes inertial shear at the tip while the bending
/// moment stays zero. Dividing by cosh(beta) keeps higher roots well conditioned.
fn loaded_root(order: usize, tip_mass_ratio: f64) -> f64 {
    let equation =
        |b: f64| b.cos() + 1. / b.cosh() + tip_mass_ratio * b * (b.cos() * b.tanh() - b.sin());
    let mut lo = PI * order as f64;
    let mut hi = beam::ROOTS[order] + 0.00001;
    let start = equation(lo).is_sign_positive();
    for _ in 0..48 {
        let mid = (lo + hi) * 0.5;
        if equation(mid).is_sign_positive() == start {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) * 0.5
}

pub struct Key {
    reed: ContactReed,
    rate: f64,
    blow: f64,
    throw: f64,
    mass: f64,
    tip_k: f64,
    tip_b: f64,
    gap: f64,
    native_gap: f64,
    pole_face: f64,
    lateral_offset: f64,
    winding: f64,
    key: f64,
    target: f64,
    speed: f64,
    pending: Option<f64>,
    hammer: Hammer,
    x: f64,
    v: f64,
    flux: f64,
    observation: Observation,
}

impl Key {
    pub fn new(midi: u8, rate: f64) -> Self {
        assert!(midi <= 127 && rate.is_finite() && rate >= 16000.);
        let n = midi.clamp(super::LOW, super::HIGH);
        let p = |knots: &[(u8, f64)]| interpolate(knots, n);
        let nominal = beam::midi_hz(n);
        let hz = beam::midi_hz(midi);
        let eta = p(&[(36, 1.7), (48, 0.85), (64, 0.30), (84, 0.10), (96, 0.035)]);
        let first = loaded_root(0, eta);
        // A single estimated reed stock and a graduated end weight. The
        // effective free length follows the pitch of that weighted cantilever.
        let thickness = 0.0006;
        let width = 0.0047;
        let length =
            first * (thickness * (200e9_f64 / (12. * 7850.)).sqrt() / (TAU * nominal)).sqrt();
        let beam_mass = 7850. * width * thickness * length;
        let modes: Vec<_> = (0..beam::ROOTS.len())
            .filter_map(|i| {
                let root = if i == 0 { first } else { loaded_root(i, eta) };
                let f = hz * (root / first).powi(2);
                if i > 0 && f > 20000_f64.min(0.43 * rate) {
                    return None;
                }
                let integral = (0..64)
                    .map(|j| beam::shape(root, (j as f64 + 0.5) / 64.).powi(2))
                    .sum::<f64>()
                    / 64.;
                let mass = beam_mass * (integral + eta);
                let strike = beam::shape(root, 0.68);
                let felt = beam::shape(root, 0.82);
                let loss = p(&[
                    (36, 0.28),
                    (44, 0.39),
                    (60, 0.62),
                    (72, 1.26),
                    (84, 2.24),
                    (96, 4.0),
                ]) * (f / hz).powf(1.25);
                Some(Bending {
                    mode: beam::driven_mode(f, loss, strike, mass, 1. / rate),
                    felt_to_hammer: felt / strike,
                })
            })
            .collect();
        let reed = ContactReed::new(modes, beam_mass * (0.25 + eta));
        let gap = p(&[(36, 0.0015), (60, 0.00095), (84, 0.00065), (96, 0.0006)]);
        let mut k = Self {
            reed,
            rate,
            blow: 0.005,
            throw: 0.0044,
            mass: p(&[(36, 0.0023), (96, 0.0011)]),
            tip_k: p(&[(36, 19000.), (60, 39000.), (96, 160000.)]),
            tip_b: 0.13,
            gap,
            native_gap: gap,
            pole_face: p(&[(36, 0.0019), (60, 0.0008), (96, 0.00024)]),
            lateral_offset: 0.50 * gap,
            winding: if n.is_multiple_of(2) { 1. } else { -1. },
            key: 0.,
            target: 0.,
            speed: 0.,
            pending: None,
            hammer: Hammer::Jack,
            x: 0.,
            v: 0.,
            flux: 0.,
            observation: Observation::default(),
        };
        k.reset();
        k
    }
    fn field(&self, x: f64) -> f64 {
        // US4175461 Fig. 11: constant reluctance over a finite arcuate
        // face, then quadratic growth in distance beyond either edge.
        let position = x - self.lateral_offset;
        // Smooth both finite pole edges over a small fraction of the gap.
        // Fringing supplies residual sensitivity inside the nearly flat face.
        let edge = 0.08 * self.gap;
        let edge_extent = |x: f64| 0.5 * (x + (x * x + edge * edge).sqrt());
        let outside = position - edge_extent(position) + edge_extent(position - self.pole_face);
        0.00012 * self.native_gap.powi(2) / (self.gap.powi(2) + outside.powi(2))
    }
    pub fn set_lateral_offset_mm(&mut self, offset: f32) {
        self.lateral_offset = 0.50 * self.native_gap + f64::from(offset.clamp(-1., 1.)) * 0.001;
        self.flux = self.field(self.observation.reed_tip_m);
    }
    pub fn step(&mut self, lift: f64) -> Observation {
        let dt = 1. / self.rate;
        let advance = self.speed * dt;
        self.key += (self.target - self.key).clamp(-advance, advance);
        if self.key < 0.12 && self.target == 0. && self.hammer != Hammer::Flight {
            self.hammer = Hammer::Jack;
            if let Some(v) = self.pending.take() {
                self.drive(v);
            }
        }
        if self.hammer == Hammer::Jack {
            self.x = -self.blow + self.throw * (self.key / 0.72).min(1.);
            if self.key >= 0.72 && self.target > 0. {
                self.hammer = Hammer::Flight;
                self.v = self.speed * self.throw / 0.72;
            }
        }
        let (xhit, vhit, xfelt, vfelt) = self.reed.free();
        let mut impact = 0.;
        if self.hammer == Hammer::Flight {
            self.v -= 18. * dt;
            self.x += self.v * dt;
            let compression = self.x - xhit;
            if compression > 0. {
                impact = self.reed.impact(
                    compression,
                    self.v - vhit,
                    self.tip_k,
                    self.tip_b,
                    dt,
                    self.mass,
                );
                self.v -= impact * dt / self.mass;
                self.x -= impact * dt * dt / self.mass;
            }
            if self.v < 0. && self.x < -self.blow * 0.55 {
                self.hammer = Hammer::Backcheck;
                self.x = -self.blow * 0.55;
                self.v = 0.;
            }
        }
        let stop = 0.008 * (self.key / 0.55).min(1.).max(lift);
        let felt = self.reed.damper(stop, xfelt, vfelt);
        let tip = self.reed.apply(impact, felt);
        let flux = self.field(tip);
        let emf = self.winding * (self.flux - flux) * self.rate;
        self.flux = flux;
        self.observation = Observation {
            key_fraction: self.key,
            hammer_m: self.x,
            hammer_force_n: impact,
            escaped: self.hammer == Hammer::Flight,
            caught: self.hammer == Hammer::Backcheck,
            damper_force_n: felt,
            reed_tip_m: tip,
            flux_wb_turns: flux,
            pickup_v: emf,
        };
        self.observation
    }
    fn drive(&mut self, v: f64) {
        self.target = 1.;
        self.speed = 27. + 158. * v.clamp(0., 1.);
    }
}
impl Action for Key {
    type Observation = Observation;
    fn press(&mut self, v: f64) {
        if self.target > 0. || self.key > 0.12 || self.hammer != Hammer::Jack {
            self.release();
            self.pending = Some(v);
        } else {
            self.drive(v);
        }
    }
    fn release(&mut self) {
        self.target = 0.;
        self.speed = 95.;
        self.pending = None;
    }
    fn reset(&mut self) {
        self.reed.reset();
        self.key = 0.;
        self.target = 0.;
        self.speed = 0.;
        self.pending = None;
        self.hammer = Hammer::Jack;
        self.x = -self.blow;
        self.v = 0.;
        self.flux = self.field(0.);
        self.observation = Observation {
            hammer_m: self.x,
            flux_wb_turns: self.flux,
            ..Default::default()
        };
    }
    fn set_pickup_distance_mm(&mut self, offset: f32) {
        self.gap = self.native_gap + f64::from(offset.clamp(-0.15, 1.)) * 0.001;
        self.flux = self.field(self.observation.reed_tip_m);
    }
    fn induced_voltage(&mut self, lift: f64) -> f64 {
        self.step(lift).pickup_v
    }
    fn observation(&self) -> Observation {
        self.observation
    }
}
