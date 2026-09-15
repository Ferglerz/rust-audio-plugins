use crate::band::{Band, Shape};
use std::f64::consts::PI;

pub fn db_gain(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}
pub fn gain_db(x: f64) -> f64 {
    20.0 * x.max(1e-12).log10()
}
#[derive(Clone, Copy, Debug)]
pub struct Coeff {
    pub b: [f64; 3],
    pub a: [f64; 2],
}
impl Coeff {
    pub fn make(shape: Shape, freq: f64, gain: f64, q: f64, sr: f64) -> Self {
        let w = 2.0 * PI * freq.clamp(5.0, sr * 0.45) / sr;
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * q.clamp(0.15, 18.0));
        let a = 10.0_f64.powf(gain / 40.0);
        let (b0, b1, b2, a0, a1, a2) = match shape {
            Shape::Bell => (
                1.0 + alpha * a,
                -2.0 * c,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * c,
                1.0 - alpha / a,
            ),
            Shape::LowCut => (
                (1.0 + c) / 2.0,
                -(1.0 + c),
                (1.0 + c) / 2.0,
                1.0 + alpha,
                -2.0 * c,
                1.0 - alpha,
            ),
            Shape::HighCut => (
                (1.0 - c) / 2.0,
                1.0 - c,
                (1.0 - c) / 2.0,
                1.0 + alpha,
                -2.0 * c,
                1.0 - alpha,
            ),
            Shape::Notch => (1.0, -2.0 * c, 1.0, 1.0 + alpha, -2.0 * c, 1.0 - alpha),
            Shape::LowShelf | Shape::HighShelf => {
                let beta = 2.0 * a.sqrt() * alpha;
                if shape == Shape::LowShelf {
                    (
                        a * ((a + 1.0) - (a - 1.0) * c + beta),
                        2.0 * a * ((a - 1.0) - (a + 1.0) * c),
                        a * ((a + 1.0) - (a - 1.0) * c - beta),
                        (a + 1.0) + (a - 1.0) * c + beta,
                        -2.0 * ((a - 1.0) + (a + 1.0) * c),
                        (a + 1.0) + (a - 1.0) * c - beta,
                    )
                } else {
                    (
                        a * ((a + 1.0) + (a - 1.0) * c + beta),
                        -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
                        a * ((a + 1.0) + (a - 1.0) * c - beta),
                        (a + 1.0) - (a - 1.0) * c + beta,
                        2.0 * ((a - 1.0) - (a + 1.0) * c),
                        (a + 1.0) - (a - 1.0) * c - beta,
                    )
                }
            }
        };
        Self {
            b: [b0 / a0, b1 / a0, b2 / a0],
            a: [a1 / a0, a2 / a0],
        }
    }
    pub fn bandpass(freq: f64, q: f64, sr: f64) -> Self {
        let w = 2.0 * PI * freq.clamp(5.0, sr * 0.45) / sr;
        let alpha = w.sin() / (2.0 * q);
        Self {
            b: [alpha / (1.0 + alpha), 0.0, -alpha / (1.0 + alpha)],
            a: [
                -2.0 * w.cos() / (1.0 + alpha),
                (1.0 - alpha) / (1.0 + alpha),
            ],
        }
    }
    pub fn response(self, f: f64, sr: f64) -> f64 {
        let w = 2.0 * PI * f / sr;
        let re = self.b[0] + self.b[1] * w.cos() + self.b[2] * (2.0 * w).cos();
        let im = -self.b[1] * w.sin() - self.b[2] * (2.0 * w).sin();
        let ar = 1.0 + self.a[0] * w.cos() + self.a[1] * (2.0 * w).cos();
        let ai = -self.a[0] * w.sin() - self.a[1] * (2.0 * w).sin();
        10.0 * ((re * re + im * im).max(1e-24) / (ar * ar + ai * ai).max(1e-24)).log10()
    }
}
#[derive(Clone, Copy, Default)]
pub struct Filter {
    z1: f64,
    z2: f64,
}
impl Filter {
    pub fn tick(&mut self, x: f64, c: Coeff) -> f64 {
        let y = c.b[0] * x + self.z1;
        self.z1 = c.b[1] * x - c.a[0] * y + self.z2;
        self.z2 = c.b[2] * x - c.a[1] * y;
        if y.is_finite() {
            y
        } else {
            *self = Self::default();
            0.0
        }
    }
}
#[derive(Clone)]
pub struct BandRuntime {
    pub band: Band,
    filters: [Filter; 2],
    detectors: [Filter; 2],
    coeff: Coeff,
    detector: Coeff,
    env: f64,
    pub reduction: f64,
    current_gain: f64,
    current_freq: f64,
    current_q: f64,
    attack: f64,
    release: f64,
    phase: usize,
}
impl BandRuntime {
    pub fn new(mut b: Band, sr: f64) -> Self {
        b.sanitize();
        Self {
            filters: [Filter::default(); 2],
            detectors: [Filter::default(); 2],
            coeff: Coeff::make(b.shape, b.freq, b.gain, b.q, sr),
            detector: Coeff::bandpass(b.freq, b.q, sr),
            env: 0.0,
            reduction: 0.0,
            current_gain: b.gain,
            current_freq: b.freq,
            current_q: b.q,
            attack: (-1.0 / (sr * b.attack * 0.001)).exp(),
            release: (-1.0 / (sr * b.release * 0.001)).exp(),
            phase: 0,
            band: b,
        }
    }
    pub fn inherit(&mut self, old: &Self) {
        if self.band.shape == old.band.shape {
            self.filters = old.filters;
            self.detectors = old.detectors;
            self.env = old.env;
            self.reduction = old.reduction;
            self.current_gain = old.current_gain;
            self.current_freq = old.current_freq;
            self.current_q = old.current_q;
            self.coeff = old.coeff;
        }
    }
    pub fn reset(&mut self) {
        self.filters = [Filter::default(); 2];
        self.detectors = [Filter::default(); 2];
        self.env = 0.0;
        self.reduction = 0.0;
    }
    pub fn tick(&mut self, mut x: [f64; 2], sr: f64) -> [f64; 2] {
        if !self.band.enabled {
            return x;
        }
        let mut peak: f64 = 0.0;
        if self.band.dynamic && self.band.shape.has_gain() {
            for (i, v) in x.iter().enumerate() {
                peak = peak.max(self.detectors[i].tick(*v, self.detector).abs());
            }
            let k = if peak > self.env {
                self.attack
            } else {
                self.release
            };
            self.env = k * self.env + (1.0 - k) * peak;
            self.reduction = ((gain_db(self.env) - self.band.threshold).max(0.0)
                * (1.0 - 1.0 / self.band.ratio))
                .min(self.band.range);
        } else {
            self.reduction = 0.0;
        }
        if self.phase == 0 {
            let k = (-32.0 / (sr * 0.015)).exp();
            self.current_gain =
                k * self.current_gain + (1.0 - k) * (self.band.gain - self.reduction);
            self.current_freq = k * self.current_freq + (1.0 - k) * self.band.freq;
            self.current_q = k * self.current_q + (1.0 - k) * self.band.q;
            self.coeff = Coeff::make(
                self.band.shape,
                self.current_freq,
                self.current_gain,
                self.current_q,
                sr,
            );
        }
        self.phase = (self.phase + 1) % 32;
        for (i, v) in x.iter_mut().enumerate() {
            *v = self.filters[i].tick(*v, self.coeff);
        }
        x
    }
}
#[derive(Default)]
pub struct VocalComp {
    sc: [Filter; 2],
    env: f64,
    reduction: f64,
    gate_gain: f64,
}
impl VocalComp {
    pub fn new() -> Self {
        Self {
            gate_gain: 1.0,
            ..Self::default()
        }
    }
    pub fn tick(&mut self, x: [f64; 2], settings: CompSettings, sr: f64) -> ([f64; 2], f64) {
        let c = Coeff::make(Shape::LowCut, settings.hpf, 0.0, 0.707, sr);
        let peak = self.sc[0]
            .tick(x[0], c)
            .abs()
            .max(self.sc[1].tick(x[1], c).abs());
        let k = (-1.0 / (sr * if peak > self.env { 0.003 } else { 0.100 })).exp();
        self.env = k * self.env + (1.0 - k) * peak;
        let level = gain_db(self.env);
        let over = level + settings.amount;
        let knee = 6.0;
        let target = if settings.amount <= 0.0001 {
            0.0
        } else if over <= -knee / 2.0 {
            0.0
        } else if over < knee / 2.0 {
            0.75 * (over + knee / 2.0).powi(2) / (2.0 * knee)
        } else {
            over * 0.75
        };
        let k = (-1.0
            / (sr
                * if target > self.reduction {
                    0.002
                } else {
                    0.120
                }))
        .exp();
        self.reduction = k * self.reduction + (1.0 - k) * target;
        let gate_target = if settings.gate <= -79.9 {
            1.0
        } else {
            db_gain(((level - settings.gate) * 2.0).clamp(-60.0, 0.0))
        };
        let k = (-1.0
            / (sr
                * if gate_target > self.gate_gain {
                    0.002
                } else {
                    0.100
                }))
        .exp();
        self.gate_gain = k * self.gate_gain + (1.0 - k) * gate_target;
        let makeup = settings.amount * 0.35;
        let gain = db_gain(makeup - self.reduction) * self.gate_gain;
        (
            [
                x[0] * (1.0 - settings.mix + settings.mix * gain),
                x[1] * (1.0 - settings.mix + settings.mix * gain),
            ],
            self.reduction,
        )
    }
}
#[derive(Clone, Copy)]
pub struct CompSettings {
    pub amount: f64,
    pub gate: f64,
    pub hpf: f64,
    pub mix: f64,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bell_center_and_shelves() {
        assert!(
            (Coeff::make(Shape::Bell, 1000.0, 6.0, 1.0, 48000.0).response(1000.0, 48000.0) - 6.0)
                .abs()
                < 0.001
        );
        assert!(
            (Coeff::make(Shape::LowShelf, 200.0, 6.0, 0.707, 48000.0).response(1.0, 48000.0) - 6.0)
                .abs()
                < 0.01
        );
        assert!(
            Coeff::make(Shape::LowCut, 1000.0, 0.0, 0.707, 48000.0).response(50.0, 48000.0) < -50.0
        );
    }
    #[test]
    fn all_filters_finite_at_extremes() {
        for sr in [8000.0, 44100.0, 48000.0, 96000.0, 192000.0] {
            for shape in [
                Shape::Bell,
                Shape::LowShelf,
                Shape::HighShelf,
                Shape::LowCut,
                Shape::HighCut,
                Shape::Notch,
            ] {
                for f in [20.0, 20000.0] {
                    for q in [0.15, 18.0] {
                        let c = Coeff::make(shape, f, 24.0, q, sr);
                        let mut filter = Filter::default();
                        for i in 0..12000 {
                            let out = filter.tick(if i == 0 { 1.0 } else { 0.0 }, c);
                            assert!(out.is_finite() && out.abs() < 1000.0);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn dynamic_band_compresses_and_releases() {
        let mut b = BandRuntime::new(
            Band {
                dynamic: true,
                threshold: -30.0,
                attack: 1.0,
                release: 10.0,
                ..Band::default()
            },
            48000.0,
        );
        for i in 0..48000 {
            let v = (2.0 * PI * 1000.0 * i as f64 / 48000.0).sin() * 0.5;
            b.tick([v, v], 48000.0);
        }
        assert!(b.reduction > 5.0);
        for _ in 0..48000 {
            b.tick([0.0; 2], 48000.0);
        }
        assert!(b.reduction < 0.01);
    }
    #[test]
    fn vocal_stereo_link_and_silence() {
        let mut c = VocalComp::new();
        let s = CompSettings {
            amount: 30.0,
            gate: -80.0,
            hpf: 80.0,
            mix: 1.0,
        };
        let mut gr = 0.0;
        for i in 0..48000 {
            let v = (2.0 * PI * 1000.0 * i as f64 / 48000.0).sin() * 0.5;
            let (out, g) = c.tick([v, v], s, 48000.0);
            assert_eq!(out[0], out[1]);
            gr = g;
        }
        assert!(gr > 10.0);
        for _ in 0..96000 {
            let (out, _) = c.tick([0.0; 2], s, 48000.0);
            assert_eq!(out, [0.0; 2]);
        }
    }
}
