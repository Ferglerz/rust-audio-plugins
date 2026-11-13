use super::*;

pub(in crate::ui) fn power_rect() -> Rect {
    (ARP_SURFACE.0 + 12.0, ARP_SURFACE.1 + 10.0, 28.0, 28.0)
}
pub(in crate::ui) fn mode_rect() -> Rect {
    let r = fill_rect();
    (r.0 + r.2 + 8.0, r.1, 96.0, r.3)
}
pub(in crate::ui) fn order_rect() -> Rect {
    (ARP_SURFACE.0 + 224.0, ARP_SURFACE.1 + 10.0, 134.0, 28.0)
}
pub(in crate::ui) fn loop_add_rect() -> Rect {
    let top = ARP_LANE_TOP - 28.0;
    (
        ARP_SURFACE.0 + ARP_SURFACE.2 - 40.0,
        ARP_SURFACE.1 + top,
        28.0,
        ARP_SURFACE.3 - top - 12.0,
    )
}
pub(in crate::ui) fn sync_rect() -> Rect {
    (ARP_SURFACE.0 + ARP_SURFACE.2 - 272.0, ARP_SURFACE.1 + 10.0, 104.0, 28.0)
}
pub(in crate::ui) fn rate_rect() -> Rect {
    let r = sync_rect();
    (r.0 + r.2 + 8.0, r.1, 148.0, r.3)
}
pub(in crate::ui) fn humanize_rect() -> Rect {
    (GLOBAL_SURFACE.0 + 144.0, GLOBAL_SURFACE.1 + 48.0, 112.0, 100.0)
}
pub(in crate::ui) fn swing_rect() -> Rect {
    let r = humanize_rect();
    (r.0, GLOBAL_SURFACE.1 + 196.0, r.2, 100.0)
}
pub(super) fn adjustment_knob_layout(rect: Rect) -> pleasant_ui::draw::KnobLayout {
    let mut layout = pleasant_ui::draw::KnobLayout::new(rect);
    layout.radius = 26.0;
    layout.cy = rect.1 + 52.0;
    layout.label_y = rect.1 + 12.0;
    layout.value_y = rect.1 + rect.3 - 10.0;
    layout.with_text_sizes(TEXT_LABEL, TEXT_LABEL)
}
pub(in crate::ui) fn adjustment_value_rect(rect: Rect) -> Rect {
    adjustment_knob_layout(rect).value_rect(rect.0 + 4.0, rect.2 - 8.0)
}
#[cfg(test)]
pub(in crate::ui) fn delay_rect() -> Rect {
    super::super::midi_fx::FX_TAB
}
pub(in crate::ui) fn lock_length_rect() -> Rect {
    let r = custom_edit_rect();
    (r.0 + r.2 + 8.0, r.1, 28.0, r.3)
}
pub(in crate::ui) fn fixed_steps_rect() -> Rect {
    let r = lock_length_rect();
    (r.0 + r.2 + 8.0, r.1, 142.0, r.3)
}
pub(in crate::ui) fn fill_rect() -> Rect {
    let r = fixed_steps_rect();
    (r.0 + r.2 + 8.0, r.1, 156.0, r.3)
}
pub(in crate::ui) fn custom_edit_rect() -> Rect {
    let r = order_rect();
    (r.0 + r.2 + 8.0, r.1, 120.0, r.3)
}
pub(in crate::ui) const OUTPUT_COLUMN_W: f32 = 0.0;

pub(in crate::ui) fn page_block() -> Rect {
    (
        ARP_SURFACE.0 + OUTPUT_COLUMN_W + 12.0,
        ARP_SURFACE.1 + ARP_LANE_TOP - 28.0,
        134.0,
        28.0,
    )
}
// A page dropdown row has independent selection, copy and paste columns.
pub(in crate::ui) fn page_rect(page: usize) -> Rect {
    let r = page_block();
    (r.0, r.1 + r.3 + 4.0 + page as f32 * 24.0, 100.0, 24.0)
}
pub(in crate::ui) fn page_copy_rect(page: usize) -> Rect {
    let r = page_rect(page);
    (r.0 + 104.0, r.1, 54.0, r.3)
}
pub(in crate::ui) fn page_paste_rect(page: usize) -> Rect {
    let r = page_rect(page);
    (r.0 + 162.0, r.1, 66.0, r.3)
}
const LANE_BUTTON_W: f32 = 72.0;
const LANE_GRID_TOP: f32 = ARP_LANE_TOP + 8.0;
const LANE_GAP: f32 = 6.0;
pub(in crate::ui) fn inherit_rect() -> Rect {
    let lane = lane_rect(0);
    (lane.0, lane.1 - 36.0, 2.0 * LANE_BUTTON_W + LANE_GAP, 28.0)
}
pub(super) fn tracker_voicing_rect(inversion: bool) -> Rect {
    let r = inherit_rect();
    (
        graph_rect().0 + 158.0 * u8::from(inversion) as f32,
        r.1,
        150.0,
        r.3,
    )
}
pub(in crate::ui) fn tracker_voicing_button(inversion: bool, plus: bool) -> Rect {
    let r = tracker_voicing_rect(inversion);
    (if plus { r.0 + r.2 - 28.0 } else { r.0 }, r.1, 28.0, r.3)
}
pub(in crate::ui) fn lane_rect(index: usize) -> Rect {
    let height = LANE_BUTTON_W;
    (
        ARP_SURFACE.0 + OUTPUT_COLUMN_W + 12.0 + (index % 2) as f32 * (LANE_BUTTON_W + LANE_GAP),
        ARP_SURFACE.1 + LANE_GRID_TOP + (index / 2) as f32 * (height + LANE_GAP),
        LANE_BUTTON_W,
        height,
    )
}
pub(in crate::ui) fn gate_rect() -> Rect {
    let g = graph_rect();
    (g.0, g.1 + g.3 + 6.0, g.2, 22.0)
}
pub(super) fn content_rect() -> Rect {
    let g = graph_rect();
    (
        g.0,
        ARP_SURFACE.1 + ARP_LANE_TOP - 28.0,
        g.2,
        ARP_SURFACE.3 - ARP_LANE_TOP + 16.0,
    )
}
pub(super) fn loop_rect(index: usize, count: usize) -> Rect {
    let g = graph_rect();
    let width = g.2 / count.max(1) as f32;
    (
        g.0 + index as f32 * width + 2.0,
        ARP_SURFACE.1 + ARP_LANE_TOP - 26.0,
        width - 4.0,
        32.0,
    )
}
pub(super) fn loop_header_button(r: Rect, button: usize) -> Rect {
    // Keep the close action at the right edge, including at four loops.
    // 0/1 sit under Inv, 3/4 under Oct. Positions match the labels.
    if button == 2 {
        return (r.0 + r.2 - 26.0, r.1 + 2.0, 22.0, r.3 - 4.0);
    }
    let offsets = [0.0, 96.0, 0.0, 124.0, 204.0];
    let widths = [22.0; 5];
    (
        r.0 + 60.0 + offsets[button],
        r.1 + 2.0,
        widths[button],
        r.3 - 4.0,
    )
}
pub(in crate::ui) fn graph_rect() -> Rect {
    let left = ARP_SURFACE.0 + OUTPUT_COLUMN_W + 178.0;
    (
        left,
        ARP_SURFACE.1 + ARP_LANE_TOP + 36.0,
        loop_add_rect().0 - 16.0 - left,
        ARP_SURFACE.3 - ARP_LANE_TOP - 123.0,
    )
}

pub(super) struct StringLayout {
    pub origin: usize,
    pub x: [f32; STEPS],
    pub interval: [f32; STEPS],
    pub spacing: f32,
    pub len: usize,
}

impl StringLayout {
    pub(super) fn new(steps: &[Step; STEPS], len: usize, swing: f32, phase: i64) -> Self {
        let swing = if len > 1 {
            swing.clamp(-0.75, 0.75)
        } else {
            0.0
        };
        // Reserve the extremes of both swing and ±49% microtiming inside the graph.
        let graph = graph_rect();
        let spacing = graph.2 / (len.max(1) as f32 + swing.abs());
        let mut result = Self {
            origin: 0,
            x: [0.0; STEPS],
            interval: [spacing; STEPS],
            spacing,
            len,
        };
        let mut clock = phase;
        let origin = if phase.rem_euclid(2) != 0 { swing } else { 0.0 };
        for (i, step) in steps.iter().enumerate().take(len) {
            let odd = clock.rem_euclid(2) != 0;
            result.x[i] = graph.0
                + (i as f32 + 0.5 + swing.abs() * 0.5 + if odd { swing } else { 0.0 } - origin)
                    * spacing;
            result.interval[i] = spacing * (1.0 + if odd { -swing } else { swing });
            if step.state != StepState::Skip {
                clock += 1;
            }
        }
        result
    }
    pub(super) fn rotated(mut self, origin: usize) -> Self {
        if self.len == 0 {
            return self;
        }
        self.origin = origin % self.len;
        for i in 0..self.len {
            self.x[i] += (self.visual(i) as f32 - i as f32) * self.spacing;
        }
        self
    }
    pub(super) fn visual(&self, index: usize) -> usize {
        (index + self.len - self.origin) % self.len.max(1)
    }
    pub(super) fn source(&self, index: usize) -> usize {
        (index + self.origin) % self.len.max(1)
    }
    pub(super) fn at(&self, x: f32) -> usize {
        (0..self.len)
            .min_by(|&a, &b| (self.x[a] - x).abs().total_cmp(&(self.x[b] - x).abs()))
            .unwrap_or(0)
    }
    pub(super) fn status(&self, index: usize) -> Rect {
        let w = (self.spacing - 2.0).clamp(2.0, 36.0);
        (
            self.x[index] - w * 0.5,
            ARP_SURFACE.1 + ARP_SURFACE.3 - 39.0,
            w,
            18.0,
        )
    }
}

pub(in crate::ui) fn page_paste_option_rect(page: usize, index: usize) -> Rect {
    let r = page_paste_rect(page);
    let y = r.1.min(ARP_SURFACE.1 + ARP_SURFACE.3 - 50.0);
    (r.0 + r.2 + 6.0, y + index as f32 * 22.0, 154.0, 22.0)
}

pub(super) fn dropdown_rect(which: Dropdown, index: usize) -> Rect {
    if let Dropdown::Paste(page) = which {
        return page_paste_option_rect(page, index);
    }
    let r = match which {
        Dropdown::TapLane => tap_lane_rect(),
        Dropdown::Pages => page_block(),
        Dropdown::Paste(_) => unreachable!(),
    };
    let n = which.len();
    let h = 22.0;
    let surface = if which == Dropdown::TapLane {
        DELAY_SURFACE
    } else {
        ARP_SURFACE
    };
    let y = (r.1 + r.3 + 4.0).min(surface.1 + surface.3 - n as f32 * h - 6.0);
    (r.0, y + index as f32 * h, r.2.max(154.0), h)
}

pub(super) fn chooser_area() -> Rect {
    let s = ARP_SURFACE;
    (
        s.0 + OUTPUT_COLUMN_W + 16.0,
        s.1 + ARP_LANE_TOP + 5.0,
        s.2 - OUTPUT_COLUMN_W - 128.0,
        s.3 - ARP_LANE_TOP - 17.0,
    )
}
#[cfg(test)]
pub(in crate::ui) fn chooser_card(index: usize) -> Rect {
    chooser_card_for(index, 8)
}
pub(in crate::ui) fn chooser_card_for(index: usize, count: usize) -> Rect {
    let r = chooser_area();
    let gap = 8.0;
    let w = (r.2 - gap * (count - 1) as f32) / count as f32;
    (r.0 + index as f32 * (w + gap), r.1, w, r.3)
}

// These are the ordering depictions from 00d16be's performance controls.
pub(super) const ORDER_CONTOURS: [[u8; 5]; 8] = [
    [0, 1, 2, 3, 4],
    [4, 3, 2, 1, 0],
    [0, 2, 4, 2, 0],
    [0, 2, 1, 3, 2],
    [4, 2, 3, 1, 2],
    [0, 3, 1, 2, 4],
    [2, 4, 0, 3, 1],
    [0, 2, 1, 4, 3],
];

pub(in crate::ui) fn draw_direction_icon(d: &mut Draw, index: usize, r: Rect, color: Color) {
    let points = std::array::from_fn::<_, 5, _>(|j| {
        (
            r.0 + j as f32 * r.2 / 4.0,
            r.1 + r.3 - ORDER_CONTOURS[index][j] as f32 * r.3 / 4.0,
        )
    });
    d.poly(&points, color, 1.6);
    for (x, y) in points {
        d.circle(x, y, 2.0, color, true);
    }
}

pub(super) fn value_y(value: f32, range: (f32, f32), g: Rect) -> f32 {
    g.1 + g.3 * (1.0 - (value - range.0) / (range.1 - range.0))
}

pub(super) fn paint_between(
    from: usize,
    to: usize,
    start: f32,
    end: f32,
    mut apply: impl FnMut(usize, f32),
) {
    for i in from.min(to)..=from.max(to) {
        let f = if from == to {
            1.0
        } else {
            (i as f32 - from as f32) / (to as f32 - from as f32)
        };
        apply(i, start + (end - start) * f);
    }
}

pub(super) fn dotted(d: &mut Draw, x: f32, y: f32, h: f32, color: Color) {
    let mut yy = y;
    while yy < y + h {
        d.line(x, yy, x, (yy + 2.0).min(y + h), color, 1.0);
        yy += 5.0;
    }
}

pub(super) fn time_label(time: f32, sync: bool) -> String {
    if !sync {
        return format!("{time:.0} ms");
    };
    ARP_RATES
        .iter()
        .find(|(_, b)| (*b - time).abs() < 0.0001)
        .map_or_else(|| format!("{time:.3} beats"), |(name, _)| (*name).into())
}
