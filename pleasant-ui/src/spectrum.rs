/// Damian analyzer fall: 3 dB per 2048-sample FFT frame.
pub const SPECTRUM_FALL_DB_PER_FRAME: f32 = 3.0;
pub const SPECTRUM_FALL_REF_SAMPLES: f32 = 2048.0;

/// 7-tap weights used to spatially smooth analyzer / lift bins.
pub const SPECTRUM_SMOOTH_WEIGHTS: [f64; 7] = [0.01, 0.06, 0.24, 0.38, 0.24, 0.06, 0.01];

/// Peak-hold with a linear dB fall. Peaks stick, then decay by `fall_db`.
pub fn peak_hold(old: f32, new: f32, fall_db: f32) -> f32 {
    new.max(old - fall_db)
}

/// Scale Damian's 3 dB / 2048-sample fall to an arbitrary hop size.
pub fn spectrum_fall_db(frame_samples: f32) -> f32 {
    SPECTRUM_FALL_DB_PER_FRAME * (frame_samples / SPECTRUM_FALL_REF_SAMPLES).max(0.0)
}

pub fn smooth_bins(raw: &[f64], out: &mut [f64]) {
    let n = raw.len().min(out.len());
    if n == 0 {
        return;
    }
    let last = (n - 1) as isize;
    for (k, dst) in out.iter_mut().take(n).enumerate() {
        let mut sum = 0.0;
        let mut w_sum = 0.0;
        for (i, &w) in SPECTRUM_SMOOTH_WEIGHTS.iter().enumerate() {
            let idx = (k as isize + i as isize - 3).clamp(0, last) as usize;
            sum += raw[idx] * w;
            w_sum += w;
        }
        *dst = sum / w_sum;
    }
}

pub fn smooth_bins_f32(raw: &[f32], out: &mut [f32]) {
    let n = raw.len().min(out.len());
    if n == 0 {
        return;
    }
    let last = (n - 1) as isize;
    for (k, dst) in out.iter_mut().take(n).enumerate() {
        let mut sum = 0.0;
        let mut w_sum = 0.0;
        for (i, &w) in SPECTRUM_SMOOTH_WEIGHTS.iter().enumerate() {
            let idx = (k as isize + i as isize - 3).clamp(0, last) as usize;
            sum += raw[idx] as f64 * w;
            w_sum += w;
        }
        *dst = (sum / w_sum) as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_hold_captures_and_falls() {
        assert!((peak_hold(-12.0, -6.0, 3.0) + 6.0).abs() < f32::EPSILON);
        assert!((peak_hold(-6.0, -20.0, 3.0) + 9.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fall_matches_damian_at_reference_hop() {
        assert!((spectrum_fall_db(2048.0) - 3.0).abs() < 1e-6);
        assert!((spectrum_fall_db(256.0) - 0.375).abs() < 1e-6);
    }

    #[test]
    fn spatial_smooth_preserves_flat_signal() {
        let raw = [4.0_f64; 8];
        let mut out = [0.0; 8];
        smooth_bins(&raw, &mut out);
        for v in out {
            assert!((v - 4.0).abs() < 1e-9);
        }
    }
}
