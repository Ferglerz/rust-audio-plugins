use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

pub struct StftAnalyzer {
    sample_rate: f32,
    window_size: usize,
    hop: usize,
    write_pos: usize,
    hop_count: usize,
    ring: Vec<f32>,
    window: Vec<f32>,
    window_sum: f32,
    fft: Option<Arc<dyn Fft<f32>>>,
    fft_buf: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    magnitudes: Vec<f32>,
    phases: Vec<f32>,
    prev_phases: Vec<f32>,
}

impl Default for StftAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

fn window_size_for(sample_rate: f32) -> usize {
    let target = 4096.0 * sample_rate.max(0.0) / 48000.0;
    if !target.is_finite() || target <= 0.0 {
        return 2048;
    }
    let n = 2f32.powi(target.log2().round() as i32);
    (n as usize).clamp(2048, 16384)
}

impl StftAnalyzer {
    pub fn new() -> Self {
        Self {
            sample_rate: 0.0,
            window_size: 0,
            hop: 0,
            write_pos: 0,
            hop_count: 0,
            ring: Vec::new(),
            window: Vec::new(),
            window_sum: 0.0,
            fft: None,
            fft_buf: Vec::new(),
            scratch: Vec::new(),
            magnitudes: Vec::new(),
            phases: Vec::new(),
            prev_phases: Vec::new(),
        }
    }

    pub fn prepare(&mut self, sample_rate: f32) {
        let n = window_size_for(sample_rate);
        let hop = n / 16;
        let bins = n / 2 + 1;

        let mut window = vec![0.0; n];
        let mut window_sum = 0.0;
        let two_pi_over_n = 2.0 * std::f32::consts::PI / n as f32;
        for (i, w) in window.iter_mut().enumerate() {
            *w = 0.5 * (1.0 - (two_pi_over_n * i as f32).cos());
            window_sum += *w;
        }

        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(n);
        let scratch_len = fft.get_inplace_scratch_len();

        self.sample_rate = sample_rate;
        self.window_size = n;
        self.hop = hop;
        self.write_pos = 0;
        self.hop_count = 0;
        self.ring = vec![0.0; n];
        self.window = window;
        self.window_sum = window_sum;
        self.fft = Some(fft);
        self.fft_buf = vec![Complex::new(0.0, 0.0); n];
        self.scratch = vec![Complex::new(0.0, 0.0); scratch_len];
        self.magnitudes = vec![0.0; bins];
        self.phases = vec![0.0; bins];
        self.prev_phases = vec![0.0; bins];
    }

    pub fn reset(&mut self) {
        self.write_pos = 0;
        self.hop_count = 0;
        self.ring.fill(0.0);
        self.magnitudes.fill(0.0);
        self.phases.fill(0.0);
        self.prev_phases.fill(0.0);
    }

    pub fn push(&mut self, sample: f32) -> bool {
        let n = self.window_size;
        let hop = self.hop;
        if n == 0 || hop == 0 {
            return false;
        }
        if self.fft.is_none() {
            return false;
        }

        self.ring[self.write_pos] = sample;
        self.write_pos += 1;
        if self.write_pos == n {
            self.write_pos = 0;
        }
        self.hop_count += 1;
        if self.hop_count < hop {
            return false;
        }
        self.hop_count = 0;

        let start = self.write_pos;
        for i in 0..n {
            let s = self.ring[(start + i) % n];
            self.fft_buf[i] = Complex::new(s * self.window[i], 0.0);
        }
        if let Some(fft) = &self.fft {
            fft.process_with_scratch(&mut self.fft_buf, &mut self.scratch);
        }

        self.prev_phases.copy_from_slice(&self.phases);

        let nyquist = n / 2;
        let inv_sum = 1.0 / self.window_sum;
        let twice = 2.0 * inv_sum;
        for k in 0..=nyquist {
            let c = self.fft_buf[k];
            self.phases[k] = c.im.atan2(c.re);
            let mag = (c.re * c.re + c.im * c.im).sqrt();
            self.magnitudes[k] = mag
                * if k == 0 || k == nyquist {
                    inv_sum
                } else {
                    twice
                };
        }
        true
    }

    pub fn magnitudes(&self) -> &[f32] {
        &self.magnitudes
    }

    pub fn phases(&self) -> &[f32] {
        &self.phases
    }

    pub fn prev_phases(&self) -> &[f32] {
        &self.prev_phases
    }

    pub fn bin_hz(&self) -> f32 {
        if self.window_size == 0 {
            0.0
        } else {
            self.sample_rate / self.window_size as f32
        }
    }

    pub fn hop(&self) -> usize {
        self.hop
    }

    pub fn window_size(&self) -> usize {
        self.window_size
    }

    pub fn latency(&self) -> usize {
        self.window_size / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_size_tracks_sample_rate() {
        let mut stft = StftAnalyzer::new();
        assert!(!stft.push(0.0));

        stft.prepare(48_000.0);
        assert_eq!(stft.window_size(), 4096);
        assert_eq!(stft.hop(), 256);
        assert_eq!(stft.latency(), 2048);
        assert!((stft.bin_hz() - 48_000.0 / 4096.0).abs() < 1e-4);

        stft.prepare(96_000.0);
        assert_eq!(stft.window_size(), 8192);

        stft.prepare(44_100.0);
        assert_eq!(stft.window_size(), 4096);

        stft.prepare(192_000.0);
        assert_eq!(stft.window_size(), 16384);
    }

    #[test]
    fn push_true_once_per_hop() {
        let mut stft = StftAnalyzer::new();
        stft.prepare(48_000.0);
        let hop = stft.hop();
        for i in 0..hop * 4 {
            let framed = stft.push(0.0);
            assert_eq!(framed, (i + 1) % hop == 0, "i={i}");
        }
    }

    #[test]
    fn bin_centered_sine_reads_amplitude() {
        let mut stft = StftAnalyzer::new();
        stft.prepare(48_000.0);
        let n = stft.window_size();
        let two_pi_k_over_n = 2.0 * std::f64::consts::PI * 37.0 / n as f64;
        for i in 0..(2 * n) {
            let sample = (0.5 * (two_pi_k_over_n * i as f64).sin()) as f32;
            stft.push(sample);
        }
        let mags = stft.magnitudes();
        let (peak_bin, peak) = mags
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();
        assert_eq!(peak_bin, 37);
        assert!(
            (0.49..=0.51).contains(peak),
            "magnitude at bin 37 was {peak}"
        );
    }

    #[test]
    fn reset_zeroes_magnitudes() {
        let mut stft = StftAnalyzer::new();
        stft.prepare(48_000.0);
        let n = stft.window_size();
        let two_pi_k_over_n = 2.0 * std::f64::consts::PI * 37.0 / n as f64;
        for i in 0..(2 * n) {
            let sample = (0.5 * (two_pi_k_over_n * i as f64).sin()) as f32;
            stft.push(sample);
        }
        assert!(stft.magnitudes().iter().any(|&m| m != 0.0));
        stft.reset();
        assert!(stft.magnitudes().iter().all(|&m| m == 0.0));
    }
}
