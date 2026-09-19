/// 4-point Hermite cubic interpolation for high-fidelity audio resampling.
///
/// Given 4 consecutive sample points y0, y1, y2, y3 and a fractional phase `frac` in [0.0, 1.0)
/// between y1 and y2, returns the interpolated sample value.
#[inline(always)]
pub fn hermite_interpolate(y0: f32, y1: f32, y2: f32, y3: f32, frac: f32) -> f32 {
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);

    ((c3 * frac + c2) * frac + c1) * frac + y1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hermite_endpoints() {
        let y0 = 0.0;
        let y1 = 1.0;
        let y2 = 2.0;
        let y3 = 3.0;

        let at_zero = hermite_interpolate(y0, y1, y2, y3, 0.0);
        assert!((at_zero - y1).abs() < 1e-6);

        let at_one = hermite_interpolate(y0, y1, y2, y3, 1.0);
        assert!((at_one - y2).abs() < 1e-6);
    }
}
