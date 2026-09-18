use crate::theme::{self, rgb, LINE, MUTED, PANEL, TEXT};
use nih_plug_vizia::vizia::{
    prelude::*,
    vg::{Color, FontId, Paint, Path},
};

pub struct Draw<'a> {
    pub light: bool,
    pub c: &'a mut Canvas,
    pub s: f32,
    pub ox: f32,
    pub oy: f32,
    pub font: Option<FontId>,
    pub offset_x: f32,
    pub alpha_mul: f32,
}

impl<'a> Draw<'a> {
    pub fn new(
        c: &'a mut Canvas,
        light: bool,
        s: f32,
        ox: f32,
        oy: f32,
        font: Option<FontId>,
    ) -> Self {
        Self {
            light,
            c,
            s,
            ox,
            oy,
            font,
            offset_x: 0.0,
            alpha_mul: 1.0,
        }
    }

    pub fn color(&self, c: Color) -> Color {
        let mut col = theme::transform_color(c, self.light);
        col.a = (col.a * self.alpha_mul).clamp(0.0, 1.0);
        col
    }

    pub fn scissor(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.c.scissor(
            self.ox + x * self.s,
            self.oy + y * self.s,
            w * self.s,
            h * self.s,
        );
    }

    pub fn reset_scissor(&mut self) {
        self.c.reset_scissor();
    }

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

    pub fn area(&mut self, points: &[(f32, f32)], bottom: f32, color: Color) {
        if points.is_empty() {
            return;
        }
        let mut p = Path::new();
        p.move_to(
            self.ox + (points[0].0 + self.offset_x) * self.s,
            self.oy + bottom * self.s,
        );
        for (x, y) in points {
            p.line_to(self.ox + (x + self.offset_x) * self.s, self.oy + y * self.s);
        }
        p.line_to(
            self.ox + (points.last().unwrap().0 + self.offset_x) * self.s,
            self.oy + bottom * self.s,
        );
        p.close();
        self.c.fill_path(&p, &Paint::color(self.color(color)));
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

    pub fn text(&mut self, x: f32, y: f32, text: &str, size: f32, color: Color) {
        let mut p = Paint::color(self.color(color));
        if let Some(font) = self.font {
            p.set_font(&[font]);
        }
        p.set_font_size(size * self.s);
        let _ = self.c.fill_text(
            self.ox + (x + self.offset_x) * self.s,
            self.oy + y * self.s,
            text,
            &p,
        );
    }

    pub fn text_centered(&mut self, cx: f32, y: f32, text: &str, size: f32, color: Color) {
        let mut p = Paint::color(self.color(color));
        if let Some(font) = self.font {
            p.set_font(&[font]);
        }
        p.set_font_size(size * self.s);
        let width = if let Ok(m) = self.c.measure_text(0.0, 0.0, text, &p) {
            m.width() / self.s
        } else {
            text.len() as f32 * size * 0.55
        };
        let _ = self.c.fill_text(
            self.ox + (cx + self.offset_x - width * 0.5) * self.s,
            self.oy + y * self.s,
            text,
            &p,
        );
    }

    pub fn button(&mut self, r: (f32, f32, f32, f32), label: &str, on: bool, color: Color) {
        self.button_aligned(r, label, on, color, true);
    }

    pub fn button_left(&mut self, r: (f32, f32, f32, f32), label: &str, on: bool, color: Color) {
        self.button_aligned(r, label, on, color, false);
    }

    fn button_aligned(
        &mut self,
        r: (f32, f32, f32, f32),
        label: &str,
        on: bool,
        color: Color,
        centered: bool,
    ) {
        self.rect(r.0, r.1, r.2, r.3, PANEL);
        if on {
            self.rect(r.0, r.1 + r.3 - 2.0, r.2, 2.0, color);
        }
        let label_color = if on { color } else { MUTED };
        if centered {
            self.text_centered(
                r.0 + r.2 * 0.5,
                r.1 + r.3 / 2.0 + 4.0,
                label,
                11.0,
                label_color,
            );
        } else {
            self.text(r.0 + 10.0, r.1 + r.3 / 2.0 + 4.0, label, 11.0, label_color);
        }
    }

    pub fn tab_button(&mut self, r: (f32, f32, f32, f32), label: &str, on: bool, color: Color) {
        self.rect(r.0, r.1, r.2, r.3, PANEL);
        if on {
            self.rect(r.0, r.1 + r.3 - 2.0, r.2, 2.0, color);
        }
        self.text_centered(
            r.0 + r.2 * 0.5,
            r.1 + r.3 / 2.0 + 4.0,
            label,
            11.0,
            if on { color } else { MUTED },
        );
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

    pub fn grab_bar(&mut self, r: (f32, f32, f32, f32), color: Color) {
        self.rect(r.0, r.1, r.2, r.3, color);
        let grip = Color {
            r: color.r * 0.35,
            g: color.g * 0.35,
            b: color.b * 0.35,
            a: 1.0,
        };
        let cx = r.0 + r.2 * 0.5;
        let cy = r.1 + r.3 * 0.5;
        let line_w = (r.2 * 0.38).min(22.0);
        self.line(
            cx - line_w * 0.5,
            cy - 2.2,
            cx + line_w * 0.5,
            cy - 2.2,
            grip,
            1.4,
        );
        self.line(
            cx - line_w * 0.5,
            cy + 2.2,
            cx + line_w * 0.5,
            cy + 2.2,
            grip,
            1.4,
        );
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

    pub fn area_gradient_span(
        &mut self,
        points: &[(f32, f32)],
        bottom: f32,
        sx: f32,
        ex: f32,
        c1: Color,
        c2: Color,
    ) {
        if points.is_empty() {
            return;
        }
        let mut p = Path::new();
        p.move_to(
            self.ox + (points[0].0 + self.offset_x) * self.s,
            self.oy + bottom * self.s,
        );
        for (x, y) in points {
            p.line_to(self.ox + (x + self.offset_x) * self.s, self.oy + y * self.s);
        }
        p.line_to(
            self.ox + (points.last().unwrap().0 + self.offset_x) * self.s,
            self.oy + bottom * self.s,
        );
        p.close();
        let paint = Paint::linear_gradient(
            self.ox + (sx + self.offset_x) * self.s,
            self.oy,
            self.ox + (ex + self.offset_x) * self.s,
            self.oy,
            self.color(c1),
            self.color(c2),
        );
        self.c.fill_path(&p, &paint);
    }

    pub fn bypass_button(&mut self, r: (f32, f32, f32, f32), bypassed: bool, color: Color) {
        self.rect(r.0, r.1, r.2, r.3, PANEL);
        self.outline(r, if bypassed { LINE } else { color });
        self.power_icon(
            r.0 + r.2 * 0.5,
            r.1 + r.3 * 0.5,
            if bypassed { MUTED } else { color },
        );
    }

    pub fn power_icon(&mut self, cx: f32, cy: f32, color: Color) {
        let mut points = Vec::new();
        for i in 0..=16 {
            let angle = -std::f32::consts::FRAC_PI_2
                + 0.8
                + (std::f32::consts::TAU - 1.6) * i as f32 / 16.0;
            points.push((cx + 5.5 * angle.cos(), cy + 5.5 * angle.sin()));
        }
        self.poly(&points, color, 1.4);
        self.line(cx, cy - 6.5, cx, cy - 1.0, color, 1.4);
    }

    pub fn close_icon(&mut self, cx: f32, cy: f32, color: Color) {
        self.line(cx - 4.0, cy - 4.0, cx + 4.0, cy + 4.0, color, 1.4);
        self.line(cx + 4.0, cy - 4.0, cx - 4.0, cy + 4.0, color, 1.4);
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
        let r_outer = 6.0 * self.s;
        let r_inner = 3.8 * self.s;
        let ox = self.ox + (cx + self.offset_x) * self.s;
        let oy = self.oy + cy * self.s;
        for i in 0..6 {
            let mid = (i as f32 * 60.0).to_radians();
            let a1 = mid - 18.0_f32.to_radians();
            let a2 = mid - 8.0_f32.to_radians();
            let a3 = mid + 8.0_f32.to_radians();
            let a4 = mid + 18.0_f32.to_radians();
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
        self.circle(cx, cy, 1.8, PANEL, true);
    }

    pub fn knob(
        &mut self,
        r: (f32, f32, f32, f32),
        label: &str,
        value: &str,
        n: f32,
        color: Color,
        bypassed: bool,
    ) {
        let cx = r.0 + r.2 * 0.5;
        let cy = r.1 + 45.0;
        let rad = 17.0;

        self.text_centered(
            cx,
            r.1 + 22.0,
            label,
            9.2,
            if bypassed { MUTED } else { TEXT },
        );

        let start_angle = 135.0_f32.to_radians();
        let total_sweep = 270.0_f32.to_radians();
        let cur_angle = start_angle + total_sweep * n.clamp(0.0, 1.0);

        let mut bg_pts = Vec::with_capacity(25);
        for i in 0..=24 {
            let a = start_angle + total_sweep * (i as f32 / 24.0);
            bg_pts.push((cx + rad * a.cos(), cy + rad * a.sin()));
        }
        self.poly(&bg_pts, LINE, 2.5);

        if !bypassed && n > 0.005 {
            let steps = ((24.0 * n).ceil() as usize).max(2);
            let mut val_pts = Vec::with_capacity(steps + 1);
            for i in 0..=steps {
                let a = start_angle + (cur_angle - start_angle) * (i as f32 / steps as f32);
                val_pts.push((cx + rad * a.cos(), cy + rad * a.sin()));
            }
            self.poly(&val_pts, color, 2.5);
        }

        self.circle(cx, cy, 13.0, rgb(28, 33, 40), true);
        self.circle(
            cx,
            cy,
            13.0,
            if bypassed {
                LINE
            } else {
                Color {
                    r: (rgb(48, 56, 68).r + color.r * 0.25).min(1.0),
                    g: (rgb(48, 56, 68).g + color.g * 0.25).min(1.0),
                    b: (rgb(48, 56, 68).b + color.b * 0.25).min(1.0),
                    a: 1.0,
                }
            },
            false,
        );

        let ind_color = if bypassed { MUTED } else { color };
        let nx = cx + 11.5 * cur_angle.cos();
        let ny = cy + 11.5 * cur_angle.sin();
        let n_inner_x = cx + 5.0 * cur_angle.cos();
        let n_inner_y = cy + 5.0 * cur_angle.sin();
        self.line(n_inner_x, n_inner_y, nx, ny, ind_color, 2.0);

        self.text_centered(
            cx,
            r.1 + 75.0,
            value,
            11.0,
            if bypassed { MUTED } else { color },
        );
    }

    pub fn control(
        &mut self,
        r: (f32, f32, f32, f32),
        label: &str,
        value: &str,
        n: f32,
        color: Color,
    ) {
        self.rect(r.0, r.1, r.2, r.3, Color::rgba(27, 31, 37, 225));
        self.outline(
            r,
            Color {
                r: (LINE.r + color.r * 0.3).min(1.0),
                g: (LINE.g + color.g * 0.3).min(1.0),
                b: (LINE.b + color.b * 0.3).min(1.0),
                a: 1.0,
            },
        );
        self.text(r.0 + 12.0, r.1 + 18.0, label, 9.5, MUTED);
        let mut paint = Paint::color(self.color(TEXT));
        if let Some(font) = self.font {
            paint.set_font(&[font]);
        }
        paint.set_font_size(11.0 * self.s);
        let width = self
            .c
            .measure_text(0.0, 0.0, value, &paint)
            .map(|m| m.width() / self.s)
            .unwrap_or(70.0);
        self.text(r.0 + r.2 - 12.0 - width, r.1 + 18.0, value, 11.0, TEXT);
        let y = r.1 + 28.0;
        self.rect(r.0 + 12.0, y, r.2 - 24.0, 16.0, LINE);
        self.rect(r.0 + 12.0, y, (r.2 - 24.0) * n.clamp(0.0, 1.0), 16.0, color);
    }
}
