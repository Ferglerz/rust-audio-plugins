use super::*;

impl ScdEditorView {
    pub(super) fn tooltip(&self) -> &'static str {
        if self.note_edit.is_some() {
            return "Type a MIDI note number · Enter applies · Escape cancels";
        }
        if self.vel_map_open.is_some() {
            return "Drag velocity curve points and handles to shape the response · Play MIDI to see incoming velocities";
        }
        if self.samples_open {
            return "Click a note to edit or learn it · Alt-click to reset notes · Click an instrument name for velocity mapping";
        }
        if self.sub_kick_open {
            return "Adjust the sub-kick settings · Click outside to close";
        }
        if self.add_preset_open {
            return "Name the preset and choose what to save · Enter saves · Escape cancels";
        }
        if self.cc_menu_open {
            return "Choose the MIDI CC used for hi-hat pedal control";
        }
        if self.preset_open {
            return "Choose a preset or save the current settings";
        }
        let (x, y) = self.mouse;
        for (rect, text) in [
            (
                (PRESET_X, PRESET_Y, PRESET_W, PRESET_H),
                "Load, save, or reset a preset.",
            ),
            (
                (SAMPLES_X, SAMPLES_Y, SAMPLES_W, SAMPLES_H),
                "Open MIDI note assignments and instrument velocity mapping.",
            ),
            (CC_INVERT, "Reverse the hi-hat pedal response to the assigned MIDI CC."),
            (
                CC_SELECT,
                "Choose the hi-hat pedal controller number; scroll to adjust.",
            ),
            (
                Self::lock_rect(),
                "Lock the faders together for linked adjustment.",
            ),
            (SUB_KICK, "Open the sub-kick settings."),
        ] {
            if Self::hit(x, y, rect.0, rect.1, rect.2, rect.3) {
                return text;
            }
        }
        if Self::sof_at(x, y).is_some() {
            return "Choose the microphone channel shown by the mixer.";
        }
        for piece in self.visible_pieces() {
            let sx = Self::strip_x(piece);
            if (sx..=sx + STRIP_W).contains(&x) {
                if (PITCH_Y..=PITCH_Y + SLIDER_H).contains(&y) {
                    return "Pitch: drag to tune this instrument · Double-click or Alt-click to reset";
                }
                if (PAN_Y..=PAN_Y + SLIDER_H).contains(&y) {
                    return "Pan: drag to position this instrument · Double-click or Alt-click to reset";
                }
                if (FADER_Y..=FADER_Y + FADER_H).contains(&y) {
                    return "Level: drag the fader · Double-click or Alt-click to reset · Lock links the faders";
                }
                if (PUNCH_Y..=PUNCH_Y + PUNCH_SIZE).contains(&y) {
                    return "Punch: drag to adjust this instrument's transient character · Double-click or Alt-click to reset";
                }
                if (ICON_Y..=ICON_Y + ICON_H).contains(&y) {
                    return "Open Mapping and click the instrument name to shape its MIDI velocity response";
                }
            }
        }
        "Hover a control for help · Drag to adjust · Double-click or Alt-click to reset"
    }
}
