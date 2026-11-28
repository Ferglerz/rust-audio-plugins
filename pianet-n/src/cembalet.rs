//! Hohner Cembalet plectrum, independent sprung damper and early RF receiver.
use crate::{beam, scale, Model, Observation};
use pleasant_dsp::circuit::{Circuit, Filter};
use std::f64::consts::{PI, TAU};

const STROKE: f64 = 0.004;
const ESCAPE: f64 = 0.585;
const REARM: f64 = 0.1;
const TOUCH: f64 = 0.16;

pub struct Key {
    modes: Vec<beam::Mode>,
    damper_ratio: Vec<f64>,
    dt: f64,
    height: f64,
    target: f64,
    speed: f64,
    armed: bool,
    damper_landed: bool,
    pluck: f64,
    stiffness: f64,
    damper_stiffness: f64,
    damper_drag: f64,
    pick_response: f64,
    damper_response: f64,
    damper_velocity_response: f64,
    damper_preload: f64,
    area: f64,
    gap: f64,
    native_gap: f64,
    rest_tip: f64,
}

impl Key {
    pub fn new(midi: u8, rate: f64) -> Option<Self> {
        Self::for_model(Model::Cembalet, midi, rate)
    }
    pub fn for_model(model: Model, midi: u8, rate: f64) -> Option<Self> {
        if !model.key_range().contains(&midi) {
            return None;
        }
        Self::extended(model, midi, rate)
    }
    pub(crate) fn extended(model: Model, midi: u8, rate: f64) -> Option<Self> {
        if !model.is_cembalet() || midi > 127 || !rate.is_finite() || rate < 16000.0 {
            return None;
        }
        let range = model.key_range();
        let anchor = midi.clamp(*range.start(), *range.end());
        let s = scale::section(model, anchor)?;
        let pitch = s.hz * 2f64.powf((f64::from(midi) - f64::from(anchor)) / 12.);
        let (thickness, mass, modes) =
            beam::modes_at_pitch(&s, pitch, rate, 0.93, |hz| s.loss * (hz / pitch).powf(1.30));
        let damper_ratio: Vec<_> = modes
            .iter()
            .enumerate()
            .map(|(i, m)| beam::damper_shape(i, 0.80) / m.pad)
            .collect();
        let static_pick = modes.iter().map(|m| m.pad * m.compliance).sum::<f64>();
        let static_damper = modes
            .iter()
            .zip(&damper_ratio)
            .map(|(m, r)| m.pad * m.compliance * r * r)
            .sum::<f64>();
        let damper_stiffness = 3.0 / static_damper;
        let damper_drag = 1.55 * (mass * damper_stiffness).sqrt();
        let pick_response = modes.iter().map(|m| m.pad * m.dx_per_force).sum();
        let damper_response = modes
            .iter()
            .zip(&damper_ratio)
            .map(|(m, r)| m.pad * r * r * m.dx_per_force)
            .sum();
        let damper_velocity_response = modes
            .iter()
            .zip(&damper_ratio)
            .map(|(m, r)| m.pad * r * r * (m.omega * m.dq_per_force - m.loss * m.dx_per_force))
            .sum();
        let mut key = Self {
            modes,
            damper_ratio,
            dt: 1. / rate,
            height: 0.,
            target: 0.,
            speed: 0.,
            armed: true,
            damper_landed: true,
            pluck: s.pluck,
            stiffness: 3. / static_pick,
            damper_stiffness,
            damper_drag,
            pick_response,
            damper_response,
            damper_velocity_response,
            damper_preload: -s.pluck * 0.04 / (static_damper + 1. / damper_stiffness),
            area: s.width * thickness * 2.6,
            gap: s.gap,
            native_gap: s.gap,
            rest_tip: 0.,
        };
        key.reset();
        key.rest_tip = key.modes.iter().map(|m| m.position).sum();
        Some(key)
    }
    pub fn set_pickup_distance_mm(&mut self, offset: f32) {
        self.gap = self.native_gap + f64::from(offset.clamp(-0.15, 1.)) * 0.001;
    }
    pub(crate) fn capacitance(&self, tip: f64) -> f64 {
        let overlap = 0.5 + ((tip + 0.5 * self.gap) / self.gap).atan() / PI;
        8.8541878e-12 * self.area / self.gap * overlap
    }
    pub(crate) fn resting_capacitance(&self) -> f64 {
        self.capacitance(self.rest_tip)
    }
    pub fn reset(&mut self) {
        self.target = 0.;
        self.height = 0.;
        self.speed = 0.;
        self.armed = true;
        self.damper_landed = true;
        for (m, &r) in self.modes.iter_mut().zip(&self.damper_ratio) {
            m.position = m.compliance * r * self.damper_preload;
            m.quadrature = m.loss * m.position / m.omega;
        }
    }
    pub fn press(&mut self, velocity: u8) {
        self.move_to(if velocity == 0 { 0. } else { 1. }, velocity);
    }
    pub fn release(&mut self) {
        self.move_to(0., 0);
    }
    /// Sweep key depth to test escapement and incomplete returns without resetting the reed.
    pub fn move_to(&mut self, depth: f64, velocity: u8) {
        self.target = STROKE * depth.clamp(0., 1.);
        self.speed = if self.target < self.height {
            0.44
        } else {
            0.23 + 0.065 * f64::from(velocity.min(127)) / 127.
        };
    }
    pub fn step(&mut self) -> Observation {
        let old = self.height;
        self.height +=
            (self.target - self.height).clamp(-self.speed * self.dt, self.speed * self.dt);
        let depth = self.height / STROKE;
        if depth < REARM && self.damper_landed {
            self.armed = true;
        }
        if depth >= ESCAPE && self.armed {
            self.damper_landed = false;
            self.armed = false;
        }
        let mut pick_x = 0.;
        let mut damper_x = 0.;
        let mut damper_v = 0.;
        for (m, &r) in self.modes.iter_mut().zip(&self.damper_ratio) {
            m.free();
            pick_x += m.pad * m.position;
            damper_x += r * m.pad * m.position;
            damper_v += r * m.pad * m.velocity();
        }
        // Overlap vanishes as the plectrum rotates laterally past the reed edge.
        // On the return stroke the flexible blade folds under the reed. The damper
        // arm reaches the reed before enough travel is available to rearm the pick.
        let pick_y = (depth - TOUCH) / (ESCAPE - TOUCH) * self.pluck;
        let pick = if self.armed && depth >= TOUCH {
            (self.stiffness * (pick_y - pick_x) / (1. + self.stiffness * self.pick_response))
                .max(0.)
        } else {
            0.
        };
        let arm_travel = (depth - 0.20).max(0.);
        // A spring holds the damper on the reed over the first part of the stroke.
        let damper_y = self.pluck * (-0.04 + 12. * arm_travel);
        let arm_speed = if depth > 0.20 {
            12. * self.pluck / STROKE * (self.height - old) / self.dt
        } else {
            0.
        };
        let damper = if damper_y < damper_x {
            ((self.damper_stiffness * (damper_y - damper_x)
                + self.damper_drag * (arm_speed - damper_v))
                / (1.
                    + self.damper_stiffness * self.damper_response
                    + self.damper_drag * self.damper_velocity_response))
                .min(0.)
        } else {
            0.
        };
        if damper < 0. && depth < 0.40 && self.target < STROKE {
            self.damper_landed = true;
        }
        let mut tip = 0.;
        let mut at_pick = 0.;
        for (m, &r) in self.modes.iter_mut().zip(&self.damper_ratio) {
            let force = pick + r * damper;
            m.position += m.dx_per_force * force;
            m.quadrature += m.dq_per_force * force;
            tip += m.position;
            at_pick += m.pad * m.position;
        }
        Observation {
            pad_height: pick_y,
            reed_at_pad: at_pick,
            reed_tip: tip,
            pad_force: pick,
            bonded: false,
            seating: 0.,
            capacitance: self.capacitance(tip),
            flux_wb_turns: 0.,
            suction_pa: 0.,
            damper_force: damper,
            pick_engaged: pick > 0.,
            pick_armed: self.armed,
            key_fraction: depth,
        }
    }
}

const CARRIER: f64 = 1.75e6;
const BANK: f64 = 105e-12;
const Q: f64 = 28.;
const FLANK: f64 = std::f64::consts::FRAC_1_SQRT_2;
const EXCITATION: f64 = 3.;

fn rectified(amplitude: f64) -> f64 {
    let threshold = 0.38;
    if amplitude <= threshold {
        return 0.;
    }
    let angle = (threshold / amplitude).acos();
    (amplitude * angle.sin() - threshold * angle) / PI
}

/// Complex rotating-frame solution of the tuned receiver at the transmit frequency.
/// The argument is the sum of moving electrodes' changes from resting capacitance.
pub struct Receiver {
    dt: f64,
    real: f64,
    imag: f64,
    rest: f64,
    flank: f64,
    previous: f64,
}
impl Receiver {
    pub fn new(rate: f64) -> Self {
        Self::at_flank(rate, FLANK)
    }
    /// Set the normalized resting susceptance B/G. Zero selects resonance center.
    pub fn at_flank(rate: f64, flank: f64) -> Self {
        let real = EXCITATION / (1. + flank * flank);
        let imag = -real * flank;
        Self {
            dt: 1. / rate,
            real,
            imag,
            rest: rectified(real.hypot(imag)),
            flank,
            previous: 0.,
        }
    }
    pub fn tick(&mut self, change: f64) -> f64 {
        let midpoint = 0.5 * (change + self.previous);
        self.previous = change;
        let capacitance = (BANK + midpoint).max(BANK * 0.25);
        let b = self.flank + Q * (capacitance - BANK) / BANK;
        let damping = TAU * CARRIER * BANK / (2. * capacitance * Q);
        let target_re = EXCITATION / (1. + b * b);
        let target_im = -target_re * b;
        let (s, c) = (-b * damping * self.dt).sin_cos();
        let decay = (-damping * self.dt).exp();
        let (x, y) = (self.real - target_re, self.imag - target_im);
        self.real = target_re + decay * (c * x - s * y);
        self.imag = target_im + decay * (s * x + c * y);
        rectified(self.real.hypot(self.imag)) - self.rest
    }
}
pub(crate) struct Panel {
    receiver: Receiver,
    audio: pleasant_dsp::circuit::CrossfadingFilter<4>,
    rate: f64,
    phase: f64,
    swell: f64,
    target: f64,
    modulation: f64,
    enable: bool,
    smoothing: f64,
    load: f64,
    interval: u32,
    count: u32,
}
fn tune(audio: &mut Filter<4>, rate: f64, swell: f64, load: f64) {
    // ECH81 heptode plate / coupling / triode grid / triode plate / feedback / knee / jack.
    let mut c = Circuit::<4>::new(7, rate);
    c.input(1, -0.00014); // estimated small-signal detector transconductance
    c.r(1, 0, 200e3);
    c.input(7, 0.00014);
    c.r(1, 7, 500e3);
    c.c(1, 2, 1e-9);
    c.r(2, 0, 400e3);
    c.r(2, 4, 1e6);
    c.r(3, 0, 100e3);
    c.r(3, 7, 22e3);
    c.gm(3, 7, 2, 0.002);
    c.r(7, 0, 1000.);
    c.c(7, 0, 50e-6);
    c.c(3, 4, 50e-9);
    c.r(4, 5, 20e3);
    c.c(5, 0, 100e-12);
    let lower = (250e3 * swell.powf(2.4)).max(0.01);
    c.r(5, 6, (250e3 - lower).max(0.01));
    c.r(6, 0, lower);
    c.r(6, 0, load);
    c.compile(6, audio);
}
impl Panel {
    pub fn new(rate: f64, load: f64) -> Self {
        let mut audio =
            pleasant_dsp::circuit::CrossfadingFilter::with_length((rate / 960.).round() as u32);
        tune(audio.prepare(), rate, 1., load);
        Self {
            receiver: Receiver::new(rate),
            audio,
            rate,
            phase: 0.,
            swell: 1.,
            target: 1.,
            modulation: 0.,
            enable: false,
            smoothing: -(-1. / (rate * 0.008)).exp_m1(),
            load,
            interval: (rate / 960.).round() as u32,
            count: 0,
        }
    }
    pub fn set(&mut self, swell: f32, vibrato: bool) {
        self.target = f64::from(swell.clamp(0., 1.));
        self.enable = vibrato;
    }
    pub fn reset(&mut self) {
        self.receiver = Receiver::new(self.rate);
        self.audio.reset();
        self.phase = 0.;
        self.modulation = 0.;
        self.swell = self.target;
        tune(self.audio.prepare(), self.rate, self.swell, self.load);
        self.count = 0;
    }
    pub fn tick(&mut self, change: f64) -> f64 {
        if self.count == 0 {
            if self.swell != self.target {
                self.swell += (self.target - self.swell)
                    * (-(-f64::from(self.interval) / (self.rate * 0.008)).exp_m1());
                if (self.swell - self.target).abs() < 1e-5 {
                    self.swell = self.target;
                }
                tune(self.audio.prepare(), self.rate, self.swell, self.load);
            }
            self.count = self.interval;
        }
        self.count -= 1;
        self.modulation += (f64::from(self.enable) - self.modulation) * self.smoothing;
        self.phase = (self.phase + 7. / self.rate).fract();
        self.audio.tick(
            self.receiver.tick(change) * (1. + 0.30 * self.modulation * (TAU * self.phase).sin()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(key: &mut Key, frames: usize) -> Observation {
        let mut o = key.step();
        for _ in 1..frames {
            o = key.step();
        }
        o
    }
    #[test]
    fn damper_lands_before_plectrum_rearms() {
        for midi in [36, 60, 84, 96] {
            let mut k = Key::new(midi, 96000.).unwrap();
            k.move_to(0.45, 80);
            assert!(run(&mut k, 5000).pick_armed);
            k.release();
            run(&mut k, 5000);
            k.press(80);
            let mut e = k.step();
            while e.pick_armed {
                e = k.step();
            }
            assert!((0.585..0.588).contains(&e.key_fraction));
            run(&mut k, 8000);
            k.release();
            assert!(!run(&mut k, 300).pick_armed);
            k.press(80);
            assert!(!run(&mut k, 3000).pick_armed);
            k.release();
            let mut landed = false;
            for _ in 0..12000 {
                let o = k.step();
                if o.pick_armed {
                    assert!(landed, "MIDI {midi} rearms before damper contact");
                    break;
                }
                landed |= o.damper_force < 0.;
            }
            assert!(k.step().pick_armed);
            k.press(80);
            assert!(!run(&mut k, 5000).pick_armed);
        }
    }
}
