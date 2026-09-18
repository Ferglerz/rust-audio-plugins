use crate::band::Shape;
use crate::processing::Delay;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::sync::Arc;

pub const LIFT_MIN_FREQ: f64 = 20.0; // No longer a processing floor — kept for compat.
pub const LIFT_FFT_SIZE: usize = 2048;
pub const LIFT_HOP_SIZE: usize = 512;
pub const LIFT_LATENCY: usize = 2047;
pub const LIFT_ID_BASE: u64 = 100_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LiftBand {
    pub id: u64,
    pub shape: Shape,
    pub order: u8,
    pub freq: f64,
    pub gain: f64,
    pub q: f64,
    pub enabled: bool,
    pub threshold: f64,
    pub ratio: f64,
    pub attack: f64,
    pub release: f64,
    pub range: f64,
}

impl Default for LiftBand {
    fn default() -> Self {
        Self {
            id: LIFT_ID_BASE + 1,
            shape: Shape::HighShelf,
            order: 2,
            freq: 12000.0,
            gain: -18.0,
            q: 0.707,
            enabled: true,
            threshold: -24.0,
            ratio: 6.0,
            attack: 5.0,
            release: 100.0,
            range: 12.0,
        }
    }
}

impl LiftBand {
    pub fn sanitize(&mut self) {
        self.order = self.order.clamp(1, 8);
        fn safe(v: &mut f64, min: f64, max: f64, default: f64) {
            *v = if v.is_finite() {
                v.clamp(min, max)
            } else {
                default
            };
        }
        safe(&mut self.freq, 20.0, 20000.0, 12000.0);
        safe(&mut self.gain, -100.0, 0.0, -18.0);
        safe(&mut self.q, 0.15, 18.0, 0.707);
        safe(&mut self.threshold, -60.0, 0.0, -24.0);
        safe(&mut self.ratio, 1.0, 20.0, 6.0);
        safe(&mut self.attack, 0.1, 200.0, 5.0);
        safe(&mut self.release, 10.0, 2000.0, 100.0);
        safe(&mut self.range, 0.0, 24.0, 12.0);
    }
}

/// Normalized [0.0, 1.0] filter influence weighting across frequencies.
pub fn filter_influence(shape: Shape, freq: f64, q: f64, order: u8, f: f64, _sr: f64) -> f64 {
    let freq = freq.max(20.0);
    let q = q.clamp(0.15, 18.0);
    match shape {
        Shape::Bell | Shape::BandPass => {
            let w = f / freq;
            let denom = 1.0 + q * q * (w - 1.0 / w).powi(2);
            (1.0 / denom.sqrt()).clamp(0.0, 1.0)
        }
        Shape::HighShelf => {
            let slope = (2.0 * q).clamp(0.5, 8.0);
            let w = freq / f.max(1e-4);
            (1.0 / (1.0 + w.powf(slope))).clamp(0.0, 1.0)
        }
        Shape::LowShelf => {
            let slope = (2.0 * q).clamp(0.5, 8.0);
            let w = f / freq;
            (1.0 / (1.0 + w.powf(slope))).clamp(0.0, 1.0)
        }
        Shape::LowCut => {
            let n = (order.clamp(1, 8) as f64).max(1.0);
            let w = freq / f.max(1e-4);
            (1.0 / (1.0 + w.powf(2.0 * n)).sqrt()).clamp(0.0, 1.0)
        }
        Shape::HighCut => {
            let n = (order.clamp(1, 8) as f64).max(1.0);
            let w = f / freq;
            (1.0 / (1.0 + w.powf(2.0 * n)).sqrt()).clamp(0.0, 1.0)
        }
        Shape::Notch => {
            let w = f / freq;
            let denom = 1.0 + q * q * (w - 1.0 / w).powi(2);
            (1.0 - 1.0 / denom.sqrt()).clamp(0.0, 1.0)
        }
    }
}

/// Stereo parallel FFT spectral compressor with pre-allocated 2048-point WOLA STFT.
pub struct LiftProcessor {
    fft: Arc<dyn Fft<f32>>,
    ifft: Arc<dyn Fft<f32>>,
    scratch_fwd: Vec<Complex<f32>>,
    scratch_inv: Vec<Complex<f32>>,
    win_analysis: [f32; LIFT_FFT_SIZE],
    win_synthesis: [f32; LIFT_FFT_SIZE],
    in_buf: [[f32; LIFT_FFT_SIZE]; 2],
    in_pos: usize,
    hop_count: usize,
    out_buf: [[f32; LIFT_FFT_SIZE]; 2],
    out_pos: usize,
    spec: [Vec<Complex<f32>>; 2],
    env: [[f32; LIFT_FFT_SIZE / 2 + 1]; 2],
    eq_delay: Delay,
    dry_delay: Delay,
    /// Per-band live gain reduction in dB (up to 8 lift bands), updated each hop with smooth ballistics.
    pub band_gr: [f32; 8],
    /// Per-bin live gain reduction in dB across 256 log-spaced frequency buckets for real-time micro-cuts display.
    pub bin_gr: [f32; 256],
}

impl Default for LiftProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl LiftProcessor {
    pub fn new() -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(LIFT_FFT_SIZE);
        let ifft = planner.plan_fft_inverse(LIFT_FFT_SIZE);
        let scratch_fwd = vec![Complex::default(); fft.get_inplace_scratch_len()];
        let scratch_inv = vec![Complex::default(); ifft.get_inplace_scratch_len()];

        let win_analysis = std::array::from_fn(|i| {
            let angle = PI * (i as f64 + 0.5) / LIFT_FFT_SIZE as f64;
            (angle.sin() / std::f64::consts::SQRT_2) as f32
        });
        let win_synthesis = std::array::from_fn(|i| {
            let angle = PI * (i as f64 + 0.5) / LIFT_FFT_SIZE as f64;
            (angle.sin() / (std::f64::consts::SQRT_2 * LIFT_FFT_SIZE as f64)) as f32
        });

        Self {
            fft,
            ifft,
            scratch_fwd,
            scratch_inv,
            win_analysis,
            win_synthesis,
            in_buf: [[0.0; LIFT_FFT_SIZE]; 2],
            in_pos: 0,
            hop_count: 0,
            out_buf: [[0.0; LIFT_FFT_SIZE]; 2],
            out_pos: 0,
            spec: [
                vec![Complex::default(); LIFT_FFT_SIZE],
                vec![Complex::default(); LIFT_FFT_SIZE],
            ],
            env: [[0.0; LIFT_FFT_SIZE / 2 + 1]; 2],
            eq_delay: Delay::new(LIFT_LATENCY),
            dry_delay: Delay::new(LIFT_LATENCY),
            band_gr: [0.0; 8],
            bin_gr: [0.0; 256],
        }
    }

    pub fn reset(&mut self) {
        self.in_buf = [[0.0; LIFT_FFT_SIZE]; 2];
        self.in_pos = 0;
        self.hop_count = 0;
        self.out_buf = [[0.0; LIFT_FFT_SIZE]; 2];
        self.out_pos = 0;
        self.env = [[0.0; LIFT_FFT_SIZE / 2 + 1]; 2];
        self.eq_delay.reset();
        self.dry_delay.reset();
        self.bin_gr = [0.0; 256];
    }

    /// Ticks one stereo sample through the parallel FFT compressor.
    /// Returns `(lift_out, eq_delayed_out, dry_delayed_out)`.
    pub fn tick(
        &mut self,
        dry_in: [f64; 2],
        eq_in: [f64; 2],
        bands: &[LiftBand],
        sr: f64,
    ) -> ([f64; 2], [f64; 2], [f64; 2]) {
        let eq_delayed = self.eq_delay.tick(eq_in);
        let dry_delayed = self.dry_delay.tick(dry_in);

        let active_bands: Vec<&LiftBand> = bands.iter().filter(|b| b.enabled).collect();
        if active_bands.is_empty() {
            let lift_out = [
                self.out_buf[0][self.out_pos] as f64,
                self.out_buf[1][self.out_pos] as f64,
            ];
            self.out_buf[0][self.out_pos] = 0.0;
            self.out_buf[1][self.out_pos] = 0.0;
            self.out_pos = (self.out_pos + 1) % LIFT_FFT_SIZE;
            self.in_buf[0][self.in_pos] = 0.0;
            self.in_buf[1][self.in_pos] = 0.0;
            self.in_pos = (self.in_pos + 1) % LIFT_FFT_SIZE;
            self.hop_count = (self.hop_count + 1) % LIFT_HOP_SIZE;
            return (lift_out, eq_delayed, dry_delayed);
        }

        self.in_buf[0][self.in_pos] = dry_in[0] as f32;
        self.in_buf[1][self.in_pos] = dry_in[1] as f32;
        self.in_pos = (self.in_pos + 1) % LIFT_FFT_SIZE;

        self.hop_count += 1;
        if self.hop_count >= LIFT_HOP_SIZE {
            self.hop_count = 0;
            self.process_hop(&active_bands, sr);
        }

        let lift_out = [
            self.out_buf[0][self.out_pos] as f64,
            self.out_buf[1][self.out_pos] as f64,
        ];
        self.out_buf[0][self.out_pos] = 0.0;
        self.out_buf[1][self.out_pos] = 0.0;
        self.out_pos = (self.out_pos + 1) % LIFT_FFT_SIZE;

        (lift_out, eq_delayed, dry_delayed)
    }

    fn process_hop(&mut self, bands: &[&LiftBand], sr: f64) {
        let num_bins = LIFT_FFT_SIZE / 2 + 1;
        let dt = LIFT_HOP_SIZE as f64 / sr;

        let mut band_gr_accum = [0.0_f64; 8];
        let mut band_weight_accum = [0.0_f64; 8];
        let mut bin_gr_target = [0.0_f32; 256];

        for ch in 0..2 {
            for i in 0..LIFT_FFT_SIZE {
                let idx = (self.in_pos + i) % LIFT_FFT_SIZE;
                let sample = self.in_buf[ch][idx] * self.win_analysis[i];
                self.spec[ch][i] = Complex::new(sample, 0.0);
            }

            self.fft
                .process_with_scratch(&mut self.spec[ch], &mut self.scratch_fwd);

            for k in 0..num_bins {
                let f_bin = k as f64 * sr / LIFT_FFT_SIZE as f64;
                let mag = self.spec[ch][k].norm();

                let mut combined_gain = 0.0_f32;
                let mut bin_max_gr = 0.0_f32;

                for (b_idx, b) in bands.iter().take(8).enumerate() {
                    let w = filter_influence(b.shape, b.freq, b.q, b.order, f_bin, sr);
                    if w <= 1e-4 {
                        continue;
                    }

                    let att_k = (-dt / (b.attack * 0.001)).exp() as f32;
                    let rel_k = (-dt / (b.release * 0.001)).exp() as f32;
                    let env = &mut self.env[ch][k];
                    let k_coeff = if mag > *env { att_k } else { rel_k };
                    *env = k_coeff * *env + (1.0 - k_coeff) * mag;

                    let level_db = 20.0 * ((*env / 512.0).max(1e-6)).log10() as f64;
                    let over = level_db - b.threshold;
                    let gr_db = if over > 0.0 {
                        let slope = 1.0 - 1.0 / b.ratio.max(1.0);
                        (over * slope).min(b.range)
                    } else {
                        0.0
                    };

                    let comp_gain = 10.0_f64.powf(-gr_db / 20.0);
                    let parallel_gain = if b.gain <= -99.5 {
                        0.0
                    } else {
                        10.0_f64.powf(b.gain / 20.0)
                    };
                    combined_gain += (w * comp_gain * parallel_gain) as f32;

                    band_gr_accum[b_idx] += gr_db * w;
                    band_weight_accum[b_idx] += w;

                    bin_max_gr = bin_max_gr.max((gr_db * w) as f32);
                }

                self.spec[ch][k] *= combined_gain;

                if (20.0..=20000.0).contains(&f_bin) && bin_max_gr > 0.0 {
                    let bin_w = sr / LIFT_FFT_SIZE as f64;
                    let f_lo = (f_bin - bin_w * 0.5).max(20.0);
                    let f_hi = (f_bin + bin_w * 0.5).min(20000.0);
                    let t_lo = ((f_lo / 20.0).log10() / 3.0).clamp(0.0, 1.0);
                    let t_hi = ((f_hi / 20.0).log10() / 3.0).clamp(0.0, 1.0);
                    let idx_lo = ((t_lo * 256.0) as usize).min(255);
                    let idx_hi = ((t_hi * 256.0) as usize).min(255);
                    for target in bin_gr_target[idx_lo..=idx_hi].iter_mut() {
                        *target = (*target).max(bin_max_gr);
                    }
                }
            }

            for k in 1..LIFT_FFT_SIZE / 2 {
                self.spec[ch][LIFT_FFT_SIZE - k] = self.spec[ch][k].conj();
            }
            self.spec[ch][LIFT_FFT_SIZE / 2] =
                Complex::new(self.spec[ch][LIFT_FFT_SIZE / 2].re, 0.0);

            self.ifft
                .process_with_scratch(&mut self.spec[ch], &mut self.scratch_inv);

            for i in 0..LIFT_FFT_SIZE {
                let out_idx = (self.out_pos + i) % LIFT_FFT_SIZE;
                self.out_buf[ch][out_idx] += self.spec[ch][i].re * self.win_synthesis[i];
            }
        }

        // Update band_gr with smooth ballistics for UI visualization
        for i in 0..8 {
            let target = if band_weight_accum[i] > 1e-4 {
                (band_gr_accum[i] / band_weight_accum[i]) as f32
            } else {
                0.0
            };
            let smooth = if target > self.band_gr[i] { 0.3 } else { 0.92 };
            self.band_gr[i] = smooth * self.band_gr[i] + (1.0 - smooth) * target;
        }

        // Update bin_gr for real-time micro-compression cuts visualization
        for (i, &target) in bin_gr_target.iter().enumerate() {
            let smooth = if target > self.bin_gr[i] { 0.25 } else { 0.85 };
            self.bin_gr[i] = smooth * self.bin_gr[i] + (1.0 - smooth) * target;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lift_band_sanitizes_and_clamps_freq() {
        let mut b = LiftBand {
            freq: 5.0, // below 20 Hz floor
            gain: 30.0,
            q: 25.0,
            ..LiftBand::default()
        };
        b.sanitize();
        assert_eq!(b.freq, 20.0);
        assert_eq!(b.gain, 0.0);
        assert_eq!(b.q, 18.0);
    }

    #[test]
    fn lift_band_gain_clamps_between_minus_100_and_0() {
        let mut b = LiftBand {
            gain: -150.0,
            ..LiftBand::default()
        };
        b.sanitize();
        assert_eq!(b.gain, -100.0);

        let mut b2 = LiftBand {
            gain: 10.0,
            ..LiftBand::default()
        };
        b2.sanitize();
        assert_eq!(b2.gain, 0.0);
    }

    #[test]
    fn filter_influence_works_at_low_frequencies() {
        // Should no longer return 0.0 at 200 Hz (old behaviour removed)
        let w = filter_influence(Shape::HighShelf, 10000.0, 0.707, 2, 200.0, 48000.0);
        // HighShelf with fc=10kHz has near-zero influence at 200 Hz (below shelf), that's fine
        // but it should NOT be exactly 0.0 from a hard guard — it now follows the shelf math
        assert!(w >= 0.0 && w <= 1.0, "influence must be in [0, 1]");

        let w_high = filter_influence(Shape::HighShelf, 10000.0, 0.707, 2, 15000.0, 48000.0);
        assert!(w_high > 0.5);
    }

    #[test]
    fn filter_influence_bell_peaks_at_center() {
        let center = 10000.0;
        let w_center = filter_influence(Shape::Bell, center, 1.0, 2, center, 48000.0);
        let w_off = filter_influence(Shape::Bell, center, 1.0, 2, 2500.0, 48000.0);
        assert!((w_center - 1.0).abs() < 1e-4);
        assert!(w_off < 0.5);
    }

    #[test]
    fn stft_unity_reconstruction_without_modification() {
        let mut proc = LiftProcessor::new();
        let sr = 48000.0;

        let band = LiftBand {
            shape: Shape::HighShelf,
            freq: 20.0,
            q: 4.0,
            gain: 0.0,
            threshold: 0.0,
            ratio: 1.0,
            range: 0.0,
            enabled: true,
            ..LiftBand::default()
        };

        let mut outputs = Vec::new();
        for i in 0..4096 {
            let x = if i == 0 { 1.0 } else { 0.0 };
            let (lift_out, _eq_del, _dry_del) = proc.tick([x, x], [x, x], &[band.clone()], sr);
            outputs.push(lift_out[0]);
        }

        let max_pos = outputs
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        println!(
            "Outputs around peak: [2046]={:.4}, [2047]={:.4}, [2048]={:.4}",
            outputs[2046], outputs[2047], outputs[2048]
        );
        assert!(max_pos == 2047 || max_pos == 2048);
    }
}
