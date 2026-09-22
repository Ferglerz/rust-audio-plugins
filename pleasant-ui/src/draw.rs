use crate::handles::{tag_body_rect, tag_vertical_grips, TagPointer, TAG_POINTER_BASE, TAG_RADIUS};
use crate::theme::{self, rgb, LINE, MUTED, PANEL, TEXT};
use nih_plug_vizia::vizia::{
    prelude::*,
    vg::{Color, FontId, Paint, Path},
};

#[derive(Debug, Default)]
pub struct ButtonAnim {
    click: std::cell::Cell<f32>,
}

impl ButtonAnim {
    pub const fn new() -> Self {
        Self {
            click: std::cell::Cell::new(0.0),
        }
    }

    pub fn trigger_click(&self) {
        self.click.set(1.0);
    }

    pub fn step(&self) -> f32 {
        let val = self.click.get();
        if val > 0.005 {
            self.click.set((val - 0.08).max(0.0));
        }
        val
    }
}

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
        self.circle(x, y_range, 3.0, color, true);
    }

    /// Filled pill used to meter live gain reduction/increase inside the range outline.
    pub fn pill_fill(&mut self, x: f32, y: f32, y_amount: f32, r: f32, color: Color) {
        let Some(p) = self.pill_path(x, y, y_amount, r) else {
            return;
        };
        self.c.fill_path(&p, &Paint::color(self.color(color)));
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

    pub fn text_right(&mut self, x: f32, y: f32, text: &str, size: f32, color: Color) {
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
            self.ox + (x + self.offset_x - width) * self.s,
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

    fn grip_color(color: Color) -> Color {
        Color {
            r: color.r * 0.35,
            g: color.g * 0.35,
            b: color.b * 0.35,
            a: 1.0,
        }
    }

    pub fn grip_lines(&mut self, r: (f32, f32, f32, f32), color: Color, vertical: bool) {
        let grip = Self::grip_color(color);
        let cx = r.0 + r.2 * 0.5;
        let cy = r.1 + r.3 * 0.5;
        if vertical {
            let line_h = (r.3 * 0.42).min(10.0);
            self.line(
                cx - 2.2,
                cy - line_h * 0.5,
                cx - 2.2,
                cy + line_h * 0.5,
                grip,
                1.4,
            );
            self.line(
                cx + 2.2,
                cy - line_h * 0.5,
                cx + 2.2,
                cy + line_h * 0.5,
                grip,
                1.4,
            );
        } else {
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
    }

    pub fn grab_bar(&mut self, r: (f32, f32, f32, f32), color: Color) {
        self.rect(r.0, r.1, r.2, r.3, color);
        self.grip_lines(r, color, false);
    }

    /// Rounded grab tag with an optional triangle pointing at `anchor`.
    pub fn tag_handle(&mut self, anchor_x: f32, anchor_y: f32, pointer: TagPointer, color: Color) {
        let body = tag_body_rect(anchor_x, anchor_y, pointer);
        self.rounded_rect(body.0, body.1, body.2, body.3, TAG_RADIUS, color);

        match pointer {
            TagPointer::Down => {
                let base_y = body.1 + body.3 - 1.5;
                self.fill_poly(
                    &[
                        (anchor_x - TAG_POINTER_BASE * 0.5, base_y),
                        (anchor_x + TAG_POINTER_BASE * 0.5, base_y),
                        (anchor_x, anchor_y),
                    ],
                    color,
                );
            }
            TagPointer::Right => {
                let base_x = body.0 + body.2 - 1.5;
                self.fill_poly(
                    &[
                        (base_x, anchor_y - TAG_POINTER_BASE * 0.5),
                        (base_x, anchor_y + TAG_POINTER_BASE * 0.5),
                        (anchor_x, anchor_y),
                    ],
                    color,
                );
            }
            TagPointer::None => {}
        }

        self.grip_lines(body, color, tag_vertical_grips(pointer));
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

    pub fn bypass_button(
        &mut self,
        r: (f32, f32, f32, f32),
        bypassed: bool,
        color: Color,
        hovered: bool,
        click_amt: f32,
    ) {
        let base_color = if bypassed { MUTED } else { color };
        let now_sec = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            % 100_000) as f32
            * 0.001;
        let pulse = if hovered {
            (now_sec * 4.5).sin() * 0.5 + 0.5
        } else {
            0.0
        };

        // Background panel
        self.rect(r.0, r.1, r.2, r.3, PANEL);

        // Subtle hover / click interior tint
        if hovered || click_amt > 0.01 {
            let mut tint = base_color;
            tint.a = (0.04 + 0.06 * pulse + click_amt * 0.2).clamp(0.0, 1.0);
            self.rounded_rect(r.0, r.1, r.2, r.3, 5.0, tint);
        }

        // Hover outer halo
        if hovered || click_amt > 0.01 {
            let halo_expand = 1.5 + pulse * 2.0 + click_amt * 4.0;
            let mut halo_col = base_color;
            halo_col.a = (0.10 + 0.15 * pulse + click_amt * 0.35).clamp(0.0, 1.0);
            self.outline_rounded(
                r.0 - halo_expand,
                r.1 - halo_expand,
                r.2 + 2.0 * halo_expand,
                r.3 + 2.0 * halo_expand,
                5.0 + halo_expand * 0.5,
                halo_col,
                1.0,
            );
        }

        // Click burst ripple ring
        if click_amt > 0.05 {
            let rip_progress = 1.0 - click_amt;
            let rip_expand = 1.0 + rip_progress * 8.0;
            let mut rip_col = base_color;
            rip_col.a = (click_amt * 0.6).clamp(0.0, 1.0);
            self.outline_rounded(
                r.0 - rip_expand,
                r.1 - rip_expand,
                r.2 + 2.0 * rip_expand,
                r.3 + 2.0 * rip_expand,
                5.0 + rip_expand * 0.5,
                rip_col,
                1.2,
            );
        }

        // Main outline
        let mut border_col = if bypassed {
            if hovered {
                TEXT
            } else {
                LINE
            }
        } else {
            color
        };
        if hovered && !bypassed {
            border_col.a = (0.8 + 0.2 * pulse).clamp(0.0, 1.0);
        }
        self.outline(r, border_col);

        // Power icon with click pop
        let icon_col = if bypassed {
            if hovered {
                TEXT
            } else {
                MUTED
            }
        } else {
            color
        };
        if click_amt > 0.01 {
            let scale = 1.0 + click_amt * 0.15;
            let line_width = 1.4 + click_amt * 0.4;
            self.power_icon_scaled(
                r.0 + r.2 * 0.5,
                r.1 + r.3 * 0.5,
                icon_col,
                scale,
                line_width,
            );
        } else {
            self.power_icon(r.0 + r.2 * 0.5, r.1 + r.3 * 0.5, icon_col);
        }
    }

    pub fn bypass_button_static(&mut self, r: (f32, f32, f32, f32), bypassed: bool, color: Color) {
        self.bypass_button(r, bypassed, color, false, 0.0);
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

    pub fn knob(
        &mut self,
        r: (f32, f32, f32, f32),
        label: &str,
        value: &str,
        n: f32,
        color: Color,
        bypassed: bool,
    ) {
        let compact = r.2 <= 64.0 && r.3 <= 96.0;
        let cx = r.0 + r.2 * 0.5;
        let (cy, rad, label_y, value_y, label_size, value_size) = if compact {
            (r.1 + 45.0, 17.0, r.1 + 22.0, r.1 + 75.0, 9.2, 11.0)
        } else {
            let label_y = r.1 + 16.0;
            let value_y = r.1 + r.3 - 12.0;
            let cy = (label_y + value_y) * 0.5;
            let rad = ((cy - label_y - 8.0).min(r.2 * 0.38)).clamp(18.0, 42.0);
            (cy, rad, label_y, value_y, 11.0, 13.0)
        };
        let scale = rad / 17.0;

        self.text_centered(
            cx,
            label_y,
            label,
            label_size,
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
        self.poly(&bg_pts, LINE, 2.5 * scale);

        if !bypassed && n > 0.005 {
            let steps = ((24.0 * n).ceil() as usize).max(2);
            let mut val_pts = Vec::with_capacity(steps + 1);
            for i in 0..=steps {
                let a = start_angle + (cur_angle - start_angle) * (i as f32 / steps as f32);
                val_pts.push((cx + rad * a.cos(), cy + rad * a.sin()));
            }
            self.poly(&val_pts, color, 2.5 * scale);
        }

        self.circle(cx, cy, 13.0 * scale, rgb(28, 33, 40), true);
        self.circle(
            cx,
            cy,
            13.0 * scale,
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
        let nx = cx + 11.5 * scale * cur_angle.cos();
        let ny = cy + 11.5 * scale * cur_angle.sin();
        let n_inner_x = cx + 5.0 * scale * cur_angle.cos();
        let n_inner_y = cy + 5.0 * scale * cur_angle.sin();
        self.line(n_inner_x, n_inner_y, nx, ny, ind_color, 2.0 * scale);

        self.text_centered(
            cx,
            value_y,
            value,
            value_size,
            if bypassed { MUTED } else { color },
        );
    }

    pub fn knob_bipolar(
        &mut self,
        r: (f32, f32, f32, f32),
        label: &str,
        value: &str,
        n: f32,
        color: Color,
        bypassed: bool,
    ) {
        let cx = r.0 + r.2 * 0.5;
        let cy = r.1 + r.3 * 0.5;
        let rad = (r.2.min(r.3) * 0.27).clamp(17.0, 32.0);
        let scale = rad / 17.0;

        self.text_centered(
            cx,
            cy - rad - 8.0 * scale,
            label,
            9.2 * scale.min(1.25),
            if bypassed { MUTED } else { TEXT },
        );

        let start_angle = 135.0_f32.to_radians();
        let total_sweep = 270.0_f32.to_radians();
        let center_angle = start_angle + total_sweep * 0.5;
        let cur_angle = start_angle + total_sweep * n.clamp(0.0, 1.0);

        let mut bg_pts = Vec::with_capacity(25);
        for i in 0..=24 {
            let a = start_angle + total_sweep * (i as f32 / 24.0);
            bg_pts.push((cx + rad * a.cos(), cy + rad * a.sin()));
        }
        self.poly(&bg_pts, LINE, 2.5 * scale);

        if !bypassed && (cur_angle - center_angle).abs() > 0.01 {
            let (a_start, a_end) = if cur_angle > center_angle {
                (center_angle, cur_angle)
            } else {
                (cur_angle, center_angle)
            };
            let steps = (((a_end - a_start) / total_sweep * 24.0).ceil() as usize).max(2);
            let mut val_pts = Vec::with_capacity(steps + 1);
            for i in 0..=steps {
                let a = a_start + (a_end - a_start) * (i as f32 / steps as f32);
                val_pts.push((cx + rad * a.cos(), cy + rad * a.sin()));
            }
            self.poly(&val_pts, color, 2.5 * scale);
        }

        self.circle(cx, cy, 13.0 * scale, rgb(28, 33, 40), true);
        self.circle(
            cx,
            cy,
            13.0 * scale,
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
        let nx = cx + (rad - 5.5 * scale) * cur_angle.cos();
        let ny = cy + (rad - 5.5 * scale) * cur_angle.sin();
        let n_inner_x = cx + 5.0 * scale * cur_angle.cos();
        let n_inner_y = cy + 5.0 * scale * cur_angle.sin();
        self.line(n_inner_x, n_inner_y, nx, ny, ind_color, 2.0 * scale);

        self.text_centered(
            cx,
            cy + rad + 14.0 * scale,
            value,
            11.0 * scale.min(1.25),
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
        self.draw_control(r, label, value, n, color, false);
    }

    pub fn control_bipolar(
        &mut self,
        r: (f32, f32, f32, f32),
        label: &str,
        value: &str,
        n: f32,
        color: Color,
    ) {
        self.draw_control(r, label, value, n, color, true);
    }

    fn draw_control(
        &mut self,
        r: (f32, f32, f32, f32),
        label: &str,
        value: &str,
        n: f32,
        color: Color,
        bipolar: bool,
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
        let bx = r.0 + 12.0;
        let bw = r.2 - 24.0;
        let bh = 16.0;
        self.rect(bx, y, bw, bh, LINE);
        let n = n.clamp(0.0, 1.0);
        if bipolar {
            let center = bx + bw * 0.5;
            let pos = bx + bw * n;
            if (n - 0.5).abs() > 0.002 {
                let (fx, fw) = if n >= 0.5 {
                    (center, pos - center)
                } else {
                    (pos, center - pos)
                };
                self.rect(fx, y, fw.max(1.0), bh, color);
            }
            self.rect(center - 0.5, y, 1.0, bh, MUTED);
        } else {
            self.rect(bx, y, bw * n, bh, color);
        }
    }

    pub fn value_underline(&mut self, r: (f32, f32, f32, f32), color: Color) {
        self.line(
            r.0 + 4.0,
            r.1 + r.3 - 1.0,
            r.0 + r.2 - 4.0,
            r.1 + r.3 - 1.0,
            color,
            1.0,
        );
    }

    pub fn value_edit<T>(&mut self, edit: &crate::value_edit::ValueEdit<T>, accent: Color) {
        let r = edit.rect;
        self.rect(r.0, r.1, r.2, r.3, PANEL);
        self.outline(
            r,
            if edit.invalid {
                rgb(238, 110, 95)
            } else {
                accent
            },
        );
        let capacity = ((r.2 - 8.0) / 6.6) as usize;
        let start = edit.cursor.saturating_sub(capacity);
        let end = (start + capacity).min(edit.text.len());
        let selection = edit.selection();
        let left = selection.start.max(start).min(end);
        let right = selection.end.min(end).max(left);
        self.rect(
            r.0 + 4.0 + (left - start) as f32 * 6.6,
            r.1 + 2.0,
            (right - left) as f32 * 6.6,
            r.3 - 4.0,
            Color { a: 0.25, ..accent },
        );
        self.text(
            r.0 + 4.0,
            r.1 + r.3 * 0.5 + 4.0,
            &edit.text[start..end],
            11.0,
            TEXT,
        );
        let caret_x = r.0 + 4.0 + (edit.cursor - start) as f32 * 6.6;
        self.line(caret_x, r.1 + 3.0, caret_x, r.1 + r.3 - 3.0, accent, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_anim_trigger_and_decay() {
        let anim = ButtonAnim::new();
        assert_eq!(anim.step(), 0.0);

        anim.trigger_click();
        let val1 = anim.step();
        assert!((val1 - 1.0).abs() < 1e-6);

        let val2 = anim.step();
        assert!((val2 - 0.92).abs() < 1e-6);

        // Step until fully decayed
        for _ in 0..20 {
            anim.step();
        }
        assert_eq!(anim.step(), 0.0);
    }
}
