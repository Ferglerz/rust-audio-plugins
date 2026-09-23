/// Symmetric S-curve used by Tape Stop's deceleration trajectory.
#[inline(always)]
pub fn s_curve(t: f32, exponent: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else if t < 0.5 {
        0.5 * (2.0 * t).powf(exponent)
    } else {
        1.0 - 0.5 * (2.0 * (1.0 - t)).powf(exponent)
    }
}

/// Inverse of [`s_curve`].
#[inline(always)]
pub fn inv_s_curve(y: f32, exponent: f32) -> f32 {
    let y = y.clamp(0.0, 1.0);
    let exponent = exponent.max(1.0e-3);
    if y <= 0.0 {
        0.0
    } else if y >= 1.0 {
        1.0
    } else if y < 0.5 {
        0.5 * (2.0 * y).powf(1.0 / exponent)
    } else {
        1.0 - 0.5 * (2.0 * (1.0 - y)).powf(1.0 / exponent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_and_inverse_match() {
        for exponent in [0.5, 1.0, 2.5, 4.0] {
            assert_eq!(s_curve(0.0, exponent), 0.0);
            assert_eq!(s_curve(1.0, exponent), 1.0);
            for t in [0.0, 0.1, 0.5, 0.73, 1.0] {
                assert!((inv_s_curve(s_curve(t, exponent), exponent) - t).abs() < 1.0e-5);
            }
        }
    }
}
