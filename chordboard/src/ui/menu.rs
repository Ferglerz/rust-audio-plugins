use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Menu {
    Meter(bool),
    RouteSource,
    RouteTarget,
    Key,
    Scale,
    KeyboardParam(&'static str),
    ArpRate,
    ArpPattern,
}

impl Menu {
    pub(super) fn items(self) -> Vec<String> {
        let labels: &[&str] = match self {
            Self::Meter(den) => return if den { (0..=5).map(|n| (1 << n).to_string()).collect() } else { (1..=32).map(|n| n.to_string()).collect() },
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
            Self::ArpPattern => {
                return ARP_PATTERN_NAMES.iter().map(|label| (*label).to_string()).collect()
            }
            Self::ArpRate => {
                return ARP_RATES
                    .iter()
                    .map(|(label, _)| label.to_string())
                    .collect()
            }
        };
        labels.iter().map(|s| s.to_string()).collect()
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Meter(den) => if den { "Beat unit" } else { "Beats per bar" },
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
            Self::ArpRate => "Arp rate",
            Self::ArpPattern => "Strum direction",
        }
    }

    fn cell_width(self) -> f32 {
        match self {
            Self::RouteSource | Self::RouteTarget => 146.0,
            Self::Key => 60.0,
            Self::Scale => 146.0,
            Self::Meter(_) | Self::KeyboardParam(_) => 40.0,
            Self::ArpRate => 112.0,
            Self::ArpPattern => 148.0,
        }
    }

    pub(super) fn columns(self) -> usize {
        match self {
            Self::RouteSource => 2,
            Self::RouteTarget => 4,
            Self::Scale | Self::ArpRate => 2,
            Self::ArpPattern => 1,
            Self::Key => 3,
            Self::Meter(_) | Self::KeyboardParam(_) => 6,
        }
    }

    pub(super) fn groups(self) -> Vec<(&'static str, Vec<usize>)> {
        match self {
            Self::RouteSource => vec![
                ("Routing", vec![0]),
                ("MIDI expression", (1..7).collect()),
                ("Performance", (7..10).collect()),
                ("Arpeggiator", (10..13).collect()),
            ],
            Self::RouteTarget => {
                let targets: Vec<_> = crate::engine::routing::available_targets().collect();
                let mut groups = vec![
                    ("Harmony & voicing", Vec::new()),
                    ("Strum", Vec::new()),
                    ("Arpeggiator", Vec::new()),
                    ("Performance", Vec::new()),
                    ("Controllers & MIDI", Vec::new()),
                    ("Modulators", Vec::new()),
                ];
                for (index, (_, target)) in targets.iter().enumerate() {
                    let group = match target.id {
                        _ if target.source_index().is_some() => 5,
                        "spread" | "quality" | "inversion" | "transpose" | "filter" => 0,
                        "strings" | "strings_played" | "x" => 1,
                        "arp_pattern" | "rate" | "strum_ms" | "strum_sync" | "strum_hold"
                        | "gate" | "swing" | "octaves" | "humanize" | "arp_octaves" | "arp_sync" | "arp_free_ms" | "arp_mode" | "arp_page" => 2,
                        "velocity" | "length_ms" | "contour" | "root_on_select" | "latch"
                        => 3,
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
            Self::Meter(den) => if den { METER_DEN } else { METER_NUM },
            Self::RouteSource => ROUTE_SOURCE,
            Self::RouteTarget => ROUTE_TARGET,
            Self::Key => KEY_MENU,
            Self::Scale => SCALE_MENU,
            Self::KeyboardParam(id) => keyboard_control_rect(id, false),
            Self::ArpRate => arp_rate_rect(),
            Self::ArpPattern => arp_direction_rect(),
        }
    }
}

impl ChordboardView {
    /// Navigate the displayed rows, including grouped menus and disabled choices.
    pub(super) fn menu_neighbor(&self, menu: Menu, index: usize, code: Code) -> usize {
        let current = self.menu_option_rect(menu, index);
        let horizontal = matches!(code, Code::ArrowLeft | Code::ArrowRight);
        let forward = matches!(code, Code::ArrowRight | Code::ArrowDown);
        let mut candidates: Vec<_> = (0..menu.items().len())
            .filter(|&next| next != index && self.route_menu_allowed(menu, next))
            .filter_map(|next| {
                let r = self.menu_option_rect(menu, next);
                let primary = if horizontal { r.0 - current.0 } else { r.1 - current.1 };
                if (horizontal && (r.1 - current.1).abs() > 0.5)
                    || (forward && primary <= 0.5) || (!forward && primary >= -0.5) {
                    return None;
                }
                let secondary = if horizontal { 0.0 } else { (r.0 - current.0).abs() };
                Some((next, primary.abs(), secondary))
            }).collect();
        candidates.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)));
        candidates.first().map_or(index, |candidate| candidate.0)
    }
}
