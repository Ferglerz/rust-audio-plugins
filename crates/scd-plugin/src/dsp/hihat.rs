/// Hi-hat continuous controller (CC04) state machine & choke logic.
/// Matches SCD BUILD_DATA.js ccSplits: [0, 16, 34, 59, 84, 127]

pub const CC_SPLITS: [u8; 6] = [0, 16, 34, 59, 84, 127];

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

    pub fn set_cc4(&mut self, val: u8) {
        self.cc_value = val.min(127);
    }

    /// Translate note based on CC4 position
    pub fn translate_note(&self, note: u8) -> u8 {
        // Tip notes: Firm=66, A=67, B=68, C=69, Open=70
        // Shoulder notes: Firm=60, A=61, B=62, C=63, Open=64
        let split_idx = self.get_split_index();
        match note {
            46 | 42 => { // Hihat Tip triggers
                match split_idx {
                    0 => 66, // Firm
                    1 => 67, // A
                    2 => 68, // B
                    3 => 69, // C
                    _ => 70, // Open
                }
            }
            26 | 22 => { // Hihat Shoulder triggers
                match split_idx {
                    0 => 60, // Firm
                    1 => 61, // A
                    2 => 62, // B
                    3 => 63, // C
                    _ => 64, // Open
                }
            }
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
