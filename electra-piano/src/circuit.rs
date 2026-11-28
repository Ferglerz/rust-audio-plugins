//! Hohner 680209. All winding-network impedances are referred to the secondary.
use pleasant_dsp::circuit::{CrossfadingFilter, SizedCircuit};

pub const GROUP_COUNTS: [usize; 7] = [10, 10, 10, 10, 10, 10, 12];
const STATES: usize = 17;
const RATIO: f64 = 7.0;
const COIL_R: f64 = 8.;
const COIL_L: f64 = 0.0005;
type Network = SizedCircuit<STATES, 42, 24>;

/// Travel of the three pots and seven shunts, normalized to 0..1.
/// `volume` is physical log-pot travel. The other controls are linear.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub group_trim: [f64; 7],
    pub basic_volume: f64,
    pub volume: f64,
    pub soft_pedal: bool,
    pub soft_depth: f64,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            group_trim: [1.; 7],
            basic_volume: 0.85,
            volume: 0.75,
            soft_pedal: false,
            soft_depth: 1.,
        }
    }
}

fn pot(x: f64) -> f64 {
    if x.is_finite() {
        x.clamp(0., 1.)
    } else {
        0.
    }
}

/// The assumed middle group boundaries divide the unlabelled 50 notes equally.
pub fn group(note: u8) -> Option<usize> {
    (29..=100)
        .contains(&note)
        .then(|| usize::from((note - 29) / 10).min(6))
}

pub fn winding_sign(note: u8) -> f64 {
    if (i16::from(note) - 29) % 2 == 0 {
        1.
    } else {
        -1.
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Output {
    pub auxiliary_v: f64,
    pub speaker_v: f64,
}

pub struct Panel {
    filter: CrossfadingFilter<STATES, 7, 2>,
    controls: Controls,
    rate: f64,
}

// gm, gpi, go from the .op in research/680209.cir; germanium and bias are estimates.
const HYBRID_PI: [[f64; 3]; 7] = [
    [0.01191285962061, 0.00005054326081951, 0.000002905104934451],
    [0.03428299486540, 0.0001464555615029, 0.000008417913150323],
    [0.3050658877284, 0.001297220387011, 0.00007456110583150],
    [0.2777344794603, 0.002529698179621, 0.00007851652013184],
    [0.2804518429277, 0.002554555718614, 0.00007928804595150],
    [1.352726838129, 0.01753764997420, 0.0004536098610575],
    [1.466297104332, 0.01901093159390, 0.0004917161678793],
];

impl Panel {
    pub fn new(rate: f64, controls: Controls) -> Self {
        let mut panel = Self {
            filter: CrossfadingFilter::new(rate),
            rate,
            controls,
        };
        panel.compile();
        panel
    }

    pub fn set_controls(&mut self, controls: Controls) {
        if controls != self.controls {
            self.controls = controls;
            self.compile();
        }
    }

    pub fn reset(&mut self) {
        self.filter.reset();
    }

    pub fn tick(&mut self, group_emf: [f64; 7]) -> Output {
        let [auxiliary_v, speaker_v] = self.filter.tick_ports(group_emf);
        Output {
            auxiliary_v,
            speaker_v,
        }
    }

    pub(crate) fn at_rest(&self) -> bool {
        self.filter.history_below(1e-11)
    }

    fn compile(&mut self) {
        let p = self.controls;
        let mut n = Network::new(37, self.rate);
        let n2 = RATIO * RATIO;
        // Group g lies between g and g+1, with a current injection in parallel
        // with winding resistance and the inductance in series above that pair.
        for (g, count) in GROUP_COUNTS.into_iter().enumerate() {
            let internal = g + 8;
            let r = n2 * count as f64 * COIL_R;
            n.r(g, internal, r);
            n.input_port(g, internal, RATIO / r);
            n.input_port(g, g, -RATIO / r);
            n.l(internal, g + 1, n2 * count as f64 * COIL_L);
            n.r(g, g + 1, n2 * (100. * pot(p.group_trim[g])).max(0.001));
        }
        let (basic_top, basic_tap, secondary, jack) = (15, 16, 17, 18);
        let soft = if p.soft_pedal {
            pot(p.soft_depth) * 5000.
        } else {
            0.
        };
        n.r(7, basic_top, n2 * soft.max(0.001));
        let basic = pot(p.basic_volume);
        n.r(basic_top, basic_tap, n2 * ((1. - basic) * 5000.).max(0.001));
        n.r(basic_tap, 0, n2 * (basic * 5000.).max(0.001));
        n.r(basic_tap, secondary, 35. * n2 + 250.);
        n.l(secondary, 0, 0.8 * n2);
        let vol = (100_f64.powf(pot(p.volume)) - 1.) / 99.;
        n.r(secondary, jack, ((1. - vol) * 100_000.).max(0.001));
        n.r(jack, 0, (vol * 100_000.).max(0.001));
        n.r(jack, 0, 1e6); // External amplifier/interface input
        let (pre, b1, c1, e1, b2, c2, e2) = (19, 20, 21, 22, 23, 24, 25);
        let (b3, e3, drive, bt, mid, top, e6, b5, bottom, e7, spk, z) =
            (26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37);
        n.r(0, pre, 6800.);
        n.c(pre, 0, 500e-6);
        n.c(jack, b1, 1e-6);
        n.r(pre, b1, 2e6);
        n.r(b1, 0, 120e3);
        n.r(pre, c1, 30e3);
        n.r(e1, 0, 2400.);
        n.c(c1, b2, 1e-6);
        n.r(pre, b2, 2e6);
        n.r(b2, 0, 75e3);
        n.r(pre, c2, 12e3);
        n.r(e2, 0, 51.);
        n.c(c2, b3, 0.47e-6);
        n.r(0, bt, 510.);
        n.c(bt, mid, 10e-6);
        n.r(bt, drive, 1000.);
        n.r(mid, b3, 67798.29566104);
        n.r(b3, 0, 12e3);
        n.c(b3, drive, 300e-12);
        n.r(e3, 0, 200.);
        n.r(drive, mid, 200.);
        n.r(0, top, 51.);
        n.r(0, e6, 0.5);
        n.c(e3, b5, 1e-6);
        n.r(0, z, 2400.);
        n.r(0, z, 3. + 1. / 0.1781874502485);
        n.r(z, b5, 1000.);
        n.r(b5, 0, 104.43288765);
        n.r(b5, 0, 1000.);
        n.r(mid, bottom, 51.);
        n.r(mid, e7, 0.5);
        n.c(mid, spk, 2500e-6);
        n.r(spk, 0, 4.);
        n.r(spk, e2, 1000.);
        n.c(spk, e2, 300e-12);
        for ((c, b, e), [gm, gpi, go]) in [
            (c1, b1, e1),
            (c2, b2, e2),
            (drive, b3, e3),
            (top, drive, mid),
            (bottom, b5, 0),
            (mid, top, e6),
            (0, bottom, e7),
        ]
        .into_iter()
        .zip(HYBRID_PI)
        {
            n.gm(c, e, b, gm);
            n.r(b, e, 1. / gpi);
            n.r(c, e, 1. / go);
        }
        n.compile_ports([jack, spk], self.filter.prepare());
    }
}
