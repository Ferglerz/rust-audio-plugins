// Shared geometry for rendering and event hit-testing.
use super::KEY_COLUMNS;
const WIDTH_REDUCTION: f32 = 128.0;
const OLD_MOD_COLUMN: f32 = 122.0;
pub(super) const MAIN_W: f32 = 1871.0 - WIDTH_REDUCTION - OLD_MOD_COLUMN;
pub(super) const W: f32 = MAIN_W + 190.0;
pub(super) const H: f32 = 740.0;
pub(super) type Rect = (f32, f32, f32, f32);
pub(super) const HEADER_H: f32 = 76.0;
pub(super) const MODULE_HEADER_H: f32 = 44.0;
pub(super) const MODULE_TITLE_SIZE: f32 = 15.0;

pub(super) fn module_header_mid(y: f32) -> f32 {
    y + MODULE_HEADER_H * 0.5
}

pub(super) fn module_title_y(y: f32, size: f32) -> f32 {
    module_header_mid(y) + size * 0.35
}

pub(super) const PAD: Rect = (1096.0 - WIDTH_REDUCTION, 136.0, 621.0, 358.0);
// Keep drawing and pointer targets on the same performance surface.
const fn field_corner(pad: Rect) -> Rect {
    (pad.0 + pad.2 - 28.0, 100.0, 28.0, 28.0)
}
const fn field_latch(pad: Rect) -> Rect {
    let corner = field_corner(pad);
    (corner.0 - 136.0, corner.1, 128.0, 28.0)
}
const fn manual_field_inset(pad: Rect) -> Rect {
    // Center the field including its tags: 14px beyond the left, top and
    // right handle edges. Keep the bottom clear of the note-length control.
    const GRIP: f32 = pleasant_ui::handles::TAG_LONG + pleasant_ui::handles::TAG_POINTER_LEN;
    const LEFT: f32 = 14.0 + GRIP;
    const RIGHT: f32 = 14.0 + pleasant_ui::handles::TAG_THICK * 0.5;
    (pad.0 + LEFT, pad.1 + LEFT, pad.2 - LEFT - RIGHT, pad.3 - LEFT - 72.0)
}
pub(super) const ARP_MAIN_WIDTH: f32 = 612.0;
pub(super) const ARP_PAD: Rect = (32.0, 136.0, 892.0, 358.0);
pub(super) const ARP_EDITOR: Rect = (660.0, 136.0, 264.0, 358.0);
pub(super) const PLAY_PAD: Rect = manual_field_inset(PAD);
pub(super) const AUTO_FIELD: Rect = (PAD.0 + 20.0, 286.0, PAD.2 - 40.0, 100.0);
pub(super) const CHORDS_SURFACE: Rect = (16.0, 92.0, 924.0, 418.0);
pub(super) const PERF_SURFACE: Rect = (1080.0 - WIDTH_REDUCTION, 92.0, 653.0, 418.0);
// Vertical modulation module beside both performance and keyboard surfaces.
pub(super) const MOD_SURFACE: Rect = (MAIN_W, 92.0, 174.0, 630.0);
pub(super) const EXPAND: Rect = field_corner(PAD);
pub(super) const EXPANDED_SURFACE: Rect = (16.0, 92.0, MAIN_W - 32.0, 418.0);
pub(super) const EXPANDED_PAD: Rect = (32.0, PAD.1, MAIN_W - 64.0, PAD.3);
pub(super) const EXPANDED_PLAY_PAD: Rect = manual_field_inset(EXPANDED_PAD);
pub(super) const EXPANDED_EXPAND: Rect = field_corner(EXPANDED_PAD);
pub(super) const EXPANDED_STRUM_SYNC: Rect =
    (1567.0 - WIDTH_REDUCTION - OLD_MOD_COLUMN, 116.0, 92.0, 28.0);
pub(super) const STRUM_RATE: Rect = (ARP_PAD.0 + 8.0, 344.0, ARP_MAIN_WIDTH - 16.0, 36.0);
pub(super) const EXPANDED_STRUM_RATE: Rect = (
    1339.0 - WIDTH_REDUCTION - OLD_MOD_COLUMN,
    116.0,
    220.0,
    28.0,
);
pub(super) const LATCH: Rect = (
    32.0,
    KEY_TOP + 2.0 * KEY_ROW_PITCH + TILE_HEIGHT + 10.0,
    78.0,
    TILE_HEIGHT,
);
// Chords header row: Key, Scale, Order, selection mode, Transpose on an 8px rhythm.
pub(super) const KEY_MENU: Rect = (104.0, 100.0, 104.0, 28.0);
pub(super) const SCALE_MENU: Rect = (216.0, 100.0, 148.0, 28.0);
pub(super) const ORDER: Rect = (372.0, 100.0, 116.0, 28.0);
pub(super) const ROOT_ON_SELECT: Rect = (496.0, 100.0, 148.0, 28.0);
pub(super) const TRANSPOSE: Rect = (652.0, 100.0, 272.0, 28.0);
pub(super) const APPEARANCE: Rect = (W - 16.0 - 72.0, 25.0, 72.0, 26.0);
pub(super) const STRUM_BYPASS: Rect = (PERF_SURFACE.0 + 10.0, 100.0, 28.0, 28.0);
pub(super) fn strum_bypass_rect(expand: f32) -> Rect {
    let r = perf_surface_rect(expand);
    (r.0 + 10.0, STRUM_BYPASS.1, 28.0, 28.0)
}
// Octaves label and buttons share the header slot left of the touch latch.
pub(super) fn strum_octaves_rect(expand: f32) -> Rect {
    let latch = strum_latch_rect(expand);
    (latch.0 - 205.0, latch.1, 197.0, 28.0)
}
pub(super) fn octave_button_rect(expand: f32, i: usize) -> Rect {
    let slot = strum_octaves_rect(expand);
    (
        slot.0 + slot.2 - 4.0 * 28.0 - 3.0 * 4.0 + i as f32 * 32.0,
        slot.1,
        28.0,
        28.0,
    )
}
#[cfg(test)]
pub(super) fn octave_rect(i: usize) -> Rect {
    octave_button_rect(0.0, i)
}
// Arp page grid: four rows with a shared 20px rhythm inside ARP_PAD.
const ARP_X: f32 = ARP_PAD.0 + 8.0;
const ARP_INNER_W: f32 = ARP_PAD.2 - 16.0;
const ARP_GAP: f32 = 20.0;
const ARP_ROW1: f32 = ARP_PAD.1 + 8.0;
const ARP_ROW1_H: f32 = 38.0;
const ARP_ROW4_H: f32 = 38.0;
const ARP_ROW4: f32 = ARP_PAD.1 + ARP_PAD.3 - 8.0 - ARP_ROW4_H;
const ARP_ROW2: f32 = ARP_ROW1 + ARP_ROW1_H + ARP_GAP;
const PATTERN_HEIGHT: f32 = 128.0;
const ARP_ROW3: f32 = ARP_ROW2 + PATTERN_HEIGHT + ARP_GAP;
const ARP_ROW3_H: f32 = ARP_ROW4 - ARP_GAP - ARP_ROW3;
const REPEAT_W: f32 = 88.0;
const HOLD_W: f32 = 200.0;
pub(super) fn repeat_rect(looping: bool) -> Rect {
    (
        ARP_X + if looping { REPEAT_W + 8.0 } else { 0.0 },
        ARP_ROW1 + (ARP_ROW1_H - 28.0) * 0.5,
        REPEAT_W,
        28.0,
    )
}
pub(super) const HEADER_PLAY_TAB: Rect = (650.0, 20.0, 108.0, 38.0);
pub(super) const HEADER_SEQ_TAB: Rect = (766.0, 20.0, 164.0, 38.0);
#[cfg(test)]
pub(super) fn inversion_rect(i: usize) -> Rect {
    inversion_control_rect(false, i)
}
pub(super) fn inversion_control_rect(mpe: bool, i: usize) -> Rect {
    (
        chord_container(mpe).0 + 84.0 + i as f32 * 36.0,
        KEYBOARD_CONTROL_Y,
        28.0,
        28.0,
    )
}
const PATTERN_GAP: f32 = 8.0;
const PATTERN_WIDTH: f32 = (ARP_INNER_W - 6.0 * PATTERN_GAP) / 7.0;
pub(super) fn output_controls(mode: i32) -> Vec<(&'static str, Rect)> {
    let ids: &[&str] = match mode {
        1 => &["length_ms", "strings_played"],
        2 => &["length_ms"],
        _ => &[],
    };
    if ids.is_empty() { return Vec::new(); }
    let (x, y, row) = if mode == 2 {
        (PAD.0 + 8.0, 450.0, PAD.2 - 16.0)
    } else {
        (ARP_X, ARP_ROW4, ARP_INNER_W)
    };
    let gap = 12.0;
    let width = (row - (ids.len() - 1) as f32 * gap) / ids.len() as f32;
    ids.iter()
        .enumerate()
        .map(|(i, &id)| (id, (x + i as f32 * (width + gap), y, width, 38.0)))
        .collect()
}

pub(super) const ARP_RATES: [(&str, f32); 8] = [
    ("1/2", 2.0),
    ("1/4", 1.0),
    ("1/8", 0.5),
    ("1/16", 0.25),
    ("1/32", 0.125),
    ("1/64", 0.0625),
    ("1/8T", 1.0 / 3.0),
    ("1/16T", 1.0 / 6.0),
];
pub(super) fn pattern_rect(i: usize) -> Rect {
    (
        ARP_X + i as f32 * (PATTERN_WIDTH + PATTERN_GAP),
        ARP_ROW2,
        PATTERN_WIDTH,
        PATTERN_HEIGHT,
    )
}
// Humanize and Swing share the top row to leave the lower body for step editing.
pub(super) fn arp_controls(_looping: bool) -> [(&'static str, Rect); 2] {
    let left = ARP_X + 2.0 * REPEAT_W + 8.0 + 16.0;
    let width = (ARP_X + ARP_INNER_W - left - 12.0) * 0.5;
    [
        ("humanize", (left, ARP_ROW1, width, ARP_ROW1_H)),
        ("swing", (left + width + 12.0, ARP_ROW1, width, ARP_ROW1_H)),
    ]
}
pub(super) const MPE: Rect = (CHORD_CONTAINER.0 + 160.0, KEYBOARD_CONTROL_Y, 208.0, 28.0);
pub(super) const AFFECT_CHORDS: Rect =
    (MELODY_CONTAINER.0 + 116.0, KEYBOARD_CONTROL_Y, 112.0, 28.0);
#[cfg(test)]
pub(super) fn protocol_rect(i: usize) -> Rect {
    output_protocol_rect(false, i)
}
pub(super) const TEMPO_SYNC: Rect = (1373.0 - WIDTH_REDUCTION - OLD_MOD_COLUMN, 24.0, 126.0, 30.0);
pub(super) const TEMPO_CONTROL: Rect = (
    1507.0 - WIDTH_REDUCTION - OLD_MOD_COLUMN,
    TEMPO_SYNC.1,
    150.0,
    TEMPO_SYNC.3,
);
pub(super) const STRUM_SYNC: Rect = (PAD.0 + 8.0, 430.0, 130.0, 28.0);
pub(super) const STRUM_HOLD: Rect = (ARP_X + ARP_INNER_W - HOLD_W, ARP_ROW3, HOLD_W, ARP_ROW3_H);
pub(super) const EXPANDED_STRUM_HOLD: Rect =
    (1667.0 - WIDTH_REDUCTION - OLD_MOD_COLUMN, 116.0, 66.0, 28.0);
pub(super) const STRUM_LATCH: Rect = field_latch(PAD);
pub(super) const EXPANDED_STRUM_LATCH: Rect = field_latch(EXPANDED_PAD);

pub(super) fn mapping_summary_rect(i: usize) -> Rect {
    let r = meter_rect(7 + i);
    (r.0 + r.2 - 24.0, r.1, 24.0, 20.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Panel {
    Mapping,
    Routes,
}
impl Panel {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Routes => CHORDS_SURFACE,
            Self::Mapping => (760.0, 276.0, 488.0, 242.0),
        }
    }
}
pub(super) fn mapping_panel_rect(axis: usize) -> Rect {
    let summary = mapping_summary_rect(axis);
    let height = if axis == 0 { 104.0 } else { 180.0 };
    (
        (summary.0 + summary.2 - 384.0).clamp(16.0, W - 400.0),
        summary.1 - height - 6.0,
        384.0,
        height,
    )
}
pub(super) fn mapping_control_rect(axis: usize, id: &str) -> Rect {
    let r = mapping_panel_rect(axis);
    match id {
        "x_reverse" => (r.0 + 256.0, r.1 + 48.0, 116.0, 28.0),
        "y_reverse" => (r.0 + 200.0, r.1 + 132.0, 172.0, 28.0),
        "y_cc" => (r.0 + 200.0, r.1 + 84.0, 172.0, 42.0),
        _ => (r.0 + 12.0, r.1 + 132.0, 172.0, 42.0),
    }
}
pub(super) const TRANSPOSE_STEPS: [i32; 5] = [-12, -1, 0, 1, 12];
#[cfg(test)]
pub(super) fn transpose_rect(i: usize) -> Rect {
    transpose_control_rect(false, i)
}
pub(super) fn transpose_control_rect(_mpe: bool, i: usize) -> Rect {
    let widths = [46.0, 38.0, 104.0, 38.0, 46.0];
    (
        TRANSPOSE.0 + widths[..i].iter().sum::<f32>(),
        TRANSPOSE.1,
        widths[i],
        TRANSPOSE.3,
    )
}
pub(super) fn voicing_controls() -> [(&'static str, Rect); 0] {
    []
}
pub(super) fn meter_rect(i: usize) -> Rect {
    const INSET: f32 = 12.0;
    const GAP: f32 = 8.0;
    let count = crate::engine::routing::SOURCE_COUNT as f32;
    let height = (MOD_SURFACE.3 - MODULE_HEADER_H - 2.0 * INSET - (count - 1.0) * GAP) / count;
    (
        MOD_SURFACE.0 + INSET,
        MOD_SURFACE.1 + MODULE_HEADER_H + INSET + i as f32 * (height + GAP),
        MOD_SURFACE.2 - 2.0 * INSET,
        height,
    )
}

pub(super) const KEY_PITCH: f32 = 70.0;
pub(super) const TILE_WIDTH: f32 = KEY_PITCH - 4.0;
pub(super) const TILE_HEIGHT: f32 = 78.0;
const KEY_ROW_PITCH: f32 = TILE_HEIGHT + 8.0;
// Center all four tile rows in the body beneath the inline header.
const TILE_GROUP_HEIGHT: f32 = 3.0 * TILE_HEIGHT + 2.0 * 8.0 + 10.0 + TILE_HEIGHT;
const KEY_TOP: f32 = CHORDS_SURFACE.1
    + MODULE_HEADER_H
    + (CHORDS_SURFACE.3 - MODULE_HEADER_H - TILE_GROUP_HEIGHT) * 0.5;
pub(super) fn key_rect(index: usize) -> Rect {
    let row = index / KEY_COLUMNS;
    let col = index % KEY_COLUMNS;
    (
        32.0 + ([0.0, 0.25, 0.75][row] + col as f32) * KEY_PITCH,
        KEY_TOP + row as f32 * KEY_ROW_PITCH,
        TILE_WIDTH,
        TILE_HEIGHT,
    )
}
// Minimum readable size for hints and secondary labels at the native window size.
pub(super) const TEXT_SMALL: f32 = 11.0;
pub(super) const TEXT_LABEL: f32 = 12.0;
pub(super) fn memory_rect(i: usize) -> Rect {
    (
        32.0 + (1.25 + i as f32) * KEY_PITCH,
        KEY_TOP + 2.0 * KEY_ROW_PITCH + TILE_HEIGHT + 10.0,
        TILE_WIDTH,
        TILE_HEIGHT,
    )
}
pub(super) fn memory_delete_rect(i: usize) -> Rect {
    let r = memory_rect(i);
    (r.0 + r.2 - 22.0, r.1, 22.0, 22.0)
}

// A quarter-pad minimum span keeps even twelve strings usable.
pub(super) const STRUM_MIN_SPAN: f32 = 0.25;
pub(super) fn strum_bounds(min: f32, max: f32) -> (f32, f32) {
    let min = min.clamp(0.0, 1.0 - STRUM_MIN_SPAN);
    (min, max.clamp(min + STRUM_MIN_SPAN, 1.0))
}
#[cfg(test)]
pub(super) fn strum_bound_rect(value: f32) -> Rect {
    let (x, y, pointer) = strum_bound_anchor(false, value);
    pleasant_ui::tag_hit_rect(x, y, pointer)
}

pub(super) fn expression_bounds(min: f32, max: f32) -> (f32, f32) {
    let min = min.clamp(0.0, 0.99);
    (min, max.clamp(min + 0.01, 1.0))
}

pub(super) fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
        a.3 + (b.3 - a.3) * t,
    )
}

pub(super) fn pad_rect(expand: f32) -> Rect {
    lerp_rect(PAD, EXPANDED_PAD, expand)
}

pub(super) fn play_pad_rect(expand: f32) -> Rect {
    lerp_rect(PLAY_PAD, EXPANDED_PLAY_PAD, expand)
}

pub(super) fn auto_field_rect(expand: f32) -> Rect {
    lerp_rect(AUTO_FIELD, EXPANDED_PLAY_PAD, expand)
}

pub(super) fn field_rect(expand: f32, manual: bool) -> Rect {
    if manual {
        play_pad_rect(expand)
    } else {
        auto_field_rect(expand)
    }
}

pub(super) fn expand_rect(expand: f32) -> Rect {
    lerp_rect(EXPAND, EXPANDED_EXPAND, expand)
}

pub(super) fn strum_sync_rect(expand: f32) -> Rect {
    lerp_rect(STRUM_SYNC, EXPANDED_STRUM_SYNC, expand)
}

pub(super) fn strum_hold_rect(expand: f32) -> Rect {
    lerp_rect(STRUM_HOLD, EXPANDED_STRUM_HOLD, expand)
}

pub(super) fn strum_latch_rect(expand: f32) -> Rect {
    lerp_rect(STRUM_LATCH, EXPANDED_STRUM_LATCH, expand)
}

pub(super) fn strum_rate_rect(expand: f32) -> Rect {
    lerp_rect(STRUM_RATE, EXPANDED_STRUM_RATE, expand)
}

pub(super) fn perf_surface_rect(expand: f32) -> Rect {
    lerp_rect(PERF_SURFACE, EXPANDED_SURFACE, expand)
}

pub(super) fn shrink_width(rect: Rect, expand: f32) -> Rect {
    (rect.0, rect.1, rect.2 * (1.0 - expand), rect.3)
}

#[cfg(test)]
pub(super) fn strum_bound_anchor(y_axis: bool, value: f32) -> (f32, f32, pleasant_ui::TagPointer) {
    strum_bound_anchor_in(PLAY_PAD, y_axis, value)
}

pub(super) fn strum_bound_anchor_in(
    field: Rect,
    y_axis: bool,
    value: f32,
) -> (f32, f32, pleasant_ui::TagPointer) {
    if y_axis {
        (
            field.0,
            field.1 + (1.0 - value) * field.3,
            pleasant_ui::TagPointer::Right,
        )
    } else {
        (
            field.0 + value * field.2,
            field.1,
            pleasant_ui::TagPointer::Down,
        )
    }
}

pub(super) fn route_slot_rect(i: usize) -> Rect {
    (
        32.0 + (i % 6) as f32 * 150.0,
        144.0 + (i / 6) as f32 * 30.0,
        144.0,
        26.0,
    )
}
pub(super) const ROUTE_SOURCE: Rect = (32.0, 240.0, 400.0, 28.0);
pub(super) const ROUTE_TARGET: Rect = (508.0, 240.0, 416.0, 28.0);
pub(super) const ROUTE_ENABLED: Rect = (752.0, 100.0, 126.0, 28.0);
pub(super) const ROUTE_CLEAR: Rect = (674.0, 100.0, 68.0, 28.0);
pub(super) const ROUTE_GRAPH: Rect = (44.0, 288.0, 868.0, 158.0);
pub(super) const ROUTE_LINEAR: Rect = (832.0, 466.0, 92.0, 28.0);
pub(super) fn route_control_rect(id: &str) -> Rect {
    if id == "route_smoothing_ms" { (456.0, 100.0, 208.0, 28.0) } else { ROUTE_ENABLED }
}

pub(super) const PIANO_SURFACE: Rect = (16.0, 522.0, MAIN_W - 32.0, 200.0);
pub(super) const SEQ_BYPASS: Rect = (PIANO_SURFACE.0 + 10.0, PIANO_SURFACE.1 + 10.0, 24.0, 24.0);
// Arp rate lives at the right end of the sequencer header; the free-rate slider shares the slot.
pub(super) const SEQ_RATE: Rect = (
    PIANO_SURFACE.0 + PIANO_SURFACE.2 - 16.0 - 240.0,
    PIANO_SURFACE.1 + 8.0,
    240.0,
    28.0,
);
pub(super) const SEQ_RATE_SYNC: Rect = (SEQ_RATE.0 - 8.0 - 104.0, SEQ_RATE.1, 104.0, 28.0);
const KEYBOARD_MODULE_W: f32 = (MAIN_W - 64.0 - 24.0) / 3.0;
const KEYBOARD_CONTROL_Y: f32 = 674.0;
pub(super) const BASS_CONTAINER: Rect = (32.0, 666.0, KEYBOARD_MODULE_W, 44.0);
pub(super) const CHORD_CONTAINER: Rect = (44.0 + KEYBOARD_MODULE_W, 666.0, KEYBOARD_MODULE_W, 44.0);
pub(super) const MELODY_CONTAINER: Rect = (
    56.0 + KEYBOARD_MODULE_W * 2.0,
    666.0,
    KEYBOARD_MODULE_W,
    44.0,
);
pub(super) const BASS_BYPASS: Rect = (42.0, 676.0, 24.0, 24.0);
pub(super) const ALWAYS_BASS: Rect = (BASS_CONTAINER.0 + 116.0, KEYBOARD_CONTROL_Y, 112.0, 28.0);
pub(super) const SPLIT_NOTE: Rect = (MELODY_CONTAINER.0 + 236.0, KEYBOARD_CONTROL_Y, 112.0, 28.0);
pub(super) const PIANO_KEYS: Rect = (32.0, 594.0, MAIN_W - 64.0, 60.0);
pub(super) const PIANO_SPLITS: Rect = (32.0, 566.0, MAIN_W - 64.0, 24.0);
pub(super) const PIANO_LEADING: Rect = (32.0, 532.0, MAIN_W - 64.0, 28.0);
pub(super) const ARP_PAGE_ACCENT: Rect = (520.0, 100.0, 204.0, 28.0);
pub(super) const INTERLOCK: Rect = (732.0, 100.0, 192.0, 28.0);

pub(super) fn piano_black(note: u8) -> bool {
    matches!(note % 12, 1 | 3 | 6 | 8 | 10)
}

pub(super) fn piano_key_rect(note: u8) -> Rect {
    // MIDI 0..127 has 75 natural keys. Accidentals share the boundary between them.
    let before = (0..note).filter(|&n| !piano_black(n)).count() as f32;
    let width = PIANO_KEYS.2 / 75.0;
    if piano_black(note) {
        (
            PIANO_KEYS.0 + (before - 0.31) * width,
            PIANO_KEYS.1,
            width * 0.62,
            PIANO_KEYS.3 * 0.62,
        )
    } else {
        (
            PIANO_KEYS.0 + before * width,
            PIANO_KEYS.1,
            width,
            PIANO_KEYS.3,
        )
    }
}

pub(super) fn piano_note_x(note: u8) -> f32 {
    let r = piano_key_rect(note);
    r.0 + r.2 / 2.0
}

pub(super) fn piano_note_at(x: f32, y: f32) -> Option<u8> {
    if !super::hit(PIANO_KEYS, x, y) {
        return None;
    }
    (0..128u8)
        .filter(|&n| piano_black(n))
        .chain((0..128u8).filter(|&n| !piano_black(n)))
        .find(|&n| super::hit(piano_key_rect(n), x, y))
}

pub(super) fn piano_nearest_note(x: f32) -> u8 {
    (0..128u8)
        .min_by(|&a, &b| {
            (piano_note_x(a) - x)
                .abs()
                .total_cmp(&(piano_note_x(b) - x).abs())
        })
        .unwrap_or(60)
}

// Always-visible MIDI settings inside the three keyboard submodules.
// Bass container (GOLD, left).
// Chord container (center).
// Melody container (TEAL, right).
pub(super) fn melody_container(_mpe: bool) -> Rect {
    MELODY_CONTAINER
}
pub(super) fn chord_container(_mpe: bool) -> Rect {
    CHORD_CONTAINER
}
pub(super) fn melody_split_rect(_mpe: bool) -> Rect {
    SPLIT_NOTE
}
pub(super) fn affect_chords_rect(_mpe: bool) -> Rect {
    AFFECT_CHORDS
}
pub(super) fn key_split_rect(mpe: bool) -> Rect {
    let r = melody_container(mpe);
    (r.0 + 10.0, module_header_mid(r.1) - 12.0, 24.0, 24.0)
}
fn keyboard_channel_rect(r: Rect) -> Rect {
    (r.0 + r.2 - 76.0, module_header_mid(r.1) - 14.0, 64.0, 28.0)
}
pub(super) fn output_protocol_rect(_mpe: bool, _i: usize) -> Rect {
    MPE
}
pub(super) fn keyboard_control_rect(id: &str, mpe: bool) -> Rect {
    match id {
        "bass_channel" => keyboard_channel_rect(BASS_CONTAINER),
        "bass_split" => (BASS_CONTAINER.0 + 236.0, KEYBOARD_CONTROL_Y, 112.0, 28.0),
        "output_channel" => keyboard_channel_rect(chord_container(mpe)),
        "upper_channel" => keyboard_channel_rect(melody_container(mpe)),
        "upper" => (MELODY_CONTAINER.0 + 356.0, KEYBOARD_CONTROL_Y, 64.0, 28.0),
        "members" => keyboard_channel_rect(melody_container(mpe)),
        _ => unreachable!("unknown keyboard control"),
    }
}

pub(super) fn keyboard_menu_range(id: &str) -> Option<(i32, i32)> {
    match id {
        "bass_channel" | "output_channel" | "upper_channel" => Some((1, 16)),
        "members" => Some((1, 15)),
        _ => None,
    }
}

// Chord names share the header and keep enough room for extensions and slash bass.
pub(super) fn chord_readout_rect() -> Rect {
    (202.0, 18.0, HEADER_PLAY_TAB.0 - 218.0, 40.0)
}

pub(super) const METER_NUM: Rect = (1007.0, 24.0, 44.0, 30.0);
pub(super) const METER_DEN: Rect = (1067.0, 24.0, 44.0, 30.0);

/// Labels sit above/below their tags. Flip sides when another label or tag
/// occupies that space, including when the Y bounds nearly meet at the top.
pub(super) fn strum_bound_labels(pad: Rect, field: Rect, x: (f32, f32), y: (f32, f32)) -> [Rect; 4] {
    let anchors = [(false, x.0), (false, x.1), (true, y.0), (true, y.1)]
        .map(|(axis, value)| strum_bound_anchor_in(field, axis, value));
    let handles = anchors.map(|(x, y, pointer)| pleasant_ui::handles::tag_body_rect(x, y, pointer));
    let candidates: [[Rect; 2]; 4] = std::array::from_fn(|i| {
        let (ax, ay, _) = anchors[i];
        let body = handles[i];
        let center_x = if i < 2 { ax } else { body.0 + body.2 * 0.5 - 2.0 };
        let above = (center_x - 14.0, body.1 - 14.0, 28.0, 12.0);
        let below = (center_x - 14.0, if i < 2 { ay + 4.0 } else { body.1 + body.3 + 4.0 }, 28.0, 12.0);
        if i == 3 { [above, below] } else { [below, above] }
    });
    let overlaps = |a: Rect, b: Rect| {
        a.0 < b.0 + b.2 + 2.0 && a.0 + a.2 + 2.0 > b.0
            && a.1 < b.1 + b.3 + 2.0 && a.1 + a.3 + 2.0 > b.1
    };
    let mut best = candidates.map(|sides| sides[0]);
    let mut best_score = u32::MAX;
    for flips in 0u32..16 {
        let labels: [Rect; 4] = std::array::from_fn(|i| candidates[i][((flips >> i) & 1) as usize]);
        let mut score = flips.count_ones();
        for (i, r) in labels.iter().copied().enumerate() {
            if r.0 < pad.0 || r.1 < pad.1 || r.0 + r.2 > pad.0 + pad.2 || r.1 + r.3 > pad.1 + pad.3 {
                score += 1000;
            }
            score += 16 * handles.iter().filter(|&&handle| overlaps(r, handle)).count() as u32;
            score += 16 * labels[..i].iter().filter(|&&label| overlaps(r, label)).count() as u32;
        }
        if score < best_score {
            best = labels;
            best_score = score;
        }
    }
    best
}
