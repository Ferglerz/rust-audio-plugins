pub use pleasant_curves::{inv_s_curve, s_curve};

/// Semitone drop constant: 1.0 - 2^(-1/12) ≈ 0.0561256873183065
pub(super) const SEMI_DROP: f64 = 1.0 - 0.9438743126816935;

/// Catch-up playback is not allowed to exceed this multiple of realtime.
pub(super) const MAX_RETURN_SPEED: f64 = 8.0;

pub(super) fn lag_samples(write_head: usize, play_pos: f64, buffer_size: usize) -> f64 {
    let size = buffer_size as f64;
    let lag = (write_head as f64 - play_pos).rem_euclid(size);
    if lag > size * 0.5 {
        0.0
    } else {
        lag
    }
}

/// Quadratic bump coefficient so average speed is `1 + lag/n`, with s(0)=s0 and s(1)=1.
pub(super) fn return_coeff(s0: f64, lag: f64, n: f64) -> f64 {
    let excess = if n > 0.0 { lag / n } else { 0.0 };
    3.0 * (1.0 + 2.0 * excess - s0)
}

pub(super) fn return_speed_at(s0: f64, coeff: f64, u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    s0 + (1.0 - s0) * u + coeff * u * (1.0 - u)
}

fn return_peak(s0: f64, coeff: f64) -> f64 {
    if coeff.abs() < 1e-9 {
        return s0.max(1.0);
    }
    let u = (1.0 - s0 + coeff) / (2.0 * coeff);
    if !(0.0..=1.0).contains(&u) {
        s0.max(1.0)
    } else {
        return_speed_at(s0, coeff, u)
    }
}

pub(super) fn minimum_return_len(s0: f64, lag: f64, requested: f64) -> f64 {
    let requested = requested.max(1.0);
    if return_peak(s0, return_coeff(s0, lag, requested)) <= MAX_RETURN_SPEED {
        return requested;
    }
    let mut hi = (lag / 1.0).max(requested * 2.0);
    for _ in 0..48 {
        if return_peak(s0, return_coeff(s0, lag, hi)) <= MAX_RETURN_SPEED {
            break;
        }
        hi *= 1.5;
    }
    let mut lo = requested;
    for _ in 0..48 {
        let mid = 0.5 * (lo + hi);
        if return_peak(s0, return_coeff(s0, lag, mid)) <= MAX_RETURN_SPEED {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hi
}

/// Bernstein speed segment with control values [s0, c..c, 1].
/// Its convex hull bounds speed, even when replanning above unity. Its integral
/// is exactly n + lag, so the playhead meets the live head without a final jump.
#[derive(Clone, Copy, Debug)]
pub(super) struct LiveReturn {
    start: f64,
    middle: f64,
    degree: f64,
}

impl LiveReturn {
    pub fn minimum_len(speed: f64, lag: f64) -> f64 {
        9.0 * lag / (48.0 + speed)
    }

    pub fn new(speed: f64, lag: f64, len: f64) -> Self {
        // Above unity, a long new duration needs a short initial deceleration
        // followed by gentle catch-up. Raising the degree keeps all inner
        // controls >= 1, preventing the playhead from passing the live head.
        let degree = if speed > 1.0 && lag > 0.0 {
            ((speed - 1.0) * len / lag - 1.0).ceil().max(8.0)
        } else {
            8.0
        };
        let start = if lag == 0.0 && speed > 1.0 {
            1.0
        } else {
            speed
        };
        Self {
            start,
            middle: 1.0 + ((degree + 1.0) * lag / len - (start - 1.0)) / (degree - 1.0),
            degree,
        }
    }

    fn integral(self, u: f64) -> f64 {
        let power = |v: f64| {
            if self.degree == 8.0 {
                v.powi(9)
            } else {
                v.powf(self.degree + 1.0)
            }
        };
        self.middle * u
            + (self.start - self.middle) * (1.0 - power(1.0 - u)) / (self.degree + 1.0)
            + (1.0 - self.middle) * power(u) / (self.degree + 1.0)
    }

    pub fn advance(self, from: f64, to: f64, len: f64) -> f64 {
        (self.integral(to) - self.integral(from)) * len
    }
}

#[cfg(test)]
mod live_return_tests {
    use super::*;

    #[test]
    fn replans_are_bounded_and_integrate_to_the_live_head() {
        for speed in [0.0, 0.7, 1.0, 2.0, 7.99, 8.0] {
            for lag in [0.0, 1.0, 500.0, 100_000.0] {
                for requested in [1.0_f64, 1000.0, 100_000.0] {
                    let len = requested.max(LiveReturn::minimum_len(speed, lag)).ceil();
                    let segment = LiveReturn::new(speed, lag, len);
                    assert!(segment.middle >= 0.0 && segment.middle <= MAX_RETURN_SPEED + 1e-12);
                    assert!((segment.advance(0.0, 1.0, len) - (len + lag)).abs() < 1e-8);
                    for i in 0..100 {
                        let average =
                            segment.advance(i as f64 / 100.0, (i + 1) as f64 / 100.0, 100.0);
                        assert!((-1e-9..=MAX_RETURN_SPEED + 1e-9).contains(&average));
                        let u = (i + 1) as f64 / 100.0;
                        assert!(
                            segment.integral(u) * len <= lag + u * len + 1e-8,
                            "playhead passed live audio"
                        );
                    }
                }
            }
        }
    }
}
