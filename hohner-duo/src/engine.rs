//! Both actions follow the position of one key. Pickup switching happens after motion.
use crate::{circuit::Panel, Bank, Controls, HIGH, LAST_BASS, LOW};
use clavinet_d6::{live::LinkedString, panel::PICKUP_VOLTS, Pickups};
use pianet_n::{Model, Reed};
use pleasant_dsp::instrument::Decimator;

pub const LATENCY: u32 = 8;

struct Key {
    action: LinkedString,
    reed: Reed,
    flux: f64,
    reed_offset: f64,
    down: bool,
    repeat: u8,
    remaining: usize,
    // When the pedal catches a reed the string still retires on its stock timer.
    string_remaining: usize,
    string_parked: bool,
    reed_held: bool,
    choke: usize,
}

impl Key {
    fn new(midi: u8, rate: f64) -> Self {
        let mut reed = Reed::at_rate_extended(Model::T, midi, rate).expect("MIDI Duo reed");
        let flux = reed.step().flux_wb_turns;
        Self {
            action: LinkedString::new(midi, rate),
            reed,
            flux,
            reed_offset: 0.,
            down: false,
            repeat: 0,
            remaining: 0,
            string_remaining: 0,
            string_parked: false,
            reed_held: false,
            choke: 0,
        }
    }
    fn clear(&mut self) {
        self.action.string.clear();
        self.reed.reset();
        self.flux = self.reed.step().flux_wb_turns;
        self.down = false;
        self.repeat = 0;
        self.remaining = 0;
        self.string_remaining = 0;
        self.string_parked = false;
        self.reed_held = false;
        self.choke = 0;
    }
    fn strike(&mut self, velocity: u8) {
        self.reed.rearm_sustained();
        self.reed_held = false;
        self.string_remaining = 0;
        self.string_parked = false;
        self.choke = 0;
        self.remaining = 0;
        if self.down {
            self.action.string.release();
            self.repeat = velocity;
        } else {
            self.action.string.press(velocity);
            self.repeat = 0;
        }
        self.down = true;
    }
    fn lift(&mut self, tail: usize, catch_reed: bool) {
        self.down = false;
        self.repeat = 0;
        self.reed_held = catch_reed && self.reed.hold_for_sustain();
        if self.reed_held {
            self.string_remaining = tail;
            self.remaining = 0;
        } else {
            self.reed.release();
            self.remaining = tail;
        }
        self.action.string.release();
    }
    fn tick(&mut self, ticks_per_second: f64) -> (Pickups, f64) {
        let (pickup, fraction) = if self.string_parked {
            (Pickups::default(), 0.)
        } else {
            self.action.tick()
        };
        if self.string_remaining != 0 {
            self.string_remaining -= 1;
            if self.string_remaining == 0 {
                self.action.string.clear();
                self.string_parked = true;
            }
        }
        let o = self
            .reed
            .follow_key(fraction, self.down && self.repeat == 0);
        if self.repeat != 0 && fraction <= 0. {
            self.action.string.press(self.repeat);
            self.repeat = 0;
        }
        let voltage = (o.flux_wb_turns - self.flux) * ticks_per_second;
        self.flux = o.flux_wb_turns;
        (pickup, voltage)
    }
}

#[derive(Default, Clone, Copy)]
struct Section {
    string: Pickups,
    reed: f64,
}

pub struct Engine {
    keys: Vec<Key>,
    controls: Controls,
    panel: Panel,
    decimator: Decimator<3>,
    first: u8,
    last: u8,
    distance_mm: [f64; 3],
    release_noise: f64,
    sustain_pad: bool,
    active: u128,
    held: u128,
    audition: Option<u8>,
    substeps: usize,
    internal_rate: f64,
    release_ticks: usize,
    choke_ticks: usize,
    panel_tail: usize,
}

impl Engine {
    pub fn new(rate: f64) -> Self {
        Self::with_range(rate, LOW, HIGH)
    }
    pub fn extended(rate: f64) -> Self {
        Self::with_range(rate, 0, 127)
    }
    fn with_range(rate: f64, first: u8, last: u8) -> Self {
        assert!(rate.is_finite() && rate >= 8_000.);
        let substeps = (88_200. / rate).ceil() as usize;
        let internal_rate = rate * substeps as f64;
        let controls = Controls::default();
        Self {
            keys: (first..=last).map(|n| Key::new(n, internal_rate)).collect(),
            first,
            last,
            distance_mm: [0.; 3],
            release_noise: 1.,
            sustain_pad: false,
            controls,
            panel: Panel::new(internal_rate, controls),
            decimator: Decimator::new(substeps, rate, LATENCY as usize),
            active: 0,
            held: 0,
            audition: None,
            substeps,
            internal_rate,
            release_ticks: (0.80 * internal_rate).ceil() as usize,
            choke_ticks: (0.004 * internal_rate).ceil() as usize,
            panel_tail: 0,
        }
    }

    pub fn set_pickup_distance_mm(&mut self, centre: f64, bridge: f64, reed: f64) {
        let next = [centre, bridge, reed];
        if self.distance_mm == next || !next.iter().all(|v| v.is_finite()) {
            return;
        }
        let mut keys = self.active;
        while keys != 0 {
            let k = &mut self.keys[keys.trailing_zeros() as usize];
            keys &= keys - 1;
            k.action.string.set_pickup_distance_mm(centre, bridge);
            if self.distance_mm[2] != reed {
                k.reed_offset = reed;
                k.reed.set_pickup_distance_mm(reed as f32);
                k.flux = k.reed.current_flux();
            }
        }
        self.distance_mm = next;
    }

    pub fn set_pad_release_noise(&mut self, amount: f64) {
        let amount = amount.clamp(0., 1.);
        if amount.is_finite() && self.release_noise != amount {
            for k in &mut self.keys {
                k.reed.set_pad_release(amount as f32);
                k.flux = k.reed.current_flux();
            }
            self.release_noise = amount;
        }
    }

    /// Optional pedal modification for the Pianet half. The tangent always
    /// follows the physical key, including when the pedal holds a plucked reed.
    pub fn set_sustain_mod(&mut self, enabled: bool, pedal_down: bool) {
        let catch = enabled && pedal_down;
        if self.sustain_pad && !catch {
            let mut active = self.active;
            while active != 0 {
                let index = active.trailing_zeros() as usize;
                active &= active - 1;
                let key = &mut self.keys[index];
                if key.reed_held {
                    key.reed.release_sustain();
                    key.reed_held = false;
                    key.remaining = self.release_ticks;
                }
            }
        }
        self.sustain_pad = catch;
    }

    pub fn set_controls(&mut self, controls: Controls) {
        if self.controls == controls {
            return;
        }
        if self.controls.mute != controls.mute {
            for k in &mut self.keys {
                k.action.string.mute(f64::from(controls.mute));
            }
        }
        self.panel.set(controls);
        self.controls = controls;
    }
    pub fn reset(&mut self) {
        let mut active = self.active;
        while active != 0 {
            self.keys[active.trailing_zeros() as usize].clear();
            active &= active - 1;
        }
        self.active = 0;
        self.held = 0;
        self.audition = None;
        self.sustain_pad = false;
        self.panel_tail = 0;
        self.panel.reset();
        self.decimator.reset();
    }
    pub fn sounding(&self) -> bool {
        self.active != 0 || self.panel_tail != 0
    }
    pub fn pressed(&self) -> u128 {
        (self.held << self.first) | self.audition.map_or(0, |n| 1u128 << n)
    }
    pub fn caught_pads(&self) -> u128 {
        if !self.sustain_pad {
            return 0;
        }
        let mut active = self.active;
        let mut mask = 0;
        while active != 0 {
            let i = active.trailing_zeros() as usize;
            active &= active - 1;
            if self.keys[i].reed_held {
                mask |= 1u128 << i;
            }
        }
        mask << self.first
    }
    fn strike(&mut self, n: u8, velocity: f32) {
        let i = usize::from(n - self.first);
        let k = &mut self.keys[i];
        if self.active & (1 << i) == 0 {
            let [centre, bridge, reed] = self.distance_mm;
            k.action.string.set_pickup_distance_mm(centre, bridge);
            if k.reed_offset != reed {
                k.reed.set_pickup_distance_mm(reed as f32);
                k.flux = k.reed.current_flux();
                k.reed_offset = reed;
            }
        }
        self.active |= 1 << i;
        k.strike((velocity.clamp(0., 1.) * 127.).round().max(1.) as u8);
    }
    pub fn note_on(&mut self, n: u8, velocity: f32) {
        if !(self.first..=self.last).contains(&n) {
            return;
        }
        if velocity <= 0. {
            self.note_off(n);
            return;
        }
        self.held |= 1 << (n - self.first);
        self.strike(n, velocity);
    }
    pub fn note_off(&mut self, n: u8) {
        if !(self.first..=self.last).contains(&n) {
            return;
        }
        let bit = 1 << (n - self.first);
        if self.held & bit == 0 {
            return;
        }
        self.held &= !bit;
        if self.audition != Some(n) {
            self.keys[usize::from(n - self.first)].lift(self.release_ticks, self.sustain_pad);
        }
    }
    pub fn audition(&mut self, note: Option<u8>, velocity: f32) {
        let next = note.filter(|n| (self.first..=self.last).contains(n) && velocity > 0.);
        if next == self.audition {
            return;
        }
        if let Some(n) = self.audition.take() {
            if self.held & (1 << (n - self.first)) == 0 {
                self.keys[usize::from(n - self.first)].lift(self.release_ticks, self.sustain_pad);
            }
        }
        self.audition = next;
        if let Some(n) = next {
            self.strike(n, velocity);
        }
    }
    pub fn choke(&mut self, n: u8) {
        if !(self.first..=self.last).contains(&n) {
            return;
        }
        let bit = 1 << (n - self.first);
        if self.active & bit == 0 {
            return;
        }
        if self.audition == Some(n) {
            self.audition = None;
        }
        self.held &= !bit;
        let k = &mut self.keys[usize::from(n - self.first)];
        k.lift(self.choke_ticks, false);
        k.choke = self.choke_ticks;
    }

    fn tick(&mut self) -> [f64; 3] {
        let mut halves = [Section::default(); 2];
        let mut active = self.active;
        while active != 0 {
            let i = active.trailing_zeros() as usize;
            active &= active - 1;
            let key = &mut self.keys[i];
            let (s, r) = key.tick(self.internal_rate);
            let midi = self.first + i as u8;
            let half = &mut halves[usize::from(midi > LAST_BASS)];
            let gain = if key.choke > 0 {
                let v = key.choke as f64 / self.choke_ticks as f64;
                key.choke -= 1;
                v
            } else {
                1.
            };
            half.string.centre += gain * s.centre;
            half.string.bridge += gain * s.bridge;
            // Successive reed coils alternate winding direction in the series chain.
            half.reed += if midi % 2 == LOW % 2 {
                gain * r
            } else {
                -gain * r
            };
            if key.remaining != 0 {
                key.remaining -= 1;
                if key.remaining == 0 {
                    key.clear();
                    self.active &= !(1 << i);
                }
            }
        }
        let [lo, hi] = halves;
        let (strings, reeds) = self.controls.split.banks();
        let s = match strings {
            Bank::Bass => lo.string,
            Bank::Treble => hi.string,
            Bank::Both => Pickups {
                centre: lo.string.centre + hi.string.centre,
                bridge: lo.string.bridge + hi.string.bridge,
            },
        };
        let r = match reeds {
            Bank::Bass => lo.reed,
            Bank::Treble => hi.reed,
            Bank::Both => lo.reed + hi.reed,
        };
        self.panel
            .tick(s.centre * PICKUP_VOLTS, s.bridge * PICKUP_VOLTS, r)
    }
    /// The matched recording branches (Clavinet, Pianet) and the active mono mixer, volts.
    pub fn jacks(&mut self) -> [f64; 3] {
        if !self.sounding() {
            return [0.; 3];
        }
        if self.active != 0 {
            self.panel_tail = (self.internal_rate / self.substeps as f64 * 1.5) as usize;
        } else {
            self.panel_tail -= 1;
        }
        let mut out = [0.; 3];
        for step in 0..self.substeps {
            let sample = self.tick();
            self.decimator.push(sample);
            if step == 0 {
                out = self.decimator.read();
            }
        }
        out
    }
    pub fn render(&mut self, out: &mut [f32]) {
        for x in out {
            *x = self.jacks()[2] as f32;
        }
    }
}

#[cfg(test)]
mod sustain_mod_tests {
    use super::*;

    #[test]
    fn pedal_latches_only_the_reed_and_submits_the_real_return_on_pedal_up() {
        let mut stock = Engine::new(48000.);
        let mut modified = Engine::new(48000.);
        let n = 64;
        let index = usize::from(n - LOW);
        stock.set_sustain_mod(false, true);
        modified.set_sustain_mod(true, true);
        for e in [&mut stock, &mut modified] {
            e.note_on(n, 0.8);
        }
        for _ in 0..7200 {
            assert_eq!(stock.jacks(), modified.jacks());
        }
        stock.note_off(n);
        modified.note_off(n);
        assert_eq!(modified.pressed(), 0);
        assert!(modified.keys[index].reed_held);
        assert_eq!(modified.caught_pads(), 1u128 << n);
        let mut energy = [0.; 2];
        for i in 0..46000 {
            let a = stock.jacks();
            let b = modified.jacks();
            if i < 6000 {
                assert_eq!(
                    stock.keys[index].action.string.displacements(),
                    modified.keys[index].action.string.displacements()
                );
            }
            if i >= 20000 {
                energy[0] += a[1] * a[1];
                energy[1] += b[1] * b[1];
            }
        }
        assert!(modified.keys[index].string_parked);
        assert!(energy[1] > energy[0] * 5. && energy[1] > 1e-8);
        modified.set_sustain_mod(true, false);
        assert_eq!(modified.caught_pads(), 0);
        assert!(!modified.keys[index].reed_held);
        for _ in 0..40000 {
            modified.jacks();
        }
        assert_eq!(modified.active, 0);
    }

    #[test]
    fn toggling_mod_off_and_releasing_ui_notes_does_not_capture_old_releases() {
        let mut e = Engine::extended(44100.);
        e.set_sustain_mod(true, true);
        e.audition(Some(90), 0.8);
        for _ in 0..5000 {
            e.jacks();
        }
        e.audition(None, 0.);
        assert_eq!(e.pressed(), 0);
        assert!(e.keys[90].reed_held);
        e.set_sustain_mod(false, true);
        assert!(!e.keys[90].reed_held);
        let tail = e.keys[90].remaining;
        e.set_sustain_mod(true, true);
        assert_eq!(e.keys[90].remaining, tail);
        assert!(!e.keys[90].reed_held);
        e.choke(90);
        for _ in 0..400 {
            e.jacks();
        }
        assert_eq!(e.active, 0);
        e.reset();
        assert!(!e.sounding() && !e.sustain_pad);
    }
}
