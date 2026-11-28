//! Optional MK8 FX board. Block types, order, delay range, and spillover are
//! documented. The internal gains, bucket count, RC constants and ranges are estimates.
use std::f64::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub fitted: bool,
    pub compressor: bool,
    pub amount: f64,
    pub makeup_db: f64,
    pub ratio: f64,
    pub chorus: bool,
    pub chorus_rate_hz: f64,
    pub chorus_depth: f64,
    pub chorus_ms: f64,
    pub phaser: bool,
    pub phaser_rate_hz: f64,
    pub phaser_depth: f64,
    pub phaser_hz: f64,
    pub delay: bool,
    /// Panel dial spans 60–800 ms. Tap tempo can extend beyond the dial.
    pub delay_ms: f64,
    pub feedback: f64,
    pub mix: f64,
    pub spread: f64,
    pub jitter: f64,
    pub drive_out_db: f64,
    pub fx_send_db: f64,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            fitted: false,
            compressor: false,
            amount: 0.5,
            makeup_db: 0.,
            ratio: 4.,
            chorus: false,
            chorus_rate_hz: 3.24,
            chorus_depth: 0.45,
            chorus_ms: 5.8,
            phaser: false,
            phaser_rate_hz: 0.35,
            phaser_depth: 0.65,
            phaser_hz: 420.,
            delay: false,
            delay_ms: 300.,
            feedback: 0.35,
            mix: 0.35,
            spread: 0.,
            jitter: 0.,
            drive_out_db: 0.,
            fx_send_db: 0.,
        }
    }
}
impl Controls {
    fn bounded(mut self, previous: Self) -> Self {
        let clip =
            |v: f64, old: f64, lo: f64, hi: f64| if v.is_finite() { v.clamp(lo, hi) } else { old };
        macro_rules! bound { ($($field:ident: $lo:expr, $hi:expr);+ $(;)?) => { $(
            self.$field = clip(self.$field, previous.$field, $lo, $hi);
        )+ }; }
        bound!(amount:0.,1.; makeup_db:0.,24.; ratio:1.,20.;
            chorus_rate_hz:0.05,8.; chorus_depth:0.,1.; chorus_ms:4.,25.;
            phaser_rate_hz:0.05,8.; phaser_depth:0.,1.; phaser_hz:120.,2400.;
            delay_ms:60.,2000.; feedback:0.,1.; mix:0.,1.; spread:0.,1.; jitter:0.,1.;
            drive_out_db:-12.,12.; fx_send_db:-12.,12.);
        self
    }
}

const SILENT: f64 = 1e-11;

/// A variable transfer clock advances charge packets through a fixed number of
/// buckets. Changing that clock also changes the bandwidth and the time taken
/// for charge already in the device to reach the output.
struct BucketLine {
    charge: Box<[f64]>,
    cell: usize,
    charged: usize,
    phase: f64,
    increment: f64,
    filter_gain: f64,
    input: [f64; 2],
    output: [f64; 2],
    previous_input: f64,
    hold: f64,
}
impl BucketLine {
    fn new(cells: usize) -> Self {
        Self {
            charge: vec![0.; cells].into_boxed_slice(),
            cell: 0,
            charged: 0,
            phase: 0.,
            increment: 1.,
            filter_gain: 1.,
            input: [0.; 2],
            output: [0.; 2],
            previous_input: 0.,
            hold: 0.,
        }
    }
    fn clear(&mut self) {
        self.charge.fill(0.);
        self.cell = 0;
        self.charged = 0;
        self.phase = 0.;
        self.input = [0.; 2];
        self.output = [0.; 2];
        self.previous_input = 0.;
        self.hold = 0.;
    }
    fn tune(&mut self, seconds: f64, rate: f64) {
        let transfer_hz = self.charge.len() as f64 / seconds;
        self.increment = transfer_hz / rate;
        let cutoff = (transfer_hz * 0.38).min(8800.).min(0.4 * rate);
        self.filter_gain = 1. - (-TAU * cutoff / rate).exp();
    }
    fn active(&self) -> bool {
        self.charged != 0
            || self.hold.abs() > SILENT
            || self
                .input
                .iter()
                .chain(&self.output)
                .any(|x| x.abs() > SILENT)
    }
    fn tick(&mut self, x: f64) -> f64 {
        let k = self.filter_gain;
        self.input[0] += k * (x - self.input[0]);
        self.input[1] += k * (self.input[0] - self.input[1]);
        self.phase += self.increment;
        while self.phase >= 1. {
            self.phase -= 1.;
            let fraction = 1. - self.phase / self.increment;
            let packet = self.previous_input + fraction * (self.input[1] - self.previous_input);
            // Soft overload across a finite effective bucket bias window.
            let packet = packet / (1. + (packet / 1.6).powi(2)).sqrt();
            self.hold = self.charge[self.cell];
            self.charged -= usize::from(self.hold.abs() > SILENT);
            self.charged += usize::from(packet.abs() > SILENT);
            self.charge[self.cell] = packet;
            self.cell += 1;
            if self.cell == self.charge.len() {
                self.cell = 0;
            }
        }
        self.previous_input = self.input[1];
        self.output[0] += k * (self.hold - self.output[0]);
        self.output[1] += k * (self.output[0] - self.output[1]);
        self.output[1]
    }
}
#[derive(Default)]
struct FourStage {
    memory: [f64; 4],
}
impl FourStage {
    fn tick(&mut self, mut x: f64, coef: f64) -> f64 {
        for state in &mut self.memory {
            let y = coef * x + *state;
            *state = x - coef * y;
            x = y;
        }
        x
    }
}

pub(crate) struct Board {
    c: Controls,
    target: Controls,
    ramp: pleasant_dsp::interpolation::ControlRamp,
    rate: f64,
    envelope: f64,
    attack: f64,
    release: f64,
    threshold: f64,
    makeup: f64,
    exponent: f64,
    pub(crate) drive_out: f64,
    send: f64,
    chorus: [BucketLine; 2],
    delay: [BucketLine; 2],
    phaser: [FourStage; 2],
    phase: [f64; 2],
    phase_coef: [f64; 2],
    tick: u8,
    jitter_clock: f64,
    jitter_value: f64,
    random: u32,
    tail: bool,
}
impl Board {
    pub(crate) fn new(rate: f64) -> Self {
        Self {
            c: Controls::default(),
            target: Controls::default(),
            ramp: pleasant_dsp::interpolation::ControlRamp::new(rate),
            rate,
            envelope: 0.,
            attack: 1. - (-1. / (0.008 * rate)).exp(),
            release: 1. - (-1. / (0.13 * rate)).exp(),
            threshold: 0.1,
            makeup: 1.,
            exponent: 0.75,
            drive_out: 1.,
            send: 1.,
            chorus: std::array::from_fn(|_| BucketLine::new(512)),
            delay: std::array::from_fn(|_| BucketLine::new(4096)),
            phaser: Default::default(),
            phase: [0.; 2],
            phase_coef: [0.; 2],
            tick: 0,
            jitter_clock: 0.,
            jitter_value: 0.,
            random: 0x6c001238,
            tail: false,
        }
    }
    pub(crate) fn set(&mut self, c: Controls) {
        if self.target == c {
            return;
        }
        let c = c.bounded(self.target);
        if !c.fitted && self.c.fitted {
            self.clear();
        }
        if self.c.chorus && !c.chorus {
            for line in &mut self.chorus {
                line.clear();
            }
        }
        if self.c.phaser && !c.phaser {
            self.phaser = Default::default();
        }
        if !c.compressor {
            self.envelope = 0.;
        }
        self.target = c;
        self.c.fitted = c.fitted;
        self.c.compressor = c.compressor;
        self.c.chorus = c.chorus;
        self.c.phaser = c.phaser;
        self.c.delay = c.delay;
        if self.ramp.retarget() {
            self.apply(c);
        }
        self.tick = 0;
    }
    fn apply(&mut self, c: Controls) {
        self.threshold = 0.45 * 10f64.powf(-1.5 * c.amount);
        self.makeup = 10f64.powf(c.makeup_db / 20.);
        self.exponent = 1. - 1. / c.ratio;
        self.drive_out = if c.fitted {
            10f64.powf(c.drive_out_db / 20.)
        } else {
            1.
        };
        self.send = 10f64.powf(c.fx_send_db / 20.);
        self.c = c;
    }
    pub(crate) fn clear(&mut self) {
        self.ramp.reset();
        self.apply(self.target);
        self.envelope = 0.;
        self.phase = [0.; 2];
        self.phaser = Default::default();
        self.jitter_clock = 0.;
        self.jitter_value = 0.;
        self.random = 0x6c001238;
        self.tick = 0;
        self.tail = false;
        for line in self.chorus.iter_mut().chain(&mut self.delay) {
            line.clear();
        }
    }
    pub(crate) fn compress(&mut self, x: f64) -> f64 {
        if let Some(step) = self.ramp.step() {
            let mut c = self.c;
            c.amount += step * (self.target.amount - c.amount);
            c.makeup_db += step * (self.target.makeup_db - c.makeup_db);
            c.ratio += step * (self.target.ratio - c.ratio);
            c.chorus_rate_hz += step * (self.target.chorus_rate_hz - c.chorus_rate_hz);
            c.chorus_depth += step * (self.target.chorus_depth - c.chorus_depth);
            c.chorus_ms += step * (self.target.chorus_ms - c.chorus_ms);
            c.phaser_rate_hz += step * (self.target.phaser_rate_hz - c.phaser_rate_hz);
            c.phaser_depth += step * (self.target.phaser_depth - c.phaser_depth);
            c.phaser_hz += step * (self.target.phaser_hz - c.phaser_hz);
            c.delay_ms += step * (self.target.delay_ms - c.delay_ms);
            c.feedback += step * (self.target.feedback - c.feedback);
            c.mix += step * (self.target.mix - c.mix);
            c.spread += step * (self.target.spread - c.spread);
            c.jitter += step * (self.target.jitter - c.jitter);
            c.drive_out_db += step * (self.target.drive_out_db - c.drive_out_db);
            c.fx_send_db += step * (self.target.fx_send_db - c.fx_send_db);
            self.apply(c);
        }
        if !self.c.fitted || !self.c.compressor {
            return x;
        }
        let level = x.abs();
        let speed = if level > self.envelope {
            self.attack
        } else {
            self.release
        };
        self.envelope += speed * (level - self.envelope);
        let over = (self.envelope / self.threshold).max(1.);
        x * self.makeup * over.powf(-self.exponent)
    }
    pub(crate) fn tail_active(&self) -> bool {
        self.c.fitted
            && (self.tail || (self.c.chorus && self.chorus.iter().any(BucketLine::active)))
    }
    fn update_modulation(&mut self) {
        let c = self.c;
        let dt = 16. / self.rate;
        if c.chorus {
            let modulator = (TAU * self.phase[0]).sin();
            for (line, polarity) in self.chorus.iter_mut().zip([1., -1.]) {
                line.tune(
                    c.chorus_ms * 0.001 * (1. + 0.35 * c.chorus_depth * polarity * modulator),
                    self.rate,
                );
            }
            self.phase[0] = (self.phase[0] + c.chorus_rate_hz * dt).fract();
        }
        if c.phaser {
            let (sine, cosine) = (TAU * self.phase[1]).sin_cos();
            let excursion = 2.4 * c.phaser_depth;
            for (coef, bend) in self
                .phase_coef
                .iter_mut()
                .zip([sine * excursion, cosine * excursion])
            {
                let f = (c.phaser_hz * 2_f64.powf(bend)).min(self.rate * 0.42);
                let g = (PI * f / self.rate).tan();
                *coef = (g - 1.) / (g + 1.);
            }
            self.phase[1] = (self.phase[1] + c.phaser_rate_hz * dt).fract();
        }
        if c.delay || self.tail {
            self.jitter_clock += dt * (1. + 16. * c.jitter);
            if self.jitter_clock >= 1. {
                self.jitter_clock -= 1.;
                self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
                self.jitter_value = 2. * f64::from(self.random) / f64::from(u32::MAX) - 1.;
            }
            let time = c.delay_ms * 0.001 * (1. + 0.12 * c.jitter.powi(2) * self.jitter_value);
            for (line, polarity) in self.delay.iter_mut().zip([1., -1.]) {
                line.tune((time + 0.020 * c.spread * polarity).max(0.025), self.rate);
            }
        }
    }
    pub(crate) fn effects(&mut self, x: f64) -> [f64; 2] {
        let c = self.c;
        if !c.fitted {
            return [x; 2];
        }
        if self.tick == 0 {
            self.update_modulation();
        }
        self.tick = (self.tick + 1) & 15;
        let mut out = [self.send * x; 2];
        if c.chorus {
            // Two production performances reveal opposite wet polarities.
            for ((s, line), sign) in out.iter_mut().zip(&mut self.chorus).zip([-1., 1.]) {
                *s = (*s + sign * 0.62 * line.tick(*s)) / 1.62;
            }
        }
        if c.phaser {
            // One mono feed into the swept network, with stereo dry paths preserved.
            let mono = (out[0] + out[1]) * 0.5;
            for ((s, filter), coef) in out.iter_mut().zip(&mut self.phaser).zip(self.phase_coef) {
                *s = 0.5 * (*s + filter.tick(mono, coef));
            }
        }
        if c.delay || self.tail {
            for (s, line) in out.iter_mut().zip(&mut self.delay) {
                let input = if c.delay { *s } else { 0. };
                let wet = -line.tick(input - 1.10 * c.feedback * line.output[1]);
                *s = if c.delay { *s * (1. - c.mix) } else { *s } + wet * c.mix;
            }
            self.tail = self.delay.iter().any(BucketLine::active);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panel::{Controls as Preamp, Panel};
    fn fx_panel(rate: f64, c: Controls) -> Panel {
        let mut panel = Panel::new(rate, Preamp::default());
        panel.set_fx_controls(Controls { fitted: true, ..c });
        panel
    }
    #[test]
    fn standard_board_and_fitted_board_with_four_bypasses_are_identical() {
        let all = Controls {
            compressor: true,
            chorus: true,
            phaser: true,
            delay: true,
            makeup_db: 24.,
            drive_out_db: 12.,
            fx_send_db: 12.,
            ..Controls::default()
        };
        for rate in [44100., 48000., 96000.] {
            let mut stock = Panel::new(rate, Preamp::default());
            let mut absent = Panel::new(rate, Preamp::default());
            absent.set_fx_controls(all);
            let mut bypassed = fx_panel(rate, Controls::default());
            for i in 0..5000 {
                let x = (TAU * 317. * i as f64 / rate).sin() * 0.1;
                let expected = stock.tick(x);
                for outputs in [absent.tick(x), bypassed.tick(x)] {
                    assert_eq!(
                        [expected.send_v, expected.left_v, expected.right_v],
                        [outputs.send_v, outputs.left_v, outputs.right_v]
                    );
                }
            }
        }
    }
    #[test]
    fn vca_ratio_and_four_stage_notch_frequencies_follow_controls() {
        let rate = 48000.;
        let c = Controls {
            fitted: true,
            compressor: true,
            amount: 1.,
            ratio: 4.,
            ..Controls::default()
        };
        let mut b = Board::new(rate);
        b.set(c);
        for _ in 0..24000 {
            b.compress(0.1);
        }
        let quiet = b.compress(0.1);
        for _ in 0..24000 {
            b.compress(1.);
        }
        let loud = b.compress(1.);
        assert!((20. * (loud / quiet).log10() - 5.).abs() < 0.001);
        for f in [650. * (PI / 8.).tan(), 650. * (3. * PI / 8.).tan(), 650.] {
            let mut p = Panel::new(rate, Preamp::default());
            p.set_fx_controls(Controls {
                fitted: true,
                phaser: true,
                phaser_depth: 0.,
                phaser_hz: 650.,
                ..Controls::default()
            });
            let mut energy = 0.;
            for i in 0..24000 {
                let v = p.process_return((TAU * f * i as f64 / rate).sin() * 0.1)[0];
                if i >= 12000 {
                    energy += v * v;
                }
            }
            if f == 650. {
                assert!(energy > 55.);
            } else {
                assert!(energy < 0.003);
            }
        }
    }
    #[test]
    fn delay_spillover_spread_and_feedback_follow_real_clock_after_bypass() {
        for rate in [44100., 48000., 96000.] {
            let c = Controls {
                fitted: true,
                delay: true,
                delay_ms: 177.6,
                spread: 0.33,
                feedback: 0.6,
                mix: 1.,
                ..Controls::default()
            };
            let mut board = fx_panel(rate, c);
            let mut moments = [[0.; 2]; 2];
            let mut levels = [[0_f64; 2]; 2];
            for i in 0..(rate * 0.40) as usize {
                let t = i as f64 / rate;
                if i == (rate * 0.025) as usize {
                    board.set_fx_controls(Controls { delay: false, ..c });
                }
                let pulse = if t < 0.003 && t > 0.001 { 0.1 } else { 0. };
                let stereo = board.process_return(pulse);
                if t > 0.04 {
                    let echo = usize::from(t > 0.27);
                    for ch in 0..2 {
                        if stereo[ch].abs() > levels[echo][ch].abs() {
                            levels[echo][ch] = stereo[ch];
                            moments[echo][ch] = t;
                        }
                    }
                }
            }
            for (ch, first) in [0.1842, 0.171].into_iter().enumerate() {
                assert!((moments[0][ch] - first - 0.002).abs() < 0.0016);
                assert!((moments[1][ch] - 2. * first - 0.002).abs() < 0.0018);
                assert!(levels[0][ch] < -0.07 && levels[1][ch] > 0.04);
            }
            board.reset();
            assert!(!board.effects_sounding());
        }
    }
    #[test]
    fn chorus_modulates_independent_clocks_and_fitted_aux_tap_is_dry() {
        let c = Controls {
            fitted: true,
            chorus: true,
            chorus_rate_hz: 2.,
            chorus_depth: 1.,
            ..Controls::default()
        };
        let mut board = fx_panel(48000., c);
        let mut plain = Panel::new(48000., Preamp::default());
        let mut side = 0.;
        for i in 0..48000 {
            let input = (TAU * 311. * i as f64 / 48000.).sin() * 0.1;
            let full = board.tick(input);
            assert_eq!(full.send_v, plain.tick(input).send_v);
            side += (full.left_v - full.right_v).powi(2);
        }
        assert!(side > 10.);
        board.set_fx_controls(Controls { chorus: false, ..c });
        for i in 0..1024 {
            let input = (TAU * 153. * i as f64 / 48000.).sin() * 0.1;
            let x = board.tick(input);
            let y = plain.tick(input);
            assert_eq!([x.left_v, x.right_v], [y.left_v, y.right_v]);
        }
    }
}
