// Shared geometry for rendering and event hit-testing.
use super::KEY_COLUMNS;
pub(super) const W: f32 = 1120.0;
pub(super) const H: f32 = 780.0;
pub(super) type Rect = (f32, f32, f32, f32);
pub(super) const PAD: Rect = (600.0, 190.0, 488.0, 310.0);
// Keep drawing and pointer targets on the same performance surface.
pub(super) const PLAY_PAD: Rect = (620.0, 235.0, 448.0, 235.0);
pub(super) const LATCH: Rect = (440.0, 154.0, 124.0, 28.0);
pub(super) const MODE_LABELS: [&str; 4] = ["CHORD", "AUTO STRUM", "MANUAL STRUM", "ARPEGGIATOR"];
pub(super) fn mode_rect(i: usize) -> Rect {
    (600.0 + i as f32 * 124.0, 154.0, 116.0, 28.0)
}
pub(super) fn inversion_rect(i: usize) -> Rect {
    (32.0 + i as f32 * 42.0, 154.0, 34.0, 28.0)
}
pub(super) fn output_controls() -> [(&'static str, Rect); 3] {
    [
        ("velocity", (600.0, 514.0, 156.0, 36.0)),
        ("length_ms", (766.0, 514.0, 156.0, 36.0)),
        ("strings", (932.0, 514.0, 156.0, 36.0)),
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
pub(super) fn pattern_rect(i: usize, main: bool) -> Rect {
    if main {
        (618.0 + i as f32 * 92.0, 226.0, 84.0, 42.0)
    } else {
        (44.0 + i as f32 * 65.0, 650.0, 59.0, 42.0)
    }
}
pub(super) fn rate_rect(i: usize, main: bool) -> Rect {
    if main {
        (618.0 + i as f32 * 58.0, 294.0, 46.0, 24.0)
    } else {
        (
            388.0 + (i % 4) as f32 * 58.0,
            647.0 + (i / 4) as f32 * 27.0,
            52.0,
            23.0,
        )
    }
}
pub(super) fn rate_header(main: bool) -> Rect {
    if main {
        (618.0, 272.0, 452.0, 20.0)
    } else {
        (388.0, 624.0, 226.0, 20.0)
    }
}
pub(super) fn octave_rect(i: usize, main: bool) -> Rect {
    if main {
        (690.0 + i as f32 * 34.0, 334.0, 28.0, 26.0)
    } else {
        (132.0 + i as f32 * 54.0, 711.0, 46.0, 26.0)
    }
}
pub(super) fn arp_controls(main: bool) -> [(&'static str, Rect); 3] {
    if main {
        [
            ("humanize", (844.0, 330.0, 226.0, 34.0)),
            ("gate", (618.0, 376.0, 452.0, 54.0)),
            ("swing", (618.0, 436.0, 452.0, 54.0)),
        ]
    } else {
        [
            ("humanize", (388.0, 708.0, 226.0, 34.0)),
            ("gate", (632.0, 624.0, 456.0, 54.0)),
            ("swing", (632.0, 688.0, 456.0, 54.0)),
        ]
    }
}
pub(super) const SETUP: Rect = (982.0, 585.0, 106.0, 28.0);
pub(super) const PAGE: Rect = (850.0, 585.0, 120.0, 28.0);
pub(super) const LEARN: Rect = (348.0, 624.0, 180.0, 34.0);
pub(super) const LEARN_OCTAVE: Rect = (600.0, 624.0, 220.0, 34.0);
pub(super) fn axis_rect(i: usize) -> Rect {
    (32.0 + i as f32 * 100.0, 624.0, 92.0, 34.0)
}
pub(super) fn mapping_summary_rect(i: usize) -> Rect {
    (600.0 + i as f32 * 164.0, 556.0, 160.0, 22.0)
}
pub(super) fn group_rect(i: usize) -> Rect {
    (32.0 + i as f32 * 180.0, 585.0, 168.0, 28.0)
}
pub(super) const PERFORMANCE_GROUPS: [(usize, &str); 3] =
    [(0, "VOICING"), (1, "RHYTHM"), (3, "EXPRESSION")];
pub(super) const SETUP_GROUPS: [(usize, &str); 4] =
    [(6, "MAPPING"), (2, "MPE"), (4, "INPUT"), (5, "DISCOVERY")];
pub(super) fn key_rect(index: usize) -> Rect {
    let row = index / KEY_COLUMNS;
    let col = index % KEY_COLUMNS;
    (
        32.0 + ([0.0, 0.25, 0.75][row] + col as f32) * 42.0,
        230.0 + row as f32 * 67.0,
        38.0,
        59.0,
    )
}
pub(super) fn control_rect(index: usize) -> Rect {
    (
        32.0 + (index % 4) as f32 * 268.0,
        624.0 + (index / 4) as f32 * 64.0,
        252.0,
        54.0,
    )
}

// Minimum readable size for hints and secondary labels at the native window size.
pub(super) const TEXT_SMALL: f32 = 11.0;
pub(super) const TEXT_LABEL: f32 = 12.0;
pub(super) fn memory_rect(i: usize) -> Rect {
    (32.0 + i as f32 * 67.0, 504.0, 61.0, 46.0)
}
pub(super) fn memory_delete_rect(i: usize) -> Rect {
    let r = memory_rect(i);
    (r.0 + r.2 - 22.0, r.1, 22.0, 22.0)
}
