//! Storytone keyboard path, Radio-Craft Feb 1940 fig. 1 (p. 469) and fig. 3.
use pleasant_dsp::circuit::{CrossfadingFilter, PushPullTriodes, SizedCircuit};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub optimum: f64,
    pub manual: f64,
    pub swell: f64,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            optimum: 0.70,
            manual: 0.70,
            swell: 0.0,
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Output {
    pub grid_v: f64,
    pub speaker_v: f64,
    pub auxiliary_v: f64,
}

const STATES: usize = 20;
const MU: f64 = 4.2;
const IQ: f64 = 0.040;
const GM: f64 = 0.0046;
const N2: f64 = 0.85; // estimated T2 half-secondary / primary turns
const N3: f64 = 4.79; // effective fit to the period 15.8 W / 2% rating; T3 half-primary / secondary turns
const T1: f64 = 35.;
const GRID: usize = 0;
const SPEAKER: usize = 1;
const GI: [usize; 2] = [2, 3];
const PL: [usize; 2] = [4, 5];
const OUTPUT_PAIR: PushPullTriodes = PushPullTriodes {
    gm: GM,
    mu: MU,
    idle_a: IQ,
    plate_v: 300.,
    knee_v: 30.,
    parallel: 2.,
    grids: GI,
    plates: PL,
    ports: [2, 3],
};
type Network = SizedCircuit<STATES, 37, 24>;

pub struct Amplifier {
    filter: CrossfadingFilter<STATES, 4, 6>,
    controls: Controls,
    rate: f64,
    correction: [f64; 2],
}

impl Amplifier {
    pub fn new(rate: f64, controls: Controls) -> Self {
        let mut a = Self {
            filter: CrossfadingFilter::new(rate),
            controls,
            rate,
            correction: [0.; 2],
        };
        a.compile();
        a
    }
    pub fn set_controls(&mut self, controls: Controls) {
        if self.controls != controls {
            self.controls = controls;
            self.compile();
        }
    }
    pub fn reset(&mut self) {
        self.filter.reset();
        self.correction = [0.; 2];
    }
    pub fn at_rest(&self) -> bool {
        self.filter.history_below(1e-10)
    }
    pub fn linear(&mut self, pickups: [f64; 2]) -> Output {
        let nodes = self.filter.tick_ports([pickups[0], pickups[1], 0., 0.]);
        Self::output(nodes)
    }
    pub fn tick(&mut self, pickups: [f64; 2]) -> Output {
        let base = self.filter.observe([pickups[0], pickups[1], 0., 0.]);
        let correction = OUTPUT_PAIR.correction(base, self.filter.feedthrough(), self.correction);
        self.correction = correction;
        let nodes = self
            .filter
            .tick_ports([pickups[0], pickups[1], correction[0], correction[1]]);
        Self::output(nodes)
    }
    fn output(nodes: [f64; 6]) -> Output {
        let speaker_v = nodes[SPEAKER];
        Output {
            grid_v: nodes[GRID],
            speaker_v,
            auxiliary_v: speaker_v * 220. / 4120.,
        }
    }
    fn compile(&mut self) {
        let p = self.controls;
        let pot = |v: f64| if v.is_finite() { v.clamp(0., 1.) } else { 0. };
        let mut n = Network::new(36, self.rate);
        // The 9/9-ohm primaries share a core. Reflect the separate series
        // pickup runs into its secondary, preserving each source impedance.
        let (core, sec, grid, kat, plate, screen, pre) = (5, 6, 7, 8, 9, 10, 11);
        for i in 0..2 {
            let rev = T1 * T1;
            let r = (8.0 + [0.36, 0.49][i]) * rev;
            let (series, coil) = (1 + i * 2, 2 + i * 2);
            n.r(series, 0, r);
            n.input_port(i, series, T1 / r);
            n.l(series, coil, 0.012 * rev);
            n.r(coil, core, 0.001);
        }
        n.l(core, 0, 22.);
        n.r(core, sec, 670.);
        let opt = pot(p.optimum);
        n.r(sec, grid, ((1. - opt) * 250000.).max(0.001));
        n.r(grid, 0, (opt * 250000.).max(0.001));
        n.r(kat, 0, 1250.);
        n.c(kat, 0, 20e-6);
        n.r(plate, pre, 220e3);
        // Screen supply from decoupled 56k/1.2M chain. 6J7 suppressor tied
        // to cathode, 33-ohm heater balancing legs excluded from audio model.
        n.r(pre, 0, 56e3);
        n.c(pre, 0, 0.25e-6);
        n.r(screen, 0, 1.2e6);
        n.c(screen, 0, 0.1e-6);
        n.gm(plate, kat, grid, 0.00122);
        n.r(plate, kat, 800e3);
        n.gm(plate, kat, screen, 0.000028);
        n.r(screen, kat, 160e3);
        let (ac, manual_top, manual_w, swell_w, driver_g, driver_k, driver_p, driver_core) =
            (12, 13, 14, 15, 16, 17, 18, 19);
        n.c(plate, ac, 0.25e-6);
        n.r(ac, manual_top, 220e3);
        let man = pot(p.manual);
        n.r(manual_top, manual_w, ((1. - man) * 250e3).max(0.001));
        n.r(manual_w, 0, (man * 250e3).max(0.001));
        let swell = pot(p.swell);
        n.r(manual_w, swell_w, (swell * 250e3).max(0.001));
        n.r(swell_w, ac, ((1. - swell) * 250e3).max(0.001));
        n.r(swell_w, driver_g, 470e3);
        n.r(driver_g, 0, 360e3);
        // With the external input unplugged, R11 ends at an open jack.
        // R13 still loads the piano through the external tone rheostat and C6.
        n.r(driver_g, 36, 390e3 + 125e3);
        n.c(36, 0, 2.5e-9);
        n.r(driver_k, 0, 820.);
        n.c(driver_k, 0, 20e-6);
        n.gm(driver_p, driver_k, driver_g, 0.002);
        n.r(driver_p, driver_k, 10000.);
        n.r(driver_p, driver_core, 2200.);
        n.l(driver_core, 0, 40.);
        let (top, bot, g1, g2, p1, p2, h1, h2, spk, sec3) =
            (20, 21, 22, 23, 24, 25, 26, 27, 28, 29);
        n.r(35, 0, 100e3);
        n.c(35, 0, 16e-6);
        n.transformer([driver_core, 0], [top, 35], 30, N2);
        n.transformer([driver_core, 0], [35, bot], 31, N2);
        n.r(top, g1, 800.);
        n.r(bot, g2, 800.);
        // R14/C18 snubber across T2 secondary and the four shunt grid caps.
        n.r(g2, 32, 270e3);
        n.c(32, g1, 0.01e-6);
        n.c(g1, 0, 300e-12);
        n.c(g2, 0, 360e-12);
        for (i, (pg, gr, half)) in [(p1, g1, h1), (p2, g2, h2)].into_iter().enumerate() {
            n.gm(pg, 0, gr, 2. * GM);
            n.r(pg, 0, MU / (2. * GM));
            n.input_port(i + 2, pg, -1.);
            n.r(pg, half, 63.5);
        }
        // Series 127-ohm center-tapped primary, 0.6-ohm secondary copper.
        n.transformer([sec3, 0], [h1, 0], 33, N3);
        n.transformer([sec3, 0], [0, h2], 34, N3);
        n.l(sec3, 0, 0.18);
        n.r(sec3, spk, 0.6);
        n.c(p1, p2, 0.002e-6);
        n.c(p2, 0, 0.005e-6);
        n.r(spk, 0, 15.);
        n.r(spk, 0, 3900. + 220.);
        // Finite HT decoupling: two output conductors are referenced to the
        // filtered rail as AC ground. Small winding capacitances are estimates.
        n.c(sec, 0, 100e-12);
        n.c(driver_core, 0, 80e-12);
        n.c(sec3, 0, 0.40e-9);
        n.compile_ports([grid, spk, g1, g2, p1, p2], self.filter.prepare());
    }
}
