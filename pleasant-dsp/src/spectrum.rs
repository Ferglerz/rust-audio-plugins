/// Damian analyzer fall: 3 dB per 2048-sample FFT frame.
pub const SPECTRUM_FALL_DB_PER_FRAME: f32 = 3.0;
pub const SPECTRUM_FALL_REF_SAMPLES: f32 = 2048.0;

/// Seven-tap spatial smoothing kernel.
pub const SPECTRUM_SMOOTH_WEIGHTS: [f64; 7] = [0.01, 0.06, 0.24, 0.38, 0.24, 0.06, 0.01];

pub fn peak_hold(old: f32, new: f32, fall_db: f32) -> f32 {
    new.max(old - fall_db)
}

pub fn spectrum_fall_db(frame_samples: f32) -> f32 {
    SPECTRUM_FALL_DB_PER_FRAME * (frame_samples / SPECTRUM_FALL_REF_SAMPLES).max(0.0)
}

pub fn smooth_bins(raw: &[f64], out: &mut [f64]) {
    smooth(raw, out, |value| value, |value| value);
}

pub fn smooth_bins_f32(raw: &[f32], out: &mut [f32]) {
    smooth(raw, out, |value| value as f64, |value| value as f32);
}

fn smooth<T: Copy>(
    raw: &[T],
    out: &mut [T],
    to_f64: impl Fn(T) -> f64,
    from_f64: impl Fn(f64) -> T,
) {
    let count = raw.len().min(out.len());
    if count == 0 {
        return;
    }
    let last = (count - 1) as isize;
    for (bin, destination) in out.iter_mut().take(count).enumerate() {
        let mut sum = 0.0;
        let mut weight_sum = 0.0;
        for (offset, &weight) in SPECTRUM_SMOOTH_WEIGHTS.iter().enumerate() {
            let index = (bin as isize + offset as isize - 3).clamp(0, last) as usize;
            sum += to_f64(raw[index]) * weight;
            weight_sum += weight;
        }
        *destination = from_f64(sum / weight_sum);
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
        assert!((spectrum_fall_db(2048.0) - 3.0).abs() < 1.0e-6);
        assert!((spectrum_fall_db(256.0) - 0.375).abs() < 1.0e-6);
    }

    #[test]
    fn spatial_smooth_preserves_flat_signal() {
        let raw = [4.0_f64; 8];
        let mut out = [0.0; 8];
        smooth_bins(&raw, &mut out);
        for value in out {
            assert!((value - 4.0).abs() < 1.0e-9);
        }
    }
}
