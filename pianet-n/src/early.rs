//! Directly coupled Pianet C (28.2.63) and mains Pianet L (E71) circuits.
use pleasant_dsp::circuit::{Circuit, CrossfadingFilter, Filter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Monitor {
    External,
    Phones,
    Speakers,
}

/// Electrical outputs in volts. The L speaker reading is across both series
/// drivers. Its motion voltage is the combined back EMF at the two voice coils.
#[derive(Default, Clone, Copy)]
pub struct Taps {
    pub line: f64,
    pub headphones: f64,
    pub speaker: f64,
    pub motion: f64,
}

const C_STATES: usize = 10;
const L_STATES: usize = 11;

fn collector_feedback<const S: usize>(
    c: &mut Circuit<S>,
    base: usize,
    collector: usize,
    ohms: f64,
    gm: f64,
    beta: f64,
) {
    c.r(base, collector, ohms);
    c.r(base, 0, beta / gm);
    c.gm(collector, 0, base, gm);
}

fn electrode<const S: usize>(c: &mut Circuit<S>, bias_cap: f64) {
    // 1 = electrode; 2 = filtered 280 VAC rectifier supply; 3 = AC107 base.
    // The reservoir before the 2 MΩ resistor is treated as an AC ground.
    c.input(1, -1.);
    c.r(2, 0, 2e6);
    c.c(2, 0, bias_cap);
    c.r(2, 1, 20e6);
    c.c(1, 0, 180e-12); // estimated total stationary bank and wiring
    c.c(1, 3, 22e-9);
}

/// Fixed-load linearized C board with the marked AC107/125/126 stages and two DIN ports.
fn c_board(
    f: &mut Filter<C_STATES, 1, 2>,
    rate: f64,
    knee: f64,
    cell: f64,
    line_load: f64,
    phones: f64,
) {
    // 4 c1, 5 AC107/125 rail, 6 optical out, 7 knee in, 8 knee out,
    // 9 line, 10 b2, 11 c2, 12 b3/trimmer wiper, 13 AC126 rail, 14 c3,
    // 15 e3, 16 headphones.
    let mut c = Circuit::<C_STATES>::new(16, rate);
    electrode(&mut c, 22e-9);
    collector_feedback(&mut c, 3, 4, 1e6, 0.011, 70.);
    c.r(4, 5, 39e3);
    c.r(5, 0, 6.8e3);
    c.r(5, 0, 2.4e3);
    c.c(5, 0, 50e-6);
    c.r(4, 6, 250e3);
    c.r(4, 6, cell);
    c.c(6, 7, 10e-6);
    // Both ends of the wiper contact are tied on the dated drawing.
    c.r(7, 8, (250e3 * (1. - knee).powf(2.4)).max(0.01));
    c.c(8, 9, 33e-9);
    c.r(9, 0, line_load);
    c.r(8, 10, 39e3);
    collector_feedback(&mut c, 10, 11, 100e3, 0.026, 60.);
    c.r(11, 5, 10e3);
    c.c(11, 12, 25e-6);
    c.r(13, 0, 50.);
    c.c(13, 0, 50e-6);
    c.r(13, 12, 2.4e3 + 5000.);
    c.r(12, 0, 5000.); // 10 kΩ internal bias trimmer at half travel
    c.r(12, 15, 52. / 0.11);
    c.gm(14, 15, 12, 0.11);
    c.r(14, 13, 1000.);
    c.r(15, 0, 50.);
    c.c(15, 0, 100e-6); // emitter bypass value unreadable
    c.c(14, 16, 100e-6);
    c.r(16, 0, phones);
    c.compile_ports([9, 16], f);
}

/// Mains L preamp and driver. Returns the external DIN and the AD130 base signal.
fn l_board(f: &mut Filter<L_STATES, 1, 2>, rate: f64, knee: f64, line_load: f64) {
    // 4 c1, 5–7 bypassed supply ladder, 8 knee input, 9 b2, 10 c2,
    // 11 OC308 base, 12 OC308 collector, 13 AD130 base, 14 line.
    let mut c = Circuit::<L_STATES>::new(14, rate);
    electrode(&mut c, 0.1e-6);
    c.c(3, 0, 10e-9);
    collector_feedback(&mut c, 3, 4, 100e3, 0.010, 65.);
    c.r(4, 5, 39e3);
    c.r(5, 6, 500.);
    c.r(6, 7, 500.);
    c.r(7, 0, 120.);
    for node in 5..=7 {
        c.c(node, 0, 50e-6);
    }
    c.c(4, 8, 5e-6);
    c.r(8, 9, (1e6 * (1. - knee)).max(0.01));
    collector_feedback(&mut c, 9, 10, 50e3, 0.04, 60.);
    c.r(10, 6, 2.4e3);
    c.c(10, 14, 22e-9);
    c.r(14, 0, line_load);
    c.c(10, 11, 50e-6);
    collector_feedback(&mut c, 11, 12, 2.4e3, 0.12, 40.);
    c.r(12, 7, 500.);
    c.c(12, 13, 100e-6);
    c.r(13, 7, 1000.);
    c.r(13, 0, 45. / 4.0);
    c.compile_ports([14, 13], f);
}

struct Speaker {
    load: Filter<3, 1, 2>,
    direct: f64,
}
impl Speaker {
    fn new(rate: f64) -> Self {
        // Two matched 5 Ω nominal drivers. A coil's reflected mechanical
        // impedance is Bl²/(Rms + s*Mms + 1/(s*Cms)).
        let mut c = Circuit::<3>::new(3, rate);
        let bl = 1.7_f64;
        c.input(1, 1.);
        c.r(1, 0, 180.); // finite AD130 output resistance
        c.r(1, 2, 2. * 4.1);
        c.l(2, 3, 2. * 0.00016);
        c.r(3, 0, 2. * bl.powi(2) / 1.1);
        c.c(3, 0, 0.0038 / (2. * bl.powi(2)));
        c.l(3, 0, 2. * bl.powi(2) * 0.00025);
        let mut load = Filter::default();
        c.compile_ports([1, 3], &mut load);
        let direct = load.clone().tick_ports([1.])[0];
        Self { load, direct }
    }
    fn tick(&mut self, base: f64) -> [f64; 2] {
        // Single-ended transistor, emitter at ground and collector returned
        // through both loudspeakers. Check current cutoff and remaining Vce.
        let bias = 0.34;
        let demand = (-4. * base).max(-bias);
        let trial = self.load.clone().tick_ports([demand])[0];
        let limit = 14.6 - bias * 8.2 - 0.20;
        let current = demand - (trial - limit).max(0.) / self.direct;
        self.load.tick_ports([current])
    }
}

/// A knee swell is a two-terminal rheostat in these boards. Both loaded outputs
/// remain available during control movement. The optical circuit is specific to C.
pub struct CPanel {
    filter: CrossfadingFilter<C_STATES, 1, 2>,
    rate: f64,
    line: f64,
    phones: f64,
    interval: u32,
    count: u32,
    knee: f64,
    target: f64,
    phase: f64,
    light: f64,
    vibrato: bool,
}
impl CPanel {
    pub fn new(rate: f64, line_load: f64, phones: f64) -> Self {
        let mut p = Self {
            filter: CrossfadingFilter::with_length((rate / 960.).round() as u32),
            rate,
            line: line_load,
            phones,
            interval: (rate / 960.).round() as u32,
            count: 0,
            knee: 1.,
            target: 1.,
            phase: 0.,
            light: 1.,
            vibrato: false,
        };
        p.reset();
        p
    }
    pub fn set(&mut self, knee: f32, vibrato: bool) {
        self.target = f64::from(knee.clamp(0., 1.));
        self.vibrato = vibrato;
    }
    pub fn reset(&mut self) {
        self.count = 0;
        self.phase = 0.;
        self.light = 1.;
        self.knee = self.target;
        self.filter.reset();
        c_board(
            self.filter.prepare(),
            self.rate,
            self.knee,
            resistance(self.light),
            self.line,
            self.phones,
        );
    }
    pub fn tick(&mut self, current: f64) -> Taps {
        if self.count == 0 {
            let dt = f64::from(self.interval) / self.rate;
            let old = (self.knee, self.light);
            self.knee = approach(self.knee, self.target, dt, 0.008);
            // 25 µF / 2.4 kΩ and 100 µF / (500 Ω + speed trimmer).
            let t1 = std::f64::consts::LN_2 * 25e-6 * 2400.;
            let t2 = std::f64::consts::LN_2 * 100e-6 * (500. + 1300.);
            self.phase = (self.phase + dt / (t1 + t2)).fract();
            // Grounding the right AC125 base stops the astable. The
            // left base is still pulled to its supply and the lamp stays on.
            let target = f64::from(!self.vibrato || self.phase < t1 / (t1 + t2));
            self.light = approach(
                self.light,
                target,
                dt,
                if target > self.light { 0.022 } else { 0.046 },
            );
            if old != (self.knee, self.light) {
                let cell = resistance(self.light);
                c_board(
                    self.filter.prepare(),
                    self.rate,
                    self.knee,
                    cell,
                    self.line,
                    self.phones,
                );
            }
            self.count = self.interval;
        }
        self.count -= 1;
        let [line, headphones] = self.filter.tick_ports([current]);
        Taps {
            line,
            headphones,
            ..Taps::default()
        }
    }
}

fn resistance(light: f64) -> f64 {
    1. / (1. / 3e6 + light.powf(1.5) / 2200.)
}

fn approach(value: f64, target: f64, dt: f64, tau: f64) -> f64 {
    let v = value + (target - value) * (-(-dt / tau).exp_m1());
    if (v - target).abs() < 1e-6 {
        target
    } else {
        v
    }
}

pub struct LPanel {
    filter: CrossfadingFilter<L_STATES, 1, 2>,
    speaker: Speaker,
    rate: f64,
    line: f64,
    interval: u32,
    count: u32,
    knee: f64,
    target: f64,
    power: f64,
    settling: f64,
}
impl LPanel {
    pub fn new(rate: f64, line_load: f64) -> Self {
        let mut p = Self {
            filter: CrossfadingFilter::with_length((rate / 960.).round() as u32),
            speaker: Speaker::new(rate),
            rate,
            line: line_load,
            interval: (rate / 960.).round() as u32,
            count: 0,
            knee: 1.,
            target: 1.,
            power: 1.,
            settling: -(-1. / (rate * 0.008)).exp_m1(),
        };
        p.reset();
        p
    }
    pub fn set(&mut self, knee: f32) {
        self.target = f64::from(knee.clamp(0., 1.));
    }
    pub fn reset(&mut self) {
        self.count = 0;
        self.knee = self.target;
        self.power = f64::from(self.target > 0.);
        self.filter.reset();
        self.speaker.load.reset();
        l_board(self.filter.prepare(), self.rate, self.knee, self.line);
    }
    pub fn tick(&mut self, current: f64) -> Taps {
        if self.count == 0 {
            let old = self.knee;
            self.knee = approach(
                old,
                self.target,
                f64::from(self.interval) / self.rate,
                0.008,
            );
            if old != self.knee {
                l_board(self.filter.prepare(), self.rate, self.knee, self.line);
            }
            self.count = self.interval;
        }
        self.count -= 1;
        // The mains L manual explicitly links the rear knee stop and power switch.
        self.power += (f64::from(self.target > 0.) - self.power) * self.settling;
        let [line, base] = self.filter.tick_ports([current * self.power]);
        let [speaker, motion] = self.speaker.tick(base);
        Taps {
            line: line * self.power,
            speaker: speaker * self.power,
            motion: motion * self.power,
            ..Taps::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_knee_switches_only_the_mains_l_off() {
        let sr = 48_000.;
        let mut c = CPanel::new(sr, 100e3, 200.);
        c.set(0., false);
        c.reset();
        let mut l = LPanel::new(sr, 100e3);
        l.set(0.);
        l.reset();
        let mut c_peak = 0f64;
        for i in 0..2000 {
            let input = 1e-8 * (i as f64 * 0.03).sin();
            let taps = l.tick(input);
            assert!(taps.line == 0. && taps.speaker == 0.);
            c_peak = c_peak.max(c.tick(input).line.abs());
        }
        assert!(c_peak > 1e-6);
        l.set(1.);
        let mut l_peak = 0f64;
        for i in 0..6000 {
            l_peak = l_peak.max(l.tick(1e-8 * (i as f64 * 0.03).sin()).speaker.abs());
        }
        assert!(l_peak > 1e-3);
    }
}
