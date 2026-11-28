//! Three-course Helpinstill Roadmaster 64 mechanics and separate sensor/board taps.
pub mod live;
use pleasant_dsp::instrument::Decimator;
use std::f64::consts::{PI, TAU};

pub const PILOT: [u8; 3] = [48, 60, 84];
pub const LOW: u8 = 33;
pub const HIGH: u8 = 96;
pub const LATENCY: usize = 12;

type Three = [f64; 3];

fn dot(a: Three, b: Three) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
fn mv(m: [Three; 3], x: Three) -> Three {
    m.map(|row| dot(row, x))
}
fn inverse(mut m: [Three; 3]) -> [Three; 3] {
    let mut inv = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for i in 0..3 {
        let pivot = m[i][i];
        assert!(pivot > 0. && pivot.is_finite());
        for k in 0..3 {
            m[i][k] /= pivot;
            inv[i][k] /= pivot;
        }
        for j in 0..3 {
            if j == i {
                continue;
            }
            let a = m[j][i];
            for k in 0..3 {
                m[j][k] -= a * m[i][k];
                inv[j][k] -= a * inv[i][k];
            }
        }
    }
    inv
}

struct Mode {
    x: f64,
    v: f64,
    mass: f64,
    omega2: f64,
    choke_loss: f64,
    decay: f64,
    hammer: f64,
    sensor: f64,
    bridge: f64,
    acceleration: f64,
}
struct Course {
    choked: bool,
    band: usize,
    native: bool,
    sounding: bool,
    returning: u32,
    damped_modes: usize,
    residual_mass: [f64; 2],
    modes: Vec<Mode>,
    attach: Three,
    count: usize,
    tension: f64,
    length: f64,
    strike_at: f64,
    listen_at: f64,
    hammer_mass: f64,
    hammer_k: f64,
    hammer_x: f64,
    hammer_v: f64,
    striking: bool,
    held: bool,
}
impl Course {
    fn new(note: u8, internal: f64, live: bool) -> Self {
        let native = note.clamp(LOW, HIGH);
        let scale = |anchors: [f64; 5]| -> f64 {
            let notes = [LOW, 48, 60, 84, HIGH];
            let i = notes.windows(2).position(|v| native <= v[1]).unwrap_or(3);
            let t = f64::from(native - notes[i]) / f64::from(notes[i + 1] - notes[i]);
            anchors[i] * (1. - t) + anchors[i + 1] * t
        };
        let freq = 440. * 2_f64.powf((f64::from(note) - 69.) / 12.);
        let f_native = 440. * 2_f64.powf((f64::from(native) - 69.) / 12.);
        let length = scale([1.05, 0.90, 0.56, 0.14, 0.055]);
        let stock_tension = scale([450., 470., 535., 530., 505.]);
        let tension = stock_tension * (freq / f_native).powi(2);
        let linear_density = stock_tension / (2. * f_native * length).powi(2);
        let string_mass = linear_density * length;
        let strike_at = scale([0.13, 0.125, 0.121, 0.102, 0.082]);
        let listen_at = scale([0.24, 0.22, 0.18, 0.14, 0.18]);
        let hammer_width = scale([0.0085, 0.007, 0.0055, 0.004, 0.0025]);
        let stiff = scale([0.0011, 0.00053, 0.00015, 0.00030, 0.0011]);
        let loss = scale([0.94, 1.15, 1.36, 2.05, 3.7]);
        let attach = [
            scale([0.65, 0.86, 1., 0.66, 0.39]),
            scale([0.38, 0.55, -0.43, -0.78, -0.43]),
            scale([-0.23, -0.28, 0.72, -0.51, -0.70]),
        ];
        let i = if note < 54 {
            0
        } else if note < 75 {
            1
        } else {
            2
        };
        let mut modes = vec![];
        let count = 2;
        for strand in 0..count {
            let detuning = 2_f64.powf((strand as f64 - 0.5) * 0.9 / 1200.);
            for n in 1..=60 {
                let order = n as f64;
                let f =
                    freq * order * ((1. + stiff * order * order) / (1. + stiff)).sqrt() * detuning;
                if n > 1 && f > 6500. {
                    break;
                }
                let area = |at: f64, width: f64| {
                    let w = PI * order * width / (2. * length);
                    (PI * order * at).sin() * w.sin() / w
                };
                modes.push(Mode {
                    x: 0.,
                    v: 0.,
                    mass: string_mass / 2.,
                    omega2: if live {
                        (2. * internal * (PI * f / internal).sin()).powi(2)
                    } else {
                        (TAU * f).powi(2)
                    },
                    choke_loss: (2. * TAU * f).min(1400.),
                    decay: loss * (1. + 0.040 * order.powf(1.46)),
                    hammer: area(strike_at, hammer_width),
                    sensor: area(listen_at, 0.0127),
                    bridge: if n % 2 == 0 { -1. } else { 1. } * string_mass / (PI * order),
                    acceleration: 0.,
                });
            }
        }
        let damped_modes = if live {
            3.min(modes.len() / count)
        } else {
            modes.len() / count
        };
        let residual_mass = [damped_modes, modes.len() / count].map(|limit| {
            let strand_len = modes.len() / count;
            let cross: f64 = modes
                .chunks(strand_len)
                .flat_map(|s| &s[..limit])
                .map(|m| m.bridge * m.bridge / m.mass)
                .sum();
            count as f64 * string_mass / 3. - cross
        });
        Self {
            choked: false,
            band: i,
            native: note == native,
            sounding: !live,
            returning: 0,
            damped_modes,
            residual_mass,
            modes,
            attach,
            count,
            tension,
            length,
            strike_at,
            listen_at,
            hammer_mass: scale([0.0105, 0.009, 0.0065, 0.0045, 0.0032]),
            hammer_k: scale([1.15e8, 1.7e8, 3.5e8, 6e8, 9e8]),
            hammer_x: 0.,
            hammer_v: 0.,
            striking: false,
            held: false,
        }
    }
}

/// Electrical pickup EMF from each bar and area-weighted board velocity in m/s.
/// The latter is an acoustic source observable ahead of room and microphone.
#[derive(Clone, Copy, Default)]
pub struct Taps {
    pub sensor_v: Three,
    pub surface_m_s: f64,
}
pub struct Probe {
    keys: Vec<Course>,
    live: bool,
    board_x: Three,
    board_v: Three,
    board_mass: Three,
    board_k: Three,
    board_c: Three,
    inverse_mass: [Three; 3],
    dt: f64,
    steps: usize,
    sustain: bool,
    mobility: bool,
    gap_gain: Three,
    prev_flux: Three,
    decimator: Decimator<4>,
}
impl Probe {
    pub fn new(rate: f64, substeps: usize, mobile_board: bool) -> Self {
        Self::from_scale(rate, substeps, mobile_board, &PILOT, false)
    }
    fn from_scale(
        rate: f64,
        substeps: usize,
        mobile_board: bool,
        notes: &[u8],
        live: bool,
    ) -> Self {
        assert!(rate.is_finite() && rate >= 22050. && substeps >= 1);
        let keys: Vec<_> = notes
            .iter()
            .map(|&n| Course::new(n, rate * substeps as f64, live))
            .collect();
        let board_mass = [1.75, 1.15, 0.68];
        let board_hz = [105., 181., 313.];
        let board_q = [18., 22., 19.];
        let board_k = std::array::from_fn(|i| board_mass[i] * (TAU * board_hz[i]).powi(2));
        let board_c = std::array::from_fn(|i| board_mass[i] * TAU * board_hz[i] / board_q[i]);
        let inverse_mass = Self::schur(&keys, board_mass);
        Self {
            keys,
            live,
            board_x: [0.; 3],
            board_v: [0.; 3],
            board_mass,
            board_k,
            board_c,
            inverse_mass,
            dt: 1. / (rate * substeps as f64),
            steps: substeps,
            sustain: false,
            mobility: mobile_board,
            gap_gain: [1.; 3],
            prev_flux: [0.; 3],
            decimator: Decimator::new(substeps, rate, LATENCY),
        }
    }
    fn schur(keys: &[Course], board_mass: Three) -> [Three; 3] {
        let mut m = [[0.; 3]; 3];
        for i in 0..3 {
            m[i][i] = board_mass[i];
        }
        for key in keys.iter().filter(|k| k.native || k.sounding) {
            let residual = key.residual_mass[usize::from(key.sounding)];
            for (row, u) in m.iter_mut().zip(key.attach) {
                for (element, v) in row.iter_mut().zip(key.attach) {
                    *element += u * v * residual;
                }
            }
        }
        inverse(m)
    }
    pub fn strike(&mut self, key: usize, velocity: f64) {
        let k = &mut self.keys[key];
        let add_mass = !k.sounding;
        k.held = true;
        k.sounding = true;
        k.choked = false;
        k.returning = 0;
        k.striking = true;
        k.hammer_x = -0.0015;
        k.hammer_v = 0.40 + 3.45 * velocity.clamp(0., 1.).powf(1.5);
        if add_mass {
            self.inverse_mass = Self::schur(&self.keys, self.board_mass);
        }
    }
    pub fn release(&mut self, key: usize) {
        self.keys[key].held = false;
    }
    pub fn sustain(&mut self, down: bool) {
        self.sustain = down;
    }
    /// Bench adjustment along the string, measured as a fraction from the fixed end.
    /// Factory bar positions require a trace of the original installation.
    pub fn position_sensor(&mut self, band: usize, fraction: f64) {
        assert!(band < 3 && fraction.is_finite() && (0.025..0.98).contains(&fraction));
        for key in self.keys.iter_mut().filter(|key| key.band == band) {
            key.listen_at = fraction;
            let len = key.modes.len() / key.count;
            for strand in key.modes.chunks_mut(len) {
                for (i, mode) in strand.iter_mut().enumerate() {
                    let n = (i + 1) as f64;
                    let w = PI * n * 0.0127 / (2. * key.length);
                    mode.sensor = (PI * n * fraction).sin() * w.sin() / w;
                }
            }
        }
    }
    pub fn set_gap_mm(&mut self, offset: f64) {
        if offset.is_finite() {
            self.gap_gain = [(-offset.clamp(-1., 2.) / 3.5).exp(); 3];
        }
    }
    fn step(&mut self) -> [f64; 4] {
        let mut board_force = std::array::from_fn(|i| {
            -self.board_k[i] * self.board_x[i] - self.board_c[i] * self.board_v[i]
        });
        for key in &mut self.keys {
            if !key.native && !key.sounding {
                continue;
            }
            let support = dot(key.attach, self.board_x);
            let middle = key.count;
            let strand_len = key.modes.len() / middle;
            let mut support_force = -(middle as f64) * key.tension / key.length * support;
            let mut hammer_force = 0.;
            let limit = if key.sounding {
                strand_len
            } else {
                key.damped_modes
            };
            for group in key.modes.chunks_mut(strand_len) {
                let modes = &mut group[..limit];
                let y = if key.striking {
                    key.strike_at * support + modes.iter().map(|m| m.hammer * m.x).sum::<f64>()
                } else {
                    0.
                };
                let compression = (key.hammer_x - y).max(0.);
                let force = if key.striking {
                    key.hammer_k * compression.powf(2.3)
                } else {
                    0.
                };
                hammer_force += force;
                support_force += key.strike_at * force;
                for m in modes {
                    let loss = m.decay
                        + if key.choked {
                            m.choke_loss
                        } else if key.held || self.sustain {
                            0.
                        } else {
                            100.
                        };
                    m.acceleration = force * m.hammer / m.mass - m.omega2 * m.x - loss * m.v;
                    support_force -= m.bridge * m.acceleration;
                }
            }
            if key.striking {
                key.hammer_v -= self.dt * hammer_force / key.hammer_mass;
                key.hammer_x += self.dt * key.hammer_v;
                if key.hammer_x < -0.002 && key.hammer_v < 0. {
                    key.striking = false;
                }
            }
            for (i, f) in board_force.iter_mut().enumerate() {
                *f += key.attach[i] * support_force;
            }
        }
        let acceleration = if self.mobility {
            mv(self.inverse_mass, board_force)
        } else {
            [0.; 3]
        };
        for ((position, velocity), force_over_mass) in self
            .board_x
            .iter_mut()
            .zip(&mut self.board_v)
            .zip(acceleration)
        {
            *velocity += self.dt * force_over_mass;
            *position += self.dt * *velocity;
        }
        let mut output = [0.; 4];
        let mut flux = [0.; 3];
        for key in &mut self.keys {
            if !key.native && !key.sounding {
                continue;
            }
            let acc = dot(key.attach, acceleration);
            let support = dot(key.attach, self.board_x);
            let mut linkage = 0.;
            let strand_len = key.modes.len() / key.count;
            let limit = if key.sounding {
                strand_len
            } else {
                key.damped_modes
            };
            for group in key.modes.chunks_mut(strand_len) {
                let modes = &mut group[..limit];
                let mut displacement = key.listen_at * support;
                for m in modes {
                    m.v += self.dt * (m.acceleration - m.bridge / m.mass * acc);
                    m.x += self.dt * m.v;
                    displacement += m.sensor * m.x;
                }
                // Differentiate a change of flux from rest. The common pickup
                // offset changes sensitivity without moving the string equilibrium.
                let field = 0.0035;
                let ratio = displacement / field;
                linkage += if ratio.abs() < 0.0001 {
                    ratio * (1. + 0.5 * ratio)
                } else {
                    ratio.exp_m1()
                };
            }
            flux[key.band] += 300. * 1.60e-8 * linkage;
        }
        for i in 0..3 {
            output[i] = self.gap_gain[i] * (flux[i] - self.prev_flux[i]) / self.dt;
        }
        self.prev_flux = flux;
        output[3] = dot([0.76, -0.22, 0.13], self.board_v);
        output
    }
    pub fn sample(&mut self) -> Taps {
        if self.live {
            let mut mass_changed = false;
            for key in &mut self.keys {
                if key.choked || (key.sounding && !key.held && !self.sustain) {
                    key.returning += 1;
                    if key.returning >= (0.42 / (self.dt * self.steps as f64)).ceil() as u32 {
                        mass_changed |= key.sounding;
                        key.sounding = false;
                        key.choked = false;
                        key.striking = false;
                        key.returning = 0;
                        let strand_len = key.modes.len() / key.count;
                        let keep = if key.native { key.damped_modes } else { 0 };
                        for group in key.modes.chunks_mut(strand_len) {
                            for m in &mut group[keep..] {
                                m.x = 0.;
                                m.v = 0.;
                            }
                        }
                    }
                }
            }
            if mass_changed {
                self.inverse_mass = Self::schur(&self.keys, self.board_mass);
            }
        }
        for _ in 0..self.steps {
            let x = self.step();
            self.decimator.push(x);
        }
        let block = self.decimator.read::<4>();
        Taps {
            sensor_v: [block[0], block[1], block[2]],
            surface_m_s: block[3],
        }
    }
    pub fn board_energy(&self) -> f64 {
        (0..3)
            .map(|i| {
                0.5 * (self.board_mass[i] * self.board_v[i].powi(2)
                    + self.board_k[i] * self.board_x[i].powi(2))
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(p: &Probe) -> f64 {
        let mut e = p.board_energy();
        for k in &p.keys {
            let displacement = dot(k.attach, p.board_x);
            let velocity = dot(k.attach, p.board_v);
            let mass = 2. * k.modes[0].mass;
            e += 0.5
                * k.count as f64
                * (mass / 3. * velocity * velocity
                    + k.tension / k.length * displacement * displacement);
            for m in &k.modes {
                e += m.mass * (m.v * m.v + m.omega2 * m.x * m.x) / 2. + m.bridge * m.v * velocity;
            }
        }
        e
    }
    #[test]
    fn mobile_support_exchanges_energy_and_passively_decays() {
        for i in 0..3 {
            let mut p = Probe::new(48000., 4, true);
            p.sustain(true);
            p.strike(i, 0.7);
            for _ in 0..4800 {
                p.sample();
            }
            assert!(!p.keys[i].striking && p.board_energy() > 0.);
            let start = stored(&p);
            let mut peak = start;
            for _ in 0..12000 {
                p.sample();
                peak = f64::max(peak, stored(&p));
            }
            assert!(start > 0. && peak < start * 1.02 && stored(&p) < start);
            assert!(p
                .keys
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .all(|(_, k)| k.modes.iter().any(|m| m.v.abs() > 1e-7)));
        }
    }
    #[test]
    fn fixed_support_and_gap_isolate_mechanics_from_sensor() {
        let mut a = Probe::new(48000., 4, false);
        let mut b = Probe::new(48000., 4, false);
        a.strike(0, 0.72);
        b.strike(0, 0.72);
        b.set_gap_mm(1.);
        for _ in 0..4800 {
            let x = a.sample();
            let y = b.sample();
            assert_eq!(x.surface_m_s, 0.);
            assert_eq!(x.sensor_v[1..], [0., 0.]);
            assert!((x.sensor_v[0] * (-1.0_f64 / 3.5).exp() - y.sensor_v[0]).abs() < 1e-12);
        }
    }
    #[test]
    fn four_substeps_predict_the_refined_pickup_peak() {
        for i in 0..3 {
            let mut peaks = [0f64; 2];
            for (out, steps) in peaks.iter_mut().zip([4, 8]) {
                let mut p = Probe::new(48000., steps, true);
                p.strike(i, 0.8);
                for _ in 0..3600 {
                    *out = out.max(p.sample().sensor_v[i].abs());
                }
            }
            assert!(
                (peaks[0] / peaks[1] - 1.).abs() < 0.055,
                "course {i}: {peaks:?}"
            );
        }
    }
}
