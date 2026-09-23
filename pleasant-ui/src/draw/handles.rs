use super::Draw;
use crate::handles::{tag_body_rect, tag_vertical_grips, TagPointer, TAG_POINTER_BASE, TAG_RADIUS};
use nih_plug_vizia::vizia::vg::Color;

impl Draw<'_> {
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
}
