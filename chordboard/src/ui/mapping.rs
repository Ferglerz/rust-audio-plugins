use super::*;

impl ChordboardView {
    pub(super) fn draw_mapping(&self, d: &mut Draw) {
        for (axis, label) in ["X", "Y", "GATE"].iter().enumerate() {
            d.tab_button(axis_rect(axis), label, self.mapping_axis == axis, TEAL);
        }
        d.button(
            LEARN,
            if self.snapshot.learning == self.mapping_axis as u8 + 1 {
                "CANCEL LEARN"
            } else {
                "MIDI LEARN"
            },
            self.snapshot.learning == self.mapping_axis as u8 + 1,
            GOLD,
        );
        let m = self.mapping();
        for (menu, label) in [
            (
                Menu::MappingKind,
                format!(
                    "{} INPUT: {} ▾",
                    ["X", "Y", "GATE"][self.mapping_axis],
                    ["OFF", "CC 7-BIT", "CC 14-BIT", "PITCH BEND"][m.kind as usize]
                ),
            ),
            (
                Menu::MappingChannel(self.mapping().kind == 3),
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
            d.button(menu.trigger_rect(), &label, false, TEAL);
            d.outline(menu.trigger_rect(), LINE);
        }
        d.button(
            LEARN_OCTAVE,
            if self.snapshot.learning == 4 {
                "CANCEL OCTAVE LEARN"
            } else {
                "LEARN CONTROL OCTAVE"
            },
            self.snapshot.learning == 4,
            GOLD,
        );
        d.text(
            600.0,
            687.0,
            if self.snapshot.learning == 4 {
                "Play your keyboard's lowest key."
            } else {
                "Reserve the lowest octave for chord controls."
            },
            TEXT_SMALL,
            MUTED,
        );
        d.text(
            32.0,
            738.0,
            if (1..=3).contains(&self.snapshot.learning) {
                "Move the controller to assign it. Click Cancel Learn to stop."
            } else {
                "Choose X, Y or Gate, then learn a controller or select its source."
            },
            TEXT_SMALL,
            MUTED,
        );
    }
}
