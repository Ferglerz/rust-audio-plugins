// Shared geometry for rendering and event hit-testing.
use super::KEY_COLUMNS;
pub(super) const W: f32 = 1120.0;
pub(super) const H: f32 = 856.0;
pub(super) type Rect = (f32, f32, f32, f32);
pub(super) const HEADER_H: f32 = 76.0;
pub(super) const PAD: Rect = (600.0, 158.0, 488.0, 310.0);
// Keep drawing and pointer targets on the same performance surface.
pub(super) const PLAY_PAD: Rect = (660.0, 233.0, 408.0, 205.0);
pub(super) const AUTO_FIELD: Rect = (620.0, 203.0, 448.0, 115.0);
pub(super) const CHORDS_SURFACE: Rect = (16.0, 92.0, 548.0, 436.0);
pub(super) const VOICING_SURFACE: Rect = (16.0, 536.0, 548.0, 138.0);
pub(super) const PERF_SURFACE: Rect = (584.0, 92.0, 520.0, 484.0);
pub(super) const METERS_SURFACE: Rect = (584.0, 596.0, 520.0, 78.0);
pub(super) const EXPAND: Rect = (808.0, 162.0, 88.0, 28.0);
pub(super) const EXPANDED_SURFACE: Rect = (16.0, 92.0, 1088.0, 580.0);
pub(super) const EXPANDED_PAD: Rect = (32.0, 108.0, 1056.0, 548.0);
pub(super) const EXPANDED_PLAY_PAD: Rect = (80.0, 168.0, 960.0, 456.0);
pub(super) const EXPANDED_EXPAND: Rect = (988.0, 116.0, 88.0, 28.0);
pub(super) const EXPANDED_STRUM_SYNC: Rect = (816.0, 116.0, 164.0, 28.0);
pub(super) const STRUM_RATE: Rect = (852.0, 342.0, 220.0, 42.0);
pub(super) const EXPANDED_STRUM_RATE: Rect = (588.0, 116.0, 220.0, 28.0);
pub(super) const LATCH: Rect = (462.0, 154.0, 86.0, 28.0);
pub(super) const QWERTY: Rect = (432.0, 190.0, 116.0, 28.0);
pub(super) const ORDER: Rect = (312.0, 190.0, 112.0, 28.0);
pub(super) const APPEARANCE: Rect = (950.0, 24.0, 138.0, 30.0);
pub(super) const SAVE_MEMORY: Rect = (416.0, 442.0, 132.0, 28.0);
pub(super) const QUALITY: Rect = (32.0, 564.0, 176.0, 32.0);
pub(super) const MODE_LABELS: [&str; 3] = ["Auto Strum", "Manual Strum", "Arpeggiator"];
pub(super) fn mode_rect(i: usize) -> Rect {
    (600.0 + i as f32 * 164.0, 122.0, 160.0, 28.0)
}
pub(super) fn inversion_rect(i: usize) -> Rect {
    (32.0 + i as f32 * 38.0, 630.0, 32.0, 28.0)
}
pub(super) fn direction_rect(i: usize) -> Rect {
    (616.0 + i as f32 * 76.0, 366.0, 70.0, 28.0)
}
pub(super) fn output_controls(mode: i32) -> Vec<(&'static str, Rect)> {
    if mode == 1 {
        return ["velocity", "length_ms", "strings", "strings_played"]
            .into_iter()
            .enumerate()
            .map(|(i, id)| {
                (
                    id,
                    (
                        600.0 + i as f32 * 124.0,
                        482.0,
                        116.0,
                        36.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
                    ),
                )
            })
            .collect();
    }
    if !matches!(mode, 1 | 2) {
        return vec![(
            "velocity",
            (
                600.0,
                482.0,
                488.0,
                36.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        )];
    }
    vec![
        (
            "velocity",
            (
                600.0,
                482.0,
                156.0,
                36.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        ),
        (
            "length_ms",
            (
                766.0,
                482.0,
                156.0,
                36.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        ),
        (
            "strings",
            (
                932.0,
                482.0,
                156.0,
                36.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        ),
    ]
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
    (618.0 + i as f32 * 92.0, 194.0, 84.0, 42.0)
}
pub(super) fn rate_rect(i: usize) -> Rect {
    (618.0 + i as f32 * 58.0, 262.0, 46.0, 24.0)
}
pub(super) const RATE_HEADER: Rect = (618.0, 240.0, 452.0, 20.0);
pub(super) fn octave_rect(i: usize) -> Rect {
    (690.0 + i as f32 * 34.0, 302.0, 28.0, 26.0)
}
pub(super) fn arp_controls() -> [(&'static str, Rect); 3] {
    [
        (
            "humanize",
            (
                844.0,
                298.0,
                194.0,
                34.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        ),
        (
            "gate",
            (
                618.0,
                344.0,
                452.0,
                54.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        ),
        (
            "swing",
            (
                618.0,
                404.0,
                452.0,
                54.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        ),
    ]
}
pub(super) const MPE: Rect = (790.0, 94.0, 190.0, 24.0);
pub(super) const TEMPO_SYNC: Rect = (600.0, 24.0, 108.0, 24.0);
pub(super) const TEMPO_CONTROL: Rect = (
    716.0,
    18.0,
    220.0,
    42.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
);
pub(super) const STRUM_SYNC: Rect = (904.0, 162.0, 164.0, 28.0);
pub(super) const STRUM_LATCH: Rect = (1000.0, 162.0, 88.0, 28.0);
pub(super) const EXPANDED_STRUM_LATCH: Rect = (892.0, 116.0, 88.0, 28.0);
pub(super) const OUTPUT: Rect = (988.0, 94.0, 100.0, 24.0);
pub(super) const LEARN_OCTAVE: Rect = (344.0, 102.0, 204.0, 28.0);
pub(super) fn mapping_summary_rect(i: usize) -> Rect {
    (600.0 + i as f32 * 248.0, 524.0, 240.0, 40.0)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Panel {
    Mapping,
    Output,
    Routes,
}
impl Panel {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Routes => (680.0, 218.0, 408.0, 300.0),
            Self::Mapping => (600.0, 276.0, 488.0, 242.0),
            Self::Output => (600.0, 122.0, 488.0, 224.0),
        }
    }
}
pub(super) fn local_control_rect(panel: Panel, index: usize) -> Rect {
    let r = panel.rect();
    let width = (r.2 - 40.0) / 2.0;
    (
        r.0 + 12.0 + (index % 2) as f32 * (width + 16.0),
        r.1 + 36.0 + (index / 2) as f32 * 52.0,
        width,
        42.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
    )
}
pub(super) fn mapping_panel_rect(axis: usize) -> Rect {
    let summary = mapping_summary_rect(axis);
    let height = if axis == 0 { 104.0 } else { 180.0 };
    (
        summary.0.min(704.0),
        summary.1 - height - 6.0,
        384.0,
        height,
    )
}
pub(super) fn mapping_learn_rect(axis: usize) -> Rect {
    let r = mapping_panel_rect(axis);
    (r.0 + 228.0, r.1 + 8.0, 108.0, 28.0)
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
pub(super) fn transpose_rect(i: usize) -> Rect {
    (326.0 + i as f32 * 45.0, 630.0, 42.0, 28.0)
}
pub(super) fn voicing_controls() -> [(&'static str, Rect); 3] {
    [
        ("quality", QUALITY),
        ("spread", (400.0, 564.0, 148.0, 32.0)),
        ("voice_leading", (218.0, 564.0, 166.0, 32.0)),
    ]
}
pub(super) fn meter_rect(i: usize) -> Rect {
    (600.0 + i as f32 * 54.0, 627.0, 48.0, 42.0)
}
pub(super) fn strum_controls() -> [(&'static str, Rect); 2] {
    [
        (
            "strum_ms",
            (
                852.0,
                342.0,
                220.0,
                42.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        ),
        (
            "contour",
            (
                616.0,
                404.0,
                456.0,
                42.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        ),
    ]
}
pub(super) fn key_rect(index: usize) -> Rect {
    let row = index / KEY_COLUMNS;
    let col = index % KEY_COLUMNS;
    (
        32.0 + ([0.0, 0.25, 0.75][row] + col as f32) * 40.5,
        226.0 + row as f32 * 64.0,
        36.5,
        57.0,
    )
}
// Minimum readable size for hints and secondary labels at the native window size.
pub(super) const TEXT_SMALL: f32 = 11.0;
pub(super) const TEXT_LABEL: f32 = 12.0;
pub(super) fn memory_rect(i: usize) -> Rect {
    (32.0 + (1.25 + i as f32) * 40.5, 426.0, 36.5, 57.0)
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

pub(super) const ROUTES_BUTTON: Rect = (956.0, 598.0, 132.0, 26.0);
pub(super) const ROOT_ON_SELECT: Rect = (32.0, 488.0, 194.0, 24.0);
pub(super) fn route_slot_rect(i: usize) -> Rect {
    (
        692.0 + (i % 8) as f32 * 48.0,
        260.0 + (i / 8) as f32 * 26.0,
        44.0,
        24.0,
    )
}
pub(super) const ROUTE_SOURCE: Rect = (692.0, 316.0, 172.0, 28.0);
pub(super) const ROUTE_TARGET: Rect = (886.0, 316.0, 190.0, 28.0);
pub(super) const ROUTE_ENABLED: Rect = (906.0, 226.0, 126.0, 26.0);
pub(super) const ROUTE_CLEAR: Rect = (828.0, 226.0, 68.0, 26.0);
pub(super) const ROUTE_GRAPH: Rect = (704.0, 366.0, 360.0, 98.0);
pub(super) const ROUTE_LINEAR: Rect = (984.0, 474.0, 92.0, 24.0);
pub(super) fn route_control_rect(_id: &str) -> Rect {
    ROUTE_ENABLED
}

pub(super) const PIANO_SURFACE: Rect = (16.0, 688.0, 1088.0, 140.0);
pub(super) const ALWAYS_BASS: Rect = (32.0, 698.0, 168.0, 28.0);
pub(super) const ALWAYS_CHORD: Rect = (208.0, 698.0, 212.0, 28.0);
pub(super) const KEY_SPLIT: Rect = (440.0, 698.0, 100.0, 28.0);
pub(super) const SPLIT_NOTE: Rect = (548.0, 698.0, 162.0, 28.0);
pub(super) const PIANO_KEYS: Rect = (32.0, 766.0, 1056.0, 52.0);
pub(super) const PIANO_LEADING: Rect = (32.0, 730.0, 1056.0, 28.0);

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
