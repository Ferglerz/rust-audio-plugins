/// One-pole coefficient for a time constant measured in seconds.
#[inline]
pub fn coefficient(time_seconds: f64, sample_interval_seconds: f64) -> f64 {
    (-sample_interval_seconds / time_seconds.max(1.0e-9)).exp()
}

/// Select attack or release coefficient based on target direction.
#[inline]
pub fn smooth(current: f64, target: f64, attack: f64, release: f64) -> f64 {
    let coefficient = if target > current { attack } else { release };
    coefficient * current + (1.0 - coefficient) * target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coefficient_stays_finite_for_zero_time() {
        assert!(coefficient(0.0, 1.0 / 48_000.0).is_finite());
    }

    #[test]
    fn smoothing_selects_directional_coefficients() {
        assert_eq!(smooth(0.0, 1.0, 0.0, 1.0), 1.0);
        assert_eq!(smooth(1.0, 0.0, 0.0, 1.0), 1.0);
    }
}
