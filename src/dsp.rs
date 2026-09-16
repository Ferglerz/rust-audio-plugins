use crate::band::{Band, Shape};
use nih_plug::prelude::Enum;
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
            Shape::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * c, 1.0 - alpha),
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
/// Fixed-size cascade shared by the audio path and plotted response.
#[derive(Clone, Copy)]
pub struct BandCoeffs {
    stages: [Coeff; 4],
    len: usize,
}
impl BandCoeffs {
    pub fn make(b: &Band, sr: f64) -> Self {
        let identity = Coeff {
            b: [1.0, 0.0, 0.0],
            a: [0.0, 0.0],
        };
        let mut result = Self {
            stages: [identity; 4],
            len: 1,
        };
        if !b.shape.is_cut() {
            result.stages[0] = Coeff::make(b.shape, b.freq, b.gain, b.q, sr);
            return result;
        }
        let order = b.order.clamp(1, 8) as usize;
        result.len = order.div_ceil(2);
        // Butterworth pole pairs, highest Q first. The resonance control scales
        // only this pair, preserving the original second-order filter behavior.
        for i in 0..order / 2 {
            let q = 1.0 / (2.0 * (PI * (2 * i + 1) as f64 / (2 * order) as f64).sin());
            let q = if i == 0 {
                q * b.q / std::f64::consts::FRAC_1_SQRT_2
            } else {
                q
            };
            result.stages[i] = Coeff::make(b.shape, b.freq, 0.0, q, sr);
        }
        if order % 2 == 1 {
            let k = (PI * b.freq.clamp(5.0, sr * 0.45) / sr).tan();
            let norm = 1.0 / (1.0 + k);
            let (b0, b1) = if b.shape == Shape::LowCut {
                (norm, -norm)
            } else {
                (k * norm, k * norm)
            };
            result.stages[order / 2] = Coeff {
                b: [b0, b1, 0.0],
                a: [(k - 1.0) * norm, 0.0],
            };
        }
        result
    }
    pub fn response(self, f: f64, sr: f64) -> f64 {
        self.stages[..self.len]
            .iter()
            .map(|c| c.response(f, sr))
            .sum()
    }
}

#[derive(Clone, Copy, Default)]
pub struct Cascade {
    stages: [Filter; 4],
}
impl Cascade {
    pub fn tick(&mut self, mut x: f64, coeffs: BandCoeffs) -> f64 {
        for (filter, coeff) in self.stages.iter_mut().zip(&coeffs.stages[..coeffs.len]) {
            x = filter.tick(x, *coeff);
        }
        x
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
    filters: [Cascade; 2],
    detectors: [Filter; 2],
    coeff: BandCoeffs,
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
            filters: [Cascade::default(); 2],
            detectors: [Filter::default(); 2],
            coeff: BandCoeffs::make(&b, sr),
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
        if self.band.shape == old.band.shape && self.band.order == old.band.order {
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
        self.filters = [Cascade::default(); 2];
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
            let mut smoothed = self.band.clone();
            smoothed.freq = self.current_freq;
            smoothed.gain = self.current_gain;
            smoothed.q = self.current_q;
            self.coeff = BandCoeffs::make(&smoothed, sr);
        }
        self.phase = (self.phase + 1) % 32;
        for (i, v) in x.iter_mut().enumerate() {
            *v = self.filters[i].tick(*v, self.coeff);
        }
        x
    }
}
#[derive(Enum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PseTimeConstant {
    A,
    B,
    #[default]
    C,
    D,
    E,
    F,
}
impl PseTimeConstant {
    pub fn label(self) -> &'static str {
        ["A", "B", "C", "D", "E", "F"][self as usize]
    }
    pub fn times(self, peak: bool) -> (f64, f64) {
        if peak {
            (0.020, [0.020, 0.200, 1.0, 2.0, 5.0, 30.0][self as usize])
        } else {
            let t = [0.050, 0.100, 0.200, 0.750, 1.500, 3.0][self as usize];
            (t, t)
        }
    }
}
#[derive(Clone, Copy)]
pub struct PseSettings {
    pub depth: f64,
    pub hysteresis: f64,
    pub knee: f64,
    pub peak: bool,
    pub time: PseTimeConstant,
    pub listen: bool,
}
impl Default for PseSettings {
    fn default() -> Self {
        Self {
            depth: 10.0,
            hysteresis: 3.0,
            knee: 6.0,
            peak: false,
            time: PseTimeConstant::C,
            listen: false,
        }
    }
}
/// Ported from ZingZap src/dsp/pse.rs: full-rate detector, continuous
/// hysteresis, Hermite knee, and log-domain VCA ballistics. Input is HPF'd.
struct Pse {
    rms_sq: f64,
    effective_threshold: f64,
    reduction_db: f64,
    sample_rate: f64,
    rms_rc: f64,
    hysteresis_rc: f64,
    gain_rc: f64,
    release_rc: f64,
    peak_rc: f64,
    peak_env: f64,
    mode: bool,
    time: PseTimeConstant,
}
impl Default for Pse {
    fn default() -> Self {
        Self {
            rms_sq: 0.0,
            effective_threshold: -80.0,
            reduction_db: 0.0,
            sample_rate: 0.0,
            rms_rc: 0.0,
            hysteresis_rc: 0.0,
            gain_rc: 0.0,
            release_rc: 0.0,
            peak_rc: 0.0,
            peak_env: 0.0,
            mode: false,
            time: PseTimeConstant::C,
        }
    }
}
impl Pse {
    fn tick(&mut self, filtered: f64, threshold: f64, sr: f64, settings: PseSettings) -> f64 {
        if self.sample_rate != sr || self.mode != settings.peak || self.time != settings.time {
            self.mode = settings.peak;
            self.time = settings.time;
            self.sample_rate = sr;
            self.rms_rc = (-1.0 / (sr * 0.010)).exp();
            self.hysteresis_rc = (-1.0 / (sr * 0.005)).exp();
            let (attack, release) = settings.time.times(settings.peak);
            self.gain_rc = (-1.0 / (sr * attack)).exp();
            self.release_rc = (-1.0 / (sr * release)).exp();
            self.peak_rc = (-1.0 / (sr * 0.0015)).exp();
        }
        self.rms_sq = self.rms_sq * self.rms_rc + filtered * filtered * (1.0 - self.rms_rc);
        if self.rms_sq.abs() < 1e-25 {
            self.rms_sq = 0.0;
        }
        self.peak_env = self.peak_env * self.peak_rc + filtered.abs() * (1.0 - self.peak_rc);
        let level = if settings.peak {
            if self.peak_env > 1e-7 {
                gain_db(self.peak_env)
            } else {
                -140.0
            }
        } else if self.rms_sq > 1e-14 {
            10.0 * self.rms_sq.log10()
        } else {
            -140.0
        };
        let target_threshold = if level > self.effective_threshold {
            threshold - settings.hysteresis
        } else {
            threshold
        };
        self.effective_threshold = self.effective_threshold * self.hysteresis_rc
            + target_threshold * (1.0 - self.hysteresis_rc);
        let delta = level - self.effective_threshold;
        let knee = settings.knee.max(0.01);
        let weight = ((delta + knee * 0.5) / knee).clamp(0.0, 1.0);
        let weight = weight * weight * (3.0 - 2.0 * weight);
        // The original gate's minimum remains OFF. Release attenuation smoothly
        // when switched off, while keeping detectors warm for re-enabling.
        let target = if threshold <= -79.9 {
            0.0
        } else {
            -settings.depth * (1.0 - weight)
        };
        let rc = if target > self.reduction_db {
            self.gain_rc
        } else {
            self.release_rc
        };
        self.reduction_db = self.reduction_db * rc + target * (1.0 - rc);
        if self.reduction_db.abs() < 1e-20 {
            self.reduction_db = 0.0;
        }
        db_gain(self.reduction_db)
    }
}

#[derive(Default)]
pub struct VocalComp {
    sc: [Filter; 2],
    // Independent L/R detectors plus one linked stereo detector.
    env: [f64; 3],
    reduction: [f64; 3],
    pse: [Pse; 3],
    makeup: f64,
    link_mix: f64,
}
impl VocalComp {
    pub fn new() -> Self {
        Self {
            link_mix: 1.0,
            ..Self::default()
        }
    }
    fn target_reduction(level: f64, settings: CompSettings) -> f64 {
        let over = level - settings.threshold;
        let slope = 1.0 - 1.0 / settings.ratio.clamp(1.0, 20.0);
        // 0 dB retains the original neutral default.
        if settings.threshold >= -0.0001 {
            return 0.0;
        }
        if !settings.soft_knee {
            return over.max(0.0) * slope;
        }
        let knee = 6.0;
        if over <= -knee / 2.0 {
            0.0
        } else if over < knee / 2.0 {
            slope * (over + knee / 2.0).powi(2) / (2.0 * knee)
        } else {
            over * slope
        }
    }
    pub fn tick(&mut self, x: [f64; 2], settings: CompSettings, sr: f64) -> ([f64; 2], f64) {
        let c = Coeff::make(
            Shape::LowCut,
            settings.hpf,
            0.0,
            std::f64::consts::FRAC_1_SQRT_2,
            sr,
        );
        let filtered = [self.sc[0].tick(x[0], c), self.sc[1].tick(x[1], c)];
        let left = filtered[0].abs();
        let right = filtered[1].abs();
        let peaks = [left, right, left.max(right)];
        self.link_mix +=
            (1.0 - (-1.0 / (sr * 0.010)).exp()) * (f64::from(settings.stereo_link) - self.link_mix);
        let makeup_target = if settings.auto_makeup {
            -settings.threshold * 0.35 * (1.0 - 1.0 / settings.ratio.clamp(1.0, 20.0)) / 0.75
        } else {
            0.0
        };
        self.makeup += (1.0 - (-1.0 / (sr * 0.020)).exp()) * (makeup_target - self.makeup);
        let mut gains = [0.0; 3];
        for i in 0..3 {
            let k = (-1.0 / (sr * if peaks[i] > self.env[i] { 0.003 } else { 0.100 })).exp();
            self.env[i] = k * self.env[i] + (1.0 - k) * peaks[i];
            let level = gain_db(self.env[i]);
            let target = Self::target_reduction(level, settings);
            let k = (-1.0
                / (sr
                    * if target > self.reduction[i] {
                        0.002
                    } else {
                        0.120
                    }))
            .exp();
            self.reduction[i] = k * self.reduction[i] + (1.0 - k) * target;
            let pse_gain = self.pse[i].tick(peaks[i], settings.gate, sr, settings.pse);
            gains[i] = db_gain(self.makeup - self.reduction[i]) * pse_gain;
        }
        let mut out = [0.0; 2];
        let mut gr = 0.0_f64;
        for i in 0..2 {
            let gain = gains[i] + self.link_mix * (gains[2] - gains[i]);
            out[i] = x[i] * (settings.dry + settings.wet * gain);
            gr =
                gr.max(self.reduction[i] + self.link_mix * (self.reduction[2] - self.reduction[i]));
        }
        if settings.pse.listen {
            out = filtered;
        }
        (out, gr)
    }
}
#[derive(Clone, Copy)]
pub struct CompSettings {
    pub threshold: f64,
    pub ratio: f64,
    pub soft_knee: bool,
    pub auto_makeup: bool,
    pub stereo_link: bool,
    pub gate: f64,
    pub pse: PseSettings,
    pub hpf: f64,
    pub dry: f64,
    pub wet: f64,
}
impl Default for CompSettings {
    fn default() -> Self {
        Self {
            threshold: 0.0,
            ratio: 4.0,
            soft_knee: true,
            auto_makeup: true,
            stereo_link: true,
            gate: -80.0,
            pse: PseSettings::default(),
            hpf: 80.0,
            dry: 0.0,
            wet: 1.0,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cut_orders_match_butterworth_response() {
        let sr = 48000.0;
        for shape in [Shape::LowCut, Shape::HighCut] {
            for order in 1..=8 {
                let band = Band {
                    shape,
                    order,
                    freq: 1000.0,
                    q: std::f64::consts::FRAC_1_SQRT_2,
                    ..Band::default()
                };
                let c = BandCoeffs::make(&band, sr);
                for f in [50.0, 500.0, 1000.0, 2000.0, 10000.0] {
                    let mut ratio = (PI * f / sr).tan() / (PI * band.freq / sr).tan();
                    if shape == Shape::LowCut {
                        ratio = 1.0 / ratio;
                    }
                    let expected = -10.0 * (1.0 + ratio.powi(2 * order as i32)).log10();
                    assert!(
                        (c.response(f, sr) - expected).abs() < 1e-5,
                        "{shape:?}, order {order}, {f} Hz"
                    );
                }
            }
        }
    }
    #[test]
    fn second_order_preserves_existing_resonance() {
        for shape in [Shape::LowCut, Shape::HighCut] {
            for q in [0.15, 0.707, 1.0, 18.0] {
                let b = Band {
                    shape,
                    q,
                    ..Band::default()
                };
                let old = Coeff::make(shape, b.freq, b.gain, q, 48000.0);
                let new = BandCoeffs::make(&b, 48000.0);
                for f in [50.0, 1000.0, 10000.0] {
                    assert!((old.response(f, 48000.0) - new.response(f, 48000.0)).abs() < 1e-8);
                }
            }
        }
    }
    #[test]
    fn runtime_matches_plotted_response() {
        let sr = 48000.0;
        for shape in Shape::ALL {
            for order in 1..=8 {
                let b = Band {
                    shape,
                    order,
                    gain: 6.0,
                    q: 0.707,
                    ..Band::default()
                };
                let coeff = BandCoeffs::make(&b, sr);
                let mut runtime = BandRuntime::new(b, sr);
                let frequencies = [500.0, 1000.0, 2000.0];
                let mut re = [0.0; 3];
                let mut im = [0.0; 3];
                for n in 0..8192 {
                    let out = runtime.tick([if n == 0 { 1.0 } else { 0.0 }; 2], sr);
                    assert_eq!(out[0], out[1]);
                    for (i, f) in frequencies.iter().enumerate() {
                        let w = 2.0 * PI * f * n as f64 / sr;
                        re[i] += out[0] * w.cos();
                        im[i] -= out[0] * w.sin();
                    }
                }
                for (i, f) in frequencies.iter().enumerate() {
                    let measured = re[i].hypot(im[i]);
                    assert!(
                        (measured - db_gain(coeff.response(*f, sr))).abs() < 1e-6,
                        "{shape:?}, {order}, {f}"
                    );
                }
            }
        }
    }
    #[test]
    fn cascades_are_stable_at_extremes() {
        for sr in [8000.0, 44100.0, 48000.0, 96000.0, 192000.0] {
            for shape in [Shape::LowCut, Shape::HighCut, Shape::BandPass] {
                for order in 1..=8 {
                    for freq in [20.0, 20000.0] {
                        for q in [0.15, 18.0] {
                            let b = Band {
                                shape,
                                order,
                                freq,
                                q,
                                ..Band::default()
                            };
                            let c = BandCoeffs::make(&b, sr);
                            let mut filter = Cascade::default();
                            for n in 0..4096 {
                                let out = filter.tick(if n == 0 { 1.0 } else { 0.0 }, c);
                                assert!(out.is_finite() && out.abs() < 100.0);
                                assert!(filter
                                    .stages
                                    .iter()
                                    .all(|s| s.z1.is_finite() && s.z2.is_finite()));
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn order_changes_do_not_inherit_incompatible_filter_state() {
        let b = Band {
            shape: Shape::LowCut,
            ..Band::default()
        };
        let mut old = BandRuntime::new(b.clone(), 48000.0);
        old.tick([1.0; 2], 48000.0);
        let mut new = BandRuntime::new(Band { order: 8, ..b }, 48000.0);
        new.inherit(&old);
        assert_eq!(new.coeff.len, 4);
        assert_eq!(new.tick([0.0; 2], 48000.0), [0.0; 2]);
    }
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
    fn pse_advanced_controls_follow_zingzap_transfer_and_timing() {
        let sr = 48000.0;
        let mut settings = PseSettings {
            depth: 18.0,
            hysteresis: 0.0,
            knee: 12.0,
            ..PseSettings::default()
        };
        let mut pse = Pse::default();
        // At threshold, the Hermite knee applies half the configured depth.
        for _ in 0..144000 {
            pse.tick(db_gain(-40.0), -40.0, sr, settings);
        }
        assert!((pse.reduction_db + 9.0).abs() < 0.001);
        settings.hysteresis = 6.0;
        for _ in 0..144000 {
            pse.tick(db_gain(-39.0), -40.0, sr, settings);
        }
        assert!(pse.reduction_db.abs() < 0.001);
        // Peak C closes with a one-second time constant; RMS C uses 200 ms.
        let mut peak = Pse::default();
        settings.peak = true;
        for _ in 0..48000 {
            peak.tick(0.0, -40.0, sr, settings);
        }
        assert!((peak.reduction_db + 18.0 * (1.0 - (-1.0_f64).exp())).abs() < 0.001);
        let mut fast = Pse::default();
        settings.time = PseTimeConstant::A;
        for _ in 0..48000 {
            fast.tick(0.0, -40.0, sr, settings);
        }
        assert!((fast.reduction_db + 18.0).abs() < 0.001);
    }

    #[test]
    fn pse_c_depth_ballistics_and_off() {
        for sr in [44100.0, 48000.0, 96000.0] {
            let mut pse = Pse::default();
            // C is a 200 ms log-domain time constant: one tau toward -10 dB.
            for _ in 0..(sr * 0.2) as usize {
                pse.tick(0.0, -40.0, sr, PseSettings::default());
            }
            assert!((pse.reduction_db + 10.0 * (1.0 - (-1.0_f64).exp())).abs() < 0.002);
            for _ in 0..(sr * 2.0) as usize {
                pse.tick(0.0, -40.0, sr, PseSettings::default());
            }
            assert!((pse.reduction_db + 10.0).abs() < 0.001);
            for _ in 0..(sr * 2.0) as usize {
                pse.tick(0.0, -80.0, sr, PseSettings::default());
            }
            assert!(pse.reduction_db.abs() < 0.001);
            for _ in 0..(sr * 2.0) as usize {
                pse.tick(0.5, -40.0, sr, PseSettings::default());
            }
            assert!(pse.reduction_db.abs() < 0.001);
        }
    }

    #[test]
    fn pse_uses_shared_hpf_and_handles_opposite_phase_stereo() {
        let settings = CompSettings {
            gate: -30.0,
            hpf: 20.0,
            auto_makeup: false,
            ..CompSettings::default()
        };
        let mut low_hpf = VocalComp::new();
        let mut high_hpf = VocalComp::new();
        let mut input_energy = 0.0;
        let mut low_energy = 0.0;
        let mut high_energy = 0.0;
        for i in 0..144000 {
            let x = 0.1 * (std::f64::consts::TAU * 50.0 * i as f64 / 48000.0).sin();
            let (low, _) = low_hpf.tick([x, -x], settings, 48000.0);
            let (high, _) = high_hpf.tick(
                [x, -x],
                CompSettings {
                    hpf: 500.0,
                    ..settings
                },
                48000.0,
            );
            assert!((low[0] + low[1]).abs() < 1e-12);
            if i > 96000 {
                input_energy += x * x;
                low_energy += low[0] * low[0];
                high_energy += high[0] * high[0];
            }
        }
        assert!((10.0 * (low_energy / input_energy).log10()).abs() < 0.01);
        assert!((10.0 * (high_energy / input_energy).log10() + 10.0).abs() < 0.01);
    }

    #[test]
    fn vocal_stereo_link_and_silence() {
        let mut c = VocalComp::new();
        let s = CompSettings {
            threshold: -30.0,
            gate: -80.0,
            hpf: 80.0,
            dry: 0.0,
            wet: 1.0,
            ..CompSettings::default()
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
    #[test]
    fn vocal_ratio_and_knee_affect_reduction() {
        let mut c_low = VocalComp::new();
        let mut c_high = VocalComp::new();
        let s_low = CompSettings {
            threshold: -20.0,
            ratio: 2.0,
            soft_knee: false,
            auto_makeup: false,
            ..CompSettings::default()
        };
        let s_high = CompSettings {
            threshold: -20.0,
            ratio: 10.0,
            soft_knee: false,
            auto_makeup: false,
            ..CompSettings::default()
        };
        let mut gr_low = 0.0;
        let mut gr_high = 0.0;
        for i in 0..48000 {
            let v = (2.0 * PI * 1000.0 * i as f64 / 48000.0).sin() * 0.9;
            let (_, g_low) = c_low.tick([v, v], s_low, 48000.0);
            let (_, g_high) = c_high.tick([v, v], s_high, 48000.0);
            gr_low = g_low;
            gr_high = g_high;
        }
        assert!(gr_high > gr_low);
    }
}
