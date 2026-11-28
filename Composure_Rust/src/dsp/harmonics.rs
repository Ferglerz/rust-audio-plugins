//! Harmonic processing ported from `08_harmonic_models.jsfx-inc`.

use super::dsp_utils::{flush_denormal, linear_oversample_avg};

const HARMONIC_OVERSAMPLE: usize = 2;
const DC_BLOCK_HZ: f64 = 8.0;
const MAX_HARMONIC_EXCITATION: f64 = 1.0;

#[derive(Debug, Clone, Default)]
pub struct HarmonicProcessor {
    pub prev: [f64; 2],
    dc_state: [f64; 2],
    dc_coeff: f64,
}

impl HarmonicProcessor {
    pub fn new(srate: f64) -> Self {
        Self {
            dc_coeff: Self::dc_block_coeff(srate),
            ..Self::default()
        }
    }

    pub fn set_sample_rate(&mut self, srate: f64) {
        self.dc_coeff = Self::dc_block_coeff(srate);
    }

    fn dc_block_coeff(srate: f64) -> f64 {
        (-2.0 * std::f64::consts::PI * DC_BLOCK_HZ / srate).exp()
    }

    pub fn reset(&mut self) {
        self.prev = [0.0; 2];
        self.dc_state = [0.0; 2];
    }

    pub(crate) fn track_dry(&mut self, left: f64, right: f64) {
        self.prev = [left, right];
        self.dc_state = [0.0; 2];
    }

    pub fn flush_denormals(&mut self) {
        for v in self.prev.iter_mut().chain(self.dc_state.iter_mut()) {
            flush_denormal(v);
        }
    }
}

#[derive(Clone, Copy)]
struct HarmonicModelCoeffs {
    neg_asym: f64,
    pos_asym: f64,
    x2: f64,
    x3: f64,
    x4: f64,
    x5: f64,
}

const TUBE_COEFFS: HarmonicModelCoeffs = HarmonicModelCoeffs {
    neg_asym: 0.85,
    pos_asym: 1.0,
    x2: 0.01,
    x3: 0.002,
    x4: 0.0,
    x5: 0.001,
};

const TRANSFORMER_COEFFS: HarmonicModelCoeffs = HarmonicModelCoeffs {
    neg_asym: 0.93,
    pos_asym: 1.0,
    x2: 0.006,
    x3: 0.0004,
    x4: 0.0015,
    x5: 0.0,
};

fn harmonic_residual(
    driven: f64,
    amount: f64,
    combined_factor: f64,
    even_boost: f64,
    odd_boost: f64,
    coeffs: HarmonicModelCoeffs,
) -> f64 {
    let scaled_saturation = amount * combined_factor;
    // Post-gain expansion can exceed full scale. Limit only the input to the
    // polynomial so its higher powers cannot overwhelm the clean signal.
    let x = driven.clamp(-MAX_HARMONIC_EXCITATION, MAX_HARMONIC_EXCITATION);
    // Zero disables the odd component. Interpolate to the first selectable
    // setting (1%) so a smoothed control crossing zero fades it in.
    let odd_boost_factor = if odd_boost < 1.0 {
        odd_boost * 1.01
    } else {
        1.0 + odd_boost * 0.01
    };
    let even_boost_factor = 1.0 + even_boost * 0.01;
    let asymmetric_factor = if x > 0.0 {
        coeffs.pos_asym
    } else {
        coeffs.neg_asym
    };

    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x2 * x2;
    let x5 = x3 * x2;

    let mut result = x2 * scaled_saturation * coeffs.x2 * even_boost_factor * asymmetric_factor;

    if coeffs.x4 > 0.0 {
        result += x4 * scaled_saturation * coeffs.x4 * even_boost_factor;
    }
    if odd_boost > 0.0 {
        if coeffs.x3 > 0.0 {
            result += x3 * scaled_saturation * coeffs.x3 * odd_boost_factor * asymmetric_factor;
        }
        if coeffs.x5 > 0.0 {
            result += x5 * scaled_saturation * coeffs.x5 * odd_boost_factor * asymmetric_factor;
        }
    }

    result
}

pub fn apply_enhanced_tube_processing(
    driven: f64,
    amount: f64,
    combined_factor: f64,
    even_boost: f64,
    odd_boost: f64,
) -> f64 {
    driven
        + harmonic_residual(
            driven,
            amount,
            combined_factor,
            even_boost,
            odd_boost,
            TUBE_COEFFS,
        )
}

/// Iron-transformer saturation: even-heavy (2nd/4th), light odd from core asymmetry.
pub fn apply_enhanced_transformer_processing(
    driven: f64,
    amount: f64,
    combined_factor: f64,
    even_boost: f64,
    odd_boost: f64,
) -> f64 {
    driven
        + harmonic_residual(
            driven,
            amount,
            combined_factor,
            even_boost,
            odd_boost,
            TRANSFORMER_COEFFS,
        )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarmonicParams {
    pub harmonic_type: u8,
    pub drive: f64,
    pub mix: f64,
    pub even_boost: f64,
    pub odd_boost: f64,
    pub harmonic_amount: f64,
}

fn dc_block_sample(input: f64, state: &mut f64, coeff: f64) -> f64 {
    let out = input - *state;
    *state = input - out * coeff;
    out
}

pub fn apply_harmonic_stereo(
    processor: &mut HarmonicProcessor,
    input_l: f64,
    input_r: f64,
    params: &HarmonicParams,
    combined_factor: f64,
    intensity: f64,
) -> (f64, f64) {
    (
        apply_harmonic_processing(processor, 0, input_l, params, combined_factor, intensity),
        apply_harmonic_processing(processor, 1, input_r, params, combined_factor, intensity),
    )
}

pub fn apply_harmonic_processing(
    processor: &mut HarmonicProcessor,
    channel: u8,
    input: f64,
    params: &HarmonicParams,
    combined_factor: f64,
    intensity: f64,
) -> f64 {
    let ch = channel as usize;
    let prev = processor.prev[ch];
    processor.prev[ch] = input;

    let drive_factor = (params.drive / 100.0) * 0.5;
    let harmonic_amount_driven = params.harmonic_amount * drive_factor;
    if params.mix <= 0.0001
        || harmonic_amount_driven == 0.0
        || combined_factor == 0.0
        || intensity == 0.0
    {
        processor.dc_state[ch] = 0.0;
        return input;
    }

    let coeffs = if params.harmonic_type == 0 {
        TUBE_COEFFS
    } else {
        TRANSFORMER_COEFFS
    };
    // Integrate just the nonlinear terms. This cancels the matching
    // interpolated clean reference at every oversampling point.
    let residual = linear_oversample_avg(prev, input, HARMONIC_OVERSAMPLE, |x| {
        harmonic_residual(
            x,
            harmonic_amount_driven,
            combined_factor,
            params.even_boost,
            params.odd_boost,
            coeffs,
        )
    });
    let wet_delta = dc_block_sample(
        intensity * residual,
        &mut processor.dc_state[ch],
        processor.dc_coeff,
    );
    input + params.mix * wet_delta
}

pub fn compute_harmonic_modulation(
    target_gr_db: f64,
    global_smoothed_gr_db: f64,
) -> (f64, f64, f64) {
    let harmonic_envelope_amount = global_smoothed_gr_db.abs() / 30.0;
    let harmonic_abs_gr = target_gr_db.abs();
    let harmonic_envelope_factor = 1.0 + harmonic_envelope_amount * 0.2;
    let combined_factor = harmonic_abs_gr * 0.3 * harmonic_envelope_factor;
    let intensity = harmonic_abs_gr * 0.15 * (1.0 + harmonic_envelope_amount * 0.2);
    (combined_factor, intensity, harmonic_envelope_amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_harmonics_fade_in_across_the_first_control_step() {
        let residual = |odd| harmonic_residual(0.8, 1.0, 1.0, 0.0, odd, TUBE_COEFFS);
        assert!((residual(0.5) - 0.5 * (residual(0.0) + residual(1.0))).abs() < 1e-12);
    }

    #[test]
    fn zero_mix_passthrough() {
        let mut proc = HarmonicProcessor::new(48000.0);
        let params = HarmonicParams {
            harmonic_type: 0,
            drive: 50.0,
            mix: 0.0,
            even_boost: 100.0,
            odd_boost: 100.0,
            harmonic_amount: 1.0,
        };
        let out = apply_harmonic_processing(&mut proc, 0, 0.5, &params, 0.1, 0.05);
        assert!((out - 0.5).abs() < 1e-12);
    }

    #[test]
    fn tube_adds_harmonics() {
        let out = apply_enhanced_tube_processing(0.5, 1.0, 0.5, 100.0, 100.0);
        assert!(out != 0.5);
    }

    #[test]
    fn transformer_differs_from_tube() {
        let tube = apply_enhanced_tube_processing(0.4, 1.0, 1.0, 100.0, 0.0);
        let xfmr = apply_enhanced_transformer_processing(0.4, 1.0, 1.0, 100.0, 0.0);
        assert!((xfmr - tube).abs() > 1e-6);
    }

    #[test]
    fn oversample_matches_helper() {
        let mut processor = HarmonicProcessor::new(48_000.0);
        processor.prev[0] = 0.1;
        let out = apply_harmonic_processing(&mut processor, 0, 0.8, &params(0), 1.0, 1.0);
        let wet_avg = linear_oversample_avg(0.1, 0.8, HARMONIC_OVERSAMPLE, |x| {
            apply_enhanced_tube_processing(x, 0.5, 1.0, 100.0, 100.0)
        });
        let clean_avg = linear_oversample_avg(0.1, 0.8, HARMONIC_OVERSAMPLE, |x| x);
        let expected = 0.8 + (wet_avg - clean_avg);
        assert!((out - expected).abs() < 1e-9);
    }

    fn params(harmonic_type: u8) -> HarmonicParams {
        HarmonicParams {
            harmonic_type,
            drive: 100.0,
            mix: 1.0,
            even_boost: 100.0,
            odd_boost: 100.0,
            harmonic_amount: 1.0,
        }
    }

    #[test]
    fn disabled_harmonics_preserve_clean_signal_after_coloration() {
        for harmonic_type in [0, 1] {
            let enabled = params(harmonic_type);
            for disabled in [
                HarmonicParams {
                    mix: 0.0,
                    ..enabled
                },
                HarmonicParams {
                    drive: 0.0,
                    ..enabled
                },
                HarmonicParams {
                    harmonic_amount: 0.0,
                    ..enabled
                },
            ] {
                let mut processor = HarmonicProcessor::new(48_000.0);
                let colored = apply_harmonic_stereo(&mut processor, 0.8, -0.8, &enabled, 4.0, 2.0);
                assert_ne!(colored, (0.8, -0.8));

                for dry in [(0.3, -0.4), (0.3, -0.4), (-0.5, 0.9), (-0.0_f64, 0.0_f64)] {
                    let out =
                        apply_harmonic_stereo(&mut processor, dry.0, dry.1, &disabled, 4.0, 2.0);
                    assert_eq!(
                        (out.0.to_bits(), out.1.to_bits()),
                        (dry.0.to_bits(), dry.1.to_bits())
                    );
                }

                let mut fresh = HarmonicProcessor::new(48_000.0);
                let out = apply_harmonic_stereo(&mut processor, 0.6, -0.9, &enabled, 4.0, 2.0);
                let fresh_out = apply_harmonic_stereo(&mut fresh, 0.6, -0.9, &enabled, 4.0, 2.0);
                assert_eq!(out, fresh_out);
            }
        }
    }

    #[test]
    fn harmonic_residual_stays_bounded_after_fifty_db_expansion() {
        let (combined_factor, intensity, _) = compute_harmonic_modulation(50.0, 50.0);
        let input = 0.25 * 10.0_f64.powf(50.0 / 20.0);
        for harmonic_type in [0, 1] {
            let mut processor = HarmonicProcessor::new(48_000.0);
            let params = params(harmonic_type);
            for sample in [input, input, -input, input, -input, 0.0] {
                let (left, right) = apply_harmonic_stereo(
                    &mut processor,
                    sample,
                    -sample,
                    &params,
                    combined_factor,
                    intensity,
                );
                assert!((left - sample).abs() < 3.0);
                assert!((right + sample).abs() < 3.0);
            }
        }
    }

    #[test]
    fn oversamples_nonlinear_residual_and_preserves_clean_center_sample() {
        let input = 0.8;
        let previous = 0.1;
        for (harmonic_type, coeffs) in [(0, TUBE_COEFFS), (1, TRANSFORMER_COEFFS)] {
            let mut processor = HarmonicProcessor::new(48_000.0);
            processor.prev[0] = previous;
            let params = HarmonicParams {
                mix: 0.5,
                ..params(harmonic_type)
            };
            let residual = [0.275_f64, 0.625].map(|x| {
                coeffs.x2 * x.powi(2)
                    + coeffs.x3 * x.powi(3)
                    + coeffs.x4 * x.powi(4)
                    + coeffs.x5 * x.powi(5)
            });
            let expected = input + params.mix * (residual[0] + residual[1]) * 0.5;
            let out = apply_harmonic_processing(&mut processor, 0, input, &params, 1.0, 1.0);
            assert!((out - expected).abs() < 1e-12);
        }
    }

    #[test]
    fn dc_blocker_removes_only_steady_state_harmonic_residual() {
        let mut processor = HarmonicProcessor::new(48_000.0);
        let params = params(0);
        let input = 0.6;
        let mut output = input;
        for _ in 0..24_000 {
            output = apply_harmonic_processing(&mut processor, 0, input, &params, 4.0, 2.0);
        }
        assert!((output - input).abs() < 1e-10);
    }
}
