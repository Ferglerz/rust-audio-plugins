use super::Draw;
use crate::theme::{rgb, LINE, MUTED, PANEL, TEXT};
use nih_plug_vizia::vizia::vg::{Color, Paint};

/// Shared geometry for knob drawing, captions, and editable readouts.
pub struct KnobLayout {
    pub cx: f32,
    pub cy: f32,
    pub radius: f32,
    pub label_y: f32,
    pub value_y: f32,
    pub label_size: f32,
    pub value_size: f32,
    bottom: f32,
}

impl KnobLayout {
    pub fn new(r: (f32, f32, f32, f32)) -> Self {
        // Widget bounds can pick up subpixel rounding at fractional editor scales.
        let compact = r.2 <= 64.5 && r.3 <= 96.5;
        let (cy, radius, label_y, value_y, label_size, value_size) = if compact {
            (r.1 + 45.0, 17.0, r.1 + 22.0, r.1 + 75.0, 9.2, 11.0)
        } else {
            let label_y = r.1 + 16.0;
            let value_y = r.1 + r.3 - 12.0;
            let cy = (label_y + value_y) * 0.5;
            let radius = ((cy - label_y - 8.0).min(r.2 * 0.38)).clamp(18.0, 42.0);
            (cy, radius, label_y, value_y, 11.0, 13.0)
        };
        Self {
            cx: r.0 + r.2 * 0.5,
            cy,
            radius,
            // Match the breathing room of Damian's taller knob slots.
            label_y: label_y.min(cy - radius - 14.0),
            value_y,
            label_size,
            value_size,
            bottom: r.1 + r.3,
        }
        .with_text_sizes(label_size, value_size)
    }

    pub fn with_text_sizes(mut self, label_size: f32, value_size: f32) -> Self {
        self.label_size = label_size;
        self.value_size = value_size;
        // Baselines must leave room for the full readout above them.
        self.value_y = self
            .value_y
            .max(self.cy + self.radius + value_size + 4.0)
            .min(self.bottom - crate::value_edit::VALUE_UNDERLINE_OFFSET - 1.0);
        self.radius = self.radius.min(self.value_y - self.cy - value_size - 4.0);
        self
    }

    pub fn value_rect(&self, x: f32, width: f32) -> (f32, f32, f32, f32) {
        (
            x,
            self.value_y - self.value_size - 3.0,
            width,
            self.value_size + 9.0,
        )
    }
}

impl Draw<'_> {
    /// Shared small-radius button face and curved selection accent.
    /// Animation amounts are in 0..=1. Reserve the lower 2 px for the offset
    /// backing so the highlight stays visible inside clipped widget bounds.
    pub fn button_surface(
        &mut self,
        r: (f32, f32, f32, f32),
        color: Color,
        selected: f32,
        hover: f32,
        press: f32,
    ) {
        let selected = selected.clamp(0.0, 1.0);
        let offset = 2.0_f32.min(r.3 * 0.5);
        let height = r.3 - offset;
        if selected > 0.0 {
            let mut accent = color;
            accent.a *= selected;
            self.rect(r.0, r.1 + offset, r.2, height, accent);
        }
        // The opaque face hides the backing except around its lower corners.
        self.rect(r.0, r.1, r.2, height, LINE);
        let mut tint = if selected > 0.0 && color != MUTED {
            color
        } else {
            TEXT
        };
        tint.a *= hover.clamp(0.0, 1.0) * 0.08 + press.clamp(0.0, 1.0) * 0.07 + selected * 0.08;
        self.rect(r.0, r.1, r.2, height, tint);
    }

    pub fn button(&mut self, r: (f32, f32, f32, f32), label: &str, on: bool, color: Color) {
        self.button_aligned(r, label, on, color, true);
    }

    pub fn appearance_button(&mut self, r: (f32, f32, f32, f32), label: &str) {
        const TEXT_SIZE: f32 = 13.0;
        const ICON_WIDTH: f32 = 14.0;
        const GAP: f32 = 6.0;
        let mut paint = Paint::color(self.color(MUTED));
        if let Some(font) = self.font {
            paint.set_font(&[font]);
        }
        paint.set_font_size(TEXT_SIZE * self.s);
        let text_width = self
            .c
            .measure_text(0.0, 0.0, label, &paint)
            .map(|metrics| metrics.width() / self.s)
            .unwrap_or(label.len() as f32 * TEXT_SIZE * 0.6);
        let left = r.0 + (r.2 - ICON_WIDTH - GAP - text_width) * 0.5;
        let center_y = r.1 + r.3 * 0.5;
        self.button_surface(r, MUTED, 0.0, self.is_hovered(r) as u8 as f32, 0.0);
        self.palette_icon(left + ICON_WIDTH * 0.5, center_y, MUTED);
        self.text_middle(left + ICON_WIDTH + GAP, center_y, label, TEXT_SIZE, MUTED);
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
        let hovered = self.is_hovered(r);
        self.button_surface(
            r,
            if !on && color == MUTED { TEXT } else { color },
            if on {
                1.0
            } else if hovered {
                0.5
            } else {
                0.0
            },
            hovered as u8 as f32,
            0.0,
        );
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

    pub fn button_tinted(&mut self, r: (f32, f32, f32, f32), label: &str, on: bool, color: Color) {
        self.button_tinted_aligned(r, label, on, color, true);
    }

    pub fn button_tinted_left(
        &mut self,
        r: (f32, f32, f32, f32),
        label: &str,
        on: bool,
        color: Color,
    ) {
        self.button_tinted_aligned(r, label, on, color, false);
    }

    fn button_tinted_aligned(
        &mut self,
        r: (f32, f32, f32, f32),
        label: &str,
        on: bool,
        color: Color,
        centered: bool,
    ) {
        self.button_surface(
            r,
            color,
            on as u8 as f32,
            self.is_hovered(r) as u8 as f32,
            0.0,
        );
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
        self.button_aligned(r, label, on, color, true);
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

    pub fn knob(
        &mut self,
        r: (f32, f32, f32, f32),
        label: &str,
        value: &str,
        n: f32,
        color: Color,
        bypassed: bool,
    ) {
        let layout = KnobLayout::new(r);
        self.knob_with_layout(&layout, label, value, n, color, bypassed);
    }

    pub fn knob_with_layout(
        &mut self,
        layout: &KnobLayout,
        label: &str,
        value: &str,
        n: f32,
        color: Color,
        bypassed: bool,
    ) {
        self.draw_knob_with_layout(layout, label, value, n, color, bypassed, false);
    }

    pub fn knob_with_layout_bipolar(
        &mut self,
        layout: &KnobLayout,
        label: &str,
        value: &str,
        n: f32,
        color: Color,
        bypassed: bool,
    ) {
        self.draw_knob_with_layout(layout, label, value, n, color, bypassed, true);
    }

    fn draw_knob_with_layout(
        &mut self,
        layout: &KnobLayout,
        label: &str,
        value: &str,
        n: f32,
        color: Color,
        bypassed: bool,
        bipolar: bool,
    ) {
        let KnobLayout {
            cx,
            cy,
            radius: rad,
            label_y,
            value_y,
            label_size,
            value_size,
            ..
        } = *layout;
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

        let value_start = if bipolar {
            start_angle + total_sweep * 0.5
        } else {
            start_angle
        };
        let value_sweep = cur_angle - value_start;
        let draw_value = if bipolar {
            value_sweep.abs() > 0.001
        } else {
            n > 0.005
        };
        if !bypassed && draw_value {
            let extent = if bipolar {
                value_sweep.abs() / total_sweep
            } else {
                n
            };
            let steps = ((24.0 * extent).ceil() as usize).max(2);
            let mut val_pts = Vec::with_capacity(steps + 1);
            for i in 0..=steps {
                let a = value_start + value_sweep * (i as f32 / steps as f32);
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
        let layout = KnobLayout::new(r);
        let (cx, cy, rad) = (layout.cx, layout.cy, layout.radius);
        let scale = rad / 17.0;

        self.text_centered(
            cx,
            layout.label_y,
            label,
            layout.label_size,
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
            layout.value_y,
            value,
            layout.value_size,
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
        let y = r.1 + 28.0 + crate::value_edit::SLIDER_SPACING_EXTRA;
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

    /// Position from the text baseline, independently of the click/edit bounds.
    pub fn value_underline(&mut self, r: (f32, f32, f32, f32), baseline: f32, color: Color) {
        self.line(
            r.0 + 4.0,
            crate::value_edit::value_underline_y(baseline),
            r.0 + r.2 - 4.0,
            crate::value_edit::value_underline_y(baseline),
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
mod knob_layout_tests {
    use super::KnobLayout;

    #[test]
    fn knob_captions_clear_arcs_and_stay_inside_slots() {
        for (w, h) in [
            (64.0, 96.0),
            (64.00001, 96.00001),
            (108.0, 108.0),
            (109.0, 109.0),
            (120.0, 180.0),
        ] {
            for (label_size, value_size) in [(9.2, 11.0), (13.0, 15.0)] {
                let layout =
                    KnobLayout::new((0.0, 0.0, w, h)).with_text_sizes(label_size, value_size);
                assert!(layout.label_y <= layout.cy - layout.radius - 14.0);
                assert!(layout.value_y - value_size >= layout.cy + layout.radius + 4.0);
                let rect = layout.value_rect(0.0, w);
                assert!(rect.1 + rect.3 <= h);
                assert!(crate::value_edit::value_underline_y(layout.value_y) + 0.5 < h);
            }
        }
        assert_eq!(KnobLayout::new((0.0, 0.0, 64.00001, 96.00001)).cy, 45.0);
    }
}
