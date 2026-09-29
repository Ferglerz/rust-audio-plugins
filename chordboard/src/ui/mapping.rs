use super::*;

impl ChordboardView {
    pub(super) fn draw_panel(&self, d: &mut Draw) {
        let Some(panel) = self.panel else {
            return;
        };
        let r = panel.rect();
        d.rounded_rect(r.0 - 4.0, r.1 - 4.0, r.2 + 8.0, r.3 + 8.0, 10.0, BG);
        d.rounded_rect(r.0, r.1, r.2, r.3, 8.0, PANEL);
        d.outline_rounded(r.0, r.1, r.2, r.3, 8.0, LINE, 1.0);
        match panel {
            Panel::Mapping => self.draw_mapping(d),
            Panel::Output => {
                d.text(
                    r.0 + 16.0,
                    r.1 + 23.0,
                    if self.params.mpe_enabled() {
                        "MPE OUTPUT"
                    } else {
                        "MIDI OUTPUT"
                    },
                    TEXT_LABEL,
                    TEAL,
                );
                d.text(
                    r.0 + 16.0,
                    r.1 + r.3 - 16.0,
                    if self.params.mpe_enabled() {
                        "Each voice uses its own member channel."
                    } else {
                        "Channels apply to standard MIDI output."
                    },
                    TEXT_SMALL,
                    MUTED,
                );
            }
        }
    }
    fn draw_mapping(&self, d: &mut Draw) {
        d.text(
            616.0,
            326.0,
            ["X CONTROLLER", "Y CONTROLLER"][self.mapping_axis],
            TEXT_LABEL,
            TEAL,
        );
        d.button(
            LEARN,
            if self.learning() == self.mapping_axis as u8 + 1 {
                "CANCEL LEARN"
            } else {
                "MIDI LEARN"
            },
            self.learning() == self.mapping_axis as u8 + 1,
            TEAL,
        );
        let m = self.mapping();
        for (menu, label) in [
            (
                Menu::MappingKind,
                format!(
                    "{} ▾",
                    ["OFF", "CC 7-BIT", "CC 14-BIT", "PITCH BEND"][m.kind as usize]
                ),
            ),
            (
                Menu::MappingChannel(m.kind == 3),
                format!(
                    "CH {} ▾",
                    if m.channel == 16 {
                        "ANY".into()
                    } else {
                        (m.channel + 1).to_string()
                    }
                ),
            ),
            (
                Menu::MappingCc(m.kind == 2),
                if matches!(m.kind, 1 | 2) {
                    format!("CC {} ▾", m.number)
                } else {
                    "CC —".into()
                },
            ),
        ] {
            if matches!(menu, Menu::MappingChannel(_)) && self.mapping_axis < 2 && m.kind != 3 {
                let r = menu.trigger_rect();
                let label = if m.channel == 16 || m.kind == 0 {
                    "ANY CHANNEL".into()
                } else {
                    format!("LEARNED CH {}", m.channel + 1)
                };
                d.text_centered(
                    r.0 + r.2 / 2.0,
                    r.1 + r.3 / 2.0 + 4.0,
                    &label,
                    TEXT_SMALL,
                    MUTED,
                );
            } else {
                d.button(menu.trigger_rect(), &label, true, TEAL);
            }
        }
        if self.mapping_axis == 1 {
            d.button(
                Menu::YTarget.trigger_rect(),
                &format!(
                    "Destination: {} ▾",
                    Menu::YTarget.items()[self.params.y_target.value() as usize]
                ),
                self.menu == Some(Menu::YTarget),
                TEAL,
            );
        } else {
            d.text(
                616.0,
                530.0,
                if (1..=2).contains(&self.learning()) {
                    "Move a controller to assign it."
                } else {
                    "Drag MIN / MAX on the strum pad to set its range."
                },
                TEXT_SMALL,
                MUTED,
            );
        }
    }
}
