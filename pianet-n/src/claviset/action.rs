//! Rotating one-way cam, felt damper and magnetic sensing at a free steel reed.
use crate::{beam, scale};
use std::f64::consts::PI;

#[derive(Clone, Copy, Default)]
pub struct Observation {
    pub key_fraction: f64,
    pub cam_angle_rad: f64,
    pub lip_overlap_m: f64,
    pub lip_force_n: f64,
    pub armed: bool,
    pub damper_force_n: f64,
    pub reed_tip_m: f64,
    pub pickup_v: f64,
}

pub struct Key {
    modes: Vec<beam::Mode>,
    damper_ratio: Vec<f64>,
    rate: f64,
    travel: f64,
    goal: f64,
    speed: f64,
    pending: Option<f64>,
    armed: bool,
    cam: Cam,
    lip_k: f64,
    lip_response: f64,
    felt_k: f64,
    felt_b: f64,
    felt_response: f64,
    felt_velocity_response: f64,
    gap: f64,
    native_gap: f64,
    flux_scale: f64,
    previous_flux: f64,
    pub observation: Observation,
}

struct Cam {
    radius: f64,
    edge: f64,
    reed_height: f64,
}
impl Cam {
    fn position(&self, fraction: f64) -> (f64, f64, f64) {
        // The lip sweeps up and laterally off the edge. The pivoting mount
        // bypasses the reed on return until the lip is underneath it again.
        let angle = (-24. + 100. * fraction) * PI / 180.;
        let (s, c) = angle.sin_cos();
        (
            angle,
            self.radius * c - self.edge,
            self.radius * s - self.reed_height,
        )
    }
}

impl Key {
    pub fn new(midi: u8, rate: f64) -> Self {
        assert!((super::LOW..=super::HIGH).contains(&midi) && rate >= 16_000.);
        Self::extended(midi, rate)
    }
    pub(crate) fn extended(midi: u8, rate: f64) -> Self {
        let anchor = midi.clamp(super::LOW, super::HIGH);
        let p = |pts: &[(u8, f64)]| scale::interpolate(pts, anchor);
        let hz = crate::beam::midi_hz(midi);
        let s = scale::Section {
            hz: crate::beam::midi_hz(anchor),
            length: p(&[(36, 0.082), (60, 0.047), (84, 0.026), (96, 0.018)]),
            width: p(&[(36, 0.0058), (60, 0.0046), (96, 0.0034)]),
            pluck: p(&[(36, 0.00135), (48, 0.00083), (64, 0.00042), (96, 0.00024)]),
            gap: p(&[(36, 0.00042), (48, 0.00055), (64, 0.00085), (96, 0.001)]),
            loss: p(&[(36, 1.0), (60, 1.27), (69, 3.05), (84, 4.1), (96, 6.)]),
            field_slope: 0.,
        };
        let (_, mass, modes) =
            beam::modes_at_pitch(&s, hz, rate, 0.91, |f| s.loss * (f / hz).powf(1.3));
        let damper_ratio: Vec<_> = modes
            .iter()
            .enumerate()
            .map(|(i, m)| beam::damper_shape(i, 0.76) / m.pad)
            .collect();
        let lip_k = 4. / modes.iter().map(|m| m.pad * m.compliance).sum::<f64>();
        let felt_k = 2.
            / modes
                .iter()
                .zip(&damper_ratio)
                .map(|(m, r)| m.pad * m.compliance * r * r)
                .sum::<f64>();
        let felt_b = 1.8 * (mass * felt_k).sqrt();
        let lip_response = modes.iter().map(|m| m.pad * m.dx_per_force).sum();
        let felt_response = modes
            .iter()
            .zip(&damper_ratio)
            .map(|(m, r)| m.pad * m.dx_per_force * r * r)
            .sum();
        let felt_velocity_response = modes
            .iter()
            .zip(&damper_ratio)
            .map(|(m, r)| m.pad * (m.omega * m.dq_per_force - m.loss * m.dx_per_force) * r * r)
            .sum();
        // Set the undisturbed reed height at -4 degrees and the edge at +38.
        // These are unmeasured effective lip dimensions projected into the reed plane.
        let radius = s.pluck / (38f64.to_radians().sin() - (-4f64).to_radians().sin());
        let mut k = Self {
            modes,
            damper_ratio,
            rate,
            travel: 0.,
            goal: 0.,
            speed: 0.,
            pending: None,
            armed: true,
            cam: Cam {
                radius,
                edge: radius * 38f64.to_radians().cos(),
                reed_height: radius * (-4f64).to_radians().sin(),
            },
            lip_k,
            lip_response,
            felt_k,
            felt_b,
            felt_response,
            felt_velocity_response,
            gap: s.gap,
            native_gap: s.gap,
            flux_scale: 0.000055,
            previous_flux: 0.,
            observation: Observation::default(),
        };
        k.reset();
        k
    }
    pub fn set_pickup_distance_mm(&mut self, offset: f32) {
        self.gap = self.native_gap + f64::from(offset.clamp(-0.15, 1.)) * 0.001;
        self.flux_scale = 0.000055 * self.native_gap / self.gap;
        self.previous_flux = self.flux(self.observation.reed_tip_m);
    }
    fn flux(&self, x: f64) -> f64 {
        // Approximate the linked flux as the tip passes a laterally offset pole.
        let offset = 0.57 - x / self.gap;
        self.flux_scale / (1. + offset * offset).sqrt()
    }
    pub fn reset(&mut self) {
        for m in &mut self.modes {
            m.position = 0.;
            m.quadrature = 0.;
        }
        self.travel = 0.;
        self.goal = 0.;
        self.pending = None;
        self.armed = true;
        self.previous_flux = self.flux(0.);
        let (cam_angle_rad, lip_overlap_m, _) = self.cam.position(0.);
        self.observation = Observation {
            armed: true,
            cam_angle_rad,
            lip_overlap_m,
            ..Default::default()
        };
    }
    pub fn press(&mut self, velocity: f64) {
        if self.goal > 0. || self.travel > 0.12 || !self.armed {
            self.release();
            self.pending = Some(velocity);
        } else {
            self.move_to(1., velocity);
        }
    }
    pub fn release(&mut self) {
        self.pending = None;
        self.move_to(0., 0.);
    }
    pub fn move_to(&mut self, fraction: f64, velocity: f64) {
        self.goal = fraction.clamp(0., 1.);
        // Fixed escapement sets pluck displacement. Attack speed varies slightly.
        self.speed = if self.goal < self.travel {
            96.
        } else {
            73. + 8. * velocity.clamp(0., 1.)
        };
    }
    pub fn step(&mut self, pedal_lift: f64) -> Observation {
        let old = self.travel;
        self.travel +=
            (self.goal - self.travel).clamp(-self.speed / self.rate, self.speed / self.rate);
        let (angle, overlap, height) = self.cam.position(self.travel);
        if self.travel > old && overlap <= 0. {
            self.armed = false;
        }
        if !self.armed && self.travel < 0.12 && height < 0. {
            self.armed = true;
        }
        if self.armed && self.travel < 0.12 {
            if let Some(v) = self.pending.take() {
                self.move_to(1., v);
            }
        }
        let (mut x_pick, mut x_felt, mut v_felt) = (0., 0., 0.);
        for (m, r) in self.modes.iter_mut().zip(&self.damper_ratio) {
            m.free();
            x_pick += m.pad * m.position;
            x_felt += r * m.pad * m.position;
            v_felt += r * m.pad * m.velocity();
        }
        let lip = if self.armed && self.goal > 0. && self.goal >= self.travel && overlap > 0. {
            (self.lip_k * (height - x_pick) / (1. + self.lip_k * self.lip_response)).max(0.)
        } else {
            0.
        };
        let lift = self.travel.max(pedal_lift);
        let stop_y = self.cam.radius * 8. * lift;
        let felt = if x_felt > stop_y {
            ((self.felt_k * (stop_y - x_felt) - self.felt_b * v_felt)
                / (1.
                    + self.felt_k * self.felt_response
                    + self.felt_b * self.felt_velocity_response))
                .min(0.)
        } else {
            0.
        };
        let mut tip = 0.;
        for (m, r) in self.modes.iter_mut().zip(&self.damper_ratio) {
            let force = lip + r * felt;
            m.position += m.dx_per_force * force;
            m.quadrature += m.dq_per_force * force;
            tip += m.position;
        }
        let flux = self.flux(tip);
        let pickup_v = (flux - self.previous_flux) * self.rate;
        self.previous_flux = flux;
        self.observation = Observation {
            key_fraction: self.travel,
            cam_angle_rad: angle,
            lip_overlap_m: overlap,
            lip_force_n: lip,
            armed: self.armed,
            damper_force_n: felt,
            reed_tip_m: tip,
            pickup_v,
        };
        self.observation
    }
}
