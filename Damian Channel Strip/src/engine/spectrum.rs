use super::Engine;
use pleasant_dsp::spectrum::{peak_hold, SPECTRUM_FALL_DB_PER_FRAME};
use rustfft::num_complex::Complex;
use std::sync::atomic::Ordering;

pub(super) fn hann_window() -> [f32; 2048] {
    std::array::from_fn(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / 2048.0).cos())
}

pub(super) fn bin_ranges(sr: f64) -> [(usize, usize); 128] {
    std::array::from_fn(|i| {
        let lo = 20.0_f64 * 1000.0_f64.powf(i as f64 / 128.0);
        let hi = 20.0_f64 * 1000.0_f64.powf((i + 1) as f64 / 128.0);
        (
            (lo / sr * 2048.0).floor() as usize,
            ((hi / sr * 2048.0).ceil() as usize).min(1023),
        )
    })
}

impl Engine {
    pub(super) fn push_spectrum_sample(&mut self, input: [f64; 2]) {
        if self.display_enabled {
            // Pick the louder channel to avoid mono cancellation.
            let sample = if input[0].abs() > input[1].abs() {
                input[0]
            } else {
                input[1]
            };
            self.samples[self.spectrum_position] =
                Complex::new(sample as f32 * self.window[self.spectrum_position], 0.0);
            self.spectrum_position += 1;
        }
    }

    pub(super) fn flush_spectrum(&mut self) {
        if self.spectrum_position == 2048 {
            self.spectrum_position = 0;
            self.fft
                .process_with_scratch(&mut self.samples, &mut self.scratch);
            for i in 0..128 {
                let (start, end) = self.spectrum_bin_ranges[i];
                let mut mag = 0.0_f32;
                if start <= end {
                    for bin in start.max(1)..=end {
                        mag = mag.max(self.samples[bin].norm() / 512.0);
                    }
                }
                let db = (20.0 * mag.max(0.0000316).log10()).max(-90.0);
                let old = self.shared.spectrum[i].load(Ordering::Relaxed);
                self.shared.spectrum[i].store(
                    peak_hold(old, db, SPECTRUM_FALL_DB_PER_FRAME),
                    Ordering::Relaxed,
                );
            }
        }
    }
}
