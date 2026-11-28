//! Dry small-signal network of Hohner drawing 30 002 167 D at full swell.
use crate::{RATE, SUBSTEPS};
use pleasant_dsp::circuit::{Circuit, CrossfadingFilter, Filter};

const STEP_RATE: f64 = RATE as f64 * SUBSTEPS as f64;
const PORT_LOAD: f64 = 10e6;

fn output<const S: usize>(node: usize, step_rate: f64, load: f64) -> Filter<S> {
    let mut filter = Filter::default();
    tune(&mut filter, node, step_rate, 1.0, 3e6, load);
    filter
}

fn tune<const S: usize>(
    filter: &mut Filter<S>,
    node: usize,
    step_rate: f64,
    swell: f64,
    cell_ohms: f64,
    load: f64,
) {
    // Refer the primary circuit to the secondary, with Vsecondary / Vprimary = n.
    // Magnetizing inductance, winding loss, stray C and ratio await a transformer sweep.
    let n = 0.10_f64;
    // Referred coordinates: 1 = n*(electrode - bias), 2 = n*(ground - bias).
    // The winding voltage is node 1 relative to ground as seen by the secondary.
    // 3 winding loss/inductance junction, 4 base, 5 emitter, 6 collector;
    // 7 LDR in, 8 swell wiper/100 mV jack, 9 EQ branch, 10 volume-to-base,
    // 11 base, 12 emitter, 13 collector, 14 high jack, 15 middle jack.
    let top = if swell == 1.0 { 8 } else { 16 };
    let cembalet = S == 11;
    let low = if cembalet { top.max(15) + 1 } else { 8 };
    let mut c = Circuit::<S>::new(top.max(low).max(15), step_rate);
    c.input(1, -1.0 / n);
    c.input(2, 1.0 / n);
    c.r(2, 0, 20e6 * n * n);
    c.c(2, 0, 0.1e-6 / (n * n));
    c.c(1, 2, 180e-12 / (n * n));
    c.r(1, 0, 2e6 * n * n);
    c.r(1, 3, 1600. * n * n);
    c.l(3, 0, 220. * n * n);
    c.c(1, 4, 25e-6);
    c.r(4, 0, 100e3);
    c.r(4, 0, 3.9e3);
    c.r(4, 5, 70.0 / 0.014);
    c.gm(6, 5, 4, 0.014);
    c.r(5, 0, 1000.0);
    c.c(5, 0, 100e-6);
    c.r(6, 0, 24e3);
    c.c(6, 7, 0.33e-6);
    // ORP60 with the lamp off. The 51k and amplitude trimmer bypass the cell.
    c.r(7, top, cell_ohms);
    c.r(7, top, 51e3 + 50e3);
    let lower = (250e3 * swell.powf(2.4)).max(0.001);
    if top != 8 {
        c.r(top, 8, (250e3 - lower).max(0.001));
    }
    c.r(8, 0, lower);
    if cembalet {
        // Drawing 650720. The vertical leg from the wiper crosses the ground bus
        // without a junction and feeds the 100 mV jack through a series 100 kΩ.
        c.r(8, low, 100e3);
        c.c(top, 0, 5e-9);
    }
    c.r(low, 0, PORT_LOAD);
    c.c(low, 9, 10e-9);
    c.r(9, 0, 100e3);
    c.r(8, 10, 51e3);
    c.c(10, 11, 25e-6);
    c.r(11, 0, 100e3);
    c.r(11, 0, 10e3);
    c.r(11, 12, 54.0 / 0.04);
    c.gm(13, 12, 11, 0.04);
    c.r(12, 0, 1000.0);
    c.c(12, 0, 100e-6);
    c.r(13, 0, 6800.0);
    c.c(13, 14, 25e-6);
    c.r(14, 0, load);
    c.r(14, 15, if cembalet { 600e3 } else { 200e3 });
    c.r(15, 0, 200e3);
    c.r(15, 0, PORT_LOAD);
    c.compile(node, filter);
}

pub struct Panel {
    old_capacitance: Option<f64>,
    low: Filter<10>,
    high: Filter<10>,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            old_capacitance: None,
            low: output(8, STEP_RATE, PORT_LOAD),
            high: output(14, STEP_RATE, PORT_LOAD),
        }
    }
}

impl Panel {
    /// Returns three jack voltages and the Norton drive current in amperes.
    pub fn tick(&mut self, capacitance: f64) -> ([f64; 3], f64) {
        // The manual gives 300 V across the electrodes. 400 V is the supply before the feed.
        let current = 300.0
            * (capacitance
                - self
                    .old_capacitance
                    .replace(capacitance)
                    .unwrap_or(capacitance))
            * STEP_RATE;
        (self.drive(current), current)
    }

    pub fn drive(&mut self, current: f64) -> [f64; 3] {
        let high = self.high.tick(current);
        let middle = high / (2.0 + 200e3 / PORT_LOAD);
        [self.low.tick(current), middle, high]
    }
}

/// AC107/AC125 jack with ORP60 intensity branch and the swell-pot wiper.
/// Capacitor and inductor companion histories survive each resistance update.
pub(crate) struct NPanel<const S: usize = 10> {
    jack: CrossfadingFilter<S>,
    load: f64,
    rate: f64,
    interval: u32,
    countdown: u32,
    target: f64,
    swell: f64,
    vibrato: bool,
    phase: f64,
    light: f64,
}
impl NPanel<10> {
    pub fn new(rate: f64) -> Self {
        Self::with_load(rate, PORT_LOAD)
    }
}
impl<const S: usize> NPanel<S> {
    pub fn with_load(rate: f64, load: f64) -> Self {
        let interval = ((rate / 960.).round() as u32).max(1);
        let mut jack = CrossfadingFilter::with_length(interval);
        *jack.prepare() = output(14, rate, load);
        Self {
            jack,
            load,
            rate,
            interval,
            countdown: 0,
            target: 1.0,
            swell: 1.0,
            vibrato: false,
            phase: 0.,
            light: 0.,
        }
    }
    pub fn set(&mut self, swell: f32, vibrato: bool) {
        self.target = f64::from(swell.clamp(0., 1.));
        self.vibrato = vibrato;
    }
    pub fn reset(&mut self) {
        self.phase = 0.;
        self.light = 0.;
        self.swell = self.target;
        self.countdown = 0;
        self.jack.reset();
        tune(
            self.jack.prepare(),
            14,
            self.rate,
            self.swell,
            3e6,
            self.load,
        );
    }
    pub fn tick(&mut self, input: f64) -> f64 {
        if self.countdown == 0 {
            let dt = f64::from(self.interval) / self.rate;
            let old_light = self.light;
            let old_swell = self.swell;
            // Astable switches the 18 V lamp. Hohner gives approximately six
            // cycles/s; lamp heating and cooling plus ORP60 response are estimates.
            self.phase = (self.phase + 6.0 * dt).fract();
            let lit = f64::from(self.vibrato && self.phase < 0.5);
            let tau = if lit > self.light { 0.022 } else { 0.046 };
            self.light += (lit - self.light) * (-(-dt / tau).exp_m1());
            self.swell += (self.target - self.swell) * (-(-dt / 0.008).exp_m1());
            if (self.swell - self.target).abs() < 1e-5 {
                self.swell = self.target;
            }
            if self.light < 1e-6 {
                self.light = 0.;
            }
            if self.swell != old_swell || self.light != old_light {
                let ohms = 1.0 / (1.0 / 3e6 + self.light.powf(1.5) / 2200.0);
                tune(
                    self.jack.prepare(),
                    14,
                    self.rate,
                    self.swell,
                    ohms,
                    self.load,
                );
            }
            self.countdown = self.interval;
        }
        self.countdown -= 1;
        self.jack.tick(input)
    }
}

/// Full-swell 650720 diagnostic returns the low, middle and high sockets
/// simultaneously, including their mutual loading, for circuit comparison.
pub fn cembalet_n_jacks(rate: f64, high_load: f64) -> impl FnMut(f64) -> [f64; 3] {
    let mut low = output::<11>(16, rate, high_load);
    let mut high = output::<11>(14, rate, high_load);
    move |current| {
        let h = high.tick(current);
        [low.tick(current), h / (4. + 600e3 / PORT_LOAD), h]
    }
}

/// Sixty alternating coils feeding a passive transformer; the factory drawing
/// specifies 2 nF across the jack. Measured winding impedances are unavailable.
pub fn magnetic_jack(rate: f64, load: f64) -> Filter<3> {
    let turns = 4.0_f64; // secondary to primary
    let copper = 60.0 * 18.0 * turns.powi(2);
    let mut c = Circuit::<3>::new(2, rate);
    c.input(1, turns / copper);
    c.r(1, 0, copper);
    c.l(1, 2, 60.0 * 0.0011 * turns.powi(2));
    c.l(2, 0, 5.0 * turns.powi(2));
    c.c(2, 0, 2e-9);
    c.r(2, 0, load);
    let mut jack = Filter::default();
    c.compile(2, &mut jack);
    jack
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    #[test]
    fn swell_and_lamp_act_on_the_loaded_output() {
        fn spectrum(swell: f32, modulation: bool) -> [f64; 4] {
            let rate = 48_000;
            let mut panel = NPanel::new(f64::from(rate));
            panel.set(swell, modulation);
            let mut fourier = [(0.0, 0.0); 4];
            for i in 0..rate * 2 {
                let t = f64::from(i) / f64::from(rate);
                let out = panel.tick(1e-8 * (TAU * 440. * t).sin());
                if i >= rate {
                    for (z, hz) in fourier.iter_mut().zip([440., 434., 446., 445.]) {
                        z.0 += out * (TAU * hz * t).cos();
                        z.1 += out * (TAU * hz * t).sin();
                    }
                }
            }
            fourier.map(|(x, y)| 2.0 * x.hypot(y) / f64::from(rate))
        }
        let dry = spectrum(1., false);
        let wet = spectrum(1., true);
        let half = spectrum(0.5, false);
        let closed = spectrum(0., false);
        assert!(dry[1] < 1e-5 * dry[0] && wet[1] > 0.015 * wet[0] && wet[2] > 0.015 * wet[0]);
        assert!(wet[3] < 0.025 * wet[2]);
        assert!(half[0] > 0.01 * dry[0] && half[0] < 0.65 * dry[0]);
        assert!(closed[0] < 1e-6 * dry[0]);
    }
}
