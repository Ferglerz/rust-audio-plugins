//! Transformer, switched tone ladder and loaded jack of the D6 and E7.
use crate::{Pickups, Switch, Wiring};
use pleasant_dsp::circuit::{Circuit, CrossfadingFilter, SizedCircuit};

/// Prototype calibration from one medium E4 at Hohner's nominal 100 mV jack level.
pub const PICKUP_VOLTS: f64 = 0.000_071;
pub const SOFT: u8 = 1;
pub const MEDIUM: u8 = 2;
pub const TREBLE: u8 = 4;
pub const BRILLIANT: u8 = 8;
pub const ALL_TONES: u8 = SOFT | MEDIUM | TREBLE | BRILLIANT;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Board {
    D6,
    E7,
}

/// Small-signal NPN hybrid-pi coefficients at the board's DC operating point.
pub struct Transistor {
    pub gm: f64,
    pub gpi: f64,
    pub go: f64,
}

impl Transistor {
    pub fn stamp<const S: usize, const N: usize, const R: usize>(
        &self,
        c: &mut SizedCircuit<S, N, R>,
        collector: usize,
        base: usize,
        emitter: usize,
    ) {
        c.r(base, emitter, 1. / self.gpi);
        c.r(collector, emitter, 1. / self.go);
        c.gm(collector, emitter, base, self.gm);
    }
}

// Hybrid-pi parameters at the DC operating points in d6.cir and e7.cir.
struct Preamp {
    first: Transistor,
    second: Transistor,
    collector_load: f64,
    feedback: f64,
    bias_upper: f64,
    bias_lower: f64,
    emitter_load: f64,
}

impl Board {
    pub fn selector(self, selector: Switch, wiring: Wiring) -> (f64, f64, f64) {
        let add = match wiring {
            Wiring::DaAdds => 1.0,
            Wiring::DaSubtracts => -1.0,
        };
        // Thévenin equivalent of equal source resistances, before transformer loading.
        let (centre, bridge, source_ohms) = match selector {
            Switch::CA => (1.0, 0.0, 200.0),
            Switch::CB => (0.0, 1.0, 200.0),
            Switch::DA => (0.5, 0.5 * add, 100.0),
            Switch::DB => (0.5, -0.5 * add, 100.0),
        };
        // On 801 111 one bank stays on the red primary lead; the rocker
        // interchanges the terminals of the other bank (centre in the CA-solo map).
        let (centre, bridge) = if self == Board::E7 && bridge < 0. {
            (-centre, -bridge)
        } else {
            (centre, bridge)
        };
        (centre, bridge, source_ohms)
    }

    /// Simple loaded selector tap, before the transformer. The probe's test load is 2 kΩ.
    pub fn selector_output(self, selector: Switch, wiring: Wiring, pickups: Pickups) -> f64 {
        let (centre, bridge, ohms) = self.selector(selector, wiring);
        (centre * pickups.centre + bridge * pickups.bridge) * (2000. / (2000. + ohms))
    }

    fn preamp(self) -> Preamp {
        match self {
            Self::D6 => Preamp {
                first: Transistor {
                    gm: 5.383049363389e-3,
                    gpi: 1.265842561688e-5,
                    go: 1.375118229059e-6,
                },
                second: Transistor {
                    gm: 1.333415038252e-2,
                    gpi: 6.394230286271e-5,
                    go: 3.307724661851e-6,
                },
                collector_load: 51_000.,
                feedback: 3.9e6,
                bias_upper: 120_000.,
                bias_lower: 20_000.,
                emitter_load: 820.,
            },
            Self::E7 => Preamp {
                first: Transistor {
                    gm: 1.83672159243438e-2,
                    gpi: 4.32586436904599e-5,
                    go: 4.69930105273075e-6,
                },
                second: Transistor {
                    gm: 4.96258090976694e-3,
                    gpi: 1.12371896696021e-5,
                    go: 1.22072559979872e-6,
                },
                collector_load: 12_000.,
                feedback: 1e6,
                bias_upper: 620_000.,
                bias_lower: 100_000.,
                emitter_load: 2200.,
            },
        }
    }
}

/// Collapse the four ideal tone contacts to the active electrical nodes.
pub fn tone_nodes(tones: u8, nodes: &mut usize) -> [usize; 5] {
    let mut ladder = [0; 5];
    for i in (0..4).rev() {
        ladder[i] = if (tones >> i) & 1 == 1 {
            *nodes += 1;
            *nodes
        } else {
            ladder[i + 1]
        };
    }
    ladder
}

/// Two RC sections and two wound chokes shared by the D6, E7 and Duo.
pub fn tone_ladder<const S: usize, const N: usize, const R: usize>(
    c: &mut SizedCircuit<S, N, R>,
    ladder: [usize; 5],
    treble_coil: usize,
    brilliant_coil: usize,
) {
    let [top, low, mid, high, ground] = ladder;
    c.r(top, low, 30_000.0);
    c.c(top, low, 100e-9);
    c.r(low, mid, 10_000.0);
    c.c(low, mid, 15e-9);
    c.l(mid, treble_coil, 2.0);
    c.r(treble_coil, high, 43.0);
    c.c(mid, high, 4.7e-9);
    c.l(high, brilliant_coil, 0.6);
    c.r(brilliant_coil, ground, 20.0);
}

/// Beyer input transformer, reflected to the secondary. Drive port zero in primary EMF volts.
pub fn transformer<const S: usize, const N: usize, const R: usize>(
    c: &mut SizedCircuit<S, N, R>,
    core: usize,
    blue: usize,
    source_ohms: f64,
) {
    let ratio = 15.0;
    let primary = (source_ohms + 70.0) * ratio * ratio;
    c.r(core, 0, primary);
    c.input(core, ratio / primary);
    c.l(core, 0, 6.0 * ratio * ratio);
    c.r(core, blue, 10_000.0);
}

pub struct Panel {
    board: Board,
    power: bool,
    rate: f64,
    network: CrossfadingFilter<13>,
    centre: f64,
    bridge: f64,
    controls: Option<(Switch, Wiring, u8, f64)>,
}

impl Panel {
    /// Rate in Hz, active tone-tab bit mask and linear volume pot travel (0–1).
    pub fn new(rate: f64, selector: Switch, wiring: Wiring, tones: u8, volume: f64) -> Self {
        Self::for_board(Board::D6, rate, selector, wiring, tones, volume)
    }

    pub fn for_board(
        board: Board,
        rate: f64,
        selector: Switch,
        wiring: Wiring,
        tones: u8,
        volume: f64,
    ) -> Self {
        let mut panel = Self {
            board,
            power: true,
            rate,
            network: CrossfadingFilter::new(rate),
            centre: 0.0,
            bridge: 0.0,
            controls: None,
        };
        panel.set_controls(selector, wiring, tones, volume);
        panel
    }

    /// Reconnect the switches while preserving capacitor and coil histories.
    pub fn set_controls(&mut self, selector: Switch, wiring: Wiring, tones: u8, volume: f64) {
        let controls = (selector, wiring, tones, volume);
        if self.controls == Some(controls) {
            return;
        }
        self.controls = Some(controls);
        let (centre, bridge, source_ohms) = self.board.selector(selector, wiring);
        self.centre = centre;
        self.bridge = bridge;

        // Ladder bottom is ground. An unselected tab shorts its two terminals.
        // On the D6 the ideal battery is AC ground. The E7 rail is the
        // emitter of a BC238 pass transistor biased by a ZPD8.2 zener.
        let rail = if self.board == Board::E7 { 13 } else { 0 };
        let mut nodes = 12.max(rail);
        let stage = self.board.preamp();
        let ladder = tone_nodes(tones, &mut nodes);
        let top = ladder[0];
        let ground = 0;
        let mut c = Circuit::<13>::new(nodes, self.rate);
        let (core, blue, base1, collector1) = (1, 2, 3, 4);
        let (treble_coil, brilliant_coil, coupled, base2) = (5, 6, 7, 8);
        let (emitter, bypass, collector2, pot) = (9, 10, 11, 12);
        // Reflect primary copper, source resistance and magnetizing L to the secondary.
        transformer(&mut c, core, blue, source_ohms);
        c.c(blue, base1, 10e-9);

        stage.first.stamp(&mut c, collector1, base1, ground);
        c.r(collector1, rail, stage.collector_load);
        c.r(collector1, base1, stage.feedback);
        c.c(collector1, base1, 8.2e-12);
        c.c(collector1, top, 220e-9);

        tone_ladder(&mut c, ladder, treble_coil, brilliant_coil);

        c.r(top, coupled, 30_000.0);
        c.c(coupled, base2, 10e-9);
        c.r(base2, rail, stage.bias_upper);
        c.r(base2, ground, stage.bias_lower);
        stage.second.stamp(&mut c, collector2, base2, emitter);
        c.r(emitter, bypass, stage.emitter_load);
        c.r(bypass, ground, 1000.0);
        c.c(bypass, ground, 6.8e-6);
        c.r(collector2, rail, 10_000.0);
        c.c(collector2, pot, 220e-9);

        if self.board == Board::E7 {
            // Reduce the linearized emitter follower to its one-port impedance.
            // The base sees the feed resistor in parallel with the zener's r_d + Rs.
            let rb = 1. / (1. / 1800. + 1. / (5. + 1. / 1.263275755456e-2));
            let gpi = 1.17280879440578e-4;
            let gm = 2.36583194274549e-2;
            let go = 6.06692037478527e-6;
            c.r(rail, ground, 1. / (go + (gm + gpi) / (1. + rb * gpi)));
        }
        c.c(rail, ground, 100e-6);

        // The E7 +log slider has an estimated 10% electrical midpoint.
        // Both boards see the same 1 MΩ DI on the wiper.
        let volume = volume.clamp(0.0, 1.0);
        let volume = match self.board {
            Board::D6 => volume,
            Board::E7 => (81_f64.powf(volume) - 1.) / 80.,
        };
        let bottom = volume * 100_000.0;
        let lower = bottom / (1.0 + bottom / 1e6);
        let total = (1.0 - volume) * 100_000.0 + lower;
        c.r(pot, ground, total);
        let next = self.network.prepare();
        c.compile(pot, next);
        next.scale_output(0, lower / total);
    }

    pub(super) fn select_board(&mut self, board: Board) {
        if self.board != board {
            let (selector, wiring, tones, volume) = self.controls.unwrap();
            self.board = board;
            self.power = true;
            self.controls = None;
            self.network.reset();
            self.set_controls(selector, wiring, tones, volume);
        }
    }

    /// The steady on/off states of the E7 switch. Switching spikes are uncalibrated.
    pub fn set_power(&mut self, on: bool) {
        let powered = on || self.board == Board::D6;
        if self.power != powered {
            self.network.reset();
            self.power = powered;
        }
    }

    pub fn reset(&mut self) {
        self.network.reset();
    }

    /// Open-circuit centre and bridge EMFs in volts. Returns volts at the loaded jack.
    pub fn process(&mut self, pickups: Pickups) -> f64 {
        if !self.power {
            return 0.;
        }
        self.network
            .tick(self.centre * pickups.centre + self.bridge * pickups.bridge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn tone_ladder_tracks_spice_at_bass_midrange_and_treble() {
        let rate = 44_100.0;
        // 1 MΩ jack, 73% linear pot travel, CA, 1 V open-circuit. Targets from d6.cir.
        for (mask, expected_db) in [
            (SOFT, [40.10, 39.78, 24.16]),
            (MEDIUM, [38.61, 51.83, 39.74]),
            (TREBLE, [26.96, 60.47, 48.21]),
            (BRILLIANT, [16.15, 55.26, 58.53]),
            (ALL_TONES, [40.96, 60.47, 58.69]),
        ] {
            let mut panel = Panel::new(rate, Switch::CA, Wiring::DaAdds, mask, 0.73);
            let omega = [100.0, 1600.0, 10_000.0].map(|hz| 2.0 * (PI * hz / rate).atan());
            let mut sum = [(0.0_f64, 0.0_f64); 3];
            for i in 0..32768 {
                let y = panel.process(Pickups {
                    centre: f64::from(i == 0),
                    bridge: 0.0,
                });
                for (p, &angle) in sum.iter_mut().zip(&omega) {
                    let (sin, cos) = (angle * f64::from(i)).sin_cos();
                    p.0 += y * cos;
                    p.1 += y * sin;
                }
            }
            for ((re, im), expected) in sum.into_iter().zip(expected_db) {
                let db = 20.0 * re.hypot(im).log10();
                assert!(
                    (db - expected).abs() < 0.03,
                    "tone tabs {mask}, {db} vs {expected} dB"
                );
            }
        }
    }

    #[test]
    fn muted_and_opposed_positions_are_silent_and_switching_retains_state() {
        let rate = 44_100.0;
        let equal = Pickups {
            centre: 0.001,
            bridge: 0.001,
        };
        for (mode, wiring, mask, volume) in [
            (Switch::CA, Wiring::DaAdds, 0, 1.0),
            (Switch::DB, Wiring::DaAdds, ALL_TONES, 1.0),
            (Switch::DA, Wiring::DaSubtracts, ALL_TONES, 1.0),
            (Switch::DA, Wiring::DaAdds, ALL_TONES, 0.0),
        ] {
            let mut panel = Panel::new(rate, mode, wiring, mask, volume);
            for _ in 0..512 {
                assert_eq!(panel.process(equal), 0.0);
            }
        }
        let mut live = Panel::new(rate, Switch::CA, Wiring::DaAdds, ALL_TONES, 1.0);
        live.process(equal);
        live.set_controls(Switch::DA, Wiring::DaAdds, MEDIUM, 1.0);
        let carry = (0..256)
            .map(|_| live.process(Pickups::default()).abs())
            .sum::<f64>();
        assert!(carry.is_finite() && carry > 0.01);
    }
}
