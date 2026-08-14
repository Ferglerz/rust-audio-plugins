//! Shared DSP helpers (oversampling, denormal flush).

use super::constants::DENORMAL_THRESHOLD;

#[inline]
pub fn flush_denormal(v: &mut f64) {
    if v.abs() < DENORMAL_THRESHOLD {
        *v = 0.0;
    }
}

/// Piecewise-linear oversampling: max absolute value of `f` along segments from `prev` to `input`.
pub fn linear_oversample_peak<F>(prev: f64, input: f64, steps: usize, mut f: F) -> f64
where
    F: FnMut(f64) -> f64,
{
    let mut peak = f(prev).abs().max(f(input).abs());
    if steps <= 1 {
        return peak;
    }
    let step = 1.0 / steps as f64;
    let mut t = step;
    while t < 1.0 {
        let s = prev + (input - prev) * t;
        peak = peak.max(f(s).abs());
        t += step;
    }
    peak
}

/// Piecewise-linear oversampling: average of `f` along segments from `prev` to `input`.
pub fn linear_oversample_avg<F>(prev: f64, input: f64, steps: usize, mut f: F) -> f64
where
    F: FnMut(f64) -> f64,
{
    if steps <= 1 {
        return f(input);
    }
    let step = 1.0 / steps as f64;
    let mut sum = 0.0;
    for i in 0..steps {
        let t = (i as f64 + 0.5) * step;
        let x = prev + (input - prev) * t;
        sum += f(x);
    }
    sum / steps as f64
}
