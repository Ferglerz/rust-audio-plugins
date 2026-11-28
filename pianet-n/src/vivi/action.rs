//! Loar’s pivoted picker and a steel through-bar with two overhanging ends.
use crate::{
    beam,
    contact_reed::{Bending, ContactReed},
    keyboard::Action,
    scale::interpolate,
};
use std::f64::consts::TAU;
// Mode ratio/front picker/rear felt/rear pickup. See research/vivi_section.py.
const SHAPE: [(f64, f64, f64, f64); 10] = [
    (1., 2.9911158, 0.7485558, 0.8014318),
    (1.2005320, 0.8218683, -2.9241530, -3.1453409),
    (6.1554455, 2.2959956, 0.7700168, 0.9923047),
    (7.4835910, -1.6500578, 1.7096087, 2.2903950),
    (9.4289871, -0.9870557, -1.4005688, -1.9699921),
    (19.1307157, -2.4289451, 0.3834213, 0.6601955),
    (22.3624309, -1.0864206, -1.2581873, -2.3172047),
    (26.4453891, 0.8150488, -0.8880177, -1.7850109),
    (39.2316485, -2.3495544, -0.1671096, -0.4531709),
    (45.6006965, 0.6946324, -0.7168885, -2.3076024),
];
#[derive(Clone, Copy, Default)]
pub struct Observation {
    pub key_fraction: f64,
    pub jack_rad: f64,
    pub armed: bool,
    pub pick_force_n: f64,
    pub front_damper_n: f64,
    pub rear_damper_n: f64,
    pub front_m: f64,
    pub armature_m: f64,
    pub flux_wb_turns: f64,
    pub pickup_v: f64,
}
pub struct Key {
    bar: ContactReed,
    dt: f64,
    key: f64,
    target: f64,
    speed: f64,
    pending: Option<f64>,
    armed: bool,
    angle: f64,
    angular_speed: f64,
    jack_inertia: f64,
    stroke: f64,
    engagement: f64,
    pick_k: f64,
    front_b: f64,
    front_k: f64,
    stock_gap: f64,
    gap: f64,
    flux: f64,
    observation: Observation,
}
impl Key {
    pub fn new(midi: u8, rate: f64) -> Self {
        let n = midi.clamp(super::LOW, super::HIGH);
        let p = |knots: &[(u8, f64)]| interpolate(knots, n);
        let hz = beam::midi_hz(midi);
        let native = beam::midi_hz(n);
        let thick = p(&[(24, 0.00120), (60, 0.00085), (96, 0.00064)]);
        let width = p(&[(24, 0.007), (96, 0.0045)]);
        let length = (22.0248072 * SHAPE[1].0 * thick * (200e9_f64 / (12. * 7850.)).sqrt()
            / (TAU * native))
            .sqrt();
        let mass = 7850. * width * thick * length;
        let loss = p(&[
            (24, 0.50),
            (36, 0.65),
            (48, 1.1),
            (60, 1.6),
            (72, 2.4),
            (96, 5.2),
        ]);
        let modes = SHAPE
            .iter()
            .enumerate()
            .filter_map(|(i, &(r, front, felt, rear))| {
                let f = hz * r / SHAPE[1].0;
                if i > 1 && f > 20000f64.min(0.43 * rate) {
                    return None;
                }
                // The armature-localized mode carries the sustained pitch. Clamping
                // losses on the other bearing modes are estimated from the clean ring.
                let decay = if i == 1 {
                    loss
                } else {
                    (21. + loss) * (r / SHAPE[1].0).powf(0.91)
                }
                .min(0.72 * TAU * f);
                Some(Bending {
                    mode: beam::driven_mode(
                        f,
                        decay,
                        front / rear,
                        mass / (rear * rear),
                        1. / rate,
                    ),
                    felt_to_hammer: felt / front,
                })
            })
            .collect();
        let bar = ContactReed::new(modes, mass / SHAPE[1].3.powi(2));
        let stroke = p(&[(24, 0.005), (48, 0.0031), (72, 0.00096), (96, 0.00023)]);
        let pick_k = p(&[(24, 23000.), (60, 47000.), (96, 220000.)]);
        let gap = p(&[(24, 0.0026), (48, 0.0013), (72, 0.00082), (96, 0.00065)]);
        let mut k = Self {
            bar,
            dt: 1. / rate,
            key: 0.,
            target: 0.,
            speed: 0.,
            pending: None,
            armed: true,
            angle: 0.,
            angular_speed: 0.,
            jack_inertia: p(&[(24, 1.0e-7), (60, 0.75e-7), (96, 0.31e-7)]),
            stroke,
            engagement: 0.095 * stroke,
            pick_k,
            front_b: 0.10 * (mass * pick_k).sqrt(),
            front_k: pick_k * 0.5,
            stock_gap: gap,
            gap,
            flux: 0.,
            observation: Default::default(),
        };
        k.reset();
        k
    }
    fn flux_at(&self, x: f64) -> f64 {
        // Finite facing pole below the armature (Fig. 3). Core/return reluctance
        // adds a fixed effective air path. A soft bound limits approach to the face.
        let h = self.gap + x;
        let fringe = 0.12 * self.stock_gap;
        let air = 0.5 * (h + (h * h + fringe * fringe).sqrt());
        0.00018 * self.stock_gap / (0.6 * self.stock_gap + air)
    }
    pub fn step(&mut self, lift: f64) -> Observation {
        let dt = self.dt;
        let delta = self.speed * dt;
        self.key += (self.target - self.key).clamp(-delta, delta);
        if self.target == 0. && self.key <= 0.64 {
            self.armed = true;
            self.angle = 0.;
            self.angular_speed = 0.;
            if let Some(v) = self.pending.take() {
                self.drive(v);
            }
        }
        let (front, vfront, rear, vrear) = self.bar.free();
        let mut pick = 0.;
        if self.armed && self.target > 0. && self.key >= 0.65 {
            // Load acts off-centre on the oscillating jack. The picker swings
            // away through the bevel while the wippen continues its ascent.
            let arm = 0.004;
            let tip = self.stroke * (self.key - 0.65) - 0.0012 * self.angle;
            if tip > front {
                pick = self.bar.impact(
                    tip - front,
                    (self.speed * self.stroke - 0.0012 * self.angular_speed) - vfront,
                    self.pick_k,
                    0.015,
                    dt,
                    f64::INFINITY,
                );
            }
            self.angular_speed += (pick * 0.0012 / self.jack_inertia + 8500.) * dt;
            self.angle += self.angular_speed * dt;
            if arm * self.angle >= self.engagement || self.key >= 0.997 {
                self.armed = false;
            }
        }
        let open = (self.key / 0.44).min(1.);
        let front_stop = 0.012 * open;
        let front_felt = if front > front_stop {
            -self.bar.impact(
                front - front_stop,
                vfront,
                self.front_k,
                self.front_b,
                dt,
                f64::INFINITY,
            )
        } else {
            0.
        };
        let rear_felt = self.bar.damper(0.012 * open.max(lift), rear, vrear);
        let armature = self.bar.apply(pick + front_felt, rear_felt);
        let flux = self.flux_at(armature);
        let emf = (self.flux - flux) / dt;
        self.flux = flux;
        self.observation = Observation {
            key_fraction: self.key,
            jack_rad: self.angle,
            armed: self.armed,
            pick_force_n: pick,
            front_damper_n: front_felt,
            rear_damper_n: rear_felt,
            front_m: self.bar.drive_position(),
            armature_m: armature,
            flux_wb_turns: flux,
            pickup_v: emf,
        };
        self.observation
    }
    fn drive(&mut self, v: f64) {
        self.target = 1.;
        self.speed = 18. + 175. * v.clamp(0., 1.);
    }
}
impl Action for Key {
    type Observation = Observation;
    fn reset(&mut self) {
        self.bar.reset();
        self.key = 0.;
        self.target = 0.;
        self.speed = 0.;
        self.pending = None;
        self.armed = true;
        self.angle = 0.;
        self.angular_speed = 0.;
        self.flux = self.flux_at(0.);
        self.observation = Observation {
            armed: true,
            flux_wb_turns: self.flux,
            ..Default::default()
        };
    }
    fn press(&mut self, v: f64) {
        if self.target > 0. || self.key > 0.64 || !self.armed {
            self.release();
            self.pending = Some(v);
        } else {
            self.drive(v);
        }
    }
    fn release(&mut self) {
        self.target = 0.;
        self.speed = 93.;
        self.pending = None;
    }
    fn set_pickup_distance_mm(&mut self, offset: f32) {
        self.gap = self.stock_gap + 0.001 * f64::from(offset.clamp(-0.15, 1.));
        self.flux = self.flux_at(self.observation.armature_m);
    }
    fn induced_voltage(&mut self, lift: f64) -> f64 {
        self.step(lift).pickup_v
    }
    fn observation(&self) -> Observation {
        self.observation
    }
}
