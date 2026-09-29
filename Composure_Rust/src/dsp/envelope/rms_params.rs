//! RMS coefficient updates ported from `Envelope/05_envelope_parameters.jsfx-inc`.

use super::super::core_math::ms_to_coeff;

/// Target RMS smoothing coefficients.
pub fn update_rms_coefficient(rms_size_ms: f64, srate: f64) -> (f64, f64) {
    if rms_size_ms > 0.0 {
        let coeff = ms_to_coeff(rms_size_ms, srate);
        (coeff, 1.0 - coeff)
    } else {
        (0.0, 1.0)
    }
}

/// Smoothly interpolate RMS coefficients toward target (0.5 ms time constant).
pub fn smooth_rms_coefficient(current_coeff: f64, target_coeff: f64, srate: f64) -> (f64, f64) {
    let smoothing_time = 0.0005;
    let smooth_coeff = (-1.0 / (smoothing_time * srate)).exp();
    let smooth_one_minus = 1.0 - smooth_coeff;

    let mut coeff = current_coeff * smooth_coeff + target_coeff * smooth_one_minus;

    if (coeff - target_coeff).abs() < 0.001 {
        coeff = target_coeff;
    }

    (coeff, 1.0 - coeff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_ms_gives_peak_mode_coeffs() {
        let (c, om) = update_rms_coefficient(0.0, 48000.0);
        assert_eq!(c, 0.0);
        assert_eq!(om, 1.0);
    }

    #[test]
    fn positive_ms_gives_exponential_coeff() {
        let (c, _) = update_rms_coefficient(10.0, 48000.0);
        assert!(c > 0.0 && c < 1.0);
    }
}
