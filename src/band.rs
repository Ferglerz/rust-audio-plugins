use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Shape {
    Bell,
    LowShelf,
    HighShelf,
    LowCut,
    HighCut,
    Notch,
    BandPass,
}
impl Shape {
    pub const ALL: [Self; 7] = [
        Self::Bell,
        Self::LowShelf,
        Self::HighShelf,
        Self::LowCut,
        Self::HighCut,
        Self::Notch,
        Self::BandPass,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Bell => "Bell",
            Self::LowShelf => "Low shelf",
            Self::HighShelf => "High shelf",
            Self::LowCut => "Low cut",
            Self::HighCut => "High cut",
            Self::Notch => "Notch",
            Self::BandPass => "Band pass",
        }
    }
    pub fn uppercase_name(self) -> &'static str {
        match self {
            Self::Bell => "BELL",
            Self::LowShelf => "LOW SHELF",
            Self::HighShelf => "HIGH SHELF",
            Self::LowCut => "LOW CUT",
            Self::HighCut => "HIGH CUT",
            Self::Notch => "NOTCH",
            Self::BandPass => "BAND PASS",
        }
    }
    pub fn is_cut(self) -> bool {
        matches!(self, Self::LowCut | Self::HighCut)
    }
    pub fn has_gain(self) -> bool {
        matches!(self, Self::Bell | Self::LowShelf | Self::HighShelf)
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Band {
    pub id: u64,
    pub shape: Shape,
    /// High/low-pass order (6 dB/octave per pole). Ignored for other shapes.
    pub order: u8,
    pub freq: f64,
    pub gain: f64,
    pub q: f64,
    pub enabled: bool,
    pub dynamic: bool,
    pub threshold: f64,
    pub ratio: f64,
    pub attack: f64,
    pub release: f64,
    pub range: f64,
}
impl Default for Band {
    fn default() -> Self {
        Self {
            id: 1,
            shape: Shape::Bell,
            order: 2,
            freq: 1000.0,
            gain: 0.0,
            q: 1.0,
            enabled: true,
            dynamic: false,
            threshold: -24.0,
            ratio: 3.0,
            attack: 15.0,
            release: 140.0,
            range: 6.0,
        }
    }
}
impl Band {
    pub fn sanitize(&mut self) {
        self.order = self.order.clamp(1, 8);
        fn safe(v: &mut f64, min: f64, max: f64, default: f64) {
            *v = if v.is_finite() {
                v.clamp(min, max)
            } else {
                default
            };
        }
        safe(&mut self.freq, 20.0, 20000.0, 1000.0);
        safe(&mut self.gain, -24.0, 24.0, 0.0);
        safe(&mut self.q, 0.15, 18.0, 1.0);
        safe(&mut self.threshold, -60.0, 0.0, -24.0);
        safe(&mut self.ratio, 1.0, 20.0, 3.0);
        safe(&mut self.attack, 0.1, 200.0, 15.0);
        safe(&mut self.release, 10.0, 2000.0, 140.0);
        safe(&mut self.range, -24.0, 24.0, 6.0);
    }
}
/// Background clicks at the outer edges create cuts. Pulling the curve at its ends creates shelves.
pub fn infer_shape(x: f32, y: f32, curve_drag: bool) -> Shape {
    if curve_drag && x < 0.18 {
        Shape::LowShelf
    } else if curve_drag && x > 0.82 {
        Shape::HighShelf
    } else if x < 0.055 {
        Shape::LowCut
    } else if x > 0.945 {
        Shape::HighCut
    } else if y > 0.90 {
        Shape::Notch
    } else {
        Shape::Bell
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_sessions_default_to_second_order() {
        let b: Band = serde_json::from_str(r#"{"shape":"LowCut","freq":80.0,"q":0.707}"#).unwrap();
        assert_eq!(b.order, 2);
        assert_eq!(b.shape, Shape::LowCut);
    }
    #[test]
    fn shapes_and_orders_round_trip() {
        for shape in Shape::ALL {
            for order in 1..=8 {
                let b = Band {
                    shape,
                    order,
                    ..Band::default()
                };
                assert_eq!(
                    b,
                    serde_json::from_str::<Band>(&serde_json::to_string(&b).unwrap()).unwrap()
                );
            }
        }
        for (invalid, expected) in [(0, 1), (255, 8)] {
            let mut b = Band {
                order: invalid,
                ..Band::default()
            };
            b.sanitize();
            assert_eq!(b.order, expected);
        }
    }
    #[test]
    fn creation_zones() {
        assert_eq!(infer_shape(0.01, 0.5, false), Shape::LowCut);
        assert_eq!(infer_shape(0.99, 0.5, false), Shape::HighCut);
        assert_eq!(infer_shape(0.1, 0.5, true), Shape::LowShelf);
        assert_eq!(infer_shape(0.9, 0.5, true), Shape::HighShelf);
        assert_eq!(infer_shape(0.5, 0.98, false), Shape::Notch);
        assert_eq!(infer_shape(0.5, 0.4, false), Shape::Bell);
    }
    #[test]
    fn dynamic_range_bipolar_sanitization() {
        let mut b = Band {
            range: -12.5,
            ..Band::default()
        };
        b.sanitize();
        assert_eq!(b.range, -12.5);

        b.range = -30.0;
        b.sanitize();
        assert_eq!(b.range, -24.0);

        b.range = 30.0;
        b.sanitize();
        assert_eq!(b.range, 24.0);
    }
}
