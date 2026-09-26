use super::super::*;

pub(in crate::ui) fn freq_x_at(freq: f64, gx: f32, gw: f32) -> f32 {
    gx + gw * (freq / 20.0).log(1000.0).clamp(0.0, 1.0) as f32
}

pub(in crate::ui) fn x_freq_at(x: f32, gx: f32, gw: f32) -> f64 {
    20.0 * 1000.0_f64.powf(((x - gx) / gw).clamp(0.0, 1.0) as f64)
}

pub(in crate::ui) fn db_y(db: f64, range: f64) -> f32 {
    GY + GH * (0.5 - (db.clamp(-range, range) / (2.0 * range)) as f32)
}

pub(in crate::ui) fn y_db(y: f32, range: f64) -> f64 {
    let range = range.abs().max(1.0e-6);
    ((0.5 - (y - GY) as f64 / GH as f64) * 2.0 * range).clamp(-range, range)
}

/// Pixel-spaced x plus each band's exact frequency so peaks and notch zeros
/// land on a vertex instead of being skipped by a coarse chord.
pub(in crate::ui) fn eq_curve_xs(
    gx: f32,
    gw: f32,
    extra_freqs: impl IntoIterator<Item = f64>,
) -> Vec<f32> {
    let n = gw.max(2.0).ceil() as usize;
    let mut xs: Vec<f32> = (0..=n).map(|i| gx + gw * (i as f32 / n as f32)).collect();
    for f in extra_freqs {
        if f.is_finite() && f > 0.0 {
            let x = freq_x_at(f, gx, gw);
            if x >= gx && x <= gx + gw {
                xs.push(x);
            }
        }
    }
    xs.sort_by(|a, b| a.total_cmp(b));
    xs.dedup_by(|a, b| (*a - *b).abs() < 0.15);
    xs
}

pub(in crate::ui) fn eq_db_on_xs(
    coeff: &BandCoeffs,
    xs: &[f32],
    gx: f32,
    gw: f32,
    sr: f64,
    eq_sr: f64,
) -> Vec<f64> {
    let nyq = sr * 0.49;
    xs.iter()
        .map(|&x| coeff.response(x_freq_at(x, gx, gw).min(nyq), eq_sr))
        .collect()
}

pub(in crate::ui) fn eq_points(xs: &[f32], dbs: &[f64], graph_db: f64) -> Vec<(f32, f32)> {
    xs.iter()
        .zip(dbs)
        .map(|(&x, &db)| (x, db_y(db, graph_db)))
        .collect()
}

pub(in crate::ui) fn hover_preview_label(has_selected: bool, shape: Shape) -> &'static str {
    if has_selected {
        "DESELECT"
    } else {
        shape.uppercase_name()
    }
}

/// Label baseline Y equals circle center; draw uses Middle baseline so glyph centers on mouse.
pub(in crate::ui) fn hover_preview_text_y(circle_y: f32) -> f32 {
    circle_y
}

fn hover_label_x(x: f32, gx: f32, gw: f32, label: &str) -> f32 {
    let label_w = label.len() as f32 * 12.0 * 0.6;
    if x + 12.0 + label_w > gx + gw - 8.0 {
        (x - 12.0 - label_w).max(gx + 8.0)
    } else {
        x + 12.0
    }
}

/// Cursor glyph in viewport space (`offset_x` cleared) so page-slide residue cannot shift it.
pub(in crate::ui) fn draw_hover_preview(
    d: &mut Draw,
    x: f32,
    y: f32,
    gx: f32,
    gw: f32,
    label: &str,
) {
    let saved = d.offset_x;
    d.offset_x = 0.0;
    d.circle(x, y, 5.0, MUTED, false);
    d.text_middle(
        hover_label_x(x, gx, gw, label),
        hover_preview_text_y(y),
        label,
        12.0,
        MUTED,
    );
    d.offset_x = saved;
}

/// Control name only — no create-cursor circle (threshold / range-end grabs).
pub(in crate::ui) fn draw_hover_control_label(
    d: &mut Draw,
    x: f32,
    y: f32,
    gx: f32,
    gw: f32,
    label: &str,
    color: C,
) {
    let saved = d.offset_x;
    d.offset_x = 0.0;
    d.text_middle(
        hover_label_x(x, gx, gw, label),
        hover_preview_text_y(y),
        label,
        12.0,
        color,
    );
    d.offset_x = saved;
}

pub(in crate::ui) fn draw_eq_preview_influence(
    d: &mut Draw,
    shape: Shape,
    freq: f64,
    q: f64,
    points: &[(f32, f32)],
    graph_db: f64,
    graph: (f32, f32),
) {
    let (gx, gw) = graph;
    let baseline_y = db_y(0.0, graph_db);
    let fill_a = 0.08;
    let stroke_a = 0.40;
    let color = MUTED;
    let c_trans = C {
        r: color.r,
        g: color.g,
        b: color.b,
        a: 0.0,
    };
    let c_solid = C {
        r: color.r,
        g: color.g,
        b: color.b,
        a: fill_a,
    };
    let s_trans = C {
        r: color.r,
        g: color.g,
        b: color.b,
        a: 0.0,
    };
    let s_solid = C {
        r: color.r,
        g: color.g,
        b: color.b,
        a: stroke_a,
    };
    match shape {
        Shape::Bell | Shape::BandPass | Shape::Notch => {
            let oct_span = (2.0 / q.clamp(0.15, 18.0)).clamp(0.5, 4.0);
            let f_left = (freq * 2.0_f64.powf(-oct_span)).max(20.0);
            let f_right = (freq * 2.0_f64.powf(oct_span)).min(20000.0);
            let x_left = freq_x_at(f_left, gx, gw);
            let x_center = freq_x_at(freq, gx, gw);
            let x_right = freq_x_at(f_right, gx, gw);
            let center_idx = points
                .iter()
                .position(|(x, _)| *x >= x_center)
                .unwrap_or(points.len() / 2);
            let left_pts = &points[..=center_idx];
            let right_pts = &points[center_idx..];
            if fill_a > 0.0 {
                d.area_gradient_span(left_pts, baseline_y, x_left, x_center, c_trans, c_solid);
                d.area_gradient_span(right_pts, baseline_y, x_center, x_right, c_solid, c_trans);
            }
            d.poly_gradient_span(left_pts, x_left, x_center, s_trans, s_solid, 1.4);
            d.poly_gradient_span(right_pts, x_center, x_right, s_solid, s_trans, 1.4);
        }
        Shape::HighShelf | Shape::HighCut => {
            let oct_span = (1.5 / q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
            let f_fade_start = (freq * 2.0_f64.powf(-oct_span)).max(20.0);
            let f_fade_end = (freq * 2.0_f64.powf(oct_span * 0.5)).min(20000.0);
            let x_start = freq_x_at(f_fade_start, gx, gw);
            let x_end = freq_x_at(f_fade_end, gx, gw);
            if fill_a > 0.0 {
                d.area_gradient_span(points, baseline_y, x_start, x_end, c_trans, c_solid);
            }
            d.poly_gradient_span(points, x_start, x_end, s_trans, s_solid, 1.4);
        }
        Shape::LowShelf | Shape::LowCut => {
            let oct_span = (1.5 / q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
            let f_fade_start = (freq * 2.0_f64.powf(-oct_span * 0.5)).max(20.0);
            let f_fade_end = (freq * 2.0_f64.powf(oct_span)).min(20000.0);
            let x_start = freq_x_at(f_fade_start, gx, gw);
            let x_end = freq_x_at(f_fade_end, gx, gw);
            if fill_a > 0.0 {
                d.area_gradient_span(points, baseline_y, x_start, x_end, c_solid, c_trans);
            }
            d.poly_gradient_span(points, x_start, x_end, s_solid, s_trans, 1.4);
        }
    }
}

pub(in crate::ui) fn solo_shade_rects(
    shape: Shape,
    freq: f64,
    q: f64,
    gx: f32,
    gw: f32,
) -> (Option<(f32, f32)>, Option<(f32, f32)>) {
    let graph_l = gx;
    let graph_r = gx + gw;
    let left_rect = |x_edge: f32| {
        let w = x_edge - graph_l;
        (w > 1.0).then_some((graph_l, w))
    };
    let right_rect = |x_edge: f32| {
        let w = graph_r - x_edge;
        (w > 1.0).then_some((x_edge, w))
    };
    match shape {
        Shape::LowCut => {
            let oct = (1.5 / q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
            let f_edge = (freq * 2.0_f64.powf(-oct * 0.5)).max(20.0);
            (left_rect(freq_x_at(f_edge, gx, gw)), None)
        }
        Shape::HighCut => {
            let oct = (1.5 / q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
            let f_edge = (freq * 2.0_f64.powf(oct * 0.5)).min(20000.0);
            (None, right_rect(freq_x_at(f_edge, gx, gw)))
        }
        _ => {
            let oct_span = (2.0 / q.clamp(0.15, 18.0)).clamp(0.5, 4.0);
            let x_left = freq_x_at((freq * 2.0_f64.powf(-oct_span)).max(20.0), gx, gw);
            let x_right = freq_x_at((freq * 2.0_f64.powf(oct_span)).min(20000.0), gx, gw);
            (left_rect(x_left), right_rect(x_right))
        }
    }
}

pub(in crate::ui) fn greyscale_darken(color: C, amount: f32) -> C {
    let y = (0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b) * (1.0 - amount);
    C {
        r: y,
        g: y,
        b: y,
        a: color.a,
    }
}

pub(in crate::ui) fn intersect_rect(
    a: (f32, f32, f32, f32),
    b: (f32, f32, f32, f32),
) -> Option<(f32, f32, f32, f32)> {
    let x0 = a.0.max(b.0);
    let y0 = a.1.max(b.1);
    let x1 = (a.0 + a.2).min(b.0 + b.2);
    let y1 = (a.1 + a.3).min(b.1 + b.3);
    (x1 - x0 > 0.5 && y1 - y0 > 0.5).then_some((x0, y0, x1 - x0, y1 - y0))
}

pub(in crate::ui) fn draw_solo_shade(
    d: &mut Draw,
    shape: Shape,
    freq: f64,
    q: f64,
    gx: f32,
    gw: f32,
    clip: (f32, f32, f32, f32),
    sum: &[(f32, f32)],
    sum_color: C,
    sum_width: f32,
    fill: Option<(&[(f32, f32)], f32, C)>,
) {
    let overlay = C::rgba(12, 12, 14, 178);
    let grey_sum = greyscale_darken(sum_color, 0.32);
    let (left, right) = solo_shade_rects(shape, freq, q, gx, gw);
    for (x, w) in [left, right].into_iter().flatten() {
        d.rounded_rect(x, GY, w, GH, 0.0, overlay);
        let Some((ix, iy, iw, ih)) = intersect_rect((x + d.offset_x, GY, w, GH), clip) else {
            continue;
        };
        d.scissor(ix, iy, iw, ih);
        if let Some((pts, baseline, fill_c)) = fill {
            d.area(pts, baseline, greyscale_darken(fill_c, 0.40));
        }
        d.poly(sum, grey_sum, sum_width);
        d.scissor(clip.0, clip.1, clip.2, clip.3);
    }
}

pub(in crate::ui) fn draw_eq_hover_preview(
    d: &mut Draw,
    x: f32,
    y: f32,
    gx: f32,
    gw: f32,
    graph_db: f64,
    sr: f64,
    eq_sr: f64,
    selected: Option<u64>,
    shape: Shape,
) {
    if selected.is_none() {
        let f = x_freq_at(x, gx, gw).clamp(20.0, sr * 0.49);
        let gain = if shape.has_gain() {
            y_db(y, graph_db)
        } else {
            0.0
        };
        let q = if matches!(shape, Shape::LowCut | Shape::HighCut) {
            0.707
        } else {
            1.0
        };
        let preview_band = Band {
            id: 0,
            shape,
            freq: f,
            gain,
            q,
            order: 2,
            ..Band::default()
        };
        let coeff = BandCoeffs::make(&preview_band, eq_sr);
        let xs = eq_curve_xs(gx, gw, [preview_band.freq]);
        let dbs = eq_db_on_xs(&coeff, &xs, gx, gw, sr, eq_sr);
        let points = eq_points(&xs, &dbs, graph_db);
        draw_eq_preview_influence(
            d,
            shape,
            preview_band.freq,
            preview_band.q,
            &points,
            graph_db,
            (gx, gw),
        );
    }
    draw_hover_preview(
        d,
        x,
        y,
        gx,
        gw,
        hover_preview_label(selected.is_some(), shape),
    );
}

/// Threshold / range-end grab labels, else empty-space create/deselect preview.
pub(in crate::ui) fn draw_band_graph_hover(
    d: &mut Draw,
    x: f32,
    y: f32,
    gx: f32,
    gw: f32,
    graph_db: f64,
    sr: f64,
    eq_sr: f64,
    selected: Option<u64>,
    bands: &[Band],
    meters: &[(u64, f32)],
    ratio_mode: bool,
) {
    if bands
        .iter()
        .any(|b| node_handle_hit(b, x, y, graph_db, gx, gw))
    {
        return;
    }
    if let Some(b) = selected.and_then(|id| bands.iter().find(|b| b.id == id)) {
        let control = dynamics_control_band(b, ratio_mode);
        if threshold_handle_hit(&control, x, y, graph_db, gx, gw, selected) {
            draw_hover_control_label(
                d,
                x,
                y,
                gx,
                gw,
                if ratio_mode { "RATIO" } else { "THRESHOLD" },
                if ratio_mode { GOLD } else { MUTED },
            );
            return;
        }
        if range_end_hit(b, x, y, graph_db, gx, gw, selected)
            || range_grip_hit(b, x, y, (gx, gw, graph_db), (sr, eq_sr))
        {
            draw_hover_control_label(d, x, y, gx, gw, "RANGE END", MUTED);
            return;
        }
    }
    if bands
        .iter()
        .any(|b| near_eq_node(b, x, y, graph_db, gx, gw))
    {
        return;
    }
    let db = bands
        .iter()
        .filter(|b| b.enabled)
        .map(|b| {
            BandCoeffs::make(&plot_band(b, meters), eq_sr).response(x_freq_at(x, gx, gw), eq_sr)
        })
        .sum::<f64>();
    let curve = (y - db_y(db, graph_db)).abs() < 12.0;
    let shape = infer_shape((x - gx) / gw, (y - GY) / GH, curve);
    draw_eq_hover_preview(d, x, y, gx, gw, graph_db, sr, eq_sr, selected, shape);
}

pub(in crate::ui) const DYN_PILL_R: f32 = 12.0;
pub(in crate::ui) const NODE_HIT_R: f32 = 16.0;
/// Padding from the node toward the meter track (matches old end-cap clearance).
pub(in crate::ui) const DYN_METER_PAD: f32 = DYN_PILL_R;
/// Gap between the node and a detached meter start.
pub(in crate::ui) const DYN_DETACHED_NODE_GAP: f32 = 28.0;
/// Minimum stem length (px) before the input meter sits inside the range pill.
/// Doubled from the original `DYN_METER_PAD + 28` (40px) so medium ranges stay detached.
pub(in crate::ui) const DYN_METER_MIN_INLINE: f32 = (DYN_METER_PAD + 28.0) * 2.0;
/// Track width inside the range pill / detached bar (scaled with the capsule).
pub(in crate::ui) const DYN_METER_TRACK_W: f32 = 6.5;
pub(in crate::ui) const DYN_THRESH_RULE_GAP: f32 = 2.5;
pub(in crate::ui) const DYN_RANGE_W: f32 = DYN_PILL_R * 2.0;
/// Overhang past each pill edge, as a fraction of the full range-UI width.
pub(in crate::ui) const DYN_THRESH_OVERHANG: f32 = 0.20;
const RANGE_GRIP_SPACING: f32 = 4.0;

#[derive(Clone, Copy, Debug)]
pub(in crate::ui) struct DynMeterGeom {
    pub x: f32,
    #[cfg(test)]
    pub node_y: f32,
    #[cfg(test)]
    pub range_y: f32,
    pub inline: bool,
    /// Detached meter sits above the stem (toward the top of the graph).
    pub detached_above: bool,
    /// Screen y for 0 dB (toward the node).
    pub y0: f32,
    /// Screen y for -60 dB (range-end cap when inline).
    pub y60: f32,
    pub thresh_y: f32,
    pub rule_half_w: f32,
}

impl DynMeterGeom {
    pub fn level_y(self, level_db: f64) -> f32 {
        let n = ((level_db + 60.0) / 60.0).clamp(0.0, 1.0) as f32;
        self.y60 + (self.y0 - self.y60) * n
    }

    pub fn y_to_threshold(self, y: f32) -> f64 {
        let span = self.y0 - self.y60;
        if span.abs() < 1.0e-3 {
            return -24.0;
        }
        let n = ((y - self.y60) / span).clamp(0.0, 1.0) as f64;
        (-60.0 + 60.0 * n).clamp(-60.0, 0.0)
    }

    pub fn thresh_hit(self, x: f32, y: f32) -> bool {
        (x - self.x).abs() <= self.rule_half_w + 4.0 && (y - self.thresh_y).abs() <= 8.0
    }
}

/// Longer open span above `from_y`: graph top (with edge pad) vs 0 dB center.
fn detached_far_above(from_y: f32, graph_db: f64) -> f32 {
    let edge = GY + DYN_METER_PAD;
    let center = db_y(0.0, graph_db);
    let span_edge = (from_y - edge).max(0.0);
    let span_center = if center < from_y {
        from_y - center
    } else {
        0.0
    };
    if span_edge >= span_center && span_edge > 0.0 {
        edge
    } else if span_center > 0.0 {
        center
    } else {
        edge.min(from_y - 8.0)
    }
}

/// Longer open span below `from_y`: graph bottom (with edge pad) vs 0 dB center.
fn detached_far_below(from_y: f32, graph_db: f64) -> f32 {
    let edge = GRAPH_BOTTOM - DYN_METER_PAD;
    let center = db_y(0.0, graph_db);
    let span_edge = (edge - from_y).max(0.0);
    let span_center = if center > from_y {
        center - from_y
    } else {
        0.0
    };
    if span_edge >= span_center && span_edge > 0.0 {
        edge
    } else if span_center > 0.0 {
        center
    } else {
        edge.max(from_y + 8.0)
    }
}

fn detached_span_len(from_y: f32, far_y: f32) -> f32 {
    (from_y - far_y).abs()
}

pub(in crate::ui) fn dyn_meter_geom(b: &Band, graph_db: f64, gx: f32, gw: f32) -> DynMeterGeom {
    let x = freq_x_at(b.freq, gx, gw);
    let node_y = db_y(b.gain, graph_db);
    let range_y = db_y(b.gain - b.range, graph_db);
    let stem = range_y - node_y;
    let stem_len = stem.abs();
    // Pill half-width plus 20% of the full range-UI width past each edge.
    let rule_half_w = DYN_PILL_R + DYN_RANGE_W * DYN_THRESH_OVERHANG;
    // Compare usable track lengths after clearing both ends of the range pill.
    let range_down = stem >= 0.0;
    let y0_above = if range_down {
        node_y - DYN_DETACHED_NODE_GAP
    } else {
        range_y - DYN_METER_PAD
    };
    let far_above = detached_far_above(y0_above, graph_db);
    let span_above = detached_span_len(y0_above, far_above);
    let y0_below = if range_down {
        range_y + DYN_METER_PAD
    } else {
        node_y + DYN_DETACHED_NODE_GAP
    };
    let far_below = detached_far_below(y0_below, graph_db);
    let span_below = detached_span_len(y0_below, far_below);
    let inline_span = (stem_len - 2.0 * DYN_METER_PAD).max(0.0);
    let (inline, detached_above, y0, y60) =
        if stem_len >= DYN_METER_MIN_INLINE && inline_span >= span_above.max(span_below) {
            let dir = if stem >= 0.0 { 1.0 } else { -1.0 };
            (
                true,
                false,
                node_y + dir * DYN_METER_PAD,
                range_y - dir * DYN_METER_PAD,
            )
        } else {
            if span_below >= span_above {
                (false, false, y0_below, far_below)
            } else {
                (false, true, y0_above, far_above)
            }
        };
    let thresh_y = {
        let n = ((b.threshold + 60.0) / 60.0).clamp(0.0, 1.0) as f32;
        y60 + (y0 - y60) * n
    };
    DynMeterGeom {
        x,
        #[cfg(test)]
        node_y,
        #[cfg(test)]
        range_y,
        inline,
        detached_above,
        y0,
        y60,
        thresh_y,
        rule_half_w,
    }
}

/// Reuse the meter's full travel for the 1:1–20:1 ratio while CMD is held.
pub(in crate::ui) fn dynamics_control_band(b: &Band, ratio_mode: bool) -> Band {
    let mut control = b.clone();
    if ratio_mode {
        control.threshold = -60.0 + 60.0 * ((b.ratio - 1.0) / 19.0).clamp(0.0, 1.0);
    }
    control
}

/// Include the 12px selection ring and a 4px grab margin in node movement.
pub(in crate::ui) fn node_handle_hit(
    b: &Band,
    x: f32,
    y: f32,
    graph_db: f64,
    gx: f32,
    gw: f32,
) -> bool {
    let nx = freq_x_at(b.freq, gx, gw);
    let ny = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, graph_db);
    (x - nx).hypot(y - ny) <= NODE_HIT_R
}

pub(in crate::ui) fn range_handle_hit(
    b: &Band,
    x: f32,
    y: f32,
    graph_db: f64,
    gx: f32,
    gw: f32,
    selected: Option<u64>,
) -> bool {
    if !b.shape.has_gain() || node_handle_hit(b, x, y, graph_db, gx, gw) {
        return false;
    }
    let x_node = freq_x_at(b.freq, gx, gw);
    let node_y = db_y(b.gain, graph_db);
    if b.dynamic {
        let y_range = db_y(b.gain - b.range, graph_db);
        if (x - x_node).abs() <= NODE_HIT_R && (y - y_range).abs() <= DYN_PILL_R {
            return true;
        }
        let top = node_y.min(y_range);
        let bot = node_y.max(y_range);
        if (x - x_node).abs() <= DYN_PILL_R && y >= top && y <= bot {
            return true;
        }
        if Some(b.id) == selected {
            let geom = dyn_meter_geom(b, graph_db, gx, gw);
            if !geom.inline {
                let top = geom.y0.min(geom.y60) - 4.0;
                let bot = geom.y0.max(geom.y60) + 4.0;
                if (x - geom.x).abs() <= geom.rule_half_w && y >= top && y <= bot {
                    return true;
                }
            }
        }
    }
    false
}

pub(in crate::ui) fn threshold_handle_hit(
    b: &Band,
    x: f32,
    y: f32,
    graph_db: f64,
    gx: f32,
    gw: f32,
    selected: Option<u64>,
) -> bool {
    if Some(b.id) != selected || !b.dynamic || !b.shape.has_gain() {
        return false;
    }
    !node_handle_hit(b, x, y, graph_db, gx, gw)
        && dyn_meter_geom(b, graph_db, gx, gw).thresh_hit(x, y)
}

/// Range-end cap only (not the whole stem / node hit).
pub(in crate::ui) fn range_end_hit(
    b: &Band,
    x: f32,
    y: f32,
    graph_db: f64,
    gx: f32,
    gw: f32,
    selected: Option<u64>,
) -> bool {
    if Some(b.id) != selected || !b.dynamic || !b.shape.has_gain() {
        return false;
    }
    let x_node = freq_x_at(b.freq, gx, gw);
    let node_y = db_y(b.gain, graph_db);
    if (x - x_node).hypot(y - node_y) <= NODE_HIT_R {
        return false;
    }
    let y_range = db_y(b.gain - b.range, graph_db);
    (x - x_node).abs() <= NODE_HIT_R && (y - y_range).abs() <= DYN_PILL_R
}

pub(in crate::ui) fn near_eq_node(
    b: &Band,
    x: f32,
    y: f32,
    graph_db: f64,
    gx: f32,
    gw: f32,
) -> bool {
    let nx = freq_x_at(b.freq, gx, gw);
    let ny = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, graph_db);
    if (x - nx).abs() < 15.0 && (y - ny).abs() < 15.0 {
        return true;
    }
    if b.dynamic && b.shape.has_gain() {
        let y_range = db_y(b.gain - b.range, graph_db);
        if (x - nx).abs() < 15.0 && (y - y_range).abs() < 15.0 {
            return true;
        }
        let top = ny.min(y_range);
        let bot = ny.max(y_range);
        if (x - nx).abs() <= DYN_PILL_R && y >= top - 4.0 && y <= bot + 4.0 {
            return true;
        }
    }
    false
}

/// Graph axis, minus 25%, as an integer dB range when dynamics first turns on.
pub(in crate::ui) fn default_dyn_range(graph_db: f64) -> f64 {
    (graph_db.abs() * 0.75).round().clamp(1.0, 24.0)
}

/// Shift-drag: downward movement raises Q.
pub(in crate::ui) fn q_from_shift_drag(q: f64, dy: f64) -> f64 {
    (q * (1.0 - dy * 0.025)).clamp(0.15, 18.0)
}

pub(in crate::ui) fn threshold_from_drag(threshold: f64, dy: f64) -> f64 {
    (threshold + dy * 0.2).clamp(-60.0, 0.0)
}

pub(in crate::ui) fn snap_dyn_range(gain: f64, y: f32, graph_db: f64) -> f64 {
    let mut new_range = gain - y_db(y, graph_db);
    if new_range.abs() < 0.25 {
        new_range = 0.0;
    }
    new_range.clamp(-MAX_RANGE_DB, MAX_RANGE_DB)
}

fn draw_threshold_rule(d: &mut Draw, geom: DynMeterGeom, color: C) {
    let y = geom.thresh_y;
    let x0 = geom.x - geom.rule_half_w;
    let x1 = geom.x + geom.rule_half_w;
    let gap = DYN_THRESH_RULE_GAP * 0.5;
    d.line(x0, y - gap, x1, y - gap, color, 1.0);
    d.line(x0, y + gap, x1, y + gap, color, 1.0);
}

fn draw_band_input_meter(
    d: &mut Draw,
    geom: DynMeterGeom,
    level_db: f64,
    color: C,
    ratio: Option<f64>,
) {
    let track_x = geom.x - DYN_METER_TRACK_W * 0.5;
    let top = geom.y0.min(geom.y60);
    let bot = geom.y0.max(geom.y60);
    let h = (bot - top).max(1.0);
    let mut track = color;
    track.a = 0.22;
    d.rect(track_x, top, DYN_METER_TRACK_W, h, track);
    let level_y = geom.level_y(level_db);
    let fill_top = level_y.min(geom.y60);
    let fill_bot = level_y.max(geom.y60);
    let fill_h = (fill_bot - fill_top).max(0.0);
    if fill_h > 0.5 {
        let mut fill = color;
        fill.a = 0.70;
        d.rect(track_x, fill_top, DYN_METER_TRACK_W, fill_h, fill);
    }
    let mut rule = geom;
    if let Some(ratio) = ratio {
        rule.thresh_y = geom.level_y(-60.0 + 60.0 * (ratio - 1.0) / 19.0);
    }
    draw_threshold_rule(d, rule, if ratio.is_some() { GOLD } else { color });
}

/// The static response at full dynamic range, independent of the live reduction meter.
pub(in crate::ui) fn range_band(b: &Band) -> Band {
    let mut endpoint = b.clone();
    endpoint.gain -= b.range;
    endpoint
}

pub(in crate::ui) fn draw_dyn_range_curve(
    d: &mut Draw,
    b: &Band,
    xs: &[f32],
    graph: (f32, f32, f64),
    rates: (f64, f64),
    color: C,
) {
    let (gx, gw, graph_db) = graph;
    let (sr, eq_sr) = rates;
    let coeff = BandCoeffs::make(&range_band(b), eq_sr);
    let dbs = eq_db_on_xs(&coeff, xs, gx, gw, sr, eq_sr);
    let points = eq_points(xs, &dbs, graph_db);
    d.poly(
        &points,
        C {
            a: color.a * 0.5,
            ..color
        },
        1.2,
    );
    for line in range_grip_lines(b, graph, rates) {
        d.poly(&line, color, 1.2);
    }
}

pub(in crate::ui) fn range_grip_lines(
    b: &Band,
    graph: (f32, f32, f64),
    rates: (f64, f64),
) -> [Vec<(f32, f32)>; 2] {
    let (gx, gw, graph_db) = graph;
    let (sr, eq_sr) = rates;
    let coeff = BandCoeffs::make(&range_band(b), eq_sr);
    let node_x = freq_x_at(b.freq, gx, gw);
    let endpoint_y = db_y(b.gain - b.range, graph_db);
    let center_response = coeff.response(b.freq.min(sr * 0.49), eq_sr);
    let width = dyn_meter_geom(b, graph_db, gx, gw).rule_half_w * 2.0 * 0.75;
    let curve_y_at = |sample_x: f32| {
        let response = coeff.response(x_freq_at(sample_x, gx, gw).min(sr * 0.49), eq_sr);
        endpoint_y + db_y(response, graph_db) - db_y(center_response, graph_db)
    };
    std::array::from_fn(|side| {
        (0..=32)
            .map(|i| {
                let arc_distance = (i as f32 / 32.0 - 0.5) * width;
                let x = curve_x_at_arc_distance(&curve_y_at, node_x, arc_distance);
                let curve_y = curve_y_at(x);
                let slope = (curve_y_at(x + 0.25) - curve_y_at(x - 0.25)) * 2.0;
                let normal_scale = (1.0 + slope * slope).sqrt().recip();
                let offset = if side == 0 {
                    -RANGE_GRIP_SPACING
                } else {
                    RANGE_GRIP_SPACING
                };
                (
                    x - slope * normal_scale * offset,
                    curve_y + normal_scale * offset,
                )
            })
            .collect()
    })
}

/// Find an X position by distance travelled along the mirrored curve, so a
/// narrow Q does not stretch the grip's spacing vertically.
pub(in crate::ui) fn curve_x_at_arc_distance(
    curve_y_at: &impl Fn(f32) -> f32,
    center_x: f32,
    signed_distance: f32,
) -> f32 {
    let direction = signed_distance.signum();
    let target = signed_distance.abs();
    let mut x = center_x;
    let mut y = curve_y_at(x);
    let mut travelled = 0.0;
    for _ in 0..256 {
        if travelled >= target {
            break;
        }
        let next_x = x + direction * 0.25;
        let next_y = curve_y_at(next_x);
        let segment = (next_x - x).hypot(next_y - y);
        if travelled + segment >= target {
            return x + (next_x - x) * (target - travelled) / segment;
        }
        travelled += segment;
        x = next_x;
        y = next_y;
    }
    x
}

pub(in crate::ui) fn range_grip_hit(
    b: &Band,
    x: f32,
    y: f32,
    graph: (f32, f32, f64),
    rates: (f64, f64),
) -> bool {
    b.dynamic
        && b.shape.has_gain()
        && !node_handle_hit(b, x, y, graph.2, graph.0, graph.1)
        && range_grip_lines(b, graph, rates)
            .iter()
            .flat_map(|line| line.iter())
            .any(|&(px, py)| (x - px).hypot(y - py) <= 4.0)
}

pub(in crate::ui) fn draw_dyn_range_stem(
    d: &mut Draw,
    b: &Band,
    graph_db: f64,
    graph: (f32, f32),
    color: C,
    uncapped: Option<f64>,
    selected: bool,
    input_level_db: Option<f64>,
    ratio_mode: bool,
) {
    let (gx, gw) = graph;
    let x = freq_x_at(b.freq, gx, gw);
    let y = db_y(b.gain, graph_db);
    let y_range = db_y(b.gain - b.range, graph_db);
    if selected {
        let geom = dyn_meter_geom(b, graph_db, gx, gw);
        draw_band_input_meter(
            d,
            geom,
            input_level_db.unwrap_or(-90.0),
            color,
            ratio_mode.then_some(b.ratio),
        );
        return;
    }
    d.pill_with_end_dot(x, y, y_range, DYN_PILL_R, color, true);
    if let Some(uncapped) = uncapped {
        let live = capped_reduction(b.range, uncapped);
        if live.abs() > 0.05 {
            let y_live = db_y(b.gain - live, graph_db);
            let mut fill = color;
            fill.a = 0.45;
            d.pill_fill(x, y, y_live, DYN_PILL_R - 1.0, fill);
        }
        if let Some(over) = overflow_reduction(b.range, uncapped) {
            let y_over = db_y(b.gain - over, graph_db);
            if (y_over - y_range).abs() > 0.5 {
                d.line(x, y_range, x, y_over, MUTED, 3.0);
                d.circle(x, y_over, 3.0, MUTED, true);
            }
        }
    }
}

pub(in crate::ui) fn inside(x: f32, y: f32, r: (f32, f32, f32, f32)) -> bool {
    x >= r.0 && x <= r.0 + r.2 && y >= r.1 && y <= r.1 + r.3
}

pub(in crate::ui) fn plot_band(b: &Band, meters: &[(u64, f32)]) -> Band {
    let mut plot = b.clone();
    if plot.dynamic && plot.shape.has_gain() {
        if let Some(uncapped) = uncapped_for(plot.id, meters) {
            plot.gain -= capped_reduction(plot.range, uncapped);
        }
    }
    plot
}
