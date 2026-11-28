//! Shared regulated supply, four audio stages and loaded recording/combined jacks.
use crate::Controls;
use clavinet_d6::panel::{tone_ladder, tone_nodes, transformer, Board, Transistor};
use pleasant_dsp::circuit::{CrossfadingFilter, SizedCircuit};

type Net = SizedCircuit<19, 34, 22>;
pub struct Panel {
    rate: f64,
    network: CrossfadingFilter<19, 2, 3>,
    weights: [f64; 2],
    powered: bool,
    open_outputs: [bool; 2],
}

fn node(count: &mut usize) -> usize {
    *count += 1;
    *count
}

struct Slider {
    top: usize,
    wiper: usize,
    fraction: f64,
}

impl Slider {
    fn new(count: &mut usize, travel: f64, shorted: bool) -> Self {
        let top = if shorted { 0 } else { node(count) };
        let fraction = (81_f64.powf(travel.clamp(0., 1.)) - 1.) / 80.;
        let wiper = if fraction == 0. {
            0
        } else if fraction == 1. {
            top
        } else {
            node(count)
        };
        Self {
            top,
            wiper,
            fraction,
        }
    }
    fn stamp(&self, c: &mut Net) {
        c.r(self.top, self.wiper, 100e3 * (1. - self.fraction));
        c.r(self.wiper, 0, 100e3 * self.fraction);
        c.r(self.wiper, 0, 1e6);
    }
}

impl Panel {
    pub fn new(rate: f64, controls: Controls) -> Self {
        let mut p = Self {
            rate,
            weights: [1., 0.],
            powered: true,
            open_outputs: [true; 2],
            network: CrossfadingFilter::new(rate),
        };
        p.set(controls);
        p
    }
    pub fn set(&mut self, controls: Controls) {
        let (strings, reeds) = controls.split.banks();
        let (centre, bridge, source) = Board::E7.selector(controls.pickup, controls.wiring);
        self.weights = [centre, bridge];
        // The pedal grounds the top of one slider through a preset rocker. With
        // both rockers forward the foot contact is disconnected from both paths.
        let mut count = 21;
        self.open_outputs = controls.open_outputs();
        let [c_enabled, p_enabled] = self.open_outputs;
        let clav = Slider::new(&mut count, controls.clavinet_volume, !c_enabled);
        let pianet = Slider::new(&mut count, controls.pianet_volume, !p_enabled);
        let ladder = tone_nodes(controls.tones, &mut count);
        let mut c = Net::new(count, self.rate);
        let (core, blue, b1, c1, coil1, coil2) = (1, 2, 3, 4, 5, 6);
        let (b2, e2, bypass, c2, rail) = (7, 8, 9, 10, 11);
        let (winding, reed, bp, cp, ep) = (12, 13, 14, 15, 16);
        let (sum, bm, cm, em, mix) = (17, 18, 19, 20, 21);

        transformer(&mut c, core, blue, source * f64::from(strings.keys()) / 60.);
        c.c(blue, b1, 15e-9);
        Transistor {
            gm: 1.83290353153304e-2,
            gpi: 4.31697027142798e-5,
            go: 4.68963915866753e-6,
        }
        .stamp(&mut c, c1, b1, 0);
        c.r(c1, rail, 12e3);
        c.r(c1, b1, 1e6);
        c.c(c1, b1, 8.2e-12);
        c.c(c1, ladder[0], 220e-9);
        tone_ladder(&mut c, ladder, coil1, coil2);
        c.c(ladder[0], b2, 15e-9);
        c.r(b2, rail, 620e3);
        c.r(b2, 0, 100e3);
        Transistor {
            gm: 4.94134120944334e-3,
            gpi: 1.11898257483505e-5,
            go: 1.21558033145104e-6,
        }
        .stamp(&mut c, c2, b2, e2);
        c.r(e2, bypass, 2.2e3);
        c.r(bypass, 0, 1e3);
        c.c(bypass, 0, 4.7e-6);
        c.r(c2, rail, 10e3);
        c.c(c2, clav.top, 220e-9);
        clav.stamp(&mut c);

        let coils = f64::from(reeds.keys());
        let copper = 18. * coils;
        c.input_port(1, winding, 1. / copper);
        c.r(winding, 0, copper);
        c.l(winding, reed, 0.0011 * coils);
        c.c(reed, bp, 0.1e-6);
        Transistor {
            gm: 8.51732360412516e-3,
            gpi: 1.98462519368193e-5,
            go: 2.15595086021305e-6,
        }
        .stamp(&mut c, cp, bp, ep);
        c.r(bp, 0, 470e3);
        c.r(bp, cp, 1e6);
        c.r(ep, 0, 820.);
        c.r(cp, rail, 20e3);
        c.c(cp, 0, 10e-9);
        c.c(cp, pianet.top, 0.1e-6);
        pianet.stamp(&mut c);

        c.r(clav.wiper, sum, 470e3);
        c.r(pianet.wiper, sum, 470e3);
        c.c(sum, bm, 0.1e-6);
        Transistor {
            gm: 8.54630610203788e-3,
            gpi: 1.99456370240997e-5,
            go: 2.16674732575693e-6,
        }
        .stamp(&mut c, cm, bm, em);
        c.r(bm, 0, 620e3);
        c.r(bm, cm, 1e6);
        c.r(em, 0, 1.5e3);
        c.r(cm, rail, 20e3);
        c.c(cm, mix, 0.1e-6);
        c.r(mix, 0, 1e6);

        let rb = 1. / (1. / 1800. + 1. / (5. + 1. / 1.25788191611037e-2));
        let gpi = 2.01965831969572e-4;
        let gm = 4.07413430565702e-2;
        let go = 1.04476589126203e-5;
        c.r(rail, 0, 1. / (go + (gm + gpi) / (1. + rb * gpi)));
        c.c(rail, 0, 100e-6);
        c.compile_ports([clav.wiper, pianet.wiper, mix], self.network.prepare());
        if controls.power != self.powered {
            self.reset();
        }
        self.powered = controls.power;
    }
    pub fn reset(&mut self) {
        self.network.reset();
    }
    /// Open-circuit EMFs selected at the three coil banks, in volts.
    /// Returns the two recording terminals (Clavinet then Pianet), and the mixed jack.
    #[inline]
    pub fn tick(&mut self, centre: f64, bridge: f64, reed: f64) -> [f64; 3] {
        if !self.powered {
            return [0.; 3];
        }
        let mut output = self
            .network
            .tick_ports([centre * self.weights[0] + bridge * self.weights[1], reed]);
        for (jack, open) in output.iter_mut().zip(self.open_outputs) {
            if !open {
                *jack = 0.;
            }
        }
        output
    }
}
