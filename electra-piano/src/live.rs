//! F1–E7 Hohner console, 680209 auxiliary output and speaker terminals.
use crate::circuit::{group, Controls, Output, Panel};
use crate::fork::{Fork, ForkSpec, KEYS};

pub const LOW: u8 = 29;
pub const HIGH: u8 = 100;

pub fn scale(note: u8) -> Option<ForkSpec> {
    if !(LOW..=HIGH).contains(&note) {
        return None;
    }
    if let Some(spec) = KEYS.iter().find(|s| s.note == note) {
        return Some(*spec);
    }
    let [lo, hi] = if note < KEYS[1].note {
        [KEYS[0], KEYS[1]]
    } else {
        [KEYS[1], KEYS[2]]
    };
    // Positive physical quantities interpolate as power laws of frequency.
    // Beyond the outer anchors the same slopes are provisional extrapolations.
    let t = (f64::from(note) - f64::from(lo.note)) / f64::from(hi.note - lo.note);
    let fit = |a: f64, b: f64| a * (b / a).powf(t);
    Some(ForkSpec {
        note,
        tip_mass: fit(lo.tip_mass, hi.tip_mass),
        return_mass: fit(lo.return_mass, hi.return_mass),
        return_ratio: fit(lo.return_ratio, hi.return_ratio),
        coupling: fit(lo.coupling, hi.coupling),
        hammer_mass: fit(lo.hammer_mass, hi.hammer_mass),
        contact_k: fit(lo.contact_k, hi.contact_k),
        field_width: fit(lo.field_width, hi.field_width),
        magnet_polarity: 1.,
        ring_t60: fit(lo.ring_t60, hi.ring_t60),
        damped_t60: fit(lo.damped_t60, hi.damped_t60),
    })
}

pub fn hammer_speed(velocity: f32) -> f64 {
    // A reversible controller convention, since the source has no MIDI data.
    1.7 * f64::from(velocity.clamp(0., 1.))
}

struct Voice {
    fork: Fork,
    group: usize,
    choke: u32,
}

pub struct Engine {
    voices: Vec<Voice>,
    panel: Panel,
    active: u128,
    held: u128,
    extended: bool,
    pickup_gap_mm: f64,
    audition: Option<u8>,
    sustain: bool,
    tail: bool,
    phase: u8,
    fade_frames: u32,
}

impl Engine {
    /// Allocate all MIDI actions at initialization, with the native range enabled.
    pub fn new(rate: f64) -> Self {
        let voices = (0_u8..=127)
            .map(|n| {
                let nearest = n.clamp(LOW, HIGH);
                let mut spec = scale(nearest).unwrap();
                spec.note = n;
                Voice {
                    fork: Fork::new(spec, rate),
                    group: group(nearest).unwrap(),
                    choke: 0,
                }
            })
            .collect();
        Self {
            voices,
            panel: Panel::new(rate, Controls::default()),
            active: 0,
            held: 0,
            extended: false,
            pickup_gap_mm: 0.,
            audition: None,
            sustain: false,
            tail: false,
            phase: 0,
            fade_frames: (0.004 * rate).ceil() as u32,
        }
    }

    fn accepts(&self, note: u8) -> bool {
        note <= 127 && (self.extended || (LOW..=HIGH).contains(&note))
    }

    /// The factory compass is active by default. Retire only newly disabled keys.
    pub fn set_extended(&mut self, enabled: bool) {
        if self.extended && !enabled {
            for n in (0..LOW).chain(HIGH + 1..=127) {
                if (self.active | self.held_notes()) & (1u128 << n) != 0 {
                    self.choke(n);
                }
            }
        }
        self.extended = enabled;
    }

    /// Physical pickup setback in millimeters. Zero preserves native regulation.
    pub fn set_pickup_gap_mm(&mut self, offset: f64) {
        if !offset.is_finite() {
            return;
        }
        let offset = offset.clamp(-0.25, 2.);
        if self.pickup_gap_mm != offset {
            self.pickup_gap_mm = offset;
            let mut keys = self.active;
            while keys != 0 {
                let voice = &mut self.voices[keys.trailing_zeros() as usize];
                keys &= keys - 1;
                voice.fork.set_pickup_gap_mm(offset);
            }
        }
    }

    pub fn reset(&mut self) {
        for v in &mut self.voices {
            v.fork.clear();
            v.choke = 0;
        }
        self.active = 0;
        self.held = 0;
        self.audition = None;
        self.sustain = false;
        self.tail = false;
        self.phase = 0;
        self.panel.reset();
    }

    pub fn set_controls(&mut self, controls: Controls) {
        self.panel.set_controls(controls);
    }
    pub fn set_sustain(&mut self, down: bool) {
        if self.sustain != down {
            self.sustain = down;
            for v in &mut self.voices {
                v.fork.sustain(down);
            }
        }
    }

    fn strike(&mut self, note: u8, velocity: f32) {
        let v = &mut self.voices[usize::from(note)];
        v.choke = 0;
        v.fork.set_pickup_gap_mm(self.pickup_gap_mm);
        v.fork.sustain(self.sustain);
        v.fork.strike(hammer_speed(velocity));
        self.active |= 1u128 << note;
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) {
        if !self.accepts(note) || !velocity.is_finite() {
            return;
        }
        if velocity <= 0. {
            self.note_off(note);
            return;
        }
        self.held |= 1u128 << note;
        self.strike(note, velocity);
    }

    pub fn note_off(&mut self, note: u8) {
        if note > 127 {
            return;
        }
        self.held &= !(1u128 << note);
        if self.audition != Some(note) {
            self.voices[usize::from(note)].fork.release();
        }
    }

    pub fn audition(&mut self, note: u8, velocity: f32) {
        if !self.accepts(note) || !(velocity.is_finite() && velocity > 0.) {
            return;
        }
        self.audition_off();
        self.audition = Some(note);
        self.strike(note, velocity);
    }

    pub fn audition_off(&mut self) {
        if let Some(n) = self.audition.take() {
            if self.held & (1u128 << n) == 0 {
                self.voices[usize::from(n)].fork.release();
            }
        }
    }

    pub fn choke(&mut self, note: u8) {
        if note > 127 {
            return;
        }
        if self.audition == Some(note) {
            self.audition = None;
        }
        self.note_off(note);
        self.voices[usize::from(note)].choke = self.fade_frames;
    }

    pub fn held_notes(&self) -> u128 {
        self.held | self.audition.map_or(0, |n| 1u128 << n)
    }

    pub fn sounding(&self) -> bool {
        self.active != 0 || self.tail
    }

    /// Advance one sample and observe both electrical terminals without
    /// normalization, clipping, DC blocking or an extra call to the engine.
    pub fn next_outputs(&mut self) -> Output {
        if !self.sounding() {
            return Output::default();
        }
        let mut groups = [0.; 7];
        let mut mask = self.active;
        self.phase = self.phase.wrapping_add(1);
        while mask != 0 {
            let n = mask.trailing_zeros();
            mask &= mask - 1;
            let voice = &mut self.voices[n as usize];
            let emf = voice.fork.tick().pickup_v;
            if voice.choke != 0 {
                groups[voice.group] += emf * f64::from(voice.choke) / f64::from(self.fade_frames);
                voice.choke -= 1;
                if voice.choke != 0 {
                    continue;
                }
            } else {
                groups[voice.group] += emf;
                if self.phase != 0 || !voice.fork.at_rest() {
                    continue;
                }
            }
            voice.fork.clear();
            self.active &= !(1u128 << n);
        }
        let out = self.panel.tick(groups);
        self.tail = self.active != 0 || self.phase != 0 || !self.panel.at_rest();
        if !self.tail {
            self.panel.reset();
        }
        out
    }

    pub fn render(&mut self, out: &mut [f32]) {
        for frame in out {
            *frame = self.next_outputs().auxiliary_v as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fork::KEYS;

    #[test]
    fn anchor_transfers_match_the_offline_forks_at_three_rates() {
        for rate in [44100., 48000., 96000.] {
            let mut engine = Engine::new(rate);
            for k in KEYS {
                engine.reset();
                let mut fork = Fork::new(k, rate);
                let mut panel = Panel::new(rate, Controls::default());
                fork.strike(0.85);
                engine.note_on(k.note, 0.5);
                for i in 0..10000 {
                    if i == 3000 {
                        fork.sustain(true);
                        engine.set_sustain(true);
                    }
                    if i == 4000 {
                        fork.release();
                        engine.note_off(k.note);
                    }
                    if i == 5000 {
                        let p = Controls {
                            soft_pedal: true,
                            ..Default::default()
                        };
                        panel.set_controls(p);
                        engine.set_controls(p);
                    }
                    if i == 6500 {
                        fork.sustain(false);
                        engine.set_sustain(false);
                    }
                    let mut groups = [0.; 7];
                    groups[group(k.note).unwrap()] = fork.tick().pickup_v;
                    let expected = panel.tick(groups);
                    let actual = engine.next_outputs();
                    assert_eq!(actual.auxiliary_v, expected.auxiliary_v);
                    assert_eq!(actual.speaker_v, expected.speaker_v);
                }
                engine.set_controls(Controls::default());
            }
        }
    }

    #[test]
    fn each_key_and_group_boundary_produces_both_physical_outputs() {
        for rate in [8000., 44100., 48000., 96000.] {
            let mut engine = Engine::new(rate);
            for note in LOW..=HIGH {
                engine.reset();
                engine.note_on(note, 1.);
                let mut peak = [0_f64; 2];
                for _ in 0..(rate * 0.025) as usize {
                    let x = engine.next_outputs();
                    assert!(x.auxiliary_v.is_finite() && x.speaker_v.is_finite());
                    peak[0] = peak[0].max(x.auxiliary_v.abs());
                    peak[1] = peak[1].max(x.speaker_v.abs());
                }
                assert!(
                    peak[0] > 1e-5 && peak[1] > 1e-4,
                    "missing output on {note} at {rate}"
                );
            }
            engine.reset();
            engine.note_on(LOW - 1, 1.);
            engine.note_on(HIGH + 1, 1.);
            engine.note_on(65, f32::NAN);
            assert_eq!(engine.held_notes(), 0);
            assert_eq!(engine.next_outputs().auxiliary_v, 0.);
        }
    }

    #[test]
    fn host_and_audition_share_the_action_and_release_their_own_keys() {
        let mut e = Engine::new(48000.);
        e.note_on(53, 0.3);
        e.audition(53, 0.7);
        e.note_off(53);
        assert_eq!(e.held_notes(), 1u128 << 53);
        e.audition(54, 0.8);
        e.note_on(54, 0.5);
        e.audition_off();
        assert_eq!(e.held_notes(), 1u128 << 54);
        e.set_sustain(true);
        e.note_off(54);
        assert_eq!(e.held_notes(), 0);
        assert!(e.sounding());
        let mut x = [0_f32; 128];
        e.render(&mut x);
        assert!(x.iter().any(|v| v.abs() > 1e-5));
        e.choke(53);
        e.choke(54);
        e.set_sustain(false);
        for _ in 0..3000 {
            e.render(&mut x);
            if !e.sounding() {
                break;
            }
        }
        assert!(!e.sounding(), "post-choke electrical tail still active");
        e.note_on(54, 0.7);
        e.render(&mut x);
        e.reset();
        e.render(&mut x);
        assert!(x.iter().all(|v| *v == 0.));
    }
}

#[cfg(test)]
mod extended_tests {
    use super::*;

    #[test]
    fn full_midi_forks_tune_to_each_event_and_disabling_keeps_native_notes() {
        let mut engine = Engine::new(48000.);
        engine.note_on(0, 0.5);
        engine.audition(127, 0.5);
        assert_eq!(engine.held_notes(), 0);
        engine.set_extended(true);
        for n in [0u8, 127] {
            engine.reset();
            engine.note_on(n, 0.75);
            let f = 440. * 2_f64.powf((f64::from(n) - 69.) / 12.);
            let mut bins = [(0_f64, 0_f64); 3];
            for i in 0..48000 {
                let y = engine.next_outputs().auxiliary_v;
                let window = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / 48000.).cos();
                for ((a, b), ratio) in bins.iter_mut().zip([0.90, 1., 1.10]) {
                    let (sin, cos) =
                        (std::f64::consts::TAU * f * ratio * i as f64 / 48000.).sin_cos();
                    *a += y * window * sin;
                    *b += y * window * cos;
                }
            }
            let mag = bins.map(|(a, b)| a.hypot(b));
            assert!(mag[1] > mag[0] && mag[1] > mag[2], "note {n}, {mag:?}");
        }
        engine.note_on(40, 0.5);
        engine.audition(0, 0.6);
        engine.set_sustain(true);
        engine.set_extended(false);
        assert_eq!(engine.held_notes(), 1u128 << 40);
        engine.audition(127, 0.5);
        assert_eq!(engine.held_notes(), 1u128 << 40);
    }

    #[test]
    fn pickup_gap_changes_flux_and_keeps_the_fork_motion_identical() {
        let mut f = Fork::new(KEYS[1], 48000.);
        let mut other = Fork::new(KEYS[1], 48000.);
        other.set_pickup_gap_mm(1.2);
        f.strike(0.85);
        other.strike(0.85);
        let mut power = [0.; 2];
        for _ in 0..4000 {
            let a = f.tick();
            let b = other.tick();
            assert_eq!(a.sensed_m, b.sensed_m);
            assert_eq!(a.contact_n, b.contact_n);
            power[0] += a.pickup_v.powi(2);
            power[1] += b.pickup_v.powi(2);
        }
        assert!(power[1] < power[0] * 0.6);
        other.set_pickup_gap_mm(0.);
        for _ in 0..100 {
            f.tick();
            other.tick();
        }
        assert!((f.tick().pickup_v - other.tick().pickup_v).abs() < 1e-10);
    }
}
