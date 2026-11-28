//! Valente production 61-key steel-reed piano.
mod action;
mod panel;
pub use action::{Key, Observation};
pub use panel::{Controls, Harp, Panel};
pub const LOW: u8 = 36;
pub const HIGH: u8 = 96;
pub const LATENCY: u32 = 8;
pub type Engine = crate::keyboard::Keyboard<Key, Harp>;
impl Engine {
    pub fn new(host_rate: f64) -> Self {
        Self::with_load(host_rate, 1e6)
    }
    pub fn with_load(host_rate: f64, receiver_ohms: f64) -> Self {
        Self::from_parts(LOW..=HIGH, true, LATENCY, host_rate, Key::new, |rate| {
            Harp::new(rate, receiver_ohms)
        })
    }
    pub fn set_controls(&mut self, controls: Controls) {
        self.panel.panel.set(controls);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard::Action;
    #[test]
    fn counter_hammer_can_miss_then_strike_with_velocity() {
        for n in [36, 64, 96] {
            let mut k = Key::new(n, 96000.);
            let mut last = 0.;
            for vel in [8, 40, 80, 115] {
                k.reset();
                k.press(f64::from(vel) / 127.);
                let mut force = 0f64;
                let mut launch = 0f64;
                for _ in 0..13000 {
                    let o = k.step(0.);
                    force = force.max(o.hammer_force_n);
                    launch = launch.max(o.launch_mps);
                }
                assert!(launch > 0.);
                if vel == 8 {
                    assert_eq!(force, 0.);
                } else {
                    assert!(force > last);
                }
                last = force;
            }
        }
    }
    #[test]
    fn range_and_pickup_regulation() {
        crate::keyboard::check_register(Engine::new);
    }
    #[test]
    fn pedal_opens_neighbour_reeds_and_restrike_preserves_motion() {
        let mut up = Engine::new(48000.);
        let mut down = Engine::new(48000.);
        down.set_sustain(true);
        for e in [&mut up, &mut down] {
            e.note_on(67, 0.73);
        }
        let mut difference = 0.;
        let mut pressed_energy = 0.;
        for _ in 0..16000 {
            let a = f64::from(up.sample());
            let b = f64::from(down.sample());
            difference += (a - b).powi(2);
            pressed_energy += a * a;
        }
        assert!(difference > 1e-9 * pressed_energy && difference < 0.2 * pressed_energy);
        down.note_off(67);
        let old = down.observation(67).unwrap().reed_tip_m;
        down.note_on(67, 0.74);
        assert_eq!(down.observation(67).unwrap().reed_tip_m, old);
        down.note_off(67);
        up.note_off(67);
        up.render(&mut [0.; 4800]);
        down.render(&mut [0.; 4800]);
        let mut ringing = 0f64;
        let mut stopped = 0f64;
        for _ in 0..4800 {
            ringing += f64::from(down.sample()).powi(2);
            stopped += f64::from(up.sample()).powi(2);
        }
        assert!(ringing > stopped * 50.);
        down.set_sustain(false);
        down.render(&mut [0.; 48000]);
        assert!(!down.sounding());
    }
}
