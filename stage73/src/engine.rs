//! Framework-independent Stage 73, calibrated to a passive late Mark I.
use crate::{circuit::Panel, field::Field, tine::Tine};
use crate::{
    era::Era,
    period_panel::{EarlyPanel, Jordan},
};
use std::f64::consts::PI;
pub const LOW: u8 = 28;
pub const HIGH: u8 = 100;
pub const LATENCY: u32 = 12;
const OS: usize = 4;
const TAPS: usize = 2 * LATENCY as usize * OS + 1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub era: Era,
    pub peterson: bool,
    pub peterson_hz: f64,
    pub intensity: f64,
    pub jordan_volume: f64,
    pub jordan_bass: f64,
    pub treble: f64,
    pub rate_hz: f64,
    pub depth: f64,
    pub volume: f64,
    pub bass: f64,
    pub cable_pf: f64,
    pub height_mm: f64,
    pub gap_mm: f64,
    /// Post-circuit playback alignment. Unity measures the original jack level.
    pub playback_gain: f64,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            era: Era::LateI,
            peterson: false,
            peterson_hz: 5.,
            intensity: 0.7,
            jordan_volume: 0.8,
            jordan_bass: 0.5,
            treble: 0.5,
            rate_hz: 5.0,
            depth: 0.0,
            volume: 1.0,
            bass: 1.0,
            cable_pf: crate::circuit::DEFAULT_CABLE_PF,
            height_mm: 0.0,
            gap_mm: 0.0,
            playback_gain: 1.0,
        }
    }
}

struct Decimator {
    coeff: [f64; TAPS],
    ring: [f64; TAPS],
    cursor: usize,
}
impl Decimator {
    fn new() -> Self {
        let mut coeff = std::array::from_fn(|i| {
            let t = i as f64 - (TAPS / 2) as f64;
            let phase = 2.0 * PI * i as f64 / (TAPS - 1) as f64;
            let window = 0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos();
            let cutoff = 0.455 / OS as f64;
            window
                * if t == 0.0 {
                    2.0 * cutoff
                } else {
                    (2.0 * PI * cutoff * t).sin() / (PI * t)
                }
        });
        let gain: f64 = coeff.iter().sum();
        for c in &mut coeff {
            *c /= gain;
        }
        Self {
            coeff,
            ring: [0.0; TAPS],
            cursor: 0,
        }
    }
    fn process(&mut self, block: [f64; OS]) -> f32 {
        for x in block {
            self.ring[self.cursor] = x;
            self.cursor = (self.cursor + 1) % TAPS;
        }
        let (old, new) = self.ring.split_at(self.cursor);
        new.iter()
            .chain(old)
            .zip(&self.coeff)
            .map(|(x, c)| x * c)
            .sum::<f64>() as f32
    }
}

pub struct Stage73Engine {
    rate: f64,
    low: u8,
    high: u8,
    voices: Box<[Tine]>,
    harp: crate::harp::Harp,
    active: u128,
    pedal: bool,
    audition: Option<u8>,
    field: &'static Field,
    panel: Panel,
    early: EarlyPanel,
    jordan: Jordan,
    peterson: crate::peterson::Peterson,
    suitcase_out: Decimator,
    // Parallel circuit outputs are crossfaded without reallocating or
    // restarting tines. Rapid automation starts from the current mixture.
    mix: [f64; 4],
    mix_target: [f64; 4],
    mix_start: [f64; 4],
    mix_ramp: u32,
    control_clock: u8,
    harp_tap: bool,
    decimator: Decimator,
    now: Controls,
    target: Controls,
    ramp: u32,
}
impl Stage73Engine {
    pub fn new(rate: f64) -> Self {
        Self::with_range(rate, LOW, HIGH)
    }
    pub fn extended(rate: f64) -> Self {
        Self::with_range(rate, 0, 127)
    }
    fn with_range(rate: f64, low: u8, high: u8) -> Self {
        assert!(rate.is_finite() && rate >= 8000.0);
        let subrate = rate * OS as f64;
        Self {
            rate,
            low,
            high,
            voices: (low..=high).map(|n| Tine::new(n, subrate)).collect(),
            active: 0,
            harp: crate::harp::Harp::new(rate),
            pedal: false,
            audition: None,
            field: Field::shared(),
            panel: Panel::new(subrate),
            early: EarlyPanel::new(subrate),
            jordan: Jordan::new(subrate),
            peterson: crate::peterson::Peterson::new(rate, OS),
            suitcase_out: Decimator::new(),
            mix: [1., 0., 0., 0.],
            mix_target: [1., 0., 0., 0.],
            mix_start: [1., 0., 0., 0.],
            mix_ramp: 0,
            control_clock: 0,
            harp_tap: false,
            decimator: Decimator::new(),
            now: Controls::default(),
            target: Controls::default(),
            ramp: 0,
        }
    }
    pub fn is_ringing(&self) -> bool {
        self.active != 0 || self.harp.modes.ringing()
    }

    pub fn reset(&mut self) {
        for v in &mut self.voices {
            v.silence();
        }
        self.active = 0;
        self.harp.modes.clear();
        self.pedal = false;
        self.audition = None;
        self.panel = Panel::new(self.rate * OS as f64);
        self.decimator.ring.fill(0.0);
        self.decimator.cursor = 0;
        self.early.clear();
        self.jordan.clear();
        self.peterson.clear();
        self.suitcase_out.ring.fill(0.);
        self.suitcase_out.cursor = 0;
        self.mix = self.mix_target;
        self.mix_ramp = 0;
        self.control_clock = 0;
        self.now = self.target;
        self.harp
            .voice(self.now.era, self.now.height_mm, self.now.gap_mm);
        self.ramp = 0;
    }
    /// Open-circuit harp voltage for offline comparisons recorded before the panel.
    pub fn set_harp_tap(&mut self, enabled: bool) {
        self.harp_tap = enabled;
    }

    pub fn set_controls(&mut self, c: Controls) {
        let bounded = |value: f64, lo: f64, hi: f64, default: f64| {
            if value.is_finite() {
                value.clamp(lo, hi)
            } else {
                default
            }
        };
        let c = Controls {
            era: c.era,
            peterson: c.peterson,
            peterson_hz: bounded(
                c.peterson_hz,
                crate::peterson::MIN_HZ,
                crate::peterson::MAX_HZ,
                5.,
            ),
            intensity: bounded(c.intensity, 0., 1., 0.7),
            jordan_volume: bounded(c.jordan_volume, 0., 1., 0.8),
            jordan_bass: bounded(c.jordan_bass, 0., 1., 0.5),
            treble: bounded(c.treble, 0., 1., 0.5),
            rate_hz: bounded(
                c.rate_hz,
                crate::period_panel::MIN_TREM_HZ,
                crate::period_panel::MAX_TREM_HZ,
                5.,
            ),
            depth: bounded(c.depth, 0., 1., 0.),
            volume: bounded(c.volume, 0.0, 1.0, 1.0),
            bass: bounded(c.bass, 0.0, 1.0, 1.0),
            cable_pf: bounded(
                c.cable_pf,
                crate::circuit::MIN_CABLE_PF,
                crate::circuit::MAX_CABLE_PF,
                crate::circuit::DEFAULT_CABLE_PF,
            ),
            height_mm: bounded(c.height_mm, -3.0, 3.0, 0.0),
            gap_mm: bounded(c.gap_mm, -1.0, 1.0, 0.0),
            playback_gain: bounded(c.playback_gain, 0., 4., 1.),
        };
        if c.era != self.target.era || c.peterson != self.target.peterson {
            let next = match c.era {
                Era::Sparkletop => [0., 0., 1., 0.],
                _ if c.peterson => [0., 0., 0., 1.],
                Era::EarlyI => [0., 1., 0., 0.],
                _ => [1., 0., 0., 0.],
            };
            if next != self.mix_target {
                // Clear a disconnected circuit before reconnecting it.
                if next[0] > 0. && self.mix[0] == 0. {
                    self.panel = Panel::new(self.rate * OS as f64);
                }
                if next[1] > 0. && self.mix[1] == 0. {
                    self.early.clear();
                }
                if next[2] > 0. && self.mix[2] == 0. {
                    self.jordan.clear();
                }
                if next[3] > 0. && self.mix[3] == 0. {
                    self.peterson.clear_audio();
                    self.suitcase_out.ring.fill(0.);
                }
                self.mix_start = self.mix;
                self.mix_target = next;
                self.mix_ramp = (self.rate * 0.010) as u32;
                self.control_clock = 0;
            }
        }
        if c != self.target {
            self.target = c;
            self.ramp = (0.01 * self.rate) as u32;
        }
    }
    pub fn note_on(&mut self, note: u8, velocity: f32) {
        if !(self.low..=self.high).contains(&note) {
            return;
        }
        if velocity <= 0.0 {
            self.note_off(note);
            return;
        }
        self.voices[(note - self.low) as usize].held = true;
        self.strike(note, velocity);
    }

    /// A mouse and a MIDI key can hold the same tine independently.
    pub fn audition(&mut self, note: Option<u8>, velocity: f32) {
        if let Some(old) = self.audition.take() {
            let v = &mut self.voices[(old - self.low) as usize];
            v.damp(!self.pedal && !v.held);
            self.harp.modes.held(old, v.held);
        }
        self.audition = note.filter(|n| (self.low..=self.high).contains(n));
        if let Some(note) = self.audition {
            self.strike(note, velocity);
        }
    }

    pub fn playing_notes(&self) -> u128 {
        let mut notes = 0;
        let mut active = self.active;
        while active != 0 {
            let i = active.trailing_zeros() as usize;
            active &= active - 1;
            if self.voices[i].undamped() {
                notes |= 1 << (self.low as usize + i);
            }
        }
        notes
    }

    fn strike(&mut self, note: u8, velocity: f32) {
        let i = (note - self.low) as usize;
        self.harp.modes.held(note, true);
        self.voices[i].strike(
            velocity.min(1.0) as f64,
            self.rate * OS as f64,
            self.target.era,
        );
        self.active |= 1u128 << i;
    }
    pub fn note_off(&mut self, note: u8) {
        if !(self.low..=self.high).contains(&note) {
            return;
        }
        let v = &mut self.voices[(note - self.low) as usize];
        v.held = false;
        self.harp.modes.held(note, self.audition == Some(note));
        v.damp(!self.pedal && self.audition != Some(note));
    }
    pub fn sustain(&mut self, on: bool) {
        self.pedal = on;
        self.harp.modes.pedal(on);
        for (i, v) in self.voices.iter_mut().enumerate() {
            v.damp(!on && !v.held && self.audition != Some(self.low + i as u8));
        }
    }
    pub fn release_all(&mut self) {
        self.audition = None;
        for n in self.low..=self.high {
            self.note_off(n);
        }
    }
    pub fn choke_all(&mut self) {
        for note in self.low..=self.high {
            self.choke(note);
        }
    }
    pub fn choke(&mut self, note: u8) {
        if self.audition == Some(note) {
            self.audition = None;
        }
        if (self.low..=self.high).contains(&note) {
            self.voices[(note - self.low) as usize].choke();
            self.harp.modes.choke(note);
        }
    }

    pub fn render(&mut self, out: &mut [f32]) {
        for y in out {
            let [left, right] = self.sample();
            *y = (left + right) * 0.5;
        }
    }

    pub fn render_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            [*l, *r] = self.sample();
        }
    }

    fn sample(&mut self) -> [f32; 2] {
        if self.ramp > 0 {
            let step = 1.0 / self.ramp as f64;
            let approach = |v: &mut f64, to: f64| *v += (to - *v) * step;
            approach(&mut self.now.volume, self.target.volume);
            approach(&mut self.now.bass, self.target.bass);
            approach(&mut self.now.cable_pf, self.target.cable_pf);
            approach(&mut self.now.height_mm, self.target.height_mm);
            approach(&mut self.now.gap_mm, self.target.gap_mm);
            approach(&mut self.now.playback_gain, self.target.playback_gain);
            approach(&mut self.now.jordan_volume, self.target.jordan_volume);
            approach(&mut self.now.jordan_bass, self.target.jordan_bass);
            approach(&mut self.now.treble, self.target.treble);
            approach(&mut self.now.rate_hz, self.target.rate_hz);
            approach(&mut self.now.depth, self.target.depth);
            approach(&mut self.now.peterson_hz, self.target.peterson_hz);
            approach(&mut self.now.intensity, self.target.intensity);
            self.ramp -= 1;
        }
        if self.mix_ramp > 0 {
            self.mix_ramp -= 1;
            let t = 1. - self.mix_ramp as f64 / (self.rate * 0.010).floor();
            let blend = t * t * (3. - 2. * t);
            self.mix = if self.mix_ramp == 0 {
                self.mix_target
            } else {
                std::array::from_fn(|i| {
                    self.mix_start[i] + blend * (self.mix_target[i] - self.mix_start[i])
                })
            };
        }
        let [stage, early, jordan, suitcase] = self.mix;
        if stage > 0. {
            self.panel
                .set_loaded(self.now.volume, self.now.bass, self.now.cable_pf);
        }
        if self.control_clock == 0 || self.ramp == 0 {
            if early > 0. {
                self.early
                    .set_loaded(self.now.volume, self.now.bass, self.now.cable_pf);
            }
            if jordan > 0. {
                self.jordan.set(
                    self.now.jordan_volume,
                    self.now.jordan_bass,
                    self.now.treble,
                );
            }
        }
        self.control_clock = (self.control_clock + 1) % 32;
        if jordan > 0. {
            self.jordan.tremolo(self.now.rate_hz, self.now.depth);
        }
        let mut original = [0.; OS];
        let mut sum = [0.0; OS];
        let mut bits = self.active;
        while bits != 0 {
            let i = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            let voice = &mut self.voices[i];
            for (k, s) in sum.iter_mut().enumerate() {
                let sample = voice.tick(self.field, self.now.height_mm, self.now.gap_mm);
                *s += sample;
                if jordan > 0. {
                    // Tine::tick carries the late group weight; undo it
                    // before applying the weight for the early winding.
                    let ratio = match (self.low + i as u8).saturating_sub(LOW) {
                        0..=3 => 4. / 7.,
                        4..=6 => 3. / 7.,
                        _ => 0.5,
                    };
                    original[k] += sample * ratio;
                }
            }
            self.harp.modes.drive(self.low + i as u8, voice.velocity);
            if !voice.active {
                self.active &= !(1u128 << i);
            }
        }
        // Flux normalization and nominal jack-to-host alignment. One gain
        // for every key, dynamic and panel setting.
        if self.control_clock == 0 {
            self.harp
                .voice(self.target.era, self.now.height_mm, self.now.gap_mm);
        }
        let [ring, early_ring] = self.harp.modes.tick();
        for (modern, vintage) in sum.iter_mut().zip(&mut original) {
            *modern += ring;
            *vintage += early_ring;
        }
        let gain = crate::calibration::JACK_GAIN;
        let jack = std::array::from_fn(|k| {
            let emf = sum[k] * gain;
            if self.harp_tap {
                return (stage + early + suitcase) * emf + jordan * original[k] * gain;
            }
            let mut x = 0.;
            if stage > 0. {
                x += stage * self.panel.tick(emf);
            }
            if early > 0. {
                x += early * self.early.tick(emf);
            }
            if jordan > 0. {
                x += jordan * self.jordan.tick(original[k] * gain);
            }
            x
        });
        let dry = self.decimator.process(jack);
        let optical = self
            .peterson
            .gains(self.now.peterson_hz, self.now.intensity);
        if self.harp_tap {
            return [dry; 2];
        }
        let level = self.now.playback_gain as f32;
        if suitcase > 0. {
            let preamp = sum.map(|emf| self.peterson.audio(emf * gain));
            let wet = self.suitcase_out.process(preamp) as f64 * suitcase;
            optical.map(|g| (dry + (g * wet) as f32) * level)
        } else {
            [dry * level; 2]
        }
    }
}
