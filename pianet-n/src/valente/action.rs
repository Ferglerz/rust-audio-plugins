//! Short-throw counter-hammer, shouldered steel reed and inductive pickup.
use crate::{
    beam,
    contact_reed::{Bending, ContactReed},
    keyboard::Action,
    scale::interpolate,
};
use std::f64::consts::TAU;

// Variable width finite element solution; see research/valente_section.py.
// Frequency ratio, modal mass scale, hammer participation, damper participation.
const SECTION: [(f64, f64, f64, f64); 8] = [
    (1.0000000, 0.2266307, 0.5202060, 0.7567278),
    (5.7723800, 0.2450217, -0.4629631, 0.1870346),
    (15.1681944, 0.2567720, -0.6425325, -0.2205024),
    (28.5342370, 0.2566675, -0.2119521, -0.5182050),
    (46.3087230, 0.2530610, 0.4392669, -0.6553053),
    (68.5893439, 0.2521062, 0.7062385, -0.6102628),
    (95.2682980, 0.2519708, 0.3316768, -0.4015198),
    (126.3715730, 0.2513629, -0.3456655, -0.0852710),
];
const THROW: f64 = 0.00125;
#[derive(Clone, Copy, Default)]
pub struct Observation {
    pub key_fraction: f64,
    pub counter_m: f64,
    pub launch_mps: f64,
    pub hammer_force_n: f64,
    pub damper_force_n: f64,
    pub reed_tip_m: f64,
    pub tip_velocity_mps: f64,
    pub flux_wb_turns: f64,
    pub pickup_v: f64,
}
pub struct Key {
    reed: ContactReed,
    rate: f64,
    mass: f64,
    tip_k: f64,
    key: f64,
    target: f64,
    speed: f64,
    trigger_armed: bool,
    pending: Option<f64>,
    x: f64,
    v: f64,
    airborne: bool,
    stock_gap: f64,
    gap: f64,
    edge_offset: f64,
    flux: f64,
    observation: Observation,
}
pub(super) fn clearance(n: u8) -> f64 {
    interpolate(&[(41, 0.00113), (53, 0.000675), (74, 0.0005)], n)
}
pub(super) fn neutral_offset(n: u8) -> f64 {
    clearance(n) * interpolate(&[(41, 1.03), (53, 0.53), (74, 0.43)], n)
}
pub(super) fn loss(n: u8) -> f64 {
    interpolate(
        &[(36, 0.46), (48, 0.62), (64, 0.9), (76, 1.8), (96, 3.9)],
        n,
    )
}
impl Key {
    pub fn new(midi: u8, rate: f64) -> Self {
        let n = midi.clamp(super::LOW, super::HIGH);
        let hz = beam::midi_hz(midi);
        let thickness = 0.00052;
        let width = 0.0038;
        let length = (4.4399764 * thickness * (200e9f64 / (12. * 7850.)).sqrt()
            / (TAU * beam::midi_hz(n)))
        .sqrt();
        let mass_unit = 7850. * width * thickness * length;
        let modes = SECTION
            .iter()
            .copied()
            .filter(|(ratio, ..)| *ratio == 1. || ratio * hz <= 20_000f64.min(rate * 0.43))
            .map(|(ratio, mass, hammer, felt)| Bending {
                mode: beam::driven_mode(
                    hz * ratio,
                    loss(n) * ratio.powf(1.33),
                    hammer,
                    mass * mass_unit,
                    1. / rate,
                ),
                felt_to_hammer: felt / hammer,
            })
            .collect();
        let gap = clearance(n);
        let mut k = Self {
            reed: ContactReed::new(modes, SECTION[0].1 * mass_unit),
            rate,
            mass: interpolate(&[(36, 0.0016), (96, 0.00072)], n),
            tip_k: interpolate(&[(36, 19000.), (64, 40000.), (96, 190000.)], n),
            key: 0.,
            target: 0.,
            speed: 0.,
            trigger_armed: true,
            pending: None,
            x: -THROW,
            v: 0.,
            airborne: false,
            stock_gap: gap,
            gap,
            edge_offset: neutral_offset(n),
            flux: 0.,
            observation: Default::default(),
        };
        k.reset();
        k
    }
    // A narrow moving steel reed crosses the fringe of a fixed pole. The finite
    // pole width and offset regularize the magnetic reluctance near its centre.
    fn flux_at(&self, tip: f64) -> f64 {
        let z = self.edge_offset - tip;
        0.000105 * self.stock_gap.powi(2) / (self.gap.powi(2) + z * z)
    }
    pub fn step(&mut self, rail: f64) -> Observation {
        let dt = 1. / self.rate;
        let delta = self.speed * dt;
        self.key += (self.target - self.key).clamp(-delta, delta);
        if self.target == 0. && self.key <= 0.16 && !self.airborne {
            self.trigger_armed = true;
            if let Some(v) = self.pending.take() {
                self.drive(v);
            }
        }
        if self.trigger_armed && self.target > 0. && self.key >= 0.63 {
            self.trigger_armed = false;
            // Relative to the contact, the heavy FATAR dummy acts on the lighter
            // counter-hammer with a finite restitution. Its tip is then free.
            let driver_mass = 0.010;
            self.v = (1. + 0.25) * driver_mass / (driver_mass + self.mass) * 0.0052 * self.speed;
            self.observation.launch_mps = self.v;
            self.airborne = true;
        }
        let (pos, vel, xfelt, vfelt) = self.reed.free();
        let mut impact = 0.;
        if self.airborne {
            // Return torque over the short stroke, in tip-referred units.
            self.v -= 20. * dt;
            self.x += self.v * dt;
            if self.x > pos {
                impact =
                    self.reed
                        .impact(self.x - pos, self.v - vel, self.tip_k, 0.085, dt, self.mass);
                self.v -= impact * dt / self.mass;
                self.x -= impact * dt * dt / self.mass;
            }
            if self.x <= -THROW && self.v < 0. {
                self.x = -THROW;
                self.v = 0.;
                self.airborne = false;
            }
        }
        let stop = 0.008 * (self.key / 0.55).min(1.).max(rail);
        let felt = self.reed.damper(stop, xfelt, vfelt);
        let tip = self.reed.apply(impact, felt);
        let flux = self.flux_at(tip);
        let emf = (self.flux - flux) * self.rate;
        self.flux = flux;
        self.observation = Observation {
            key_fraction: self.key,
            counter_m: self.x,
            launch_mps: self.observation.launch_mps,
            hammer_force_n: impact,
            damper_force_n: felt,
            reed_tip_m: tip,
            tip_velocity_mps: (tip - self.observation.reed_tip_m) * self.rate,
            flux_wb_turns: flux,
            pickup_v: emf,
        };
        self.observation
    }
    fn drive(&mut self, v: f64) {
        self.target = 1.;
        self.speed = 16. + 192. * v.clamp(0., 1.);
    }
}
impl Action for Key {
    type Observation = Observation;
    fn reset(&mut self) {
        self.reed.reset();
        self.key = 0.;
        self.target = 0.;
        self.speed = 0.;
        self.pending = None;
        self.trigger_armed = true;
        self.airborne = false;
        self.x = -THROW;
        self.v = 0.;
        self.flux = self.flux_at(0.);
        self.observation = Observation {
            counter_m: self.x,
            flux_wb_turns: self.flux,
            ..Default::default()
        };
    }
    fn press(&mut self, v: f64) {
        if self.key > 0.16 || !self.trigger_armed || self.target > 0. {
            self.release();
            self.pending = Some(v);
        } else {
            self.drive(v);
        }
    }
    fn release(&mut self) {
        self.target = 0.;
        self.speed = 105.;
        self.pending = None;
    }
    fn set_pickup_distance_mm(&mut self, offset: f32) {
        self.gap = self.stock_gap + f64::from(offset.clamp(-0.15, 1.)) * 0.001;
        self.flux = self.flux_at(self.observation.reed_tip_m);
    }
    fn induced_voltage(&mut self, rail: f64) -> f64 {
        self.step(rail).pickup_v
    }
    fn support_velocity(&self) -> f64 {
        self.observation.tip_velocity_mps
    }
    fn observation(&self) -> Observation {
        self.observation
    }
}
