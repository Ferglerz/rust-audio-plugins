use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Menu {
    Key,
    Scale,
    Quality,
    Protocol,
    YTarget,
    StrumRate,
    MappingKind,
    MappingChannel(bool),
    MappingCc(bool),
}

impl Menu {
    pub(super) fn items(self) -> Vec<String> {
        let labels: &[&str] = match self {
            Self::Key => &harmony::NOTE_NAMES,
            Self::Scale => &harmony::SCALE_NAMES,
            Self::Quality => &[
                "Major",
                "Minor",
                "Dominant 7",
                "Major 7",
                "Minor 7",
                "Diminished",
                "Augmented",
                "Major 6",
                "Minor 6",
                "Diminished 7",
                "Half diminished",
                "Power",
            ],
            Self::Protocol => &["Auto", "MPE", "Regular MIDI"],
            Self::YTarget => &[
                "Velocity",
                "Gate",
                "Pressure",
                "Timbre",
                "Bend",
                "Custom CC",
            ],
            Self::MappingKind => &["Off", "CC 7-bit", "CC 14-bit", "Pitch bend"],
            Self::StrumRate => {
                return ARP_RATES
                    .iter()
                    .map(|(label, _)| label.to_string())
                    .collect()
            }
            Self::MappingChannel(bend) => {
                return (1..=16)
                    .map(|n| n.to_string())
                    .chain((!bend).then(|| "Any".into()))
                    .collect()
            }
            Self::MappingCc(wide) => {
                return (0..if wide { 32 } else { 128 })
                    .map(|n| n.to_string())
                    .collect()
            }
        };
        labels.iter().map(|s| s.to_string()).collect()
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Key => "Key centre",
            Self::Scale => "Scale highlighting",
            Self::Quality => "Chord quality",
            Self::Protocol => "Output protocol",
            Self::YTarget => "Y destination",
            Self::StrumRate => "Sweep duration",
            Self::MappingKind => "Controller source",
            Self::MappingChannel(_) => "MIDI channel",
            Self::MappingCc(_) => "Controller number",
        }
    }

    fn cell_width(self) -> f32 {
        match self {
            Self::Key => 60.0,
            Self::Scale => 146.0,
            Self::Quality => 128.0,
            Self::Protocol => 188.0,
            Self::YTarget | Self::StrumRate => 112.0,
            Self::MappingKind => 130.0,
            Self::MappingChannel(_) => 40.0,
            Self::MappingCc(_) => 34.0,
        }
    }

    pub(super) fn columns(self) -> usize {
        match self {
            Self::Scale | Self::MappingKind | Self::StrumRate | Self::YTarget => 2,
            Self::MappingChannel(_) => 6,
            Self::MappingCc(_) => 16,
            Self::Key | Self::Quality => 3,
            Self::Protocol => 1,
        }
    }

    #[cfg(test)]
    pub(super) fn bounds(self) -> Rect {
        self.bounds_at(self.trigger_rect())
    }

    pub(super) fn bounds_at(self, trigger: Rect) -> Rect {
        let columns = self.columns();
        let rows = self.items().len().div_ceil(columns);
        let width = (self.cell_width() + 4.0) * columns as f32 + 12.0;
        let height = rows as f32 * 34.0 + 38.0;
        let below = trigger.1 + trigger.3 + 6.0;
        let y = if below + height <= H - 24.0 {
            below
        } else {
            trigger.1 - height - 6.0
        };
        (
            trigger.0.clamp(16.0, W - width - 16.0),
            y.max(16.0),
            width,
            height,
        )
    }

    #[cfg(test)]
    pub(super) fn close_rect(self) -> Rect {
        let r = self.bounds();
        (r.0 + r.2 - 34.0, r.1 + 6.0, 26.0, 26.0)
    }

    #[cfg(test)]
    pub(super) fn option_rect(self, index: usize) -> Rect {
        self.option_rect_at(self.bounds(), index)
    }

    pub(super) fn option_rect_at(self, r: Rect, index: usize) -> Rect {
        (
            r.0 + 8.0 + (index % self.columns()) as f32 * (self.cell_width() + 4.0),
            r.1 + 36.0 + (index / self.columns()) as f32 * 34.0,
            self.cell_width(),
            30.0,
        )
    }

    pub(super) fn trigger_rect(self) -> Rect {
        match self {
            Self::Key => (32.0, 190.0, 90.0, 28.0),
            Self::Scale => (130.0, 190.0, 174.0, 28.0),
            Self::Quality => QUALITY,
            Self::Protocol => MPE,
            Self::StrumRate => (852.0, 342.0, 220.0, 42.0),
            Self::YTarget => (612.0, 472.0, 224.0, 30.0),
            Self::MappingKind => (616.0, 324.0, 200.0, 30.0),
            Self::MappingChannel(_) => (824.0, 324.0, 110.0, 30.0),
            Self::MappingCc(_) => (942.0, 324.0, 130.0, 30.0),
        }
    }
}
