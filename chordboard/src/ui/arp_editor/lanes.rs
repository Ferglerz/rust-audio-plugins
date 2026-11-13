use crate::arpeggiator::Step;
use crate::midi_fx::DelayTap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Lane {
    #[default]
    Velocity,
    Chance,
    Octave,
    Gate,
    Mod1,
    Mod2,
}

pub(super) const LANES: [Lane; 6] = [
    Lane::Velocity,
    Lane::Gate,
    Lane::Chance,
    Lane::Octave,
    Lane::Mod1,
    Lane::Mod2,
];

impl Lane {
    // Existing Tabler glyphs in Vizia's bundled font, verified against its cmap.
    pub(super) fn glyph(self) -> &'static str {
        match self {
            Self::Velocity => "\u{eab1}",          // gauge
            Self::Chance => "\u{eb66}",            // dice
            Self::Octave => "\u{eb5b}",            // arrows-vertical
            Self::Gate => "\u{ea70}",            // clock
            Self::Mod1 | Self::Mod2 => "\u{ecd4}", // wave-sine
        }
    }
    pub(super) fn description(self) -> &'static str {
        match self {
            Self::Velocity => "Note strength",
            Self::Chance => "Play probability",
            Self::Octave => "Pitch in octaves",
            Self::Gate => "Note duration",
            Self::Mod1 => "Bipolar source 1",
            Self::Mod2 => "Bipolar source 2",
        }
    }
    pub(super) fn get(self, s: &Step) -> f32 {
        match self {
            Self::Velocity => s.velocity as f32,
            Self::Chance => s.probability as f32,
            Self::Octave => s.octave as f32,
            Self::Gate => s.gate as f32,
            Self::Mod1 => s.mods[0] as f32,
            Self::Mod2 => s.mods[1] as f32,
        }
    }
    pub(super) fn set(self, s: &mut Step, value: f32) {
        let (lo, hi) = self.range();
        let v = value.clamp(lo, hi).round();
        match self {
            Self::Velocity => s.velocity = v as u8,
            Self::Chance => s.probability = v as u8,
            Self::Octave => s.octave = v as i8,
            Self::Gate => s.gate = v as u8,
            Self::Mod1 => s.mods[0] = v as i16,
            Self::Mod2 => s.mods[1] = v as i16,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum TapLane {
    #[default]
    Velocity,
    Feedback,
    Pitch,
    Fine,
    Pan,
    Channel,
}

pub(super) const TAP_LANES: [TapLane; 6] = [
    TapLane::Velocity,
    TapLane::Feedback,
    TapLane::Pitch,
    TapLane::Fine,
    TapLane::Pan,
    TapLane::Channel,
];

impl TapLane {
    pub(super) fn get(self, t: &DelayTap) -> f32 {
        match self {
            Self::Velocity => t.velocity,
            Self::Feedback => t.feedback,
            Self::Pitch => t.pitch as f32,
            Self::Fine => t.fine,
            Self::Pan => t.pan,
            Self::Channel => t.channel as f32,
        }
    }
    pub(super) fn set(self, t: &mut DelayTap, value: f32) {
        let (lo, hi) = self.range();
        let v = value.clamp(lo, hi);
        match self {
            Self::Velocity => t.velocity = v,
            Self::Feedback => t.feedback = v,
            Self::Pitch => t.pitch = v.round() as i8,
            Self::Fine => t.fine = v,
            Self::Pan => t.pan = v,
            Self::Channel => t.channel = v.round() as u8,
        }
    }
}

pub(super) trait AdjustLane: Copy {
    fn label(self) -> &'static str;
    fn range(self) -> (f32, f32);
    fn format(self, value: f32) -> String;
}

impl AdjustLane for Lane {
    fn label(self) -> &'static str {
        match self {
            Self::Velocity => "Velocity",
            Self::Chance => "Chance",
            Self::Octave => "Octave",
            Self::Gate => "Gate",
            Self::Mod1 => "Mod 1",
            Self::Mod2 => "Mod 2",
        }
    }
    fn range(self) -> (f32, f32) {
        match self {
            Self::Velocity => (0.0, 200.0),
            Self::Chance => (0.0, 100.0),
            Self::Octave => (-4.0, 4.0),
            Self::Gate => (1.0, 100.0),
            Self::Mod1 | Self::Mod2 => (-100.0, 100.0),
        }
    }
    fn format(self, value: f32) -> String {
        match self {
            Self::Octave => format!("{value:+.0}"),
            Self::Mod1 | Self::Mod2 => format!("{value:+.0}%"),
            _ => format!("{value:.0}%"),
        }
    }
}

impl AdjustLane for TapLane {
    fn label(self) -> &'static str {
        match self {
            Self::Velocity => "Velocity",
            Self::Feedback => "Feedback",
            Self::Pitch => "Pitch",
            Self::Fine => "Fine Pitch",
            Self::Pan => "Pan",
            Self::Channel => "Output Channel",
        }
    }
    fn range(self) -> (f32, f32) {
        match self {
            Self::Velocity => (0.0, 2.0),
            Self::Feedback => (0.0, 0.95),
            Self::Pitch => (-24.0, 24.0),
            Self::Fine => (-100.0, 100.0),
            Self::Pan => (-1.0, 1.0),
            Self::Channel => (0.0, 16.0),
        }
    }
    fn format(self, value: f32) -> String {
        match self {
            Self::Velocity | Self::Feedback => format!("{:.0}%", value * 100.0),
            Self::Pitch => format!("{value:+.0} st"),
            Self::Fine => format!("{value:+.0} ct"),
            Self::Pan => {
                if value.abs() < 0.005 {
                    "Center".into()
                } else {
                    format!(
                        "{} {:.0}",
                        if value < 0.0 { "L" } else { "R" },
                        value.abs() * 100.0
                    )
                }
            }
            Self::Channel => {
                if value < 0.5 {
                    "Auto".into()
                } else {
                    format!("Ch {value:.0}")
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Dropdown {
    TapLane,
    Pages,
    Paste(usize),
}

impl Dropdown {
    pub(super) fn len(self) -> usize {
        match self {
            Self::TapLane => TAP_LANES.len(),
            Self::Pages => 8,
            Self::Paste(_) => 2,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Chooser {
    Order,
}

#[derive(Clone, Copy)]
pub(super) enum DragKind {
    Step(Lane),
    Timing,
    Tap(TapLane),
    TapTime,
    BasicTime,
    BasicGain,
    BasicFeedback,
}

#[derive(Clone, Copy)]
pub(super) struct Drag {
    pub kind: DragKind,
    pub page: usize,
    pub origin: usize,
    pub last: usize,
    pub x: f32,
    pub y: f32,
    pub value: f32,
    pub last_value: f32,
    pub reset: bool,
    pub pass_len: usize,
}

pub(in crate::ui) struct ArpUi {
    pub edit_page: usize,
    pub(super) lane: Lane,
    pub(super) clipboard: Option<(crate::arpeggiator::Page, crate::arpeggiator::Settings)>,
    pub(in crate::ui) tracker_clipboard: Option<crate::arpeggiator::TrackerRow>,
    pub(super) tap_lane: TapLane,
    pub(super) selected_tap: usize,
    pub(super) dropdown: Option<Dropdown>,
    pub(super) chooser: Option<Chooser>,
    pub(super) chooser_draw: Chooser,
    pub(super) chooser_position: f32,
    pub(super) chooser_start: f32,
    pub(super) chooser_elapsed: f32,
    pub(super) chooser_cursor: usize,
    pub(super) page_from: usize,
    pub(super) page_elapsed: f32,
    pub(super) drag: Option<Drag>,
    pub(super) pending_tap_remove: Option<usize>,
}

impl Default for ArpUi {
    fn default() -> Self {
        Self {
            edit_page: 0,
            lane: Lane::Velocity,
            clipboard: None,
            tracker_clipboard: None,
            tap_lane: TapLane::Velocity,
            selected_tap: 0,
            dropdown: None,
            chooser: None,
            chooser_draw: Chooser::Order,
            chooser_position: 0.0,
            chooser_start: 0.0,
            chooser_elapsed: pleasant_ui::page_slide::DURATION,
            chooser_cursor: 0,
            page_from: 0,
            page_elapsed: pleasant_ui::page_slide::DURATION,
            drag: None,
            pending_tap_remove: None,
        }
    }
}
