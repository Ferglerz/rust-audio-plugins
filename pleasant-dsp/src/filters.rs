use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BiquadKind {
    Bell,
    LowCut,
    HighCut,
    Notch,
    BandPass,
    LowShelf,
    HighShelf,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiquadCoefficients {
    pub b: [f64; 3],
    pub a: [f64; 2],
}

impl BiquadCoefficients {
    pub const IDENTITY: Self = Self {
        b: [1.0, 0.0, 0.0],
        a: [0.0, 0.0],
    };

    pub fn design(
        kind: BiquadKind,
        frequency_hz: f64,
        gain_db: f64,
        q: f64,
        sample_rate: f64,
    ) -> Self {
        let sample_rate = sample_rate.max(1.0);
        let angular = 2.0 * PI * frequency_hz.clamp(5.0, sample_rate * 0.45) / sample_rate;
        let (sin, cos) = angular.sin_cos();
        let alpha = sin / (2.0 * q.clamp(0.15, 18.0));
        let amplitude = 10.0_f64.powf(gain_db / 40.0);
        let (b0, b1, b2, a0, a1, a2) = match kind {
            BiquadKind::Bell => (
                1.0 + alpha * amplitude,
                -2.0 * cos,
                1.0 - alpha * amplitude,
                1.0 + alpha / amplitude,
                -2.0 * cos,
                1.0 - alpha / amplitude,
            ),
            BiquadKind::LowCut => (
                (1.0 + cos) / 2.0,
                -(1.0 + cos),
                (1.0 + cos) / 2.0,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            BiquadKind::HighCut => (
                (1.0 - cos) / 2.0,
                1.0 - cos,
                (1.0 - cos) / 2.0,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            BiquadKind::Notch => (1.0, -2.0 * cos, 1.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
            BiquadKind::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
            BiquadKind::LowShelf | BiquadKind::HighShelf => {
                let beta = 2.0 * amplitude.sqrt() * alpha;
                if kind == BiquadKind::LowShelf {
                    (
                        amplitude * ((amplitude + 1.0) - (amplitude - 1.0) * cos + beta),
                        2.0 * amplitude * ((amplitude - 1.0) - (amplitude + 1.0) * cos),
                        amplitude * ((amplitude + 1.0) - (amplitude - 1.0) * cos - beta),
                        (amplitude + 1.0) + (amplitude - 1.0) * cos + beta,
                        -2.0 * ((amplitude - 1.0) + (amplitude + 1.0) * cos),
                        (amplitude + 1.0) + (amplitude - 1.0) * cos - beta,
                    )
                } else {
                    (
                        amplitude * ((amplitude + 1.0) + (amplitude - 1.0) * cos + beta),
                        -2.0 * amplitude * ((amplitude - 1.0) + (amplitude + 1.0) * cos),
                        amplitude * ((amplitude + 1.0) + (amplitude - 1.0) * cos - beta),
                        (amplitude + 1.0) - (amplitude - 1.0) * cos + beta,
                        2.0 * ((amplitude - 1.0) - (amplitude + 1.0) * cos),
                        (amplitude + 1.0) - (amplitude - 1.0) * cos - beta,
                    )
                }
            }
        };
        Self {
            b: [b0 / a0, b1 / a0, b2 / a0],
            a: [a1 / a0, a2 / a0],
        }
    }

    /// Design Flattery's legacy bell from a linear center gain.
    ///
    /// This keeps its wider Q range and unity behavior separate from `design()`.
    pub fn design_peak_linear(
        frequency_hz: f64,
        gain_linear: f64,
        q: f64,
        sample_rate: f64,
    ) -> Self {
        let sample_rate = sample_rate.max(1.0);
        let angular = 2.0 * PI * frequency_hz / sample_rate;
        let (sin, cos) = angular.sin_cos();
        let alpha = sin / (2.0 * q.max(0.01));
        let amplitude = gain_linear.max(1.0e-9).sqrt();
        let inverse_amplitude = 1.0 / amplitude;
        let b0 = 1.0 + alpha * amplitude;
        let b1 = -2.0 * cos;
        let b2 = 1.0 - alpha * amplitude;
        let a0 = 1.0 + alpha * inverse_amplitude;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha * inverse_amplitude;
        Self {
            b: [b0 / a0, b1 / a0, b2 / a0],
            a: [a1 / a0, a2 / a0],
        }
    }

    pub fn response_db(self, frequency_hz: f64, sample_rate: f64) -> f64 {
        let angular = 2.0 * PI * frequency_hz / sample_rate.max(1.0);
        let real = self.b[0] + self.b[1] * angular.cos() + self.b[2] * (2.0 * angular).cos();
        let imaginary = -self.b[1] * angular.sin() - self.b[2] * (2.0 * angular).sin();
        let denominator_real = 1.0 + self.a[0] * angular.cos() + self.a[1] * (2.0 * angular).cos();
        let denominator_imaginary = -self.a[0] * angular.sin() - self.a[1] * (2.0 * angular).sin();
        10.0 * ((real * real + imaginary * imaginary).max(1.0e-24)
            / (denominator_real * denominator_real + denominator_imaginary * denominator_imaginary)
                .max(1.0e-24))
        .log10()
    }

    pub fn as_direct_form_array(self) -> [f64; 5] {
        [self.b[0], self.b[1], self.b[2], self.a[0], self.a[1]]
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TransposedDirectForm2 {
    z1: f64,
    z2: f64,
}

impl TransposedDirectForm2 {
    #[inline(always)]
    pub fn process(&mut self, input: f64, coefficients: BiquadCoefficients) -> f64 {
        let output = coefficients.b[0] * input + self.z1;
        self.z1 = coefficients.b[1] * input - coefficients.a[0] * output + self.z2;
        self.z2 = coefficients.b[2] * input - coefficients.a[1] * output;
        if output.is_finite() {
            output
        } else {
            self.reset();
            0.0
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn is_finite(&self) -> bool {
        self.z1.is_finite() && self.z2.is_finite()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DirectForm1 {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl DirectForm1 {
    #[inline(always)]
    pub fn process(&mut self, input: f64, coefficients: BiquadCoefficients) -> f64 {
        let output =
            coefficients.b[0] * input + coefficients.b[1] * self.x1 + coefficients.b[2] * self.x2
                - coefficients.a[0] * self.y1
                - coefficients.a[1] * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bell_center_matches_requested_gain() {
        let coefficients =
            BiquadCoefficients::design(BiquadKind::Bell, 1_000.0, 12.0, 0.707, 48_000.0);
        assert!((coefficients.response_db(1_000.0, 48_000.0) - 12.0).abs() < 1.0e-9);
    }

    #[test]
    fn state_topologies_share_impulse_response() {
        let coefficients =
            BiquadCoefficients::design(BiquadKind::Bell, 3_000.0, -6.0, 2.0, 48_000.0);
        let mut direct = DirectForm1::default();
        let mut transposed = TransposedDirectForm2::default();
        for index in 0..128 {
            let input = if index == 0 { 1.0 } else { 0.0 };
            let difference =
                direct.process(input, coefficients) - transposed.process(input, coefficients);
            assert!(difference.abs() < 1.0e-12);
        }
    }
}
