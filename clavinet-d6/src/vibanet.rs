//! Two polymer strips enclose the short afterlength on the factory Vibanet.
use super::StringVoice;

/// A standard linear solid. The relaxed spring carries static deformation;
/// the Maxwell branch loses energy as its internal reference creeps.
pub(super) struct GelCell {
    pub node: usize,
    branch: f64,
    pub(super) inv: f64,
    memory: f64,
    relaxation: f64,
    spring: [f64; 2],
    dashpot_work: f64,
}
impl GelCell {
    pub(super) fn force(&self, displacement: &mut f64) {
        *displacement = (*displacement + self.branch * self.memory) * self.inv;
    }
    pub(super) fn relax<const TRACE: bool>(&mut self, x: f64) -> f64 {
        let dz = (1. - self.relaxation) * (x - self.memory);
        self.memory += dz;
        if TRACE {
            self.dashpot_work * dz * dz
        } else {
            0.
        }
    }
    pub(super) fn stored(&self, x: f64) -> f64 {
        0.5 * (self.spring[0] * x * x + self.spring[1] * (x - self.memory).powi(2))
    }
    pub(super) fn clear(&mut self) {
        self.memory = 0.;
    }
}

impl StringVoice {
    /// Replace the woven yarn by a preloaded pair of viscoelastic strips.
    /// Stiffnesses are combined line stiffnesses in N/m² for the two faces.
    pub fn fit_gel(&mut self, relaxed: f64, transient: f64, relaxation_s: f64) {
        assert!(
            relaxed.is_finite()
                && transient.is_finite()
                && relaxed >= 0.
                && transient >= 0.
                && relaxation_s > 0.
                && relaxation_s.is_finite()
        );
        let end = self.current.len() - 2;
        let dx = self.geometry.speaking_m / (self.tangent - 1) as f64;
        let after = self.geometry.afterlength_m;
        // The 2014 film shows the strip alongside the tuning pins. Individual
        // keys' contact widths and exact termination distances are estimates.
        let start = (after - 0.034).max(0.20 * after);
        let stop = 0.92 * after;
        let a = relaxation_s / (relaxation_s + 1. / self.rate);
        self.gel.clear();
        self.gel_tangent_inv = 1.;
        for i in self.tangent..end {
            self.drag[i] = self.steel_loss;
            self.inv[i] = 1. / (1. + self.steel_loss);
            let x = (i - self.tangent) as f64 * dx;
            let width = ((x + dx / 2.).min(stop) - (x - dx / 2.).max(start)).max(0.);
            if width > 0. {
                let coeff = width * self.inv[i] / (self.cell_mass * self.rate * self.rate);
                let branch = transient * a * coeff;
                let inv = 1. / (1. + relaxed * coeff + branch);
                if i == self.tangent {
                    self.gel_tangent_inv = inv;
                }
                self.gel.push(GelCell {
                    node: i,
                    inv,
                    branch,
                    relaxation: a,
                    memory: self.current[i],
                    spring: [relaxed * width, transient * width],
                    dashpot_work: transient * width * relaxation_s * self.rate,
                });
            }
        }
    }
}

use super::live::{PickupBoard, TangentKeyboard, HIGH, LOW};
use super::Pickups;
use pleasant_dsp::{
    circuit::{Filter, SizedCircuit},
    filters::{BiquadCoefficients as Coeff, BiquadKind, DirectForm1},
};
use std::f64::consts::PI;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Selector {
    Lead,
    Rhythm,
    Fulltone,
    Phase,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Controls {
    pub pickup: Selector,
    /// The physical switches have five positions, −15 through +15 dB in 7.5 dB steps.
    pub eq: [i8; 3],
    pub volume: f64,
    pub wah: bool,
    pub sensitivity: f64,
    pub mute: f64,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            pickup: Selector::Fulltone,
            eq: [0; 3],
            volume: 1.,
            wah: false,
            sensitivity: 0.5,
            mute: 0.,
        }
    }
}

/// The documented panel and the recorded EQ provide a behavioral model of
/// the proprietary preamp. The factory wiring sheet identifies the terminals.
pub struct Panel {
    rate: f64,
    controls: Controls,
    load: Filter<2>,
    filters: [DirectForm1; 3],
    eq: [Coeff; 3],
    env: f64,
    attack: f64,
    release: f64,
    integrators: [f64; 2],
    wet: f64,
    switch_step: f64,
    ramp: pleasant_dsp::interpolation::ControlRamp,
    volume: f64,
    sensitivity: f64,
    cutoff: f64,
}
impl Panel {
    fn new(rate: f64) -> Self {
        let controls = Controls::default();
        let mut p = Self {
            rate,
            controls,
            load: Filter::default(),
            filters: [DirectForm1::default(); 3],
            eq: [Coeff::IDENTITY; 3],
            env: 0.,
            attack: 1. - (-1. / (0.150 * rate)).exp(),
            release: 1. - (-1. / (0.082 * rate)).exp(),
            integrators: [0.; 2],
            wet: 0.,
            ramp: pleasant_dsp::interpolation::ControlRamp::new(rate),
            volume: controls.volume,
            sensitivity: controls.sensitivity,
            switch_step: 1. / (0.003 * rate),
            cutoff: 310.,
        };
        p.load_coils();
        p
    }
    fn load_coils(&mut self) {
        let pair = matches!(self.controls.pickup, Selector::Fulltone | Selector::Phase);
        // Per-coil electrical constants are provisional. Two selected coils
        // present half the series impedance to the high-impedance input.
        let bank = if pair { 0.5 } else { 1. };
        // Vintage Vibe specifies the original winding and an ultrasonic
        // self-resonance. Use the related Hohner winding's nominal DC estimate
        // and 100 mH into 220 pF (34 kHz). No production impedance sweep.
        let r = 200. * bank;
        let mut c = SizedCircuit::<2, 3, 3>::new(2, self.rate);
        c.r(1, 0, r);
        c.input(1, 1. / r);
        c.l(1, 2, 0.10 * bank);
        c.r(2, 0, 1e6);
        c.c(2, 0, 220e-12);
        c.compile(2, &mut self.load);
    }
    fn set(&mut self, c: Controls) {
        let c = Controls {
            eq: c.eq.map(|v| v.clamp(-2, 2)),
            volume: c.volume.clamp(0., 1.),
            sensitivity: c.sensitivity.clamp(0., 1.),
            mute: c.mute.clamp(0., 1.),
            ..c
        };
        let old = self.controls;
        self.controls = c;
        if (old.volume != c.volume || old.sensitivity != c.sensitivity) && self.ramp.retarget() {
            self.volume = c.volume;
            self.sensitivity = c.sensitivity;
        }
        if old.pickup != c.pickup {
            self.load_coils();
        }
        for (i, (kind, freq, q)) in [
            (BiquadKind::LowShelf, 280., 0.45),
            (BiquadKind::Bell, 1830., 0.55),
            (BiquadKind::HighShelf, 6100., 0.73),
        ]
        .into_iter()
        .enumerate()
        {
            if old.eq[i] != c.eq[i] {
                self.eq[i] = if c.eq[i] == 0 {
                    Coeff::IDENTITY
                } else {
                    Coeff::design(kind, freq, 7.5 * f64::from(c.eq[i]), q, self.rate)
                };
            }
        }
    }
    pub fn cutoff_hz(&self) -> f64 {
        self.cutoff
    }
    fn wah(&mut self, x: f64) -> f64 {
        let rectified = x.abs();
        let coefficient = if rectified > self.env {
            self.attack
        } else {
            self.release
        };
        self.env += coefficient * (rectified - self.env);
        self.cutoff = 310. + 4640. * (1. - (-155. * self.sensitivity * self.env).exp());
        // Resonant low-pass. A strong low fundamental remains audible while a
        // second peak climbs through the harmonics in the factory demonstration.
        let g = (PI * self.cutoff / self.rate).tan();
        let k = 1. / 3.05;
        let (s1, s2) = (self.integrators[0], self.integrators[1]);
        let v1 = (s1 + g * (x - s2)) / (1. + g * (g + k));
        let v2 = s2 + g * v1;
        self.integrators = [2. * v1 - s1, 2. * v2 - s2];
        v2
    }
}
impl PickupBoard for Panel {
    fn tick(&mut self, p: Pickups) -> f64 {
        if let Some(step) = self.ramp.step() {
            self.volume += step * (self.controls.volume - self.volume);
            self.sensitivity += step * (self.controls.sensitivity - self.sensitivity);
        }
        let selected = match self.controls.pickup {
            Selector::Lead => p.bridge,
            Selector::Rhythm => p.centre,
            Selector::Fulltone => 0.5 * (p.centre + p.bridge),
            Selector::Phase => 0.5 * (p.centre - p.bridge),
        };
        let mut dry = 220. * self.load.tick(selected);
        for (filter, coeff) in self.filters.iter_mut().zip(self.eq) {
            dry = filter.process(dry, coeff);
        }
        let target = f64::from(self.controls.wah);
        self.wet += (target - self.wet).clamp(-self.switch_step, self.switch_step);
        let wet = self.wah(dry);
        (dry + self.wet * (wet - dry)) * self.volume
    }
    fn clear(&mut self) {
        self.load.reset();
        self.filters.iter_mut().for_each(DirectForm1::reset);
        self.integrators = [0.; 2];
        self.env = 0.;
        self.cutoff = 310.;
        self.wet = f64::from(self.controls.wah);
        self.volume = self.controls.volume;
        self.sensitivity = self.controls.sensitivity;
        self.ramp.reset();
    }
}

pub type Engine = TangentKeyboard<Panel>;
impl Engine {
    pub fn vibanet(rate: f64, extended: bool) -> Self {
        let (lo, hi) = if extended { (0, 127) } else { (LOW, HIGH) };
        let mut instrument = Self::with_panel(rate, lo, hi, Panel::new);
        instrument.fit_gel(20_000., 170_000., 0.00035);
        instrument
    }
    pub fn set_vibanet_controls(&mut self, c: Controls) {
        self.set_mute(c.mute);
        self.panel.set(c);
    }
    pub fn wah_frequency_hz(&self) -> f64 {
        self.panel.cutoff_hz()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_reconnects_both_live_coils_without_restarting_the_string() {
        let mut unchanged = Engine::vibanet(48_000., false);
        let mut switched = Engine::vibanet(48_000., false);
        for engine in [&mut unchanged, &mut switched] {
            engine.note_on(45, 0.8);
        }
        for setting in [
            Selector::Fulltone,
            Selector::Lead,
            Selector::Rhythm,
            Selector::Phase,
        ] {
            switched.set_vibanet_controls(Controls {
                pickup: setting,
                ..Default::default()
            });
            let mut difference = 0.;
            for _ in 0..2400 {
                let (a, jack_a) = unchanged.sample_with_pickups();
                let (b, jack_b) = switched.sample_with_pickups();
                assert_eq!([a.centre, a.bridge], [b.centre, b.bridge]);
                difference += (jack_a - jack_b).abs();
            }
            if setting == Selector::Fulltone {
                assert_eq!(difference, 0.);
            } else {
                assert!(difference > 0.1);
            }
        }
    }

    #[test]
    fn measured_eq_steps_and_detector_act_on_the_summed_bus() {
        let rate = 48000.;
        for (band, hz) in [(0, 70.), (1, 1830.), (2, 10000.)] {
            let mut previous = 0.;
            for step in -2..=2 {
                let mut panel = Panel::new(rate);
                let mut c = Controls::default();
                c.eq[band] = step;
                panel.set(c);
                let mut power = 0.;
                for i in 0..12000 {
                    let emf = (2. * PI * hz * i as f64 / rate).sin() * 0.0005;
                    let v = panel.tick(Pickups {
                        centre: emf,
                        bridge: emf,
                    });
                    if i >= 6000 {
                        power += v * v;
                    }
                }
                assert!(power > previous);
                previous = power;
            }
        }
        let mut panel = Panel::new(rate);
        panel.set(Controls {
            pickup: Selector::Phase,
            wah: true,
            ..Default::default()
        });
        for _ in 0..5000 {
            assert_eq!(
                panel.tick(Pickups {
                    centre: 0.001,
                    bridge: 0.001
                }),
                0.
            );
        }
        assert_eq!(panel.cutoff_hz(), 310.);
        panel.set(Controls {
            pickup: Selector::Fulltone,
            wah: true,
            ..Default::default()
        });
        for i in 0..5000 {
            let x = 0.001 * (2. * PI * 110. * i as f64 / rate).sin();
            assert!(panel
                .tick(Pickups {
                    centre: x,
                    bridge: x
                })
                .is_finite());
        }
        assert!(panel.cutoff_hz() > 1500.);
        panel.clear();
        assert_eq!(panel.cutoff_hz(), 310.);
    }

    #[test]
    fn released_string_retains_motion_and_loses_energy_to_the_gel() {
        let mut string = StringVoice::for_driven_key(46, 96000.).unwrap();
        string.fit_gel(20_000., 170_000., 0.00035);
        string.press(88);
        for _ in 0..14400 {
            string.step();
        }
        let held = string.trace();
        string.release();
        string.step();
        assert!(string.trace().stored_j > held.stored_j * 0.8);
        for _ in 0..24000 {
            string.step();
        }
        let tail = string.trace();
        assert!(tail.viscous_loss_j > held.viscous_loss_j);
        assert!(tail.stored_j < held.stored_j * 0.15);
        string.clear();
        assert_eq!(string.trace().stored_j, 0.);
        assert_eq!(string.step().centre, 0.);
    }
}
