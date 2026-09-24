//! Shared quarter-second, quintic slide for graph pages.
pub const DURATION: f32 = 0.25;

pub fn ease(progress: f32) -> f32 {
    let t = progress.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

pub fn position(start: f32, target: f32, elapsed: f32) -> f32 {
    start + (target - start) * ease(elapsed / DURATION)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slide_starts_and_settles_without_overshoot() {
        assert_eq!(position(0.0, 1.0, 0.0), 0.0);
        assert_eq!(position(0.0, 1.0, DURATION / 2.0), 0.5);
        assert_eq!(position(0.0, 1.0, DURATION), 1.0);
        assert_eq!(position(0.5, 0.0, DURATION), 0.0);
        assert_eq!(position(0.0, 1.0, 10.0), 1.0);
    }
}
