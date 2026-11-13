use super::*;

impl AutomixerView {
    pub(super) fn tooltip(&self) -> &'static str {
        if self.edit.is_some() {
            return "Type a value or a microphone name. Enter applies; Escape cancels. Names are saved with the project.";
        }
        if let Some(menu) = &self.menu {
            return match menu.kind {
                MenuKind::Channel(_) => "Choose the MIDI channel. Off ignores cues for this microphone. Scroll to move through the list.",
                MenuKind::Cc(_) => "Choose the weight CC, 0 through 119. Value 0 is -12 dB, 64 is 0 dB, and 127 is +12 dB. Scroll the list.",
            };
        }
        let Some(point) = self.hover else {
            return "All instances share this mixer. Click VOICE or OTHER on a strip to set its group. Drag a weight, or click its value to type.";
        };
        if self.bypass_at(point).is_some() {
            return "Bypass sends this mic at unity and removes it from its group, the mix, and the active count. Host bypass is controlled by the DAW.";
        }
        if self.group_at(point).is_some() {
            return "Voice mics share with each other. Chorus and voice assist apply only to them. Other mics share separately, with their own max GR.";
        }
        if point_in_rect(point.0, point.1, THEME) {
            return "Cycle Dark, Light, and Auto appearance.";
        }
        if point_in_rect(point.0, point.1, PREVIOUS) || point_in_rect(point.0, point.1, NEXT) {
            return "Browse the linked microphone strips.";
        }
        if point_in_rect(point.0, point.1, Self::channel_rect()) {
            return "MIDI channel for the selected microphone. Click to open the list. Off ignores cues. Scroll to step while the list is closed.";
        }
        if point_in_rect(point.0, point.1, Self::cc_rect()) {
            return "Weight CC for the selected microphone, 0-119. Click to open the list. The latest cue replaces the weight. Scroll to step while closed.";
        }
        if let Some((id, _)) = self.control_at(point) {
            return match id {
                Control::Amount => "Automix depth for every microphone. 0% is unity gain. 100% applies the full computed reduction, including quiet voice mics during chorus.",
                Control::VoiceCap => "Maximum reduction for Voice microphones, in sharing and in chorus. 0 dB keeps every voice mic at unity.",
                Control::OtherCap => "Maximum reduction for Other microphones. They gain-share only with each other and ignore chorus.",
                Control::Voices => "When more than N Voice mics are active, those active mics fade to unity. Quiet voice mics stay at Voice GR. Zero disables chorus. Other mics are not counted.",
                Control::Threshold => "Input RMS activity threshold, before weighting. 3 dB hysteresis and a 20 ms onset debounce prevent flicker.",
                Control::Hold => "Hold activity below threshold, and hold chorus after the voice count drops. This prevents chatter between voices.",
                Control::Fade => "Fade time for Automix and for voice chorus transitions. Quiet voice mics remain at Voice GR while chorus opens the active voice mics.",
                Control::Assist => "Voice group only. Speech confidence lowers that mic's activity threshold by up to 6 dB. 0% disables it. Other mics use the plain threshold.",
                Control::Mic => "Auto prefers the hardware input number when audio activates. Enter a number to pin the mic and its MIDI cue mapping.",
                Control::Weight(_) => "Priority, not output gain: -12 to +12 dB. Editing replaces the last MIDI cue. Shift drags finely; double-click resets.",
            };
        }
        "IN / OUT: RMS dBFS. GR: applied reduction in dB. VOICE shows speech confidence when voice assist is on. Linked instances must share a format and host process."
    }
}
