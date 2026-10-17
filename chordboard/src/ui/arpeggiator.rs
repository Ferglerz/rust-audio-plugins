//! String-based arp adjustments and the independently powered MIDI delay.
//! All pointer geometry is shared with drawing; edits publish only from the UI thread.
use super::*;
use crate::arpeggiator::{DelaySettings, DelayTap, Step, StepState, MODE_NAMES, TAPS};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Lane {
    #[default]
    Velocity,
    Gate,
    Chance,
    Octave,
    Timing,
    Mod1,
    Mod2,
}
const LANES: [Lane; 7] = [
    Lane::Velocity,
    Lane::Gate,
    Lane::Chance,
    Lane::Octave,
    Lane::Timing,
    Lane::Mod1,
    Lane::Mod2,
];
impl Lane {
    // Existing Tabler glyphs in Vizia's bundled font, verified against its cmap.
    fn glyph(self) -> &'static str {
        match self {
            Self::Velocity => "\u{eab1}", // gauge
            Self::Gate => "\u{ef93}", // hourglass
            Self::Chance => "\u{eb66}", // dice
            Self::Octave => "\u{eb5b}", // arrows-vertical
            Self::Timing => "\u{ea70}", // clock
            Self::Mod1 | Self::Mod2 => "\u{ecd4}", // wave-sine
        }
    }
    fn description(self) -> &'static str {
        match self {
            Self::Velocity => "Note strength",
            Self::Gate => "Note duration",
            Self::Chance => "Play probability",
            Self::Octave => "Pitch in octaves",
            Self::Timing => "Early / late",
            Self::Mod1 => "Bipolar source 1",
            Self::Mod2 => "Bipolar source 2",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Velocity => "Velocity",
            Self::Gate => "Gate",
            Self::Chance => "Chance",
            Self::Octave => "Octave",
            Self::Timing => "Timing",
            Self::Mod1 => "Mod 1",
            Self::Mod2 => "Mod 2",
        }
    }
    fn range(self) -> (f32, f32) {
        match self {
            Self::Velocity => (0.0, 200.0),
            Self::Gate => (1.0, 100.0),
            Self::Chance => (0.0, 100.0),
            Self::Octave => (-4.0, 4.0),
            Self::Timing => (-49.0, 49.0),
            Self::Mod1 | Self::Mod2 => (-100.0, 100.0),
        }
    }
    fn get(self, s: &Step) -> f32 {
        match self {
            Self::Velocity => s.velocity as f32,
            Self::Gate => s.gate as f32,
            Self::Chance => s.probability as f32,
            Self::Octave => s.octave as f32,
            Self::Timing => s.micro as f32,
            Self::Mod1 => s.mods[0] as f32,
            Self::Mod2 => s.mods[1] as f32,
        }
    }
    fn set(self, s: &mut Step, value: f32) {
        let (lo, hi) = self.range();
        let v = value.clamp(lo, hi).round();
        match self {
            Self::Velocity => s.velocity = v as u8,
            Self::Gate => s.gate = v as u8,
            Self::Chance => s.probability = v as u8,
            Self::Octave => s.octave = v as i8,
            Self::Timing => s.micro = v as i16,
            Self::Mod1 => s.mods[0] = v as i16,
            Self::Mod2 => s.mods[1] = v as i16,
        }
    }
    fn format(self, value: f32) -> String {
        match self {
            Self::Octave => format!("{value:+.0}"),
            Self::Mod1 | Self::Mod2 | Self::Timing => format!("{value:+.0}%"),
            _ => format!("{value:.0}%"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum TapLane {
    #[default]
    Velocity,
    Feedback,
    Pitch,
    Fine,
    Pan,
    Channel,
}
const TAP_LANES: [TapLane; 6] = [
    TapLane::Velocity,
    TapLane::Feedback,
    TapLane::Pitch,
    TapLane::Fine,
    TapLane::Pan,
    TapLane::Channel,
];
impl TapLane {
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
    fn get(self, t: &DelayTap) -> f32 {
        match self {
            Self::Velocity => t.velocity,
            Self::Feedback => t.feedback,
            Self::Pitch => t.pitch as f32,
            Self::Fine => t.fine,
            Self::Pan => t.pan,
            Self::Channel => t.channel as f32,
        }
    }
    fn set(self, t: &mut DelayTap, value: f32) {
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
    fn format(self, v: f32) -> String {
        match self {
            Self::Velocity | Self::Feedback => format!("{:.0}%", v * 100.0),
            Self::Pitch => format!("{v:+.0} st"),
            Self::Fine => format!("{v:+.0} ct"),
            Self::Pan => {
                if v.abs() < 0.005 {
                    "Center".into()
                } else {
                    format!("{} {:.0}", if v < 0.0 { "L" } else { "R" }, v.abs() * 100.0)
                }
            }
            Self::Channel => {
                if v < 0.5 {
                    "Auto".into()
                } else {
                    format!("Ch {:.0}", v)
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dropdown {
    Mode,
    TapLane,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Chooser {
    Lane,
    Order,
}
#[derive(Clone, Copy)]
enum DragKind {
    Step(Lane),
    Timing,
    Tap(TapLane),
    TapTime,
    BasicTime,
    BasicGain,
    BasicFeedback,
}
#[derive(Clone, Copy)]
struct Drag {
    kind: DragKind,
    page: usize,
    origin: usize,
    last: usize,
    x: f32,
    y: f32,
    value: f32,
    last_value: f32,
    reset: bool,
}
pub(super) struct ArpUi {
    pub edit_page: usize,
    lane: Lane,
    tap_lane: TapLane,
    selected_tap: usize,
    dropdown: Option<Dropdown>,
    chooser: Option<Chooser>,
    chooser_draw: Chooser,
    chooser_position: f32,
    chooser_start: f32,
    chooser_elapsed: f32,
    chooser_cursor: usize,
    page_from: usize,
    observed_page: usize,
    page_elapsed: f32,
    drag: Option<Drag>,
    delay_open: bool,
    delay_position: f32,
    delay_start: f32,
    delay_elapsed: f32,
    advanced: bool,
}
impl Default for ArpUi {
    fn default() -> Self {
        Self {
            edit_page: 0,
            lane: Lane::Velocity,
            tap_lane: TapLane::Velocity,
            selected_tap: 0,
            dropdown: None,
            chooser: None,
            chooser_draw: Chooser::Lane,
            chooser_position: 0.0,
            chooser_start: 0.0,
            chooser_elapsed: pleasant_ui::page_slide::DURATION,
            chooser_cursor: 0,
            page_from: 0,
            observed_page: 0,
            page_elapsed: pleasant_ui::page_slide::DURATION,
            drag: None,
            delay_open: false,
            delay_position: 0.0,
            delay_start: 0.0,
            delay_elapsed: pleasant_ui::page_slide::DURATION,
            advanced: false,
        }
    }
}

fn header_slot(index: usize) -> Rect {
    let s = ARP_SURFACE;
    let widths = [130.0, 148.0, 92.0, 96.0, 138.0, 142.0, 142.0];
    let start = s.0 + 190.0;
    let remaining = s.2 - 190.0 - 108.0 - 6.0 * 8.0;
    let scale = remaining / widths.iter().sum::<f32>();
    let x = start + widths[..index].iter().sum::<f32>() * scale + index as f32 * 8.0;
    (x, s.1 + 10.0, widths[index] * scale, 28.0)
}
pub(super) fn power_rect() -> Rect {
    (ARP_SURFACE.0 + 12.0, ARP_SURFACE.1 + 10.0, 28.0, 28.0)
}
pub(super) fn mode_rect() -> Rect {
    header_slot(0)
}
pub(super) fn order_rect() -> Rect {
    header_slot(1)
}
pub(super) fn octaves_rect() -> Rect {
    header_slot(2)
}
pub(super) fn sync_rect() -> Rect {
    header_slot(3)
}
pub(super) fn rate_rect() -> Rect {
    header_slot(4)
}
pub(super) fn humanize_rect() -> Rect {
    header_slot(5)
}
pub(super) fn swing_rect() -> Rect {
    header_slot(6)
}
pub(super) fn delay_rect() -> Rect {
    (
        ARP_SURFACE.0 + ARP_SURFACE.2 - 100.0,
        ARP_SURFACE.1 + 10.0,
        88.0,
        28.0,
    )
}
fn page_rect(page: usize) -> Rect {
    (
        ARP_SURFACE.0 + 58.0 + page as f32 * 34.0,
        ARP_SURFACE.1 + 49.0,
        28.0,
        25.0,
    )
}
fn page_block() -> Rect {
    (page_rect(0).0, page_rect(0).1, 266.0, 25.0)
}
fn lane_rect() -> Rect {
    (ARP_SURFACE.0 + 350.0, ARP_SURFACE.1 + 49.0, 154.0, 25.0)
}
fn graph_rect() -> Rect {
    (
        ARP_SURFACE.0 + 22.0,
        ARP_SURFACE.1 + 94.0,
        ARP_SURFACE.2 - 44.0,
        ARP_SURFACE.3 - 139.0,
    )
}
struct StringLayout {
    x: [f32; 96],
    interval: [f32; 96],
    spacing: f32,
    len: usize,
}
impl StringLayout {
    fn new(steps: &[Step; 96], len: usize, swing: f32, phase: i64) -> Self {
        let swing = if len > 1 { swing.clamp(-0.75, 0.75) } else { 0.0 };
        // Reserve the extremes of both swing and ±49% microtiming inside the graph.
        let spacing = graph_rect().2 / (len.max(1) as f32 + swing.abs());
        let mut result = Self { x: [0.0; 96], interval: [spacing; 96], spacing, len };
        let mut clock = phase;
        let origin = if phase.rem_euclid(2) != 0 { swing } else { 0.0 };
        for (i, step) in steps.iter().enumerate().take(len) {
            let odd = clock.rem_euclid(2) != 0;
            result.x[i] = graph_rect().0 + (i as f32 + 0.5 + swing.abs() * 0.5 + if odd { swing } else { 0.0 } - origin) * spacing;
            result.interval[i] = spacing * (1.0 + if odd { -swing } else { swing });
            if step.state != StepState::Skip { clock += 1; }
        }
        result
    }
    fn at(&self, x: f32) -> usize {
        (0..self.len).min_by(|&a, &b| (self.x[a] - x).abs().total_cmp(&(self.x[b] - x).abs())).unwrap_or(0)
    }
    fn status(&self, index: usize) -> Rect {
        let w = (self.spacing - 4.0).clamp(9.0, 36.0);
        (self.x[index] - w * 0.5, ARP_SURFACE.1 + ARP_SURFACE.3 - 27.0, w, 18.0)
    }
}
fn repeat_icon(mode: usize) -> style::Icon {
    match mode {
        0 => style::Icon::Once,
        1 => style::Icon::Loop,
        2 => style::Icon::SyncLoop,
        _ => style::Icon::AlignedLoop,
    }
}

fn dropdown_rect(which: Dropdown, index: usize) -> Rect {
    let r = match which {
        Dropdown::Mode => mode_rect(),
        Dropdown::TapLane => tap_lane_rect(),
    };
    let n = match which {
        Dropdown::Mode => 4,
        Dropdown::TapLane => 6,
    };
    let h = 22.0;
    let y = (r.1 + r.3 + 4.0).min(ARP_SURFACE.1 + ARP_SURFACE.3 - n as f32 * h - 6.0);
    (r.0, y + index as f32 * h, r.2.max(154.0), h)
}
fn chooser_area() -> Rect {
    let s = ARP_SURFACE;
    (s.0 + 16.0, s.1 + 85.0, s.2 - 32.0, s.3 - 97.0)
}
pub(super) fn chooser_card(index: usize) -> Rect {
    let r = chooser_area();
    let gap = 8.0;
    let w = (r.2 - gap * 6.0) / 7.0;
    (r.0 + index as f32 * (w + gap), r.1, w, r.3)
}

// These are the ordering depictions from 00d16be's performance controls.
const ORDER_CONTOURS: [[u8; 5]; 7] = [
    [0, 1, 2, 3, 4], [4, 3, 2, 1, 0], [0, 2, 4, 2, 0],
    [0, 2, 1, 3, 2], [4, 2, 3, 1, 2], [0, 3, 1, 2, 4], [2, 4, 0, 3, 1],
];

fn delay_close_rect() -> Rect {
    delay_rect()
}
fn delay_power_rect() -> Rect {
    power_rect()
}
fn advanced_rect() -> Rect {
    (delay_rect().0 - 122.0, delay_rect().1, 112.0, 28.0)
}
fn delay_sync_rect() -> Rect {
    (ARP_SURFACE.0 + 20.0, ARP_SURFACE.1 + 48.0, 106.0, 26.0)
}
fn delay_time_rect() -> Rect {
    (ARP_SURFACE.0 + 134.0, ARP_SURFACE.1 + 48.0, 164.0, 26.0)
}
fn delay_gain_rect() -> Rect {
    (ARP_SURFACE.0 + 306.0, ARP_SURFACE.1 + 48.0, 152.0, 26.0)
}
fn delay_feedback_rect() -> Rect {
    (ARP_SURFACE.0 + 466.0, ARP_SURFACE.1 + 48.0, 156.0, 26.0)
}
fn tap_lane_rect() -> Rect {
    (ARP_SURFACE.0 + 630.0, ARP_SURFACE.1 + 48.0, 170.0, 26.0)
}
fn ping_rect() -> Rect {
    (ARP_SURFACE.0 + 808.0, ARP_SURFACE.1 + 48.0, 104.0, 26.0)
}
fn add_rect() -> Rect {
    (
        ARP_SURFACE.0 + ARP_SURFACE.2 - 80.0,
        ARP_SURFACE.1 + 48.0,
        28.0,
        26.0,
    )
}
fn remove_rect() -> Rect {
    (
        ARP_SURFACE.0 + ARP_SURFACE.2 - 44.0,
        ARP_SURFACE.1 + 48.0,
        28.0,
        26.0,
    )
}
const SOURCE_NAMES: [&str; 5] = ["Arp", "Manual Strum", "Chords", "Bass", "Melody"];
fn source_rect(index: usize) -> Rect {
    (
        ARP_SURFACE.0 + 78.0 + index as f32 * 124.0,
        ARP_SURFACE.1 + 81.0,
        116.0,
        23.0,
    )
}
fn tap_graph(advanced: bool) -> Rect {
    (
        ARP_SURFACE.0 + 26.0,
        ARP_SURFACE.1 + if advanced { 123.0 } else { 100.0 },
        ARP_SURFACE.2 - 52.0,
        if advanced { 79.0 } else { 102.0 },
    )
}
fn time_range(sync: bool) -> (f32, f32) {
    if sync {
        (1.0 / 64.0, 16.0)
    } else {
        (1.0, 8000.0)
    }
}
// Keep a stable domain while dragging so the selected tap never runs away from the pointer.
fn time_domain(delay: &DelaySettings) -> f32 {
    delay.taps[..delay.count as usize]
        .iter()
        .map(|t| t.time)
        .fold(if delay.sync { 2.0 } else { 1000.0 }, f32::max)
        .mul_add(1.25, 0.0)
        .min(time_range(delay.sync).1)
}
fn tap_x(tap: &DelayTap, domain: f32, advanced: bool) -> f32 {
    let g = tap_graph(advanced);
    g.0 + (tap.time / domain).clamp(0.0, 1.0) * g.2
}
fn value_y(value: f32, range: (f32, f32), g: Rect) -> f32 {
    g.1 + g.3 * (1.0 - (value - range.0) / (range.1 - range.0))
}
fn paint_between(from: usize, to: usize, start: f32, end: f32, mut apply: impl FnMut(usize, f32)) {
    for i in from.min(to)..=from.max(to) {
        let f = if from == to {
            1.0
        } else {
            (i as f32 - from as f32) / (to as f32 - from as f32)
        };
        apply(i, start + (end - start) * f);
    }
}
fn dotted(d: &mut Draw, x: f32, y: f32, h: f32, color: Color) {
    let mut yy = y;
    while yy < y + h {
        d.line(x, yy, x, (yy + 2.0).min(y + h), color, 1.0);
        yy += 5.0;
    }
}
fn time_label(time: f32, sync: bool) -> String {
    if !sync {
        return format!("{time:.0} ms");
    };
    ARP_RATES
        .iter()
        .find(|(_, b)| (*b - time).abs() < 0.0001)
        .map_or_else(|| format!("{time:.3} beats"), |(name, _)| (*name).into())
}

impl ChordboardView {
    pub(super) fn arp_chooser_open(&self) -> bool {
        self.arp_ui.chooser.is_some()
    }
    fn arp_string_layout(&self, page: usize, len: usize) -> StringLayout {
        let state = self.params.arpeggiator.snapshot();
        let steps = &state.pages[page].steps;
        let mut phase = 0;
        if self.snapshot.arp_running && self.snapshot.arp_page as usize == page {
            let ordinal = steps.iter().take(self.snapshot.arp_step as usize).filter(|s| s.state != StepState::Skip).count();
            phase = self.snapshot.arp_phase - ordinal as i64;
        }
        let swing = if self.snapshot.arp_page as usize == page {
            self.routed_plain("swing").unwrap_or(self.params.arp_pages[page].swing.value())
        } else { self.params.arp_pages[page].swing.value() };
        StringLayout::new(steps, len, swing, phase)
    }
    pub(super) fn arp_editor_overlay_visible(&self) -> bool {
        self.arp_ui.delay_open || self.arp_ui.delay_position > 0.0
    }
    fn arp_cycle_for_editor(&self) -> ([u8; 96], usize) {
        self.arp_cycle_for_page(self.arp_ui.edit_page)
    }
    fn arp_cycle_for_page(&self, page: usize) -> ([u8; 96], usize) {
        if self.snapshot.arp_page as usize == page && self.snapshot.arp_len > 0 {
            (self.snapshot.arp_notes, self.snapshot.arp_len as usize)
        } else {
            crate::engine::arpeggiator::cycle_notes(
                self.snapshot.notes,
                self.params.arp_pages[page].settings(),
                0x4e6f7465,
            )
        }
    }
    pub(in crate::ui) fn arp_editor_button_rects(&self) -> Vec<(Rect, bool)> {
        if self.arp_ui.delay_position > 0.0 {
            if !self.arp_ui.delay_open || self.arp_ui.delay_position < 1.0 {
                return Vec::new();
            }
            let state = self.params.arpeggiator.snapshot();
            let delay = &state.pages[self.arp_ui.edit_page].delay;
            let mut v = vec![
                (delay_close_rect(), true),
                (delay_power_rect(), delay.enabled),
                (advanced_rect(), self.arp_ui.advanced),
                (delay_sync_rect(), delay.sync),
                (delay_time_rect(), false),
                (delay_gain_rect(), false),
                (delay_feedback_rect(), false),
                (
                    tap_lane_rect(),
                    self.arp_ui.dropdown == Some(Dropdown::TapLane),
                ),
            ];
            if self.arp_ui.advanced {
                v.extend([
                    (ping_rect(), delay.ping_pong),
                    (add_rect(), false),
                    (remove_rect(), false),
                ]);
                v.extend((0..5).map(|i| (source_rect(i), delay.sources & (1 << i) != 0)));
            }
            return v;
        }
        let mut v = vec![
            (power_rect(), self.params.arp_enabled.value()),
            (mode_rect(), self.arp_ui.dropdown == Some(Dropdown::Mode)),
            (order_rect(), self.arp_ui.chooser == Some(Chooser::Order)),
            (octaves_rect(), false),
            (sync_rect(), self.arp_params().arp_sync.value()),
            (delay_rect(), false),
            (lane_rect(), self.arp_ui.chooser == Some(Chooser::Lane)),
        ];
        v.extend((0..8).map(|p| (page_rect(p), self.arp_ui.edit_page == p)));
        if self.arp_ui.chooser.is_some() {
            let selected = match self.arp_ui.chooser_draw {
                Chooser::Lane => LANES.iter().position(|l| *l == self.arp_ui.lane).unwrap_or(0),
                Chooser::Order => self.arp_params().arp_pattern.value() as usize,
            };
            v.extend((0..7).map(|i| (chooser_card(i), selected == i)));
            return v;
        }
        let state = self.params.arpeggiator.snapshot();
        let (_, len) = self.arp_cycle_for_editor();
        let layout = self.arp_string_layout(self.arp_ui.edit_page, len);
        v.extend((0..len).map(|i| {
            (
                layout.status(i),
                state.pages[self.arp_ui.edit_page].steps[i].state != StepState::Play,
            )
        }));
        v
    }
    pub(in crate::ui) fn arp_editor_route_targets(&self) -> Vec<(usize, Rect)> {
        if self.arp_ui.delay_position > 0.0 {
            return Vec::new();
        }
        [
            ("arp_page", page_block()),
            ("arp_sync", sync_rect()),
            (
                if self.arp_params().arp_sync.value() {
                    "rate"
                } else {
                    "arp_free_ms"
                },
                rate_rect(),
            ),
            ("humanize", humanize_rect()),
            ("swing", swing_rect()),
            ("arp_octaves", octaves_rect()),
        ]
        .into_iter()
        .filter_map(|(id, r)| {
            crate::engine::routing::TARGETS
                .iter()
                .position(|t| t.available() && t.id == id)
                .map(|i| (i, r))
        })
        .collect()
    }
    pub(in crate::ui) fn arp_editor_hint(&self, x: f32, y: f32) -> Option<String> {
        if !hit(ARP_SURFACE, x, y) {
            return None;
        }
        if self.arp_ui.delay_position > 0.0 {
            if hit(delay_close_rect(), x, y) {
                return Some("Return to the arpeggiator · Delay and its tails keep playing".into());
            }
            if hit(delay_power_rect(), x, y) {
                return Some("MIDI Delay power · Independent of arpeggiator power".into());
            }
            if hit(delay_time_rect(), x, y) {
                return Some(
                    "Selected tap interval · Drag vertically or scroll · Cmd/Ctrl-click resets"
                        .into(),
                );
            }
            if hit(delay_gain_rect(), x, y) || hit(delay_feedback_rect(), x, y) {
                return Some(
                    "Selected tap · Drag vertically or scroll · Cmd/Ctrl-click resets".into(),
                );
            }
            if hit(tap_graph(self.arp_ui.advanced), x, y) {
                return Some("Drag a numbered top handle sideways to move its tap · Drag a string vertically, then across to paint · Shift makes a straight ramp · Cmd/Ctrl resets".into());
            }
            return Some("Each tap has an independent echo train · Select a tap to edit its time, velocity and feedback · Advanced adds taps, routing and sources".into());
        }
        if self.arp_ui.chooser.is_some() && hit(chooser_area(), x, y) {
            return Some("Choose a lane or ordering · Left/Right to browse · Enter to choose · Escape to return".into());
        }
        if hit(mode_rect(), x, y) {
            return Some("Once: one cycle · Loop: restart on chord change · Sync Loop: preserve rhythm · Aligned Loop: follow host beat zero (falls back to Sync Loop without host beats or a synced rate)".into());
        }
        if hit(octaves_rect(), x, y) {
            return Some("Arp octaves · Click or scroll 1–4 · Independent of manual strumming · Cmd/Ctrl-click resets".into());
        }
        if hit(page_block(), x, y) {
            return Some("Eight complete arp and Delay variations · The selected Page starts at the next rhythmic boundary".into());
        }
        if hit(lane_rect(), x, y) {
            return Some("Choose the adjustment shown on every string · Mod 1 and Mod 2 are bipolar additive modulation sources; Arp Position is unipolar".into());
        }
        let (_, len) = self.arp_cycle_for_editor();
        let layout = self.arp_string_layout(self.arp_ui.edit_page, len);
        if (0..len).any(|i| hit(layout.status(i), x, y)) {
            return Some("Click: Play → Mute → Skip · Mute consumes time; Skip removes this occurrence · Cmd/Ctrl-click restores Play".into());
        }
        if hit(graph_rect(), x, y) {
            return Some(if self.arp_ui.lane==Lane::Timing {"Drag a string sideways ±49% of its interval · The dotted line marks its original position · Cmd/Ctrl-click resets"}else{"Drag vertically to adjust · Drag across strings to paint (fast moves fill the gaps) · Shift draws a straight ramp · Cmd/Ctrl-click or drag resets"}.into());
        }
        None
    }
    pub(in crate::ui) fn arp_editor_open(&self) -> bool {
        true
    }
    pub(in crate::ui) fn arp_editor_value_at(&self, x: f32, y: f32) -> bool {
        hit(
            if self.arp_ui.delay_open {
                tap_graph(self.arp_ui.advanced)
            } else {
                graph_rect()
            },
            x,
            y,
        )
    }
    pub(in crate::ui) fn arp_editor_dragging(&self) -> bool {
        self.arp_ui.drag.is_some()
    }
    pub(in crate::ui) fn arp_editor_editing(&self) -> bool {
        false
    }
    pub(in crate::ui) fn tick_arp_editor(&mut self, dt: f32) -> bool {
        let page = self.params.arp_page.value().clamp(0, 7) as usize;
        if page != self.arp_ui.observed_page {
            self.arp_ui.observed_page = page;
            self.set_arp_editor_page(page);
        }
        let u = &mut self.arp_ui;
        let mut changed = false;
        if u.chooser_elapsed < pleasant_ui::page_slide::DURATION {
            u.chooser_elapsed = (u.chooser_elapsed + dt).min(pleasant_ui::page_slide::DURATION);
            u.chooser_position = pleasant_ui::page_slide::position(
                u.chooser_start, if u.chooser.is_some() { 1.0 } else { 0.0 }, u.chooser_elapsed,
            );
            changed = true;
        }
        if u.page_elapsed < pleasant_ui::page_slide::DURATION {
            u.page_elapsed = (u.page_elapsed + dt).min(pleasant_ui::page_slide::DURATION);
            changed = true;
        }
        if u.delay_elapsed < pleasant_ui::page_slide::DURATION {
            u.delay_elapsed = (u.delay_elapsed + dt).min(pleasant_ui::page_slide::DURATION);
            u.delay_position = pleasant_ui::page_slide::position(
                u.delay_start, if u.delay_open { 1.0 } else { 0.0 }, u.delay_elapsed,
            );
            changed = true;
        }
        changed
    }
    fn set_arp_editor_page(&mut self, page: usize) {
        if page != self.arp_ui.edit_page {
            self.arp_ui.page_from = self.arp_ui.edit_page;
            self.arp_ui.edit_page = page;
            self.arp_ui.page_elapsed = 0.0;
            self.arp_ui.selected_tap = 0;
            self.arp_ui.drag = None;
        }
    }
    fn set_arp_chooser(&mut self, chooser: Option<Chooser>) {
        if chooser.is_some() {
            self.release_keys();
            self.memory_held.fill(false);
        }
        let u = &mut self.arp_ui;
        if let Some(which) = chooser {
            if u.chooser_draw != which {
                u.chooser_position = 0.0;
            }
            u.chooser_draw = which;
            u.chooser_cursor = match which {
                Chooser::Lane => LANES.iter().position(|l| *l == u.lane).unwrap_or(0),
                Chooser::Order => self.params.arp_pages[u.edit_page].arp_pattern.value() as usize,
            };
        }
        u.chooser_start = u.chooser_position;
        u.chooser_elapsed = 0.0;
        u.chooser = chooser;
        u.dropdown = None;
        u.drag = None;
    }
    fn choose_arp_option(&mut self, cx: &mut EventContext, index: usize) {
        match self.arp_ui.chooser {
            Some(Chooser::Lane) => self.arp_ui.lane = LANES[index],
            Some(Chooser::Order) => {
                let p = &self.arp_params().arp_pattern;
                Self::emit(cx, p.as_ptr(), p.preview_normalized(index as i32));
            }
            None => return,
        }
        self.set_arp_chooser(None);
        cx.needs_redraw();
    }
    pub(in crate::ui) fn cancel_arp_editor(&mut self) {
        self.arp_ui.drag = None;
        self.arp_ui.dropdown = None;
    }
    fn set_delay_editor(&mut self, open: bool) {
        self.set_arp_chooser(None);
        let u = &mut self.arp_ui;
        u.delay_open = open;
        u.delay_start = u.delay_position;
        u.delay_elapsed = 0.0;
        u.drag = None;
        u.dropdown = None;
    }
    fn start_arp_drag(
        &mut self,
        cx: &mut EventContext,
        kind: DragKind,
        index: usize,
        value: f32,
        x: f32,
        y: f32,
    ) {
        self.restore_shift_latch(cx);
        self.arp_ui.drag = Some(Drag {
            kind,
            page: self.arp_ui.edit_page,
            origin: index,
            last: index,
            x,
            y,
            value,
            last_value: value,
            reset: cx.modifiers().intersects(Modifiers::CTRL | Modifiers::LOGO),
        });
        cx.capture();
    }
    fn apply_arp_drag(&mut self, cx: &mut EventContext, x: f32, y: f32) {
        let Some(mut drag) = self.arp_ui.drag else {
            return;
        };
        let ramp = cx.modifiers().shift();
        let reset = drag.reset;
        match drag.kind {
            DragKind::Step(lane) => {
                let (_, len) = self.arp_cycle_for_editor();
                if len == 0 {
                    return;
                }
                let next = self.arp_string_layout(self.arp_ui.edit_page, len).at(x);
                let (lo, hi) = lane.range();
                let v = if reset {
                    lane.get(&Step::default())
                } else {
                    (drag.value + (drag.y - y) * (hi - lo) / graph_rect().3).clamp(lo, hi)
                };
                let (from, start) = if ramp {
                    (drag.origin, drag.value)
                } else {
                    (drag.last, drag.last_value)
                };
                self.params.arpeggiator.edit(|s| {
                    paint_between(from, next, if reset { v } else { start }, v, |i, value| {
                        lane.set(&mut s.pages[drag.page].steps[i], value)
                    })
                });
                drag.last = next;
                drag.last_value = v;
            }
            DragKind::Timing => {
                let (_, len) = self.arp_cycle_for_editor();
                let v = if reset {
                    0.0
                } else {
                    drag.value + (x - drag.x) / self.arp_string_layout(drag.page, len).interval[drag.origin] * 100.0
                };
                self.params
                    .arpeggiator
                    .edit(|s| Lane::Timing.set(&mut s.pages[drag.page].steps[drag.origin], v));
            }
            DragKind::Tap(lane) => {
                let state = self.params.arpeggiator.snapshot();
                let delay = &state.pages[drag.page].delay;
                let domain = time_domain(delay);
                let g = tap_graph(self.arp_ui.advanced);
                let next = (0..delay.count as usize)
                    .min_by(|a, b| {
                        (tap_x(&delay.taps[*a], domain, self.arp_ui.advanced) - x)
                            .abs()
                            .total_cmp(
                                &(tap_x(&delay.taps[*b], domain, self.arp_ui.advanced) - x).abs(),
                            )
                    })
                    .unwrap_or(0);
                let (lo, hi) = lane.range();
                let v = if reset {
                    lane.get(&DelayTap::default())
                } else {
                    (drag.value + (drag.y - y) * (hi - lo) / g.3).clamp(lo, hi)
                };
                // Paint by spatial position; tap order remains stable even after taps cross.
                let from = if ramp { drag.origin } else { drag.last };
                let start = if reset {
                    v
                } else if ramp {
                    drag.value
                } else {
                    drag.last_value
                };
                let a = tap_x(&delay.taps[from], domain, self.arp_ui.advanced);
                let b = tap_x(&delay.taps[next], domain, self.arp_ui.advanced);
                self.params.arpeggiator.edit(|s| {
                    for i in 0..delay.count as usize {
                        let xx = tap_x(&delay.taps[i], domain, self.arp_ui.advanced);
                        if xx >= a.min(b) - 0.01 && xx <= a.max(b) + 0.01 {
                            let f = if (b - a).abs() < 0.01 {
                                1.0
                            } else {
                                (xx - a) / (b - a)
                            };
                            lane.set(
                                &mut s.pages[drag.page].delay.taps[i],
                                start + (v - start) * f,
                            );
                        }
                    }
                });
                drag.last = next;
                drag.last_value = v;
                self.arp_ui.selected_tap = next;
            }
            DragKind::TapTime => {
                let sync = self.params.arpeggiator.snapshot().pages[drag.page]
                    .delay
                    .sync;
                let (lo, hi) = time_range(sync);
                let value = if reset {
                    if sync {
                        0.5
                    } else {
                        250.0
                    }
                } else {
                    drag.value + (x - drag.x) / tap_graph(self.arp_ui.advanced).2 * drag.last_value
                };
                self.params.arpeggiator.edit(|s| {
                    s.pages[drag.page].delay.taps[drag.origin].time = value.clamp(lo, hi)
                });
            }
            DragKind::BasicTime | DragKind::BasicGain | DragKind::BasicFeedback => {
                let sync = self.params.arpeggiator.snapshot().pages[drag.page]
                    .delay
                    .sync;
                self.params.arpeggiator.edit(|s| {
                    let t = &mut s.pages[drag.page].delay.taps[drag.origin];
                    match drag.kind {
                        DragKind::BasicTime => {
                            let (lo, hi) = time_range(sync);
                            let value = if reset {
                                if sync {
                                    0.5
                                } else {
                                    250.0
                                }
                            } else {
                                drag.value * 2.0_f32.powf((drag.y - y) / 70.0)
                            };
                            t.time = value.clamp(lo, hi)
                        }
                        DragKind::BasicGain => {
                            t.velocity = if reset {
                                0.75
                            } else {
                                (drag.value + (drag.y - y) / 100.0).clamp(0.0, 2.0)
                            }
                        }
                        DragKind::BasicFeedback => {
                            t.feedback = if reset {
                                0.35
                            } else {
                                (drag.value + (drag.y - y) / 160.0).clamp(0.0, 0.95)
                            }
                        }
                        _ => {}
                    }
                });
            }
        }
        self.arp_ui.drag = Some(drag);
        cx.needs_redraw();
    }

    pub(in crate::ui) fn arp_editor_event(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
    ) -> bool {
        if matches!(event, WindowEvent::FocusOut) {
            self.cancel_arp_editor();
            cx.release();
            return false;
        }
        if self.arp_ui.chooser.is_some() {
            match event {
                WindowEvent::KeyDown(code, _) => {
                    match code {
                        Code::ArrowLeft | Code::ArrowUp => self.arp_ui.chooser_cursor = (self.arp_ui.chooser_cursor + 6) % 7,
                        Code::ArrowRight | Code::ArrowDown => self.arp_ui.chooser_cursor = (self.arp_ui.chooser_cursor + 1) % 7,
                        Code::Enter | Code::NumpadEnter => self.choose_arp_option(cx, self.arp_ui.chooser_cursor),
                        Code::Escape => self.set_arp_chooser(None),
                        _ => {},
                    }
                    cx.needs_redraw();
                    return true;
                }
                WindowEvent::KeyUp(_, _) | WindowEvent::CharInput(_) => return true,
                _ => {},
            }
        }
        if self.arp_ui.drag.is_some() {
            match event {
                WindowEvent::MouseMove(_, _) => self.apply_arp_drag(cx, x, y),
                WindowEvent::MouseUp(MouseButton::Left) | WindowEvent::KeyDown(Code::Escape, _) => {
                    self.arp_ui.drag = None;
                    cx.release();
                    cx.needs_redraw();
                }
                _ => {}
            }
            return true;
        }
        if matches!(event, WindowEvent::KeyDown(Code::Escape, _))
            && (self.arp_ui.dropdown.is_some() || self.arp_ui.delay_open)
        {
            if self.arp_ui.dropdown.take().is_none() {
                self.set_delay_editor(false)
            }
            cx.needs_redraw();
            return true;
        }
        let press = matches!(
            event,
            WindowEvent::MouseDown(MouseButton::Left)
                | WindowEvent::MouseDoubleClick(MouseButton::Left)
                | WindowEvent::MouseTripleClick(MouseButton::Left)
        );
        if let Some(which) = self.arp_ui.dropdown {
            if press {
                let count = match which {
                    Dropdown::Mode => 4,
                    Dropdown::TapLane => 6,
                };
                if let Some(i) = (0..count).find(|&i| hit(dropdown_rect(which, i), x, y)) {
                    match which {
                        Dropdown::Mode => {
                            let p = &self.arp_params().arp_mode;
                            Self::emit(cx, p.as_ptr(), p.preview_normalized(i as i32));
                        }
                        Dropdown::TapLane => self.arp_ui.tap_lane = TAP_LANES[i],
                    }
                }
                self.arp_ui.dropdown = None;
                cx.needs_redraw();
                return true;
            }
            return hit(ARP_SURFACE, x, y)
                && matches!(
                    event,
                    WindowEvent::MouseScroll(_, _)
                        | WindowEvent::MouseDown(_)
                        | WindowEvent::MouseUp(_)
                );
        }
        if !hit(ARP_SURFACE, x, y) {
            return false;
        }
        if self.arp_ui.delay_elapsed < pleasant_ui::page_slide::DURATION {
            return matches!(
                event,
                WindowEvent::MouseDown(_)
                    | WindowEvent::MouseUp(_)
                    | WindowEvent::MouseScroll(_, _)
            );
        }
        if self.arp_ui.delay_open {
            return self.delay_editor_event(cx, event, x, y, press);
        }
        if (self.arp_ui.chooser_position > 0.0 || self.arp_ui.chooser.is_some())
            && hit(chooser_area(), x, y)
        {
                if press && self.arp_ui.chooser.is_some()
                    && self.arp_ui.chooser_elapsed >= pleasant_ui::page_slide::DURATION
                {
                    if let Some(index) = (0..7).find(|&i| hit(chooser_card(i), x, y)) {
                        self.choose_arp_option(cx, index);
                    }
                }
            return true;
        }
        if self.arp_ui.page_elapsed < pleasant_ui::page_slide::DURATION && hit(chooser_area(), x, y) {
            return true;
        }
        let reset = cx.modifiers().intersects(Modifiers::CTRL | Modifiers::LOGO);
        if let WindowEvent::MouseScroll(_, dy) = event {
            if *dy == 0.0 {
                return true;
            }
            if hit(octaves_rect(), x, y) {
                let p = &self.arp_params().arp_octaves;
                Self::emit(
                    cx,
                    p.as_ptr(),
                    p.preview_normalized((p.value() + dy.signum() as i32).clamp(1, 4)),
                );
                cx.needs_redraw();
                return true;
            }
            if hit(graph_rect(), x, y) {
                let (_, len) = self.arp_cycle_for_editor();
                if len > 0 {
                    let i = self.arp_string_layout(self.arp_ui.edit_page, len).at(x);
                    let page = self.arp_ui.edit_page;
                    let lane = self.arp_ui.lane;
                    self.params.arpeggiator.edit(|s| {
                        let t = &mut s.pages[page].steps[i];
                        lane.set(
                            t,
                            if reset {
                                lane.get(&Step::default())
                            } else {
                                lane.get(t) + dy.signum()
                            },
                        )
                    });
                    cx.needs_redraw()
                }
                return true;
            }
            return false;
        }
        if !press {
            return false;
        }
        cx.focus();
        self.focused = true;
        if hit(power_rect(), x, y) {
            let p = &self.params.arp_enabled;
            Self::emit(cx, p.as_ptr(), p.preview_normalized(!p.value()));
        } else if hit(delay_rect(), x, y) {
            self.set_delay_editor(true);
        } else if hit(mode_rect(), x, y) {
            if reset {
                let p = &self.arp_params().arp_mode;
                Self::emit(cx, p.as_ptr(), p.preview_normalized(2));
            } else {
                self.arp_ui.dropdown = Some(Dropdown::Mode)
            }
        } else if hit(order_rect(), x, y) {
            if reset {
                let p = &self.arp_params().arp_pattern;
                Self::emit(cx, p.as_ptr(), p.preview_normalized(0));
            } else {
                if self.explain_modulation("arp_pattern") { return true; }
                self.set_arp_chooser(if self.arp_ui.chooser == Some(Chooser::Order) { None } else { Some(Chooser::Order) });
            }
        } else if hit(octaves_rect(), x, y) {
            let p = &self.arp_params().arp_octaves;
            Self::emit(
                cx,
                p.as_ptr(),
                p.preview_normalized(if reset { 1 } else { p.value() % 4 + 1 }),
            );
        } else if hit(sync_rect(), x, y) {
            let p = &self.arp_params().arp_sync;
            Self::emit(
                cx,
                p.as_ptr(),
                p.preview_normalized(if reset { true } else { !p.value() }),
            );
        } else if hit(rate_rect(), x, y) && self.arp_params().arp_sync.value() {
            if self.explain_modulation("rate") {
                return true;
            }
            if reset {
                let p = &self.arp_params().rate;
                Self::emit(cx, p.as_ptr(), p.preview_normalized(0.25));
            } else {
                self.open_menu(Menu::ArpRate)
            }
        } else if let Some(page) = (0..8).find(|&p| hit(page_rect(p), x, y)) {
            self.set_arp_editor_page(page);
            let p = &self.params.arp_page;
            Self::emit(cx, p.as_ptr(), p.preview_normalized(page as i32));
        } else if hit(lane_rect(), x, y) {
            self.set_arp_chooser(if self.arp_ui.chooser == Some(Chooser::Lane) { None } else { Some(Chooser::Lane) });
        } else {
            let (_, len) = self.arp_cycle_for_editor();
            if len == 0 {
                return false;
            }
            let page = self.arp_ui.edit_page;
            let layout = self.arp_string_layout(page, len);
            if let Some(i) = (0..len).find(|&i| hit(layout.status(i), x, y)) {
                self.params.arpeggiator.edit(|s| {
                    let t = &mut s.pages[page].steps[i];
                    t.state = if reset {
                        StepState::Play
                    } else {
                        t.state.next()
                    };
                });
            } else if hit(graph_rect(), x, y) {
                let state = self.params.arpeggiator.snapshot();
                let lane = self.arp_ui.lane;
                let i = if lane == Lane::Timing {
                    (0..len)
                        .min_by(|a, b| {
                            let xx = |i: usize| {
                                layout.x[i]
                                    + state.pages[page].steps[i].micro as f32 * layout.interval[i]
                                        / 100.0
                            };
                            (xx(*a) - x).abs().total_cmp(&(xx(*b) - x).abs())
                        })
                        .unwrap_or(0)
                } else {
                    layout.at(x)
                };
                let value = lane.get(&state.pages[page].steps[i]);
                self.start_arp_drag(
                    cx,
                    if lane == Lane::Timing {
                        DragKind::Timing
                    } else {
                        DragKind::Step(lane)
                    },
                    i,
                    value,
                    x,
                    y,
                );
                if reset {
                    self.apply_arp_drag(cx, x, y)
                }
            } else {
                return false;
            }
        }
        cx.needs_redraw();
        true
    }

    fn delay_editor_event(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
        press: bool,
    ) -> bool {
        let page = self.arp_ui.edit_page;
        let state = self.params.arpeggiator.snapshot();
        let delay = &state.pages[page].delay;
        let selected = self.arp_ui.selected_tap.min(delay.count as usize - 1);
        let tap = delay.taps[selected];
        let reset = cx.modifiers().intersects(Modifiers::CTRL | Modifiers::LOGO);
        if let WindowEvent::MouseScroll(_, dy) = event {
            if *dy == 0.0 {
                return true;
            }
            self.params.arpeggiator.edit(|s| {
                let t = &mut s.pages[page].delay.taps[selected];
                if hit(delay_time_rect(), x, y) {
                    if delay.sync {
                        let divisions = [
                            1.0 / 64.0,
                            1.0 / 32.0,
                            1.0 / 16.0,
                            1.0 / 8.0,
                            1.0 / 6.0,
                            1.0 / 4.0,
                            1.0 / 3.0,
                            0.5,
                            0.75,
                            1.0,
                            1.5,
                            2.0,
                            3.0,
                            4.0,
                            8.0,
                            16.0,
                        ];
                        t.time = if *dy > 0.0 {
                            divisions
                                .iter()
                                .copied()
                                .find(|v| *v > t.time + 0.0001)
                                .unwrap_or(16.0)
                        } else {
                            divisions
                                .iter()
                                .rev()
                                .copied()
                                .find(|v| *v < t.time - 0.0001)
                                .unwrap_or(1.0 / 64.0)
                        };
                    } else {
                        t.time = (t.time + dy.signum() * 10.0).clamp(1.0, 8000.0)
                    }
                } else if hit(delay_gain_rect(), x, y) {
                    t.velocity = (t.velocity + dy.signum() * 0.01).clamp(0.0, 2.0)
                } else if hit(delay_feedback_rect(), x, y) {
                    t.feedback = (t.feedback + dy.signum() * 0.01).clamp(0.0, 0.95)
                }
            });
            cx.needs_redraw();
            return true;
        }
        if !press {
            return matches!(event, WindowEvent::MouseDown(_) | WindowEvent::MouseUp(_));
        }
        cx.focus();
        self.focused = true;
        if hit(delay_close_rect(), x, y) {
            self.set_delay_editor(false)
        } else if hit(delay_power_rect(), x, y) {
            self.params
                .arpeggiator
                .edit(|s| s.pages[page].delay.enabled = !delay.enabled)
        } else if hit(advanced_rect(), x, y) {
            self.arp_ui.advanced = !self.arp_ui.advanced;
        } else if hit(delay_sync_rect(), x, y) {
            let beat_ms = 60_000.0 / self.snapshot.tempo.max(1.0);
            self.params.arpeggiator.edit(|s| {
                let d = &mut s.pages[page].delay;
                d.sync = !delay.sync;
                for t in &mut d.taps {
                    t.time = if d.sync {
                        t.time / beat_ms
                    } else {
                        t.time * beat_ms
                    }
                }
            });
        } else if hit(tap_lane_rect(), x, y) {
            self.arp_ui.dropdown = Some(Dropdown::TapLane)
        } else if hit(delay_time_rect(), x, y) {
            self.start_arp_drag(cx, DragKind::BasicTime, selected, tap.time, x, y);
            if reset {
                self.apply_arp_drag(cx, x, y)
            }
        } else if hit(delay_gain_rect(), x, y) {
            self.start_arp_drag(cx, DragKind::BasicGain, selected, tap.velocity, x, y);
            if reset {
                self.apply_arp_drag(cx, x, y)
            }
        } else if hit(delay_feedback_rect(), x, y) {
            self.start_arp_drag(cx, DragKind::BasicFeedback, selected, tap.feedback, x, y);
            if reset {
                self.apply_arp_drag(cx, x, y)
            }
        } else if self.arp_ui.advanced && hit(ping_rect(), x, y) {
            self.params
                .arpeggiator
                .edit(|s| s.pages[page].delay.ping_pong = !delay.ping_pong)
        } else if self.arp_ui.advanced && hit(add_rect(), x, y) {
            if (delay.count as usize) < TAPS {
                let index = delay.count as usize;
                let t = DelayTap {
                    time: (delay.taps[index - 1].time + if delay.sync { 0.5 } else { 250.0 })
                        .min(time_range(delay.sync).1),
                    ..Default::default()
                };
                self.params.arpeggiator.edit(|s| {
                    let d = &mut s.pages[page].delay;
                    d.taps[index] = t;
                    d.count += 1;
                });
                self.arp_ui.selected_tap = index
            }
        } else if self.arp_ui.advanced && hit(remove_rect(), x, y) {
            if delay.count > 1 {
                self.params.arpeggiator.edit(|s| {
                    let d = &mut s.pages[page].delay;
                    for i in selected..d.count as usize - 1 {
                        d.taps[i] = d.taps[i + 1]
                    }
                    d.count -= 1;
                });
                self.arp_ui.selected_tap = selected.min(delay.count as usize - 2)
            }
        } else if self.arp_ui.advanced && (0..5).any(|i| hit(source_rect(i), x, y)) {
            let i = (0..5).find(|&i| hit(source_rect(i), x, y)).unwrap_or(0);
            self.params
                .arpeggiator
                .edit(|s| s.pages[page].delay.sources ^= 1 << i)
        } else {
            let g = tap_graph(self.arp_ui.advanced);
            let domain = time_domain(delay);
            // Numbered caps are separate horizontal handles on every adjustment lane.
            let nearest = (0..delay.count as usize)
                .min_by(|a, b| {
                    (tap_x(&delay.taps[*a], domain, self.arp_ui.advanced) - x)
                        .abs()
                        .total_cmp(
                            &(tap_x(&delay.taps[*b], domain, self.arp_ui.advanced) - x).abs(),
                        )
                })
                .unwrap_or(0);
            let tx = tap_x(&delay.taps[nearest], domain, self.arp_ui.advanced);
            if (x - tx).abs() < 14.0 && y >= g.1 - 17.0 && y <= g.1 + 7.0 {
                self.arp_ui.selected_tap = nearest;
                self.start_arp_drag(
                    cx,
                    DragKind::TapTime,
                    nearest,
                    delay.taps[nearest].time,
                    x,
                    y,
                );
                if let Some(d) = &mut self.arp_ui.drag {
                    d.last_value = domain;
                }
                if reset {
                    self.apply_arp_drag(cx, x, y)
                }
            } else if hit(g, x, y) {
                self.arp_ui.selected_tap = nearest;
                let lane = self.arp_ui.tap_lane;
                self.start_arp_drag(
                    cx,
                    DragKind::Tap(lane),
                    nearest,
                    lane.get(&delay.taps[nearest]),
                    x,
                    y,
                );
                if reset {
                    self.apply_arp_drag(cx, x, y)
                }
            }
        }
        cx.needs_redraw();
        true
    }

    pub(in crate::ui) fn draw_arp_editor(&self, d: &mut Draw) {
        let s = ARP_SURFACE;
        d.scissor(s.0, s.1, s.2, s.3);
        let saved_offset = d.offset_x;
        self.draw_arp_strings(d);
        if self.arp_ui.chooser_position > 0.0 {
            let r = chooser_area();
            d.scissor(r.0, r.1, r.2, r.3);
            d.offset_x = saved_offset + (1.0 - self.arp_ui.chooser_position) * r.2;
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            self.draw_arp_chooser(d);
            d.offset_x = saved_offset;
            d.scissor(s.0, s.1, s.2, s.3);
        }
        if self.arp_ui.delay_position > 0.0 {
            d.offset_x = saved_offset + (1.0 - self.arp_ui.delay_position) * s.2;
            d.rect(s.0, s.1, s.2, s.3, PANEL);
            d.line(s.0, s.1, s.0, s.1 + s.3, TEAL, 2.0);
            self.draw_delay_strings(d);
        }
        d.offset_x = saved_offset;
        if let Some(which) = self.arp_ui.dropdown {
            self.draw_arp_dropdown(d, which)
        }
        d.reset_scissor();
    }
    fn draw_arp_strings(&self, d: &mut Draw) {
        let s = ARP_SURFACE;
        let state = self.params.arpeggiator.snapshot();
        let page = &state.pages[self.arp_ui.edit_page];
        let p = self.arp_params();
        let power = power_rect();
        d.bypass_button(
            power,
            !self.params.arp_enabled.value(),
            TEAL,
            self.hover_amount(power) > 0.01,
            0.0,
        );
        d.text(
            power.0 + power.2 + 9.0,
            s.1 + 29.0,
            "ARPEGGIATOR",
            16.0,
            if self.params.arp_enabled.value() {
                TEXT
            } else {
                MUTED
            },
        );
        self.icon_button(
            d,
            mode_rect(),
            &format!("{} ▾", MODE_NAMES[p.arp_mode.value().clamp(0, 3) as usize]),
            self.arp_ui.dropdown == Some(Dropdown::Mode),
            GOLD,
            repeat_icon(p.arp_mode.value().clamp(0, 3) as usize),
        );
        self.button(
            d,
            order_rect(),
            &format!(
                "{} ▾",
                ARP_PATTERN_NAMES[p.arp_pattern.value().clamp(0, 6) as usize]
            ),
            self.arp_ui.chooser == Some(Chooser::Order),
            GOLD,
        );
        self.button(
            d,
            octaves_rect(),
            &format!("Octaves: {}", p.arp_octaves.value()),
            false,
            GOLD,
        );
        self.button(
            d,
            sync_rect(),
            if p.arp_sync.value() {
                "Tempo sync"
            } else {
                "Free · ms"
            },
            p.arp_sync.value(),
            TEAL,
        );
        if p.arp_sync.value() {
            self.button(
                d,
                rate_rect(),
                &format!("Rate: {} ▾", time_label(p.rate.value(), true)),
                self.menu == Some(Menu::ArpRate),
                GOLD,
            )
        }
        for (c, r) in self.base_controls() {
            if matches!(c.id, "humanize" | "swing")
                || (!p.arp_sync.value() && c.id == "arp_free_ms")
            {
                self.draw_control(d, &c, r, GOLD)
            }
        }
        self.button(d, delay_rect(), "Delay ›", page.delay.enabled, TEAL);
        d.line(
            s.0 + 12.0,
            s.1 + 43.0,
            s.0 + s.2 - 12.0,
            s.1 + 43.0,
            LINE,
            1.0,
        );
        d.text(s.0 + 15.0, s.1 + 66.0, "Page", 11.0, MUTED);
        for i in 0..8 {
            let r = page_rect(i);
            self.button(d, r, &(i + 1).to_string(), self.arp_ui.edit_page == i, TEAL);
            if self.snapshot.arp_page as usize == i && self.snapshot.arp_running {
                d.line(
                    r.0 + 4.0,
                    r.1 + r.3 - 2.0,
                    r.0 + r.2 - 4.0,
                    r.1 + r.3 - 2.0,
                    GOLD,
                    2.0,
                )
            }
            if self.snapshot.arp_pending as usize == i {
                d.circle(r.0 + r.2 - 3.0, r.1 + 3.0, 2.5, GOLD, true)
            }
        }
        self.button(
            d,
            lane_rect(),
            &format!("{} ▾", self.arp_ui.lane.label()),
            self.arp_ui.chooser == Some(Chooser::Lane),
            GOLD,
        );
        let status = if p.arp_mode.value() == 3 && !self.snapshot.arp_aligned {
            "Aligned Loop · using Sync Loop"
        } else if self.snapshot.arp_pending < 8 {
            "Page queued for next boundary"
        } else if matches!(self.arp_ui.lane, Lane::Mod1 | Lane::Mod2) {
            "Bipolar additive modulation · −100% to +100%"
        } else {
            "Drag to adjust · Shift: straight ramp · Cmd/Ctrl: reset"
        };
        d.text(
            lane_rect().0 + lane_rect().2 + 18.0,
            s.1 + 66.0,
            status,
            11.0,
            MUTED,
        );
        self.draw_arp_page_slide(d, chooser_area(), Self::draw_arp_pattern);
    }
    fn draw_arp_page_slide(&self, d: &mut Draw, r: Rect, draw: impl Fn(&Self, &mut Draw, usize)) {
        d.scissor(r.0, r.1, r.2, r.3);
        let offset = d.offset_x;
        let progress = pleasant_ui::page_slide::position(0.0, 1.0, self.arp_ui.page_elapsed);
        let direction = if self.arp_ui.edit_page >= self.arp_ui.page_from { 1.0 } else { -1.0 };
        if progress < 1.0 {
            d.offset_x = offset - direction * progress * r.2;
            draw(self, d, self.arp_ui.page_from);
        }
        d.offset_x = offset + direction * (1.0 - progress) * r.2;
        draw(self, d, self.arp_ui.edit_page);
        d.offset_x = offset;
        d.scissor(ARP_SURFACE.0, ARP_SURFACE.1, ARP_SURFACE.2, ARP_SURFACE.3);
    }
    fn draw_arp_pattern(&self, d: &mut Draw, page_index: usize) {
        let state = self.params.arpeggiator.snapshot();
        let page = &state.pages[page_index];
        let (notes, len) = self.arp_cycle_for_page(page_index);
        let layout = self.arp_string_layout(page_index, len);
        let g = graph_rect();
        if len == 0 {
            d.text_centered(
                g.0 + g.2 * 0.5,
                g.1 + g.3 * 0.5,
                "Select a chord to adjust its arpeggio",
                13.0,
                MUTED,
            );
            return;
        }
        let lane = self.arp_ui.lane;
        let range = lane.range();
        let baseline = value_y(if range.0 < 0.0 { 0.0 } else { range.0 }, range, g);
        d.line(g.0, baseline, g.0 + g.2, baseline, alpha(LINE, 0.6), 1.0);
        for (i, &note) in notes.iter().enumerate().take(len) {
            let step = &page.steps[i];
            let nominal = layout.x[i];
            let x = if lane == Lane::Timing {
                nominal + step.micro as f32 * layout.interval[i] / 100.0
            } else {
                nominal
            };
            let active = self.snapshot.arp_page as usize == page_index
                && self.snapshot.arp_running
                && self.snapshot.arp_step as usize == i;
            let color = if step.state == StepState::Skip {
                MUTED
            } else if active {
                GOLD
            } else {
                TEAL
            };
            let strength = if step.state == StepState::Play {
                1.0
            } else {
                0.38
            };
            if lane == Lane::Timing {
                dotted(d, nominal, g.1, g.3, alpha(MUTED, 0.6));
                d.line(
                    x,
                    g.1,
                    x,
                    g.1 + g.3,
                    alpha(color, strength),
                    if active { 2.5 } else { 1.5 },
                );
                d.circle(x, g.1 + g.3 * 0.5, 3.0, color, true)
            } else {
                let y = value_y(lane.get(step), range, g);
                d.line(x, g.1, x, g.1 + g.3, alpha(color, 0.25 * strength), 1.0);
                d.line(
                    x,
                    baseline,
                    x,
                    y,
                    alpha(color, strength),
                    if active { 3.0 } else { 2.0 },
                );
                d.circle(
                    x,
                    y,
                    if active { 4.0 } else { 3.0 },
                    alpha(color, strength),
                    true,
                )
            }
            // Hover or drag reveals exact values; note labels remain stable while timing moves.
            let hovered = self
                .pointer
                .is_some_and(|(px, py)| hit(g, px, py) && layout.at(px) == i);
            if hovered
                || self.arp_ui.drag.is_some_and(|drag| {
                    matches!(drag.kind, DragKind::Step(_) | DragKind::Timing) && drag.last == i
                })
            {
                d.text_centered(x, g.1 - 6.0, &lane.format(lane.get(step)), 10.0, TEXT)
            }
            let note = (note as i16 + step.octave as i16 * 12).clamp(0, 127) as u8;
            let label = self.note_name(note);
            d.text_centered(
                nominal,
                g.1 + g.3 + 13.0,
                &label,
                if layout.spacing < 23.0 { 8.0 } else { 10.0 },
                if active { GOLD } else { MUTED },
            );
            let r = layout.status(i);
            let mark = match step.state {
                StepState::Play => "•",
                StepState::Mute => "–",
                StepState::Skip => "×",
            };
            self.button(
                d,
                r,
                mark,
                step.state != StepState::Play,
                if step.state == StepState::Skip {
                    MUTED
                } else {
                    TEAL
                },
            );
        }
    }
    fn draw_delay_strings(&self, d: &mut Draw) {
        let s = ARP_SURFACE;
        let state = self.params.arpeggiator.snapshot();
        let delay = &state.pages[self.arp_ui.edit_page].delay;
        let selected = self.arp_ui.selected_tap.min(delay.count as usize - 1);
        let tap = &delay.taps[selected];
        let u = &self.arp_ui;
        let power = delay_power_rect();
        d.bypass_button(
            power,
            !delay.enabled,
            TEAL,
            self.hover_amount(power) > 0.01,
            0.0,
        );
        d.text(
            power.0 + power.2 + 10.0,
            s.1 + 29.0,
            "MIDI DELAY",
            16.0,
            if delay.enabled { TEXT } else { MUTED },
        );
        d.text(
            s.0 + 210.0,
            s.1 + 28.0,
            &format!(
                "Page {} · Tap {} of {}",
                u.edit_page + 1,
                selected + 1,
                delay.count
            ),
            12.0,
            MUTED,
        );
        self.button(
            d,
            advanced_rect(),
            if u.advanced {
                "Advanced ▴"
            } else {
                "Advanced ▾"
            },
            u.advanced,
            GOLD,
        );
        self.button(d, delay_close_rect(), "‹ Arp", true, TEAL);
        d.line(
            s.0 + 12.0,
            s.1 + 43.0,
            s.0 + s.2 - 12.0,
            s.1 + 43.0,
            LINE,
            1.0,
        );
        self.button(
            d,
            delay_sync_rect(),
            if delay.sync {
                "Tempo sync"
            } else {
                "Free · ms"
            },
            delay.sync,
            TEAL,
        );
        self.button(
            d,
            delay_time_rect(),
            &format!("Time: {}", time_label(tap.time, delay.sync)),
            false,
            GOLD,
        );
        self.button(
            d,
            delay_gain_rect(),
            &format!("Velocity: {:.0}%", tap.velocity * 100.0),
            false,
            GOLD,
        );
        self.button(
            d,
            delay_feedback_rect(),
            &format!("Feedback: {:.0}%", tap.feedback * 100.0),
            false,
            GOLD,
        );
        self.button(
            d,
            tap_lane_rect(),
            &format!("{} ▾", u.tap_lane.label()),
            u.dropdown == Some(Dropdown::TapLane),
            GOLD,
        );
        if u.advanced {
            self.button(d, ping_rect(), "Ping pong", delay.ping_pong, TEAL);
            self.button(d, add_rect(), "+", false, TEAL);
            self.button(d, remove_rect(), "−", false, TEAL);
            d.text(
                add_rect().0 - 90.0,
                s.1 + 66.0,
                &format!("{} / 8 taps", delay.count),
                11.0,
                MUTED,
            );
            d.text(s.0 + 18.0, s.1 + 98.0, "Input", 11.0, MUTED);
            for (i, name) in SOURCE_NAMES.iter().enumerate() {
                self.button(d, source_rect(i), name, delay.sources & (1 << i) != 0, TEAL)
            }
            d.text(
                source_rect(4).0 + source_rect(4).2 + 14.0,
                s.1 + 98.0,
                "Independent repeats · Pitch stays fixed · Pan uses CC10",
                10.0,
                MUTED,
            );
        }
        let g = tap_graph(u.advanced);
        let area = (g.0, g.1 - 20.0, g.2, ARP_SURFACE.1 + ARP_SURFACE.3 - g.1 + 20.0);
        self.draw_arp_page_slide(d, area, Self::draw_delay_pattern);
    }
    fn draw_delay_pattern(&self, d: &mut Draw, page_index: usize) {
        let s = ARP_SURFACE;
        let state = self.params.arpeggiator.snapshot();
        let delay = &state.pages[page_index].delay;
        let u = &self.arp_ui;
        let selected = u.selected_tap.min(delay.count as usize - 1);
        let g = tap_graph(u.advanced);
        let domain =
            if let Some(drag) = u.drag.filter(|drag| matches!(drag.kind, DragKind::TapTime)) {
                drag.last_value
            } else {
                time_domain(delay)
            };
        let range = u.tap_lane.range();
        let baseline = value_y(if range.0 < 0.0 { 0.0 } else { range.0 }, range, g);
        for i in 0..=4 {
            let x = g.0 + i as f32 * g.2 / 4.0;
            dotted(d, x, g.1, g.3, alpha(LINE, 0.55));
            let label = time_label(domain * i as f32 / 4.0, delay.sync);
            let y = g.1 + g.3 + 14.0;
            match i {
                0 => d.text(x, y, &label, 9.0, MUTED),
                4 => d.text_right(x, y, &label, 9.0, MUTED),
                _ => d.text_centered(x, y, &label, 9.0, MUTED),
            }
        }
        d.line(g.0, baseline, g.0 + g.2, baseline, LINE, 1.0);
        for i in 0..delay.count as usize {
            let t = &delay.taps[i];
            let x = tap_x(t, domain, u.advanced);
            let y = value_y(u.tap_lane.get(t), range, g);
            let color = if i == selected { GOLD } else { TEAL };
            d.line(x, g.1, x, g.1 + g.3, alpha(color, 0.3), 1.0);
            d.line(
                x,
                baseline,
                x,
                y,
                color,
                if i == selected { 3.0 } else { 2.0 },
            );
            d.circle(x, y, 4.0, color, true);
            d.rounded_rect(x - 12.0, g.1 - 16.0, 24.0, 15.0, 3.0, alpha(color, 0.18));
            d.text_centered(x, g.1 - 5.0, &format!("{} ↔", i + 1), 9.0, color);
            d.text_centered(
                x,
                y - 7.0,
                &u.tap_lane.format(u.tap_lane.get(t)),
                10.0,
                if i == selected { TEXT } else { MUTED },
            );
        }
        d.text(s.0+20.0,s.1+s.3-9.0,"Numbered handles move time · Strings edit the selected lane · Shift: straight ramp · Cmd/Ctrl: reset",10.0,MUTED);
    }
    fn draw_arp_dropdown(&self, d: &mut Draw, which: Dropdown) {
        let count = match which {
            Dropdown::Mode => 4,
            Dropdown::TapLane => 6,
        };
        for i in 0..count {
            let r = dropdown_rect(which, i);
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            let (label, on) = match which {
                Dropdown::Mode => (
                    MODE_NAMES[i],
                    self.arp_params().arp_mode.value() as usize == i,
                ),
                Dropdown::TapLane => (TAP_LANES[i].label(), self.arp_ui.tap_lane == TAP_LANES[i]),
            };
            if which == Dropdown::Mode {
                self.icon_button(d, r, label, on, GOLD, repeat_icon(i));
            } else {
                self.button(d, r, label, on, GOLD);
            }
        }
    }
    fn draw_arp_chooser(&self, d: &mut Draw) {
        use nih_plug_vizia::vizia::vg::{Align, Baseline, Paint};
        let u = &self.arp_ui;
        let selected = match u.chooser_draw {
            Chooser::Lane => LANES.iter().position(|l| *l == u.lane).unwrap_or(0),
            Chooser::Order => self.arp_params().arp_pattern.value() as usize,
        };
        for i in 0..7 {
            let r = chooser_card(i);
            let on = selected == i;
            self.button_surface(d, r, on, GOLD);
            let color = if on { GOLD } else if self.hover_amount(r) > 0.01 { TEXT } else { MUTED };
            let cx = r.0 + r.2 * 0.5;
            let cy = r.1 + r.3 * 0.36;
            let label = match u.chooser_draw {
                Chooser::Lane => {
                    if let Some(font) = d.icon_font {
                        let mut paint = Paint::color(d.color(color));
                        paint.set_font(&[font]);
                        paint.set_font_size(36.0 * d.s);
                        paint.set_text_align(Align::Center);
                        paint.set_text_baseline(Baseline::Middle);
                        let _ = d.c.fill_text(d.ox + (cx + d.offset_x) * d.s, d.oy + cy * d.s, LANES[i].glyph(), &paint);
                    }
                    if i >= 5 {
                        d.text_centered(cx + 23.0, cy - 11.0, &(i - 4).to_string(), 11.0, color);
                    }
                    d.text_centered(cx, r.1 + r.3 * 0.82, LANES[i].description(), 10.0, MUTED);
                    LANES[i].label()
                }
                Chooser::Order => {
                    let w = (r.2 - 36.0).min(76.0);
                    let h = 30.0;
                    let points: Vec<_> = ORDER_CONTOURS[i].iter().enumerate().map(|(j, &n)|
                        (cx - w * 0.5 + j as f32 * w / 4.0, cy + h * 0.5 - n as f32 * h / 4.0)
                    ).collect();
                    d.poly(&points, color, 1.6);
                    for &(x, y) in &points { d.circle(x, y, 2.5, color, true); }
                    ARP_PATTERN_NAMES[i]
                }
            };
            let old_font = d.font;
            d.font = self.bold_font.get();
            d.text_centered(cx, r.1 + r.3 * 0.66, label, 14.0, color);
            d.font = old_font;
            if u.chooser_cursor == i && u.chooser.is_some() {
                d.outline_rounded(r.0 + 2.0, r.1 + 2.0, r.2 - 4.0, r.3 - 4.0, 6.0, alpha(GOLD, 0.5), 1.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chooser_cards_fill_the_string_area_with_separate_large_targets() {
        let area = chooser_area();
        for i in 0..7 {
            let r = chooser_card(i);
            assert!(r.2 > 140.0 && r.3 > 100.0);
            assert_eq!(r.1, area.1);
            assert_eq!(r.3, area.3);
            if i > 0 {
                let previous = chooser_card(i - 1);
                assert!((r.0 - previous.0 - previous.2 - 8.0).abs() < 0.001);
            }
        }
        let last = chooser_card(6);
        assert!((last.0 + last.2 - area.0 - area.2).abs() < 0.001);
    }
    #[test]
    fn swing_positions_and_microtiming_follow_the_effective_clock_inside_the_graph() {
        let mut steps = [Step::default(); 96];
        for len in [2, 3, 4, 95, 96] {
            for swing in [-0.75, 0.0, 0.75] {
                for phase in [0, 1] {
                    let layout = StringLayout::new(&steps, len, swing, phase);
                    for i in 0..len {
                        assert_eq!(layout.at(layout.x[i]), i);
                        for micro in [-0.49, 0.49] {
                            let x = layout.x[i] + micro * layout.interval[i];
                            assert!(x >= graph_rect().0 - 0.001 && x <= graph_rect().0 + graph_rect().2 + 0.001);
                        }
                        if i > 0 {
                            assert!((layout.x[i] - layout.x[i - 1] - layout.interval[i - 1]).abs() < 0.001);
                        }
                    }
                }
            }
        }
        steps[1].state = StepState::Skip;
        let layout = StringLayout::new(&steps, 4, 0.5, 0);
        assert_eq!(layout.interval[1], layout.interval[2]);
        steps[1].state = StepState::Mute;
        let muted = StringLayout::new(&steps, 4, 0.5, 0);
        assert_ne!(muted.interval[1], muted.interval[2]);
    }
    #[test]
    fn chooser_and_page_changes_use_the_shared_slide_and_keep_their_destination() {
        let mut view = ChordboardView::new(Arc::new(ChordboardParams::default()), Arc::new(Bridge::default()), Arc::new(PreviewContext));
        view.set_arp_chooser(Some(Chooser::Lane));
        view.tick_arp_editor(pleasant_ui::page_slide::DURATION * 0.5);
        assert!((view.arp_ui.chooser_position - 0.5).abs() < 0.001);
        view.set_arp_chooser(None);
        view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
        assert_eq!(view.arp_ui.chooser_position, 0.0);
        view.set_arp_editor_page(1);
        view.tick_arp_editor(pleasant_ui::page_slide::DURATION * 0.5);
        assert_eq!(view.arp_ui.page_from, 0);
        assert_eq!(view.arp_ui.edit_page, 1);
        assert_eq!(view.arp_ui.page_elapsed, pleasant_ui::page_slide::DURATION * 0.5);
        view.set_arp_editor_page(2);
        view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
        assert_eq!(view.arp_ui.page_from, 1);
        assert_eq!(view.arp_ui.edit_page, 2);
    }
    #[test]
    fn full_cycles_stay_even_and_inside_the_module() {
        for len in [1, 4, 32, 48, 94, 96] {
            let layout = StringLayout::new(&[Step::default(); 96], len, 0.0, 0);
            for i in 0..len {
                let r = layout.status(i);
                assert!(hit(ARP_SURFACE, r.0, r.1));
                assert!(hit(ARP_SURFACE, r.0 + r.2, r.1 + r.3));
                if i > 0 {
                    assert!((layout.x[i] - layout.x[i - 1] - layout.spacing).abs() < 0.001)
                }
            }
        }
    }
    #[test]
    fn fast_paint_and_reverse_ramp_fill_every_crossed_string() {
        let mut values = [0.0; 8];
        paint_between(1, 6, 20.0, 120.0, |i, v| values[i] = v);
        assert_eq!(values, [0.0, 20.0, 40.0, 60.0, 80.0, 100.0, 120.0, 0.0]);
        paint_between(6, 2, 100.0, 20.0, |i, v| values[i] = v);
        assert_eq!(&values[2..=6], &[20.0, 40.0, 60.0, 80.0, 100.0]);
    }
    #[test]
    fn adjustment_defaults_and_bounds_match_the_engine() {
        let mut s = Step::default();
        assert_eq!(Lane::Velocity.get(&s), 100.0);
        assert_eq!(Lane::Gate.get(&s), 65.0);
        Lane::Timing.set(&mut s, 500.0);
        assert_eq!(s.micro, 49);
        Lane::Mod2.set(&mut s, -200.0);
        assert_eq!(s.mods[1], -100);
        let mut t = DelayTap::default();
        TapLane::Feedback.set(&mut t, 1.0);
        assert_eq!(t.feedback, 0.95);
        TapLane::Channel.set(&mut t, 20.0);
        assert_eq!(t.channel, 16);
    }
    #[test]
    fn dropdowns_and_delay_graph_fit_and_handles_are_separate() {
        for (which, n) in [
            (Dropdown::Mode, 4),
            (Dropdown::TapLane, 6),
        ] {
            for i in 0..n {
                let r = dropdown_rect(which, i);
                assert!(hit(ARP_SURFACE, r.0, r.1));
                assert!(hit(ARP_SURFACE, r.0 + r.2, r.1 + r.3));
            }
        }
        for advanced in [false, true] {
            let g = tap_graph(advanced);
            assert!(g.1 > delay_time_rect().1 + delay_time_rect().3);
            assert!(g.1 + g.3 < ARP_SURFACE.1 + ARP_SURFACE.3 - 27.0)
        }
    }
}
