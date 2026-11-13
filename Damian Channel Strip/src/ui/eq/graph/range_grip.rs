use super::*;

const RANGE_GRIP_SPACING: f32 = 4.0;

/// The static response at full dynamic range, independent of the live reduction meter.
pub(in crate::ui) fn range_band(b: &Band) -> Band {
    let mut endpoint = b.clone();
    endpoint.gain -= b.range;
    endpoint
}

pub(in crate::ui) fn between_curve_polygon(
    main: &[(f32, f32)],
    range: &[(f32, f32)],
) -> Vec<(f32, f32)> {
    let mut points = Vec::with_capacity(main.len() + range.len());
    points.extend_from_slice(main);
    points.extend(range.iter().rev().copied());
    points
}

pub(in crate::ui) fn range_curve_points(
    b: &Band,
    xs: &[f32],
    graph: (f32, f32, f64),
    rates: (f64, f64),
) -> Vec<(f32, f32)> {
    let (gx, gw, graph_db) = graph;
    let (sr, eq_sr) = rates;
    let coeff = BandCoeffs::make(&range_band(b), eq_sr);
    let dbs = eq_db_on_xs(&coeff, xs, gx, gw, sr, eq_sr);
    eq_points(xs, &dbs, graph_db)
}

pub(in crate::ui) fn draw_dyn_range_curve(
    d: &mut Draw,
    b: &Band,
    xs: &[f32],
    graph: (f32, f32, f64),
    rates: (f64, f64),
    color: C,
) {
    let points = range_curve_points(b, xs, graph, rates);
    let range_color = C {
        a: color.a * 0.5,
        ..color
    };
    d.poly(&points, range_color, 1.2);
    for line in range_grip_lines(b, graph, rates) {
        d.poly(&line, range_color, 1.2);
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
    let mut span = width;
    // At very high Q an inward parallel curve develops a loop. Remove that
    // loop, and extend the common source span if its end normals fall inside
    // the cusp. Both strokes must still terminate on the same source normals.
    let mut lines = [Vec::new(), Vec::new()];
    for _ in 0..8 {
        let anchors: Vec<_> = (0..=128)
            .map(|i| {
                let distance = (i as f32 / 128.0 - 0.5) * span;
                let x = curve_x_at_arc_distance(&curve_y_at, node_x, distance);
                let slope = (curve_y_at(x + 0.05) - curve_y_at(x - 0.05)) / 0.1;
                let length = (1.0 + slope * slope).sqrt();
                (x, curve_y_at(x), -slope / length, 1.0 / length)
            })
            .collect();
        lines = std::array::from_fn(|side| {
            let offset = if side == 0 {
                -RANGE_GRIP_SPACING
            } else {
                RANGE_GRIP_SPACING
            };
            trim_offset_loops(
                &anchors
                    .iter()
                    .map(|&(x, y, nx, ny)| (x + offset * nx, y + offset * ny))
                    .collect::<Vec<_>>(),
            )
        });
        if lines
            .iter()
            .all(|line| line.windows(2).all(|p| p[1].0 > p[0].0))
        {
            break;
        }
        span *= 1.5;
    }
    lines
}

/// Keep the exterior branch of a parallel curve when a tight bend produces a
/// self-intersection. The original first and last points remain the end caps.
fn trim_offset_loops(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let cross = |a: (f32, f32), b: (f32, f32)| a.0 * b.1 - a.1 * b.0;
    let mut result: Vec<(f32, f32)> = Vec::with_capacity(points.len());
    for &point in points {
        while result.len() >= 3 {
            let start = result[result.len() - 1];
            let direction = (point.0 - start.0, point.1 - start.1);
            let intersection = (0..result.len() - 2).find_map(|j| {
                let a = result[j];
                let edge = (result[j + 1].0 - a.0, result[j + 1].1 - a.1);
                let denominator = cross(edge, direction);
                if denominator.abs() < 1e-6 {
                    return None;
                }
                let delta = (start.0 - a.0, start.1 - a.1);
                let t = cross(delta, direction) / denominator;
                let u = cross(delta, edge) / denominator;
                if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                    Some((j, (a.0 + t * edge.0, a.1 + t * edge.1)))
                } else {
                    None
                }
            });
            let Some((j, intersection)) = intersection else {
                break;
            };
            result.truncate(j + 1);
            result.push(intersection);
        }
        result.push(point);
    }
    result
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
