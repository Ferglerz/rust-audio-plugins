/// Four-point Hermite interpolation between `y1` and `y2`.
#[inline(always)]
pub fn hermite_f32(y0: f32, y1: f32, y2: f32, y3: f32, fraction: f32) -> f32 {
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * fraction + c2) * fraction + c1) * fraction + y1
}

/// Linear interpolation without clamping the fraction.
#[inline(always)]
pub fn linear_f32(start: f32, end: f32, fraction: f32) -> f32 {
    start + (end - start) * fraction
}

/// A finite linear ramp shared by the continuous controls of a DSP stage.
/// A new target starts from the current interpolated values, even mid-ramp.
pub struct ControlRamp {
    length: u32,
    remaining: u32,
    running: bool,
}

impl ControlRamp {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            length: (0.005 * sample_rate).round().max(1.) as u32,
            remaining: 0,
            running: false,
        }
    }

    /// Returns true when the target should be applied immediately (before
    /// rendering starts, or after a transport reset).
    pub fn retarget(&mut self) -> bool {
        if self.running {
            self.remaining = self.length;
        }
        !self.running
    }

    pub fn reset(&mut self) {
        self.running = false;
        self.remaining = 0;
    }

    /// Apply value += (target - value) * fraction, once per sample.
    pub fn step(&mut self) -> Option<f64> {
        self.running = true;
        if self.remaining == 0 {
            return None;
        }
        let fraction = 1. / f64::from(self.remaining);
        self.remaining -= 1;
        Some(fraction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolation_endpoints_match() {
        assert!((hermite_f32(0.0, 1.0, 2.0, 3.0, 0.0) - 1.0).abs() < 1.0e-6);
        assert!((hermite_f32(0.0, 1.0, 2.0, 3.0, 1.0) - 2.0).abs() < 1.0e-6);
        assert_eq!(linear_f32(1.0, 2.0, 0.0), 1.0);
        assert_eq!(linear_f32(1.0, 2.0, 1.0), 2.0);
    }
}
