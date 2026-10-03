use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Menu {
    RouteSource,
    RouteTarget,
    Key,
    Scale,
    KeyboardParam(&'static str),
    YTarget,
    StrumRate,
    MappingKind,
    MappingChannel(bool),
    MappingCc(bool),
}

impl Menu {
    pub(super) fn items(self) -> Vec<String> {
        let labels: &[&str] = match self {
            Self::RouteSource => &crate::engine::routing::SOURCES,
            Self::RouteTarget => {
                return crate::engine::routing::available_targets()
                    .map(|(_, t)| t.name.to_string())
                    .collect()
            }
            Self::Key => {
                return harmony::KEY_CHOICES
                    .iter()
                    .map(|(name, _, _)| (*name).into())
                    .collect()
            }
            Self::Scale => &harmony::SCALE_NAMES,
            Self::KeyboardParam(id) => {
                let (min, max) = keyboard_menu_range(id).expect("keyboard dropdown");
                return (min..=max).map(|value| value.to_string()).collect();
            }
            Self::YTarget => &[
                "Velocity",
                "Gate",
                "Pressure",
                "Timbre",
                "Bend",
                "Custom CC",
            ],
            Self::MappingKind => &["Off", "CC · Auto 7/14-bit", "Pitch bend"],
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
            Self::RouteSource => "Source",
            Self::RouteTarget => "Destination",
            Self::Key => "Key centre",
            Self::Scale => "Scale",
            Self::KeyboardParam(id) => match id {
                "members" => "MPE members",
                "bass_channel" => "Bass MIDI channel",
                "upper_channel" => "Melody MIDI channel",
                _ => "Chord MIDI channel",
            },
            Self::YTarget => "Y destination",
            Self::StrumRate => "Sweep duration",
            Self::MappingKind => "Controller source",
            Self::MappingChannel(_) => "MIDI channel",
            Self::MappingCc(_) => "Controller number",
        }
    }

    fn cell_width(self) -> f32 {
        match self {
            Self::RouteSource | Self::RouteTarget => 146.0,
            Self::Key => 60.0,
            Self::Scale => 146.0,
            Self::KeyboardParam(_) => 40.0,
            Self::YTarget | Self::StrumRate => 112.0,
            Self::MappingKind => 130.0,
            Self::MappingChannel(_) => 40.0,
            Self::MappingCc(_) => 34.0,
        }
    }

    pub(super) fn columns(self) -> usize {
        match self {
            Self::RouteSource => 2,
            Self::RouteTarget => 4,
            Self::Scale | Self::MappingKind | Self::StrumRate | Self::YTarget => 2,
            Self::MappingChannel(_) => 6,
            Self::MappingCc(_) => 16,
            Self::Key => 3,
            Self::KeyboardParam(_) => 6,
        }
    }

    pub(super) fn groups(self) -> Vec<(&'static str, Vec<usize>)> {
        match self {
            Self::RouteSource => vec![
                ("Routing", vec![0]),
                ("MIDI expression", (1..7).collect()),
                ("Performance", (7..10).collect()),
            ],
            Self::RouteTarget => {
                let targets: Vec<_> = crate::engine::routing::available_targets().collect();
                let mut groups = vec![
                    ("Harmony & voicing", Vec::new()),
                    ("Strum", Vec::new()),
                    ("Arpeggiator", Vec::new()),
                    ("Performance", Vec::new()),
                    ("Controllers & MIDI", Vec::new()),
                ];
                for (index, (_, target)) in targets.iter().enumerate() {
                    let group = match target.id {
                        "spread" | "quality" | "inversion" | "transpose" | "filter" => 0,
                        "strings" | "strings_played" | "x" => 1,
                        "arp_pattern" | "rate" | "strum_ms" | "strum_sync" | "strum_hold"
                        | "gate" | "swing" | "octaves" | "humanize" => 2,
                        "velocity" | "length_ms" | "contour" | "root_on_select" | "latch"
                        | "mode" => 3,
                        _ => 4,
                    };
                    groups[group].1.push(index);
                }
                groups
            }
            _ => Vec::new(),
        }
    }
    pub(super) fn group_heading_rects(self, r: Rect) -> Vec<(&'static str, Rect)> {
        let mut y = r.1 + 38.0;
        self.groups()
            .into_iter()
            .map(|(name, indices)| {
                let heading = (r.0 + 12.0, y, r.2 - 24.0, 20.0);
                y += 22.0 + indices.len().div_ceil(self.columns()) as f32 * 34.0;
                (name, heading)
            })
            .collect()
    }
    #[cfg(test)]
    pub(super) fn bounds(self) -> Rect {
        self.bounds_at(self.trigger_rect())
    }

    pub(super) fn bounds_at(self, trigger: Rect) -> Rect {
        let columns = self.columns();
        let rows = self.items().len().div_ceil(columns);
        let width = (self.cell_width() + 4.0) * columns as f32 + 12.0;
        let groups = self.groups();
        let height = if groups.is_empty() {
            rows as f32 * 34.0 + 38.0
        } else {
            40.0 + groups
                .iter()
                .map(|(_, indices)| 22.0 + indices.len().div_ceil(columns) as f32 * 34.0)
                .sum::<f32>()
        };
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
    pub(super) fn option_rect(self, index: usize) -> Rect {
        self.option_rect_at(self.bounds(), index)
    }

    pub(super) fn option_rect_at(self, r: Rect, index: usize) -> Rect {
        let mut y = r.1 + 38.0;
        for (_, indices) in self.groups() {
            if let Some(position) = indices.iter().position(|&i| i == index) {
                return (
                    r.0 + 8.0 + (position % self.columns()) as f32 * (self.cell_width() + 4.0),
                    y + 22.0 + (position / self.columns()) as f32 * 34.0,
                    self.cell_width(),
                    30.0,
                );
            }
            y += 22.0 + indices.len().div_ceil(self.columns()) as f32 * 34.0;
        }
        (
            r.0 + 8.0 + (index % self.columns()) as f32 * (self.cell_width() + 4.0),
            r.1 + 36.0 + (index / self.columns()) as f32 * 34.0,
            self.cell_width(),
            30.0,
        )
    }

    pub(super) fn trigger_rect(self) -> Rect {
        match self {
            Self::RouteSource => ROUTE_SOURCE,
            Self::RouteTarget => ROUTE_TARGET,
            Self::Key => (104.0, 100.0, 104.0, 28.0),
            Self::Scale => (216.0, 100.0, 238.0, 28.0),
            Self::KeyboardParam(id) => keyboard_control_rect(id, false),
            Self::StrumRate => STRUM_RATE,
            Self::YTarget => (772.0, 472.0, 224.0, 30.0),
            Self::MappingKind => (776.0, 324.0, 200.0, 30.0),
            Self::MappingChannel(_) => (984.0, 324.0, 110.0, 30.0),
            Self::MappingCc(_) => (1102.0, 324.0, 130.0, 30.0),
        }
    }
}
