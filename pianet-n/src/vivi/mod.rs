//! Vivi-Tone Clavier, National Music Museum C 1004 functional reconstruction.
mod action;
mod panel;
pub use action::{Key, Observation};
pub use panel::{Controls, Panel};
pub const LOW: u8 = 24;
pub const HIGH: u8 = 96;
pub const LATENCY: u32 = 8;
/// Estimated extent of the separately lifted bass damper rail.
pub const BASS_HIGH: u8 = 47;
pub type Engine = crate::keyboard::Keyboard<Key, Panel>;
impl Engine {
    pub fn new(host_rate: f64) -> Self {
        Self::with_load(host_rate, 1e6)
    }
    pub fn with_load(host_rate: f64, load: f64) -> Self {
        Self::from_parts(LOW..=HIGH, true, LATENCY, host_rate, Key::new, |rate| {
            Panel::new(rate, load)
        })
    }
    pub fn set_bass_sustain(&mut self, down: bool) {
        self.set_bass_rail(down, BASS_HIGH);
    }
    pub fn set_controls(&mut self, c: Controls) {
        self.panel.set(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard::Action;
    #[test]
    fn picker_slips_then_rearms_after_one_third_return() {
        for n in [24, 64, 96] {
            let mut peaks = [0.; 2];
            for (i, v) in [0.24, 0.89].into_iter().enumerate() {
                let mut k = Key::new(n, 96000.);
                k.press(v);
                let (mut contact, mut escaped) = (false, false);
                for _ in 0..5600 {
                    let o = k.step(0.);
                    contact |= o.pick_force_n > 0.;
                    escaped |= !o.armed;
                    peaks[i] = f64::max(peaks[i], o.armature_m.abs());
                }
                assert!(contact && escaped);
                k.release();
                let mut rearmed = None;
                for _ in 0..480 {
                    let o = k.step(0.);
                    assert_eq!(o.pick_force_n, 0.);
                    if o.armed {
                        rearmed.get_or_insert(o.key_fraction);
                    }
                }
                assert!((0.635..=0.64).contains(&rearmed.unwrap()));
                let y = k.observation().armature_m;
                k.press(v);
                assert_eq!(y, k.observation().armature_m);
                assert!((0..4000).any(|_| k.step(0.).pick_force_n > 0.));
            }
            assert!(peaks[1] > peaks[0] * 1.15, "note {n}, {peaks:?}");
        }
    }
    #[test]
    fn full_and_bass_dampers_overlap_and_hold_later_bass_keys() {
        let energy = |note, bass, full, release_full| {
            let mut e = Engine::new(48000.);
            e.set_bass_sustain(bass);
            e.set_sustain(full);
            e.note_on(note, 0.7);
            e.render(&mut [0.; 5000]);
            e.note_off(note);
            if release_full {
                e.set_sustain(false);
            }
            e.render(&mut [0.; 5000]);
            let mut a = [0.; 4800];
            e.render(&mut a);
            a.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>()
        };
        for n in [BASS_HIGH, BASS_HIGH + 1] {
            let open = energy(n, false, true, false);
            let closed = energy(n, false, false, false);
            assert!(open > 100. * closed && open > 1e-8);
            let bass = energy(n, true, true, true);
            if n == BASS_HIGH {
                assert!((bass / open - 1.).abs() < 0.001);
            } else {
                assert!(bass < open * 0.01);
            }
        }
        let mut e = Engine::new(44100.);
        e.audition(Some(36), 0.6);
        e.note_on(36, 0.7);
        e.audition(None, 0.);
        assert_eq!(e.pressed(), 1u128 << 36);
        e.set_bass_sustain(true);
        e.note_off(36);
        e.choke(36);
        e.render(&mut [0.; 18000]);
        assert!(!e.sounding());
    }
    #[test]
    fn register_and_clearance() {
        crate::keyboard::check_register(Engine::new);
    }
}
