//! Columbia Elepian EP-61C, with its original passive control circuit.
mod action;
mod panel;
pub use action::{Key, Observation};
pub use panel::{Controls, Panel};

pub const LOW: u8 = 36;
pub const HIGH: u8 = 96;
pub const LATENCY: u32 = 8;
pub type Engine = crate::keyboard::Keyboard<Key, Panel>;

impl Engine {
    pub fn new(host_rate: f64) -> Self {
        Self::with_load(host_rate, 1e6)
    }
    pub fn with_load(host_rate: f64, receiver_ohms: f64) -> Self {
        Self::from_parts(LOW..=HIGH, true, LATENCY, host_rate, Key::new, |rate| {
            Panel::new(rate, receiver_ohms)
        })
    }
    pub fn set_controls(&mut self, c: Controls) {
        let alignment = c.alignment_mm.clamp(-1., 1.);
        if alignment != self.panel.controls.alignment_mm {
            for key in self.keys_mut() {
                key.set_lateral_offset_mm(alignment as f32);
            }
        }
        self.panel.set(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy(y: &[f32]) -> f64 {
        y.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() / y.len() as f64
    }

    #[test]
    fn hammer_escapes_before_impact_and_backcheck_catches_the_rebound() {
        use crate::keyboard::Action;
        for n in [36, 64, 96] {
            let mut key = Key::new(n, 96000.);
            let mut first = [0usize; 2];
            let mut peak = [0.; 2];
            for (i, v) in [0.25, 0.90].into_iter().enumerate() {
                key.reset();
                key.press(v);
                let mut caught = false;
                for t in 1..9000 {
                    let o = key.step(0.);
                    if o.hammer_force_n > 0. {
                        assert!(o.escaped);
                        if first[i] == 0 {
                            first[i] = t;
                        }
                        peak[i] = f64::max(peak[i], o.hammer_force_n);
                    }
                    caught |= o.caught;
                }
                assert!(caught);
            }
            assert!(first[1] < first[0] && peak[1] > peak[0]);
        }
    }

    #[test]
    fn damper_pedal_and_partial_return_restrike_preserve_motion() {
        let mut e = Engine::new(48000.);
        e.note_on(60, 0.7);
        e.render(&mut [0.; 4000]);
        e.set_sustain(true);
        e.note_off(60);
        let mut a = [0.; 4000];
        e.render(&mut a);
        let held = energy(&a[2000..]);
        assert!(held > 1e-6);
        let old = e.observation(60).unwrap().reed_tip_m;
        e.note_on(60, 0.6);
        assert_eq!(old, e.observation(60).unwrap().reed_tip_m);
        e.note_off(60);
        e.render(&mut [0.; 120]);
        e.note_on(60, 0.8);
        e.render(&mut a);
        assert!(energy(&a) > 1e-7);
        e.note_off(60);
        e.set_sustain(false);
        e.render(&mut a);
        e.render(&mut a);
        assert!(energy(&a) < held * 0.002);
    }

    #[test]
    fn extended_pitch_receiver_clearance_and_native_gate_are_independent() {
        crate::keyboard::check_register(Engine::new);
    }
}
