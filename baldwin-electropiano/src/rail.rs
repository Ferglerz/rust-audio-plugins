//! A short section of the U-channel at each grid point. The upper lip bends
//! on its folded base. Adjacent lip sections share flexural stresses. A ceramic
//! and rubber/shim contact on every other section transmits compression to ground.
use crate::action::{frequency, length_m, tension_n, HIGH, LOW};
use std::f64::consts::PI;

pub const SENSORS: usize = 22;
pub const CELLS: usize = 2 * SENSORS;
// Approximate six ceramic stations on the wound-string section, sixteen on treble.
const BASS_CELLS: usize = 12;
const PITCH_M: f64 = 0.028;

#[derive(Clone, Copy)]
struct Position {
    left: usize,
    right: usize,
    fraction: f64,
}
impl Position {
    fn within(x: f64, bass: bool) -> Self {
        let (lo, hi) = if bass {
            (0, BASS_CELLS - 1)
        } else {
            (BASS_CELLS, CELLS - 1)
        };
        let x = x.clamp(lo as f64, hi as f64);
        let left = x.floor() as usize;
        Self {
            left,
            right: (left + 1).min(hi),
            fraction: x - left as f64,
        }
    }
    fn read(self, data: &[f64; CELLS]) -> f64 {
        data[self.left] + self.fraction * (data[self.right] - data[self.left])
    }
    fn add(self, data: &mut [f64; CELLS], value: f64) {
        data[self.left] += (1. - self.fraction) * value;
        data[self.right] += self.fraction * value;
    }
}

pub fn section(note: u8) -> usize {
    usize::from(note >= LOW + 2 * BASS_CELLS as u8)
}
fn key_position(note: u8) -> Position {
    let x = (f64::from(note.clamp(LOW, HIGH) - LOW) + 0.5) / 2. - 0.5;
    Position::within(x, section(note) == 0)
}

/// Drive force of each struck course, in newtons, at its bearing.
pub struct BearingForces {
    forces: [f64; CELLS],
}
impl Default for BearingForces {
    fn default() -> Self {
        Self {
            forces: [0.; CELLS],
        }
    }
}
impl BearingForces {
    pub fn add(&mut self, n: u8, force: f64) {
        key_position(n).add(&mut self.forces, force);
    }
}

struct Resonance {
    mass: f64,
    stiffness: f64,
    displacement: f64,
    speed: f64,
}
struct StringLoad {
    port: Position,
    resonances: [Resonance; 3],
    undamped: bool,
}
impl StringLoad {
    fn new(note: u8, rate: f64) -> Self {
        let hz = frequency(note);
        let length = length_m(note);
        let mass = tension_n(note) / (4. * hz * hz * length);
        Self {
            port: key_position(note),
            undamped: note >= 90,
            resonances: std::array::from_fn(|i| {
                let h = (i + 1) as f64;
                // Reflected modal mass for a linearly displaced endpoint.
                let effective = 2. * mass / (PI * h).powi(2);
                let omega = 2. * rate * (PI * h * hz / rate).sin();
                Resonance {
                    mass: effective,
                    stiffness: effective * omega * omega,
                    displacement: 0.,
                    speed: 0.,
                }
            }),
        }
    }
}

pub struct Rail {
    displacement: [f64; CELLS],
    speed: [f64; CELLS],
    strings: Vec<StringLoad>,
    pads: [Position; SENSORS],
    offset_mm: f64,
    dt: f64,
    pad_k: f64,
    pad_c: f64,
}
impl Rail {
    pub fn new(rate: f64) -> Self {
        Self::with_shim(rate, 250_000., 10.)
    }
    /// Dynamic compression stiffness and damping of each preloaded rubber/ceramic stack.
    pub fn with_shim(rate: f64, stiffness_n_m: f64, damping_ns_m: f64) -> Self {
        Self {
            displacement: [0.; CELLS],
            speed: [0.; CELLS],
            strings: (LOW..=HIGH).map(|n| StringLoad::new(n, rate)).collect(),
            pads: Self::pads_at(0.),
            offset_mm: 0.,
            dt: 1. / rate,
            pad_k: stiffness_n_m,
            pad_c: damping_ns_m,
        }
    }
    fn pads_at(offset: f64) -> [Position; SENSORS] {
        std::array::from_fn(|p| {
            Position::within(
                2. * p as f64 + 0.5 + offset * 0.001 / PITCH_M,
                p < BASS_CELLS / 2,
            )
        })
    }
    /// Lateral position of the compression elements along the rail, in mm.
    /// Moving a support changes both the load on the lip and the sensed pressure.
    pub fn set_offset_mm(&mut self, offset: f64) {
        if offset.is_finite() {
            let offset = offset.clamp(-8., 8.);
            if offset != self.offset_mm {
                self.pads = Self::pads_at(offset);
                self.offset_mm = offset;
            }
        }
    }
    pub fn reset(&mut self) {
        self.displacement.fill(0.);
        self.speed.fill(0.);
        for s in &mut self.strings {
            for m in &mut s.resonances {
                m.displacement = 0.;
                m.speed = 0.;
            }
        }
    }
    pub fn sample(&mut self, bearings: BearingForces, held: u128, sustain: bool) -> [f64; 2] {
        let mut force = bearings.forces;
        // Euler–Bernoulli lip stiffness across the U and along the extrusion.
        // Geometry is an estimate (2.0 mm aluminum lip, 26 mm cantilever).
        const METAL: f64 = 69e9;
        const THICK: f64 = 0.002;
        const ARM: f64 = 0.026;
        const EFFECTIVE_KG: f64 = 0.011;
        const ROOT_K: f64 = METAL * PITCH_M * THICK * THICK * THICK / (4. * ARM * ARM * ARM);
        const SPAN_K: f64 =
            METAL * ARM * THICK * THICK * THICK / (12. * PITCH_M * PITCH_M * PITCH_M);
        for (i, f) in force.iter_mut().enumerate() {
            *f -= ROOT_K * self.displacement[i] + 2.1 * self.speed[i];
        }
        for run in [0..BASS_CELLS, BASS_CELLS..CELLS] {
            for i in run.start + 1..run.end - 1 {
                let flex = SPAN_K
                    * (self.displacement[i - 1] - 2. * self.displacement[i]
                        + self.displacement[i + 1]);
                force[i - 1] -= flex;
                force[i] += 2. * flex;
                force[i + 1] -= flex;
            }
        }
        let mut pressure = [0.; 2];
        for (i, &p) in self.pads.iter().enumerate() {
            let f = self.pad_k * p.read(&self.displacement) + self.pad_c * p.read(&self.speed);
            p.add(&mut force, -f);
            pressure[usize::from(i >= BASS_CELLS / 2)] += f;
        }
        // These passive modes are the reaction of the unstruck and struck
        // strings to movement of the rail. The fixed-end hammer solution is
        // superposed at bearings.forces. Both sides receive equal/opposite force.
        for (i, string) in self.strings.iter_mut().enumerate() {
            let loose =
                sustain || string.undamped || (held & (1u128 << (usize::from(LOW) + i))) != 0;
            let drag = if loose { 2.8 } else { 148. };
            let port = string.port;
            let y = port.read(&self.displacement);
            let v = port.read(&self.speed);
            let mut total = 0.;
            for m in &mut string.resonances {
                let reaction = m.stiffness * (y - m.displacement) + drag * m.mass * (v - m.speed);
                m.speed += self.dt * reaction / m.mass;
                m.displacement += self.dt * m.speed;
                total += reaction;
            }
            port.add(&mut force, -total);
        }
        for (i, f) in force.into_iter().enumerate() {
            self.speed[i] += self.dt * f / EFFECTIVE_KG;
            self.displacement[i] += self.dt * self.speed[i];
        }
        pressure
    }
    #[cfg(test)]
    fn string_energy(&self, n: u8) -> f64 {
        let s = &self.strings[usize::from(n - LOW)];
        s.resonances
            .iter()
            .map(|m| {
                0.5 * (m.mass * m.speed.powi(2)
                    + m.stiffness * (m.displacement - s.port.read(&self.displacement)).powi(2))
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unstruck_neighbor_accepts_work_through_its_section_of_rail() {
        let mut r = Rail::new(48000.);
        let mut peak: f64 = 0.;
        for i in 0..16000 {
            let mut f = BearingForces::default();
            if i < 9000 {
                f.add(62, (2. * PI * frequency(61) * f64::from(i) / 48000.).sin());
            }
            let [bass, treble] = r.sample(f, 0, true);
            assert_eq!(bass, 0.);
            peak = peak.max(treble.abs());
        }
        assert!(peak > 0.001);
        assert!(r.string_energy(61) > 1e-15);
        assert_eq!(r.string_energy(35), 0.);
    }
}
