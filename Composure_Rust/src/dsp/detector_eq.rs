//! Fixed band storage keeps detector-EQ updates and processing allocation-free.
use crate::detector_eq::EQ_BANDS;
use pleasant_eq::{BandProcessor, BandSettings};

pub struct DetectorEq {
    bands: [BandProcessor; EQ_BANDS],
    sample_rate: f64,
}

impl DetectorEq {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            bands: std::array::from_fn(|_| {
                BandProcessor::new(
                    BandSettings {
                        enabled: false,
                        ..BandSettings::default()
                    },
                    sample_rate,
                )
            }),
            sample_rate,
        }
    }

    pub fn update(&mut self, settings: &[BandSettings; EQ_BANDS]) {
        for (processor, settings) in self.bands.iter_mut().zip(settings) {
            if processor.settings() != settings {
                let mut next = BandProcessor::new(settings.clone(), self.sample_rate);
                // A bypassed processor does not advance its histories. Never
                // revive old resonant state after bypass or slot replacement.
                if settings.enabled && processor.settings().enabled {
                    next.inherit_state_from(processor);
                }
                *processor = next;
            }
        }
    }

    pub fn process(&mut self, mut input: [f64; 2]) -> [f64; 2] {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_detector_eq_is_bit_exact_and_does_not_link_channels() {
        let mut eq = DetectorEq::new(48000.0);
        for i in 0..512 {
            let input = [(i as f64 * 0.03).sin(), (i as f64 * 0.08).cos()];
            assert_eq!(eq.process(input), input);
        }
    }

    #[test]
    fn reenabled_band_does_not_revive_stale_audio() {
        let mut eq = DetectorEq::new(48000.0);
        let mut settings = std::array::from_fn(|_| BandSettings {
            enabled: false,
            ..BandSettings::default()
        });
        settings[0] = BandSettings {
            gain_db: 18.0,
            q: 18.0,
            ..BandSettings::default()
        };
        eq.update(&settings);
        eq.process([1.0, 1.0]);
        settings[0].enabled = false;
        eq.update(&settings);
        for _ in 0..128 {
            assert_eq!(eq.process([0.0, 0.0]), [0.0, 0.0]);
        }
        settings[0].enabled = true;
        eq.update(&settings);
        assert_eq!(eq.process([0.0, 0.0]), [0.0, 0.0]);
    }

    #[test]
    fn detector_bell_changes_detection_and_bypass_restores_identity() {
        let mut eq = DetectorEq::new(48000.0);
        let mut settings = std::array::from_fn(|_| BandSettings {
            enabled: false,
            ..BandSettings::default()
        });
        settings[0] = BandSettings {
            gain_db: -12.0,
            ..BandSettings::default()
        };
        eq.update(&settings);
        let mut energy = 0.0;
        let mut input_energy = 0.0;
        for i in 0..9600 {
            let input = (i as f64 * std::f64::consts::TAU * 1000.0 / 48000.0).sin();
            let output = eq.process([input, 0.0]);
            assert_eq!(output[1], 0.0);
            if i >= 4800 {
                energy += output[0] * output[0];
                input_energy += input * input;
            }
        }
        assert!((10.0 * (energy / input_energy).log10() + 12.0).abs() < 0.05);
        settings[0].enabled = false;
        eq.update(&settings);
        assert_eq!(eq.process([0.2, -0.3]), [0.2, -0.3]);
    }
}
