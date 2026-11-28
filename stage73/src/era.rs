//! Representative instruments and mechanical assumptions. See research/eras.md.
use nih_plug::prelude::Enum;

#[derive(Enum, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Era {
    #[id = "sparkletop"]
    #[name = "Sparkletop"]
    Sparkletop,
    #[id = "early_i"]
    #[name = "Early Mark I"]
    EarlyI,
    #[id = "mid_i"]
    #[name = "Mid Mark I"]
    MidI,
    #[default]
    #[id = "late_i"]
    #[name = "Late Mark I"]
    LateI,
    #[id = "mark_ii"]
    #[name = "Mark II"]
    MarkII,
    #[id = "mark_v"]
    #[name = "Mark V"]
    MarkV,
}

impl Era {
    pub const ALL: [Self; 6] = [
        Self::Sparkletop,
        Self::EarlyI,
        Self::MidI,
        Self::LateI,
        Self::MarkII,
        Self::MarkV,
    ];

    pub fn slug(self) -> &'static str {
        Self::ids().unwrap()[self.to_index()]
    }

    /// Fixed stereo gain after the electronics. Calibrated at stock tone,
    /// with modulation off, across E1–E7 and four velocities (research/eras.md).
    pub(crate) fn playback_gain(self, peterson: bool) -> f64 {
        let db = match (self, peterson) {
            (Self::Sparkletop, _) => -0.44,
            (Self::EarlyI, false) => 1.94,
            (Self::MidI, false) => 0.69,
            (Self::EarlyI | Self::MidI, true) => 1.47,
            (Self::LateI, false) => 2.02,
            (Self::LateI, true) => 2.85,
            (Self::MarkII, false) => 7.14,
            (Self::MarkII, true) => 7.85,
            (Self::MarkV, false) => -0.46,
            (Self::MarkV, true) => 0.27,
        };
        f64::from(nih_plug::util::db_to_gain(db))
    }

    /// Stock tine height and pickup gap in mm for a playable MIDI note.
    pub fn pickup_geometry(self, note: u8) -> Option<(f64, f64)> {
        use crate::engine::{HIGH, LOW};
        (LOW..=HIGH)
            .contains(&note)
            .then(|| self.geometry(usize::from(note - LOW)))
    }

    pub(crate) fn geometry(self, index: usize) -> (f64, f64) {
        if self == Era::LateI {
            let [height, gap, ..] = crate::calibration::LATE_VOICING[index];
            return (height, gap);
        }
        if self == Era::MarkII {
            let [height, gap, _, _] = crate::calibration::MARK_II[index];
            return (height, gap);
        }
        let row = crate::calibration::KEYS[index];
        (row[2], row[3])
    }

    pub(crate) fn swing(self, index: usize, v: f64) -> f64 {
        use crate::calibration::{KEYS, LATE_VOICING, MARK_II, VELOCITIES};
        if self == Era::LateI {
            let [_, _, p, mp, mf, f] = LATE_VOICING[index];
            return interpolated_swing([p, mp, mf, f], v);
        }
        let (lo, hi, quiet, loud) = if self == Era::MarkII {
            let [_, _, quiet, loud] = MARK_II[index];
            (0.37075, 1.0, quiet, loud)
        } else {
            let i = VELOCITIES[1..3].partition_point(|&knot| knot < v);
            let anchors = &KEYS[index][4..8];
            (VELOCITIES[i], VELOCITIES[i + 1], anchors[i], anchors[i + 1])
        };
        let swing = if v < lo {
            quiet * v / lo
        } else {
            quiet * (loud / quiet).powf((v - lo) / (hi - lo))
        };
        swing * self.strike_gain(v)
    }

    pub(crate) fn strike_gain(self, velocity: f64) -> f64 {
        match self {
            // Fig. 3 of US4308783A: rearward balance pin, longer finger lever.
            // Travel at a given key speed is proportional to the lever ratio.
            Self::MarkII => ((5.5 - 0.375) / (7.5 + 0.375)) / (5.5 / 7.5),
            // Production MK V stroke vs previous action (1984 Rhodes ad).
            // Scale the upper end of hammer speed, keeping gentle strikes playable.
            Self::MarkV => 1.0 + (2.0 / 1.625 - 1.0) * velocity,
            _ => 1.0,
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            Self::Sparkletop => "1965–69 · Felt hammers, original harp wiring and a mono Jordan Suitcase preamp.",
            Self::EarlyI => "1970–73 · The 1972 action pairs neoprene tips with the early treble-cut panel.",
            Self::MidI => "1974–75 · Hybrid hammers and cube tips feed the later bass-cut panel.",
            Self::LateI => "1976–79 · The original Stage 73 calibration and passive Bass Boost circuit.",
            Self::MarkII => "1979–83 · Later plastic-key action, modeled at the rear balance-pin position.",
            Self::MarkV => "1984 · Longer hammer throw and revised dampers. Passive tone circuit is provisional.",
        }
    }
}

/// Interpolate log displacement between measured strikes with monotone
/// Hermite slopes. Flat neighboring layers stay flat.
fn interpolated_swing(anchors: [f64; 4], velocity: f64) -> f64 {
    let x = crate::calibration::VELOCITIES;
    let y = anchors.map(f64::ln);
    let secant = |k: usize| (y[k + 1] - y[k]) / (x[k + 1] - x[k]);
    let slope = |k: usize| {
        if k == 3 {
            return secant(2);
        }
        let (left, a) = if k == 0 {
            (x[0], 1. / x[0])
        } else {
            (x[k] - x[k - 1], secant(k - 1))
        };
        let right = x[k + 1] - x[k];
        let b = secant(k);
        if a * b <= 0. {
            return 0.;
        }
        3. * (left + right) / ((2. * right + left) / a + (right + 2. * left) / b)
    };
    if velocity < x[0] {
        let t = velocity / x[0];
        return anchors[0] * t * (1. + (1. - slope(0) * x[0]) * t * (1. - t));
    }
    let i = x[1..3].partition_point(|&knot| knot < velocity);
    let width = x[i + 1] - x[i];
    let t = (velocity - x[i]) / width;
    let log_swing = y[i]
        + (y[i + 1] - y[i]) * t * t * (3. - 2. * t)
        + width * t * (1. - t) * ((1. - t) * slope(i) - t * slope(i + 1));
    log_swing.exp()
}
