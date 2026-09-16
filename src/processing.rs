//! Independent processing modes; these do not reproduce FabFilter's proprietary algorithms.
use crate::{
    band::Band,
    dsp::{db_gain, BandCoeffs, BandRuntime},
};
use nih_plug::prelude::Enum;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ProcessingMode {
    #[default]
    #[id = "zero-latency"]
    #[name = "Zero Latency"]
    ZeroLatency,
    #[id = "natural-phase"]
    #[name = "Natural Phase (4x)"]
    NaturalPhase,
    #[id = "linear-phase"]
    #[name = "Linear Phase"]
    LinearPhase,
}
#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Resolution {
    #[id = "low"]
    Low,
    #[default]
    #[id = "medium"]
    Medium,
    #[id = "high"]
    High,
    #[id = "very-high"]
    #[name = "Very High"]
    VeryHigh,
    #[id = "maximum"]
    Maximum,
}
pub const MODES: [ProcessingMode; 3] = [
    ProcessingMode::ZeroLatency,
    ProcessingMode::NaturalPhase,
    ProcessingMode::LinearPhase,
];
pub const RESOLUTIONS: [Resolution; 5] = [
    Resolution::Low,
    Resolution::Medium,
    Resolution::High,
    Resolution::VeryHigh,
    Resolution::Maximum,
];
impl ProcessingMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::ZeroLatency => "ZERO LATENCY",
            Self::NaturalPhase => "NATURAL PHASE (4x)",
            Self::LinearPhase => "LINEAR PHASE",
        }
    }
    pub fn rate(self, sr: f64) -> f64 {
        if self == Self::NaturalPhase {
            sr * 4.0
        } else {
            sr
        }
    }
}
impl Resolution {
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::VeryHigh => "VERY HIGH",
            Self::Maximum => "MAXIMUM",
        }
    }
    /// Round to partitions, preserving approximately constant resolution in Hz across rates.
    fn order(self, sr: f64) -> usize {
        let base = [4096, 8192, 16384, 32768, 131072][self as usize];
        ((base as f64 * sr / 44100.0 / HOP as f64).ceil() as usize).max(2) * HOP
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub mode: ProcessingMode,
    pub resolution: Resolution,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            mode: ProcessingMode::ZeroLatency,
            resolution: Resolution::Medium,
        }
    }
}
impl Config {
    pub fn encode(self) -> u32 {
        self.mode as u32 * 8 + self.resolution as u32
    }
    pub fn decode(value: u32) -> Self {
        Self {
            mode: MODES.get((value / 8) as usize).copied().unwrap_or_default(),
            resolution: RESOLUTIONS
                .get((value % 8) as usize)
                .copied()
                .unwrap_or_default(),
        }
    }
    pub fn latency(self, sr: f64) -> usize {
        match self.mode {
            ProcessingMode::ZeroLatency => 0,
            ProcessingMode::NaturalPhase => 64,
            ProcessingMode::LinearPhase => self.resolution.order(sr) / 2 + HOP,
        }
    }
}

pub struct Delay {
    samples: Vec<[f64; 2]>,
    pos: usize,
}
impl Delay {
    pub fn new(length: usize) -> Self {
        Self {
            samples: vec![[0.0; 2]; length],
            pos: 0,
        }
    }
    pub fn tick(&mut self, x: [f64; 2]) -> [f64; 2] {
        if self.samples.is_empty() {
            return x;
        }
        let out = self.samples[self.pos];
        self.samples[self.pos] = x;
        self.pos = (self.pos + 1) % self.samples.len();
        out
    }
    pub fn reset(&mut self) {
        self.samples.fill([0.0; 2]);
        self.pos = 0;
    }
}

pub enum EqPath {
    Direct,
    Natural(Box<Oversampled>),
    Linear(Box<LinearPhase>),
}
impl EqPath {
    pub fn new(bands: &[Band], sr: f64, config: Config) -> Self {
        match config.mode {
            ProcessingMode::ZeroLatency => Self::Direct,
            ProcessingMode::NaturalPhase => Self::Natural(Box::new(Oversampled::new())),
            ProcessingMode::LinearPhase => {
                Self::Linear(Box::new(LinearPhase::new(bands, sr, config.resolution)))
            }
        }
    }
    pub fn tick(&mut self, input: [f64; 2], bands: &mut [BandRuntime], sr: f64) -> [f64; 2] {
        match self {
            Self::Direct => cascade(input, bands, sr),
            Self::Natural(p) => p.tick(input, bands, sr),
            Self::Linear(p) => p.tick(input),
        }
    }
    pub fn reset(&mut self) {
        match self {
            Self::Direct => {}
            Self::Natural(p) => p.reset(),
            Self::Linear(p) => p.reset(),
        }
    }
}
fn cascade(mut x: [f64; 2], bands: &mut [BandRuntime], sr: f64) -> [f64; 2] {
    for b in bands {
        x = b.tick(x, sr);
    }
    x
}

// Windowed-sinc interpolation and anti-alias filters at 4x. The two 257-tap
// filters contribute 256 high-rate samples = 64 host samples of group delay.
const OS_TAPS: usize = 257;
pub struct Oversampled {
    kernel: [f64; OS_TAPS],
    up: [[f64; 2]; OS_TAPS],
    down: [[f64; 2]; OS_TAPS],
    pos: usize,
}
impl Oversampled {
    fn new() -> Self {
        let mut kernel = std::array::from_fn(|i| {
            let t = i as f64 - (OS_TAPS / 2) as f64;
            let cutoff = 0.11875;
            let sinc = if t == 0.0 {
                2.0 * cutoff
            } else {
                (std::f64::consts::TAU * cutoff * t).sin() / (std::f64::consts::PI * t)
            };
            let a = std::f64::consts::TAU * i as f64 / (OS_TAPS - 1) as f64;
            sinc * (0.42 - 0.5 * a.cos() + 0.08 * (2.0 * a).cos())
        });
        let sum: f64 = kernel.iter().sum();
        for k in &mut kernel {
            *k /= sum;
        }
        Self {
            kernel,
            up: [[0.0; 2]; OS_TAPS],
            down: [[0.0; 2]; OS_TAPS],
            pos: 0,
        }
    }
    fn reset(&mut self) {
        self.up.fill([0.0; 2]);
        self.down.fill([0.0; 2]);
        self.pos = 0;
    }
    fn tick(&mut self, x: [f64; 2], bands: &mut [BandRuntime], sr: f64) -> [f64; 2] {
        let mut out = [0.0; 2];
        for phase in 0..4 {
            self.up[self.pos] = if phase == 0 {
                [x[0] * 4.0, x[1] * 4.0]
            } else {
                [0.0; 2]
            };
            let mut interpolated = [0.0; 2];
            // Only one polyphase branch has nonzero input samples.
            for i in (phase..OS_TAPS).step_by(4) {
                let value = self.up[(self.pos + OS_TAPS - i) % OS_TAPS];
                for ch in 0..2 {
                    interpolated[ch] += self.kernel[i] * value[ch];
                }
            }
            self.down[self.pos] = cascade(interpolated, bands, sr * 4.0);
            if phase == 0 {
                for i in 0..OS_TAPS {
                    let value = self.down[(self.pos + OS_TAPS - i) % OS_TAPS];
                    for ch in 0..2 {
                        out[ch] += self.kernel[i] * value[ch];
                    }
                }
            }
            self.pos = (self.pos + 1) % OS_TAPS;
        }
        out
    }
}

// Uniform partitioned overlap-add convolution: bounded 2048-point transforms
// on the audio thread, with all FIR design and allocation on the bank worker.
const HOP: usize = 1024;
const FFT_SIZE: usize = 2 * HOP;
pub struct LinearPhase {
    kernels: Vec<Vec<Complex<f64>>>,
    history: [Vec<Vec<Complex<f64>>>; 2],
    input: [[f64; 2]; HOP],
    output: [[f64; 2]; HOP],
    overlap: [[f64; 2]; HOP],
    work: Vec<Complex<f64>>,
    scratch: Vec<Complex<f64>>,
    forward: Arc<dyn Fft<f64>>,
    inverse: Arc<dyn Fft<f64>>,
    pos: usize,
    head: usize,
}
impl LinearPhase {
    fn new(bands: &[Band], sr: f64, resolution: Resolution) -> Self {
        let order = resolution.order(sr);
        let design_size = (order * 2).next_power_of_two();
        let mut planner = FftPlanner::<f64>::new();
        let design_inverse = planner.plan_fft_inverse(design_size);
        let coeffs: Vec<_> = bands
            .iter()
            .filter(|b| b.enabled)
            .map(|b| BandCoeffs::make(b, sr))
            .collect();
        let mut response: Vec<_> = (0..design_size)
            .map(|i| {
                let f = i.min(design_size - i) as f64 * sr / design_size as f64;
                let db: f64 = coeffs.iter().map(|c| c.response(f, sr)).sum();
                Complex::new(db_gain(db.clamp(-240.0, 120.0)), 0.0)
            })
            .collect();
        design_inverse.process(&mut response);
        let forward = planner.plan_fft_forward(FFT_SIZE);
        let inverse = planner.plan_fft_inverse(FFT_SIZE);
        let kernel: Vec<_> = (0..=order)
            .map(|i| {
                let source = (i + design_size - order / 2) % design_size;
                let a = std::f64::consts::TAU * i as f64 / order as f64;
                response[source].re / design_size as f64
                    * (0.42 - 0.5 * a.cos() + 0.08 * (2.0 * a).cos())
            })
            .collect();
        let kernels: Vec<_> = kernel
            .chunks(HOP)
            .map(|chunk| {
                let mut partition = vec![Complex::default(); FFT_SIZE];
                for (v, k) in partition.iter_mut().zip(chunk) {
                    v.re = *k;
                }
                forward.process(&mut partition);
                partition
            })
            .collect();
        let history =
            std::array::from_fn(|_| vec![vec![Complex::default(); FFT_SIZE]; kernels.len()]);
        let scratch = vec![
            Complex::default();
            forward
                .get_inplace_scratch_len()
                .max(inverse.get_inplace_scratch_len())
        ];
        Self {
            kernels,
            history,
            input: [[0.0; 2]; HOP],
            output: [[0.0; 2]; HOP],
            overlap: [[0.0; 2]; HOP],
            work: vec![Complex::default(); FFT_SIZE],
            scratch,
            forward,
            inverse,
            pos: 0,
            head: 0,
        }
    }
    fn reset(&mut self) {
        for channel in &mut self.history {
            for partition in channel {
                partition.fill(Complex::default());
            }
        }
        self.input.fill([0.0; 2]);
        self.output.fill([0.0; 2]);
        self.overlap.fill([0.0; 2]);
        self.pos = 0;
        self.head = 0;
    }
    fn tick(&mut self, x: [f64; 2]) -> [f64; 2] {
        let out = self.output[self.pos];
        self.input[self.pos] = x;
        self.pos += 1;
        if self.pos == HOP {
            self.pos = 0;
            for ch in 0..2 {
                let current = &mut self.history[ch][self.head];
                current.fill(Complex::default());
                for (v, input) in current.iter_mut().zip(&self.input) {
                    v.re = input[ch];
                }
                self.forward
                    .process_with_scratch(current, &mut self.scratch);
                self.work.fill(Complex::default());
                for (p, kernel) in self.kernels.iter().enumerate() {
                    let source = &self.history[ch]
                        [(self.head + self.kernels.len() - p) % self.kernels.len()];
                    for ((sum, x), h) in self.work.iter_mut().zip(source).zip(kernel) {
                        *sum += x * h;
                    }
                }
                self.inverse
                    .process_with_scratch(&mut self.work, &mut self.scratch);
                for i in 0..HOP {
                    self.output[i][ch] = self.work[i].re / FFT_SIZE as f64 + self.overlap[i][ch];
                    self.overlap[i][ch] = self.work[i + HOP].re / FFT_SIZE as f64;
                }
            }
            self.head = (self.head + 1) % self.kernels.len();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::band::Shape;

    #[test]
    fn linear_impulse_is_symmetric_and_has_correct_latency_at_every_resolution() {
        for resolution in RESOLUTIONS {
            let config = Config {
                mode: ProcessingMode::LinearPhase,
                resolution,
            };
            let mut path = EqPath::new(&[], 44100.0, config);
            let latency = config.latency(44100.0);
            for i in 0..latency + HOP * 2 {
                let input = if i == 0 { [1.0, -0.5] } else { [0.0; 2] };
                let actual = path.tick(input, &mut [], 44100.0);
                let expected = if i == latency { [1.0, -0.5] } else { [0.0; 2] };
                for ch in 0..2 {
                    assert!(
                        (actual[ch] - expected[ch]).abs() < 1e-10,
                        "{resolution:?}, sample {i}: {actual:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn linear_bell_matches_gain_and_is_symmetric_across_partition_boundaries() {
        let band = Band {
            freq: 1000.0,
            gain: 9.0,
            q: 2.0,
            ..Band::default()
        };
        let config = Config {
            mode: ProcessingMode::LinearPhase,
            resolution: Resolution::Low,
        };
        let sr = 48000.0;
        let mut path = EqPath::new(std::slice::from_ref(&band), sr, config);
        let latency = config.latency(sr);
        let impulse_at = HOP - 13;
        let center = latency + impulse_at;
        let mut impulse = Vec::new();
        for i in 0..center * 2 + HOP {
            let x = if i == impulse_at {
                [1.0, 0.0]
            } else {
                [0.0; 2]
            };
            let out = path.tick(x, &mut [], sr);
            assert!(out[1].abs() < 1e-12);
            impulse.push(out[0]);
        }
        for i in 1..center {
            assert!(
                (impulse[center - i] - impulse[center + i]).abs() < 1e-10,
                "asymmetric at {i}"
            );
        }
        let gain: f64 = impulse
            .iter()
            .enumerate()
            .map(|(i, x)| {
                x * (std::f64::consts::TAU * band.freq * (i as f64 - center as f64) / sr).cos()
            })
            .sum();
        assert!(
            (20.0 * gain.log10() - band.gain).abs() < 0.1,
            "gain: {gain}"
        );
        path.reset();
        for _ in 0..center * 2 {
            assert_eq!(path.tick([0.0; 2], &mut [], sr), [0.0; 2]);
        }
    }
    #[test]
    fn oversampled_neutral_path_has_unity_gain_and_declared_group_delay() {
        let sr = 48000.0;
        let config = Config {
            mode: ProcessingMode::NaturalPhase,
            ..Config::default()
        };
        let mut path = EqPath::new(&[], sr, config);
        let latency = config.latency(sr);
        let impulse: Vec<_> = (0..256)
            .map(|i| path.tick(if i == 0 { [1.0, -1.0] } else { [0.0; 2] }, &mut [], sr))
            .collect();
        let peak = impulse
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a[0].total_cmp(&b[0]))
            .unwrap()
            .0;
        assert_eq!(peak, latency);
        for i in 0..64 {
            assert!((impulse[latency - i][0] - impulse[latency + i][0]).abs() < 1e-12);
        }
        assert!((impulse.iter().map(|x| x[0]).sum::<f64>() - 1.0).abs() < 1e-5);
        for x in &impulse {
            assert!((x[0] + x[1]).abs() < 1e-12);
        }
        for i in 0..4800 {
            let x = (std::f64::consts::TAU * 1000.0 * i as f64 / sr).sin();
            let out = path.tick([x, x], &mut [], sr);
            if i > 1000 {
                let expected =
                    (std::f64::consts::TAU * 1000.0 * (i as f64 - latency as f64) / sr).sin();
                assert!((out[0] - expected).abs() < 1e-4);
            }
        }
        path.reset();
        for _ in 0..256 {
            assert_eq!(path.tick([0.0; 2], &mut [], sr), [0.0; 2]);
        }
    }
    #[test]
    fn oversampled_eq_matches_high_rate_filter_and_keeps_dynamics() {
        let sr = 48000.0;
        let band = Band {
            freq: 8000.0,
            gain: 6.0,
            q: 1.0,
            ..Band::default()
        };
        let config = Config {
            mode: ProcessingMode::NaturalPhase,
            ..Config::default()
        };
        let mut path = EqPath::new(std::slice::from_ref(&band), sr, config);
        let mut bands = vec![BandRuntime::new(band.clone(), sr * 4.0)];
        let mut energy = 0.0;
        for i in 0..9600 {
            let x = (std::f64::consts::TAU * band.freq * i as f64 / sr).sin() * 0.25;
            let out = path.tick([x, x], &mut bands, sr);
            if i >= 4800 {
                energy += out[0] * out[0];
            }
        }
        let gain_db = 10.0 * (energy / 4800.0 / (0.25 * 0.25 / 2.0)).log10();
        assert!((gain_db - 6.0).abs() < 0.05, "gain: {gain_db}");
        bands[0] = BandRuntime::new(
            Band {
                dynamic: true,
                threshold: -36.0,
                ratio: 8.0,
                range: 12.0,
                ..band
            },
            sr * 4.0,
        );
        path.reset();
        for i in 0..24000 {
            let x = (std::f64::consts::TAU * 8000.0 * i as f64 / sr).sin() * 0.25;
            path.tick([x, x], &mut bands, sr);
        }
        assert!(bands[0].reduction > 10.0);
    }
    #[test]
    fn linear_uses_static_gain_of_dynamic_bands_and_ignores_disabled_bands() {
        let dynamic = Band {
            gain: 6.0,
            freq: 1500.0,
            dynamic: true,
            ..Band::default()
        };
        let disabled = Band {
            shape: Shape::LowCut,
            freq: 8000.0,
            enabled: false,
            ..Band::default()
        };
        let config = Config {
            mode: ProcessingMode::LinearPhase,
            resolution: Resolution::Low,
        };
        let mut a = EqPath::new(&[dynamic.clone(), disabled], 44100.0, config);
        let mut b = EqPath::new(
            &[Band {
                dynamic: false,
                ..dynamic
            }],
            44100.0,
            config,
        );
        for i in 0..10000 {
            let x = [(i as f64 * 0.1).sin(); 2];
            assert_eq!(a.tick(x, &mut [], 44100.0), b.tick(x, &mut [], 44100.0));
        }
    }
    #[test]
    fn resolution_scales_with_sample_rate_and_config_round_trips() {
        for mode in MODES {
            for resolution in RESOLUTIONS {
                let config = Config { mode, resolution };
                assert_eq!(Config::decode(config.encode()), config);
                if mode == ProcessingMode::LinearPhase {
                    let order = resolution.order(96000.0);
                    assert!(order as f64 / 96000.0 >= resolution.order(44100.0) as f64 / 44100.0);
                    assert_eq!(order % HOP, 0);
                }
            }
        }
    }
}
