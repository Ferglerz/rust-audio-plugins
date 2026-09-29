// Shared geometry for rendering and event hit-testing.
use super::KEY_COLUMNS;
pub(super) const W: f32 = 1120.0;
pub(super) const H: f32 = 704.0;
pub(super) type Rect = (f32, f32, f32, f32);
pub(super) const PAD: Rect = (600.0, 158.0, 488.0, 310.0);
// Keep drawing and pointer targets on the same performance surface.
pub(super) const PLAY_PAD: Rect = (660.0, 233.0, 408.0, 205.0);
pub(super) const LATCH: Rect = (440.0, 154.0, 124.0, 28.0);
pub(super) const QWERTY: Rect = (440.0, 190.0, 124.0, 28.0);
pub(super) const APPEARANCE: Rect = (1010.0, 24.0, 78.0, 30.0);
pub(super) const MODE_LABELS: [&str; 3] = ["AUTO STRUM", "MANUAL STRUM", "ARPEGGIATOR"];
pub(super) fn mode_rect(i: usize) -> Rect {
    (600.0 + i as f32 * 164.0, 122.0, 160.0, 28.0)
}
pub(super) fn inversion_rect(i: usize) -> Rect {
    (32.0 + i as f32 * 42.0, 154.0, 34.0, 28.0)
}
pub(super) fn output_controls(mode: i32) -> Vec<(&'static str, Rect)> {
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
pub(super) const MPE: Rect = (908.0, 94.0, 72.0, 24.0);
pub(super) const TEMPO_SYNC: Rect = (600.0, 24.0, 108.0, 24.0);
pub(super) const TEMPO_CONTROL: Rect = (
    716.0,
    18.0,
    220.0,
    42.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
);
pub(super) const STRUM_SYNC: Rect = (904.0, 162.0, 164.0, 28.0);
pub(super) const OUTPUT: Rect = (988.0, 94.0, 100.0, 24.0);
pub(super) const LEARN: Rect = (884.0, 308.0, 188.0, 28.0);
pub(super) const LEARN_OCTAVE: Rect = (202.0, 190.0, 220.0, 28.0);
pub(super) fn mapping_summary_rect(i: usize) -> Rect {
    (600.0 + i as f32 * 248.0, 524.0, 240.0, 22.0)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Panel {
    Mapping,
    Output,
}
impl Panel {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Mapping => (600.0, 300.0, 488.0, 248.0),
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
pub(super) fn mapping_control_rect(index: usize) -> Rect {
    let mut r = local_control_rect(Panel::Mapping, index);
    r.1 += 54.0;
    r
}
pub(super) const TRANSPOSE_STEPS: [i32; 5] = [-12, -1, 0, 1, 12];
pub(super) fn transpose_rect(i: usize) -> Rect {
    (226.0 + i as f32 * 42.0, 154.0, 38.0, 28.0)
}
pub(super) fn voicing_controls() -> [(&'static str, Rect); 2] {
    [
        (
            "quality",
            (
                600.0,
                624.0,
                140.0,
                42.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        ),
        (
            "spread",
            (
                748.0,
                624.0,
                140.0,
                42.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        ),
    ]
}
pub(super) fn meter_rect(i: usize) -> Rect {
    (904.0 + i as f32 * 64.0, 594.0, 56.0, 80.0)
}
pub(super) fn strum_controls() -> [(&'static str, Rect); 3] {
    [
        (
            "direction",
            (
                616.0,
                342.0,
                220.0,
                42.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
            ),
        ),
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
        32.0 + ([0.0, 0.25, 0.75][row] + col as f32) * 42.0,
        230.0 + row as f32 * 67.0,
        38.0,
        59.0,
    )
}
// Minimum readable size for hints and secondary labels at the native window size.
pub(super) const TEXT_SMALL: f32 = 11.0;
pub(super) const TEXT_LABEL: f32 = 12.0;
pub(super) fn memory_rect(i: usize) -> Rect {
    (32.0 + (1.25 + i as f32) * 42.0, 431.0, 38.0, 59.0)
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

pub(super) fn strum_bound_anchor(y_axis: bool, value: f32) -> (f32, f32, pleasant_ui::TagPointer) {
    if y_axis {
        (
            PLAY_PAD.0,
            PLAY_PAD.1 + (1.0 - value) * PLAY_PAD.3,
            pleasant_ui::TagPointer::Right,
        )
    } else {
        (
            PLAY_PAD.0 + value * PLAY_PAD.2,
            PLAY_PAD.1,
            pleasant_ui::TagPointer::Down,
        )
    }
}
