use super::*;

pub(super) fn lift_gain_y(gain: f64) -> f32 {
    let norm = ((gain + 100.0) / 100.0).clamp(0.0, 1.0) as f32;
    GY + GH * (1.0 - norm)
}

pub(super) fn lift_y_gain(y: f32) -> f64 {
    let norm = (1.0 - (y - GY) / GH).clamp(0.0, 1.0) as f64;
    -100.0 + norm * 100.0
}

pub(super) fn lift_bin_curve_fit(bins: &[f64; 256], t: f64) -> f64 {
    let u = (t * 256.0 - 0.5).clamp(0.0, 255.0);
    let k = (u.floor() as usize).min(254);
    let s = u - k as f64;
    let y0 = if k > 0 { bins[k - 1] } else { bins[0] };
    let y1 = bins[k];
    let y2 = bins[k + 1];
    let y3 = if k + 2 < 256 { bins[k + 2] } else { bins[255] };

    let a = -0.5 * y0 + 1.5 * y1 - 1.5 * y2 + 0.5 * y3;
    let b = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c = -0.5 * y0 + 0.5 * y2;
    let d = y1;

    (((a * s + b) * s + c) * s + d).max(0.0)
}

pub(super) fn smooth_lift_bins(raw: &[f64; 256]) -> [f64; 256] {
    let mut smooth = [0.0_f64; 256];
    smooth_bins(raw, &mut smooth);
    smooth
}

pub(super) fn lift_curve_points(
    b: &LiftBand,
    gr: &[f64; 256],
    gx: f32,
    gw: f32,
    sr: f64,
) -> Vec<(f32, f32)> {
    let graph_bottom_y = GY + GH;
    (0..=500)
        .map(|i| {
            let x = gx + gw * (i as f32 / 500.0);
            let f = x_freq_at(x, gx, gw);
            let inf = filter_influence(b.shape, b.freq, b.q, b.order, f, sr);
            let t = ((f / 20.0).log10() / 3.0).clamp(0.0, 1.0);
            let gr_db = lift_bin_curve_fit(gr, t);
            let cut_gain = (b.gain - gr_db * 1.5).clamp(-100.0, 0.0);
            let norm = ((cut_gain + 100.0) / 100.0) as f32;
            let y = graph_bottom_y - (GH * norm * inf as f32);
            (x, y)
        })
        .collect()
}

pub(super) fn draw_lift_influence(
    d: &mut Draw,
    b: &LiftBand,
    points: &[(f32, f32)],
    color: C,
    alphas: (f32, f32),
    graph: (f32, f32),
) {
    let (fill_a, stroke_a) = alphas;
    let (gx, gw) = graph;
    let graph_bottom_y = GY + GH;
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
    match b.shape {
        Shape::Bell | Shape::BandPass | Shape::Notch => {
            let oct_span = (2.0 / b.q.clamp(0.15, 18.0)).clamp(0.5, 4.0);
            let f_left = (b.freq * 2.0_f64.powf(-oct_span)).max(20.0);
            let f_right = (b.freq * 2.0_f64.powf(oct_span)).min(20000.0);
            let x_left = freq_x_at(f_left, gx, gw);
            let x_center = freq_x_at(b.freq, gx, gw);
            let x_right = freq_x_at(f_right, gx, gw);
            let center_idx = points
                .iter()
                .position(|(x, _)| *x >= x_center)
                .unwrap_or(points.len() / 2);
            let left_pts = &points[..=center_idx];
            let right_pts = &points[center_idx..];
            if fill_a > 0.0 {
                d.area_gradient_span(left_pts, graph_bottom_y, x_left, x_center, c_trans, c_solid);
                d.area_gradient_span(
                    right_pts,
                    graph_bottom_y,
                    x_center,
                    x_right,
                    c_solid,
                    c_trans,
                );
            }
            d.poly_gradient_above(
                left_pts,
                graph_bottom_y,
                x_left,
                x_center,
                s_trans,
                s_solid,
                1.8,
            );
            d.poly_gradient_above(
                right_pts,
                graph_bottom_y,
                x_center,
                x_right,
                s_solid,
                s_trans,
                1.8,
            );
        }
        Shape::HighShelf | Shape::LowCut => {
            let oct_span = (1.5 / b.q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
            let f_fade_start = (b.freq * 2.0_f64.powf(-oct_span)).max(20.0);
            let f_fade_end = (b.freq * 2.0_f64.powf(oct_span * 0.5)).min(20000.0);
            let x_start = freq_x_at(f_fade_start, gx, gw);
            let x_end = freq_x_at(f_fade_end, gx, gw);
            if fill_a > 0.0 {
                d.area_gradient_span(points, graph_bottom_y, x_start, x_end, c_trans, c_solid);
            }
            d.poly_gradient_above(
                points,
                graph_bottom_y,
                x_start,
                x_end,
                s_trans,
                s_solid,
                1.8,
            );
        }
        Shape::LowShelf | Shape::HighCut => {
            let oct_span = (1.5 / b.q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
            let f_fade_start = (b.freq * 2.0_f64.powf(-oct_span * 0.5)).max(20.0);
            let f_fade_end = (b.freq * 2.0_f64.powf(oct_span)).min(20000.0);
            let x_start = freq_x_at(f_fade_start, gx, gw);
            let x_end = freq_x_at(f_fade_end, gx, gw);
            if fill_a > 0.0 {
                d.area_gradient_span(points, graph_bottom_y, x_start, x_end, c_solid, c_trans);
            }
            d.poly_gradient_above(
                points,
                graph_bottom_y,
                x_start,
                x_end,
                s_solid,
                s_trans,
                1.8,
            );
        }
    }
}
