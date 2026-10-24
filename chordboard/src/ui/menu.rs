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
            Self::Meter(den) => {
                return if den {
                    (0..=5).map(|n| (1 << n).to_string()).collect()
                } else {
                    (1..=32).map(|n| n.to_string()).collect()
                }
            }
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
                return ARP_PATTERN_NAMES
                    .iter()
                    .map(|label| (*label).to_string())
                    .collect()
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
            Self::Meter(den) => {
                if den {
                    "Beat unit"
                } else {
                    "Beats per bar"
                }
            }
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
            Self::Scale => 230.0,
            Self::Meter(_) | Self::KeyboardParam(_) => 40.0,
            Self::ArpRate => 112.0,
            Self::ArpPattern => 148.0,
        }
    }

    pub(super) fn columns(self) -> usize {
        match self {
            Self::RouteSource => 2,
            Self::RouteTarget => 4,
            Self::Scale => 3,
            Self::ArpRate => 2,
            Self::ArpPattern => 1,
            Self::Key => 3,
            Self::Meter(_) | Self::KeyboardParam(_) => 6,
        }
    }

    pub(super) fn groups(self) -> Vec<(&'static str, Vec<usize>)> {
        match self {
            Self::Scale => harmony::SCALE_FAMILIES
                .iter()
                .map(|(name, indices)| (*name, indices.to_vec()))
                .chain(std::iter::once(("Pentatonic", vec![6, 7])))
                .collect(),
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
                        | "gate" | "swing" | "octaves" | "humanize" | "arp_octaves"
                        | "arp_sync" | "arp_free_ms" | "arp_mode" | "arp_page"
                        | "arp_loop_1_octave" | "arp_loop_2_octave" | "arp_loop_3_octave"
                        | "arp_loop_4_octave" => 2,
                        "velocity" | "length_ms" | "contour" | "root_on_select" | "latch" => 3,
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
        if self == Self::Scale {
            return self
                .groups()
                .into_iter()
                .enumerate()
                .map(|(i, (name, _))| {
                    let heading = if i < 3 {
                        (r.0 + 12.0 + i as f32 * 238.0, r.1 + 150.0, 230.0, 20.0)
                    } else {
                        (r.0 + 12.0, r.1 + 424.0, r.2 - 24.0, 20.0)
                    };
                    (name, heading)
                })
                .collect();
        }
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
        if matches!(self, Self::Key | Self::Scale) {
            let root = KEY_MENU;
            return (root.0, root.1 + root.3 + 6.0, 730.0, 492.0);
        }
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
        if self == Self::Key {
            let (_, note, spelling) = harmony::KEY_CHOICES[index];
            let key = root_key_rect(r, note);
            return if piano_black(note) {
                (
                    key.0,
                    key.1 + if spelling == 2 { key.3 * 0.5 } else { 0.0 },
                    key.2,
                    key.3 * 0.5,
                )
            } else {
                key
            };
        }
        if self == Self::Scale {
            for (column, (_, indices)) in harmony::SCALE_FAMILIES.iter().enumerate() {
                if let Some(row) = indices.iter().position(|&i| i == index) {
                    return (
                        r.0 + 12.0 + column as f32 * 238.0,
                        r.1 + 176.0 + row as f32 * 34.0,
                        230.0,
                        30.0,
                    );
                }
            }
            return (
                r.0 + 12.0 + (index - 6) as f32 * 238.0,
                r.1 + 450.0,
                230.0,
                30.0,
            );
        }
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
            Self::Meter(den) => {
                if den {
                    METER_DEN
                } else {
                    METER_NUM
                }
            }
            Self::RouteSource => ROUTE_SOURCE,
            Self::RouteTarget => ROUTE_TARGET,
            Self::Key => KEY_MENU,
            Self::Scale => KEY_MENU,
            Self::KeyboardParam(id) => keyboard_control_rect(id, false),
            Self::ArpRate => arp_rate_rect(),
            Self::ArpPattern => arp_direction_rect(),
        }
    }
}

// Use the same natural/accidental proportions as the performance keyboard.
pub(super) fn root_key_rect(r: Rect, note: u8) -> Rect {
    let width = (r.2 - 24.0) / 7.0;
    let before = (0..note).filter(|&n| !piano_black(n)).count() as f32;
    if piano_black(note) {
        (
            r.0 + 12.0 + (before - 0.31) * width,
            r.1 + 38.0,
            width * 0.62,
            100.0 * 0.62,
        )
    } else {
        (r.0 + 12.0 + before * width, r.1 + 38.0, width, 100.0)
    }
}

impl ChordboardView {
    pub(super) fn root_choice_at(&self, x: f32, y: f32) -> Option<usize> {
        let bounds = self.menu_bounds(Menu::Key);
        // Accidentals are in front of the natural keys, including both spellings.
        [true, false].into_iter().find_map(|black| {
            harmony::KEY_CHOICES
                .iter()
                .enumerate()
                .filter(|(_, (_, note, _))| piano_black(*note) == black)
                .find(|(i, _)| hit(Menu::Key.option_rect_at(bounds, *i), x, y))
                .map(|(i, _)| i)
        })
    }
    pub(super) fn draw_root_menu(&self, d: &mut Draw, menu: Menu) {
        let bounds = self.menu_bounds(menu);
        self.surface(d, bounds);
        d.text(bounds.0 + 12.0, bounds.1 + 24.0, "Root note", 12.0, MUTED);
        for black in [false, true] {
            for note in (0..12u8).filter(|&n| piano_black(n) == black) {
                let r = root_key_rect(bounds, note);
                let fill = if black {
                    if d.light {
                        TEXT
                    } else {
                        BG
                    }
                } else if d.light {
                    BG
                } else {
                    alpha(TEXT, 0.65)
                };
                d.rect(r.0, r.1, r.2, r.3, fill);
                d.outline(r, if black || d.light { LINE } else { BG });
                for (i, (label, pitch, _)) in harmony::KEY_CHOICES
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, pitch, _))| *pitch == note)
                {
                    let option = Menu::Key.option_rect_at(bounds, i);
                    let on = *pitch as i32 == self.params.key.value()
                        && i == self.menu_selection(Menu::Key);
                    if on
                        || self
                            .hover_pointer()
                            .and_then(|(x, y)| self.root_choice_at(x, y))
                            == Some(i)
                    {
                        d.rounded_rect(
                            option.0 + 1.5,
                            option.1 + 2.0,
                            option.2 - 3.0,
                            option.3 - 4.0,
                            1.0,
                            alpha(GOLD, if on { 0.7 } else { 0.3 }),
                        );
                    }
                    if menu == Menu::Key && self.menu_cursor == i {
                        d.outline_rounded(
                            option.0 + 1.5,
                            option.1 + 2.0,
                            option.2 - 3.0,
                            option.3 - 4.0,
                            1.0,
                            GOLD,
                            1.5,
                        );
                    }
                    d.text_centered(
                        option.0 + option.2 * 0.5,
                        option.1 + option.3 - 8.0,
                        &music::label(label),
                        11.0,
                        if on {
                            GOLD
                        } else if black {
                            if d.light {
                                BG
                            } else {
                                TEXT
                            }
                        } else if d.light {
                            TEXT
                        } else {
                            BG
                        },
                    );
                }
            }
        }
        for (label, heading) in Menu::Scale.group_heading_rects(bounds) {
            d.text(heading.0, heading.1 + 14.0, label, TEXT_SMALL, GOLD);
        }
        for (i, label) in Menu::Scale.items().iter().enumerate() {
            let r = self.menu_option_rect(Menu::Scale, i);
            self.button(d, r, label, i == self.menu_selection(Menu::Scale), GOLD);
            if menu == Menu::Scale && self.menu_cursor == i {
                d.outline_rounded(r.0, r.1, r.2, r.3, 6.0, GOLD, 1.0);
            }
        }
    }
    /// Navigate the displayed rows, including grouped menus and disabled choices.
    pub(super) fn menu_neighbor(&self, menu: Menu, index: usize, code: Code) -> usize {
        let current = self.menu_option_rect(menu, index);
        let horizontal = matches!(code, Code::ArrowLeft | Code::ArrowRight);
        let forward = matches!(code, Code::ArrowRight | Code::ArrowDown);
        let mut candidates: Vec<_> = (0..menu.items().len())
            .filter(|&next| next != index && self.route_menu_allowed(menu, next))
            .filter_map(|next| {
                let r = self.menu_option_rect(menu, next);
                let primary = if horizontal {
                    r.0 - current.0
                } else {
                    r.1 - current.1
                };
                if (horizontal && (r.1 - current.1).abs() > 0.5)
                    || (forward && primary <= 0.5)
                    || (!forward && primary >= -0.5)
                {
                    return None;
                }
                let secondary = if horizontal {
                    0.0
                } else {
                    (r.0 - current.0).abs()
                };
                Some((next, primary.abs(), secondary))
            })
            .collect();
        candidates.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)));
        candidates.first().map_or(index, |candidate| candidate.0)
    }
}
