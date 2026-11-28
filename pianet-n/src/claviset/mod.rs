//! Klingenthal cam-plucked reed keyboards. Physical state is shared only across
//! the two reconstruction targets where the source evidence supports it.
mod action;
mod panel;
pub use action::{Key, Observation};
pub use panel::Panel;

pub const LOW: u8 = 36;
pub const HIGH: u8 = 96;
pub const LATENCY: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instrument {
    Claviset200,
    Pianotron,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Register {
    Bass,
    Guitar,
    Tutti,
    Piano,
    Harp,
    Celesta,
    MusicBox,
    Metallophone,
    Xylophone,
}
impl Register {
    pub const ALL: [Self; 9] = [
        Self::Bass,
        Self::Guitar,
        Self::Tutti,
        Self::Piano,
        Self::Harp,
        Self::Celesta,
        Self::MusicBox,
        Self::Metallophone,
        Self::Xylophone,
    ];
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modulation {
    Off,
    Slow,
    Medium,
    Fast,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub power: bool,
    pub register: Register,
    pub modulation: Modulation,
    pub swell: f64,
    pub tone: f64,
    pub depth: f64,
    pub rate: f64,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            power: true,
            register: Register::Piano,
            modulation: Modulation::Off,
            swell: 1.,
            tone: 0.5,
            depth: 0.,
            rate: 0.5,
        }
    }
}

pub type Engine = crate::keyboard::Keyboard<Key, Panel>;

impl Engine {
    pub fn new(instrument: Instrument, host_rate: f64) -> Self {
        Self::from_parts(
            LOW..=HIGH,
            instrument == Instrument::Claviset200,
            LATENCY,
            host_rate,
            Key::extended,
            |rate| Panel::new(instrument, rate),
        )
    }
    pub fn set_controls(&mut self, c: Controls) {
        self.panel.set(Controls {
            swell: c.swell.clamp(0., 1.),
            tone: c.tone.clamp(0., 1.),
            depth: c.depth.clamp(0., 1.),
            rate: c.rate.clamp(0., 1.),
            ..c
        });
    }
}

impl crate::keyboard::Action for Key {
    type Observation = Observation;
    fn reset(&mut self) {
        self.reset()
    }
    fn press(&mut self, velocity: f64) {
        self.press(velocity)
    }
    fn release(&mut self) {
        self.release()
    }
    fn set_pickup_distance_mm(&mut self, offset: f32) {
        self.set_pickup_distance_mm(offset)
    }
    fn observation(&self) -> Observation {
        self.observation
    }
    fn induced_voltage(&mut self, lift: f64) -> f64 {
        self.step(lift).pickup_v
    }
}

impl crate::keyboard::Board for Panel {
    fn reset(&mut self) {
        self.reset()
    }
    fn step(&mut self, emf: f64) -> f64 {
        self.step(emf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn energy(e: &mut Engine, n: usize) -> f64 {
        let mut total = 0.;
        for _ in 0..n {
            let y = f64::from(e.sample());
            assert!(y.is_finite());
            total += y * y;
        }
        total / n as f64
    }
    #[test]
    fn lip_loses_overlap_once_then_bypasses_the_ringing_reed_on_return() {
        let rate = 96_000.;
        for midi in [LOW, 60, 84, HIGH] {
            let mut key = Key::new(midi, rate);
            key.press(0.6);
            let mut loaded = false;
            let mut escaped = false;
            for _ in 0..3000 {
                let o = key.step(0.);
                loaded |= o.lip_force_n > 0.;
                if !o.armed && !escaped {
                    assert!(loaded);
                    assert!((37.9..38.2).contains(&o.cam_angle_rad.to_degrees()));
                    escaped = true;
                }
                if escaped {
                    assert_eq!(o.lip_force_n, 0.);
                }
            }
            assert!(escaped);
            key.move_to(0.35, 0.);
            for _ in 0..3000 {
                let o = key.step(1.);
                assert!(!o.armed && o.lip_force_n == 0.);
            }
            key.release();
            for _ in 0..3000 {
                let o = key.step(1.);
                assert_eq!(o.lip_force_n, 0.);
            }
            assert!(key.observation.armed);
            key.press(0.8);
            let mut plucked = false;
            for _ in 0..3000 {
                plucked |= key.step(1.).lip_force_n > 0.;
            }
            assert!(plucked);
        }
    }
    #[test]
    fn pedal_lifts_felt_on_200_and_sustained_restrike_reuses_the_moving_reed() {
        for instrument in [Instrument::Claviset200, Instrument::Pianotron] {
            let mut e = Engine::new(instrument, 48_000.);
            e.note_on(60, 0.65);
            let held = energy(&mut e, 3000);
            e.set_sustain(true);
            e.note_off(60);
            energy(&mut e, 5000);
            let tail = energy(&mut e, 2000);
            if instrument == Instrument::Claviset200 {
                assert!(tail > 1e-5);
                assert!(e.observation(60).unwrap().armed);
                e.note_on(60, 0.7);
                assert!(e.observation(60).unwrap().reed_tip_m.abs() > 1e-9);
                energy(&mut e, 2900);
                e.note_off(60);
                e.set_sustain(false);
                energy(&mut e, 4500);
                let damped = energy(&mut e, 2000);
                assert!(damped < tail * 0.001, "damped/sustained {}", damped / tail);
            } else {
                let mut direct = Engine::new(instrument, 48_000.);
                direct.note_on(60, 0.65);
                energy(&mut direct, 3000);
                direct.note_off(60);
                energy(&mut direct, 5000);
                assert_eq!(tail, energy(&mut direct, 2000));
                assert!(tail < held * 0.001, "released/held {}", tail / held);
            }
        }
    }
    #[test]
    fn native_pitch_range_pointer_and_midi_ownership_work_at_all_host_rates() {
        for rate in [8000, 44100, 48000, 96000] {
            for instrument in [Instrument::Claviset200, Instrument::Pianotron] {
                let mut e = Engine::new(instrument, f64::from(rate));
                e.note_on(LOW - 1, 1.);
                e.note_on(HIGH + 1, 1.);
                assert_eq!(e.pressed(), 0);
                for n in LOW..=HIGH {
                    e.reset();
                    e.note_on(n, 0.6);
                    assert!(energy(&mut e, rate as usize / 24) > 1e-8, "{n} {rate}");
                }
                e.audition(Some(HIGH), 0.5);
                e.note_off(HIGH);
                assert_eq!(e.pressed(), 1u128 << HIGH);
                e.audition(None, 0.);
                e.choke(HIGH);
                assert_eq!(e.pressed(), 0);
                energy(&mut e, rate as usize);
                assert!(!e.sounding());
            }
        }
    }
    #[test]
    fn extending_cam_keys_preserves_native_action_and_reaches_both_midi_limits() {
        for kind in [Instrument::Claviset200, Instrument::Pianotron] {
            let mut e = Engine::new(kind, 48000.);
            e.set_extended(true);
            for n in [0, 127] {
                e.reset();
                e.note_on(n, 0.7);
                assert_eq!(e.pressed(), 1u128 << n);
                let mut y = vec![0.; if n == 0 { 96000 } else { 8000 }];
                e.render(&mut y);
                let hz = 440. * 2f64.powf((f64::from(n) - 69.) / 12.);
                let project = |frequency: f64| {
                    let (mut a, mut b) = (0., 0.);
                    let tail = &y[2400..];
                    for (i, &x) in tail.iter().enumerate() {
                        let w = (0.5
                            - 0.5
                                * (std::f64::consts::TAU * i as f64 / (tail.len() - 1) as f64)
                                    .cos())
                            * f64::from(x);
                        let (s, c) =
                            (std::f64::consts::TAU * frequency * i as f64 / 48000.).sin_cos();
                        a += w * s;
                        b += w * c;
                    }
                    a.hypot(b)
                };
                assert!(
                    project(hz) > 1e-8
                        && project(hz) > 2. * project(hz * 0.8)
                        && project(hz) > 2. * project(hz * 1.2),
                    "{kind:?} {n}"
                );
                e.set_extended(false);
                e.render(&mut [0.; 800]);
                assert_eq!(e.pressed(), 0);
                e.set_extended(true);
            }
            e.reset();
            e.note_on(64, 0.7);
            let native = energy(&mut e, 3000);
            e.reset();
            e.set_pickup_distance_mm(0.8);
            e.note_on(64, 0.7);
            assert!(energy(&mut e, 3000) < native * 0.75);
        }
    }
    #[test]
    fn resumed_key_after_a_short_off_and_switch_during_hold() {
        let mut e = Engine::new(Instrument::Claviset200, 48000.);
        e.note_on(60, 0.62);
        energy(&mut e, 4000);
        e.note_off(60);
        energy(&mut e, 30);
        e.note_on(60, 0.7);
        let mut rearmed = false;
        let mut escaped = false;
        for _ in 0..2200 {
            e.sample();
            let o = e.observation(60).unwrap();
            rearmed |= o.armed;
            if rearmed && !o.armed {
                escaped = true;
            }
        }
        assert!(escaped);
        let mut p = Controls {
            power: false,
            ..Default::default()
        };
        e.set_controls(p);
        energy(&mut e, 80);
        assert_eq!(energy(&mut e, 500), 0.);
        p.power = true;
        p.register = Register::Xylophone;
        e.set_controls(p);
        assert!(energy(&mut e, 2000) > 1e-6);
    }
}
