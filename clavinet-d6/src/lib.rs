//! Offline transverse stiff-string reference for three D6 keys. SI units internally.
//! The tailpiece and pin are simple supports. Positive displacement points to the anvil.
pub mod live;
pub mod panel;
pub mod vibanet;

use std::f64::consts::PI;

pub const OUTPUT_RATE: u32 = 44_100;
pub const SUBSTEPS: usize = 8;
pub const KEYS: [u8; 3] = [46, 64, 78];

#[derive(Clone, Copy)]
struct Key {
    length: f64,
    tension_pitch_hz: f64,
    anvil_m: f64,
    core_mass_per_m: f64,
    winding_mass_per_m: f64,
    inharmonicity: f64,
    centre_m: f64,
    bridge_m: f64,
    loss: f64,
    cells: usize,
}

fn key(midi: u8) -> Option<Key> {
    // Geometry, action estimates and tension fits are documented in README.md.
    match midi {
        46 => Some(Key {
            length: 0.8,
            tension_pitch_hz: 116.17,
            anvil_m: 0.0018,
            core_mass_per_m: 0.0014,
            winding_mass_per_m: 0.0019,
            inharmonicity: 0.00015,
            centre_m: 0.16,
            bridge_m: 0.042,
            loss: 0.35,
            cells: 112,
        }),
        64 => Some(Key {
            length: 0.44,
            tension_pitch_hz: 329.67,
            anvil_m: 0.0018,
            core_mass_per_m: 0.00089,
            winding_mass_per_m: 0.0,
            inharmonicity: 0.0001,
            centre_m: 0.113,
            bridge_m: 0.04,
            loss: 0.84,
            cells: 80,
        }),
        78 => Some(Key {
            length: 0.174,
            tension_pitch_hz: 737.18,
            anvil_m: 0.0024,
            core_mass_per_m: 0.00055,
            winding_mass_per_m: 0.0,
            inharmonicity: 9e-05,
            centre_m: 0.084,
            bridge_m: 0.038,
            loss: 2.15,
            cells: 64,
        }),
        _ => None,
    }
}

#[derive(Clone, Copy, Default)]
pub struct Pickups {
    pub centre: f64,
    pub bridge: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    CA,
    CB,
    DA,
    DB,
}

/// Which paired position adds the two terminal voltages in our electrical convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wiring {
    DaAdds,
    DaSubtracts,
}

impl Switch {
    pub fn output(self, p: Pickups, wiring: Wiring) -> f64 {
        panel::Board::D6.selector_output(self, wiring, p)
    }
}

// Five midpoint samples of a 5 mm aperture. Winding references give equal voltage signs
// for small positive motion at either pickup despite the opposite magnetic gap gradients.
struct Pickup {
    taps: [(usize, f64); 5],
    side: f64,
    previous_flux: f64,
    displacement: f64,
    distance_mm: f64,
    field_length: f64,
    sensitivity: f64,
}

impl Pickup {
    const FIELD_LENGTH: f64 = 0.0035;
    fn new(metres: f64, dx: f64, side: f64) -> Self {
        let taps = std::array::from_fn(|i| {
            let cell = (metres + (i as f64 - 2.0) * 0.001) / dx;
            (cell.floor() as usize + 1, cell.fract())
        });
        Self {
            taps,
            side,
            previous_flux: 0.0,
            displacement: 0.0,
            distance_mm: 0.,
            field_length: Self::FIELD_LENGTH,
            sensitivity: 1.,
        }
    }

    fn set_distance_mm(&mut self, distance: f64, string: &[f64]) {
        let distance = distance.clamp(-1., 1.);
        if !distance.is_finite() || self.distance_mm == distance {
            return;
        }
        self.distance_mm = distance;
        // The local gradient of a dipole field decreases as inverse distance
        // to the fourth power. Its logarithmic length grows with clearance.
        self.field_length = Self::FIELD_LENGTH + distance * 0.001 / 4.;
        self.sensitivity = (Self::FIELD_LENGTH / self.field_length).powi(4);
        // Compare consecutive string positions at the new sensor setting.
        self.sense::<false>(string, 1.);
    }

    #[inline]
    fn sense<const REFERENCE: bool>(&mut self, string: &[f64], rate: f64) -> f64 {
        let mut flux = 0.0;
        self.displacement = 0.0;
        for &(i, fraction) in &self.taps {
            let y = string[i] + fraction * (string[i + 1] - string[i]);
            self.displacement += y / 5.0;
            let z = self.side * y / self.field_length;
            // Degree-seven Taylor approximation over one field length. Its absolute
            // error is below 6.8e-5; unusually large excursions use the exact curve.
            let induction = if REFERENCE || z.abs() > 1.0 {
                z.exp_m1()
            } else {
                z * (1.0
                    + z * (0.5
                        + z * (1.0 / 6.0
                            + z * (1.0 / 24.0
                                + z * (1.0 / 120.0 + z * (1.0 / 720.0 + z / 5040.0))))))
            };
            flux += self.side * self.field_length * induction / 5.0;
        }
        let voltage = (flux - self.previous_flux) * rate;
        self.previous_flux = flux;
        voltage * self.sensitivity
    }
}

pub struct Trace {
    /// Unloaded rubber tip face, measured from the resting string toward the anvil.
    pub tip_position_m: f64,
    /// String displacement at the tangent/anvil contact.
    pub tangent_m: f64,
    pub centre_m: f64,
    pub bridge_m: f64,
    pub tip_n: f64,
    pub anvil_n: f64,
    pub contact_work_j: f64,
    pub viscous_loss_j: f64,
    pub stored_j: f64,
}

/// Longitudinal geometry in metres. Zero is the tailpiece.
pub struct Geometry {
    pub speaking_m: f64,
    pub afterlength_m: f64,
    pub centre_m: f64,
    pub bridge_m: f64,
    pub anvil_drop_m: f64,
}

pub struct StringVoice {
    geometry: Geometry,
    current: Vec<f64>,
    past: Vec<f64>,
    next: Vec<f64>,
    drag: Vec<f64>,
    inv: Vec<f64>,
    gel: Vec<vibanet::GelCell>,
    gel_tangent_inv: f64,
    tangent: usize,
    rate: f64,
    tension_step: f64,
    bending_step: f64,
    viscous_step: f64,
    cell_mass: f64,
    tip_ratio: f64,
    steel_loss: f64,
    anvil_m: f64,
    tip: f64,
    pressed: bool,
    speed: f64,
    centre: Pickup,
    bridge: Pickup,
    tip_force: f64,
    anvil_force: f64,
    work: f64,
    loss: f64,
}

const REST: f64 = -0.0007;
const TIP_STIFFNESS: f64 = 12_000.0;
const OVERTRAVEL: f64 = 0.0010;

impl StringVoice {
    /// Offset each pickup from the factory setup in mm. Positive opens the gap.
    pub fn set_pickup_distance_mm(&mut self, centre: f64, bridge: f64) {
        self.centre.set_distance_mm(centre, &self.current);
        self.bridge.set_distance_mm(bridge, &self.current);
    }
    pub fn new(midi: u8) -> Option<Self> {
        Self::at_resolution(midi, 1)
    }

    fn at_resolution(midi: u8, refinement: usize) -> Option<Self> {
        let k = key(midi)?;
        let rate = f64::from(OUTPUT_RATE) * SUBSTEPS as f64 * (refinement * refinement) as f64;
        let cells = k.cells * refinement;
        Some(Self::with_grid(k, rate, cells))
    }

    fn with_grid(k: Key, rate: f64, cells: usize) -> Self {
        let dx = k.length / cells as f64;
        let tail = (cells as f64 * (2.0_f64.powf(3.0 / 12.0) - 1.0)).round() as usize;
        let tangent = 1 + cells;
        let len = cells + tail + 3; // one ghost beyond each pinned endpoint
        let c = 2.0 * k.tension_pitch_hz * k.length / (1.0 + k.inharmonicity).sqrt();
        let kappa = c * k.length * k.inharmonicity.sqrt() / PI;
        let tension_step = (c / (rate * dx)).powi(2);
        // The fourth-order tension stencil shares the same fourth difference as stiffness.
        let bending_step = (kappa / (rate * dx * dx)).powi(2) + tension_step / 12.0;
        let viscous_step = 2.0 * 0.0015 * k.length.powi(2) / (rate * dx * dx);
        assert!(tension_step + 4.0 * bending_step + 2.0 * viscous_step < 1.0);
        let cell_mass = (k.core_mass_per_m + k.winding_mass_per_m) * dx;
        let drag: Vec<f64> = (0..len)
            .map(|i| {
                let span = (i.saturating_sub(tangent) as f64 / tail as f64).min(1.0);
                (k.loss + 1800.0 * (16.0 * span).min(1.0).powi(2)) / rate
            })
            .collect();
        let inv = drag.iter().map(|d| 1.0 / (1.0 + d)).collect();
        Self {
            geometry: Geometry {
                speaking_m: k.length,
                afterlength_m: tail as f64 * dx,
                centre_m: k.centre_m,
                bridge_m: k.bridge_m,
                anvil_drop_m: k.anvil_m,
            },
            current: vec![0.0; len],
            past: vec![0.0; len],
            next: vec![0.0; len],
            drag,
            inv,
            gel: Vec::new(),
            gel_tangent_inv: 1.,
            tangent,
            rate,
            tension_step,
            bending_step,
            viscous_step,
            cell_mass,
            anvil_m: k.anvil_m,
            steel_loss: k.loss / rate,
            tip_ratio: TIP_STIFFNESS / (cell_mass * rate * rate),
            tip: REST,
            pressed: false,
            speed: 0.0,
            // Positive motion is down towards the anvil and lower (centre) pickup.
            // Reversing a winding is electrical and is handled at the selector.
            centre: Pickup::new(k.centre_m, dx, 1.0),
            bridge: Pickup::new(k.bridge_m, dx, -1.0),
            tip_force: 0.0,
            anvil_force: 0.0,
            work: 0.0,
            loss: 0.0,
        }
    }

    /// Key travel changes without replacing either string segment.
    pub fn press(&mut self, velocity: u8) {
        if velocity == 0 {
            self.release();
            return;
        }
        self.speed = 1.0 + 3.0 * f64::from(velocity.min(127) - 1) / 126.0;
        self.pressed = true;
    }

    pub fn release(&mut self) {
        self.pressed = false;
    }

    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }

    /// Equidistant grid samples including the two pinned ends, excluding ghosts.
    pub fn displacements(&self) -> &[f64] {
        &self.current[1..self.current.len() - 1]
    }

    /// Advance one internal tick at 8 × 44.1 kHz for the default grid.
    pub fn step(&mut self) -> Pickups {
        self.advance::<true>()
    }

    #[inline]
    fn advance<const TRACE: bool>(&mut self) -> Pickups {
        self.tip = if self.pressed {
            (self.tip + self.speed / self.rate).min(self.anvil_m + OVERTRAVEL)
        } else {
            (self.tip - 0.6 / self.rate).max(REST)
        };
        self.solve::<TRACE>()
    }

    /// Advance the tangent and observe the same lever motion at a linked pad arm.
    pub fn step_linked_key(&mut self) -> (Pickups, f64) {
        let pickups = self.advance::<false>();
        (
            pickups,
            (self.tip - REST) / (self.anvil_m + OVERTRAVEL - REST),
        )
    }

    #[inline]
    fn solve<const TRACE: bool>(&mut self) -> Pickups {
        let end = self.current.len() - 2;
        // Odd extension imposes zero displacement and bending moment at each string end.
        self.current[0] = -self.current[2];
        self.current[end + 1] = -self.current[end - 1];
        let length = end + 2;
        let current = &self.current[..length];
        let past = &self.past[..length];
        let drag = &self.drag[..length];
        let inv = &self.inv[..length];
        let next = &mut self.next[..length];
        // Collect the five-point stiff-string and three-point viscous stencils.
        // Every interior point uses these same coefficients, including the afterlength.
        let adjacent = self.tension_step + 4.0 * self.bending_step + self.viscous_step;
        let middle =
            2.0 - 2.0 * self.tension_step - 6.0 * self.bending_step - 2.0 * self.viscous_step;
        let previous = 2.0 * self.viscous_step - 1.0;
        for i in 2..end {
            next[i] = (middle * current[i] + adjacent * (current[i - 1] + current[i + 1])
                - self.bending_step * (current[i - 2] + current[i + 2])
                + (previous + drag[i]) * past[i]
                - self.viscous_step * (past[i - 1] + past[i + 1]))
                * inv[i];
        }
        // A polymer strip loads only the afterlength cells underneath it.
        for cell in &self.gel {
            cell.force(&mut next[cell.node]);
        }
        // First solve the implicit one-sided rubber spring, then impose the rigid stop.
        let j = self.tangent;
        let free = self.next[j];
        let stiffness = self.tip_ratio * self.inv[j] * self.gel_tangent_inv;
        let contact = free + (self.tip - free).max(0.0) * stiffness / (1.0 + stiffness);
        self.next[j] = contact.min(self.anvil_m);
        for cell in &mut self.gel {
            let loss = cell.relax::<TRACE>(self.next[cell.node]);
            if TRACE {
                self.loss += loss;
            }
        }
        if TRACE {
            self.tip_force = TIP_STIFFNESS * (self.tip - self.next[j]).max(0.0);
            let net_force = (self.next[j] - free) * self.cell_mass * self.rate.powi(2)
                / (self.inv[j] * self.gel_tangent_inv);
            self.anvil_force = (self.tip_force - net_force).max(0.0);
            self.work += net_force * (self.next[j] - self.past[j]) * 0.5;
            // Centred velocity for local drag, backward velocity gradient for Kelvin-Voigt loss.
            let mut previous_delta = 0.0;
            for i in 2..=end {
                let delta = self.current[i] - self.past[i];
                let centred = 0.5 * (self.next[i] - self.past[i]);
                self.loss += self.cell_mass
                    * self.rate.powi(2)
                    * (2.0 * self.drag[i] * centred * centred
                        + self.viscous_step * (delta - previous_delta).powi(2));
                previous_delta = delta;
            }
        }
        std::mem::swap(&mut self.past, &mut self.current);
        std::mem::swap(&mut self.current, &mut self.next);
        Pickups {
            centre: self.centre.sense::<TRACE>(&self.current, self.rate),
            bridge: self.bridge.sense::<TRACE>(&self.current, self.rate),
        }
    }

    pub fn trace(&self) -> Trace {
        let mut energy = 0.0;
        let end = self.current.len() - 2;
        for i in 2..=end {
            let delta = self.current[i] - self.past[i];
            let slope = self.current[i] - self.current[i - 1];
            let curvature = if i == end {
                0.0
            } else {
                self.current[i - 1] - 2.0 * self.current[i] + self.current[i + 1]
            };
            energy += delta * delta
                + self.tension_step * slope * slope
                + self.bending_step * curvature * curvature;
        }
        Trace {
            tip_position_m: self.tip,
            tangent_m: self.current[self.tangent],
            centre_m: self.centre.displacement,
            bridge_m: self.bridge.displacement,
            tip_n: self.tip_force,
            anvil_n: self.anvil_force,
            contact_work_j: self.work,
            viscous_loss_j: self.loss,
            stored_j: 0.5 * self.cell_mass * self.rate.powi(2) * energy
                + self
                    .gel
                    .iter()
                    .map(|cell| cell.stored(self.current[cell.node]))
                    .sum::<f64>(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contacts_open_without_discarding_string_motion() {
        for midi in KEYS {
            let mut v = StringVoice::new(midi).unwrap();
            v.press(80);
            for _ in 0..(0.05 * v.rate) as usize {
                v.step();
                assert!(v.current[v.tangent] <= v.anvil_m);
            }
            let held = v.trace();
            assert!(held.tip_n > 0.0 && held.anvil_n > 0.0);
            assert_eq!(held.tangent_m, v.anvil_m);
            let saved = (v.current.clone(), v.past.clone());
            v.release();
            assert_eq!((&v.current, &v.past), (&saved.0, &saved.1));
            for _ in 0..(0.2 * v.rate) as usize {
                v.step();
            }
            let free = v.trace();
            assert_eq!((free.tip_n, free.anvil_n), (0.0, 0.0));
            assert!(free.stored_j > 0.0 && free.stored_j < 0.03 * held.stored_j);
            assert!(free.viscous_loss_j > held.viscous_loss_j);
            let balance = free.contact_work_j - free.viscous_loss_j - free.stored_j;
            assert!(
                balance.abs() < 0.01 * free.contact_work_j,
                "MIDI {midi}, energy balance {balance:e} J"
            );
            let before = v.current.clone();
            v.press(100);
            assert_eq!(v.current, before);
            for _ in 0..(0.01 * v.rate) as usize {
                v.step();
            }
            assert!(v.trace().anvil_n > 0.0);
        }
    }

    #[test]
    fn hard_strikes_deliver_more_attack_energy_than_soft_strikes() {
        for midi in KEYS {
            let attack_energy = |velocity| {
                let mut v = StringVoice::new(midi).unwrap();
                v.press(velocity);
                (0..(0.025 * v.rate) as usize)
                    .map(|_| {
                        let p = v.step();
                        p.centre.powi(2) + p.bridge.powi(2)
                    })
                    .sum::<f64>()
            };
            assert!(attack_energy(112) > attack_energy(32), "MIDI {midi}");
        }
    }

    #[test]
    fn both_wirings_preserve_solos_and_cancel_identical_stems_in_opposition() {
        let p = Pickups {
            centre: 0.7,
            bridge: -0.2,
        };
        for w in [Wiring::DaAdds, Wiring::DaSubtracts] {
            assert!((Switch::CA.output(p, w) - 0.7 * 10.0 / 11.0).abs() < 1e-12);
            assert!((Switch::CB.output(p, w) + 0.2 * 10.0 / 11.0).abs() < 1e-12);
        }
        let same = Pickups {
            centre: 0.8,
            bridge: 0.8,
        };
        assert_eq!(Switch::DB.output(same, Wiring::DaAdds), 0.0);
        assert_eq!(Switch::DA.output(same, Wiring::DaSubtracts), 0.0);
        assert_eq!(
            Switch::DA.output(p, Wiring::DaAdds),
            Switch::DB.output(p, Wiring::DaSubtracts)
        );
        assert_eq!(
            Switch::DB.output(p, Wiring::DaAdds),
            Switch::DA.output(p, Wiring::DaSubtracts)
        );
    }

    #[test]
    fn local_grid_refinement_preserves_low_partial_levels_at_both_pickups() {
        for midi in KEYS {
            let coarse = projections(midi, 1);
            let fine = projections(midi, 2);
            for (i, (a, b)) in coarse.into_iter().zip(fine).enumerate() {
                let db = 20.0 * (a / b).log10();
                assert!(
                    db.abs() < 1.5,
                    "MIDI {midi}, projection {i}, grid difference {db:.2} dB"
                );
            }
        }
    }

    #[test]
    fn grid_refinement_preserves_contact_timing_and_attack_and_release_energy() {
        for midi in KEYS {
            let observe = |refinement| {
                let mut v = StringVoice::at_resolution(midi, refinement).unwrap();
                let release_tick = (v.rate * 0.050).round() as usize;
                let mut clamp = None;
                let mut energies = [0.0; 4];
                v.press(80);
                for i in 0..(v.rate * 0.110) as usize {
                    if i == release_tick {
                        v.release();
                    }
                    let p = v.step();
                    let t = i as f64 / v.rate;
                    if clamp.is_none() && v.current[v.tangent] == v.anvil_m {
                        clamp = Some(t);
                    }
                    if t < 0.025 || t >= 0.060 {
                        let offset = if t < 0.025 { 0 } else { 2 };
                        energies[offset] += p.centre.powi(2) / v.rate;
                        energies[offset + 1] += p.bridge.powi(2) / v.rate;
                    }
                }
                (clamp.unwrap(), energies)
            };
            let (coarse_clamp, coarse) = observe(1);
            let (fine_clamp, fine) = observe(2);
            assert!(
                (coarse_clamp - fine_clamp).abs() < 0.00005,
                "MIDI {midi}, clamp timing"
            );
            for (i, (a, b)) in coarse.into_iter().zip(fine).enumerate() {
                let db = 10.0 * (a / b).log10();
                assert!(
                    db.abs() < 1.5,
                    "MIDI {midi}, energy band {i}, grid difference {db:.2} dB"
                );
            }
        }
    }

    fn projections(midi: u8, refinement: usize) -> [f64; 4] {
        let mut v = StringVoice::at_resolution(midi, refinement).unwrap();
        let f = 440.0 * 2.0_f64.powf((midi as f64 - 69.0) / 12.0);
        let mut sums = [(0.0, 0.0); 4];
        v.press(80);
        for i in 0..(v.rate * 0.08) as usize {
            let p = v.step();
            let t = (i + 1) as f64 / v.rate;
            if t < 0.02 {
                continue;
            }
            let window = 0.5 - 0.5 * (2.0 * PI * (t - 0.02) / 0.06).cos();
            for (j, value) in [p.centre, p.centre, p.bridge, p.bridge]
                .into_iter()
                .enumerate()
            {
                let phase = 2.0 * PI * f * (1 + j % 2) as f64 * t;
                let (sin, cos) = phase.sin_cos();
                sums[j].0 += value * window * cos / v.rate;
                sums[j].1 += value * window * sin / v.rate;
            }
        }
        sums.map(|(a, b)| a.hypot(b))
    }
}
