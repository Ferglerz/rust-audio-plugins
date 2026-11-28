//! The series pickup terminals before the Siemens amplifier.
use crate::action::{Action, HIGH, LOW};
use crate::pickup::{head, Pickups, HEADS};

struct Voice {
    action: Action,
    group: usize,
    displacement: f64,
    choke: u32,
}

#[derive(Clone, Copy, Default)]
pub struct Terminals {
    pub head_emf_v: [f64; HEADS],
    pub input_v: f64,
}

pub struct Keyboard {
    voices: Vec<Voice>,
    pickups: Pickups,
    active: u128,
    held: u128,
    audition: Option<u8>,
    extended: bool,
    sustain: bool,
    felt: bool,
    phase: u8,
    fade: u32,
}

impl Keyboard {
    pub fn new(rate: f64) -> Self {
        assert!(rate.is_finite() && rate >= 8000.);
        Self {
            voices: (0..=127)
                .map(|n| Voice {
                    action: Action::extended(n, rate).unwrap(),
                    group: head(n),
                    displacement: 0.,
                    choke: 0,
                })
                .collect(),
            pickups: Pickups::new(rate),
            active: 0,
            held: 0,
            audition: None,
            extended: false,
            sustain: false,
            felt: false,
            phase: 0,
            fade: (rate * 0.004).ceil() as u32,
        }
    }

    fn accepts(&self, n: u8) -> bool {
        n < 128 && (self.extended || (LOW..=HIGH).contains(&n))
    }

    pub fn set_extended(&mut self, enabled: bool) {
        if self.extended && !enabled {
            for n in (0..LOW).chain(HIGH + 1..=127) {
                self.choke(n);
            }
        }
        self.extended = enabled;
    }

    pub fn set_pickup_gap_mm(&mut self, offset: f64) {
        if self.pickups.set_distance_mm(offset) {
            let mut motion = [0.; HEADS];
            for v in &self.voices {
                motion[v.group] += self.pickups.motion(v.displacement, v.action.strings);
            }
            self.pickups.rebase(motion);
        }
    }

    fn strike(&mut self, n: u8, v: f32) {
        let key = &mut self.voices[usize::from(n)];
        key.action.strike(v);
        key.choke = 0;
        self.active |= 1u128 << n;
    }
    pub fn note_on(&mut self, n: u8, velocity: f32) {
        if velocity <= 0. {
            self.note_off(n);
            return;
        }
        if self.accepts(n) && velocity.is_finite() {
            self.held |= 1u128 << n;
            self.strike(n, velocity);
        }
    }
    pub fn note_off(&mut self, n: u8) {
        if n < 128 {
            self.held &= !(1u128 << n);
            if self.audition != Some(n) {
                self.voices[usize::from(n)].action.release();
            }
        }
    }
    pub fn audition(&mut self, n: u8, velocity: f32) {
        self.audition_off();
        if self.accepts(n) && velocity.is_finite() && velocity > 0. {
            self.audition = Some(n);
            self.strike(n, velocity);
        }
    }
    pub fn audition_off(&mut self) {
        if let Some(n) = self.audition.take() {
            if self.held & (1u128 << n) == 0 {
                self.voices[usize::from(n)].action.release();
            }
        }
    }
    pub fn choke(&mut self, n: u8) {
        if n > 127 {
            return;
        }
        let bit = 1u128 << n;
        self.held &= !bit;
        if self.audition == Some(n) {
            self.audition = None;
        }
        if self.active & bit != 0 {
            let key = &mut self.voices[usize::from(n)];
            key.action.release();
            if key.choke == 0 {
                key.choke = self.fade;
            }
        }
    }

    pub fn held_notes(&self) -> u128 {
        self.held | self.audition.map_or(0, |n| 1u128 << n)
    }
    pub fn sounding(&self) -> bool {
        self.active != 0
    }
    pub(super) fn clear_pickup_tail(&mut self) {
        self.pickups.reset();
    }
    pub fn set_sustain(&mut self, down: bool) {
        if self.sustain != down {
            self.sustain = down;
            for v in &mut self.voices {
                v.action.sustain(down);
            }
        }
    }
    pub fn set_felt(&mut self, down: bool) {
        if self.felt != down {
            self.felt = down;
            for v in &mut self.voices {
                v.action.felt(down);
            }
        }
    }
    pub fn reset(&mut self) {
        self.set_sustain(false);
        self.set_felt(false);
        for v in &mut self.voices {
            v.action.reset();
            v.choke = 0;
            v.displacement = 0.;
        }
        self.active = 0;
        self.held = 0;
        self.audition = None;
        self.phase = 0;
        self.pickups.reset();
    }
    pub fn sample(&mut self) -> Terminals {
        let mut motion = [0.; HEADS];
        let mut bits = self.active;
        let held = self.held_notes();
        while bits != 0 {
            let n = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            let bit = 1u128 << n;
            let v = &mut self.voices[n];
            let mut x = v.action.sample();
            if v.choke > 0 {
                v.choke -= 1;
                x *= f64::from(v.choke) / f64::from(self.fade);
                if v.choke == 0 {
                    v.action.reset();
                    self.active &= !bit;
                }
            } else if self.phase == 0 && held & bit == 0 && v.action.quiet() {
                self.active &= !bit;
                v.action.reset();
                x = 0.;
            }
            v.displacement = x;
            motion[v.group] += self.pickups.motion(x, v.action.strings);
        }
        self.phase = self.phase.wrapping_add(1);
        let (head_emf_v, input_v) = self.pickups.tick(motion);
        Terminals {
            head_emf_v,
            input_v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn covers_all_heads_and_range_changes_preserve_the_native_held_key() {
        let mut k = Keyboard::new(44100.);
        for n in [0, 127] {
            k.note_on(n, 0.8);
        }
        assert_eq!(k.held_notes(), 0);
        k.set_extended(true);
        for n in [0, 21, 127] {
            k.note_on(n, 0.8);
        }
        k.audition(44, 0.8);
        k.note_off(44);
        assert_eq!(k.held_notes(), 1 | 1u128 << 21 | 1u128 << 44 | 1u128 << 127);
        k.audition_off();
        k.set_extended(false);
        assert_eq!(k.held_notes(), 1u128 << 21);
        assert_eq!(head(100), 15);
        assert_eq!(head(101), 16);
        assert_eq!(head(104), 16);
        assert_eq!(head(105), 17);
        assert_eq!(head(127), 17);
    }
    fn energy(felt: bool, sustain: bool, offset: f64) -> [f64; 3] {
        let mut k = Keyboard::new(44100.);
        k.set_felt(felt);
        k.set_sustain(sustain);
        k.set_pickup_gap_mm(offset);
        k.note_on(64, 0.72);
        let mut energies = [0.; 3];
        for i in 0..24000 {
            if i == 12500 {
                k.note_off(64);
            }
            let frame = k.sample();
            for (g, x) in frame.head_emf_v.iter().enumerate() {
                if g != head(64) {
                    assert_eq!(*x, 0.);
                }
            }
            let v = frame.input_v;
            assert!(v.is_finite());
            if i < 5000 {
                energies[0] += v * v;
            } else if (10000..12500).contains(&i) {
                energies[1] += v * v;
            } else if i >= 20000 {
                energies[2] += v * v;
            }
        }
        energies
    }
    #[test]
    fn held_felt_absorbs_energy_and_sustain_lifts_the_separate_key_damper() {
        let ordinary = energy(false, false, 0.);
        let felt = energy(true, false, 0.);
        let sustained = energy(false, true, 0.);
        let farther = energy(false, false, 1.);
        assert!(ordinary[0] > 1e-7);
        // At E4 the soft hammer predominantly excites the fundamental. After
        // 0.25 s, loss at x/L=0.13 predicts an energy ratio near 1.67.
        assert!(
            ordinary[1] > 1.45 * felt[1],
            "held felt ratio {}",
            ordinary[1] / felt[1]
        );
        assert_eq!(ordinary[0], sustained[0]);
        assert!(sustained[2] > 100. * ordinary[2]);
        assert!(farther[0] < ordinary[0] * 0.8);
    }
}
