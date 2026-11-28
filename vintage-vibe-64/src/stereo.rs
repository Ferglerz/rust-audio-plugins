//! Serviced StereoVibe Rev 4.1. The published PCB component positions identify
//! the revision; its complete schematic and most component values are unavailable.
use pleasant_dsp::circuit::{Circuit, Filter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Classic,
    Fet,
    Parallel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub volume: f64,
    pub bass: f64,
    pub treble: f64,
    pub rate: f64,
    pub depth: f64,
    pub tremolo: bool,
    pub half_speed: bool,
    pub input: Input,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            volume: 1.,
            bass: 0.5,
            treble: 0.5,
            rate: 0.5,
            depth: 0.7,
            tremolo: false,
            half_speed: false,
            input: Input::Classic,
        }
    }
}
impl Controls {
    /// Estimated speed-pot range. The manual specifies a push-operated half cut.
    pub fn rate_hz(&self) -> f64 {
        let full = 9.0 / (1. + (1. - self.rate) * 5.);
        if self.half_speed {
            full * 0.5
        } else {
            full
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Output {
    pub pickup_v: f64,
    pub send_v: f64,
    pub left_v: f64,
    pub right_v: f64,
}

pub struct Panel {
    controls: Controls,
    input: Filter<3, 1, 2>,
    rate: f64,
    phase: f64,
    light: [f64; 2],
    optical_step: [f64; 2],
    smoothing: f64,
    wet: f64,
    gain: f64,
    gain_target: f64,
    classic_mix: f64,
    fet_mix: f64,
    tone_history: [f64; 2],
    tone_step: [f64; 2],
    shelf: [f64; 2],
    shelf_target: [f64; 2],
    phase_step: f64,
    target_phase_step: f64,
}
impl Panel {
    pub fn new(rate: f64, controls: Controls) -> Self {
        assert!(rate.is_finite() && rate >= 8000.);
        let mut panel = Self {
            controls: Controls::default(),
            input: Filter::default(),
            rate,
            phase: 0.,
            light: [0.5; 2],
            optical_step: [0.069, 0.016].map(|s| -(-1. / (s * rate)).exp_m1()),
            smoothing: -(-1. / (0.004 * rate)).exp_m1(),
            wet: 0.,
            gain: 1.,
            gain_target: 1.,
            classic_mix: 1.,
            fet_mix: 0.,
            tone_history: [0.; 2],
            tone_step: [140., 1800.].map(|hz| -(-std::f64::consts::TAU * hz / rate).exp_m1()),
            shelf: [1.; 2],
            shelf_target: [1.; 2],
            target_phase_step: 0.,
            phase_step: 0.,
        };
        panel.compile_input();
        panel.set_controls(controls);
        panel.reset();
        panel
    }

    fn compile_input(&mut self) {
        let mut n = Circuit::<3>::new(3, self.rate);
        // Same series harp R/L/C as the passive model. The jack plate exposes
        // node 2 before the preamp. Loading changes with the under-lid DIP bank.
        n.input(1, 1. / 2500.);
        n.r(1, 0, 2500.);
        n.l(1, 2, 0.90);
        n.c(2, 0, 150e-12);
        n.c(2, 3, 0.22e-6);
        if self.controls.input != Input::Fet {
            // Effective Peterson-style small-signal input bias and base impedance.
            n.r(3, 0, 33e3);
            n.r(3, 0, 390e3);
            n.r(3, 0, 150e3);
        }
        if self.controls.input != Input::Classic {
            n.r(3, 0, 1e6);
        }
        n.compile_ports([2, 3], &mut self.input);
    }

    pub fn set_controls(&mut self, mut c: Controls) {
        if c == self.controls && self.phase_step > 0. {
            return;
        }
        let bounded = |v: f64, old: f64| if v.is_finite() { v.clamp(0., 1.) } else { old };
        c.volume = bounded(c.volume, self.controls.volume);
        c.bass = bounded(c.bass, self.controls.bass);
        c.treble = bounded(c.treble, self.controls.treble);
        c.depth = bounded(c.depth, self.controls.depth);
        c.rate = bounded(c.rate, self.controls.rate);
        let input_changed = c.input != self.controls.input;
        self.controls = c;
        if input_changed {
            self.compile_input();
        }
        self.target_phase_step = c.rate_hz() / self.rate;
        self.gain_target = c.volume.powf(2.5);
        self.shelf_target = [c.bass, c.treble].map(|knob| 10_f64.powf((knob - 0.5) * 24. / 20.));
    }

    pub fn reset(&mut self) {
        self.clear_audio();
        self.phase = 0.;
        self.phase_step = self.target_phase_step;
        self.light = [0.5; 2];
        self.wet = if self.controls.tremolo {
            self.controls.depth
        } else {
            0.
        };
        self.gain = self.gain_target;
        self.shelf = self.shelf_target;
        (self.classic_mix, self.fet_mix) = self.input_mix();
    }

    fn input_mix(&self) -> (f64, f64) {
        match self.controls.input {
            Input::Classic => (1., 0.),
            Input::Fet => (0., 1.),
            Input::Parallel => (0.5, 0.5),
        }
    }

    pub(crate) fn clear_audio(&mut self) {
        self.input.reset();
        self.tone_history = [0.; 2];
    }

    pub(crate) fn quiet(&self) -> bool {
        self.input.history_below(1e-9) && self.tone_history.iter().all(|x| x.abs() < 1e-9)
    }

    pub(crate) fn optical_gains(&mut self) -> [f64; 2] {
        // Opposed switching drives illuminate the two optical packages visible
        // in CSB0001. Residual conductance and cell times are fitted estimates.
        self.phase_step += self.smoothing * (self.target_phase_step - self.phase_step);
        self.phase = (self.phase + self.phase_step).fract();
        let depth = if self.controls.tremolo {
            self.controls.depth
        } else {
            0.
        };
        self.wet += self.smoothing * (depth - self.wet);
        let mut out = [0.; 2];
        for (i, y) in out.iter_mut().enumerate() {
            let drive = f64::from((self.phase < 0.5) == (i == 0));
            let light = &mut self.light[i];
            *light += self.optical_step[usize::from(drive < *light)] * (drive - *light);
            // Equivalent series photocell into a termination. The 20k scale
            // is the serviced channel-pair value; its exact wiring is unknown.
            let conductance = 1. / 20e6 + *light * (1. / 2000. - 1. / 20e6);
            let optical = 1.10 * 20e3 * conductance / (1. + 20e3 * conductance);
            *y = 1. + self.wet * (optical - 1.);
        }
        out
    }

    pub fn tick(&mut self, source: f64) -> Output {
        let [send_v, v] = self.input.tick_ports([source]);
        let (classic, fet) = self.input_mix();
        self.classic_mix += self.smoothing * (classic - self.classic_mix);
        self.fet_mix += self.smoothing * (fet - self.fet_mix);
        // Local source-degenerated FET transfer with an estimated 2 V knee.
        let fet_v = 2. * (v * 1.15).tanh();
        let mut audio = self.classic_mix * (1.8 * v) + self.fet_mix * fet_v;
        for (gain, target) in self.shelf.iter_mut().zip(self.shelf_target) {
            *gain += self.smoothing * (target - *gain);
        }
        self.tone_history[0] += self.tone_step[0] * (audio - self.tone_history[0]);
        audio += (self.shelf[0] - 1.) * self.tone_history[0];
        self.tone_history[1] += self.tone_step[1] * (audio - self.tone_history[1]);
        audio += (self.shelf[1] - 1.) * (audio - self.tone_history[1]);
        self.gain += self.smoothing * (self.gain_target - self.gain);
        audio *= self.gain;

        let [left, right] = self.optical_gains();
        Output {
            pickup_v: source,
            send_v,
            left_v: audio * left,
            right_v: audio * right,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::{Engine, StereoEngine};
    use std::f64::consts::TAU;

    fn rms(c: Controls, hz: f64, rate: f64, amplitude: f64) -> [f64; 2] {
        let mut panel = Panel::new(rate, c);
        let mut power = [0.; 2];
        for i in 0..24000 {
            let v = panel.tick(amplitude * (TAU * hz * f64::from(i) / rate).sin());
            if i >= 12000 {
                power[0] += v.send_v.powi(2);
                power[1] += v.left_v.powi(2);
                assert_eq!(v.left_v, v.right_v);
            }
        }
        power.map(|v| (v / 12000.).sqrt())
    }
    #[test]
    fn dry_taps_distinguish_input_load_and_each_shelf() {
        let c = Controls::default();
        for rate in [44100., 48000., 96000.] {
            for (hz, low) in [(40., true), (5000., false)] {
                let dry = rms(c, hz, rate, 0.03);
                let mut hi = c;
                let mut lo = c;
                if low {
                    hi.bass = 1.;
                    lo.bass = 0.;
                } else {
                    hi.treble = 1.;
                    lo.treble = 0.;
                }
                let boost = rms(hi, hz, rate, 0.03);
                let cut = rms(lo, hz, rate, 0.03);
                assert_eq!(dry[0], boost[0]);
                assert_eq!(dry[0], cut[0]);
                assert!(boost[1] > dry[1] * 1.9 && cut[1] < dry[1] * 0.7);
            }
            let classic = rms(c, 5000., rate, 0.03);
            let fet = rms(
                Controls {
                    input: Input::Fet,
                    ..c
                },
                5000.,
                rate,
                0.03,
            );
            let both = rms(
                Controls {
                    input: Input::Parallel,
                    ..c
                },
                5000.,
                rate,
                0.03,
            );
            assert!(fet[0] > classic[0] * 1.20 && both[0] < classic[0]);
            assert_ne!(both[1], classic[1]);
        }
    }
    #[test]
    fn actual_harp_and_pedal_are_shared_across_both_outputs() {
        for n in [0, 45, 100, 127] {
            let mut passive = Engine::new(48000.);
            let mut active = StereoEngine::active(48000.);
            passive.set_extended(true);
            active.set_extended(true);
            passive.set_pickup_gap_mm(0.5);
            active.set_pickup_gap_mm(0.5);
            passive.set_sustain(true);
            active.set_sustain(true);
            passive.note_on(n, 0.8);
            active.note_on(n, 0.8);
            let mut peak = 0_f64;
            for t in 0..12000 {
                if t == 3500 {
                    passive.note_off(n);
                    active.note_off(n);
                }
                if t == 8000 {
                    passive.set_sustain(false);
                    active.set_sustain(false);
                }
                let p = passive.next_outputs();
                let a = active.next_outputs();
                peak = peak.max(a.left_v.abs());
                assert_eq!(p.pickup_v, a.pickup_v);
                assert_eq!(a.left_v, a.right_v);
            }
            assert!(peak > 1e-9);
        }
    }

    #[test]
    fn optical_rate_halves_and_dry_bypass_remains_mono() {
        for rate in [44100., 48000., 96000.] {
            let normal = Controls {
                tremolo: true,
                depth: 1.,
                rate: 0.68,
                ..Controls::default()
            };
            for half in [false, true] {
                let c = Controls {
                    half_speed: half,
                    ..normal
                };
                let mut p = Panel::new(rate, c);
                let mut changes = vec![];
                let mut prev = false;
                for i in 0..(rate as usize * 2) {
                    let [a, b] = p.optical_gains();
                    if i > rate as usize / 10 && a > b && !prev {
                        changes.push(i);
                    }
                    prev = a > b;
                }
                let mean =
                    (changes.last().unwrap() - changes[0]) as f64 / (changes.len() - 1) as f64;
                assert!((rate / mean - c.rate_hz()).abs() < 0.001);
                p.set_controls(Controls {
                    tremolo: false,
                    ..c
                });
                for _ in 0..rate as usize / 2 {
                    p.optical_gains();
                }
                let [a, b] = p.optical_gains();
                assert!((a - b).abs() < 1e-12);
            }
        }
    }
}
