/// Hi-hat continuous controller (CC04) state machine & choke logic.
/// Matches SCD BUILD_DATA.js ccSplits: [0, 16, 34, 59, 84, 127]
pub const CC_SPLITS: [u8; 6] = [0, 16, 34, 59, 84, 127];
const TIP_NOTES: [u8; 5] = [66, 67, 68, 69, 70];
const SHOULDER_NOTES: [u8; 5] = [60, 61, 62, 63, 64];

pub struct HiHatTracker {
    pub cc_value: u8,
}

impl Default for HiHatTracker {
    fn default() -> Self {
        Self { cc_value: 127 } // Default fully open
    }
}

impl HiHatTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_cc(&mut self, val: u8) {
        self.cc_value = val.min(127);
    }

    /// Apply the header Invert toggle (`127 - value` when enabled).
    pub fn process_cc(raw: u8, invert: bool) -> u8 {
        let raw = raw.min(127);
        if invert {
            127 - raw
        } else {
            raw
        }
    }

    /// Translate note based on CC4 position
    pub fn translate_note(&self, note: u8) -> u8 {
        // Tip notes: Firm=66, A=67, B=68, C=69, Open=70
        // Shoulder notes: Firm=60, A=61, B=62, C=63, Open=64
        let split_idx = self.get_split_index().min(TIP_NOTES.len() - 1);
        match note {
            46 | 42 => TIP_NOTES[split_idx],
            26 | 22 => SHOULDER_NOTES[split_idx],
            _ => note,
        }
    }

    /// Determine if this note chokes previous open hi-hat voices
    pub fn is_choke_trigger(note: u8) -> bool {
        matches!(note, 44 | 60 | 66) // Stomp or fully closed firm hits
    }

    fn get_split_index(&self) -> usize {
        for i in 0..CC_SPLITS.len() - 1 {
            if self.cc_value >= CC_SPLITS[i] && self.cc_value <= CC_SPLITS[i + 1] {
                return i;
            }
        }
        CC_SPLITS.len() - 2
    }
}
