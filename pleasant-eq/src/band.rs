use pleasant_dsp::filters::BiquadKind;

/// Maximum static EQ gain supported by the editor and processor.
pub const MAX_GAIN_DB: f64 = 72.0;
/// A dynamic endpoint can span from one gain limit to the opposite limit.
pub const MAX_RANGE_DB: f64 = MAX_GAIN_DB * 2.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EqShape {
    Bell,
    LowShelf,
    HighShelf,
    LowCut,
    HighCut,
    Notch,
    BandPass,
}

impl EqShape {
    pub fn is_cut(self) -> bool {
        matches!(self, Self::LowCut | Self::HighCut)
    }

    pub fn has_gain(self) -> bool {
        matches!(self, Self::Bell | Self::LowShelf | Self::HighShelf)
    }

    pub fn biquad_kind(self) -> BiquadKind {
        match self {
            Self::Bell => BiquadKind::Bell,
            Self::LowShelf => BiquadKind::LowShelf,
            Self::HighShelf => BiquadKind::HighShelf,
            Self::LowCut => BiquadKind::LowCut,
            Self::HighCut => BiquadKind::HighCut,
            Self::Notch => BiquadKind::Notch,
            Self::BandPass => BiquadKind::BandPass,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BandSettings {
    pub shape: EqShape,
    pub order: u8,
    pub frequency_hz: f64,
    pub gain_db: f64,
    pub q: f64,
    pub enabled: bool,
    pub dynamic: bool,
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
    pub range_db: f64,
}

impl Default for BandSettings {
    fn default() -> Self {
        Self {
            shape: EqShape::Bell,
            order: 2,
            frequency_hz: 1_000.0,
            gain_db: 0.0,
            q: 1.0,
            enabled: true,
            dynamic: false,
            threshold_db: -24.0,
            ratio: 3.0,
            attack_ms: 15.0,
            release_ms: 140.0,
            range_db: 6.0,
        }
    }
}

impl BandSettings {
    pub fn sanitize(&mut self) {
        self.order = self.order.clamp(1, 8);
        sanitize(&mut self.frequency_hz, 20.0, 20_000.0, 1_000.0);
        sanitize(&mut self.gain_db, -MAX_GAIN_DB, MAX_GAIN_DB, 0.0);
        sanitize(&mut self.q, 0.15, 18.0, 1.0);
        sanitize(&mut self.threshold_db, -60.0, 0.0, -24.0);
        sanitize(&mut self.ratio, 1.0, 20.0, 3.0);
        sanitize(&mut self.attack_ms, 0.1, 200.0, 15.0);
        sanitize(&mut self.release_ms, 10.0, 2_000.0, 140.0);
        sanitize(&mut self.range_db, -MAX_RANGE_DB, MAX_RANGE_DB, 6.0);
    }
}

fn sanitize(value: &mut f64, minimum: f64, maximum: f64, default: f64) {
    *value = if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        default
    };
}
