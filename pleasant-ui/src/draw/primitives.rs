use super::Draw;
use nih_plug_vizia::vizia::vg::{Color, Paint, Path};

/// Screen pixels from a trace baseline that still count as rest.
/// Covers a 1px stroke centered on the baseline plus the antialias fringe.
pub const TRACE_REST_PX: f32 = 1.25;

pub(super) fn trace_rest_px(stroke_width: f32) -> f32 {
    TRACE_REST_PX.max(stroke_width * 0.5 + 0.5)
}

/// Pieces of `points` that leave `baseline` by more than `rest_px`.
/// Endpoints sit on the rest boundary so a flat rest line is not stroked or filled.
pub fn for_each_span_off_baseline(
    points: &[(f32, f32)],
    baseline: f32,
    rest_px: f32,
    mut f: impl FnMut(Option<(f32, f32)>, &[(f32, f32)], Option<(f32, f32)>),
) {
    if points.len() < 2 {
        return;
    }
    let off = |y: f32| (y - baseline).abs() > rest_px;
    let mut run_start: Option<usize> = None;
    for (i, p) in points.iter().enumerate() {
        if off(p.1) {
            if run_start.is_none() {
                run_start = Some(i);
            }
        } else if let Some(s) = run_start.take() {
            let start = (s > 0).then(|| rest_boundary(points[s - 1], points[s], baseline, rest_px));
            let body = &points[s..i];
            let end = Some(rest_boundary(points[i - 1], *p, baseline, rest_px));
            if start.is_some() as usize + body.len() + 1 >= 2 {
                f(start, body, end);
            }
        }
    }
    if let Some(s) = run_start {
        let start = (s > 0).then(|| rest_boundary(points[s - 1], points[s], baseline, rest_px));
        let body = &points[s..];
        if start.is_some() as usize + body.len() >= 2 {
            f(start, body, None);
        }
    }
}

#[cfg(test)]
pub fn spans_off_baseline(
    points: &[(f32, f32)],
    baseline: f32,
    rest_px: f32,
) -> Vec<Vec<(f32, f32)>> {
    let mut spans = Vec::new();
    for_each_span_off_baseline(points, baseline, rest_px, |start, body, end| {
        let mut span =
            Vec::with_capacity(start.is_some() as usize + body.len() + end.is_some() as usize);
        if let Some(p) = start {
            span.push(p);
        }
        span.extend_from_slice(body);
        if let Some(p) = end {
            span.push(p);
        }
        spans.push(span);
    });
    spans
}

fn rest_boundary(a: (f32, f32), b: (f32, f32), baseline: f32, rest_px: f32) -> (f32, f32) {
    let side = if (a.1 - baseline).abs() > rest_px {
        a.1 - baseline
    } else {
        b.1 - baseline
    };
    let target = baseline + side.signum() * rest_px;
    let dy = b.1 - a.1;
    let t = if dy.abs() < 1.0e-6 {
        0.0
    } else {
        ((target - a.1) / dy).clamp(0.0, 1.0)
    };
    (a.0 + (b.0 - a.0) * t, a.1 + dy * t)
}

impl Draw<'_> {
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let mut p = Path::new();
        p.rounded_rect(
            self.ox + (x + self.offset_x) * self.s,
            self.oy + y * self.s,
            w * self.s,
            h * self.s,
            5.0_f32.min(w * 0.5).min(h * 0.5) * self.s,
        );
        self.c.fill_path(&p, &Paint::color(self.color(color)));
    }

    pub fn rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
        let mut p = Path::new();
        p.rounded_rect(
            self.ox + (x + self.offset_x) * self.s,
            self.oy + y * self.s,
            w * self.s,
            h * self.s,
            radius * self.s,
        );
        self.c.fill_path(&p, &Paint::color(self.color(color)));
    }

    pub fn line(&mut self, x: f32, y: f32, ex: f32, ey: f32, color: Color, width: f32) {
        self.poly(&[(x, y), (ex, ey)], color, width);
    }

    pub fn poly(&mut self, points: &[(f32, f32)], color: Color, width: f32) {
        let mut p = Path::new();
        for (i, (x, y)) in points.iter().enumerate() {
            if i == 0 {
                p.move_to(self.ox + (x + self.offset_x) * self.s, self.oy + y * self.s);
            } else {
                p.line_to(self.ox + (x + self.offset_x) * self.s, self.oy + y * self.s);
            }
        }
        let mut paint = Paint::color(self.color(color));
        paint.set_line_width(width * self.s);
        self.c.stroke_path(&p, &paint);
    }

    fn span_stroke_path(
        &self,
        start: Option<(f32, f32)>,
        body: &[(f32, f32)],
        end: Option<(f32, f32)>,
    ) -> Path {
        let mut p = Path::new();
        let mut started = false;
        for (x, y) in start
            .into_iter()
            .chain(body.iter().copied())
            .chain(end.into_iter())
        {
            let px = self.ox + (x + self.offset_x) * self.s;
            let py = self.oy + y * self.s;
            if !started {
                p.move_to(px, py);
                started = true;
            } else {
                p.line_to(px, py);
            }
        }
        p
    }

    fn span_area_path(
        &self,
        start: Option<(f32, f32)>,
        body: &[(f32, f32)],
        end: Option<(f32, f32)>,
        bottom: f32,
    ) -> Option<Path> {
        let first = start.or_else(|| body.first().copied()).or(end)?;
        let last = end.or_else(|| body.last().copied()).or(start)?;
        let mut p = Path::new();
        p.move_to(
            self.ox + (first.0 + self.offset_x) * self.s,
            self.oy + bottom * self.s,
        );
        for (x, y) in start
            .into_iter()
            .chain(body.iter().copied())
            .chain(end.into_iter())
        {
            p.line_to(self.ox + (x + self.offset_x) * self.s, self.oy + y * self.s);
        }
        p.line_to(
            self.ox + (last.0 + self.offset_x) * self.s,
            self.oy + bottom * self.s,
        );
        p.close();
        Some(p)
    }

    /// Stroke `points`, omitting runs that sit on `baseline`.
    pub fn poly_above(&mut self, points: &[(f32, f32)], baseline: f32, color: Color, width: f32) {
        let mut paint = Paint::color(self.color(color));
        paint.set_line_width(width * self.s);
        for_each_span_off_baseline(points, baseline, trace_rest_px(width), |start, body, end| {
            let p = self.span_stroke_path(start, body, end);
            self.c.stroke_path(&p, &paint);
        });
    }

    pub fn area(&mut self, points: &[(f32, f32)], bottom: f32, color: Color) {
        let paint = Paint::color(self.color(color));
        for_each_span_off_baseline(points, bottom, TRACE_REST_PX, |start, body, end| {
            if let Some(p) = self.span_area_path(start, body, end, bottom) {
                self.c.fill_path(&p, &paint);
            }
        });
    }

    /// Shared EQ handle: solid node with an optional full selection ring.
    pub fn eq_node(&mut self, x: f32, y: f32, color: Color, selected: bool) {
        if selected {
            self.circle(x, y, 12.0, color, false);
        }
        self.circle(x, y, 7.0, color, true);
    }

    pub fn circle(&mut self, x: f32, y: f32, r: f32, color: Color, fill: bool) {
        let mut p = Path::new();
        p.circle(
            self.ox + (x + self.offset_x) * self.s,
            self.oy + y * self.s,
            r * self.s,
        );
        let mut paint = Paint::color(self.color(color));
        paint.set_line_width(self.s);
        if fill {
            self.c.fill_path(&p, &paint);
        } else {
            self.c.stroke_path(&p, &paint);
        }
    }

    fn pill_path(&self, x: f32, y: f32, y_range: f32, r: f32) -> Option<Path> {
        let top_y = y.min(y_range);
        let bot_y = y.max(y_range);
        if bot_y - top_y < 0.5 {
            return None;
        }
        let mut p = Path::new();
        let ox = self.ox + (x + self.offset_x) * self.s;
        let s = self.s;
        let top_oy = self.oy + top_y * s;
        let bot_oy = self.oy + bot_y * s;
        let rs = r * s;
        let steps = 16;
        for i in 0..=steps {
            let theta = std::f32::consts::PI + std::f32::consts::PI * (i as f32 / steps as f32);
            let px = ox + rs * theta.cos();
            let py = top_oy + rs * theta.sin();
            if i == 0 {
                p.move_to(px, py);
            } else {
                p.line_to(px, py);
            }
        }
        for i in 0..=steps {
            let theta = std::f32::consts::PI * (i as f32 / steps as f32);
            let px = ox + rs * theta.cos();
            let py = bot_oy + rs * theta.sin();
            p.line_to(px, py);
        }
        p.close();
        Some(p)
    }

    /// Split-circle range handle: semicircle caps at `y` and `y_range`, joined by vertical sides.
    pub fn pill(&mut self, x: f32, y: f32, y_range: f32, r: f32, color: Color) {
        self.pill_with_end_dot(x, y, y_range, r, color, true);
    }

    pub fn pill_with_end_dot(
        &mut self,
        x: f32,
        y: f32,
        y_range: f32,
        r: f32,
        color: Color,
        end_dot: bool,
    ) {
        let Some(p) = self.pill_path(x, y, y_range, r) else {
            self.circle(x, y, r, color, false);
            return;
        };
        let mut fill_color = color;
        fill_color.a = 0.12;
        self.c.fill_path(&p, &Paint::color(self.color(fill_color)));
        let mut stroke = Paint::color(self.color(color));
        stroke.set_line_width(self.s);
        self.c.stroke_path(&p, &stroke);
        if end_dot {
            self.circle(x, y_range, 3.0, color, true);
        }
    }

    /// Filled pill used to meter live gain reduction/increase inside the range outline.
    pub fn pill_fill(&mut self, x: f32, y: f32, y_amount: f32, r: f32, color: Color) {
        let Some(p) = self.pill_path(x, y, y_amount, r) else {
            return;
        };
        self.c.fill_path(&p, &Paint::color(self.color(color)));
    }

    pub fn outline(&mut self, r: (f32, f32, f32, f32), color: Color) {
        let mut path = Path::new();
        path.rounded_rect(
            self.ox + (r.0 + self.offset_x) * self.s,
            self.oy + r.1 * self.s,
            r.2 * self.s,
            r.3 * self.s,
            5.0 * self.s,
        );
        let mut paint = Paint::color(self.color(color));
        paint.set_line_width(self.s);
        self.c.stroke_path(&path, &paint);
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Drawing primitives use separate coordinates and style values"
    )]
    pub fn outline_rounded(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        color: Color,
        width: f32,
    ) {
        let mut path = Path::new();
        path.rounded_rect(
            self.ox + (x + self.offset_x) * self.s,
            self.oy + y * self.s,
            w * self.s,
            h * self.s,
            radius * self.s,
        );
        let mut paint = Paint::color(self.color(color));
        paint.set_line_width(width * self.s);
        self.c.stroke_path(&path, &paint);
    }

    fn rounded_poly_path(&self, points: &[(f32, f32)], radius: f32) -> Option<Path> {
        let n = points.len();
        if n < 3 {
            return None;
        }
        let mut path = Path::new();
        let s = self.s;
        let ox = self.ox;
        let oy = self.oy;
        let off = self.offset_x;
        let mut started = false;
        for i in 0..n {
            let prev = points[(i + n - 1) % n];
            let curr = points[i];
            let next = points[(i + 1) % n];
            let (vix, viy, din) = {
                let dx = curr.0 - prev.0;
                let dy = curr.1 - prev.1;
                let len = (dx * dx + dy * dy).sqrt().max(0.0001);
                (dx / len, dy / len, len)
            };
            let (vox, voy, dout) = {
                let dx = next.0 - curr.0;
                let dy = next.1 - curr.1;
                let len = (dx * dx + dy * dy).sqrt().max(0.0001);
                (dx / len, dy / len, len)
            };
            let r = radius.min(din * 0.49).min(dout * 0.49);
            let p_enter = (curr.0 - vix * r, curr.1 - viy * r);
            let p_exit = (curr.0 + vox * r, curr.1 + voy * r);
            let enter_x = ox + (p_enter.0 + off) * s;
            let enter_y = oy + p_enter.1 * s;
            if !started {
                path.move_to(enter_x, enter_y);
                started = true;
            } else {
                path.line_to(enter_x, enter_y);
            }
            for k in 1..=8 {
                let t = k as f32 / 8.0;
                let u = 1.0 - t;
                let bx = u * u * p_enter.0 + 2.0 * u * t * curr.0 + t * t * p_exit.0;
                let by = u * u * p_enter.1 + 2.0 * u * t * curr.1 + t * t * p_exit.1;
                path.line_to(ox + (bx + off) * s, oy + by * s);
            }
        }
        path.close();
        Some(path)
    }

    /// Stroke a closed polygon with rounded corners at every vertex.
    pub fn stroke_rounded_poly(
        &mut self,
        points: &[(f32, f32)],
        radius: f32,
        color: Color,
        width: f32,
    ) {
        let Some(path) = self.rounded_poly_path(points, radius) else {
            return;
        };
        let mut paint = Paint::color(self.color(color));
        paint.set_line_width(width * self.s);
        self.c.stroke_path(&path, &paint);
    }

    /// Fill a closed polygon with rounded corners at every vertex.
    pub fn fill_rounded_poly(&mut self, points: &[(f32, f32)], radius: f32, color: Color) {
        let Some(path) = self.rounded_poly_path(points, radius) else {
            return;
        };
        self.c.fill_path(&path, &Paint::color(self.color(color)));
    }

    pub fn fill_poly(&mut self, points: &[(f32, f32)], color: Color) {
        if points.len() < 3 {
            return;
        }
        let mut p = Path::new();
        for (i, (x, y)) in points.iter().enumerate() {
            let px = self.ox + (x + self.offset_x) * self.s;
            let py = self.oy + y * self.s;
            if i == 0 {
                p.move_to(px, py);
            } else {
                p.line_to(px, py);
            }
        }
        p.close();
        self.c.fill_path(&p, &Paint::color(self.color(color)));
    }

    pub fn poly_gradient_span(
        &mut self,
        points: &[(f32, f32)],
        sx: f32,
        ex: f32,
        c1: Color,
        c2: Color,
        width: f32,
    ) {
        if points.is_empty() {
            return;
        }
        let mut p = Path::new();
        for (i, (x, y)) in points.iter().enumerate() {
            if i == 0 {
                p.move_to(self.ox + (x + self.offset_x) * self.s, self.oy + y * self.s);
            } else {
                p.line_to(self.ox + (x + self.offset_x) * self.s, self.oy + y * self.s);
            }
        }
        let mut paint = Paint::linear_gradient(
            self.ox + (sx + self.offset_x) * self.s,
            self.oy,
            self.ox + (ex + self.offset_x) * self.s,
            self.oy,
            self.color(c1),
            self.color(c2),
        );
        paint.set_line_width(width * self.s);
        self.c.stroke_path(&p, &paint);
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Drawing primitives use separate coordinates and style values"
    )]
    pub fn poly_gradient_above(
        &mut self,
        points: &[(f32, f32)],
        baseline: f32,
        sx: f32,
        ex: f32,
        c1: Color,
        c2: Color,
        width: f32,
    ) {
        let mut paint = Paint::linear_gradient(
            self.ox + (sx + self.offset_x) * self.s,
            self.oy,
            self.ox + (ex + self.offset_x) * self.s,
            self.oy,
            self.color(c1),
            self.color(c2),
        );
        paint.set_line_width(width * self.s);
        for_each_span_off_baseline(points, baseline, trace_rest_px(width), |start, body, end| {
            let p = self.span_stroke_path(start, body, end);
            self.c.stroke_path(&p, &paint);
        });
    }

    pub fn area_gradient_span(
        &mut self,
        points: &[(f32, f32)],
        bottom: f32,
        sx: f32,
        ex: f32,
        c1: Color,
        c2: Color,
    ) {
        let paint = Paint::linear_gradient(
            self.ox + (sx + self.offset_x) * self.s,
            self.oy,
            self.ox + (ex + self.offset_x) * self.s,
            self.oy,
            self.color(c1),
            self.color(c2),
        );
        for_each_span_off_baseline(points, bottom, TRACE_REST_PX, |start, body, end| {
            if let Some(p) = self.span_area_path(start, body, end, bottom) {
                self.c.fill_path(&p, &paint);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resting_trace_is_omitted() {
        let flat = [(0.0, 100.0), (40.0, 100.2), (80.0, 99.4), (120.0, 100.0)];
        assert!(spans_off_baseline(&flat, 100.0, TRACE_REST_PX).is_empty());
    }

    #[test]
    fn trace_clear_of_baseline_is_kept() {
        let line = [(0.0, 80.0), (50.0, 80.0), (100.0, 70.0)];
        let spans = spans_off_baseline(&line, 100.0, TRACE_REST_PX);
        assert_eq!(spans, vec![line.to_vec()]);
    }

    #[test]
    fn resting_runs_split_around_a_hump() {
        let points = [
            (0.0, 0.0),
            (10.0, 0.2),
            (20.0, 8.0),
            (30.0, 0.0),
            (40.0, 6.0),
            (50.0, 0.1),
        ];
        let spans = spans_off_baseline(&points, 0.0, 1.0);
        assert_eq!(spans.len(), 2);
        assert!((spans[0][0].1 - 1.0).abs() < 1.0e-4);
        assert!((spans[0].last().unwrap().1 - 1.0).abs() < 1.0e-4);
        assert!(spans[0].iter().any(|p| (p.1 - 8.0).abs() < 1.0e-4));
        assert!(spans[1].iter().any(|p| (p.1 - 6.0).abs() < 1.0e-4));
        for span in &spans {
            assert!(span.iter().all(|p| p.1 + 1.0e-4 >= 1.0));
        }
    }

    #[test]
    fn span_callback_matches_vec_output_across_edge_cases() {
        let cases: [&[(f32, f32)]; 4] = [
            &[],
            &[(0.0, 10.0)],
            &[(0.0, 8.0), (10.0, 0.0), (20.0, 0.2), (30.0, 6.0)],
            &[(0.0, 0.0), (10.0, 5.0), (20.0, 0.0), (30.0, -4.0), (40.0, 0.0)],
        ];
        for points in cases {
            let expected = spans_off_baseline(points, 0.0, 1.0);
            let mut got = Vec::new();
            for_each_span_off_baseline(points, 0.0, 1.0, |start, body, end| {
                got.push(
                    start
                        .into_iter()
                        .chain(body.iter().copied())
                        .chain(end)
                        .collect::<Vec<_>>(),
                );
            });
            assert_eq!(got, expected);
        }
    }
}
