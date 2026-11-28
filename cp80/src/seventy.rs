//! Yamaha CP-70 / CP-70B physical key numbers start at E1. The tuning chart in
//! service manual 006386 uses 88-key numbering. The comparative parts list uses 1–73.
use crate::KeySpec;
use std::f64::consts::PI;

pub const LOW: u8 = 28;
pub const HIGH: u8 = 100;
pub(crate) const FINE_TREBLE_FROM: u8 = 89;
const CENTS: [i8; 73] = [
    -13, -12, -11, -10, -9, -8, -7, -7, -6, -6, -5, -5, -5, -4, -4, -4, -4, -4, -3, -3, -3, -3, -3,
    -2, -2, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 3, 3,
    3, 4, 4, 4, 5, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 11, 12, 14, 16, 18, 20,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Revision {
    Original,
    B,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Strings {
    Wound {
        left: u32,
        right: Option<u32>,
    }, // NB part suffix, formatted as six digits
    Plain {
        wire_part: u16,
        gauge: f64,
        diameter_mm: f64,
    }, // MV part suffix
}

#[derive(Clone, Copy)]
pub struct Course {
    pub physical: u8,
    pub service: u8,
    pub midi: u8,
    pub cents: i8,
    pub strings: Strings,
    pub hammer_part: &'static str,
    pub damper_part: Option<u32>, // NB part suffix
    /// The B service drawing identifies the numbered stands and three mixing buses.
    pub b_wiring: Option<(u8, usize)>,
}

pub fn course(revision: Revision, midi: u8) -> Option<Course> {
    if !(LOW..=HIGH).contains(&midi) {
        return None;
    }
    let k = midi - LOW + 1;
    let strings = if k <= 37 {
        let first = match revision {
            Revision::Original => 500890,
            Revision::B => 504030,
        };
        let preceding = u32::from(k - 1) + u32::from(k.saturating_sub(16));
        let left = first + 10 * preceding;
        Strings::Wound {
            left,
            right: (k >= 16).then_some(left + 10),
        }
    } else {
        // Yamaha wire numbers, nominal piano-wire diameters in mm.
        let (wire_part, gauge, diameter_mm) = match k {
            38..=44 => (40, 17., 0.975),
            45..=50 => (50, 16.5, 0.95),
            51..=56 => (60, 16., 0.925),
            57..=64 => (70, 15.5, 0.90),
            65..=69 => (80, 15., 0.875),
            _ => (100, 14., 0.825),
        };
        Strings::Plain {
            wire_part,
            gauge,
            diameter_mm,
        }
    };
    Some(Course {
        physical: k,
        service: k + 7,
        midi,
        cents: CENTS[usize::from(k - 1)],
        strings,
        hammer_part: match k {
            1..=25 => "NB970050",
            26..=50 => "NB970060",
            51..=63 => "NB970070",
            _ => "NB970080",
        },
        damper_part: (k <= 66).then_some(506000 + 10 * u32::from(k)),
        b_wiring: (revision == Revision::B).then(|| {
            (
                match k {
                    1..=15 => 1,
                    16..=25 => 2,
                    26..=37 => 3,
                    38..=50 => 4,
                    51..=56 => 5,
                    _ => 6,
                },
                bus(midi),
            )
        }),
    })
}

/// Preserve the edge construction and its frequency offset outside the physical compass.
pub fn extended(midi: u8) -> KeySpec {
    extended_for(Revision::B, midi)
}

pub fn extended_for(revision: Revision, midi: u8) -> KeySpec {
    let edge = midi.clamp(LOW, HIGH);
    let mut spec = course(revision, edge)
        .expect("clamped 73-key pitch")
        .mechanics();
    spec.length *= 2f64.powf((f64::from(edge) - f64::from(midi)) / 12.);
    spec.midi = midi;
    spec
}

pub fn bus(midi: u8) -> usize {
    match midi {
        0..=90 => 0,
        91..=92 => 1,
        _ => 2,
    }
}

impl Course {
    pub fn hz(self) -> f64 {
        440. * 2f64.powf((f64::from(self.midi) - 69. + f64::from(self.cents) / 100.) / 12.)
    }

    /// Provisional geometry and action. Interpolate only within the same construction.
    /// Plain wire mass and bending rigidity follow its nominal circular steel section.
    pub fn mechanics(self) -> KeySpec {
        let k = self.physical;
        let (lo, hi, long, short, first_b, last_b, count) = match self.strings {
            Strings::Wound { right: None, .. } => (1, 15, 1.03, 0.86, 0.00135, 0.00021, 1),
            Strings::Wound { right: Some(_), .. } => (16, 37, 0.86, 0.49, 0.00020, 0.00036, 2),
            Strings::Plain { .. } => (38, 73, 0.47, 0.106, 0., 0., 2),
        };
        let t = f64::from(k - lo) / f64::from(hi - lo);
        let length = long * f64::powf(short / long, t);
        let f = self.hz();
        let (tension, stiffness) = match self.strings {
            Strings::Plain { diameter_mm, .. } => {
                let d = diameter_mm * 0.001;
                let mass_per_m = 7850. * PI * d * d / 4.;
                let bending_rigidity = 200e9 * PI * d.powi(4) / 64.;
                let bending = PI.powi(2) * bending_rigidity / length.powi(2);
                let tension = (2. * length * f).powi(2) * mass_per_m - bending;
                (tension, bending / tension)
            }
            Strings::Wound { .. } => (490. + 60. * t, first_b + t * (last_b - first_b)),
        };
        let (hammer_kg, contact_k, strike_fraction) = match k {
            1..=25 => (0.0060, 1.05e7, 0.10),
            26..=50 => (0.0051, 1.65e7, 0.092),
            51..=63 => (0.0042, 3.5e7, 0.088),
            _ => (0.0037, 6.0e7, 0.083),
        };
        // Borrow loss envelopes from the adjacent CP-80 construction as a starting estimate.
        let loss_key = match self.strings {
            Strings::Wound { right: Some(_), .. } => self.midi.min(62),
            _ => self.midi,
        };
        let control =
            KeySpec::for_note(loss_key).expect("73-key compass is inside the CP-80 scale");
        KeySpec {
            midi: self.midi,
            strings: count,
            cents: self.cents.into(),
            stiffness,
            length,
            tension,
            hammer_kg,
            contact_k,
            contact_linear: 0.,
            strike_fraction,
            unison_cents: 0.8,
            bearing_imbalance: 0.,
            losses_db_s: control.losses_db_s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_treble_tracks_fine_integration_at_each_host_rate() {
        use crate::{Key, LiveKey};
        use std::f64::consts::TAU;

        let mut worst = 0.0_f64;
        for note in 88..=100 {
            let spec = course(Revision::B, note).unwrap().mechanics();
            let partials: Vec<_> = (1..=8)
                .map(|i| {
                    let n = f64::from(i);
                    spec.frequency()
                        * n
                        * ((1. + n * n * spec.stiffness) / (1. + spec.stiffness)).sqrt()
                })
                .filter(|f| *f < 16_000.)
                .collect();
            let levels = |rate: f64, next: &mut dyn FnMut() -> f64| {
                let mut sums = vec![(0., 0.); partials.len()];
                for i in 0..(rate * 0.060).round() as usize {
                    let seconds = i as f64 / rate;
                    let x = next();
                    if seconds >= 0.010 {
                        let window = 0.5 - 0.5 * (TAU * (seconds - 0.010) / 0.050).cos();
                        for (&f, (re, im)) in partials.iter().zip(&mut sums) {
                            let (s, c) = (TAU * f * seconds).sin_cos();
                            *re += window * x * c / rate;
                            *im += window * x * s / rate;
                        }
                    }
                }
                sums.into_iter()
                    .map(|(r, i)| r.hypot(i))
                    .collect::<Vec<_>>()
            };
            let mut fine = Key::new(spec, 384_000.);
            fine.strike(1.);
            let expected = levels(384_000., &mut || fine.step());
            for rate in [44_100., 48_000., 96_000.] {
                let mut live = LiveKey::realtime(spec, rate, FINE_TREBLE_FROM);
                live.strike(1.);
                let actual = levels(rate, &mut || live.host_sample());
                for (i, (&a, &b)) in actual.iter().zip(&expected).enumerate() {
                    // Ignore inaudible numerical residuals within a contact-position notch.
                    if b > expected[0] * 0.001 {
                        let error = 20. * (a / b).log10();
                        worst = worst.max(error.abs());
                        assert!(
                            error.abs() < 0.5,
                            "MIDI {note} at {rate} Hz, partial {}, {error:.3} dB",
                            i + 1
                        );
                    }
                }
            }
        }
        eprintln!("CP-70B live vs 384 kHz, maximum partial level error {worst:.3} dB");
    }

    #[test]
    fn matches_factory_parts_and_tuning_boundaries() {
        let key = |k| course(Revision::B, LOW + k - 1).unwrap();
        assert!(course(Revision::B, 27).is_none() && course(Revision::B, 101).is_none());
        assert_eq!(
            (key(42).service, key(42).midi, key(42).hz()),
            (49, 69, 440.)
        );
        assert_eq!((key(1).cents, key(73).cents), (-13, 20));
        assert_eq!(
            key(15).strings,
            Strings::Wound {
                left: 504170,
                right: None
            }
        );
        assert_eq!(
            key(16).strings,
            Strings::Wound {
                left: 504180,
                right: Some(504190)
            }
        );
        assert_eq!(
            key(37).strings,
            Strings::Wound {
                left: 504600,
                right: Some(504610)
            }
        );
        assert!(matches!(key(38).strings, Strings::Plain { gauge: 17., .. }));
        assert_eq!(
            course(Revision::Original, 64).unwrap().strings,
            Strings::Wound {
                left: 501460,
                right: Some(501470)
            }
        );
        assert!(course(Revision::Original, 64).unwrap().b_wiring.is_none());
        assert_eq!(key(66).damper_part, Some(506660));
        assert_eq!(key(67).damper_part, None);
        assert_eq!(
            [63, 64, 65, 66].map(|k| key(k).b_wiring.unwrap().1),
            [0, 1, 1, 2]
        );
        for k in 1..=73 {
            let c = key(k);
            let m = c.mechanics();
            assert_eq!(m.strings, if k <= 15 { 1 } else { 2 });
            assert!(m.tension.is_finite() && m.tension > 0. && m.stiffness > 0.);
        }
    }
}
