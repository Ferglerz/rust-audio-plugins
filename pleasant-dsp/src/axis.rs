/// Damian-style peaking influence in `0.0..=1.0`.
pub fn bell_influence(freq: f64, center_hz: f64, q: f64) -> f64 {
    let center = center_hz.max(20.0);
    let q = q.clamp(0.15, 18.0);
    let w = freq.max(1.0e-6) / center;
    let denominator = 1.0 + q * q * (w - 1.0 / w).powi(2);
    (1.0 / denominator.sqrt()).clamp(0.0, 1.0)
}

/// Standard logarithmic frequency coordinate mapping.
pub fn freq_x(freq: f64, x: f32, width: f32, min_hz: f64, max_hz: f64) -> f32 {
    let normalized = (freq.max(min_hz).ln() - min_hz.ln()) / (max_hz.ln() - min_hz.ln());
    x + normalized as f32 * width
}

pub fn x_freq(px: f32, x: f32, width: f32, min_hz: f64, max_hz: f64) -> f64 {
    let normalized = ((px - x) / width).clamp(0.0, 1.0) as f64;
    (min_hz.ln() + normalized * (max_hz.ln() - min_hz.ln())).exp()
}

pub fn db_y(db: f64, db_range: f64, y: f32, height: f32) -> f32 {
    let center = y + height * 0.5;
    center - (db / db_range) as f32 * (height * 0.5)
}

pub fn y_db(py: f32, db_range: f64, y: f32, height: f32) -> f64 {
    let center = y + height * 0.5;
    ((center - py) / (height * 0.5)) as f64 * db_range
}

/// Blend used by Flattery's frequency graph.
pub const FLATTERY_FREQ_MAP_LOG_AMOUNT: f64 = 0.85;

pub fn flattery_pos_to_freq(pos: f64, min_hz: f64, max_hz: f64) -> f64 {
    let pos = pos.clamp(0.0, 1.0);
    let linear = min_hz + (max_hz - min_hz) * pos;
    let logarithmic =
        (min_hz.max(1.0).ln() + (max_hz.max(min_hz + 1.0).ln() - min_hz.max(1.0).ln()) * pos).exp();
    linear * (1.0 - FLATTERY_FREQ_MAP_LOG_AMOUNT) + logarithmic * FLATTERY_FREQ_MAP_LOG_AMOUNT
}

pub fn flattery_freq_to_pos(freq: f64, min_hz: f64, max_hz: f64) -> f64 {
    let log_min = min_hz.max(1.0).ln();
    let log_max = max_hz.max(min_hz + 1.0).ln();
    let log_range = log_max - log_min;
    let range = max_hz - min_hz;
    let mut pos = ((freq.max(1.0).ln() - log_min) / (log_range + 1.0e-9)).clamp(0.0, 1.0);
    let mut best = pos;
    let mut best_error = (freq - flattery_pos_to_freq(pos, min_hz, max_hz)).abs();

    for _ in 0..8 {
        let test_frequency = (min_hz + range * pos) * (1.0 - FLATTERY_FREQ_MAP_LOG_AMOUNT)
            + (log_min + log_range * pos).exp() * FLATTERY_FREQ_MAP_LOG_AMOUNT;
        let error = (freq - test_frequency).abs();
        if error < best_error {
            best = pos;
            best_error = error;
        }
        let derivative = range * (1.0 - FLATTERY_FREQ_MAP_LOG_AMOUNT)
            + log_range * (log_min + log_range * pos).exp() * FLATTERY_FREQ_MAP_LOG_AMOUNT;
        pos = (pos + (freq - test_frequency) / (derivative + 1.0e-9)).clamp(0.0, 1.0);
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logarithmic_axis_round_trips() {
        for frequency in [20.0, 100.0, 1_000.0, 10_000.0, 20_000.0] {
            let x = freq_x(frequency, 11.0, 873.0, 20.0, 20_000.0);
            let round_trip = x_freq(x, 11.0, 873.0, 20.0, 20_000.0);
            assert!((round_trip - frequency).abs() / frequency < 1.0e-6);
        }
    }

    #[test]
    fn flattery_axis_round_trips() {
        for frequency in [20.0, 100.0, 1_000.0, 10_000.0, 20_000.0] {
            let pos = flattery_freq_to_pos(frequency, 20.0, 20_000.0);
            let round_trip = flattery_pos_to_freq(pos, 20.0, 20_000.0);
            assert!((round_trip - frequency).abs() < 1.0e-5);
        }
    }
}
