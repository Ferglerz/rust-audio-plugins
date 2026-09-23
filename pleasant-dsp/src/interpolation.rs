/// Four-point Hermite interpolation between `y1` and `y2`.
#[inline(always)]
pub fn hermite_f32(y0: f32, y1: f32, y2: f32, y3: f32, fraction: f32) -> f32 {
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * fraction + c2) * fraction + c1) * fraction + y1
}

/// Linear interpolation without clamping the fraction.
#[inline(always)]
pub fn linear_f32(start: f32, end: f32, fraction: f32) -> f32 {
    start + (end - start) * fraction
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolation_endpoints_match() {
        assert!((hermite_f32(0.0, 1.0, 2.0, 3.0, 0.0) - 1.0).abs() < 1.0e-6);
        assert!((hermite_f32(0.0, 1.0, 2.0, 3.0, 1.0) - 2.0).abs() < 1.0e-6);
        assert_eq!(linear_f32(1.0, 2.0, 0.0), 1.0);
        assert_eq!(linear_f32(1.0, 2.0, 1.0), 2.0);
    }
}
