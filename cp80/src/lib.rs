//! Offline reference mechanics for the 1978 CP-80. Distances are in metres.
mod contact;
pub mod engine;
mod lfo_table;
mod network;
pub mod panel;
pub mod plugin;
mod scale;
pub mod seventy;
pub mod seventy_pickup;
pub mod ui;
use pleasant_dsp::struck_string::{Course, CourseSpec, Lanes, Readout};
#[cfg(test)]
use std::f64::consts::TAU;
use wide::{f32x8, f64x4};

#[derive(Clone, Copy)]
pub struct KeySpec {
    pub midi: u8,
    pub strings: usize,
    pub cents: f64,
    pub stiffness: f64,
    pub length: f64,
    pub tension: f64,
    pub hammer_kg: f64,
    /// Quadratic and linear coefficients of the compression-only hammer, N/m² and N/m.
    pub contact_k: f64,
    pub contact_linear: f64,
    pub strike_fraction: f64,
    pub unison_cents: f64,
    /// Relative contribution of the two strings at the bearing, with unity mean.
    pub bearing_imbalance: f64,
    pub losses_db_s: [f64; 6],
}

impl KeySpec {
    pub fn for_note(midi: u8) -> Option<Self> {
        scale::spec(midi)
    }

    /// Continue the end key's winding, tuning offset and hammer at the requested pitch.
    pub fn extended(note: u8) -> Self {
        use engine::{HIGH, LOW};
        let edge = note.clamp(LOW, HIGH);
        let mut spec = Self::for_note(edge).unwrap();
        spec.length *= 2.0_f64.powf((f64::from(edge) - f64::from(note)) / 12.);
        spec.midi = note;
        spec
    }

    fn frequency(self) -> f64 {
        440.0 * 2f64.powf((self.midi as f64 - 69.0 + self.cents / 100.) / 12.)
    }
}

pub type Key = PackedKey<f64x4>;
pub(crate) type LiveKey = PackedKey<f32x8>;

pub struct PackedKey<V: Lanes>(Course<V>);

impl Key {
    /// The argument is the integration rate. The offline probe runs at 4x.
    pub fn new(spec: KeySpec, rate: f64) -> Self {
        Self::at_rates(spec, rate, 4)
    }
}
impl LiveKey {
    pub(crate) fn realtime(spec: KeySpec, host_rate: f64, fine_treble_from: u8) -> Self {
        let steps = if host_rate <= 48000. {
            4
        } else if host_rate <= 96000. {
            2
        } else {
            1
        };
        // The short upper-treble collisions need finer integration to preserve
        // overtone amplitudes against the 384 kHz reference.
        let steps = if spec.midi >= fine_treble_from {
            2 * steps
        } else {
            steps
        };
        Self::at_rates(spec, host_rate * steps as f64, steps)
    }
}

impl<V: Lanes> PackedKey<V> {
    fn at_rates(spec: KeySpec, rate: f64, steps: usize) -> Self {
        Self(Course::at_rates(
            CourseSpec {
                frequency_hz: spec.frequency(),
                strings: spec.strings,
                stiffness: spec.stiffness,
                length: spec.length,
                tension: spec.tension,
                hammer_kg: spec.hammer_kg,
                contact_k: spec.contact_k,
                contact_linear: spec.contact_linear,
                strike_fraction: spec.strike_fraction,
                unison_cents: spec.unison_cents,
                read_imbalance: spec.bearing_imbalance,
                losses_db_s: spec.losses_db_s,
                hammer_width_m: 0.006,
                has_damper: spec.midi <= 93,
                damper_loss: 55.,
                readout: Readout::BearingForce,
            },
            rate,
            steps,
        ))
    }
    pub fn strike(&mut self, velocity: f64) {
        if velocity <= 0. {
            self.release();
            return;
        }
        self.0
            .strike_speed(0.25 + 4.0 * velocity.clamp(0., 1.).powf(1.5));
    }
    pub fn release(&mut self) {
        self.0.release();
    }
    pub fn sustain(&mut self, down: bool) {
        self.0.sustain(down);
    }
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
    pub(crate) fn quiet(&self) -> bool {
        self.0.quiet(1e-6)
    }
    pub(crate) fn host_sample(&mut self) -> f64 {
        self.0.host_sample()
    }
    /// Total transverse bearing force in newtons.
    pub fn step(&mut self) -> f64 {
        self.0.step()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pedal_and_restrike_keep_energy_in_the_same_strings() {
        for midi in [42, 43, 50, 63] {
            let mut key = Key::new(KeySpec::for_note(midi).unwrap(), 96000.);
            key.strike(0.8);
            for _ in 0..15000 {
                assert!(key.step().is_finite());
            }
            key.sustain(true);
            key.release();
            let peak = |key: &mut Key| (0..12000).map(|_| key.step().abs()).fold(0., f64::max);
            assert!(peak(&mut key) > 0.01);
            key.sustain(false);
            peak(&mut key);
            assert!(peak(&mut key) < 0.02);
            key.strike(1.0);
            assert!(peak(&mut key) > 0.01);
        }
    }
}

#[cfg(test)]
mod scale_tests {
    use super::*;

    #[test]
    fn bb5_third_partial_tracks_native_strikes() {
        let rate = 48000.;
        let spec = KeySpec::for_note(82).unwrap();
        let frequency = spec.frequency();
        let third = 3. * frequency * ((1. + 9. * spec.stiffness) / (1. + spec.stiffness)).sqrt();
        let start = (0.065 * rate) as usize;
        let end = (0.240 * rate) as usize;
        let mut piano = engine::Engine::new(rate);
        // Sullivan native MP, F and FF, relative to the ringing fundamental.
        for (velocity, recorded_db) in [(65, -29.42), (92, -31.41), (116, -27.32)] {
            piano.reset();
            piano.note_on(82, velocity as f32 / 127.);
            let mut sums = [(0., 0.); 2];
            for i in 0..end {
                let sample = f64::from(piano.sample()[0]);
                if i >= start {
                    let window =
                        0.5 - 0.5 * (TAU * (i - start) as f64 / (end - start - 1) as f64).cos();
                    for (sum, f) in sums.iter_mut().zip([frequency, third]) {
                        let (s, c) = (TAU * f * i as f64 / rate).sin_cos();
                        sum.0 += window * sample * s;
                        sum.1 += window * sample * c;
                    }
                }
            }
            let [h1, h3] = sums.map(|(a, b)| a.hypot(b));
            let db = 20. * (h3 / h1).log10();
            assert!(
                (db - recorded_db).abs() < 3.,
                "B-flat 5 velocity {velocity}: {db:.2} dB"
            );
        }
    }

    #[test]
    fn d5_rings_at_the_level_of_its_neighbors() {
        let rate = 48000.;
        let mut piano = engine::Engine::new(rate);
        for velocity in [25_u8, 45, 65, 92, 116, 127] {
            let [lower, d5, upper] = [73, 74, 75].map(|note| {
                piano.reset();
                piano.note_on(note, f32::from(velocity) / 127.);
                let omega = TAU * KeySpec::for_note(note).unwrap().frequency() / rate;
                let (mut re, mut im) = (0., 0.);
                // Measure the ringing fundamental after the hammer transient, 50–150 ms.
                for _ in 0..2400 {
                    piano.sample();
                }
                for i in 0..4800 {
                    let t = f64::from(i);
                    let w = 0.5 - 0.5 * (TAU * t / 4800.).cos();
                    let (sin, cos) = (omega * t).sin_cos();
                    let x = f64::from(piano.sample()[0]) * w;
                    re += x * cos;
                    im += x * sin;
                }
                20. * re.hypot(im).log10()
            });
            let difference = d5 - 0.5 * (lower + upper);
            assert!(
                difference.abs() < 1.5,
                "D5 velocity {velocity}: {difference:.2} dB"
            );
        }
    }

    #[test]
    fn damper_ends_at_a6_and_restrike_retains_motion() {
        for midi in [93, 94] {
            let spec = KeySpec::for_note(midi).unwrap();
            let mut held = LiveKey::realtime(spec, 48000., engine::FINE_TREBLE_FROM);
            let mut released = LiveKey::realtime(spec, 48000., engine::FINE_TREBLE_FROM);
            for k in [&mut held, &mut released] {
                k.strike(0.7);
                for _ in 0..3000 {
                    k.host_sample();
                }
            }
            released.release();
            let mut held_energy = 0.;
            let mut released_energy = 0.;
            for _ in 0..3000 {
                held.host_sample();
                released.host_sample();
            }
            for _ in 0..2400 {
                held_energy += held.host_sample().powi(2);
                released_energy += released.host_sample().powi(2);
            }
            if midi == 94 {
                assert_eq!(released_energy, held_energy);
            } else {
                assert!(released_energy < 0.01 * held_energy);
            }
            held.strike(0.7);
            assert!(held.host_sample().abs() > 1e-6);
        }
    }
}
