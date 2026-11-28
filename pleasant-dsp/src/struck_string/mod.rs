//! Dispersive struck-string course with a compression-only moving hammer.
//! A course can observe either transverse bridge force or displacement at a pickup.
mod lanes;
pub use lanes::Lanes;
use std::f64::consts::{PI, TAU};
use wide::{f32x8, f64x4};

#[derive(Clone, Copy)]
pub enum Readout {
    BearingForce,
    Displacement { fraction: f64, aperture_m: f64 },
}

#[derive(Clone, Copy)]
pub struct CourseSpec {
    pub frequency_hz: f64,
    pub strings: usize,
    pub stiffness: f64,
    pub length: f64,
    pub tension: f64,
    pub hammer_kg: f64,
    pub contact_k: f64,
    pub contact_linear: f64,
    pub strike_fraction: f64,
    pub unison_cents: f64,
    pub read_imbalance: f64,
    pub losses_db_s: [f64; 6],
    pub hammer_width_m: f64,
    pub has_damper: bool,
    pub damper_loss: f64,
    pub readout: Readout,
}

pub type LiveCourse = Course<f32x8>;
pub type ReferenceCourse = Course<f64x4>;

// Each lane represents a different partial of the same string. Unused lanes
// have zero contact and bearing and therefore never excite the instrument.
#[derive(Clone, Copy, Default)]
struct StringModes<V: Lanes> {
    position: V,
    velocity: V,
    sin_step: V,
    cos_step: V,
    sin_host: V,
    cos_host: V,
    contact_read: V,
    read_weight: V,
}

impl<V: Lanes> StringModes<V> {
    #[inline(always)]
    fn advance(&mut self, sn: V, cs: V, velocity: V, damping: V) -> V {
        let position = self.position * cs + velocity * sn;
        self.velocity = (velocity * cs - self.position * sn) * damping;
        self.position = position * damping;
        self.read_weight * self.position
    }
}

#[derive(Default)]
struct Partials<V: Lanes> {
    strings: [StringModes<V>; 2],
    impulse: V,
    common_loss: V,
    damper: V,
    host_damper: V,
}

pub struct Course<V: Lanes> {
    spec: CourseSpec,
    partials: Vec<Partials<V>>,
    steps: usize,
    dt: f64,
    hammer_pos: f64,
    hammer_speed: f64,
    hammer_gravity: f64,
    hammer_active: bool,
    key_down: bool,
    pedal: bool,
    // Optional felt strip acting on the speaking strings even while keys are held.
    rail_loss: Vec<(V, V)>,
    rail_down: bool,
}

impl<V: Lanes> Course<V> {
    pub fn at_rates(spec: CourseSpec, rate: f64, steps: usize) -> Self {
        assert!((1..=2).contains(&spec.strings));
        assert!(steps > 0 && rate.is_finite() && rate > 0.);
        assert!(spec.length > 0. && spec.tension > 0. && spec.hammer_kg > 0.);
        assert!(spec.hammer_width_m > 0. && spec.frequency_hz > 0.);
        let f1 = spec.frequency_hz;
        let modal_mass = spec.tension / (8. * f1.powi(2) * spec.length);
        let mut partials: Vec<Partials<V>> = Vec::new();
        for n in 1..=192 {
            let order = n as f64;
            let freq =
                f1 * order * ((1. + spec.stiffness * order * order) / (1. + spec.stiffness)).sqrt();
            if freq > 19000_f64.min(0.46 * rate / steps as f64) {
                break;
            }
            let loss = if n <= 6 {
                spec.losses_db_s[n - 1]
            } else {
                spec.losses_db_s[5] + (order - 6.) * 2.
            };
            let radius = 10f64.powf(-loss / (20. * rate));
            let lane = (n - 1) % V::WIDTH;
            if lane == 0 {
                partials.push(Partials::default());
            }
            let p = partials.last_mut().unwrap();
            let width = PI * order * spec.hammer_width_m / (2. * spec.length);
            let contact = (PI * order * spec.strike_fraction).sin() * width.sin() / width;
            let readout = match spec.readout {
                Readout::BearingForce => spec.tension * PI * order / spec.length,
                Readout::Displacement {
                    fraction,
                    aperture_m,
                } => {
                    let half_width = PI * order * aperture_m / (2. * spec.length);
                    let aperture = if half_width == 0. {
                        1.
                    } else {
                        half_width.sin() / half_width
                    };
                    (PI * order * fraction).sin() * aperture
                }
            };
            for (i, s) in p.strings.iter_mut().take(spec.strings).enumerate() {
                let cents = if spec.strings == 2 {
                    (i as f64 - 0.5) * spec.unison_cents
                } else {
                    0.
                };
                let omega = TAU * freq * 2f64.powf(cents / 1200.);
                let (sn, cs) = (omega / rate).sin_cos();
                s.sin_step.set_lane(lane, radius * sn);
                s.cos_step.set_lane(lane, radius * cs);
                let (sn, cs) = (steps as f64 * omega / rate).sin_cos();
                s.sin_host.set_lane(lane, sn * radius.powi(steps as i32));
                s.cos_host.set_lane(lane, cs * radius.powi(steps as i32));
                s.contact_read.set_lane(lane, contact / omega);
                let weight = 1. + (2. * i as f64 - 1.) * spec.read_imbalance;
                s.read_weight.set_lane(lane, readout / omega * weight);
            }
            p.impulse.set_lane(lane, contact / (modal_mass * rate));
            p.common_loss
                .set_lane(lane, 0.5 * (-0.12 * order.sqrt() / rate).exp_m1());
            let damping = -spec.damper_loss * (1. + order.sqrt() * 0.14) / rate;
            p.damper.set_lane(lane, damping.exp());
            p.host_damper.set_lane(lane, (steps as f64 * damping).exp());
        }
        Self {
            spec,
            partials,
            steps,
            dt: 1. / rate,
            hammer_pos: 0.,
            hammer_speed: 0.,
            hammer_gravity: 0.,
            hammer_active: false,
            key_down: false,
            pedal: false,
            rail_loss: Vec::new(),
            rail_down: false,
        }
    }

    pub fn strike_speed(&mut self, speed_m_s: f64) {
        if speed_m_s <= 0. {
            self.release();
            return;
        }
        self.lift_damper();
        self.launch_hammer(speed_m_s, 0.001, 0.);
    }

    /// Release an already moving striker from a measured clearance. The key damper
    /// is independent because escapement can precede key-up.
    pub fn launch_hammer(&mut self, speed_m_s: f64, clearance_m: f64, gravity_m_s2: f64) {
        self.hammer_active = true;
        self.hammer_pos = -clearance_m;
        self.hammer_speed = speed_m_s;
        self.hammer_gravity = gravity_m_s2;
    }

    pub fn lift_damper(&mut self) {
        self.key_down = true;
    }

    /// A weak viscous contact dissipates energy in proportion to the squared
    /// mode shape at its position. Store the exact free-decay factors for both rates.
    pub fn configure_felt_rail(&mut self, fraction: f64, loss_per_second: f64) {
        assert!((0.0..=1.0).contains(&fraction) && loss_per_second >= 0.);
        self.rail_loss = (0..self.partials.len())
            .map(|group| {
                let mut tick = V::ONE;
                let mut host = V::ONE;
                for lane in 0..V::WIDTH {
                    let order = (1 + group * V::WIDTH + lane) as f64;
                    let decay = -loss_per_second * (PI * fraction * order).sin().powi(2) * self.dt;
                    tick.set_lane(lane, decay.exp());
                    host.set_lane(lane, (self.steps as f64 * decay).exp());
                }
                (tick, host)
            })
            .collect();
    }

    pub fn set_felt_rail(&mut self, down: bool) {
        self.rail_down = down && !self.rail_loss.is_empty();
    }

    pub fn release(&mut self) {
        self.key_down = false;
    }
    pub fn sustain(&mut self, down: bool) {
        self.pedal = down;
    }

    pub fn clear(&mut self) {
        for p in &mut self.partials {
            for s in &mut p.strings {
                s.position = V::ZERO;
                s.velocity = V::ZERO;
            }
        }
        self.key_down = false;
        self.hammer_active = false;
    }
    pub fn quiet(&self, threshold: f64) -> bool {
        if self.hammer_active {
            return false;
        }
        self.partials
            .iter()
            .flat_map(|p| &p.strings[..self.spec.strings])
            .map(|s| {
                ((s.position.magnitude() + s.velocity.magnitude()) * s.read_weight.magnitude())
                    .sum_lanes()
            })
            .sum::<f64>()
            < threshold
    }

    /// Resolve contact at the mechanical integration rate. During free vibration
    /// compose the rotations analytically while keeping state available for restrikes.
    pub fn host_sample(&mut self) -> f64 {
        if self.hammer_active {
            for _ in 1..self.steps {
                self.step();
            }
            return self.step();
        }
        if self.spec.strings == 1 {
            self.advance::<false, true>([0.; 2])
        } else {
            self.advance::<true, true>([0.; 2])
        }
    }

    /// Advance one contact-rate step. Units follow the configured readout.
    pub fn step(&mut self) -> f64 {
        if self.spec.strings == 1 {
            self.substep::<false>()
        } else {
            self.substep::<true>()
        }
    }

    #[inline]
    fn substep<const PAIRED: bool>(&mut self) -> f64 {
        let mut forces = [0.; 2];
        if self.hammer_active {
            let mut surfaces = [V::ZERO; 2];
            for p in &self.partials {
                surfaces[0] += p.strings[0].contact_read * p.strings[0].position;
                if PAIRED {
                    surfaces[1] += p.strings[1].contact_read * p.strings[1].position;
                }
            }
            let share = if PAIRED { 0.5 } else { 1.0 };
            let quadratic = self.spec.contact_k * share;
            let linear = self.spec.contact_linear * share;
            for i in 0..if PAIRED { 2 } else { 1 } {
                let compression = (self.hammer_pos - surfaces[i].sum_lanes()).max(0.);
                forces[i] = quadratic * compression.powi(2) + linear * compression;
            }
            self.hammer_speed -= self.dt * (forces[0] + forces[1]) / self.spec.hammer_kg;
            self.hammer_speed -= self.dt * self.hammer_gravity;
            self.hammer_pos += self.dt * self.hammer_speed;
            if self.hammer_speed < 0. && self.hammer_pos < -0.004 {
                self.hammer_active = false;
            }
        }
        self.advance::<PAIRED, false>(forces)
    }

    #[inline]
    fn advance<const PAIRED: bool, const FREE: bool>(&mut self, forces: [f64; 2]) -> f64 {
        let damp = self.spec.has_damper && !self.key_down && !self.pedal;
        let mut bearing = V::ZERO;
        let steps = self.steps as f64;
        for (i, p) in self.partials.iter_mut().enumerate() {
            let common = if PAIRED {
                let loss = if FREE {
                    p.common_loss * V::uniform(steps)
                } else {
                    p.common_loss
                };
                loss * (p.strings[0].velocity + p.strings[1].velocity)
            } else {
                V::ZERO
            };
            let mut damping = if damp {
                if FREE {
                    p.host_damper
                } else {
                    p.damper
                }
            } else {
                V::ONE
            };
            if self.rail_down {
                let (tick, host) = self.rail_loss[i];
                damping = damping * if FREE { host } else { tick };
            }
            for (i, s) in p
                .strings
                .iter_mut()
                .take(if PAIRED { 2 } else { 1 })
                .enumerate()
            {
                let mut v = s.velocity + common;
                if !FREE {
                    v += p.impulse * V::uniform(forces[i]);
                }
                let (sn, cs) = if FREE {
                    (s.sin_host, s.cos_host)
                } else {
                    (s.sin_step, s.cos_step)
                };
                bearing += s.advance(sn, cs, v, damping);
            }
        }
        bearing.sum_lanes()
    }
}
