//! Harmonic processing ported from `08_harmonic_models.jsfx-inc`.

use crate::params::{ComposureParams, HarmonicType};

use super::dsp_utils::{flush_denormal, linear_oversample_avg};

const HARMONIC_OVERSAMPLE: usize = 2;
const DC_BLOCK_HZ: f64 = 8.0;

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

fn apply_harmonic_model(
    driven: f64,
    amount: f64,
    combined_factor: f64,
    even_boost: f64,
    odd_boost: f64,
    coeffs: HarmonicModelCoeffs,
) -> f64 {
    let scaled_saturation = amount * combined_factor;
    let x = driven;
    let odd_boost_factor = 1.0 + odd_boost * 0.01;
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

    let mut result = x + x2 * scaled_saturation * coeffs.x2 * even_boost_factor * asymmetric_factor;

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
    apply_harmonic_model(
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
    apply_harmonic_model(
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

impl HarmonicParams {
    pub fn from_plugin(plugin_params: &ComposureParams) -> Self {
        let harmonic_type = match plugin_params.harmonic_type.value() {
            HarmonicType::Tube => 0,
            HarmonicType::Transformer => 1,
        };
        Self {
            harmonic_type,
            drive: plugin_params.harmonic_drive.value() as f64,
            mix: plugin_params.harmonic_mix.value() as f64 / 100.0,
            even_boost: plugin_params.harmonic_even_boost.value() as f64,
            odd_boost: plugin_params.harmonic_odd_boost.value() as f64,
            harmonic_amount: plugin_params.harmonic_amount.value() as f64,
        }
    }
}

fn saturate_sample(
    x: f64,
    harmonic_type: u8,
    harmonic_amount_driven: f64,
    combined_factor: f64,
    even_boost: f64,
    odd_boost: f64,
) -> f64 {
    if harmonic_type == 0 {
        apply_enhanced_tube_processing(
            x,
            harmonic_amount_driven,
            combined_factor,
            even_boost,
            odd_boost,
        )
    } else {
        apply_enhanced_transformer_processing(
            x,
            harmonic_amount_driven,
            combined_factor,
            even_boost,
            odd_boost,
        )
    }
}

fn oversample_saturate(
    input: f64,
    prev: f64,
    harmonic_type: u8,
    harmonic_amount_driven: f64,
    combined_factor: f64,
    even_boost: f64,
    odd_boost: f64,
) -> f64 {
    linear_oversample_avg(prev, input, HARMONIC_OVERSAMPLE, |x| {
        saturate_sample(
            x,
            harmonic_type,
            harmonic_amount_driven,
            combined_factor,
            even_boost,
            odd_boost,
        )
    })
}

fn dc_block_sample(input: f64, state: &mut f64, coeff: f64) -> f64 {
    let out = input - *state;
    *state = input - out * coeff;
    out
}

fn apply_harmonic_channel(
    input: f64,
    prev: f64,
    dc_state: &mut f64,
    dc_coeff: f64,
    params: &HarmonicParams,
    harmonic_amount_driven: f64,
    combined_factor: f64,
    intensity: f64,
) -> f64 {
    let processed = oversample_saturate(
        input,
        prev,
        params.harmonic_type,
        harmonic_amount_driven,
        combined_factor,
        params.even_boost,
        params.odd_boost,
    );

    let wet_delta = intensity * (processed - input);
    let wet = dc_block_sample(input + wet_delta, dc_state, dc_coeff);
    params.mix * wet + (1.0 - params.mix) * input
}

pub fn apply_harmonic_stereo(
    processor: &mut HarmonicProcessor,
    input_l: f64,
    input_r: f64,
    params: &HarmonicParams,
    combined_factor: f64,
    intensity: f64,
) -> (f64, f64) {
    if params.mix <= 0.0001 {
        return (input_l, input_r);
    }

    let drive_factor = (params.drive / 100.0) * 0.5;
    let harmonic_amount_driven = params.harmonic_amount * drive_factor;
    let inputs = [input_l, input_r];
    let mut outputs = [0.0_f64; 2];

    for ch in 0..2 {
        outputs[ch] = apply_harmonic_channel(
            inputs[ch],
            processor.prev[ch],
            &mut processor.dc_state[ch],
            processor.dc_coeff,
            params,
            harmonic_amount_driven,
            combined_factor,
            intensity,
        );
        processor.prev[ch] = inputs[ch];
    }

    (outputs[0], outputs[1])
}

pub fn apply_harmonic_processing(
    processor: &mut HarmonicProcessor,
    channel: u8,
    input: f64,
    params: &HarmonicParams,
    combined_factor: f64,
    intensity: f64,
) -> f64 {
    if params.mix <= 0.0001 {
        return input;
    }

    let prev = processor.prev[channel as usize];
    let drive_factor = (params.drive / 100.0) * 0.5;
    let harmonic_amount_driven = params.harmonic_amount * drive_factor;
    let ch = channel as usize;
    let out = apply_harmonic_channel(
        input,
        prev,
        &mut processor.dc_state[ch],
        processor.dc_coeff,
        params,
        harmonic_amount_driven,
        combined_factor,
        intensity,
    );

    processor.prev[ch] = input;

    out
}

pub fn compute_harmonic_modulation(target_gr_db: f64, global_smoothed_gr_db: f64) -> (f64, f64, f64) {
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
        let out = oversample_saturate(0.8, 0.1, 0, 0.5, 1.0, 100.0, 100.0);
        let avg = linear_oversample_avg(0.1, 0.8, HARMONIC_OVERSAMPLE, |x| {
            apply_enhanced_tube_processing(x, 0.5, 1.0, 100.0, 100.0)
        });
        assert!((out - avg).abs() < 1e-9);
    }
}
