//! Siemens ELV413 piano input, reconstructed from Wittig (2007) figs. 8 and 10.
use pleasant_dsp::{
    circuit::{CrossfadingFilter, PushPullTriodes, SizedCircuit},
    instrument::Decimator,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    /// One of the twelve master switch positions. Position one grounds the wiper.
    pub master: u8,
    /// Left-pedal movement, from 350 pF at rest to 700 pF at full travel.
    pub swell: f64,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            master: 9,
            swell: 0.,
        }
    }
}
pub const LATENCY: u32 = 24;
pub const INPUT_OHMS: f64 = 1e6;
pub const INPUT_SHUNT_F: f64 = 50e-12;
const RATE_MULTIPLE: usize = 4;
const STATES: usize = 14;
const PRE_GM: f64 = 0.0035;
const PRE_MU: f64 = 25.;
const RE604: PushPullTriodes = PushPullTriodes {
    gm: 0.0025,
    mu: 3.4,
    idle_a: 0.040,
    plate_v: 250.,
    knee_v: 28.,
    parallel: 1.,
    grids: [1, 2],
    plates: [3, 4],
    ports: [1, 2],
};
const STEPS: [f64; 12] = [
    0., 0.02, 0.035, 0.06, 0.10, 0.15, 0.22, 0.32, 0.46, 0.61, 0.80, 1.,
];
type Board = SizedCircuit<STATES, 29, 19>;

pub struct Amplifier {
    rate: f64,
    controls: Controls,
    circuit: CrossfadingFilter<STATES, 5, 7>,
    correction: [f64; 2],
    driver_correction: f64,
    up: Decimator<1>,
    down: Decimator<1>,
}

impl Amplifier {
    pub fn new(rate: f64, controls: Controls) -> Self {
        assert!(rate.is_finite() && rate >= 8000.);
        let anti_alias =
            || Decimator::with_cutoff(RATE_MULTIPLE, rate, LATENCY as usize / 2, 0.48 * rate);
        let mut a = Self {
            rate,
            controls,
            circuit: CrossfadingFilter::new(rate * RATE_MULTIPLE as f64),
            correction: [0.; 2],
            driver_correction: 0.,
            up: anti_alias(),
            down: anti_alias(),
        };
        a.compile();
        a
    }
    pub fn set_controls(&mut self, controls: Controls) {
        let controls = Controls {
            master: controls.master.clamp(1, 12),
            swell: if controls.swell.is_finite() {
                controls.swell.clamp(0., 1.)
            } else {
                0.
            },
        };
        if self.controls != controls {
            self.controls = controls;
            self.compile();
        }
    }
    pub fn reset(&mut self) {
        self.correction = [0.; 2];
        self.driver_correction = 0.;
        self.circuit.reset();
        self.up.reset();
        self.down.reset();
    }
    /// Voltage across the stock 6-ohm speaker at the output transformer secondary.
    #[inline]
    pub fn tick(&mut self, input_v: f64) -> f64 {
        self.process::<true>(input_v)
    }
    /// Small-signal response at the same audio terminals for an impulse measurement.
    pub fn linear(&mut self, input_v: f64) -> f64 {
        self.process::<false>(input_v)
    }
    fn process<const NONLINEAR: bool>(&mut self, input_v: f64) -> f64 {
        let mut speaker = 0.;
        for phase in 0..RATE_MULTIPLE {
            self.up.push([if phase == 0 {
                input_v * RATE_MULTIPLE as f64
            } else {
                0.
            }]);
            let x = self.up.read::<1>()[0];
            let mut sources = [x, 0., 0., 0., 0.];
            if NONLINEAR {
                let base = self.circuit.observe(sources);
                let d = self.circuit.feedthrough();
                let (anode, grids) = driver_currents(base, d, self.driver_correction);
                self.driver_correction = anode;
                sources[3] = anode;
                sources[4] = grids;
                let driven = std::array::from_fn(|i| base[i] + d[i][3] * anode + d[i][4] * grids);
                self.correction = RE604.correction(driven, d, self.correction);
                sources[1] = self.correction[0];
                sources[2] = self.correction[1];
            }
            let y = self.circuit.tick_ports(sources)[0];
            self.down.push([y]);
            if phase == 0 {
                speaker = self.down.read::<1>()[0];
            }
        }
        speaker
    }
    fn compile(&mut self) {
        let mut b = Board::new(28, self.rate * RATE_MULTIPLE as f64);
        // E1 is supplied as loaded terminal volts. The 1 mΩ Norton source is an ideal port approximation.
        let (input, g1, p1, a1, sw, g2, p2, a2, top, wiper, g3, p3, tt_hot, tt_winding, core) =
            (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
        b.r(input, 0, 0.001);
        b.input_port(0, input, 1000.);
        b.c(input, g1, 22e-9);
        b.r(g1, 0, INPUT_OHMS);
        // Each of the first two plates reaches its coupling capacitor through a series resistor.
        for (g, p, a) in [(g1, p1, a1), (g2, p2, a2)] {
            b.gm(p, 0, g, PRE_GM);
            b.r(p, 0, PRE_MU / PRE_GM);
            b.r(p, a, 4700.);
            b.r(a, 0, 68000.);
        }
        b.c(a1, sw, 0.30e-6);
        b.r(sw, 0, 1e6);
        let swell = if self.controls.swell.is_finite() {
            self.controls.swell.clamp(0., 1.)
        } else {
            0.
        };
        b.c(sw, g2, (350. + 350. * swell) * 1e-12);
        b.r(g2, 0, 1e6);
        b.c(g2, 0, 5.87e-9);
        b.c(a2, top, 100e-9);
        let fraction = STEPS[usize::from(self.controls.master.clamp(1, 12) - 1)];
        b.r(top, wiper, (250e3 * (1. - fraction)).max(0.001));
        b.r(wiper, 0, (250e3 * fraction).max(0.001));
        b.c(wiper, g3, 22e-9);
        b.r(g3, 0, 1e6);
        b.gm(p3, 0, g3, PRE_GM);
        b.input_port(3, p3, -1.);
        b.r(p3, 0, PRE_MU / PRE_GM);
        b.r(p3, 0, 68000.);
        b.c(p3, tt_hot, 0.673e-6);
        b.r(tt_hot, tt_winding, 300.);
        // Effective TT leakage and capacitive feed across the winding, plus core losses.
        b.l(tt_winding, core, 2.27);
        b.c(tt_winding, core, 119e-12);
        b.r(tt_winding, core, 1.68e6);
        b.l(core, 0, 9.78);
        b.c(core, 0, 10e-12);
        b.r(core, 0, 780e3);
        let (upper, lower, plate_a, plate_b, half_a, half_b, at, leak, speaker) =
            (16, 17, 18, 19, 20, 21, 22, 23, 24);
        b.transformer([core, 0], [upper, 0], 25, 0.82);
        // The two directly coupled secondary halves have opposite grid voltages.
        b.input_port(4, upper, -1.);
        b.transformer([core, 0], [0, lower], 26, 0.82);
        for (i, (g, p, h)) in [(upper, plate_a, half_a), (lower, plate_b, half_b)]
            .into_iter()
            .enumerate()
        {
            b.gm(p, 0, g, RE604.gm);
            b.r(p, 0, RE604.mu / RE604.gm);
            b.input_port(i + 1, p, -1.);
            b.r(p, h, 100.);
        }
        b.transformer([at, 0], [half_a, 0], 27, 14.4);
        b.transformer([at, 0], [0, half_b], 28, 14.4);
        b.l(at, 0, 0.30);
        b.r(at, leak, 0.30);
        b.l(leak, speaker, 68.9e-6);
        b.c(speaker, 0, 3.01e-6);
        b.r(speaker, 0, 6.);
        // Tube filaments, bias filter returns and transformer centre taps are audio grounds.
        b.compile_ports(
            [speaker, upper, lower, plate_a, plate_b, g3, p3],
            self.circuit.prepare(),
        );
    }
}

/// Rö3 is resistance-loaded. Its 2 mA idle through 68 kΩ puts a 120 V
/// plate on a 256 V supply. Positive RE604 grids draw current through TT.
/// Solve the driver's plate current together with that reflected grid load.
fn driver_currents(base: [f64; 7], d: &[[f64; 5]; 7], initial: f64) -> (f64, f64) {
    const IQ: f64 = 0.002;
    const VA: f64 = 120.;
    const KNEE: f64 = 18.;
    const GRID_BIAS: f64 = 45.;
    const GRID_R: f64 = 330.;
    let mut q = initial;
    let grid_load = |q: f64| {
        let open = base[1] + d[1][3] * q;
        if open.abs() <= GRID_BIAS {
            (0., 0.)
        } else {
            (
                (open - open.signum() * GRID_BIAS) / (GRID_R - d[1][4]),
                d[1][3] / (GRID_R - d[1][4]),
            )
        }
    };
    let (mut lower, mut upper) = (-64., 64.);
    for _ in 0..18 {
        let (i_grid, d_grid) = grid_load(q);
        let plate = base[6] + d[6][3] * q + d[6][4] * i_grid;
        let drive = base[5] + plate / PRE_MU;
        let space = (1. + drive * PRE_GM / (1.5 * IQ)).max(0.);
        let collect = ((VA + plate) / KNEE).max(0.);
        let (current, slope) = if space < collect {
            (IQ * space * space.sqrt(), PRE_GM * space.sqrt() / PRE_MU)
        } else {
            (
                IQ * collect * collect.sqrt(),
                1.5 * IQ * collect.sqrt() / KNEE,
            )
        };
        let residual = q - (current - IQ - PRE_GM * drive);
        if residual.abs() < 1e-11 {
            return (q, i_grid);
        }
        if residual > 0. {
            upper = q;
        } else {
            lower = q;
        }
        let jac = 1. - (slope - PRE_GM / PRE_MU) * (d[6][3] + d[6][4] * d_grid);
        let next = q - residual / jac;
        q = if next >= lower && next <= upper {
            next
        } else {
            0.5 * (upper + lower)
        };
    }
    (q, grid_load(q).0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rms(amp: &mut Amplifier, input: f64) -> f64 {
        amp.reset();
        let mut power = 0.;
        for i in 0..4800 {
            let x = input * 2_f64.sqrt() * (std::f64::consts::TAU * i as f64 / 48.).sin();
            let y = amp.tick(x);
            assert!(y.is_finite());
            if i >= 2400 {
                power += y * y;
            }
        }
        (power / 2400.).sqrt()
    }

    #[test]
    fn twelve_steps_swell_and_rated_power_at_the_real_terminals() {
        let mut amp = Amplifier::new(48000., Controls::default());
        let mut previous = 0.;
        for master in 1..=12 {
            amp.set_controls(Controls { master, swell: 1. });
            let out = rms(&mut amp, 0.01);
            if master == 1 {
                assert!(out < 1e-6);
            }
            assert!(out > previous);
            previous = out;
        }
        amp.set_controls(Controls {
            master: 9,
            swell: 1.,
        });
        let full = rms(&mut amp, 0.1);
        assert!((full.powi(2) / 6. - 3.).abs() < 0.1);
        amp.set_controls(Controls {
            master: 9,
            swell: 0.,
        });
        let rest = rms(&mut amp, 0.1);
        assert!(full > 1.5 * rest && full < 2.1 * rest);
        amp.reset();
        assert!((0..256).all(|_| amp.tick(0.) == 0.));
    }
}

#[cfg(test)]
mod overload {
    use super::*;
    #[test]
    fn maximum_master_and_swell_load_the_driver_on_loud_multitone_inputs() {
        for rate in [44100., 48000., 96000.] {
            let mut amp = Amplifier::new(
                rate,
                Controls {
                    master: 12,
                    swell: 1.,
                },
            );
            let mut peak = 0_f64;
            for n in 0..(rate * 0.09) as usize {
                let t = n as f64 / rate;
                let input = [
                    329.63, 493.88, 659.26, 830.61, 987.77, 1244.51, 1479.98, 1661.22,
                ]
                .into_iter()
                .map(|f| (std::f64::consts::TAU * t * f).sin())
                .sum::<f64>();
                let out = amp.tick(input);
                assert!(out.is_finite());
                peak = peak.max(out.abs());
            }
            assert!((5.5..10.).contains(&peak), "{rate} Hz, {peak} V peak");
        }
    }
}
