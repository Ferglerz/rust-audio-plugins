use crate::{
    lfo_table::CYCLES,
    panel::{optical_load, Controls, Panel},
    seventy,
    seventy_pickup::{OriginalPickup, PickupBank},
    KeySpec, LiveKey,
};
use pleasant_dsp::sympathetic::{ReceiverSpec, Sympathetic};
use std::f64::consts::PI;
pub const LOW: u8 = 21;
pub const HIGH: u8 = 108;
pub(crate) const FINE_TREBLE_FROM: u8 = 100;
const TAPS: usize = 65;
pub const LATENCY: u32 = 16;

/// The period follows the series resistance of the dual speed pot.
fn oscillator(speed: f64) -> (usize, f64, f64) {
    let speed = speed.clamp(0., 1.);
    let part = usize::from(speed > 0.5);
    let blend = 2. * speed - part as f64;
    let hz = 1. / ((1. - blend) / CYCLES[part].0 + blend / CYCLES[part + 1].0);
    (part, blend, hz)
}

pub fn tremolo_hz(speed: f64) -> f64 {
    oscillator(speed).2
}

// Band-limited interpolation and decimation around the 4x electronic circuit.
struct Resampler {
    coeff: [f64; TAPS],
    input: [[f64; 17]; 3],
    output: [f64; TAPS],
    input_pos: usize,
    output_pos: usize,
}
impl Resampler {
    fn new() -> Self {
        let mut coeff = std::array::from_fn(|i| {
            let t = i as f64 - 32.;
            let w = 2. * PI * i as f64 / 64.;
            let sinc = if t == 0. {
                0.23
            } else {
                (0.23 * PI * t).sin() / (PI * t)
            };
            sinc * (0.42 - 0.5 * w.cos() + 0.08 * (2. * w).cos())
        });
        let sum: f64 = coeff.iter().sum();
        for c in &mut coeff {
            *c /= sum;
        }
        Self {
            coeff,
            input: [[0.; 17]; 3],
            output: [0.; TAPS],
            input_pos: 0,
            output_pos: 0,
        }
    }
    fn clock<const N: usize>(
        &mut self,
        inputs: [f64; N],
        mut circuit: impl FnMut([f64; N]) -> f64,
    ) -> f64 {
        for (history, x) in self.input.iter_mut().zip(inputs) {
            history[self.input_pos] = x;
        }
        let mut result = 0.;
        for phase in 0..4 {
            let up = std::array::from_fn(|bus| {
                let mut sum = 0.;
                let mut pos = self.input_pos;
                for tap in (phase..TAPS).step_by(4) {
                    sum += self.coeff[tap] * self.input[bus][pos];
                    pos = if pos == 0 { 16 } else { pos - 1 };
                }
                sum * 4.
            });
            // Provisional piezo sensitivity and digital headroom. The knee only
            // acts on unusually strong sums. Apply it before decimation.
            let x = 0.14 * circuit(up);
            let excess = (x.abs() - 0.60).max(0.);
            self.output[self.output_pos] = if excess > 0. {
                x.signum() * (0.60 + 0.30 * excess / (0.30 + excess))
            } else {
                x
            };
            if phase == 0 {
                let (a, b) = self.output.split_at(self.output_pos + 1);
                result = a
                    .iter()
                    .rev()
                    .chain(b.iter().rev())
                    .zip(self.coeff)
                    .map(|(x, c)| x * c)
                    .sum();
            }
            self.output_pos = (self.output_pos + 1) % TAPS;
        }
        self.input_pos = (self.input_pos + 1) % 17;
        result
    }
}

enum Wiring {
    Cp80,
    Cp70B(Box<PickupBank>),
    Cp70(OriginalPickup),
}

pub struct Engine {
    low: u8,
    high: u8,
    keys: Vec<LiveKey>,
    frame: Sympathetic<88>,
    frame_mix: f64,
    active: u128,
    held: u128,
    audition: Option<u8>,
    choke: [u32; 128],
    age: u32,
    panel: Panel,
    wiring: Wiring,
    resampler: Resampler,
    controls: Controls,
    rate: f64,
    phase: f64,
    oscillator: (usize, f64, f64),
    speed: f64,
    speed_ramp: pleasant_dsp::interpolation::ControlRamp,
    gate_load: f64,
    target_gate_load: f64,
    light: [f64; 2],
    depth: f64,
    wet: f64,
    optical_alpha: [f64; 2],
    control_alpha: f64,
}
impl Engine {
    pub fn new(rate: f64) -> Self {
        Self::with_range(rate, LOW, HIGH, None)
    }
    pub fn extended(rate: f64) -> Self {
        Self::with_range(rate, 0, 127, None)
    }
    pub fn cp70b(rate: f64, extended: bool) -> Self {
        Self::seventy(rate, extended, seventy::Revision::B)
    }
    pub fn cp70(rate: f64, extended: bool) -> Self {
        Self::seventy(rate, extended, seventy::Revision::Original)
    }
    fn seventy(rate: f64, extended: bool, revision: seventy::Revision) -> Self {
        let (low, high) = if extended {
            (0, 127)
        } else {
            (seventy::LOW, seventy::HIGH)
        };
        Self::with_range(rate, low, high, Some(revision))
    }
    fn with_range(rate: f64, low: u8, high: u8, revision: Option<seventy::Revision>) -> Self {
        let fine_treble_from = if revision.is_some() {
            seventy::FINE_TREBLE_FROM
        } else {
            FINE_TREBLE_FROM
        };
        let spec = |n| match revision {
            Some(r) => seventy::extended_for(r, n),
            None => KeySpec::extended(n),
        };
        Self {
            low,
            high,
            keys: (low..=high)
                .map(|n| LiveKey::realtime(spec(n), rate, fine_treble_from))
                .collect(),
            // No soundboard. Use a weak, broad frame mode as an optional
            // approximation until a dry cross-string transfer is measured.
            frame: Sympathetic::new(
                rate,
                LOW,
                ((1_u128 << 88) - 1) ^ ((1_u128 << 73) - 1),
                300.,
                190.,
                0.8,
                |i| {
                    let n = LOW + i as u8;
                    let spec = KeySpec::for_note(n).unwrap();
                    ReceiverSpec {
                        hz: spec.frequency(),
                        decay: spec.losses_db_s[0] * std::f64::consts::LN_10 / 20.,
                        damped_decay: 55. * 1.14,
                        pickup: [if n >= 103 { 104. / 22. } else { 1. }, 0.],
                    }
                },
            ),
            frame_mix: 0.,
            active: 0,
            held: 0,
            audition: None,
            choke: [0; 128],
            age: 0,
            // Original TR values are provisional. Both panels have the same pot types.
            panel: if revision.is_some() {
                Panel::cp70b(rate * 4.)
            } else {
                Panel::new(rate * 4.)
            },
            wiring: match revision {
                Some(seventy::Revision::Original) => Wiring::Cp70(OriginalPickup::new(rate * 4.)),
                Some(seventy::Revision::B) => Wiring::Cp70B(Box::new(PickupBank::new(rate * 4.))),
                None => Wiring::Cp80,
            },
            resampler: Resampler::new(),
            controls: Default::default(),
            rate,
            phase: 0.,
            oscillator: oscillator(Controls::default().speed),
            speed: Controls::default().speed,
            speed_ramp: pleasant_dsp::interpolation::ControlRamp::new(rate),
            gate_load: optical_load(Controls::default().volume),
            target_gate_load: optical_load(Controls::default().volume),
            light: [1.; 2],
            depth: 0.,
            wet: 0.,
            optical_alpha: [
                1. - (-1. / (0.002 * rate)).exp(),
                1. - (-1. / (0.008 * rate)).exp(),
            ],
            control_alpha: 1. - (-1. / (0.003 * rate)).exp(),
        }
    }
    pub fn reset(&mut self) {
        for k in &mut self.keys {
            k.clear();
            k.sustain(false);
        }
        self.choke.fill(0);
        self.active = 0;
        self.held = 0;
        self.frame.clear();
        self.frame_mix = f64::from(self.controls.frame_ring);
        self.audition = None;
        self.phase = 0.;
        self.speed = self.controls.speed;
        self.oscillator = oscillator(self.speed);
        self.speed_ramp.reset();
        self.light = [1.; 2];
        self.wet = f64::from(self.controls.tremolo);
        self.gate_load = self.target_gate_load;
        self.depth = self.controls.depth;
        self.panel.reset();
        match &mut self.wiring {
            Wiring::Cp70(bank) => bank.reset(),
            Wiring::Cp70B(bank) => bank.reset(),
            Wiring::Cp80 => {}
        }
        self.resampler = Resampler::new();
    }
    pub fn set(&mut self, mut c: Controls) {
        if !matches!(self.wiring, Wiring::Cp80) {
            c.frame_ring = false;
            c.brilliance = 1;
        }
        if c.speed != self.controls.speed && self.speed_ramp.retarget() {
            self.speed = c.speed;
            self.oscillator = oscillator(self.speed);
        }
        if c.volume != self.controls.volume {
            self.target_gate_load = optical_load(c.volume);
        }
        self.panel.set(c);
        self.controls = c;
    }
    pub fn note_on(&mut self, n: u8, v: f32) {
        if !(self.low..=self.high).contains(&n) {
            return;
        }
        if v <= 0. {
            self.note_off(n);
            return;
        }
        self.held |= 1u128 << n;
        self.strike(n, v);
    }
    fn strike(&mut self, n: u8, v: f32) {
        let i = usize::from(n - self.low);
        self.active |= 1u128 << n;
        self.frame.held(n, true);
        self.choke[i] = 0;
        self.keys[i].strike(v as f64);
    }
    pub fn note_off(&mut self, n: u8) {
        if !(self.low..=self.high).contains(&n) {
            return;
        }
        self.held &= !(1u128 << n);
        if self.audition != Some(n) {
            self.keys[usize::from(n - self.low)].release();
            self.frame.held(n, false);
        }
    }
    pub fn audition(&mut self, n: Option<u8>, v: f32) {
        if let Some(prev) = self.audition.take() {
            if self.held & (1u128 << prev) == 0 {
                self.keys[usize::from(prev - self.low)].release();
                self.frame.held(prev, false);
            }
        }
        self.audition = n.filter(|p| (self.low..=self.high).contains(p));
        if let Some(p) = self.audition {
            self.strike(p, v);
        }
    }
    pub fn pressed(&self) -> u128 {
        self.held | self.audition.map_or(0, |n| 1u128 << n)
    }
    pub fn sustain(&mut self, pedal: bool) {
        self.frame.pedal(pedal);
        for k in &mut self.keys {
            k.sustain(pedal);
        }
    }
    pub fn release_all(&mut self) {
        for n in self.low..=self.high {
            self.note_off(n);
        }
    }
    pub fn choke_all(&mut self) {
        for note in self.low..=self.high {
            self.choke(note);
        }
    }
    pub fn choke(&mut self, n: u8) {
        if (self.low..=self.high).contains(&n) {
            self.note_off(n);
            if self.audition == Some(n) {
                self.audition = None;
            }
            self.frame.choke(n);
            self.choke[usize::from(n - self.low)] = (self.rate * 0.005) as u32;
        }
    }
    pub fn sounding(&self) -> bool {
        self.active != 0 || (self.frame_mix > 0. && self.frame.ringing())
    }
    pub fn sample(&mut self) -> [f32; 2] {
        let mut mask = self.active;
        let mut voltage = 0.;
        let mut branches = [0.; 3];
        let was_coupled = self.frame_mix > 0.;
        let target = f64::from(self.controls.frame_ring);
        self.frame_mix +=
            (target - self.frame_mix).clamp(-1. / (0.005 * self.rate), 1. / (0.005 * self.rate));
        let coupled = self.frame_mix > 0.;
        if was_coupled && !coupled {
            self.frame.clear_motion();
        }
        self.age = self.age.wrapping_add(1);
        while mask != 0 {
            let n = mask.trailing_zeros() as u8;
            let bit = 1u128 << n;
            mask &= !bit;
            let i = usize::from(n - self.low);
            let k = &mut self.keys[i];
            let mut force = k.host_sample();
            if self.choke[i] > 0 {
                force *= self.choke[i] as f64 / (self.rate * 0.005);
                self.choke[i] -= 1;
                if self.choke[i] == 0 {
                    k.clear();
                    self.active &= !bit;
                }
            }
            if (self.age.wrapping_add(u32::from(n) * 11) & 1023) == 0 && k.quiet() {
                k.clear();
                self.active &= !bit;
            }
            if coupled {
                self.frame.drive(n, force);
            }
            match self.wiring {
                Wiring::Cp70(_) => voltage += force,
                Wiring::Cp70B(_) => branches[seventy::bus(n)] += force,
                Wiring::Cp80 => voltage += force * if n >= 103 { 104. / 22. } else { 1. },
            }
        }
        if coupled {
            voltage += self.frame_mix * self.frame.tick()[0];
        }
        let dry = match &mut self.wiring {
            Wiring::Cp70(bank) => self
                .resampler
                .clock([voltage], |[x]| self.panel.tick(bank.tick(x), 0.)),
            Wiring::Cp70B(bank) => self
                .resampler
                .clock(branches, |force| self.panel.tick(bank.tick(force), 0.)),
            Wiring::Cp80 => self
                .resampler
                .clock([voltage], |[x]| self.panel.tick(x, 0.)),
        };
        self.optical_gain().map(|gain| (dry * gain) as f32)
    }

    fn optical_gain(&mut self) -> [f64; 2] {
        let c = self.controls;
        if let Some(step) = self.speed_ramp.step() {
            self.speed += step * (c.speed - self.speed);
            self.oscillator = oscillator(self.speed);
        }
        let (part, blend, freq) = self.oscillator;
        self.phase = (self.phase + freq / self.rate).fract();
        self.wet += (f64::from(c.tremolo) - self.wet) * self.control_alpha;
        self.depth += (c.depth - self.depth) * self.control_alpha;
        self.gate_load += (self.target_gate_load - self.gate_load) * self.control_alpha;
        let phase = self.phase * 64.;
        let i = phase as usize;
        let frac = phase - i as f64;
        let mut out = [0.; 2];
        for (ch, y) in out.iter_mut().enumerate() {
            let lookup = |j: usize| {
                let t = &CYCLES[j].1[ch];
                t[i] + frac * (t[(i + 1) % 64] - t[i])
            };
            let current = lookup(part) * (1. - blend) + lookup(part + 1) * blend;
            let alpha = self.optical_alpha[usize::from(current < self.light[ch])];
            self.light[ch] += (current - self.light[ch]) * alpha;
            // A power law approximates the MCD527 light-dependent resistance.
            let rest = 11343.;
            let series = 20000. * (1. - self.depth).powi(3);
            let cell = rest / self.light[ch].max(0.02).powf(1.15);
            let gain = |r: f64| 1. / (1. + 22000. * (1. / r + self.gate_load));
            let modulation = gain(cell + series) / gain(rest + series);
            *y = 1. + self.wet * (modulation - 1.);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_tremolo_reaches_both_service_depths_and_alternates() {
        for depth in [0., 1.] {
            let mut e = Engine::new(48000.);
            e.set(Controls {
                tremolo: true,
                speed: 1.,
                depth,
                ..Default::default()
            });
            for _ in 0..24000 {
                e.optical_gain();
            }
            let mut lo = [f64::INFINITY; 2];
            let mut hi = [0.0_f64; 2];
            let mut opposed = false;
            for _ in 0..9600 {
                let g = e.optical_gain();
                for i in 0..2 {
                    lo[i] = lo[i].min(g[i]);
                    hi[i] = hi[i].max(g[i]);
                }
                opposed |= g[0] > 1. && g[1] < 1.;
            }
            assert!(opposed);
            for i in 0..2 {
                let modulation = (hi[i] - lo[i]) / (hi[i] + lo[i]);
                assert!(
                    if depth == 0. {
                        modulation < 0.15
                    } else {
                        modulation > 0.40
                    },
                    "depth {depth}, modulation {modulation}"
                );
            }
        }
    }

    #[test]
    fn release_respects_audition_and_external_key_ownership() {
        let mut e = Engine::new(48000.);
        e.note_on(42, 0.8);
        e.audition(Some(42), 0.5);
        e.audition(None, 0.);
        assert_eq!(e.pressed(), 1 << 42);
        e.audition(Some(42), 0.6);
        e.note_off(42);
        assert_eq!(e.pressed(), 1 << 42);
        e.audition(None, 0.);
        e.note_on(2, 1.);
        assert_eq!(e.pressed(), 0);
    }

    #[test]
    fn panel_stays_finite_at_supported_rates_after_a_strike() {
        for rate in [
            22050., 44100., 48000., 88200., 96000., 176400., 192000., 384000.,
        ] {
            let mut e = Engine::new(rate);
            e.note_on(108, 1.);
            let mut peak = 0.0_f32;
            for _ in 0..16000 {
                let s = e.sample();
                assert!(s[0].is_finite() && s[0] == s[1]);
                peak = peak.max(s[0].abs());
            }
            assert!((0.01..1.).contains(&peak), "{rate} Hz, peak {peak}");
        }
    }
}
