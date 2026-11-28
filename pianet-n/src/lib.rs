//! Plucked reed keyboards. SI units internally; circuit data are variant-specific.
mod beam;
pub mod cembalet;
pub mod circuit;
pub mod claviset;
pub mod early;
mod contact_reed;
pub mod elepian;
pub mod valente;
pub mod vivi;
mod engine;
mod keyboard;
mod scale;
pub use engine::{Engine, LATENCY};
pub use scale::Model;

use beam::Mode;

pub const KEYS: [u8; 3] = [45, 64, 81];
pub const TRIAD: [u8; 3] = [60, 64, 67];
pub const RATE: u32 = 48_000;
pub const SUBSTEPS: usize = 4;

#[derive(Clone, Copy)]
pub struct Observation {
    pub pad_height: f64,
    pub reed_at_pad: f64,
    pub reed_tip: f64,
    pub pad_force: f64,
    pub bonded: bool,
    pub seating: f64,
    pub capacitance: f64,
    pub flux_wb_turns: f64,
    pub suction_pa: f64,
    pub damper_force: f64,
    pub pick_engaged: bool,
    pub pick_armed: bool,
    pub key_fraction: f64,
}

#[derive(Clone)]
pub struct Reed {
    modes: Vec<Mode>,
    step: f64,
    key_down: bool,
    catch: PadCatch,
    pad_height: f64,
    pad_rest: f64,
    lift_speed: f64,
    pad_speed: f64,
    stiffness: f64,
    dashpot: f64,
    bond_force: f64,
    seated: f64,
    seat_step: f64,
    bonded: bool,
    force: f64,
    force_compliance: f64,
    velocity_compliance: f64,
    gap: f64,
    area: f64,
    model: Model,
    air_height: f64,
    cup_depth: f64,
    peel_height: f64,
    suction_area: f64,
    leak_step: f64,
    field_slope: f64,
    native_gap: f64,
    pickup_gain: f64,
    tip: f64,
    previous_tip: f64,
    rest_tip: f64,
    release_mix: f64,
    contact_ticks: usize,
    contact_length: usize,
    contact_slow: f64,
    contact_lag: f64,
    contact_step: f64,
    contact_fast: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PadCatch {
    Free,
    Held,
    Rearming,
}

impl Reed {
    pub fn new(midi: u8) -> Option<Self> {
        Self::at_rate(Model::N, midi, f64::from(RATE) * SUBSTEPS as f64)
    }

    pub fn at_rate(model: Model, midi: u8, solver_rate: f64) -> Option<Self> {
        if !model.key_range().contains(&midi) {
            return None;
        }
        Self::at_rate_extended(model, midi, solver_rate)
    }

    pub fn at_rate_extended(model: Model, midi: u8, solver_rate: f64) -> Option<Self> {
        if !solver_rate.is_finite() || solver_rate < 16000.0 {
            return None;
        }
        if model.is_cembalet() || midi > 127 {
            return None;
        }
        let range = model.key_range();
        let anchor = midi.clamp(*range.start(), *range.end());
        let section = scale::section(model, anchor)?;
        let scale::Section {
            width,
            gap,
            pluck,
            field_slope,
            ..
        } = section;
        let hz = section.hz * 2f64.powf((f64::from(midi) - f64::from(anchor)) / 12.);
        let step = 1.0 / solver_rate;
        let (thickness, mass, modes) = beam::modes_at_pitch(
            &section,
            hz,
            solver_rate,
            if model != Model::T { 0.83 } else { 0.81 },
            |frequency| {
                if model != Model::T {
                    0.95 + 0.00125 * frequency + 0.00000009 * frequency.powi(2)
                } else {
                    section.loss * (frequency / hz).powf(1.35)
                }
            },
        );
        let static_compliance: f64 = modes.iter().map(|m| m.pad * m.compliance).sum();
        let stiffness = 2.4 / static_compliance;
        let dashpot = 1.7 * (stiffness * mass).sqrt();
        let pad_rest = -pluck * 0.35;
        let force = stiffness * pad_rest / (1.0 + stiffness * static_compliance);
        let force_compliance = modes.iter().map(|m| m.pad * m.dx_per_force).sum();
        let velocity_compliance = modes
            .iter()
            .map(|m| m.pad * (m.omega * m.dq_per_force - m.loss * m.dx_per_force))
            .sum();
        let bond_force = pluck / modes.iter().map(|m| m.compliance).sum::<f64>();
        let cup_depth = pluck * 0.22;
        let peel_height = pluck * 0.30;
        let mut reed = Self {
            modes,
            step,
            key_down: false,
            catch: PadCatch::Free,
            pad_height: pad_rest,
            pad_rest,
            lift_speed: 0.0,
            pad_speed: 0.0,
            stiffness,
            dashpot,
            bond_force,
            seated: 1.0,
            seat_step: -(-step / if model != Model::T { 0.025 } else { 0.012 }).exp_m1(),
            bonded: true,
            force,
            force_compliance,
            velocity_compliance,
            gap,
            area: width * thickness,
            model,
            cup_depth,
            peel_height,
            suction_area: bond_force * (1.0 + cup_depth / peel_height) / 101325.0,
            air_height: cup_depth,
            leak_step: -(-step / 0.035).exp_m1(),
            field_slope,
            native_gap: gap,
            pickup_gain: 1.,
            tip: 0.,
            previous_tip: 0.,
            rest_tip: 0.,
            release_mix: 1.,
            contact_ticks: 0,
            contact_length: (0.018 * solver_rate).ceil() as usize,
            contact_slow: 0.,
            contact_lag: 0.,
            contact_step: -(-step * 2. * std::f64::consts::PI * 520.).exp_m1(),
            contact_fast: 0.,
        };
        // Start in preloaded mechanical equilibrium, including the higher modes.
        for m in &mut reed.modes {
            m.position = m.compliance * force;
            m.quadrature = m.loss * m.position / m.omega;
            reed.rest_tip += m.position;
        }
        reed.tip = reed.rest_tip;
        reed.previous_tip = reed.rest_tip;
        Some(reed)
    }

    pub fn reset(&mut self) {
        self.tip = self.rest_tip;
        self.previous_tip = self.rest_tip;
        self.contact_ticks = 0;
        self.contact_fast = 0.;
        self.key_down = false;
        self.catch = PadCatch::Free;
        self.pad_height = self.pad_rest;
        self.pad_speed = 0.0;
        self.lift_speed = 0.0;
        self.bonded = true;
        self.seated = 1.0;
        self.air_height = self.cup_depth;
        let compliance: f64 = self.modes.iter().map(|m| m.compliance * m.pad).sum();
        self.force = self.stiffness * self.pad_rest / (1.0 + self.stiffness * compliance);
        for m in &mut self.modes {
            m.position = m.compliance * self.force;
            m.quadrature = m.loss * m.position / m.omega;
        }
    }

    /// Signed millimetres added to this key's original sensor clearance.
    pub fn set_pickup_distance_mm(&mut self, offset: f32) {
        self.gap = self.native_gap + f64::from(offset.clamp(-0.15, 1.)) * 0.001;
        self.pickup_gain = (self.native_gap / self.gap).powi(2);
        self.contact_fast = 0.;
        self.contact_slow = self.sensor_at(self.tip);
        self.contact_lag = 0.;
    }
    /// Fraction of the pad's original recontact transient, from zero to one.
    pub fn set_pad_release(&mut self, amount: f32) {
        self.release_mix = f64::from(amount.clamp(0., 1.));
    }
    fn capacitance_at(&self, tip: f64) -> f64 {
        if self.model == Model::T {
            return 0.;
        }
        let overlap = 0.5 + ((tip + 0.5 * self.gap) / self.gap).atan() / std::f64::consts::PI;
        8.8541878e-12 * self.area / self.gap * overlap
    }
    fn sensor_at(&self, tip: f64) -> f64 {
        if self.model == Model::T {
            let length = self.gap / self.field_slope;
            self.pickup_gain * 0.015 * length * (tip / length).exp_m1()
        } else {
            self.capacitance_at(tip)
        }
    }
    fn current_sensor(&self) -> f64 {
        self.sensor_at(self.tip) - (1. - self.release_mix) * self.contact_fast
    }
    pub fn current_capacitance(&self) -> f64 {
        if self.model == Model::T {
            0.
        } else {
            self.current_sensor()
        }
    }
    pub fn current_flux(&self) -> f64 {
        if self.model != Model::T {
            0.
        } else {
            self.current_sensor()
        }
    }

    pub fn press(&mut self, velocity: u8) {
        if velocity == 0 {
            self.release();
            return;
        }
        self.rearm_sustained();
        self.key_down = true;
        self.lift_speed = 0.10 + 0.28 * f64::from(velocity.min(127)) / 127.0;
    }

    pub fn release(&mut self) {
        self.key_down = false;
        self.catch = PadCatch::Free;
    }

    /// On key-up the optional pedal catches a lifted pad. A pad that is
    /// still bonded returns normally, so the pedal cannot pluck an idle key.
    pub fn hold_for_sustain(&mut self) -> bool {
        self.key_down = false;
        if self.bonded {
            self.catch = PadCatch::Free;
            false
        } else {
            self.catch = PadCatch::Held;
            true
        }
    }

    /// Drop the catch at pedal-up. A pressed key already rearming its pad
    /// completes its stroke independently of the departing pedal.
    pub fn release_sustain(&mut self) -> bool {
        if self.catch != PadCatch::Held {
            return false;
        }
        self.catch = PadCatch::Free;
        true
    }

    /// A new stroke on a sustained reed must seat its pad before it can pluck.
    /// The Duo calls this on key-down, then continues following its shared lever.
    pub fn rearm_sustained(&mut self) {
        if self.catch == PadCatch::Held {
            self.catch = PadCatch::Rearming;
        }
    }

    /// One tick at four times the WAV rate. Returning before contact retains ringing
    /// and the incomplete area of an adhesive bond limits a partially seated repeat.
    pub fn step(&mut self) -> Observation {
        // A finite acceleration avoids an instantaneous velocity impulse in the pad.
        let (target, speed) = if self.key_down {
            (0.004, self.lift_speed)
        } else {
            (self.pad_rest, 0.55)
        };
        self.move_pad(target, speed)
    }

    /// Follow the moving pad arm of the Duo's shared key lever. The pad retains
    /// its own inertia and seal: a fast key cannot move its face instantaneously.
    pub fn follow_key(&mut self, fraction: f64, down: bool) -> Observation {
        self.key_down = down;
        let target = self.pad_rest + fraction.clamp(0., 1.) * (0.004 - self.pad_rest);
        // Fixed nominal pad slew models the weak touch dependence of the Duo reed.
        let speed = if target >= self.pad_height {
            0.28
        } else {
            0.55
        };
        self.move_pad(target, speed)
    }

    fn move_pad(&mut self, target: f64, speed: f64) -> Observation {
        // The virtual catch decouples only the pad arm from the key. On a
        // repeat, wait until contact has recovered half its area before pulling
        // again. Adhesive-film and suction-seal recovery times remain distinct.
        if self.catch == PadCatch::Rearming && self.bonded && self.seated >= 0.5 {
            self.catch = PadCatch::Free;
        }
        let (target, speed, can_seat) = match self.catch {
            PadCatch::Held => (0.004, self.lift_speed.max(0.28), false),
            PadCatch::Rearming => (self.pad_rest, 0.55, true),
            PadCatch::Free => (target, speed, !self.key_down),
        };
        let remaining = target - self.pad_height;
        let acceleration = 420.0;
        let desired = remaining.signum() * speed.min((2.0 * acceleration * remaining.abs()).sqrt());
        let max_change = acceleration * self.step;
        let old_speed = self.pad_speed;
        self.pad_speed += (desired - old_speed).clamp(-max_change, max_change);
        let travel = (old_speed + self.pad_speed) * 0.5 * self.step;
        if travel.signum() == remaining.signum() && travel.abs() >= remaining.abs() {
            self.pad_height = target;
            self.pad_speed = 0.0;
        } else {
            self.pad_height += travel;
        }
        let mut free_x = 0.0;
        let mut free_v = 0.0;
        for m in &mut self.modes {
            m.free();
            free_x += m.pad * m.position;
            free_v += m.pad * m.velocity();
        }
        // Implicit Kelvin–Voigt force at the reed contact point. A separated pad can
        // compress the reed again only when their surfaces meet. The compressed
        // layers dissipate more energy than the thin adhesive film in tension.
        let damping = if self.pad_height > free_x {
            self.dashpot * 0.12
        } else {
            self.dashpot
        };
        let contact = (self.stiffness * (self.pad_height - free_x)
            + damping * (self.pad_speed - free_v))
            / (1.0 + self.stiffness * self.force_compliance + damping * self.velocity_compliance);
        if !self.bonded && can_seat && self.pad_height <= free_x && contact < 0.0 {
            self.bonded = true;
            // A suction lip can vent and reseal during the same impact.
            if self.contact_ticks == 0 {
                self.contact_ticks = self.contact_length;
                // Seed both displacement and incident velocity so suppressing the
                // impact does not create a velocity step at the recontact sample.
                let prior = self.sensor_at(self.tip);
                let delta = prior - self.sensor_at(self.previous_tip);
                self.contact_lag = delta * (1. - self.contact_step) / self.contact_step;
                self.contact_slow = prior - self.contact_lag;
            }
        }
        let mut suction_pa = 0.0;
        match self.model {
            Model::N | Model::C | Model::L => {
                let pull_rate = (self.pad_speed / 0.38).clamp(0.0, 1.0);
                if self.bonded && contact > self.bond_force * self.seated * (1.0 + 0.5 * pull_rate)
                {
                    self.bonded = false;
                    self.seated = 0.0;
                }
                self.force = if self.bonded {
                    contact
                } else if self.pad_height < free_x {
                    contact.min(0.0)
                } else {
                    0.0
                };
            }
            Model::Cembalet | Model::CembaletN => unreachable!("pad models only"),
            Model::T => {
                let gap = self.pad_height - free_x;
                self.force = if gap < 0.0 { contact.min(0.0) } else { 0. };
                if self.bonded && gap > 0.0 {
                    // Integrate Boyle's law while the lip seals. H = cavity height before
                    // applying force, c = point compliance. The volume at pressure p is
                    // proportional to H-cF. Use the smaller root of the force quadratic.
                    let h = self.cup_depth + gap;
                    let load = 101325.0 * self.suction_area * self.seated;
                    let c = self.force_compliance;
                    let root = ((h - c * load).powi(2) + 4.0 * c * load * self.air_height).sqrt();
                    let force =
                        (2.0 * load * (h - self.air_height) / (h + c * load + root)).max(0.0);
                    let stretch = gap - c * force;
                    let height = self.cup_depth + stretch;
                    if stretch > self.peel_height * self.seated {
                        self.bonded = false;
                        self.seated = 0.0;
                        self.air_height = self.cup_depth;
                    } else {
                        self.force = force;
                        suction_pa = 101325.0 * (1.0 - self.air_height / height);
                        self.air_height += (height - self.air_height) * self.leak_step;
                    }
                } else {
                    self.air_height = self.cup_depth;
                }
            }
        }
        if self.bonded && can_seat && self.force <= 0.0 {
            self.seated += (1.0 - self.seated) * self.seat_step;
        }
        let mut tip = 0.0;
        for m in &mut self.modes {
            m.position += m.dx_per_force * self.force;
            m.quadrature += m.dq_per_force * self.force;
            tip += m.position;
        }
        self.previous_tip = self.tip;
        self.tip = tip;
        self.contact_fast = 0.;
        if self.contact_ticks > 0 {
            // At the actual pad-to-reed collision, decompose the native pickup
            // into a slow component and its high-frequency complement. Blending
            // those components never feeds back into the seal, the contact force
            // or the already-moving reed. Outside the seating window the entire
            // pickup passes through. Fade the complement out before ending it.
            let raw = self.sensor_at(tip);
            self.contact_slow += self.contact_step * (raw - self.contact_slow);
            let taper = (self.contact_ticks as f64 / (0.20 * self.contact_length as f64)).min(1.);
            let taper = taper * taper * (3. - 2. * taper);
            self.contact_fast = taper * (raw - self.contact_slow - self.contact_lag);
            self.contact_ticks -= 1;
        }
        Observation {
            pad_height: self.pad_height,
            reed_at_pad: free_x + self.force_compliance * self.force,
            reed_tip: tip,
            pad_force: self.force,
            bonded: self.bonded,
            seating: self.seated,
            capacitance: self.current_capacitance(),
            // Across the small blade excursion use the local logarithmic gradient of
            // flux sensitivity at the pole shoe. Integrate before deriving voltage;
            // a nearly uniform gradient in the treble yields almost linear flux.
            flux_wb_turns: self.current_flux(),
            suction_pa,
            damper_force: 0.,
            pick_engaged: false,
            pick_armed: false,
            key_fraction: (self.pad_height - self.pad_rest) / (0.004 - self.pad_rest),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advance(reed: &mut Reed, ms: usize) -> Observation {
        let mut observation = reed.step();
        for _ in 1..ms * RATE as usize / 1000 * SUBSTEPS {
            observation = reed.step();
        }
        observation
    }

    #[test]
    fn attached_pad_pulls_off_after_lifting_and_damps_on_return() {
        for midi in KEYS {
            let mut reed = Reed::new(midi).unwrap();
            let rest = advance(&mut reed, 20);
            assert!(rest.bonded && rest.pad_force < 0.0 && rest.reed_tip < 0.0);
            reed.press(80);
            let mut o = reed.step();
            for _ in 0..(RATE as usize * SUBSTEPS / 50) {
                if !o.bonded {
                    break;
                }
                o = reed.step();
            }
            assert!(
                !o.bonded && o.pad_force == 0.0 && o.reed_tip > 0.0001,
                "MIDI {midi}"
            );
            // A three-ms return from the top of the stroke cannot reach the reed.
            advance(&mut reed, 55);
            reed.release();
            let partial = advance(&mut reed, 3);
            assert!(!partial.bonded && partial.seating == 0.0);
            reed.press(80);
            assert_eq!(advance(&mut reed, 30).pad_force, 0.0);
            reed.release();
            let short = advance(&mut reed, 22);
            assert!(
                short.bonded && (0.01..0.8).contains(&short.seating),
                "MIDI {midi}"
            );
            let settled = advance(&mut reed, 400);
            assert!(settled.seating > 0.999 && (settled.reed_tip - rest.reed_tip).abs() < 1e-7);
        }
    }

    #[test]
    fn partial_reseat_reduces_pull_off_deflection() {
        for midi in KEYS {
            let release = |gap| {
                let mut reed = Reed::new(midi).unwrap();
                reed.press(80);
                advance(&mut reed, 100);
                reed.release();
                advance(&mut reed, gap);
                reed.press(80);
                for _ in 0..(RATE as usize * SUBSTEPS / 50) {
                    let o = reed.step();
                    if !o.bonded {
                        return o.reed_tip.abs();
                    }
                }
                panic!("MIDI {midi} remained bonded");
            };
            assert!(release(22) < 0.8 * release(350), "MIDI {midi}");
        }
    }
}
