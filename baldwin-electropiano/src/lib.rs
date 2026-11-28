//! Baldwin classroom Electropiano, with pressure pickups in a common termination rail.
pub mod action;
pub mod rail;

use action::{Action, HIGH, LOW};
use pleasant_dsp::{instrument::Decimator, piezo::PiezoInput, struck_keys::StrikeKeys};
use rail::{BearingForces, Rail, SENSORS};

pub const LATENCY: u32 = 8;
pub struct Engine {
    keys: StrikeKeys<Action>,
    rail: Rail,
    sensors: [PiezoInput; 2],
    decimator: Decimator<3>,
    steps: usize,
    sustain: bool,
    soft: bool,
    volume: f64,
    target_volume: f64,
    ramp: pleasant_dsp::interpolation::ControlRamp,
    tail: u32,
    timeout: u32,
}
#[derive(Default)]
pub struct Taps {
    pub bass_v: f64,
    pub treble_v: f64,
    pub jack_v: f64,
}
impl Engine {
    pub fn new(rate: f64) -> Self {
        Self::assemble(rate, Action::new, Rail::new)
    }

    /// Reproduce the pre-fit hammer and shim on the same string scale.
    pub fn with_regulation(
        rate: f64,
        felt: f64,
        mass: f64,
        line: f64,
        stiffness: f64,
        damping: f64,
    ) -> Self {
        Self::assemble(
            rate,
            |n, hz| Action::with_regulation(n, hz, felt, mass, line),
            |hz| Rail::with_shim(hz, stiffness, damping),
        )
    }

    fn assemble(
        rate: f64,
        action: impl Fn(u8, f64) -> Action,
        rail: impl FnOnce(f64) -> Rail,
    ) -> Self {
        assert!(rate.is_finite() && rate >= 8000.);
        let steps = (48000. / rate).ceil().max(1.) as usize;
        let internal = rate * steps as f64;
        let sensor = |count: usize| {
            PiezoInput::new(
                internal,
                220e-12,
                count as f64 * 1.8e-9 + 470e-12,
                220e-9,
                1e6,
            )
        };
        Self {
            keys: StrikeKeys::new(rate, LOW, HIGH, |n| action(n, internal)),
            rail: rail(internal),
            sensors: [sensor(6), sensor(SENSORS - 6)],
            decimator: Decimator::new(steps, rate, LATENCY as usize),
            steps,
            sustain: false,
            soft: false,
            volume: 1.,
            target_volume: 1.,
            ramp: pleasant_dsp::interpolation::ControlRamp::new(rate),
            tail: 0,
            timeout: (0.6 * rate) as u32,
        }
    }
    pub fn set_extended(&mut self, yes: bool) {
        self.keys.set_extended(yes);
    }
    pub fn set_pickup_position_mm(&mut self, mm: f64) {
        self.rail.set_offset_mm(mm);
    }
    pub fn set_controls(&mut self, volume: f64, soft: bool, sustain: bool) {
        if volume.is_finite() && volume.clamp(0., 1.) != self.target_volume {
            self.target_volume = volume.clamp(0., 1.);
            if self.ramp.retarget() {
                self.volume = self.target_volume;
            }
        }
        self.soft = soft;
        self.sustain = sustain;
        self.keys.set_sustain(sustain);
    }
    pub fn note_on(&mut self, n: u8, v: f32) {
        self.keys.note_on(n, v);
    }
    pub fn note_off(&mut self, n: u8) {
        self.keys.note_off(n);
    }
    pub fn audition(&mut self, n: u8, v: f32) {
        self.keys.audition(n, v);
    }
    pub fn audition_off(&mut self) {
        self.keys.audition_off();
    }
    pub fn choke(&mut self, n: u8) {
        self.keys.choke(n);
    }
    pub fn held_notes(&self) -> u128 {
        self.keys.held_notes()
    }
    pub fn sounding(&self) -> bool {
        self.keys.active() || self.tail != 0
    }
    pub fn reset(&mut self) {
        self.keys.reset();
        self.sustain = false;
        self.tail = 0;
        self.volume = self.target_volume;
        self.ramp.reset();
        self.rail.reset();
        self.sensors.iter_mut().for_each(PiezoInput::reset);
        self.decimator.reset();
    }
    pub fn sample_with_taps(&mut self) -> Taps {
        if let Some(step) = self.ramp.step() {
            self.volume += (self.target_volume - self.volume) * step;
        }
        if !self.sounding() {
            return Taps::default();
        }
        let held = self.keys.held_notes();
        self.keys.begin_frame();
        let mut out = [0.; 3];
        for i in 0..self.steps {
            let mut f = BearingForces::default();
            self.keys.step_sources(|n, v| f.add(n, v));
            let pressure = self.rail.sample(f, held, self.sustain);
            let bass = self.sensors[0].tick(pressure[0]);
            let treble = self.sensors[1].tick(pressure[1]);
            // The owner shows two register leads. Equivalent pressure bus:
            // equal-type elements sharing a common input in the classroom instrument.
            let mix = (6. * bass + 16. * treble) / 22.;
            let jack = mix * 12. * self.volume * if self.soft { 0.5 } else { 1. };
            self.decimator.push([bass, treble, jack]);
            if i == 0 {
                out = self.decimator.read();
            }
        }
        self.keys.finish_frame();
        if self.keys.active() || out[0].abs().max(out[1].abs()) > 1e-8 {
            self.tail = self.timeout;
        } else {
            self.tail = self.tail.saturating_sub(1);
        }
        Taps {
            bass_v: out[0],
            treble_v: out[1],
            jack_v: out[2],
        }
    }
    pub fn render(&mut self, out: &mut [f32]) {
        for s in out {
            *s = self.sample_with_taps().jack_v as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_pedal_attenuates_the_jack_and_preserves_both_loaded_pickup_voltages() {
        let mut a = Engine::new(48000.);
        let mut b = Engine::new(48000.);
        for engine in [&mut a, &mut b] {
            engine.note_on(42, 0.7);
            engine.note_on(66, 0.7);
        }
        let mut peak = [0.0_f64; 2];
        for i in 0..10000 {
            if i == 4800 {
                b.set_controls(1., true, false);
            }
            let x = a.sample_with_taps();
            let y = b.sample_with_taps();
            assert_eq!([x.bass_v, x.treble_v], [y.bass_v, y.treble_v]);
            if i > 4864 {
                assert!((x.jack_v - 2. * y.jack_v).abs() < 1e-9);
            }
            peak[0] = peak[0].max(y.bass_v.abs());
            peak[1] = peak[1].max(y.treble_v.abs());
        }
        assert!(peak.iter().all(|&p| p > 1e-5));
    }

    #[test]
    fn distance_along_rail_revoices_the_ceramic_and_virtual_keys_have_their_own_pitch() {
        for rate in [44100., 48000., 96000.] {
            let mut a = Engine::new(rate);
            a.note_on(0, 1.);
            a.note_on(127, 1.);
            assert_eq!(a.held_notes(), 0);
            a.set_extended(true);
            for note in [0, 127] {
                a.reset();
                a.note_on(note, 0.75);
                assert_eq!(a.held_notes(), 1u128 << note);
                // Unplayed rail/string modes and the impact have their own
                // zero crossings. Compare the note's coherent fundamental to
                // tones on either side of it once the initial rail impulse decays.
                let f = action::frequency(note);
                let offsets = if note == 0 {
                    [-2.4, 0., 2.4]
                } else {
                    [-190., 0., 190.]
                };
                let start = (rate * 0.45) as usize;
                let length = (rate * if note == 0 { 1.85 } else { 0.3 }) as usize;
                let mut quadrature = [[0., 0.]; 3];
                for i in 0..start + length {
                    let v = a.sample_with_taps().jack_v;
                    assert!(v.is_finite());
                    if i < start {
                        continue;
                    }
                    let t = (i - start) as f64;
                    let w = 0.5 - 0.5 * (std::f64::consts::TAU * t / length as f64).cos();
                    for (bin, offset) in quadrature.iter_mut().zip(offsets) {
                        let (sin, cos) =
                            (std::f64::consts::TAU * (f + offset) * t / rate).sin_cos();
                        bin[0] += v * w * cos;
                        bin[1] += v * w * sin;
                    }
                }
                let amplitudes = quadrature.map(|v| v[0].hypot(v[1]) * 4. / length as f64);
                assert!(
                    amplitudes[1] > 1e-7 && amplitudes[1] > 3. * amplitudes[0].max(amplitudes[2]),
                    "rate={rate}, MIDI={note}, nearby and fundamental {amplitudes:?}"
                );
            }
        }
        let mut a = Engine::new(48000.);
        let mut b = Engine::new(48000.);
        b.set_pickup_position_mm(8.);
        a.note_on(64, 0.7);
        b.note_on(64, 0.7);
        let mut difference = 0.;
        for _ in 0..12000 {
            difference += (a.sample_with_taps().jack_v - b.sample_with_taps().jack_v).powi(2);
        }
        assert!(difference > 0.001);
    }
}
