pub mod hihat;
pub mod kick_sine;
pub mod simd;
pub mod voice;
pub mod voice_pool;

pub use hihat::HiHatTracker;
pub use kick_sine::KickSine;
pub use voice::Voice;
pub use voice_pool::VoicePool;

#[inline]
pub fn exp_coeff(time_ms: f32, sample_rate: f32) -> f32 {
    (-1.0 / (time_ms.max(0.001) * 0.001 * sample_rate)).exp()
}

#[inline]
pub fn semitone_ratio(semi: f32) -> f32 {
    2.0_f32.powf(semi / 12.0)
}

#[inline]
pub fn semitone_ratio_f64(semi: f64) -> f64 {
    2.0_f64.powf(semi / 12.0)
}

#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_db_is_one() {
        assert!((db_to_gain(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn octave_is_two() {
        assert!((semitone_ratio(12.0) - 2.0).abs() < 1e-6);
    }
}
