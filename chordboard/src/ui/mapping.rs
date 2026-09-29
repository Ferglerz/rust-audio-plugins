use super::*;

impl ChordboardView {
    pub(super) fn draw_panel(&self, d: &mut Draw) {
        let Some(panel) = self.panel else {
            return;
        };
        let r = self.panel_rect(panel);
        d.rounded_rect(r.0 - 4.0, r.1 - 4.0, r.2 + 8.0, r.3 + 8.0, 12.0, BG);
        self.surface(d, r);
        self.button(d, self.panel_close_rect(panel), "×", false, TEAL);
        match panel {
            Panel::Mapping => self.draw_mapping(d),
            Panel::Output => {
                d.text(
                    r.0 + 16.0,
                    r.1 + 23.0,
                    if self.params.mpe_enabled() {
                        "MPE output"
                    } else {
                        "Regular MIDI output"
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
        let r = mapping_panel_rect(self.mapping_axis);
        let learning = self.learning() == self.mapping_axis as u8 + 1;
        let title = if learning {
            ["X · Move controller", "Y · Move controller"][self.mapping_axis]
        } else {
            ["X · Strum", "Y · Expression"][self.mapping_axis]
        };
        d.text(r.0 + 12.0, r.1 + 26.0, title, 12.0, TEAL);
        self.button(
            d,
            mapping_learn_rect(self.mapping_axis),
            if learning {
                "Cancel learn"
            } else {
                "MIDI Learn"
            },
            learning,
            TEAL,
        );
        let m = self.mapping();
        self.button(
            d,
            self.menu_trigger_rect(Menu::MappingKind),
            &format!(
                "{} ▾",
                ["Off", "CC 7-bit", "CC 14-bit", "Pitch bend"][m.kind as usize]
            ),
            self.menu == Some(Menu::MappingKind),
            TEAL,
        );
        let menu = if m.kind == 3 {
            Menu::MappingChannel(true)
        } else {
            Menu::MappingCc(m.kind == 2)
        };
        if matches!(m.kind, 1..=3) {
            let label = if m.kind == 3 {
                format!("Ch {} ▾", m.channel + 1)
            } else {
                format!("CC {} ▾", m.number)
            };
            self.button(
                d,
                self.menu_trigger_rect(menu),
                &label,
                self.menu == Some(menu),
                TEAL,
            );
        } else {
            let cc = self.menu_trigger_rect(menu);
            d.text_centered(cc.0 + cc.2 / 2.0, cc.1 + 18.0, "CC —", 11.0, MUTED);
        }
        if self.mapping_axis == 1 {
            self.button(
                d,
                self.menu_trigger_rect(Menu::YTarget),
                &format!(
                    "{} ▾",
                    Menu::YTarget.items()[self.params.y_target.value() as usize]
                ),
                self.menu == Some(Menu::YTarget),
                TEAL,
            );
        }
        if !learning && matches!(m.kind, 1 | 2) {
            let channel = if m.channel == 16 {
                "Any channel".into()
            } else {
                format!("Learned Ch {}", m.channel + 1)
            };
            d.text(
                r.0 + 256.0,
                r.1 + 66.0,
                if self.mapping_axis == 1 { &channel } else { "" },
                10.0,
                MUTED,
            );
            if self.mapping_axis == 0 {
                d.text(r.0 + 12.0, r.1 + 95.0, &channel, 10.0, MUTED);
            }
        }
    }
}
