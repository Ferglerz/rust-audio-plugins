use pleasant_dsp::filters::{BiquadCoefficients, DirectForm1};

#[derive(Clone)]
pub struct PeakingFilter {
    pub center_hz: f64,
    pub gain_linear: f64,
    pub prev_gain_linear: f64,
    pub coeffs: BiquadCoefficients,
    pub state_l: DirectForm1,
    pub state_r: DirectForm1,
    coefficient_inputs: Option<[f64; 4]>,
    #[cfg(test)]
    coefficient_designs: usize,
}

impl PeakingFilter {
    pub fn new(center_hz: f64) -> Self {
        Self {
            center_hz,
            gain_linear: 1.0,
            prev_gain_linear: 1.0,
            coeffs: BiquadCoefficients::IDENTITY,
            state_l: DirectForm1::default(),
            state_r: DirectForm1::default(),
            coefficient_inputs: None,
            #[cfg(test)]
            coefficient_designs: 0,
        }
    }

    pub fn reset(&mut self) {
        self.state_l.reset();
        self.state_r.reset();
        self.gain_linear = 1.0;
        self.prev_gain_linear = 1.0;
        self.coeffs = BiquadCoefficients::IDENTITY;
        self.coefficient_inputs = None;
    }

    pub fn update_coeffs(&mut self, srate: f64, q: f64) {
        let inputs = [self.center_hz, self.gain_linear, q, srate];
        if self.coefficient_inputs == Some(inputs) {
            return;
        }
        self.coefficient_inputs = Some(inputs);
        if (self.gain_linear - 1.0).abs() < 0.001 {
            self.coeffs = BiquadCoefficients::IDENTITY;
            return;
        }
        self.coeffs =
            BiquadCoefficients::design_peak_linear(self.center_hz, self.gain_linear, q, srate);
        #[cfg(test)]
        {
            self.coefficient_designs += 1;
        }
    }

    #[inline(always)]
    pub fn process_diff(&mut self, in_l: f64, in_r: f64) -> (f64, f64) {
        if (self.gain_linear - 1.0).abs() < 0.001 {
            return (0.0, 0.0);
        }
        let out_l = self.state_l.process(in_l, self.coeffs);
        let out_r = self.state_r.process(in_r, self.coeffs);
        (out_l - in_l, out_r - in_r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coefficient_cache_reuses_design_and_invalidates_every_input() {
        let mut filter = PeakingFilter::new(1_000.0);
        let mut sample_rate = 48_000.0;
        let mut q = 10.0;
        filter.gain_linear = 2.0;
        filter.update_coeffs(sample_rate, q);
        for _ in 0..100 {
            filter.update_coeffs(sample_rate, q);
        }
        assert_eq!(filter.coefficient_designs, 1);

        for change in 0..4 {
            match change {
                0 => filter.gain_linear = 0.5,
                1 => filter.center_hz = 2_000.0,
                2 => q = 4.0,
                _ => sample_rate = 96_000.0,
            }
            filter.update_coeffs(sample_rate, q);
            assert_eq!(filter.coefficient_designs, change + 2);
            assert_eq!(
                filter.coeffs,
                BiquadCoefficients::design_peak_linear(
                    filter.center_hz,
                    filter.gain_linear,
                    q,
                    sample_rate,
                )
            );
        }

        filter.gain_linear = 1.0005;
        filter.update_coeffs(sample_rate, q);
        assert_eq!(filter.coeffs, BiquadCoefficients::IDENTITY);
        assert_eq!(filter.coefficient_designs, 5);
        filter.gain_linear = 0.5;
        filter.update_coeffs(sample_rate, q);
        assert_eq!(filter.coefficient_designs, 6);

        filter.reset();
        assert_eq!(filter.coeffs, BiquadCoefficients::IDENTITY);
        assert_eq!(filter.gain_linear, 1.0);
        assert_eq!(filter.prev_gain_linear, 1.0);
        filter.gain_linear = 0.5;
        filter.update_coeffs(sample_rate, q);
        assert_eq!(filter.coefficient_designs, 7);
    }

    #[test]
    fn coefficient_cache_preserves_audio_across_unity_changes_and_reset() {
        let mut cached = PeakingFilter::new(1_000.0);
        let mut reference_l = DirectForm1::default();
        let mut reference_r = DirectForm1::default();
        for block in 0..12 {
            if block == 9 {
                cached.reset();
                reference_l.reset();
                reference_r.reset();
            }
            // Include both sides of the existing strict unity threshold and
            // return to the same non-unity design after bypass and reset.
            let gain = [
                2.0, 2.0, 1.0005, 0.5, 0.9995, 0.9989, 1.0011, 0.5, 0.5, 0.5, 2.0, 2.0,
            ][block];
            let frequency = if block < 7 { 1_000.0 } else { 2_000.0 };
            let q = if block < 8 { 10.0 } else { 4.0 };
            let sample_rate = if block < 10 { 48_000.0 } else { 96_000.0 };
            cached.center_hz = frequency;
            cached.gain_linear = gain;
            for sample in 0..64 {
                cached.update_coeffs(sample_rate, q);
                let input_l = ((block * 64 + sample) as f64 * 0.13).sin();
                let input_r = ((block * 64 + sample) as f64 * 0.19).cos() * 0.4;
                let expected = if (gain - 1.0).abs() < 0.001 {
                    (0.0, 0.0)
                } else {
                    let coefficients =
                        BiquadCoefficients::design_peak_linear(frequency, gain, q, sample_rate);
                    (
                        reference_l.process(input_l, coefficients) - input_l,
                        reference_r.process(input_r, coefficients) - input_r,
                    )
                };
                assert_eq!(cached.process_diff(input_l, input_r), expected);
            }
        }
    }
}
