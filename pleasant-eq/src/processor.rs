use crate::{BandSettings, EqShape};
use pleasant_dsp::{
    filters::{BiquadCoefficients, BiquadKind, TransposedDirectForm2},
    units::linear_to_db_with_floor,
};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandCoefficients {
    stages: [BiquadCoefficients; 4],
    length: usize,
}

impl BandCoefficients {
    pub fn prepare(settings: &BandSettings, sample_rate: f64) -> Self {
        let mut result = Self {
            stages: [BiquadCoefficients::IDENTITY; 4],
            length: 1,
        };
        if !settings.shape.is_cut() {
            result.stages[0] = BiquadCoefficients::design(
                settings.shape.biquad_kind(),
                settings.frequency_hz,
                settings.gain_db,
                settings.q,
                sample_rate,
            );
            return result;
        }

        let order = settings.order.clamp(1, 8) as usize;
        result.length = order.div_ceil(2);
        for index in 0..order / 2 {
            let q = 1.0 / (2.0 * (PI * (2 * index + 1) as f64 / (2 * order) as f64).sin());
            let q = if index == 0 {
                q * settings.q / std::f64::consts::FRAC_1_SQRT_2
            } else {
                q
            };
            result.stages[index] = BiquadCoefficients::design(
                settings.shape.biquad_kind(),
                settings.frequency_hz,
                0.0,
                q,
                sample_rate,
            );
        }
        if order % 2 == 1 {
            let frequency = settings
                .frequency_hz
                .clamp(5.0, sample_rate.max(1.0) * 0.45);
            let k = (PI * frequency / sample_rate.max(1.0)).tan();
            let normalization = 1.0 / (1.0 + k);
            let (b0, b1) = if settings.shape == EqShape::LowCut {
                (normalization, -normalization)
            } else {
                (k * normalization, k * normalization)
            };
            result.stages[order / 2] = BiquadCoefficients {
                b: [b0, b1, 0.0],
                a: [(k - 1.0) * normalization, 0.0],
            };
        }
        result
    }

    pub fn response_db(self, frequency_hz: f64, sample_rate: f64) -> f64 {
        self.stages[..self.length]
            .iter()
            .map(|coefficients| coefficients.response_db(frequency_hz, sample_rate))
            .sum()
    }

    pub fn stage_count(self) -> usize {
        self.length
    }

    pub fn process(self, states: &mut [TransposedDirectForm2; 4], mut input: f64) -> f64 {
        for (state, coefficients) in states.iter_mut().zip(&self.stages[..self.length]) {
            input = state.process(input, *coefficients);
        }
        input
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Cascade {
    stages: [TransposedDirectForm2; 4],
}

impl Cascade {
    fn process(&mut self, mut input: f64, coefficients: BandCoefficients) -> f64 {
        for (state, coefficients) in self
            .stages
            .iter_mut()
            .zip(&coefficients.stages[..coefficients.length])
        {
            input = state.process(input, *coefficients);
        }
        input
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Debug)]
pub struct BandProcessor {
    settings: BandSettings,
    filters: [Cascade; 2],
    detectors: [TransposedDirectForm2; 2],
    coefficients: BandCoefficients,
    detector_coefficients: BiquadCoefficients,
    envelope: f64,
    reduction_db: f64,
    uncapped_reduction_db: f64,
    current_gain_db: f64,
    current_frequency_hz: f64,
    current_q: f64,
    attack_coefficient: f64,
    release_coefficient: f64,
    coefficient_phase: usize,
    coefficients_dirty: bool,
    coefficient_sample_rate: f64,
}

impl BandProcessor {
    pub fn new(mut settings: BandSettings, sample_rate: f64) -> Self {
        settings.sanitize();
        Self {
            filters: [Cascade::default(); 2],
            detectors: [TransposedDirectForm2::default(); 2],
            coefficients: BandCoefficients::prepare(&settings, sample_rate),
            detector_coefficients: BiquadCoefficients::design(
                BiquadKind::BandPass,
                settings.frequency_hz,
                0.0,
                settings.q,
                sample_rate,
            ),
            envelope: 0.0,
            reduction_db: 0.0,
            uncapped_reduction_db: 0.0,
            current_gain_db: settings.gain_db,
            current_frequency_hz: settings.frequency_hz,
            current_q: settings.q,
            attack_coefficient: (-1.0 / (sample_rate * settings.attack_ms * 0.001)).exp(),
            release_coefficient: (-1.0 / (sample_rate * settings.release_ms * 0.001)).exp(),
            coefficient_phase: 0,
            coefficients_dirty: false,
            coefficient_sample_rate: sample_rate,
            settings,
        }
    }

    pub fn settings(&self) -> &BandSettings {
        &self.settings
    }

    pub fn reduction_db(&self) -> f64 {
        self.reduction_db
    }

    pub fn uncapped_reduction_db(&self) -> f64 {
        self.uncapped_reduction_db
    }

    pub fn current_gain_db(&self) -> f64 {
        self.current_gain_db
    }

    pub fn coefficient_stage_count(&self) -> usize {
        self.coefficients.stage_count()
    }

    pub fn inherit_state_from(&mut self, previous: &Self) {
        if self.settings.shape == previous.settings.shape
            && self.settings.order == previous.settings.order
        {
            self.filters = previous.filters;
            self.detectors = previous.detectors;
            self.envelope = previous.envelope;
            self.reduction_db = previous.reduction_db;
            self.uncapped_reduction_db = previous.uncapped_reduction_db;
            self.current_gain_db = previous.current_gain_db;
            self.current_frequency_hz = previous.current_frequency_hz;
            self.current_q = previous.current_q;
            self.coefficients = previous.coefficients;
            // A new processor may use a different sample rate or settings.
            self.coefficients_dirty = true;
        }
    }

    pub fn reset(&mut self) {
        for filter in &mut self.filters {
            filter.reset();
        }
        for detector in &mut self.detectors {
            detector.reset();
        }
        self.envelope = 0.0;
        self.reduction_db = 0.0;
        self.uncapped_reduction_db = 0.0;
    }

    pub fn process(&mut self, mut input: [f64; 2], sample_rate: f64) -> [f64; 2] {
        if !self.settings.enabled {
            return input;
        }

        #[cfg(feature = "dynamic")]
        if self.settings.dynamic && self.settings.shape.has_gain() {
            let mut peak = 0.0_f64;
            for (channel, sample) in input.iter().enumerate() {
                peak = peak.max(
                    self.detectors[channel]
                        .process(*sample, self.detector_coefficients)
                        .abs(),
                );
            }
            let coefficient = if peak > self.envelope {
                self.attack_coefficient
            } else {
                self.release_coefficient
            };
            self.envelope = coefficient * self.envelope + (1.0 - coefficient) * peak;
            let maximum = self.settings.range_db.abs();
            let unlimited = (linear_to_db_with_floor(self.envelope, 1.0e-12)
                - self.settings.threshold_db)
                .max(0.0)
                * (1.0 - 1.0 / self.settings.ratio);
            let amount = unlimited.min(maximum);
            if self.settings.range_db >= 0.0 {
                self.reduction_db = amount;
                self.uncapped_reduction_db = unlimited;
            } else {
                self.reduction_db = -amount;
                self.uncapped_reduction_db = -unlimited;
            }
        } else {
            self.reduction_db = 0.0;
            self.uncapped_reduction_db = 0.0;
        }

        if self.coefficient_phase == 0 {
            #[cfg(feature = "dynamic")]
            let time_varying = self.settings.dynamic && self.settings.shape.has_gain();
            #[cfg(not(feature = "dynamic"))]
            let time_varying = false;

            let target_gain_db = self.settings.gain_db - self.reduction_db;
            let settled = (self.current_gain_db - target_gain_db).abs() <= 1.0e-9
                && (self.current_frequency_hz - self.settings.frequency_hz).abs() <= 1.0e-9
                && (self.current_q - self.settings.q).abs() <= 1.0e-9;
            if time_varying
                || self.coefficients_dirty
                || self.coefficient_sample_rate != sample_rate
                || !settled
            {
                let smoothing = (-32.0 / (sample_rate * 0.015)).exp();
                self.current_gain_db =
                    smoothing * self.current_gain_db + (1.0 - smoothing) * target_gain_db;
                self.current_frequency_hz = smoothing * self.current_frequency_hz
                    + (1.0 - smoothing) * self.settings.frequency_hz;
                self.current_q = smoothing * self.current_q + (1.0 - smoothing) * self.settings.q;
                if !time_varying
                    && (self.current_gain_db - target_gain_db).abs() <= 1.0e-9
                    && (self.current_frequency_hz - self.settings.frequency_hz).abs() <= 1.0e-9
                    && (self.current_q - self.settings.q).abs() <= 1.0e-9
                {
                    self.current_gain_db = target_gain_db;
                    self.current_frequency_hz = self.settings.frequency_hz;
                    self.current_q = self.settings.q;
                }
                let mut smoothed = self.settings.clone();
                smoothed.frequency_hz = self.current_frequency_hz;
                smoothed.gain_db = self.current_gain_db;
                smoothed.q = self.current_q;
                self.coefficients = BandCoefficients::prepare(&smoothed, sample_rate);
                self.coefficients_dirty = false;
                self.coefficient_sample_rate = sample_rate;
            }
        }
        self.coefficient_phase = (self.coefficient_phase + 1) % 32;
        for (channel, sample) in input.iter_mut().enumerate() {
            *sample = self.filters[channel].process(*sample, self.coefficients);
        }
        input
    }
}

pub struct StaticEqProcessor {
    bands: Vec<BandProcessor>,
    sample_rate: f64,
}

impl StaticEqProcessor {
    pub fn prepare(settings: &[BandSettings], sample_rate: f64) -> Self {
        Self {
            bands: settings
                .iter()
                .cloned()
                .map(|settings| BandProcessor::new(settings, sample_rate))
                .collect(),
            sample_rate,
        }
    }

    pub fn process_sample(&mut self, mut input: [f64; 2]) -> [f64; 2] {
        for band in &mut self.bands {
            input = band.process(input, self.sample_rate);
        }
        input
    }

    pub fn reset(&mut self) {
        for band in &mut self.bands {
            band.reset();
        }
    }

    pub fn latency_samples(&self) -> u32 {
        0
    }

    pub fn response_db(&self, frequency_hz: f64) -> f64 {
        self.bands
            .iter()
            .filter(|band| band.settings.enabled)
            .map(|band| {
                BandCoefficients::prepare(&band.settings, self.sample_rate)
                    .response_db(frequency_hz, self.sample_rate)
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pleasant_dsp::units::db_to_linear;

    #[test]
    fn response_and_runtime_match() {
        let sample_rate = 48_000.0;
        for shape in [
            EqShape::Bell,
            EqShape::LowShelf,
            EqShape::HighShelf,
            EqShape::LowCut,
            EqShape::HighCut,
            EqShape::Notch,
            EqShape::BandPass,
        ] {
            let settings = BandSettings {
                shape,
                gain_db: 6.0,
                q: 0.707,
                ..BandSettings::default()
            };
            let coefficients = BandCoefficients::prepare(&settings, sample_rate);
            let mut processor = BandProcessor::new(settings, sample_rate);
            let mut real = 0.0;
            let mut imaginary = 0.0;
            for index in 0..8192 {
                let output =
                    processor.process([if index == 0 { 1.0 } else { 0.0 }; 2], sample_rate)[0];
                let angular = 2.0 * PI * 1_000.0 * index as f64 / sample_rate;
                real += output * angular.cos();
                imaginary -= output * angular.sin();
            }
            let expected = db_to_linear(coefficients.response_db(1_000.0, sample_rate));
            assert!((real.hypot(imaginary) - expected).abs() < 1.0e-6);
        }
    }

    #[test]
    fn dynamic_band_reduces_and_releases() {
        let sample_rate = 48_000.0;
        let mut processor = BandProcessor::new(
            BandSettings {
                dynamic: true,
                threshold_db: -30.0,
                attack_ms: 1.0,
                release_ms: 10.0,
                ..BandSettings::default()
            },
            sample_rate,
        );
        for index in 0..48_000 {
            let sample = (2.0 * PI * 1_000.0 * index as f64 / sample_rate).sin() * 0.5;
            processor.process([sample; 2], sample_rate);
        }
        assert!(processor.reduction_db() > 5.0);
        for _ in 0..48_000 {
            processor.process([0.0; 2], sample_rate);
        }
        assert!(processor.reduction_db() < 0.01);
    }

    #[test]
    fn inherited_static_band_uses_new_sample_rate_and_settles() {
        let old_settings = BandSettings {
            shape: EqShape::Bell,
            frequency_hz: 800.0,
            gain_db: 0.0,
            ..BandSettings::default()
        };
        let new_settings = BandSettings {
            frequency_hz: 1_200.0,
            gain_db: 6.0,
            ..old_settings.clone()
        };
        let old = BandProcessor::new(old_settings, 44_100.0);
        let mut current = BandProcessor::new(new_settings.clone(), 96_000.0);
        current.inherit_state_from(&old);
        for _ in 0..96_000 {
            current.process([0.0; 2], 96_000.0);
        }
        let expected = BandCoefficients::prepare(&new_settings, 96_000.0);
        let actual = current.coefficients.response_db(1_200.0, 96_000.0);
        assert!((actual - expected.response_db(1_200.0, 96_000.0)).abs() < 1.0e-9);
        assert!(!current.coefficients_dirty);
    }

    #[test]
    fn static_band_reprepares_when_process_sample_rate_changes() {
        let settings = BandSettings {
            shape: EqShape::Bell,
            frequency_hz: 1_200.0,
            gain_db: 6.0,
            ..BandSettings::default()
        };
        let mut processor = BandProcessor::new(settings.clone(), 44_100.0);
        processor.process([0.0; 2], 96_000.0);

        let expected =
            BandCoefficients::prepare(&settings, 96_000.0).response_db(1_200.0, 96_000.0);
        let actual = processor.coefficients.response_db(1_200.0, 96_000.0);
        assert!((actual - expected).abs() < 1.0e-9);
    }
}

#[cfg(test)]
mod extended_gain_tests {
    use super::*;

    #[test]
    fn processor_preserves_extended_gain_and_range() {
        for gain in [-72.0, -36.0, 36.0, 72.0] {
            let settings = BandSettings { gain_db: gain, range_db: gain * 2.0, ..BandSettings::default() };
            let mut processor = BandProcessor::new(settings, 48000.0);
            assert_eq!(processor.settings().gain_db, gain);
            assert_eq!(processor.settings().range_db, gain * 2.0);
            let coefficients = BandCoefficients::prepare(processor.settings(), 48000.0);
            assert!((coefficients.response_db(1000.0, 48000.0) - gain).abs() < 0.01);
            for i in 0..4096 {
                let out = processor.process([if i == 0 {0.01} else {0.0}; 2], 48000.0);
                assert!(out.iter().all(|sample| sample.is_finite()));
            }
        }
    }
}
