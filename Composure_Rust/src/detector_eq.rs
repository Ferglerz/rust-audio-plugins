//! Host-backed static bands for the detector equalizer.

use nih_plug::prelude::*;
use pleasant_eq::{BandSettings, EqShape};

pub const EQ_BANDS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum DetectorShape {
    #[name = "Bell"]
    Bell,
    #[name = "Low Shelf"]
    LowShelf,
    #[name = "High Shelf"]
    HighShelf,
    #[name = "Low Cut"]
    LowCut,
    #[name = "High Cut"]
    HighCut,
    #[name = "Notch"]
    Notch,
}

impl DetectorShape {
    pub const ALL: [Self; 6] = [
        Self::Bell,
        Self::LowShelf,
        Self::HighShelf,
        Self::LowCut,
        Self::HighCut,
        Self::Notch,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Bell => "Bell",
            Self::LowShelf => "Low Shelf",
            Self::HighShelf => "High Shelf",
            Self::LowCut => "Low Cut",
            Self::HighCut => "High Cut",
            Self::Notch => "Notch",
        }
    }

    pub const fn eq_shape(self) -> EqShape {
        match self {
            Self::Bell => EqShape::Bell,
            Self::LowShelf => EqShape::LowShelf,
            Self::HighShelf => EqShape::HighShelf,
            Self::LowCut => EqShape::LowCut,
            Self::HighCut => EqShape::HighCut,
            Self::Notch => EqShape::Notch,
        }
    }
}

#[derive(Params)]
pub struct DetectorBandParams {
    #[id = "det_eq_active"]
    pub active: BoolParam,
    #[id = "det_eq_enabled"]
    pub enabled: BoolParam,
    #[id = "det_eq_shape"]
    pub shape: EnumParam<DetectorShape>,
    #[id = "det_eq_freq"]
    pub freq: FloatParam,
    #[id = "det_eq_gain"]
    pub gain: FloatParam,
    #[id = "det_eq_q"]
    pub q: FloatParam,
}

/// NIH-plug's power-skew range approximates a logarithmic control, with its
/// midpoint set to the geometric mean. The graph uses an exact logarithmic axis.
fn logarithmic_range(min: f32, max: f32) -> FloatRange {
    let midpoint = (min * max).sqrt();
    FloatRange::Skewed {
        min,
        max,
        factor: 0.5_f32.ln() / ((midpoint - min) / (max - min)).ln(),
    }
}

impl Default for DetectorBandParams {
    fn default() -> Self {
        Self {
            active: BoolParam::new("Active", false),
            enabled: BoolParam::new("Enabled", true),
            shape: EnumParam::new("Shape", DetectorShape::Bell),
            freq: FloatParam::new("Frequency", 1_000.0, logarithmic_range(20.0, 20_000.0))
                .with_unit(" Hz"),
            gain: FloatParam::new(
                "Gain",
                0.0,
                FloatRange::Linear {
                    min: -24.0,
                    max: 24.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(0.1),
            q: FloatParam::new("Q", 1.0, logarithmic_range(0.15, 18.0)),
        }
    }
}

impl DetectorBandParams {
    pub fn settings(&self) -> BandSettings {
        BandSettings {
            enabled: self.active.value() && self.enabled.value(),
            shape: self.shape.value().eq_shape(),
            frequency_hz: self.freq.value() as f64,
            gain_db: self.gain.value() as f64,
            q: self.q.value() as f64,
            dynamic: false,
            ..BandSettings::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_detector_band_is_disabled_static_bell() {
        let params = DetectorBandParams::default();
        assert!(!params.active.value());
        assert!(params.enabled.value());
        let settings = params.settings();
        assert!(!settings.enabled);
        assert!(!settings.dynamic);
        assert_eq!(settings.shape, EqShape::Bell);
        assert_eq!(settings.frequency_hz, 1_000.0);
        assert_eq!(settings.gain_db, 0.0);
        assert_eq!(settings.q, 1.0);
        assert_eq!(settings.order, 2);
    }

    #[test]
    fn detector_shapes_match_shared_eq_shapes() {
        assert_eq!(
            DetectorShape::ALL.map(DetectorShape::eq_shape),
            [
                EqShape::Bell,
                EqShape::LowShelf,
                EqShape::HighShelf,
                EqShape::LowCut,
                EqShape::HighCut,
                EqShape::Notch,
            ]
        );
    }

    #[test]
    fn only_active_unbypassed_bands_process_audio() {
        for active in [false, true] {
            for enabled in [false, true] {
                let params = DetectorBandParams {
                    active: BoolParam::new("Active", active),
                    enabled: BoolParam::new("Enabled", enabled),
                    ..DetectorBandParams::default()
                };
                assert_eq!(params.settings().enabled, active && enabled);
            }
        }
    }

    #[test]
    fn logarithmic_controls_cover_endpoints_and_geometric_midpoint() {
        for (min, max) in [(20.0, 20_000.0), (0.15, 18.0)] {
            let range = logarithmic_range(min, max);
            assert_eq!(range.unnormalize(0.0), min);
            assert_eq!(range.unnormalize(1.0), max);
            let midpoint = (min * max).sqrt();
            assert!((range.unnormalize(0.5) - midpoint).abs() < midpoint * 1e-5);
        }
    }
}
