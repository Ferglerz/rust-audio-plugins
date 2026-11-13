use pleasant_dsp::filters::{BiquadCoefficients, DirectForm1};

#[derive(Clone)]
pub struct PeakingFilter {
    pub center_hz: f64,
    pub gain_linear: f64,
    pub prev_gain_linear: f64,
    pub coeffs: BiquadCoefficients,
    pub state_l: DirectForm1,
    pub state_r: DirectForm1,
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
        }
    }

    pub fn reset(&mut self) {
        self.state_l.reset();
        self.state_r.reset();
        self.gain_linear = 1.0;
        self.prev_gain_linear = 1.0;
        self.coeffs = BiquadCoefficients::IDENTITY;
    }

    pub fn update_coeffs(&mut self, srate: f64, q: f64) {
        if (self.gain_linear - 1.0).abs() < 0.001 {
            self.coeffs = BiquadCoefficients::IDENTITY;
            return;
        }
        self.coeffs =
            BiquadCoefficients::design_peak_linear(self.center_hz, self.gain_linear, q, srate);
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
