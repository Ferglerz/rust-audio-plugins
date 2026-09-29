//! RMS detector ported from `09_audio_processing_chain.jsfx-inc`.

use super::constants::{MAX_DETECTOR_DB, MAX_DETECTOR_LINEAR, MIN_DETECTOR_LEVEL};
use super::core_math::linear_to_db;
use super::dsp_utils::flush_denormal;
use super::envelope::rms_params::{smooth_rms_coefficient, update_rms_coefficient};

fn normalization_decay_coefficient(srate: f64) -> f64 {
    // Preserve the original ~208 ms time constant at 48 kHz at every sample rate.
    0.9999_f64.powf(48000.0 / srate)
}

#[derive(Debug, Clone)]
pub struct RmsDetector {
    smoothed_squared: f64,
    smoothing_coeff: f64,
    smoothing_one_minus: f64,
    target_coeff: f64,
    target_one_minus: f64,
    rms_max: f64,
    normalization_decay_coeff: f64,
    peak_for_normalize_smooth: f64,
    rms_size_ms: f64,
    srate: f64,
}

impl RmsDetector {
    pub fn new(srate: f64) -> Self {
        let (target_coeff, target_one_minus) = update_rms_coefficient(0.0, srate);
        Self {
            smoothed_squared: 0.0,
            smoothing_coeff: target_coeff,
            smoothing_one_minus: target_one_minus,
            target_coeff,
            target_one_minus,
            rms_max: 0.5,
            normalization_decay_coeff: normalization_decay_coefficient(srate),
            peak_for_normalize_smooth: 0.0,
            rms_size_ms: 0.0,
            srate,
        }
    }

    pub fn set_sample_rate(&mut self, srate: f64) {
        self.srate = srate;
        self.normalization_decay_coeff = normalization_decay_coefficient(srate);
        self.update_rms_targets(self.rms_size_ms);
    }

    pub fn update_rms_targets(&mut self, rms_size_ms: f64) {
        self.rms_size_ms = rms_size_ms;
        let (target_coeff, target_one_minus) = update_rms_coefficient(rms_size_ms, self.srate);
        self.target_coeff = target_coeff;
        self.target_one_minus = target_one_minus;
        if self.smoothing_coeff == 0.0 && self.smoothing_one_minus == 1.0 && rms_size_ms > 0.0 {
            self.smoothing_coeff = target_coeff;
            self.smoothing_one_minus = target_one_minus;
        }
    }

    pub fn smooth_coefficients(&mut self) {
        if self.coefficients_converged() {
            return;
        }
        let (coeff, one_minus) =
            smooth_rms_coefficient(self.smoothing_coeff, self.target_coeff, self.srate);
        self.smoothing_coeff = coeff;
        self.smoothing_one_minus = one_minus;
    }

    pub fn coefficients_converged(&self) -> bool {
        (self.smoothing_coeff - self.target_coeff).abs() < 0.001
    }

    pub fn detect_level(
        &mut self,
        detect_l: f64,
        detect_r: f64,
        rms_normalization: bool,
    ) -> (f64, f64) {
        let detect_squared_l = detect_l * detect_l;
        let detect_squared_r = detect_r * detect_r;

        let rms_level = if self.rms_size_ms > 0.0 {
            let detect_squared = (detect_squared_l + detect_squared_r) * 0.5;
            self.smoothed_squared = self.smoothed_squared * self.smoothing_coeff
                + detect_squared * self.smoothing_one_minus;
            self.smoothed_squared.sqrt()
        } else {
            detect_l.abs().max(detect_r.abs())
        };

        let detector_level = if rms_normalization {
            self.rms_max = if rms_level > self.rms_max {
                rms_level
            } else {
                self.rms_max * self.normalization_decay_coeff
            };
            let normalized_rms = rms_level / (self.rms_max + 1e-30);
            let peak_level = detect_l.abs().max(detect_r.abs());
            let norm_peak_c = if self.rms_size_ms > 0.0 {
                self.smoothing_coeff
            } else {
                (-1.0 / (0.02 * self.srate)).exp()
            };
            let norm_peak_1m = 1.0 - norm_peak_c;
            self.peak_for_normalize_smooth =
                self.peak_for_normalize_smooth * norm_peak_c + peak_level * norm_peak_1m;
            normalized_rms * self.peak_for_normalize_smooth
        } else if self.rms_size_ms > 0.0 {
            rms_level
        } else {
            detect_l.abs().max(detect_r.abs())
        };

        let detector_level = detector_level.min(MAX_DETECTOR_LINEAR);
        let detector_level_db = linear_to_db(detector_level.max(MIN_DETECTOR_LEVEL))
            .clamp(super::constants::MIN_DETECTOR_DB_FLOOR, MAX_DETECTOR_DB);

        (detector_level, detector_level_db)
    }

    pub fn reset(&mut self) {
        self.rms_max = 0.5;
        self.peak_for_normalize_smooth = 0.0;
        self.smoothed_squared = 0.0;
    }

    pub fn flush_line_state(&mut self) {
        flush_denormal(&mut self.rms_max);
        flush_denormal(&mut self.peak_for_normalize_smooth);
        flush_denormal(&mut self.smoothed_squared);
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_mode_tracks_amplitude() {
        let mut det = RmsDetector::new(48000.0);
        let (level, db) = det.detect_level(0.5, 0.5, false);
        assert!((level - 0.5).abs() < 1e-9);
        assert!(db > -10.0);
    }

    #[test]
    fn rms_window_smooths() {
        let mut det = RmsDetector::new(48000.0);
        det.update_rms_targets(0.5);
        det.smoothing_coeff = det.target_coeff;
        det.smoothing_one_minus = det.target_one_minus;
        for _ in 0..50000 {
            det.detect_level(1.0, 1.0, false);
        }
        assert!(det.smoothed_squared > 0.9);
    }

    #[test]
    fn normalization_max_decay_matches_elapsed_time() {
        let expected = 0.5 * 0.9999_f64.powf(48000.0 * 0.1);
        for srate in [44100.0, 48000.0, 96000.0, 192000.0] {
            let mut det = RmsDetector::new(srate);
            for _ in 0..(srate * 0.1) as usize {
                det.detect_level(0.0, 0.0, true);
            }
            assert!(
                (det.rms_max - expected).abs() < 1e-10,
                "srate={srate}, rms_max={}, expected={expected}",
                det.rms_max
            );
        }
    }

    #[test]
    fn normalization_decay_updates_with_sample_rate() {
        let mut det = RmsDetector::new(48000.0);
        det.detect_level(1.0, 1.0, true);
        det.set_sample_rate(96000.0);
        for _ in 0..9600 {
            det.detect_level(0.0, 0.0, true);
        }
        let expected = 0.9999_f64.powf(4800.0);
        assert!((det.rms_max - expected).abs() < 1e-10);
    }

    #[test]
    fn normalized_detector_matches_elapsed_time() {
        for rms_ms in [0.0, 10.0, 100.0] {
            let mut reference: Option<f64> = None;
            for srate in [44100.0, 48000.0, 96000.0, 192000.0] {
                let mut det = RmsDetector::new(srate);
                det.update_rms_targets(rms_ms);
                let mut level = 0.0;
                for _ in 0..(srate * 0.1) as usize {
                    level = det.detect_level(0.05, 0.05, true).0;
                }
                if let Some(expected) = reference {
                    assert!(
                        (level - expected).abs() < 1e-10,
                        "rms_ms={rms_ms}, srate={srate}, level={level}, expected={expected}"
                    );
                } else {
                    reference = Some(level);
                }
            }
        }
    }
}
