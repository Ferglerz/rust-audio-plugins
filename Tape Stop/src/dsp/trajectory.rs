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
