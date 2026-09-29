//! Biquad detection filters ported from `02_InputProcessing/02_filters.jsfx-inc`.

use super::dsp_utils::flush_denormal;

#[derive(Debug, Clone, Copy, Default)]
pub struct BiquadCoeffs {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

crate::impl_lerp_f64!(BiquadCoeffs { b0, b1, b2, a1, a2 });

impl BiquadCoeffs {
    pub fn converged_to(self, target: Self, eps: f64) -> bool {
        (self.b0 - target.b0).abs() < eps
            && (self.b1 - target.b1).abs() < eps
            && (self.b2 - target.b2).abs() < eps
            && (self.a1 - target.a1).abs() < eps
            && (self.a2 - target.a2).abs() < eps
    }
}

#[derive(Debug, Clone, Default)]
pub struct BiquadFilter {
    coeffs: BiquadCoeffs,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl BiquadFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_coeffs(&mut self, coeffs: BiquadCoeffs) {
        self.coeffs = coeffs;
    }

    pub fn process(&mut self, input: f64) -> f64 {
        let c = &self.coeffs;
        let output =
            c.b0 * input + c.b1 * self.x1 + c.b2 * self.x2 - c.a1 * self.y1 - c.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        if output.is_nan() || output.is_infinite() {
            self.y1 = input;
            self.y2 = 0.0;
            self.x1 = input;
            self.x2 = 0.0;
            return input;
        }
        output
    }

    pub fn flush_denormals(&mut self) {
        flush_denormal(&mut self.x1);
        flush_denormal(&mut self.x2);
        flush_denormal(&mut self.y1);
        flush_denormal(&mut self.y2);
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// Filter type: 0 = highpass, 1 = lowpass (matches JSFX `calc_biquad`).
pub fn calc_biquad(filter_type: u8, freq: f64, srate: f64) -> BiquadCoeffs {
    let w = 2.0 * std::f64::consts::PI * freq / srate;
    let cosw = w.cos();
    let alpha = w.sin() / std::f64::consts::SQRT_2;
    let a0 = 1.0 + alpha;
    let a0_inv = 1.0 / a0;

    let (b0, b1, b2) = if filter_type == 1 {
        let b0 = (1.0 - cosw) * 0.5 * a0_inv;
        let b1 = (1.0 - cosw) * a0_inv;
        (b0, b1, b0)
    } else {
        let b0 = (1.0 + cosw) * 0.5 * a0_inv;
        let b1 = -(1.0 + cosw) * a0_inv;
        (b0, b1, b0)
    };

    BiquadCoeffs {
        b0,
        b1,
        b2,
        a1: -2.0 * cosw * a0_inv,
        a2: (1.0 - alpha) * a0_inv,
    }
}

#[derive(Debug, Clone, Default)]
pub struct DetectionFilters {
    hp_l: BiquadFilter,
    hp_r: BiquadFilter,
    lp_l: BiquadFilter,
    lp_r: BiquadFilter,
    hp_enabled: bool,
    lp_enabled: bool,
    hp_target: BiquadCoeffs,
    lp_target: BiquadCoeffs,
    hp_active: BiquadCoeffs,
    lp_active: BiquadCoeffs,
}

impl DetectionFilters {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set morph targets (block rate). Call `smooth_coefficients` each sample.
    pub fn set_target_coefficients(&mut self, hp_freq: f64, lp_freq: f64, srate: f64) {
        if hp_freq > 0.0 {
            let freq = hp_freq.max(20.0);
            self.hp_target = calc_biquad(0, freq, srate);
            self.hp_enabled = true;
        } else {
            self.hp_enabled = false;
        }

        if lp_freq > 0.0 {
            let freq = lp_freq.min(20000.0);
            self.lp_target = calc_biquad(1, freq, srate);
            self.lp_enabled = true;
        } else {
            self.lp_enabled = false;
        }
    }

    pub fn smooth_coefficients(&mut self, one_minus: f64) {
        if self.hp_enabled {
            self.hp_active = self.hp_active.lerp(self.hp_target, one_minus);
            self.hp_l.set_coeffs(self.hp_active);
            self.hp_r.set_coeffs(self.hp_active);
        }
        if self.lp_enabled {
            self.lp_active = self.lp_active.lerp(self.lp_target, one_minus);
            self.lp_l.set_coeffs(self.lp_active);
            self.lp_r.set_coeffs(self.lp_active);
        }
    }

    pub fn coefficients_converged(&self) -> bool {
        const EPS: f64 = 1e-9;
        let hp_ok = !self.hp_enabled || self.hp_active.converged_to(self.hp_target, EPS);
        let lp_ok = !self.lp_enabled || self.lp_active.converged_to(self.lp_target, EPS);
        hp_ok && lp_ok
    }

    pub fn snap_coefficients(&mut self) {
        if self.hp_enabled {
            self.hp_active = self.hp_target;
            self.hp_l.set_coeffs(self.hp_active);
            self.hp_r.set_coeffs(self.hp_active);
        }
        if self.lp_enabled {
            self.lp_active = self.lp_target;
            self.lp_l.set_coeffs(self.lp_active);
            self.lp_r.set_coeffs(self.lp_active);
        }
    }

    pub fn apply(&mut self, detect_l: f64, detect_r: f64) -> (f64, f64) {
        let (mut l, mut r) = (detect_l, detect_r);

        if self.hp_enabled {
            l = self.hp_l.process(l);
            r = self.hp_r.process(r);
        }

        if self.lp_enabled {
            l = self.lp_l.process(l);
            r = self.lp_r.process(r);
        }

        (l, r)
    }

    pub fn flush_denormals(&mut self) {
        self.hp_l.flush_denormals();
        self.hp_r.flush_denormals();
        self.lp_l.flush_denormals();
        self.lp_r.flush_denormals();
    }

    pub fn reset(&mut self) {
        self.hp_l.reset();
        self.hp_r.reset();
        self.lp_l.reset();
        self.lp_r.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hp_coefficients_at_1khz() {
        let c = calc_biquad(0, 1000.0, 48000.0);
        assert!(c.b0 > 0.0);
        assert!(c.a2.abs() < 1.0);
    }

    #[test]
    fn lp_passthrough_disabled() {
        let mut filters = DetectionFilters::new();
        filters.set_target_coefficients(0.0, 0.0, 48000.0);
        filters.snap_coefficients();
        let (l, r) = filters.apply(0.5, -0.3);
        assert!((l - 0.5).abs() < 1e-12);
        assert!((r - (-0.3)).abs() < 1e-12);
    }

    #[test]
    fn denormal_flush_zeros_tiny_state() {
        let mut f = BiquadFilter::new();
        f.x1 = 1e-20;
        f.flush_denormals();
        assert_eq!(f.x1, 0.0);
    }
}
