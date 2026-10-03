use super::Draw;
use crate::theme::{LINE, MUTED, PANEL, TEXT};
use nih_plug_vizia::vizia::vg::{Color, Paint, Path};

impl Draw<'_> {
    /// Small response diagram for filter-shape choices, with a common 0 dB guide.
    pub fn filter_curve(
        &mut self,
        kind: pleasant_dsp::filters::BiquadKind,
        rect: (f32, f32, f32, f32),
        color: Color,
    ) {
        let (x, y, w, h) = rect;
        let coeff =
            pleasant_dsp::filters::BiquadCoefficients::design(kind, 1000.0, 9.0, 0.707, 48000.0);
        let ordinate = |db: f64| y + h * ((12.0 - db.clamp(-24.0, 12.0)) / 36.0) as f32;
        let mut guide = color;
        guide.a *= 0.25;
        self.line(x, ordinate(0.0), x + w, ordinate(0.0), guide, 0.8);
        let points: Vec<_> = (0..=32)
            .map(|i| {
                let t = i as f32 / 32.0;
                let freq = 100.0 * 100.0_f64.powf(t as f64);
                (x + w * t, ordinate(coeff.response_db(freq, 48000.0)))
            })
            .collect();
        self.poly(&points, color, 1.4);
    }

    pub fn power_icon_scaled(
        &mut self,
        cx: f32,
        cy: f32,
        color: Color,
        scale: f32,
        line_width: f32,
    ) {
        let mut points = Vec::new();
        let r = 5.5 * scale;
        let top_gap = 6.5 * scale;
        let bot_gap = 1.0 * scale;
        for i in 0..=16 {
            let angle = -std::f32::consts::FRAC_PI_2
                + 0.8
                + (std::f32::consts::TAU - 1.6) * i as f32 / 16.0;
            points.push((cx + r * angle.cos(), cy + r * angle.sin()));
        }
        self.poly(&points, color, line_width);
        self.line(cx, cy - top_gap, cx, cy - bot_gap, color, line_width);
    }

    pub fn power_icon(&mut self, cx: f32, cy: f32, color: Color) {
        self.power_icon_scaled(cx, cy, color, 1.0, 1.4);
    }

    /// Outline painter's palette, drawn as geometry so it works with every font.
    pub fn palette_icon(&mut self, cx: f32, cy: f32, color: Color) {
        let points = [
            (-1.0, -7.0),
            (-5.0, -5.5),
            (-7.0, -2.0),
            (-7.0, 2.0),
            (-4.5, 5.5),
            (0.0, 7.0),
            (4.5, 5.5),
            (6.5, 2.5),
            (6.0, 0.5),
            (3.0, 0.5),
            (1.5, -1.0),
            (2.0, -3.0),
            (5.0, -4.0),
            (4.0, -6.0),
            (-1.0, -7.0),
        ];
        // Subdivide and round each corner twice: 56 short segments keep the
        // silhouette and thumb recess legible without the coarse polygon edges.
        let mut points = points[..points.len() - 1].to_vec();
        for _ in 0..2 {
            let mut rounded = Vec::with_capacity(points.len() * 2);
            for i in 0..points.len() {
                let (ax, ay) = points[i];
                let (bx, by) = points[(i + 1) % points.len()];
                rounded.push((0.75 * ax + 0.25 * bx, 0.75 * ay + 0.25 * by));
                rounded.push((0.25 * ax + 0.75 * bx, 0.25 * ay + 0.75 * by));
            }
            points = rounded;
        }
        points.push(points[0]);
        let points: Vec<_> = points.iter().map(|&(x, y)| (cx + x, cy + y)).collect();
        self.poly(&points, color, 1.2);
        for (x, y) in [(-2.0, -4.0), (-4.0, -1.0), (-3.0, 3.0), (1.0, 4.0)] {
            self.circle(cx + x, cy + y, 1.0, color, true);
        }
    }

    pub fn close_icon(&mut self, cx: f32, cy: f32, color: Color) {
        self.line(cx - 4.0, cy - 4.0, cx + 4.0, cy + 4.0, color, 1.4);
        self.line(cx + 4.0, cy - 4.0, cx - 4.0, cy + 4.0, color, 1.4);
    }

    /// Raised index finger and open touch rings, legible at toolbar size.
    pub fn touch_icon(&mut self, cx: f32, cy: f32, color: Color) {
        for radius in [3.5, 5.5] {
            let points: Vec<_> = (0..=28)
                .map(|i| {
                    let angle = 2.5 + 4.4 * i as f32 / 28.0;
                    (
                        cx - 2.0 + radius * angle.cos(),
                        cy - 4.0 + radius * angle.sin(),
                    )
                })
                .collect();
            self.poly(&points, color, 1.1);
        }
        let x = |v: f32| self.ox + (cx + v + self.offset_x) * self.s;
        let y = |v: f32| self.oy + (cy + v) * self.s;
        let mut p = Path::new();
        p.move_to(x(-3.5), y(3.0));
        p.line_to(x(-3.5), y(-4.0));
        p.bezier_to(x(-3.5), y(-6.0), x(-0.5), y(-6.0), x(-0.5), y(-4.0));
        p.line_to(x(-0.5), y(0.0));
        p.bezier_to(x(-0.5), y(-2.0), x(2.0), y(-2.0), x(2.0), y(0.0));
        p.line_to(x(2.0), y(2.0));
        p.move_to(x(2.0), y(0.5));
        p.bezier_to(x(2.0), y(-1.0), x(4.5), y(-1.0), x(4.5), y(1.0));
        p.line_to(x(4.5), y(3.0));
        p.move_to(x(4.5), y(2.0));
        p.bezier_to(x(4.5), y(0.5), x(7.0), y(0.5), x(7.0), y(2.5));
        p.line_to(x(7.0), y(5.0));
        p.bezier_to(x(7.0), y(12.0), x(-4.0), y(12.0), x(-5.0), y(5.0));
        p.line_to(x(-6.0), y(1.0));
        p.bezier_to(x(-6.5), y(-1.0), x(-4.0), y(-1.5), x(-3.5), y(1.0));
        p.line_to(x(-3.5), y(3.0));
        let mut paint = Paint::color(self.color(color));
        paint.set_line_width(1.1 * self.s);
        self.c.stroke_path(&p, &paint);
    }

    pub fn headphones(&mut self, cx: f32, cy: f32, color: Color) {
        let mut points = Vec::new();
        for i in 0..=12 {
            let angle = std::f32::consts::PI + std::f32::consts::PI * i as f32 / 12.0;
            points.push((cx + 6.0 * angle.cos(), cy + 1.0 + 6.0 * angle.sin()));
        }
        self.poly(&points, color, 1.4);
        self.rect(cx - 7.5, cy, 3.0, 6.0, color);
        self.rect(cx + 4.5, cy, 3.0, 6.0, color);
    }

    pub fn cog_icon(&mut self, cx: f32, cy: f32, color: Color) {
        let mut p = Path::new();
        let r_outer = 6.6 * self.s;
        let r_inner = 4.18 * self.s;
        let ox = self.ox + (cx + self.offset_x) * self.s;
        let oy = self.oy + cy * self.s;
        for i in 0..6 {
            let mid = (i as f32 * 60.0).to_radians();
            let a1 = mid - 21.0_f32.to_radians();
            let a2 = mid - 12.0_f32.to_radians();
            let a3 = mid + 12.0_f32.to_radians();
            let a4 = mid + 21.0_f32.to_radians();
            if i == 0 {
                p.move_to(ox + r_inner * a1.cos(), oy + r_inner * a1.sin());
            } else {
                p.line_to(ox + r_inner * a1.cos(), oy + r_inner * a1.sin());
            }
            p.line_to(ox + r_outer * a2.cos(), oy + r_outer * a2.sin());
            p.line_to(ox + r_outer * a3.cos(), oy + r_outer * a3.sin());
            p.line_to(ox + r_inner * a4.cos(), oy + r_inner * a4.sin());
        }
        p.close();
        self.c.fill_path(&p, &Paint::color(self.color(color)));
        self.circle(cx, cy, 1.98, PANEL, true);
    }
}

impl Draw<'_> {
    /// Shared graph zoom/restore corner glyphs, originally drawn by Flattery.
    pub fn graph_zoom_button(
        &mut self,
        r: (f32, f32, f32, f32),
        inward: bool,
        hot: bool,
        enabled: bool,
    ) {
        let fade = if enabled { 1.0 } else { 0.32 };
        let mut panel = PANEL;
        panel.a *= fade;
        let mut line = LINE;
        line.a *= fade;
        self.rounded_rect(r.0, r.1, r.2, r.3, 4.0, panel);
        self.outline(r, line);
        let color = if !enabled {
            Color { a: fade, ..MUTED }
        } else if hot {
            TEXT
        } else {
            MUTED
        };
        let (x, y, w, h) = r;
        let arm = 5.0;
        if inward {
            let m = 4.5;
            self.poly(
                &[(x + m, y + m + arm), (x + m, y + m), (x + m + arm, y + m)],
                color,
                1.3,
            );
            self.poly(
                &[
                    (x + w - m - arm, y + m),
                    (x + w - m, y + m),
                    (x + w - m, y + m + arm),
                ],
                color,
                1.3,
            );
            self.poly(
                &[
                    (x + m, y + h - m - arm),
                    (x + m, y + h - m),
                    (x + m + arm, y + h - m),
                ],
                color,
                1.3,
            );
            self.poly(
                &[
                    (x + w - m - arm, y + h - m),
                    (x + w - m, y + h - m),
                    (x + w - m, y + h - m - arm),
                ],
                color,
                1.3,
            );
        } else {
            let m = 3.0;
            let inset = 8.0;
            self.poly(
                &[
                    (x + m, y + inset),
                    (x + inset, y + inset),
                    (x + inset, y + m),
                ],
                color,
                1.3,
            );
            self.poly(
                &[
                    (x + w - inset, y + m),
                    (x + w - inset, y + inset),
                    (x + w - m, y + inset),
                ],
                color,
                1.3,
            );
            self.poly(
                &[
                    (x + inset, y + h - m),
                    (x + inset, y + h - inset),
                    (x + m, y + h - inset),
                ],
                color,
                1.3,
            );
            self.poly(
                &[
                    (x + w - m, y + h - inset),
                    (x + w - inset, y + h - inset),
                    (x + w - inset, y + h - m),
                ],
                color,
                1.3,
            );
        }
    }
}
