// Shared geometry for rendering and event hit-testing.
use super::KEY_COLUMNS;
const WIDTH_REDUCTION: f32 = 128.0;
const OLD_MOD_COLUMN: f32 = 122.0;
pub(super) const W: f32 = 1871.0 - WIDTH_REDUCTION - OLD_MOD_COLUMN;
pub(super) const H: f32 = 840.0;
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
    (pad.0 + pad.2 - 34.0, pad.1 + 11.0, 22.0, 22.0)
}
const fn field_latch(pad: Rect) -> Rect {
    let corner = field_corner(pad);
    (corner.0 - 96.0, pad.1 + 8.0, 88.0, 28.0)
}
const fn manual_field_inset(pad: Rect) -> Rect {
    (pad.0 + 60.0, pad.1 + 75.0, pad.2 - 80.0, pad.3 - 140.0)
}
pub(super) const PLAY_PAD: Rect = manual_field_inset(PAD);
pub(super) const AUTO_FIELD: Rect = (PAD.0 + 20.0, 286.0, PAD.2 - 40.0, 100.0);
pub(super) const CHORDS_SURFACE: Rect = (16.0, 92.0, 924.0, 418.0);
pub(super) const PERF_SURFACE: Rect = (1080.0 - WIDTH_REDUCTION, 92.0, 653.0, 418.0);
// Full-width modulation row between performance and the keyboard.
pub(super) const MOD_SURFACE: Rect = (16.0, 522.0, W - 32.0, 88.0);
pub(super) const EXPAND: Rect = field_corner(PAD);
pub(super) const EXPANDED_SURFACE: Rect = (16.0, 92.0, W - 32.0, 418.0);
pub(super) const EXPANDED_PAD: Rect = (32.0, 108.0, W - 64.0, 390.0);
pub(super) const EXPANDED_PLAY_PAD: Rect = manual_field_inset(EXPANDED_PAD);
pub(super) const EXPANDED_EXPAND: Rect = field_corner(EXPANDED_PAD);
pub(super) const EXPANDED_STRUM_SYNC: Rect =
    (1567.0 - WIDTH_REDUCTION - OLD_MOD_COLUMN, 116.0, 92.0, 28.0);
pub(super) const STRUM_RATE: Rect = (PAD.0 + 8.0, 344.0, PAD.2 - 16.0, 36.0);
pub(super) const ARP_STRINGS: Rect = (PAD.0 + 8.0, 242.0, PAD.2 - 16.0, 76.0);
pub(super) fn arp_string_pos(r: Rect, index: usize, count: usize) -> (f32, f32, f32) {
    let count = count.max(1);
    let rows = count.div_ceil(18);
    let columns = count.div_ceil(rows);
    let handle_h = 22.0;
    let usable_h = r.3 - handle_h;
    let height = usable_h / rows as f32;
    let col = index % columns;
    let row = index / columns;
    let x = r.0 + col as f32 * (r.2 - 32.0) / (columns - 1).max(1) as f32 + 16.0;
    let y = r.1 + handle_h + row as f32 * height + 1.0;
    let bottom = y + height - 19.0;
    (x, y, bottom)
}
pub(super) fn arp_string_index_at(r: Rect, x: f32, y: f32, count: usize) -> usize {
    let count = count.max(1);
    let rows = count.div_ceil(18);
    let columns = count.div_ceil(rows);
    let handle_h = 22.0;
    let usable_h = r.3 - handle_h;
    let height = usable_h / rows as f32;
    let row = if rows > 1 {
        ((y - (r.1 + handle_h)) / height).floor().clamp(0.0, (rows - 1) as f32) as usize
    } else {
        0
    };
    if columns <= 1 {
        return (row * columns).min(count.saturating_sub(1));
    }
    let col_w = (r.2 - 32.0) / (columns - 1) as f32;
    let rel_x = x - (r.0 + 16.0);
    let col = (rel_x / col_w).round().clamp(0.0, (columns - 1) as f32) as usize;
    (row * columns + col).min(count.saturating_sub(1))
}
pub(super) const RATE_SYNC: Rect = (PAD.0 + PAD.2 - 156.0, 320.0, 148.0, 24.0);
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
pub(super) const ORDER: Rect = (462.0, 100.0, 152.0, 28.0);
pub(super) const TRANSPOSE: Rect = (622.0, 100.0, 302.0, 28.0);
pub(super) const APPEARANCE: Rect = (1681.0 - WIDTH_REDUCTION - OLD_MOD_COLUMN, 24.0, 138.0, 30.0);
pub(super) const MODE_LABELS: [&str; 2] = ["Arpeggiator", "Manual Strum"];
pub(super) fn repeat_rect(looping: bool) -> Rect {
    (PAD.0 + if looping { 78.0 } else { 8.0 }, 144.0, 66.0, 28.0)
}
pub(super) fn mode_rect(i: usize) -> Rect {
    (
        PERF_SURFACE.0 + 130.0 + i as f32 * 148.0,
        100.0,
        144.0,
        28.0,
    )
}
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
const PERFORMANCE_CONTROL_WIDTH: f32 = PAD.2 - 16.0;
const PATTERN_WIDTH: f32 = (PERFORMANCE_CONTROL_WIDTH - 6.0 * 6.0) / 7.0;
const PATTERN_HEIGHT: f32 = 48.0;
pub(super) const LOOP_RESET_WIDTH: f32 = 64.0;
pub(super) fn loop_reset_rect() -> Rect {
    (PAD.0 + PAD.2 - 8.0 - LOOP_RESET_WIDTH, 450.0, LOOP_RESET_WIDTH, 38.0)
}
pub(super) fn output_controls(mode: i32) -> Vec<(&'static str, Rect)> {
    let ids: &[&str] = match mode {
        1 => &["velocity", "length_ms"],
        2 => &["velocity", "length_ms", "strings"],
        3 => &["velocity", "loop_start", "loop_end"],
        _ => &["velocity"],
    };
    let total_w = if mode == 3 {
        PAD.2 - 16.0 - LOOP_RESET_WIDTH - 8.0
    } else {
        PAD.2 - 16.0
    };
    let width = (total_w - (ids.len() - 1) as f32 * 12.0) / ids.len() as f32;
    ids.iter()
        .enumerate()
        .map(|(i, &id)| {
            (
                id,
                (PAD.0 + 8.0 + i as f32 * (width + 12.0), 450.0, width, 38.0),
            )
        })
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
        PAD.0 + 8.0 + i as f32 * (PATTERN_WIDTH + 6.0),
        190.0,
        PATTERN_WIDTH,
        PATTERN_HEIGHT,
    )
}
pub(super) fn rate_rect(i: usize) -> Rect {
    let width = (PERFORMANCE_CONTROL_WIDTH - 7.0 * 6.0) / 8.0;
    (PAD.0 + 8.0 + i as f32 * (width + 6.0), 348.0, width, 30.0)
}
pub(super) const RATE_HEADER: Rect = (PAD.0 + 8.0, 320.0, PERFORMANCE_CONTROL_WIDTH - 156.0, 20.0);
pub(super) const OCTAVE_LABEL: (f32, f32) = (PAD.0 + 156.0, 163.0);
pub(super) fn octave_rect(i: usize) -> Rect {
    (PAD.0 + 210.0 + i as f32 * 32.0, 144.0, 28.0, 28.0)
}
pub(super) fn arp_controls() -> [(&'static str, Rect); 3] {
    [
        ("humanize", (PAD.0 + 350.0, 144.0, PAD.2 - 366.0, 38.0)),
        (
            "gate",
            (
                PAD.0 + 8.0 + PERFORMANCE_CONTROL_WIDTH * 0.5 + 4.0,
                388.0,
                PERFORMANCE_CONTROL_WIDTH * 0.5 - 4.0,
                50.0,
            ),
        ),
        (
            "swing",
            (
                PAD.0 + 8.0,
                388.0,
                PERFORMANCE_CONTROL_WIDTH * 0.5 - 4.0,
                50.0,
            ),
        ),
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
pub(super) const STRUM_HOLD: Rect = (
    PAD.0 + 8.0 + PERFORMANCE_CONTROL_WIDTH * 0.5 + 4.0,
    398.0,
    PERFORMANCE_CONTROL_WIDTH * 0.5 - 4.0,
    28.0,
);
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
    Sound,
}
impl Panel {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Routes | Self::Sound => CHORDS_SURFACE,
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
    let widths = [48.0, 40.0, 126.0, 40.0, 48.0];
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
    let gap = 8.0;
    let width = (MOD_SURFACE.2 - 32.0 - gap * 8.0) / 9.0;
    (
        MOD_SURFACE.0 + 16.0 + i as f32 * (width + gap),
        MOD_SURFACE.1 + MODULE_HEADER_H,
        width,
        30.0,
    )
}
pub(super) fn strum_controls() -> [(&'static str, Rect); 1] {
    [("strum_ms", STRUM_RATE)]
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

pub(super) const ROOT_ON_SELECT: Rect = (STRUM_LATCH.0 - 205.0, STRUM_LATCH.1, 197.0, 28.0);
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
pub(super) fn route_control_rect(_id: &str) -> Rect {
    ROUTE_ENABLED
}

pub(super) const PIANO_SURFACE: Rect = (16.0, 622.0, W - 32.0, 200.0);
const KEYBOARD_MODULE_W: f32 = (W - 64.0 - 24.0) / 3.0;
pub(super) const KEYBOARD_CONTROL_Y: f32 = 774.0;
pub(super) const BASS_CONTAINER: Rect = (32.0, 766.0, KEYBOARD_MODULE_W, 44.0);
pub(super) const CHORD_CONTAINER: Rect = (44.0 + KEYBOARD_MODULE_W, 766.0, KEYBOARD_MODULE_W, 44.0);
pub(super) const MELODY_CONTAINER: Rect = (
    56.0 + KEYBOARD_MODULE_W * 2.0,
    766.0,
    KEYBOARD_MODULE_W,
    44.0,
);
pub(super) const BASS_BYPASS: Rect = (42.0, 776.0, 24.0, 24.0);
pub(super) const ALWAYS_BASS: Rect = (BASS_CONTAINER.0 + 116.0, KEYBOARD_CONTROL_Y, 112.0, 28.0);
pub(super) const SPLIT_NOTE: Rect = (MELODY_CONTAINER.0 + 236.0, KEYBOARD_CONTROL_Y, 100.0, 28.0);
pub(super) const PIANO_KEYS: Rect = (32.0, 694.0, W - 64.0, 60.0);
pub(super) const PIANO_SPLITS: Rect = (32.0, 666.0, W - 64.0, 24.0);
pub(super) const PIANO_LEADING: Rect = (32.0, 632.0, W - 64.0, 28.0);

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

pub(super) const PIANO_ARP_LOOP_RAIL: Rect = (PIANO_KEYS.0, 682.0, PIANO_KEYS.2, 14.0);

pub(super) fn piano_key_badge_rect(note: u8) -> Rect {
    let r = piano_key_rect(note);
    if piano_black(note) {
        (r.0 + 1.0, r.1 + r.3 - 14.0, (r.2 - 2.0).max(1.0), 13.0)
    } else {
        (r.0 + 2.0, r.1 + r.3 - 20.0, (r.2 - 4.0).max(1.0), 18.0)
    }
}

pub(super) fn piano_nearest_chord_string(x: f32, notes: &[u8], count: usize) -> Option<usize> {
    if notes.is_empty() || count == 0 {
        return None;
    }
    let max_count = count.min(32);
    (0..max_count).min_by(|&a, &b| {
        let na = (notes[a % notes.len()] as usize + (a / notes.len()) * 12).min(127) as u8;
        let nb = (notes[b % notes.len()] as usize + (b / notes.len()) * 12).min(127) as u8;
        let xa = piano_note_x(na);
        let xb = piano_note_x(nb);
        (xa - x).abs().total_cmp(&(xb - x).abs())
    })
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
pub(super) fn output_protocol_rect(_mpe: bool, i: usize) -> Rect {
    (MPE.0 + i as f32 * 72.0, MPE.1, 64.0, MPE.3)
}
pub(super) fn keyboard_control_rect(id: &str, mpe: bool) -> Rect {
    match id {
        "bass_channel" => keyboard_channel_rect(BASS_CONTAINER),
        "bass_split" => (BASS_CONTAINER.0 + 236.0, KEYBOARD_CONTROL_Y, 100.0, 28.0),
        "output_channel" => keyboard_channel_rect(chord_container(mpe)),
        "upper_channel" => keyboard_channel_rect(melody_container(mpe)),
        "comp_channel" => (melody_container(mpe).0 + 364.0, KEYBOARD_CONTROL_Y, 53.0, 28.0),
        "upper" => (MELODY_CONTAINER.0 + 344.0, KEYBOARD_CONTROL_Y, 76.0, 28.0),
        "members" => keyboard_channel_rect(melody_container(mpe)),
        _ => unreachable!("unknown keyboard control"),
    }
}

pub(super) fn keyboard_menu_range(id: &str) -> Option<(i32, i32)> {
    match id {
        "bass_channel" | "output_channel" | "upper_channel" | "comp_channel" => Some((1, 16)),
        "members" => Some((1, 15)),
        _ => None,
    }
}

pub(super) fn arp_accent_step_pos(r: Rect, step: usize, cycle: usize) -> (f32, f32) {
    let cycle = cycle.max(1);
    let start_x = r.0 + 16.0;
    let width = r.2 - 32.0;
    let x = if cycle == 1 {
        start_x + width * 0.5
    } else {
        start_x + step as f32 * width / (cycle - 1) as f32
    };
    let y = r.1 + 17.0;
    (x, y)
}

pub(super) fn arp_accent_step_at(r: Rect, x: f32, cycle: usize) -> usize {
    let cycle = cycle.max(1);
    if cycle == 1 {
        return 0;
    }
    let start_x = r.0 + 16.0;
    let width = r.2 - 32.0;
    let rel = (x - start_x) / width;
    ((rel * (cycle - 1) as f32).round() as isize).clamp(0, (cycle - 1) as isize) as usize
}

pub(super) fn routing_preset_rect(mpe: bool) -> Rect {
    (chord_container(mpe).0 + 372.0, KEYBOARD_CONTROL_Y, 58.0, 28.0)
}

pub(super) fn comp_mode_rect(mpe: bool) -> Rect {
    (melody_container(mpe).0 + 40.0, KEYBOARD_CONTROL_Y, 72.0, 28.0)
}

pub(super) fn comp_octave_rect(mpe: bool) -> Rect {
    (melody_container(mpe).0 + 116.0, KEYBOARD_CONTROL_Y, 46.0, 28.0)
}

pub(super) fn comp_rhythm_rect(mpe: bool) -> Rect {
    (melody_container(mpe).0 + 166.0, KEYBOARD_CONTROL_Y, 94.0, 28.0)
}

pub(super) fn comp_lag_rect(mpe: bool) -> Rect {
    (melody_container(mpe).0 + 264.0, KEYBOARD_CONTROL_Y, 44.0, 28.0)
}

pub(super) fn comp_interlock_rect(mpe: bool) -> Rect {
    (melody_container(mpe).0 + 312.0, KEYBOARD_CONTROL_Y, 48.0, 28.0)
}

pub(super) fn bass_octave_rect() -> Rect {
    (BASS_CONTAINER.0 + 344.0, KEYBOARD_CONTROL_Y, 64.0, 28.0)
}

// Keep the chord readout and its tooltip beside the memory row.
pub(super) fn chord_readout_rect() -> Rect {
    let x = memory_rect(super::MEMORY_COUNT - 1).0 + TILE_WIDTH + 16.0;
    (
        x,
        memory_rect(0).1,
        CHORDS_SURFACE.0 + CHORDS_SURFACE.2 - 16.0 - x,
        TILE_HEIGHT,
    )
}

pub(super) fn performance_page(mode: i32) -> i32 {
    match mode {
        3 => 1,
        _ => mode.max(1),
    }
}
