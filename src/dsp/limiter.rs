//! Soft-clip limiter with 4× inter-sample peak detect (stereo-linked).

use super::constants::LIMITER_SCALE;
use super::core_math::tanh_jsfx;
use super::dsp_utils::{flush_denormal, linear_oversample_peak};

const ENGAGE_LINEAR: f64 = 0.944;
const TRUE_PEAK_LINEAR: f64 = 0.95;
const TRUE_PEAK_OVERSAMPLE: usize = 4;

#[derive(Debug, Clone)]
pub struct SoftClipLimiter {
    pub prev_l: f64,
    pub prev_r: f64,
    tanh_norm: f64,
}

impl Default for SoftClipLimiter {
    fn default() -> Self {
        Self {
            prev_l: 0.0,
            prev_r: 0.0,
            tanh_norm: 1.0 / tanh_jsfx(0.95),
        }
    }
}

impl SoftClipLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Piecewise-linear 4× oversample peak between `prev` and `input`.
    fn true_peak(prev: f64, input: f64) -> f64 {
        linear_oversample_peak(prev, input, TRUE_PEAK_OVERSAMPLE, |s| s)
    }

    #[inline]
    fn apply_soft_clip(&self, input: f64) -> f64 {
        tanh_jsfx(input * LIMITER_SCALE) * self.tanh_norm
    }

    /// Stereo-linked: shared gain reduction from the hotter channel's soft-clip curve.
    pub fn process_stereo(&mut self, input_l: f64, input_r: f64) -> (f64, f64) {
        let prev_l = self.prev_l;
        let prev_r = self.prev_r;
        let peak_l = Self::true_peak(prev_l, input_l);
        let peak_r = Self::true_peak(prev_r, input_r);
        let linked_peak = peak_l.max(peak_r);

        let near_ceiling = input_l.abs() > ENGAGE_LINEAR
            || input_r.abs() > ENGAGE_LINEAR
            || prev_l.abs() > ENGAGE_LINEAR
            || prev_r.abs() > ENGAGE_LINEAR;

        let (out_l, out_r) = if near_ceiling && linked_peak > TRUE_PEAK_LINEAR {
            let hot = if peak_l >= peak_r { input_l } else { input_r };
            let clipped = self.apply_soft_clip(hot);
            let gain = if hot.abs() > 1e-12 {
                clipped / hot
            } else {
                1.0
            };
            (input_l * gain, input_r * gain)
        } else {
            (input_l, input_r)
        };

        self.prev_l = out_l;
        self.prev_r = out_r;
        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        self.prev_l = 0.0;
        self.prev_r = 0.0;
    }

    pub fn flush_denormals(&mut self) {
        flush_denormal(&mut self.prev_l);
        flush_denormal(&mut self.prev_r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn below_threshold_passthrough() {
        let mut lim = SoftClipLimiter::new();
        let (l, r) = lim.process_stereo(0.5, 0.4);
        assert!((l - 0.5).abs() < 1e-12);
        assert!((r - 0.4).abs() < 1e-12);
    }

    #[test]
    fn at_threshold_soft_clips() {
        let mut lim = SoftClipLimiter::new();
        let (l, _) = lim.process_stereo(1.5, 0.2);
        assert!(l < 1.5);
        assert!(l > 0.9);
    }

    #[test]
    fn stereo_linked_when_one_channel_peaks() {
        let mut lim = SoftClipLimiter::new();
        let (l, r) = lim.process_stereo(1.5, 0.2);
        assert!(l < 1.5);
        assert!(r < 0.2);
        assert!((l / r - 1.5 / 0.2).abs() < 1e-6);
    }

    #[test]
    fn true_peak_catches_midpoint_overshoot() {
        let peak = SoftClipLimiter::true_peak(-1.0, 1.0);
        assert!(peak >= 0.5);
    }

    #[test]
    fn inter_sample_peak_between_quiet_samples() {
        let mut lim = SoftClipLimiter::new();
        let _ = lim.process_stereo(0.2, 0.2);
        let peak = SoftClipLimiter::true_peak(lim.prev_l, 0.2);
        assert!(peak >= 0.2);
        let (l, r) = lim.process_stereo(-0.2, -0.2);
        assert!(l.is_finite());
        assert!(r.is_finite());
    }
}
