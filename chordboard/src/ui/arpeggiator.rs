//! String-based arp adjustments and the independently powered MIDI delay.
//! All pointer geometry is shared with drawing; edits publish only from the UI thread.
use super::*;
use crate::arpeggiator::{DelaySettings, DelayTap, Step, StepState, MODE_NAMES, TAPS, STEPS, LOOPS};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Lane {
    #[default]
    Velocity,
    Chance,
    Octave,
    Timing,
    Mod1,
    Mod2,
}
const LANES: [Lane; 6] = [
    Lane::Velocity,
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
            Self::Chance => "\u{eb66}", // dice
            Self::Octave => "\u{eb5b}", // arrows-vertical
            Self::Timing => "\u{ea70}", // clock
            Self::Mod1 | Self::Mod2 => "\u{ecd4}", // wave-sine
        }
    }
    fn description(self) -> &'static str {
        match self {
            Self::Velocity => "Strength + duration",
            Self::Chance => "Play probability",
            Self::Octave => "Pitch in octaves",
            Self::Timing => "Early / late",
            Self::Mod1 => "Bipolar source 1",
            Self::Mod2 => "Bipolar source 2",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Velocity => "Velocity / Gate",
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
            Self::Chance => (0.0, 100.0),
            Self::Octave => (-4.0, 4.0),
            Self::Timing => (-49.0, 49.0),
            Self::Mod1 | Self::Mod2 => (-100.0, 100.0),
        }
    }
    fn get(self, s: &Step) -> f32 {
        match self {
            Self::Velocity => s.velocity as f32,
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
    Pages,
    Paste(usize),
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Chooser {
    Order,
}
#[derive(Clone, Copy)]
enum DragKind {
    Step(Lane),
    Timing,
    Gate,
    Scroll,
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
    pass_len: usize,
}
#[derive(Clone, Copy, Debug)]
struct Viewport { zoom: f32, scroll: f32 }
impl Default for Viewport { fn default() -> Self { Self { zoom: 1.0, scroll: 0.0 } } }
impl Viewport {
    fn set_zoom(&mut self, zoom: f32) {
        let center = (self.scroll + 0.5) / self.zoom;
        self.zoom = zoom.clamp(1.0, 8.0);
        self.scroll = (center * self.zoom - 0.5).clamp(0.0, self.zoom - 1.0);
    }
    fn x(self, x: f32) -> f32 {
        let g = graph_rect();
        g.0 + (x - g.0) * self.zoom - self.scroll * g.2
    }
    fn scrollbar(self) -> Rect {
        let r = scroll_rect();
        (r.0 + self.scroll / self.zoom * r.2, r.1, r.2 / self.zoom, r.3)
    }
}
pub(super) struct ArpUi {
    pub edit_page: usize,
    lane: Lane,
    viewports: [Viewport; 8],
    clipboard: Option<(crate::arpeggiator::Page, crate::arpeggiator::Settings)>,
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
    pending_tap_remove: Option<usize>,
}
impl Default for ArpUi {
    fn default() -> Self {
        Self {
            edit_page: 0,
            lane: Lane::Velocity,
            viewports: [Viewport::default(); 8],
            clipboard: None,
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
            observed_page: 0,
            page_elapsed: pleasant_ui::page_slide::DURATION,
            drag: None,
            delay_open: false,
            delay_position: 0.0,
            delay_start: 0.0,
            delay_elapsed: pleasant_ui::page_slide::DURATION,
            pending_tap_remove: None,
        }
    }
}

fn header_slot(index: usize) -> Rect {
    let s = ARP_SURFACE;
    if index >= 5 {
        return (s.0 + 260.0 + (index - 5) as f32 * 170.0, s.1 + 49.0, 158.0, 25.0);
    }
    let widths = [130.0, 148.0, 132.0, 96.0, 138.0];
    let start = s.0 + 190.0;
    let remaining = lane_rect(0).0 - 12.0 - start - 4.0 * 8.0;
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
pub(super) fn delay_rect() -> Rect { DELAY_MENU }
pub(super) fn page_block() -> Rect {
    (ARP_SURFACE.0 + 15.0, ARP_SURFACE.1 + 49.0, 106.0, 25.0)
}
// A page dropdown row has independent selection, copy and paste columns.
pub(super) fn page_rect(page: usize) -> Rect {
    let r = page_block();
    (r.0, r.1 + r.3 + 4.0 + page as f32 * 24.0, 100.0, 24.0)
}
pub(super) fn page_copy_rect(page: usize) -> Rect {
    let r = page_rect(page);
    (r.0 + 104.0, r.1, 54.0, r.3)
}
pub(super) fn page_paste_rect(page: usize) -> Rect {
    let r = page_rect(page);
    (r.0 + 162.0, r.1, 66.0, r.3)
}
pub(super) fn lane_rect(index: usize) -> Rect {
    (ARP_SURFACE.0 + ARP_SURFACE.2 - 396.0 + (index % 3) as f32 * 130.0,
     ARP_SURFACE.1 + 10.0 + (index / 3) as f32 * 33.0, 124.0, 31.0)
}
pub(super) fn zoom_rect(index: usize) -> Rect {
    (ARP_SURFACE.0 + 134.0 + index as f32 * 40.0, ARP_SURFACE.1 + 49.0, 34.0, 25.0)
}
pub(super) fn scroll_rect() -> Rect { (graph_rect().0, ARP_SURFACE.1 + ARP_SURFACE.3 - 13.0, graph_rect().2, 7.0) }
pub(super) fn gate_rect() -> Rect { let g = graph_rect(); (g.0, g.1 + g.3 + 6.0, g.2, 22.0) }
fn clip_content(r: Rect) -> Option<Rect> {
    let g = graph_rect();
    let left = r.0.max(g.0);
    let right = (r.0 + r.2).min(g.0 + g.2);
    (right > left).then_some((left, r.1, right - left, r.3))
}
fn scroll_hit_rect() -> Rect { let r = scroll_rect(); (r.0, r.1 - 4.0, r.2, r.3 + 8.0) }
fn content_rect() -> Rect { let g = graph_rect(); (g.0, ARP_SURFACE.1 + 79.0, g.2, ARP_SURFACE.3 - 95.0) }
fn loop_rect(index: usize, count: usize) -> Rect {
    let g = graph_rect();
    let width = g.2 / count.max(1) as f32;
    (g.0 + index as f32 * width + 2.0, ARP_SURFACE.1 + 81.0, width - 4.0, 22.0)
}
fn graph_rect() -> Rect {
    (
        ARP_SURFACE.0 + 22.0,
        ARP_SURFACE.1 + 118.0,
        ARP_SURFACE.2 - 44.0,
        ARP_SURFACE.3 - 207.0,
    )
}
struct StringLayout {
    x: [f32; STEPS],
    interval: [f32; STEPS],
    spacing: f32,
    len: usize,
}
impl StringLayout {
    fn new(steps: &[Step; STEPS], len: usize, swing: f32, phase: i64) -> Self {
        let swing = if len > 1 { swing.clamp(-0.75, 0.75) } else { 0.0 };
        // Reserve the extremes of both swing and ±49% microtiming inside the graph.
        let spacing = graph_rect().2 / (len.max(1) as f32 + swing.abs());
        let mut result = Self { x: [0.0; STEPS], interval: [spacing; STEPS], spacing, len };
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
        let w = (self.spacing - 2.0).clamp(2.0, 36.0);
        (self.x[index] - w * 0.5, ARP_SURFACE.1 + ARP_SURFACE.3 - 39.0, w, 18.0)
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

pub(super) fn page_paste_option_rect(page: usize, index: usize) -> Rect {
    let r = page_paste_rect(page);
    let y = r.1.min(ARP_SURFACE.1 + ARP_SURFACE.3 - 50.0);
    (r.0 + r.2 + 6.0, y + index as f32 * 22.0, 154.0, 22.0)
}
fn dropdown_rect(which: Dropdown, index: usize) -> Rect {
    if let Dropdown::Paste(page) = which { return page_paste_option_rect(page, index); }
    let r = match which {
        Dropdown::Mode => mode_rect(),
        Dropdown::TapLane => tap_lane_rect(),
        Dropdown::Pages => page_block(),
        Dropdown::Paste(_) => unreachable!(),
    };
    let n = match which {
        Dropdown::Mode => 4,
        Dropdown::TapLane => 6,
        Dropdown::Pages => 8,
        Dropdown::Paste(_) => 2,
    };
    let h = 22.0;
    let surface = if which == Dropdown::TapLane { PIANO_SURFACE } else { ARP_SURFACE };
    let y = (r.1 + r.3 + 4.0).min(surface.1 + surface.3 - n as f32 * h - 6.0);
    (r.0, y + index as f32 * h, r.2.max(154.0), h)
}
fn chooser_area() -> Rect {
    let s = ARP_SURFACE;
    (s.0 + 16.0, s.1 + 85.0, s.2 - 32.0, s.3 - 97.0)
}
#[cfg(test)]
pub(super) fn chooser_card(index: usize) -> Rect { chooser_card_for(index, 7) }
pub(super) fn chooser_card_for(index: usize, count: usize) -> Rect {
    let r = chooser_area();
    let gap = 8.0;
    let w = (r.2 - gap * (count - 1) as f32) / count as f32;
    (r.0 + index as f32 * (w + gap), r.1, w, r.3)
}

// These are the ordering depictions from 00d16be's performance controls.
const ORDER_CONTOURS: [[u8; 5]; 7] = [
    [0, 1, 2, 3, 4], [4, 3, 2, 1, 0], [0, 2, 4, 2, 0],
    [0, 2, 1, 3, 2], [4, 2, 3, 1, 2], [0, 3, 1, 2, 4], [2, 4, 0, 3, 1],
];

fn delay_close_rect() -> Rect {
    (PIANO_SURFACE.0 + PIANO_SURFACE.2 - 100.0, PIANO_SURFACE.1 + 8.0, 88.0, 28.0)
}
fn delay_power_rect() -> Rect {
    DELAY_POWER
}
fn delay_sync_rect() -> Rect {
    (PIANO_SURFACE.0 + 20.0, PIANO_SURFACE.1 + 48.0, 106.0, 26.0)
}
fn delay_time_rect() -> Rect {
    (PIANO_SURFACE.0 + 134.0, PIANO_SURFACE.1 + 48.0, 144.0, 26.0)
}
fn delay_gain_rect() -> Rect {
    (PIANO_SURFACE.0 + 286.0, PIANO_SURFACE.1 + 48.0, 132.0, 26.0)
}
fn delay_feedback_rect() -> Rect {
    (PIANO_SURFACE.0 + 426.0, PIANO_SURFACE.1 + 48.0, 140.0, 26.0)
}
fn tap_lane_rect() -> Rect {
    (PIANO_SURFACE.0 + 574.0, PIANO_SURFACE.1 + 48.0, 140.0, 26.0)
}
fn ping_rect() -> Rect {
    (PIANO_SURFACE.0 + 730.0, PIANO_SURFACE.1 + 48.0, 104.0, 26.0)
}
pub(super) fn delay_leading_rect() -> Rect {
    (PIANO_SURFACE.0 + 850.0, PIANO_SURFACE.1 + 48.0, 144.0, 26.0)
}
pub(super) fn delay_follow_rect() -> Rect {
    (PIANO_SURFACE.0 + 1002.0, PIANO_SURFACE.1 + 48.0, 144.0, 26.0)
}
// Keep the add control in the timeline, beside the next tap position.
fn next_tap_time(delay: &DelaySettings) -> f32 {
    let last = delay.taps[..delay.count as usize].iter().map(|t| t.time).fold(0.0, f32::max);
    (last + if delay.sync { 0.5 } else { 250.0 }).min(time_range(delay.sync).1)
}
pub(super) fn add_rect(delay: &DelaySettings, domain: f32) -> Rect {
    let g = tap_graph();
    let next = g.0 + next_tap_time(delay) / domain * g.2;
    let last = delay.taps[..delay.count as usize].iter().map(|t| tap_x(t, domain)).fold(g.0, f32::max);
    let x = next.max(last + 42.0).min(PIANO_SURFACE.0 + PIANO_SURFACE.2 - 24.0);
    (x - 12.0, g.1 + g.3 * 0.5 - 12.0, 24.0, 24.0)
}
pub(super) fn tap_remove_rect(tap: &DelayTap, domain: f32) -> Rect {
    (tap_x(tap, domain) - 9.0, tap_graph().1 + tap_graph().3 + 26.0, 18.0, 18.0)
}
const SOURCE_NAMES: [&str; 5] = ["Arp", "Manual Strum", "Chords", "Bass", "Melody"];
pub(super) fn source_rect(index: usize) -> Rect {
    (
        PIANO_SURFACE.0 + 490.0 + index as f32 * 124.0,
        PIANO_SURFACE.1 + 10.0,
        116.0,
        23.0,
    )
}
fn tap_graph() -> Rect {
    (
        PIANO_SURFACE.0 + 26.0,
        PIANO_SURFACE.1 + 106.0,
        PIANO_SURFACE.2 - 96.0,
        PIANO_SURFACE.3 - 156.0,
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
pub(super) fn time_domain(delay: &DelaySettings) -> f32 {
    delay.taps[..delay.count as usize]
        .iter()
        .map(|t| t.time)
        .fold(if delay.sync { 2.0 } else { 1000.0 }, f32::max)
        .mul_add(1.25, 0.0)
        .min(time_range(delay.sync).1)
}
fn tap_x(tap: &DelayTap, domain: f32) -> f32 {
    let g = tap_graph();
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
    fn arp_pass_len_for_page(&self, page: usize) -> usize {
        let pattern = if self.snapshot.arp_page as usize == page {
            self.routed_plain("arp_pattern").unwrap_or(self.params.arp_pages[page].arp_pattern.value() as f32) as u8
        } else { self.params.arp_pages[page].arp_pattern.value() as u8 };
        crate::engine::arpeggiator::pass_length(self.snapshot.notes.len, pattern).max(1)
    }
    fn arp_string_layout(&self, page: usize, len: usize) -> StringLayout {
        let state = self.params.arpeggiator.snapshot();
        let steps = state.pages[page].flattened_steps(self.arp_pass_len_for_page(page));
        let swing = if self.snapshot.arp_page as usize == page {
            self.routed_plain("swing").unwrap_or(self.params.arp_pages[page].swing.value())
        } else { self.params.arp_pages[page].swing.value() };
        // A fixed cycle origin keeps edit targets still across odd cycles and length modulation.
        let mut layout = StringLayout::new(&steps, len, swing, 0);
        let viewport = self.arp_ui.viewports[page];
        for x in &mut layout.x { *x = viewport.x(*x); }
        for interval in &mut layout.interval { *interval *= viewport.zoom; }
        layout.spacing *= viewport.zoom;
        layout
    }
    fn loop_rect_for_page(&self, page: usize, index: usize, count: usize) -> Rect {
        let r = loop_rect(index, count);
        let viewport = self.arp_ui.viewports[page];
        (viewport.x(r.0), r.1, r.2 * viewport.zoom, r.3)
    }
    fn chooser_count(&self) -> usize { 7 }
    fn chooser_hit_rect(&self, i: usize) -> Rect { chooser_card_for(i, self.chooser_count()) }
    fn initialize_arp_loops(&self) { self.params.arpeggiator.ensure_loops(self.params.required_arp_loops()); }
    pub(super) fn arp_editor_overlay_visible(&self) -> bool {
        self.arp_ui.delay_open || self.arp_ui.delay_position > 0.0
    }
    fn arp_cycle_for_editor(&self) -> ([i16; STEPS], usize) {
        self.arp_cycle_for_page(self.arp_ui.edit_page)
    }
    fn arp_cycle_for_page(&self, page: usize) -> ([i16; STEPS], usize) {
        let state = self.params.arpeggiator.snapshot();
        let mut settings = self.params.arp_pages[page].settings();
        settings.octaves = self.maximum_routed_plain("arp_octaves", settings.octaves as f32) as u8;
        if self.snapshot.arp_page as usize == page {
            settings.pattern = self.routed_plain("arp_pattern").unwrap_or(settings.pattern as f32) as u8;
        }
        let (mut notes, len) = crate::engine::arpeggiator::loop_cycle_notes(
            self.snapshot.notes, settings, std::array::from_fn(|i| {
                let base = state.pages[page].loops[i].octave;
                if self.snapshot.arp_page as usize == page {
                    self.routed_plain(crate::engine::routing::LOOP_OCTAVE_IDS[i]).unwrap_or(base as f32) as i8
                } else { base }
            }), 0x4e6f7465,
        );
        if self.snapshot.arp_page as usize == page {
            let live = (self.snapshot.arp_len as usize).min(len);
            notes[..live].copy_from_slice(&self.snapshot.arp_notes[..live]);
        }
        (notes, len)
    }
    fn visible_loops(&self) -> usize {
        self.maximum_routed_plain("arp_octaves", self.arp_params().arp_octaves.value() as f32).round().clamp(1.0, LOOPS as f32) as usize
    }
    fn edit_loop_octave(&self, index: usize, delta: i8, reset: bool) {
        let page = self.arp_ui.edit_page;
        self.params.arpeggiator.edit(|s| {
            let octave = &mut s.pages[page].loops[index].octave;
            *octave = if reset { 0 } else { (*octave + delta).clamp(-4, 4) };
        });
    }
    fn copy_arp_page(&mut self, page: usize) {
        self.arp_ui.clipboard = Some((self.params.arpeggiator.snapshot().pages[page], self.params.arp_pages[page].settings()));
    }
    fn paste_arp_page(&self, cx: &mut EventContext, destination_page: usize, pattern_only: bool) {
        let Some((copied, copied_settings)) = self.arp_ui.clipboard else { return; };
        let destination = self.params.arpeggiator.snapshot().pages[destination_page];
        let mut page = copied;
        let mut settings = copied_settings;
        if pattern_only {
            page.delay = destination.delay;
            settings = self.params.arp_pages[destination_page].settings();
            settings.pattern = copied_settings.pattern;
            settings.octaves = copied_settings.octaves;
        }
        self.params.arpeggiator.replace_page(destination_page, page, settings);
        let p = &self.params.arp_pages[destination_page];
        let mut values = vec![
            (p.arp_pattern.as_ptr(), p.arp_pattern.preview_normalized(settings.pattern as i32)),
            (p.arp_octaves.as_ptr(), p.arp_octaves.preview_normalized(settings.octaves as i32)),
        ];
        if !pattern_only { values.extend([
            (p.arp_mode.as_ptr(), p.arp_mode.preview_normalized(settings.mode as i32)),
            (p.arp_sync.as_ptr(), p.arp_sync.preview_normalized(settings.sync)),
            (p.arp_free_ms.as_ptr(), p.arp_free_ms.preview_normalized(settings.free_ms)),
            (p.rate.as_ptr(), p.rate.preview_normalized(settings.rate)),
            (p.swing.as_ptr(), p.swing.preview_normalized(settings.swing)),
            (p.humanize.as_ptr(), p.humanize.preview_normalized(settings.humanize)),
        ]); }
        for (ptr, value) in values { Self::emit(cx, ptr, value); }
    }

    pub(in crate::ui) fn arp_editor_button_rects(&self) -> Vec<(Rect, bool)> {
        let mut delay_buttons = vec![(delay_rect(), self.arp_ui.delay_open), (delay_power_rect(), self.params.arpeggiator.snapshot().pages[self.arp_ui.edit_page].delay.enabled)];
        if self.arp_ui.delay_open && self.arp_ui.delay_position == 1.0 {
            let state = self.params.arpeggiator.snapshot();
            let delay = &state.pages[self.arp_ui.edit_page].delay;
            let mut v = vec![
                (delay_close_rect(), true),
                (delay_power_rect(), delay.enabled),
                (delay_sync_rect(), delay.sync),
                (delay_time_rect(), false),
                (delay_gain_rect(), false),
                (delay_feedback_rect(), false),
                (
                    tap_lane_rect(),
                    self.arp_ui.dropdown == Some(Dropdown::TapLane),
                ),
            ];
            v.push((ping_rect(), delay.ping_pong));
            v.push((delay_leading_rect(), delay.voice_leading));
            v.push((delay_follow_rect(), delay.follow_chords));
            v.extend((0..5).map(|i| (source_rect(i), delay.sources & (1 << i) != 0)));
            let domain = time_domain(delay);
            if (delay.count as usize) < TAPS { v.push((add_rect(delay, domain), false)); }
            if delay.count > 1 {
                v.extend(delay.taps[..delay.count as usize].iter().enumerate().map(|(i, tap)|
                    (tap_remove_rect(tap, domain), self.arp_ui.pending_tap_remove == Some(i))));
            }
            delay_buttons = v;
            delay_buttons.push((delay_rect(), true));
        }
        let mut v = delay_buttons;
        v.extend([
            (power_rect(), self.params.arp_enabled.value()),
            (mode_rect(), self.arp_ui.dropdown == Some(Dropdown::Mode)),
            (order_rect(), self.arp_ui.chooser == Some(Chooser::Order)),
            (octaves_rect(), false),
            (sync_rect(), self.arp_params().arp_sync.value()),
            (page_block(), matches!(self.arp_ui.dropdown, Some(Dropdown::Pages | Dropdown::Paste(_)))),
            (zoom_rect(0), false), (zoom_rect(1), false), (zoom_rect(2), false),
        ]);
        v.extend(LANES.iter().enumerate().map(|(i, lane)| (lane_rect(i), self.arp_ui.lane == *lane)));
        if matches!(self.arp_ui.dropdown, Some(Dropdown::Pages | Dropdown::Paste(_))) {
            for page in 0..8 {
                v.extend([(page_rect(page), self.arp_ui.edit_page == page),
                    (page_copy_rect(page), false), (page_paste_rect(page), self.arp_ui.clipboard.is_some())]);
            }
        }
        if let Some(Dropdown::Paste(page)) = self.arp_ui.dropdown {
            v.extend((0..2).map(|i| (page_paste_option_rect(page, i), false)));
        }
        if matches!(self.arp_ui.dropdown, Some(Dropdown::Pages | Dropdown::Paste(_))) { return v; }
        v.extend((0..self.visible_loops()).filter_map(|i| clip_content(self.loop_rect_for_page(self.arp_ui.edit_page, i, self.visible_loops())).map(|r| (r, false))));
        if self.arp_ui.chooser.is_some() {
            let selected = match self.arp_ui.chooser_draw {
                Chooser::Order => self.arp_params().arp_pattern.value() as usize,
            };
            v.extend((0..self.chooser_count()).map(|i| (self.chooser_hit_rect(i), selected == i)));
            return v;
        }
        let state = self.params.arpeggiator.snapshot();
        let (_, len) = self.arp_cycle_for_editor();
        let layout = self.arp_string_layout(self.arp_ui.edit_page, len);
        v.extend((0..len).map(|i| {
            (
                layout.status(i),
                state.pages[self.arp_ui.edit_page].step(i, self.arp_pass_len_for_page(self.arp_ui.edit_page)).state != StepState::Play,
            )
        }));
        v.retain(|(r, _)| r.0 >= graph_rect().0 && r.0 + r.2 <= graph_rect().0 + graph_rect().2 || r.1 < content_rect().1 || r.1 >= PIANO_SURFACE.1);
        v
    }
    pub(in crate::ui) fn arp_editor_route_targets(&self) -> Vec<(usize, Rect)> {
        let mut targets = vec![
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
        ];
        targets.extend((0..self.visible_loops()).filter_map(|i| clip_content(self.loop_rect_for_page(self.arp_ui.edit_page, i, self.visible_loops())).map(|r| (crate::engine::routing::LOOP_OCTAVE_IDS[i], r))));
        targets.into_iter()
        .filter_map(|(id, r)| {
            crate::engine::routing::TARGETS
                .iter()
                .position(|t| t.available() && t.id == id)
                .map(|i| (i, r))
        })
        .collect()
    }
    pub(in crate::ui) fn arp_editor_hint(&self, x: f32, y: f32) -> Option<String> {
        if hit(delay_rect(), x, y) { return Some("Show MIDI Delay in place of the keyboard · Keep editing the arp above".into()); }
        if hit(delay_power_rect(), x, y) { return Some("MIDI Delay power · Independent of arpeggiator power".into()); }
        if self.arp_ui.delay_position > 0.0 && hit(PIANO_SURFACE, x, y) {
            if hit(delay_leading_rect(), x, y) {
                return Some("Include sounding echoes in chord voice leading and apply its octave placement to repeats · Uses the keyboard's Nearest / Furthest mode; Off leaves pitches unchanged".into());
            }
            if hit(delay_follow_rect(), x, y) {
                return Some("Resolve each new repeat to the nearest tone of the active chord · Rechecks every feedback repeat · Keeps the captured pitch when no chord is active".into());
            }
            if hit(delay_close_rect(), x, y) {
                return Some("Return to the keyboard · Delay and its tails keep playing".into());
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
            if hit(tap_graph(), x, y) {
                return Some("Drag a numbered top handle sideways to move its tap · Drag a string vertically, then across to paint · Shift makes a straight ramp · Cmd/Ctrl resets".into());
            }
            return Some("Each tap has an independent echo train · + adds a tap · × below a tap asks for confirmation before deleting it".into());
        }
        if !hit(ARP_SURFACE, x, y) { return None; }
        if self.arp_ui.chooser.is_some() && hit(chooser_area(), x, y) {
            return Some("Choose ordering · Left/Right to browse · Enter to choose · Escape to return".into());
        }
        if hit(mode_rect(), x, y) {
            return Some("Once: one cycle · Loop: restart on chord change · Sync Loop: preserve rhythm · Aligned Loop: follow host beat zero (falls back to Sync Loop without host beats or a synced rate)".into());
        }
        if hit(octaves_rect(), x, y) {
            return Some("Loop Length · Click or scroll 1–4 passes · Set each loop’s octave above the strings · Cmd/Ctrl-click resets".into());
        }
        if matches!(self.arp_ui.dropdown, Some(Dropdown::Pages | Dropdown::Paste(_))) {
            if (0..8).any(|p| hit(page_copy_rect(p), x, y)) { return Some("Copy this complete page, including settings, adjustments and MIDI delay".into()); }
            if (0..8).any(|p| hit(page_paste_rect(p), x, y)) { return Some("Paste into this page · Choose Complete page or Pattern only".into()); }
        }
        if (0..3).any(|i| hit(zoom_rect(i), x, y)) { return Some("Zoom − / + / Fit · Scroll to inspect the phrase; playback never moves the view".into()); }
        if self.arp_ui.lane == Lane::Velocity && hit(gate_rect(), x, y) { return Some("Gate · Click a note’s slot, then drag horizontally anywhere · 200 units spans 1–100% · Outline handle: saved value · Filled bar: live duration · Cmd/Ctrl resets".into()); }
        if (0..self.visible_loops()).any(|i| clip_content(self.loop_rect_for_page(self.arp_ui.edit_page, i, self.visible_loops())).is_some_and(|r| hit(r, x, y))) {
            return Some("Loop octave · Click left/right to lower/raise · Scroll ±4 octaves · Cmd/Ctrl-click resets".into());
        }
        if hit(page_block(), x, y) {
            return Some("Eight complete arp and Delay variations · The selected Page starts at the next rhythmic boundary".into());
        }
        if let Some(i) = (0..LANES.len()).find(|&i| hit(lane_rect(i), x, y)) {
            return Some(format!("{} · {} · Select to edit directly", LANES[i].label(), LANES[i].description()));
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
        hit(graph_rect(), x, y) || (self.arp_ui.delay_open && hit(tap_graph(), x, y))
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
        self.initialize_arp_loops();
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
            self.arp_ui.pending_tap_remove = None;
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
        self.arp_ui.pending_tap_remove = None;
    }
    fn set_delay_editor(&mut self, open: bool) {
        let u = &mut self.arp_ui;
        u.delay_open = open;
        u.pending_tap_remove = None;
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
        self.arp_ui.pending_tap_remove = None;
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
            pass_len: self.arp_pass_len_for_page(self.arp_ui.edit_page),
        });
        cx.capture();
    }
    fn apply_arp_drag(&mut self, cx: &mut EventContext, x: f32, y: f32) {
        let Some(mut drag) = self.arp_ui.drag else {
            return;
        };
        if matches!(drag.kind, DragKind::Step(_) | DragKind::Timing | DragKind::Gate)
            && drag.pass_len != self.arp_pass_len_for_page(drag.page) {
            self.arp_ui.drag = None;
            cx.release();
            return;
        }
        let ramp = cx.modifiers().shift();
        let reset = drag.reset;
        match drag.kind {
            DragKind::Gate => {
                let value = if reset { Step::default().gate as f32 } else { (drag.value + (x - drag.x) * 99.0 / 200.0).round().clamp(1.0, 100.0) };
                self.params.arpeggiator.edit(|s| s.pages[drag.page].step_mut(drag.origin, drag.pass_len).gate = value as u8);
            }
            DragKind::Scroll => {
                let viewport = &mut self.arp_ui.viewports[drag.page];
                viewport.scroll = (drag.value + (x - drag.x) / scroll_rect().2 * viewport.zoom).clamp(0.0, viewport.zoom - 1.0);
            }
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
                        lane.set(s.pages[drag.page].step_mut(i, drag.pass_len), value)
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
                    .edit(|s| Lane::Timing.set(s.pages[drag.page].step_mut(drag.origin, drag.pass_len), v));
            }
            DragKind::Tap(lane) => {
                let state = self.params.arpeggiator.snapshot();
                let delay = &state.pages[drag.page].delay;
                let domain = time_domain(delay);
                let g = tap_graph();
                let next = (0..delay.count as usize)
                    .min_by(|a, b| {
                        (tap_x(&delay.taps[*a], domain) - x)
                            .abs()
                            .total_cmp(
                                &(tap_x(&delay.taps[*b], domain) - x).abs(),
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
                let a = tap_x(&delay.taps[from], domain);
                let b = tap_x(&delay.taps[next], domain);
                self.params.arpeggiator.edit(|s| {
                    for i in 0..delay.count as usize {
                        let xx = tap_x(&delay.taps[i], domain);
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
                    drag.value + (x - drag.x) / tap_graph().2 * drag.last_value
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
                        Code::ArrowLeft | Code::ArrowUp => self.arp_ui.chooser_cursor = (self.arp_ui.chooser_cursor + self.chooser_count() - 1) % self.chooser_count(),
                        Code::ArrowRight | Code::ArrowDown => self.arp_ui.chooser_cursor = (self.arp_ui.chooser_cursor + 1) % self.chooser_count(),
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
        if matches!(event, WindowEvent::KeyDown(Code::Escape, _)) && self.arp_ui.pending_tap_remove.take().is_some() {
            cx.needs_redraw();
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
        if press {
            if let Some(index) = self.arp_ui.pending_tap_remove {
                let state = self.params.arpeggiator.snapshot();
                let delay = &state.pages[self.arp_ui.edit_page].delay;
                if index >= delay.count as usize || !hit(tap_remove_rect(&delay.taps[index], time_domain(delay)), x, y) {
                    self.arp_ui.pending_tap_remove = None;
                }
            }
        }
        if let Some(which) = self.arp_ui.dropdown {
            if press {
                if matches!(which, Dropdown::Pages | Dropdown::Paste(_)) {
                    for page in 0..8 {
                        if hit(page_copy_rect(page), x, y) {
                            self.copy_arp_page(page);
                            self.arp_ui.dropdown = Some(Dropdown::Pages);
                            cx.needs_redraw();
                            return true;
                        }
                        if hit(page_paste_rect(page), x, y) {
                            if self.arp_ui.clipboard.is_some() { self.arp_ui.dropdown = Some(Dropdown::Paste(page)); }
                            cx.needs_redraw();
                            return true;
                        }
                        if hit(page_rect(page), x, y) {
                            self.set_arp_editor_page(page);
                            let p = &self.params.arp_page;
                            Self::emit(cx, p.as_ptr(), p.preview_normalized(page as i32));
                            self.arp_ui.dropdown = None;
                            cx.needs_redraw();
                            return true;
                        }
                    }
                }
                let count = match which {
                    Dropdown::Mode => 4,
                    Dropdown::TapLane => 6,
                    Dropdown::Pages => 8,
                    Dropdown::Paste(_) => 2,
                };
                if let Some(i) = (0..count).find(|&i| hit(dropdown_rect(which, i), x, y)) {
                    match which {
                        Dropdown::Mode => {
                            let p = &self.arp_params().arp_mode;
                            Self::emit(cx, p.as_ptr(), p.preview_normalized(i as i32));
                        }
                        Dropdown::TapLane => self.arp_ui.tap_lane = TAP_LANES[i],
                        Dropdown::Paste(page) => self.paste_arp_page(cx, page, i == 1),
                        Dropdown::Pages => {},
                    }
                }
                self.arp_ui.dropdown = None;
                cx.needs_redraw();
                return true;
            }
            return hit(if which == Dropdown::TapLane { PIANO_SURFACE } else { ARP_SURFACE }, x, y)
                && matches!(
                    event,
                    WindowEvent::MouseScroll(_, _)
                        | WindowEvent::MouseDown(_)
                        | WindowEvent::MouseUp(_)
                );
        }
        if press && hit(delay_rect(), x, y) {
            self.set_delay_editor(!self.arp_ui.delay_open);
            cx.needs_redraw();
            return true;
        }
        if press && hit(delay_power_rect(), x, y) {
            let page = self.arp_ui.edit_page;
            self.params.arpeggiator.edit(|s| s.pages[page].delay.enabled = !s.pages[page].delay.enabled);
            cx.needs_redraw();
            return true;
        }
        if self.arp_editor_overlay_visible() && hit(PIANO_SURFACE, x, y) {
            if self.arp_ui.delay_elapsed < pleasant_ui::page_slide::DURATION {
                return matches!(event, WindowEvent::MouseDown(_) | WindowEvent::MouseUp(_) | WindowEvent::MouseScroll(_, _));
            }
            if self.arp_ui.delay_open { return self.delay_editor_event(cx, event, x, y, press); }
        }
        if !hit(ARP_SURFACE, x, y) { return false; }
        if (self.arp_ui.chooser_position > 0.0 || self.arp_ui.chooser.is_some())
            && hit(chooser_area(), x, y)
        {
            if press && self.arp_ui.chooser.is_some()
                && self.arp_ui.chooser_elapsed >= pleasant_ui::page_slide::DURATION
            {
                if let Some(index) = (0..self.chooser_count()).find(|&i| hit(self.chooser_hit_rect(i), x, y)) {
                    self.choose_arp_option(cx, index);
                }
            }
            return true;
        }
        if self.arp_ui.page_elapsed < pleasant_ui::page_slide::DURATION && hit(content_rect(), x, y) {
            return true;
        }
        self.initialize_arp_loops();
        let reset = cx.modifiers().intersects(Modifiers::CTRL | Modifiers::LOGO);
        if let WindowEvent::MouseScroll(dx, _) = event {
            if *dx != 0.0 && hit(content_rect(), x, y) {
                let viewport = &mut self.arp_ui.viewports[self.arp_ui.edit_page];
                viewport.scroll = (viewport.scroll - dx * 0.1).clamp(0.0, viewport.zoom - 1.0);
                cx.needs_redraw();
                return true;
            }
        }
        if let WindowEvent::MouseScroll(_, dy) = event {
            if *dy == 0.0 { return true; }
            if hit(octaves_rect(), x, y) {
                let p = &self.arp_params().arp_octaves;
                Self::emit(
                    cx,
                    p.as_ptr(),
                    p.preview_normalized((p.value() + dy.signum() as i32).clamp(1, LOOPS as i32)),
                );
                cx.needs_redraw();
                return true;
            }
            if let Some(i) = (0..self.visible_loops()).find(|&i| clip_content(self.loop_rect_for_page(self.arp_ui.edit_page, i, self.visible_loops())).is_some_and(|r| hit(r, x, y))) {
                self.edit_loop_octave(i, dy.signum() as i8, reset);
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
                        let t = s.pages[page].step_mut(i, self.arp_pass_len_for_page(page));
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
                p.preview_normalized(if reset { 1 } else { p.value() % LOOPS as i32 + 1 }),
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
        } else if hit(page_block(), x, y) {
            self.arp_ui.dropdown = Some(Dropdown::Pages);
        } else if let Some(i) = (0..3).find(|&i| hit(zoom_rect(i), x, y)) {
            let viewport = &mut self.arp_ui.viewports[self.arp_ui.edit_page];
            viewport.set_zoom(match i { 0 => viewport.zoom / 2.0, 1 => viewport.zoom * 2.0, _ => 1.0 });
        } else if hit(scroll_hit_rect(), x, y) {
            let viewport = &mut self.arp_ui.viewports[self.arp_ui.edit_page];
            if !hit(viewport.scrollbar(), x, y) {
                viewport.scroll = (((x - scroll_rect().0) / scroll_rect().2) * viewport.zoom - 0.5).clamp(0.0, viewport.zoom - 1.0);
            }
            let value = viewport.scroll;
            self.start_arp_drag(cx, DragKind::Scroll, 0, value, x, y);
        } else if let Some(i) = (0..self.visible_loops()).find(|&i| clip_content(self.loop_rect_for_page(self.arp_ui.edit_page, i, self.visible_loops())).is_some_and(|r| hit(r, x, y))) {
            let r = clip_content(self.loop_rect_for_page(self.arp_ui.edit_page, i, self.visible_loops())).unwrap();
            self.edit_loop_octave(i, if x < r.0 + r.2 * 0.5 { -1 } else { 1 }, reset);
        } else if let Some(i) = (0..LANES.len()).find(|&i| hit(lane_rect(i), x, y)) {
            self.arp_ui.lane = LANES[i];
        } else {
            let (_, len) = self.arp_cycle_for_editor();
            if len == 0 {
                return false;
            }
            let page = self.arp_ui.edit_page;
            let layout = self.arp_string_layout(page, len);
            if !hit(content_rect(), x, y) { return false; }
            if self.arp_ui.lane == Lane::Velocity && hit(gate_rect(), x, y) {
                let i = layout.at(x);
                let value = self.params.arpeggiator.snapshot().pages[page].step(i, self.arp_pass_len_for_page(page)).gate as f32;
                self.start_arp_drag(cx, DragKind::Gate, i, value, x, y);
                if reset { self.apply_arp_drag(cx, x, y); }
            } else if let Some(i) = (0..len).find(|&i| hit(layout.status(i), x, y)) {
                self.params.arpeggiator.edit(|s| {
                    let t = s.pages[page].step_mut(i, self.arp_pass_len_for_page(page));
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
                                    + state.pages[page].step(i, self.arp_pass_len_for_page(page)).micro as f32 * layout.interval[i]
                                        / 100.0
                            };
                            (xx(*a) - x).abs().total_cmp(&(xx(*b) - x).abs())
                        })
                        .unwrap_or(0)
                } else {
                    layout.at(x)
                };
                let value = lane.get(state.pages[page].step(i, self.arp_pass_len_for_page(page)));
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
        let domain = time_domain(delay);
        if delay.count > 1 {
            if let Some(index) = (0..delay.count as usize).find(|&i| hit(tap_remove_rect(&delay.taps[i], domain), x, y)) {
                if self.arp_ui.pending_tap_remove == Some(index) {
                    self.params.arpeggiator.edit(|s| {
                        let d = &mut s.pages[page].delay;
                        // These are independent times, not gaps: copy each survivor intact.
                        d.taps.copy_within(index + 1..d.count as usize, index);
                        d.count -= 1;
                    });
                    self.arp_ui.selected_tap = if selected > index { selected - 1 } else { selected.min(delay.count as usize - 2) };
                    self.arp_ui.pending_tap_remove = None;
                } else {
                    self.arp_ui.pending_tap_remove = Some(index);
                }
                cx.needs_redraw();
                return true;
            }
        }
        self.arp_ui.pending_tap_remove = None;
        if hit(delay_close_rect(), x, y) {
            self.set_delay_editor(false)
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
        } else if hit(delay_leading_rect(), x, y) {
            self.params.arpeggiator.edit(|s| s.pages[page].delay.voice_leading = !delay.voice_leading)
        } else if hit(delay_follow_rect(), x, y) {
            self.params.arpeggiator.edit(|s| s.pages[page].delay.follow_chords = !delay.follow_chords)
        } else if hit(ping_rect(), x, y) {
            self.params
                .arpeggiator
                .edit(|s| s.pages[page].delay.ping_pong = !delay.ping_pong)
        } else if (delay.count as usize) < TAPS && hit(add_rect(delay, domain), x, y) {
            if (delay.count as usize) < TAPS {
                let index = delay.count as usize;
                let t = DelayTap {
                    time: next_tap_time(delay),
                    ..Default::default()
                };
                self.params.arpeggiator.edit(|s| {
                    let d = &mut s.pages[page].delay;
                    d.taps[index] = t;
                    d.count += 1;
                });
                self.arp_ui.selected_tap = index
            }
        } else if (0..5).any(|i| hit(source_rect(i), x, y)) {
            let i = (0..5).find(|&i| hit(source_rect(i), x, y)).unwrap_or(0);
            self.params
                .arpeggiator
                .edit(|s| s.pages[page].delay.sources ^= 1 << i)
        } else {
            let g = tap_graph();
            let domain = time_domain(delay);
            // Numbered caps are separate horizontal handles on every adjustment lane.
            let nearest = (0..delay.count as usize)
                .min_by(|a, b| {
                    (tap_x(&delay.taps[*a], domain) - x)
                        .abs()
                        .total_cmp(
                            &(tap_x(&delay.taps[*b], domain) - x).abs(),
                        )
                })
                .unwrap_or(0);
            let tx = tap_x(&delay.taps[nearest], domain);
            if (x - tx).abs() < 22.0 && y >= g.1 - 26.0 && y < g.1 {
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

    fn arp_button(&self, d: &mut Draw, r: Rect, label: &str, on: bool, color: Color) {
        self.button_surface(d, r, on, color);
        let label = self.fit_text(d, label, r.2 - 8.0, 13.0);
        d.text_centered(r.0 + r.2 * 0.5, r.1 + r.3 * 0.5 + 4.5, &label, 13.0, if on { color } else { TEXT });
    }
    fn arp_icon_button(&self, d: &mut Draw, r: Rect, label: &str, on: bool, color: Color, icon: style::Icon) {
        self.button_surface(d, r, on, color);
        let label = self.fit_text(d, label, r.2 - 31.0, 13.0);
        let width = d.text_width(&label, 13.0);
        let left = r.0 + (r.2 - width - 23.0) * 0.5;
        let color = if on { color } else { TEXT };
        icon.draw(d, left + 9.0, r.1 + r.3 * 0.5, color);
        d.text_middle(left + 23.0, r.1 + r.3 * 0.5, &label, 13.0, color);
    }
    fn draw_arp_lane_button(&self, d: &mut Draw, r: Rect, lane: Lane) {
        use nih_plug_vizia::vizia::vg::{Align, Baseline, Paint};
        let on = self.arp_ui.lane == lane;
        let color = if on { GOLD } else { TEXT };
        self.button_surface(d, r, on, GOLD);
        if let Some(font) = d.icon_font {
            let mut paint = Paint::color(d.color(color));
            paint.set_font(&[font]);
            paint.set_font_size(18.0 * d.s);
            paint.set_text_align(Align::Center);
            paint.set_text_baseline(Baseline::Middle);
            let _ = d.c.fill_text(d.ox + (r.0 + 15.0 + d.offset_x) * d.s,
                d.oy + (r.1 + r.3 * 0.5) * d.s, lane.glyph(), &paint);
        }
        d.text_middle(r.0 + 29.0, r.1 + r.3 * 0.5, lane.label(), 10.5, color);
    }
    fn draw_arp_page_status(&self, d: &mut Draw, r: Rect, page: usize) {
        if self.snapshot.arp_page as usize == page && self.snapshot.arp_running {
            d.line(r.0 + 4.0, r.1 + r.3 - 2.0, r.0 + r.2 - 4.0, r.1 + r.3 - 2.0, GOLD, 2.0);
        }
        if self.snapshot.arp_pending as usize == page {
            d.circle(r.0 + r.2 - 4.0, r.1 + 4.0, 2.5, GOLD, true);
        }
    }
    fn string_readout(&self, d: &mut Draw, x: f32, y: f32, g: Rect, label: &str, color: Color) {
        let width = d.text_width(label, 14.0);
        // Put three quarters of the readout to the left of its handle.
        let left = (x - width * 0.75).clamp(g.0, g.0 + g.2 - width);
        d.text(left, (y - 10.0).max(g.1 + 14.0), label, 14.0, color);
    }

    pub(in crate::ui) fn draw_arp_editor(&self, d: &mut Draw) {
        let s = ARP_SURFACE;
        d.scissor(s.0, s.1, s.2, s.3);
        let saved_offset = d.offset_x;
        let saved_disabled = d.disabled;
        d.disabled = !self.params.arp_enabled.value();
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
        d.offset_x = saved_offset;
        if let Some(which @ (Dropdown::Mode | Dropdown::Pages | Dropdown::Paste(_))) = self.arp_ui.dropdown {
            self.draw_arp_dropdown(d, which);
        }
        d.disabled = saved_disabled;
        d.reset_scissor();
    }
    fn draw_arp_strings(&self, d: &mut Draw) {
        let s = ARP_SURFACE;
        let p = self.arp_params();
        let power = power_rect();
        let disabled = d.disabled;
        d.disabled = false;
        d.bypass_button(
            power,
            !self.params.arp_enabled.value(),
            TEAL,
            self.hover_amount(power) > 0.01,
            0.0,
        );
        d.disabled = disabled;
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
        self.arp_icon_button(
            d,
            mode_rect(),
            &format!("{} ▾", MODE_NAMES[p.arp_mode.value().clamp(0, 3) as usize]),
            self.arp_ui.dropdown == Some(Dropdown::Mode),
            GOLD,
            repeat_icon(p.arp_mode.value().clamp(0, 3) as usize),
        );
        self.arp_button(
            d,
            order_rect(),
            &format!(
                "{} ▾",
                ARP_PATTERN_NAMES[p.arp_pattern.value().clamp(0, 6) as usize]
            ),
            self.arp_ui.chooser == Some(Chooser::Order),
            GOLD,
        );
        self.arp_button(
            d,
            octaves_rect(),
            &format!("Loop Length: {}", p.arp_octaves.value()),
            false,
            GOLD,
        );
        self.arp_button(
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
            self.arp_button(
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
        d.line(
            s.0 + 12.0,
            s.1 + 43.0,
            lane_rect(0).0 - 12.0,
            s.1 + 43.0,
            LINE,
            1.0,
        );
        let page_button = page_block();
        self.arp_button(d, page_button, &format!("Page {} ▾", self.arp_ui.edit_page + 1),
            matches!(self.arp_ui.dropdown, Some(Dropdown::Pages | Dropdown::Paste(_))), TEAL);
        self.draw_arp_page_status(d, page_button, self.arp_ui.edit_page);
        for (i, lane) in LANES.iter().enumerate() {
            self.draw_arp_lane_button(d, lane_rect(i), *lane);
        }
        for (i, label) in ["−", "+", "Fit"].iter().enumerate() { self.arp_button(d, zoom_rect(i), label, false, TEAL); }
        let status = if p.arp_mode.value() == 3 && !self.snapshot.arp_aligned {
            "Aligned Loop · using Sync Loop"
        } else if self.snapshot.arp_pending < 8 {
            "Page queued for next boundary"
        } else if matches!(self.arp_ui.lane, Lane::Mod1 | Lane::Mod2) {
            "Bipolar additive modulation · −100% to +100%"
        } else if self.arp_ui.lane == Lane::Velocity {
            "Velocity: drag ↑↓ · Gate strip: drag ←→ · Cmd/Ctrl: reset"
        } else {
            "Drag to adjust · Shift: straight ramp · Cmd/Ctrl: reset"
        };
        d.text(
            s.0 + 610.0,
            s.1 + 66.0,
            status,
            11.0,
            MUTED,
        );
        self.draw_arp_page_slide(d, content_rect(), Self::draw_arp_pattern);
        let track = scroll_rect();
        d.rounded_rect(track.0, track.1, track.2, track.3, 3.0, LINE);
        let thumb = self.arp_ui.viewports[self.arp_ui.edit_page].scrollbar();
        d.rounded_rect(thumb.0, thumb.1, thumb.2, thumb.3, 3.0, MUTED);
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
        let surface = if r.1 >= PIANO_SURFACE.1 { PIANO_SURFACE } else { ARP_SURFACE };
        d.scissor(surface.0, surface.1, surface.2, surface.3);
    }
    fn draw_arp_pattern(&self, d: &mut Draw, page_index: usize) {
        let state = self.params.arpeggiator.snapshot();
        let page = &state.pages[page_index];
        let (notes, len) = self.arp_cycle_for_page(page_index);
        let layout = self.arp_string_layout(page_index, len);
        let g = graph_rect();
        let count = self.maximum_routed_plain("arp_octaves", self.params.arp_pages[page_index].arp_octaves.value() as f32).round().clamp(1.0, LOOPS as f32) as usize;
        let active_loops = if self.snapshot.arp_page as usize == page_index { self.routed_plain("arp_octaves").unwrap_or(self.params.arp_pages[page_index].arp_octaves.value() as f32) as usize } else { self.params.arp_pages[page_index].arp_octaves.value() as usize };
        for i in 0..count {
            let raw = self.loop_rect_for_page(page_index, i, count);
            let Some(r) = clip_content(raw) else { continue; };
            let octave = page.loops[i].octave;
            self.arp_button(d, r, &format!("−  Loop {}: {octave:+} oct  +", i + 1), false, if i < active_loops { TEAL } else { MUTED });
            if self.snapshot.arp_page as usize == page_index {
                if let Some(live) = self.routed_plain(crate::engine::routing::LOOP_OCTAVE_IDS[i]) {
                    d.text_centered(r.0 + r.2 * 0.5, r.1 + r.3 + 11.0, &format!("live {live:+.0}"), 10.0, TEAL);
                }
            }
            if i > 0 { dotted(d, raw.0 - 2.0, g.1, g.3, alpha(MUTED, 0.3)); }
        }
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
            let step = page.step(i, self.arp_pass_len_for_page(page_index));
            if layout.x[i] + layout.spacing < g.0 || layout.x[i] - layout.spacing > g.0 + g.2 { continue; }
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
            let visible_active = if self.snapshot.arp_page as usize == page_index && self.snapshot.arp_running {
                i < self.snapshot.arp_len as usize
            } else { i < len / count * active_loops };
            let strength = if step.state == StepState::Play && visible_active { 1.0 } else { 0.38 };
            if lane == Lane::Timing {
                dotted(d, nominal, g.1, g.3, alpha(MUTED, 0.6));
                d.line(
                    x,
                    g.1,
                    x,
                    g.1 + g.3,
                    alpha(color, strength),
                    if active { 6.0 } else { 4.0 },
                );
                d.circle(x, g.1 + g.3 * 0.5, 6.0, color, true)
            } else {
                let y = value_y(lane.get(step), range, g);
                d.line(x, g.1, x, g.1 + g.3, alpha(color, 0.25 * strength), 1.0);
                d.line(
                    x,
                    baseline,
                    x,
                    y,
                    alpha(color, strength),
                    if active { 8.0 } else { 6.0 },
                );
                d.circle(
                    x,
                    y,
                    if active { 7.0 } else { 6.0 },
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
                let y = if lane == Lane::Timing { g.1 + g.3 * 0.5 } else { value_y(lane.get(step), range, g) };
                self.string_readout(d, x, y, g, &lane.format(lane.get(step)), TEXT)
            }
            if lane == Lane::Velocity {
                let strip = gate_rect();
                // Hit testing uses the fixed note slot, independently of the live bar.
                let left = nominal - layout.spacing * 0.42;
                let width = layout.spacing * 0.84;
                let saved_x = left + width * step.gate as f32 / 100.0;
                let live = if self.snapshot.arp_page as usize == page_index {
                    self.snapshot.arp_gates[i]
                } else { step.gate as f32 / 100.0 };
                d.rounded_rect(left, strip.1 + 5.0, width, 12.0, 2.0, alpha(MUTED, 0.15));
                d.rounded_rect(left, strip.1 + 6.0, (width * live).max(1.0), 10.0, 2.0, alpha(color, strength * 0.65));
                d.outline_rounded(saved_x - 2.5, strip.1 + 2.0, 5.0, 18.0, 2.0, alpha(TEXT, strength), 1.0);
                if self.pointer.is_some_and(|(px, py)| hit(strip, px, py) && layout.at(px) == i)
                    || self.arp_ui.drag.is_some_and(|drag| matches!(drag.kind, DragKind::Gate) && drag.origin == i) {
                    self.string_readout(d, nominal, strip.1, strip, &format!("{}%", step.gate), TEXT);
                }
            }
            let pitch = note + step.octave as i16 * 12;
            let label = if (0..=127).contains(&pitch) { self.note_name(pitch as u8) } else { "—".into() };
            d.text_centered(
                nominal,
                g.1 + g.3 + 43.0,
                &label,
                if layout.spacing < 23.0 { 11.0 } else { 13.0 },
                if active { GOLD } else { MUTED },
            );
            let r = layout.status(i);
            let mark = match step.state {
                StepState::Play => "•",
                StepState::Mute => "–",
                StepState::Skip => "×",
            };
            self.arp_button(
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
    pub(super) fn draw_keyboard_pages(&self, d: &mut Draw) {
        let s = PIANO_SURFACE;
        self.surface(d, s);
        d.scissor(s.0, s.1, s.2, s.3);
        let saved_offset = d.offset_x;
        let progress = self.arp_ui.delay_position;
        if progress < 1.0 {
            d.offset_x = saved_offset - progress * s.2;
            self.draw_piano(d);
        }
        if progress > 0.0 {
            d.offset_x = saved_offset + (1.0 - progress) * s.2;
            self.draw_delay_strings(d);
        }
        d.offset_x = saved_offset;
        if self.arp_ui.dropdown == Some(Dropdown::TapLane) {
            self.draw_arp_dropdown(d, Dropdown::TapLane);
        }
        d.reset_scissor();
    }
    pub(super) fn draw_delay_header(&self, d: &mut Draw) {
        let state = self.params.arpeggiator.snapshot();
        let enabled = state.pages[self.arp_ui.edit_page].delay.enabled;
        d.bypass_button(delay_power_rect(), !enabled, TEAL, self.hover_amount(delay_power_rect()) > 0.01, 0.0);
        self.arp_button(d, delay_rect(), "MIDI Delay ›", self.arp_ui.delay_open, TEAL);
    }
    fn draw_delay_strings(&self, d: &mut Draw) {
        let s = PIANO_SURFACE;
        let state = self.params.arpeggiator.snapshot();
        let delay = &state.pages[self.arp_ui.edit_page].delay;
        let selected = self.arp_ui.selected_tap.min(delay.count as usize - 1);
        let tap = &delay.taps[selected];
        let u = &self.arp_ui;
        d.text(s.0 + 18.0, s.1 + 29.0, "MIDI DELAY", 16.0, if delay.enabled { TEXT } else { MUTED });
        d.text(
            s.0 + 210.0,
            s.1 + 28.0,
            &format!(
                "Page {} · Tap {} of {}",
                u.edit_page + 1,
                selected + 1,
                delay.count
            ),
            13.0,
            MUTED,
        );
        self.arp_button(d, delay_close_rect(), "‹ Keyboard", true, TEAL);
        d.line(
            s.0 + 12.0,
            s.1 + 43.0,
            s.0 + s.2 - 12.0,
            s.1 + 43.0,
            LINE,
            1.0,
        );
        self.arp_button(
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
        self.arp_button(
            d,
            delay_time_rect(),
            &format!("Time: {}", time_label(tap.time, delay.sync)),
            false,
            GOLD,
        );
        self.arp_button(
            d,
            delay_gain_rect(),
            &format!("Velocity: {:.0}%", tap.velocity * 100.0),
            false,
            GOLD,
        );
        self.arp_button(
            d,
            delay_feedback_rect(),
            &format!("Feedback: {:.0}%", tap.feedback * 100.0),
            false,
            GOLD,
        );
        self.arp_button(
            d,
            tap_lane_rect(),
            &format!("{} ▾", u.tap_lane.label()),
            u.dropdown == Some(Dropdown::TapLane),
            GOLD,
        );
        self.arp_button(d, ping_rect(), "Ping pong", delay.ping_pong, TEAL);
        self.arp_button(d, delay_leading_rect(), "Voice leading", delay.voice_leading, COLORS[1]);
        self.arp_button(d, delay_follow_rect(), "Follow chords", delay.follow_chords, TEAL);
        d.text(s.0 + 440.0, s.1 + 27.0, "Input", 13.0, MUTED);
        for (i, name) in SOURCE_NAMES.iter().enumerate() {
            self.arp_button(d, source_rect(i), name, delay.sources & (1 << i) != 0, TEAL)
        }
        let g = tap_graph();
        let area = (s.0 + 8.0, g.1 - 28.0, s.2 - 16.0, s.3 - (g.1 - s.1) + 28.0);
        self.draw_arp_page_slide(d, area, Self::draw_delay_pattern);
    }
    fn draw_delay_pattern(&self, d: &mut Draw, page_index: usize) {
        let s = PIANO_SURFACE;
        let state = self.params.arpeggiator.snapshot();
        let delay = &state.pages[page_index].delay;
        let u = &self.arp_ui;
        let selected = u.selected_tap.min(delay.count as usize - 1);
        let g = tap_graph();
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
                0 => d.text(x, y, &label, 12.0, MUTED),
                4 => d.text_right(x, y, &label, 12.0, MUTED),
                _ => d.text_centered(x, y, &label, 12.0, MUTED),
            }
        }
        d.line(g.0, baseline, g.0 + g.2, baseline, LINE, 1.0);
        for i in 0..delay.count as usize {
            let t = &delay.taps[i];
            let x = tap_x(t, domain);
            let y = value_y(u.tap_lane.get(t), range, g);
            let color = if i == selected { GOLD } else { TEAL };
            d.line(x, g.1, x, g.1 + g.3, alpha(color, 0.3), 1.0);
            d.line(
                x,
                baseline,
                x,
                y,
                color,
                if i == selected { 8.0 } else { 6.0 },
            );
            d.circle(x, y, 6.0, color, true);
            d.rounded_rect(x - 20.0, g.1 - 26.0, 40.0, 22.0, 4.0, alpha(color, 0.18));
            d.text_centered(x, g.1 - 10.0, &format!("{} ↔", i + 1), 13.0, color);
            self.string_readout(d, x, y, g, &u.tap_lane.format(u.tap_lane.get(t)), if i == selected { TEXT } else { MUTED });
            if delay.count > 1 {
                let r = tap_remove_rect(t, domain);
                let confirming = u.pending_tap_remove == Some(i);
                d.rect(r.0, r.1, r.2, r.3, LINE);
                self.button_surface(d, r, confirming, color);
                if confirming {
                    self.close_icon(d, r, color, true);
                } else {
                    let (cx, cy) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
                    d.line(cx - 3.5, cy - 3.5, cx + 3.5, cy + 3.5, color, 1.5);
                    d.line(cx - 3.5, cy + 3.5, cx + 3.5, cy - 3.5, color, 1.5);
                }
                if confirming {
                    let label = "Are you sure?";
                    let width = d.text_width(label, 13.0) + 12.0;
                    let left = (x - width * 0.5).clamp(s.0 + 4.0, s.0 + s.2 - width - 4.0);
                    d.rounded_rect(left, r.1 - 22.0, width, 22.0, 4.0, PANEL);
                    d.text_centered(left + width * 0.5, r.1 - 6.0, label, 13.0, TEXT);
                }
            }
        }
        if (delay.count as usize) < TAPS {
            let r = add_rect(delay, domain);
            self.button_surface(d, r, false, TEAL);
            let (x, y) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
            d.line(x - 6.0, y, x + 6.0, y, TEAL, 2.0);
            d.line(x, y - 6.0, x, y + 6.0, TEAL, 2.0);
        }
    }
    fn draw_arp_dropdown(&self, d: &mut Draw, which: Dropdown) {
        if which == Dropdown::Pages {
            let first = page_rect(0);
            d.rounded_rect(first.0 - 4.0, first.1 - 4.0, 236.0, 200.0, 4.0, PANEL);
            for page in 0..8 {
                let r = page_rect(page);
                self.arp_button(d, r, &format!("Page {}", page + 1), self.arp_ui.edit_page == page, TEAL);
                self.draw_arp_page_status(d, r, page);
                self.arp_button(d, page_copy_rect(page), "Copy", false, TEAL);
                let disabled = d.disabled;
                d.disabled |= self.arp_ui.clipboard.is_none();
                self.arp_button(d, page_paste_rect(page), "Paste ▸", false, TEAL);
                d.disabled = disabled;
            }
            return;
        }
        if let Dropdown::Paste(_) = which { self.draw_arp_dropdown(d, Dropdown::Pages); }
        let count = match which {
            Dropdown::Mode => 4,
            Dropdown::TapLane => 6,
            Dropdown::Pages => 8,
            Dropdown::Paste(_) => 2,
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
                Dropdown::Paste(_) => (["Complete page", "Pattern only"][i], false),
                Dropdown::Pages => unreachable!(),
            };
            if which == Dropdown::Mode {
                self.arp_icon_button(d, r, label, on, GOLD, repeat_icon(i));
            } else {
                self.arp_button(d, r, label, on, GOLD);
            }
        }
    }
    fn draw_arp_chooser(&self, d: &mut Draw) {
        let u = &self.arp_ui;
        let selected = match u.chooser_draw {
            Chooser::Order => self.arp_params().arp_pattern.value() as usize,
        };
        for i in 0..self.chooser_count() {
            let r = self.chooser_hit_rect(i);
            let on = selected == i;
            self.button_surface(d, r, on, GOLD);
            let color = if on { GOLD } else if self.hover_amount(r) > 0.01 { TEXT } else { MUTED };
            let cx = r.0 + r.2 * 0.5;
            let cy = r.1 + r.3 * 0.36;
            let label = match u.chooser_draw {
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
        let mut steps = [Step::default(); STEPS];
        for len in [2, 3, 4, 95, 96, STEPS] {
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
        view.set_arp_chooser(Some(Chooser::Order));
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
    fn delay_slide_reverses_from_its_current_position() {
        let mut view = ChordboardView::new(Arc::new(ChordboardParams::default()), Arc::new(Bridge::default()), Arc::new(PreviewContext));
        view.set_delay_editor(true);
        view.tick_arp_editor(pleasant_ui::page_slide::DURATION * 0.5);
        assert_eq!(view.arp_ui.delay_position, 0.5);
        view.set_delay_editor(false);
        view.tick_arp_editor(pleasant_ui::page_slide::DURATION * 0.5);
        assert_eq!(view.arp_ui.delay_position, 0.25);
        view.tick_arp_editor(pleasant_ui::page_slide::DURATION);
        assert_eq!(view.arp_ui.delay_position, 0.0);
    }
    #[test]
    fn full_cycles_stay_even_and_inside_the_module() {
        for len in [1, 4, 32, 48, 94, 96, STEPS] {
            let layout = StringLayout::new(&[Step::default(); STEPS], len, 0.0, 0);
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
    fn modulated_loop_length_keeps_every_edit_position_fixed() {
        use crate::engine::routing::TARGETS;
        let mut view = ChordboardView::new(Arc::new(ChordboardParams::default()), Arc::new(Bridge::default()), Arc::new(PreviewContext));
        view.snapshot.notes = harmony::voice(60, 0, None, 0, 0, 0);
        let target = TARGETS.iter().position(|t| t.id == "arp_octaves").unwrap();
        let route = &mut Arc::get_mut(&mut view.params).unwrap().routes[1];
        route.source = IntParam::new("Source", 1, IntRange::Linear { min: 0, max: crate::engine::routing::SOURCE_COUNT as i32 });
        route.target = IntParam::new("Target", target as i32, IntRange::Linear { min: 0, max: TARGETS.len() as i32 - 1 });
        let (_, count) = view.arp_cycle_for_editor();
        assert_eq!(count, 12);
        assert_eq!(view.visible_loops(), 4);
        Arc::get_mut(&mut view.params).unwrap().arp_pages[0].swing = FloatParam::new("Swing", 0.5, FloatRange::Linear { min: -0.75, max: 0.75 });
        view.snapshot.arp_running = true;
        view.snapshot.arp_phase = 0;
        let before = view.arp_string_layout(0, count);
        view.snapshot.arp_phase = 25;
        view.snapshot.arp_step = 1;
        view.snapshot.routed[target] = Some(1.0);
        view.snapshot.arp_len = 24;
        let (_, after_count) = view.arp_cycle_for_editor();
        let after = view.arp_string_layout(0, after_count);
        assert_eq!(before.x, after.x);
        assert_eq!(before.status(11), after.status(11));
        view.snapshot.routed[target] = Some(0.0);
        view.snapshot.arp_len = 3;
        assert_eq!(view.arp_cycle_for_editor().1, count);
    }

    #[test]
    fn viewport_preserves_center_clamps_scroll_and_is_remembered_per_page() {
        let mut viewport = Viewport { zoom: 2.0, scroll: 0.5 };
        viewport.set_zoom(4.0);
        assert_eq!(viewport.scroll, 1.5);
        assert_eq!((viewport.scroll + 0.5) / viewport.zoom, 0.5);
        viewport.set_zoom(1.0);
        assert_eq!(viewport.scroll, 0.0);
        let mut ui = ArpUi::default();
        ui.viewports[2] = Viewport { zoom: 8.0, scroll: 7.0 };
        assert_eq!(ui.viewports[0].zoom, 1.0);
        assert_eq!(ui.viewports[2].scrollbar().0 + ui.viewports[2].scrollbar().2, scroll_rect().0 + scroll_rect().2);
        assert_eq!(LANES.len(), 6);
        assert_eq!(LANES[0].label(), "Velocity / Gate");
    }
    #[test]
    fn zoomed_steps_status_and_loop_boundaries_share_the_same_transform() {
        let mut view = ChordboardView::new(Arc::new(ChordboardParams::default()), Arc::new(Bridge::default()), Arc::new(PreviewContext));
        view.snapshot.notes = harmony::voice(60, 0, None, 0, 0, 0);
        let base = view.arp_string_layout(0, 12);
        let loop_base = loop_rect(1, 4);
        let viewport = Viewport { zoom: 4.0, scroll: 1.0 };
        view.arp_ui.viewports[0] = viewport;
        let zoomed = view.arp_string_layout(0, 12);
        for i in 0..12 {
            assert_eq!(zoomed.x[i], viewport.x(base.x[i]));
            assert_eq!(zoomed.at(zoomed.x[i]), i);
            let status = zoomed.status(i);
            assert_eq!(status.0 + status.2 * 0.5, zoomed.x[i]);
        }
        assert_eq!(view.loop_rect_for_page(0, 1, 4).0, viewport.x(loop_base.0));
        assert_eq!(zoomed.spacing, base.spacing * 4.0);
        assert!(clip_content(view.loop_rect_for_page(0, 3, 4)).is_none());
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
        assert_eq!(s.gate, 65);
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
    fn page_menu_and_direct_adjustments_have_separate_targets() {
        let mut controls = vec![page_block(), mode_rect(), order_rect(), octaves_rect(),
            sync_rect(), rate_rect(), humanize_rect(), swing_rect()];
        controls.extend((0..LANES.len()).map(lane_rect));
        controls.extend((0..3).map(zoom_rect));
        for (i, r) in controls.iter().enumerate() {
            assert!(r.1 + r.3 < content_rect().1);
            for other in &controls[..i] {
                assert!(r.0 >= other.0 + other.2 || other.0 >= r.0 + r.2 ||
                    r.1 >= other.1 + other.3 || other.1 >= r.1 + r.3, "{r:?} overlaps {other:?}");
            }
        }
        for page in 0..8 {
            let r = page_rect(page);
            let copy = page_copy_rect(page);
            let paste = page_paste_rect(page);
            assert!(r.0 + r.2 < copy.0 && copy.0 + copy.2 < paste.0);
            assert!(paste.1 + paste.3 < ARP_SURFACE.1 + ARP_SURFACE.3);
        }
        let g = tap_graph();
        assert!(g.3 >= 88.0);
        let remove = tap_remove_rect(&DelayTap::default(), 2.5);
        assert_eq!((remove.2, remove.3), (18.0, 18.0));
        assert!(remove.1 > g.1 + g.3 + 14.0);
        assert!(remove.1 + remove.3 <= PIANO_SURFACE.1 + PIANO_SURFACE.3);
    }

    #[test]
    fn dropdowns_and_delay_graph_fit_and_handles_are_separate() {
        for (which, n) in [
            (Dropdown::Mode, 4),
            (Dropdown::TapLane, 6),
            (Dropdown::Paste(0), 2),
            (Dropdown::Paste(7), 2),
        ] {
            for i in 0..n {
                let r = dropdown_rect(which, i);
                let surface = if which == Dropdown::TapLane { PIANO_SURFACE } else { ARP_SURFACE };
                assert!(hit(surface, r.0, r.1));
                assert!(hit(surface, r.0 + r.2, r.1 + r.3));
            }
        }
        let g = tap_graph();
        assert!(g.1 > delay_time_rect().1 + delay_time_rect().3);
        assert!(g.1 + g.3 < PIANO_SURFACE.1 + PIANO_SURFACE.3 - 27.0);
    }
}
