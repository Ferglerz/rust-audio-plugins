pub fn db_to_linear(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}

pub fn linear_to_db(lin: f64) -> f64 {
    20.0 * lin.max(1e-9).log10()
}

/// Damian-style peaking influence in 0..=1. Independent of neighboring bands.
pub fn bell_influence(freq: f64, center_hz: f64, q: f64) -> f64 {
    let center = center_hz.max(20.0);
    let q = q.clamp(0.15, 18.0);
    let w = freq.max(1e-6) / center;
    let denom = 1.0 + q * q * (w - 1.0 / w).powi(2);
    (1.0 / denom.sqrt()).clamp(0.0, 1.0)
}

/// Standard log frequency coordinate mapping (e.g. 20 Hz to 20 kHz)
pub fn freq_x(freq: f64, x: f32, w: f32, min_hz: f64, max_hz: f64) -> f32 {
    let norm = (freq.max(min_hz).ln() - min_hz.ln()) / (max_hz.ln() - min_hz.ln());
    x + norm as f32 * w
}

pub fn x_freq(px: f32, x: f32, w: f32, min_hz: f64, max_hz: f64) -> f64 {
    let norm = ((px - x) / w).clamp(0.0, 1.0) as f64;
    (min_hz.ln() + norm * (max_hz.ln() - min_hz.ln())).exp()
}

pub fn db_y(db: f64, db_range: f64, y: f32, h: f32) -> f32 {
    let center = y + h * 0.5;
    center - (db / db_range) as f32 * (h * 0.5)
}

pub fn y_db(py: f32, db_range: f64, y: f32, h: f32) -> f64 {
    let center = y + h * 0.5;
    ((center - py) / (h * 0.5)) as f64 * db_range
}

/// Flattery blended frequency mapping (85% log, 15% linear)
pub const FLATTERY_FREQ_MAP_LOG_AMOUNT: f64 = 0.85;

pub fn flattery_pos_to_freq(pos: f64, min_hz: f64, max_hz: f64) -> f64 {
    let pos = pos.clamp(0.0, 1.0);
    let lin_part = min_hz + (max_hz - min_hz) * pos;
    let log_part =
        (min_hz.max(1.0).ln() + (max_hz.max(min_hz + 1.0).ln() - min_hz.max(1.0).ln()) * pos).exp();
    lin_part * (1.0 - FLATTERY_FREQ_MAP_LOG_AMOUNT) + log_part * FLATTERY_FREQ_MAP_LOG_AMOUNT
}

pub fn flattery_freq_to_pos(freq: f64, min_hz: f64, max_hz: f64) -> f64 {
    let ln_min = min_hz.max(1.0).ln();
    let ln_max = max_hz.max(min_hz + 1.0).ln();
    let ln_rng = ln_max - ln_min;
    let rng = max_hz - min_hz;
    let mut pos = ((freq.max(1.0).ln() - ln_min) / (ln_rng + 1e-9)).clamp(0.0, 1.0);
    let mut best = pos;
    let mut best_err = (freq - flattery_pos_to_freq(pos, min_hz, max_hz)).abs();

    for _ in 0..8 {
        let test_f = (min_hz + rng * pos) * (1.0 - FLATTERY_FREQ_MAP_LOG_AMOUNT)
            + (ln_min + ln_rng * pos).exp() * FLATTERY_FREQ_MAP_LOG_AMOUNT;
        let err = (freq - test_f).abs();
        if err < best_err {
            best = pos;
            best_err = err;
        }
        let d = rng * (1.0 - FLATTERY_FREQ_MAP_LOG_AMOUNT)
            + ln_rng * (ln_min + ln_rng * pos).exp() * FLATTERY_FREQ_MAP_LOG_AMOUNT;
        pos = (pos + (freq - test_f) / (d + 1e-9)).clamp(0.0, 1.0);
    }
    best
}
