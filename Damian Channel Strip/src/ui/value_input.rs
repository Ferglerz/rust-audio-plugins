#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GlobalControl {
    Compression,
    Gate,
    PseVoice,
    Dry,
    Wet,
    CompAttack,
    CompRatio,
    PseDepth,
    PseHysteresis,
    PseTime,
    CompRelease,
    CompDepth,
    Output,
    WallEven,
    WallOdd,
    WallThreshold,
}

impl GlobalControl {
    pub(super) fn inverted(self) -> bool {
        matches!(self, Self::WallThreshold)
    }
}

/// Readout fields shared by EQ bands and Lift. Host parameter identity stays on `GlobalControl`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BandField {
    Frequency,
    Gain,
    Q,
    Threshold,
    Ratio,
    Attack,
    Release,
    Range,
}

impl BandField {
    pub(super) fn hud(slot: usize) -> Self {
        [Self::Frequency, Self::Gain, Self::Q][slot]
    }

    /// Dynamic-band bars: threshold, range, ratio, attack, release.
    pub(super) fn from_dyn_bar(slot: usize) -> Self {
        match slot {
            0 => Self::Threshold,
            1 => Self::Range,
            2 => Self::Ratio,
            3 => Self::Attack,
            _ => Self::Release,
        }
    }

    /// Lift dock sliders: threshold, ratio, attack, release, range.
    pub(super) fn from_lift_slider(slot: usize) -> Self {
        [Self::Threshold, Self::Ratio, Self::Attack, Self::Release, Self::Range][slot]
    }

    pub(super) fn dyn_bar_index(self) -> usize {
        match self {
            Self::Threshold => 0,
            Self::Range => 1,
            Self::Ratio => 2,
            Self::Attack => 3,
            Self::Release => 4,
            Self::Frequency | Self::Gain | Self::Q => 4,
        }
    }

    pub(super) fn lift_slider_index(self) -> usize {
        match self {
            Self::Threshold => 0,
            Self::Ratio => 1,
            Self::Attack => 2,
            Self::Release => 3,
            Self::Range => 4,
            Self::Frequency | Self::Gain | Self::Q => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ValueTarget {
    Global(GlobalControl),
    Band(BandField),
    Lift(BandField),
}

pub(super) fn parse_value(text: &str, target: ValueTarget) -> Option<f64> {
    let text = text.trim().to_ascii_lowercase();
    if target == ValueTarget::Global(GlobalControl::Gate) && text == "off" {
        return Some(-80.0);
    }
    if target == ValueTarget::Global(GlobalControl::PseTime) {
        match text.as_str() {
            "a" => return Some(0.0),
            "b" => return Some(1.0),
            "c" => return Some(2.0),
            "d" => return Some(3.0),
            "e" => return Some(4.0),
            "f" => return Some(5.0),
            _ => {}
        }
        if let Some(s) = text.strip_suffix("ms") {
            if let Ok(ms) = s.trim().parse::<f64>() {
                return Some(crate::dsp::seconds_to_pse_time_pos(ms * 0.001));
            }
        }
        if let Some(s) = text.strip_suffix("s") {
            if let Ok(sec) = s.trim().parse::<f64>() {
                return Some(crate::dsp::seconds_to_pse_time_pos(sec));
            }
        }
        if let Ok(val) = text.parse::<f64>() {
            if (0.0..=5.0).contains(&val) {
                return Some(val);
            }
            if val > 5.0 {
                return Some(crate::dsp::seconds_to_pse_time_pos(val * 0.001));
            }
        }
        return None;
    }
    let suffixes: &[(&str, f64)] = match target {
        ValueTarget::Band(BandField::Frequency) | ValueTarget::Lift(BandField::Frequency) => {
            &[("khz", 1000.0), ("hz", 1.0), ("k", 1000.0)]
        }
        ValueTarget::Band(BandField::Attack | BandField::Release)
        | ValueTarget::Lift(BandField::Attack | BandField::Release)
        | ValueTarget::Global(GlobalControl::CompAttack | GlobalControl::CompRelease) => {
            &[("ms", 1.0), ("s", 1000.0)]
        }
        ValueTarget::Band(BandField::Ratio)
        | ValueTarget::Lift(BandField::Ratio)
        | ValueTarget::Global(GlobalControl::CompRatio) => &[(":1", 1.0)],
        ValueTarget::Global(
            GlobalControl::PseVoice
            | GlobalControl::Dry
            | GlobalControl::Wet
            | GlobalControl::WallEven
            | GlobalControl::WallOdd,
        ) => &[("%", 1.0)],
        ValueTarget::Global(GlobalControl::Compression | GlobalControl::WallThreshold) => {
            &[("db", 1.0)]
        }
        ValueTarget::Band(BandField::Q) | ValueTarget::Lift(BandField::Q) => &[],
        _ => &[("db", 1.0)],
    };
    let value = pleasant_ui::value_edit::parse_number_with_units(&text, suffixes)?;
    value.is_finite().then_some(value)
}
