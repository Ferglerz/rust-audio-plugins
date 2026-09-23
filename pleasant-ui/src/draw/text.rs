use super::Draw;
use nih_plug_vizia::vizia::vg::{Baseline, Color, Paint};

impl Draw<'_> {
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

    /// Like [`text`](Self::text), but `y` is the vertical center of the glyph (Middle baseline).
    pub fn text_middle(&mut self, x: f32, y: f32, text: &str, size: f32, color: Color) {
        let mut p = Paint::color(self.color(color));
        if let Some(font) = self.font {
            p.set_font(&[font]);
        }
        p.set_font_size(size * self.s);
        p.set_text_baseline(Baseline::Middle);
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
}
