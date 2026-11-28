//! Hohner native ranges, anchor measurements and estimated section geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    N,
    T,
    Cembalet,
    CembaletN,
    C,
    L,
}

impl Model {
    pub const ALL: [Self; 6] = [
        Self::N,
        Self::T,
        Self::Cembalet,
        Self::CembaletN,
        Self::C,
        Self::L,
    ];
    pub fn is_cembalet(self) -> bool {
        matches!(self, Self::Cembalet | Self::CembaletN)
    }
    pub fn key_range(self) -> std::ops::RangeInclusive<u8> {
        match self {
            Self::N | Self::C | Self::L => 29..=89,
            Self::T => 29..=88,
            Self::Cembalet | Self::CembaletN => 36..=96,
        }
    }
}

pub struct Section {
    pub length: f64,
    pub width: f64,
    pub gap: f64,
    pub pluck: f64,
    pub hz: f64,
    pub loss: f64,
    pub field_slope: f64,
}

pub fn interpolate(points: &[(u8, f64)], note: u8) -> f64 {
    if note <= points[0].0 {
        return points[0].1;
    }
    for pair in points.windows(2) {
        let (a, x) = pair[0];
        let (b, y) = pair[1];
        if note <= b {
            return x + (y - x) * f64::from(note - a) / f64::from(b - a);
        }
    }
    points[points.len() - 1].1
}

pub fn section(model: Model, note: u8) -> Option<Section> {
    if !model.key_range().contains(&note) {
        return None;
    }
    let hz = crate::beam::midi_hz(note);
    let length = |base: f64, reference: u8| {
        base * 2.0_f64.powf((f64::from(reference) - f64::from(note)) / 24.0)
    };
    let p = |pts: &[(u8, f64)]| interpolate(pts, note);
    Some(match model {
        Model::N | Model::C | Model::L => {
            // Vary section only between anchors. Three mid-section keys used in
            // the original factory triad retain unsmoothed geometry from that probe.
            let n_length = p(&[
                (29, length(0.066, 45)),
                (45, 0.066),
                (60, length(0.043, 64)),
                (64, 0.043),
                (67, length(0.043, 64)),
                (81, 0.028),
                (89, length(0.028, 81)),
            ]);
            Section {
                length: n_length,
                width: p(&[(45, 0.0052), (60, 0.0046), (67, 0.0046), (81, 0.0038)]),
                gap: p(&[(45, 0.00020), (60, 0.00035), (67, 0.00035), (81, 0.00058)]),
                pluck: p(&[(45, 0.0010), (60, 0.00064), (67, 0.00064), (81, 0.00038)]),
                hz,
                loss: 0.95 + 0.00125 * hz + 0.00000009 * hz.powi(2),
                field_slope: 0.0,
            }
        }
        Model::Cembalet | Model::CembaletN => {
            // Five octaves, C to c four-line. These section dimensions await a measured scale.
            Section {
                length: p(&[(36, 0.085), (60, 0.047), (84, 0.027), (96, 0.019)]),
                width: p(&[(36, 0.0054), (60, 0.0048), (96, 0.0034)]),
                // Set the six octave anchor clearances by Hohner's equal-note
                // regulation procedure. Intermediate keys interpolate in metres.
                gap: if model == Model::Cembalet {
                    p(&[
                        (36, 0.00021),
                        (48, 0.00032),
                        (60, 0.00037),
                        (72, 0.00051),
                        (84, 0.00051),
                        (96, 0.00042),
                    ])
                } else {
                    p(&[
                        (36, 0.000225),
                        (48, 0.00037),
                        (60, 0.00047),
                        (72, 0.00065),
                        (84, 0.00070),
                        (96, 0.00057),
                    ])
                },
                pluck: p(&[(36, 0.00085), (60, 0.00054), (96, 0.00024)]),
                hz,
                loss: 0.95 + 0.0014 * hz,
                field_slope: 0.0,
            }
        }
        Model::T => {
            // P recordings from Sullivan's own jack, interpolated in semitones.
            // Withhold 33/41/49/57/65/73/81/85 for comparison. Higher-mode losses
            // and physical dimensions remain estimates throughout the compass.
            let cents = p(&[
                (29, 2.8),
                (37, 1.0),
                (45, -2.3),
                (52, -1.),
                (61, 1.),
                (69, 1.3),
                (77, 1.7),
                (88, 1.3),
            ]);
            let db_s = p(&[
                (29, 0.84),
                (37, 2.7),
                (45, 3.62),
                (52, 5.67),
                (61, 6.69),
                (69, 12.68),
                (77, 17.88),
                (88, 36.53),
            ]);
            Section {
                length: p(&[
                    (29, 0.090),
                    (45, 0.066),
                    (61, 0.047),
                    (77, 0.032),
                    (88, 0.024),
                ]),
                width: p(&[(29, 0.0055), (61, 0.0048), (88, 0.0038)]),
                gap: p(&[
                    (29, 0.0007),
                    (45, 0.00073),
                    (61, 0.0010),
                    (77, 0.0014),
                    (88, 0.0014),
                ]),
                pluck: p(&[
                    (29, 0.0013),
                    (45, 0.00105),
                    (61, 0.00070),
                    (77, 0.00044),
                    (88, 0.00032),
                ]),
                hz: hz * 2.0_f64.powf(cents / 1200.0),
                loss: db_s / 8.685889638,
                field_slope: p(&[
                    (29, 0.82),
                    (37, 0.88),
                    (45, 0.96),
                    (52, 1.01),
                    (61, 0.81),
                    (69, 1.01),
                    (77, 0.29),
                    (88, 0.067),
                ]),
            }
        }
    })
}
