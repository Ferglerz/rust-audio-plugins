use crate::{
    calibration::{CONTACT_FRACTION, KEYS, UPPER_IMPULSE},
    era::Era,
    field::Field,
};
use pleasant_dsp::instrument::rotate;
use std::f64::consts::TAU;

#[derive(Clone, Copy, Default)]
struct Mode {
    x: f64,
    quadrature: f64,
    sin: f64,
    cos: f64,
    stock_rotation: (f64, f64),
    mark_ii_rotation: (f64, f64),
    omega: f64,
    impulse_gain: f64,
}

impl Mode {
    fn new(hz: f64, decay: f64, mark_ii_decay_ratio: f64, rate: f64) -> Self {
        let omega = TAU * hz;
        let (s, c) = (omega / rate).sin_cos();
        let loss = (-decay / rate).exp();
        let mark_ii_loss = (-decay * mark_ii_decay_ratio / rate).exp();
        Self {
            omega,
            sin: s * loss,
            cos: c * loss,
            stock_rotation: (s * loss, c * loss),
            mark_ii_rotation: (s * mark_ii_loss, c * mark_ii_loss),
            ..Self::default()
        }
    }
    #[inline]
    fn step(&mut self, impulse: f64, damper: f64) {
        self.quadrature += self.impulse_gain * impulse;
        (self.x, self.quadrature) = rotate(
            self.x * damper,
            self.quadrature * damper,
            self.sin,
            self.cos,
        );
    }
}

/// Short, unilateral elastic impact in relative coordinates. With unit incoming
/// speed, x'' = -K x^(3/2) returns to zero compression at time 1. Integrating
/// the force transfers two units of momentum over the compression and rebound.
#[derive(Default)]
struct Contact {
    compression: f64,
    speed: f64,
    step: f64,
    subdivisions: u8,
}

impl Contact {
    fn start(&mut self, ticks: f64) {
        let ticks = ticks.max(1.0);
        self.subdivisions = (12.0 / ticks).ceil() as u8;
        self.step = 1.0 / (ticks * f64::from(self.subdivisions));
        self.compression = self.step * 0.5;
        self.speed = 1.0;
    }

    #[inline]
    fn impulse(&mut self) -> f64 {
        if self.compression <= 0.0 {
            return 0.0;
        }
        let mut total = 0.0;
        for _ in 0..self.subdivisions {
            let x = self.compression.max(0.0);
            let impulse = 18.57749636 * x * x.sqrt() * self.step;
            self.speed -= impulse;
            self.compression += self.speed * self.step;
            total += impulse;
        }
        0.5 * total
    }
}

pub(super) struct Tine {
    modes: [Mode; 6],
    hammer: Contact,
    count: usize,
    pub held: bool,
    pub active: bool,
    pub velocity: f64,
    choke_gain: f64,
    choke_step: f64,
    damper_loss: f64,
    cube_contact_ratio: f64,
    damper_gain: f64,
    damper_step: f64,
    damper_start: f64,
    damper_accel: f64,
    damper_floor: f64,
    damping: bool,
    ticks: u32,
    height: f64,
    gap: f64,
    key_index: usize,
    contact: f64,
    weight: f64,
}

impl Tine {
    pub fn new(note: u8, rate: f64) -> Self {
        use crate::engine::{HIGH, LOW};
        let edge = note.clamp(LOW, HIGH);
        let index = usize::from(edge - LOW);
        let [length, decay, _, _, _, _, _, _, contact_per_mm] = KEYS[index];
        let (height, gap) = Era::LateI.geometry(index);
        let hz = 440.0 * 2.0f64.powf((note as f64 - 69.0) / 12.0);
        // Normal modes of the coupled tine/tonebar fundamental, secondary
        // polarization, and the first three measured inharmonic tine modes.
        let mut modes = [Mode::default(); 6];
        let mut count = 0;
        // Kahn's E3 and E6 direct-harp recordings anchor the fundamental
        // loss on this representative Mark II. Clamp outside the fit range.
        let register = ((index as f64 + 28. - 52.) / 36.).clamp(0., 1.);
        let mark_ii_fundamental = 1.05 * (0.29_f64 / 1.05).powf(register);
        for (frequency, loss) in [
            (hz, decay),
            (hz + 0.42, decay * 0.72),
            (hz * 1.0009, decay * 1.4),
            (hz * 7.239, decay * 14.03),
            (hz * 19.662, decay * 13.17),
            (hz * 37.75, decay * 42.64),
        ] {
            if frequency < 16_000.0 && frequency < 0.42 * rate {
                let ratio = if count < 3 { mark_ii_fundamental } else { 1. };
                modes[count] = Mode::new(frequency, loss, ratio, rate);
                count += 1;
            }
        }
        Self {
            modes,
            hammer: Contact::default(),
            count,
            held: false,
            active: false,
            velocity: 0.,
            damper_loss: (38.0 + 0.7 * index as f64) / rate,
            cube_contact_ratio: (0.375_f64
                / match index + 8 {
                    8..=30 => 0.25,
                    31..=40 => 0.3125,
                    41..=50 => 0.375,
                    _ => 0.4375,
                })
            .sqrt(),
            damper_gain: 1.0,
            damper_step: 1.0,
            damper_start: 1.0,
            damper_accel: 1.0,
            damper_floor: 1.0,
            choke_gain: 1.0,
            choke_step: 1.0 / (0.005 * rate),
            damping: false,
            ticks: 0,
            height,
            gap,
            key_index: index,
            contact: length
                * contact_per_mm
                * 0.001
                * 2.0_f64.powf((f64::from(edge) - f64::from(note)) / 12.),
            weight: if index < 4 { 0.25 } else { 1.0 / 3.0 },
        }
    }
    pub fn strike(&mut self, v: f64, rate: f64, era: Era) {
        (self.height, self.gap) = era.geometry(self.key_index);
        let swing = era.swing(self.key_index, v);
        // Uncompressed felt broadens contact at low velocities. Its
        // compression at harder strikes brings back the upper partials.
        let contact_ratio = match era {
            Era::Sparkletop => 1. + 0.6 * (1. - v),
            Era::EarlyI | Era::MidI => self.cube_contact_ratio,
            _ => 1.,
        };
        let stock_time = self.contact * v.max(0.01).powf(-0.3011);
        let time = stock_time * contact_ratio;
        // A stiffer damper on the Mark V returns more promptly. The size
        // of this release correction is a model estimate (research/eras.md).
        let loss = self.damper_loss
            * match era {
                Era::Sparkletop => 1.0 / 2.0,
                Era::MarkV => 1.10,
                _ => 1.0,
            };
        // Felt loads progressively. At the former mechanical -20 dB point the
        // instantaneous loss reaches twice the average; thereafter it is fixed.
        self.damper_step = (-2.0 * loss * loss / std::f64::consts::LN_10).exp();
        self.damper_floor = (-2.0 * loss).exp();
        self.damper_accel = 1.;
        if era == Era::Sparkletop {
            // The exposed harp releases develop more loss late in contact.
            // A quadratic loading ramp gives a cubic log-amplitude envelope.
            self.damper_step = 1.;
            self.damper_accel = (-6. * loss.powi(3) / std::f64::consts::LN_10.powi(2)).exp();
            self.damper_floor = (-3. * loss).exp();
        }
        self.damper_start = self.damper_step;
        self.damper_gain = 1.0;
        // The reduced impact uses the closing speed at contact. For the
        // effective 1:4 hammer/tine mass ratio, a stationary tine takes 2/5
        // of the incoming speed. Reversing tine motion changes the impulse.
        let omega = self.modes[0].omega;
        let tine_speed: f64 = self.modes[..self.count]
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 2)
            .map(|(_, m)| m.quadrature * m.omega)
            .sum();
        let impact = (swing - 0.4 * self.choke_gain * tine_speed / omega).max(0.0);
        self.hammer = Contact::default();
        if impact > 0.0 {
            // Hertz contact time scales as closing speed to the power -1/5.
            self.hammer
                .start(rate * time * CONTACT_FRACTION * (swing / impact).powf(0.2));
        }
        let [bell, third, fourth] = UPPER_IMPULSE;
        let excitation = [0.97, 0.03, 0.045, bell, third, fourth];
        for (i, m) in self.modes[..self.count].iter_mut().enumerate() {
            (m.sin, m.cos) = if era == Era::MarkII {
                m.mark_ii_rotation
            } else {
                m.stock_rotation
            };
            m.impulse_gain = impact * excitation[i];
        }
        if self.choke_gain < 1.0 {
            for m in &mut self.modes {
                m.x *= self.choke_gain;
                m.quadrature *= self.choke_gain;
            }
        }
        self.choke_gain = 1.0;
        self.active = true;
        self.damping = false;
        self.ticks = 0;
    }
    pub fn undamped(&self) -> bool {
        !self.damping && self.choke_gain == 1.0
    }
    pub fn damp(&mut self, on: bool) {
        self.damping = on;
        if !on {
            self.damper_step = self.damper_start;
            self.damper_gain = 1.0;
        }
    }
    pub fn choke(&mut self) {
        self.held = false;
        self.choke_gain = self.choke_gain.min(1.0 - self.choke_step);
    }
    pub fn silence(&mut self) {
        self.active = false;
        self.velocity = 0.;
        self.hammer = Contact::default();
        self.held = false;
        for m in &mut self.modes {
            m.x = 0.0;
            m.quadrature = 0.0;
        }
    }
    #[inline]
    pub fn tick(&mut self, field: &Field, height_offset: f64, gap_offset: f64) -> f64 {
        if self.choke_gain < 1.0 {
            self.choke_gain = (self.choke_gain - self.choke_step).max(0.0);
            if self.choke_gain == 0.0 {
                self.silence();
                return 0.0;
            }
        }
        if self.damping && self.damper_gain > self.damper_floor {
            self.damper_step *= self.damper_accel;
            self.damper_gain = (self.damper_gain * self.damper_step).max(self.damper_floor);
        }
        let (mut x, mut speed) = (0.0, 0.0);
        let impulse = self.hammer.impulse();
        for (i, m) in self.modes[..self.count].iter_mut().enumerate() {
            m.step(impulse, self.damper_gain);
            if i != 2 {
                x += m.x;
                speed += m.quadrature * m.omega;
            }
        }
        self.ticks = self.ticks.wrapping_add(1);
        if self.ticks.is_multiple_of(1024)
            && self.hammer.compression <= 0.0
            && self.modes[..self.count]
                .iter()
                .all(|m| (m.x.abs() + m.quadrature.abs()) * m.omega < 1e-6)
        {
            self.silence();
        }
        self.velocity = self.choke_gain * speed;
        let lat = &self.modes[2];
        self.choke_gain
            * self.weight
            * field.emf(
                self.height + height_offset + x,
                lat.x,
                speed,
                lat.omega * lat.quadrature,
                self.gap + gap_offset,
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elastic_impact_transfers_one_impulse_then_separates() {
        for ticks in [1.6_f64, 12.0, 40.6, 380.0] {
            let mut contact = Contact::default();
            contact.start(ticks);
            let total: f64 = (0..ticks.ceil() as usize + 2)
                .map(|_| contact.impulse())
                .sum();
            assert!((total - 1.0).abs() < 0.005, "{ticks} ticks: {total}");
            assert_eq!(contact.impulse(), 0.0);
        }
    }

    #[test]
    fn a_restrike_transfers_more_momentum_to_an_approaching_tine() {
        let mut tine = Tine::new(55, 192000.0);
        let mut impulse = |motion| {
            tine.modes[0].quadrature = motion;
            tine.strike(0.69, 192000.0, Era::LateI);
            tine.modes[0].impulse_gain
        };
        let approaching = impulse(-0.4);
        let still = impulse(0.0);
        let receding = impulse(0.4);
        assert!(approaching > still && still > receding && receding > 0.0);
    }
}
