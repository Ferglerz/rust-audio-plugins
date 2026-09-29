use super::Draw;
use crate::theme::PANEL;
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

    /// Rounded trackpad with a contact point; pairs with a short "Latch" label.
    pub fn trackpad_icon(&mut self, cx: f32, cy: f32, color: Color) {
        self.outline_rounded(cx - 6.5, cy - 4.5, 13.0, 9.0, 2.4, color, 1.3);
        self.circle(cx + 1.0, cy - 0.2, 1.35, color, true);
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
