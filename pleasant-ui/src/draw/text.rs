use super::Draw;
use nih_plug_vizia::vizia::vg::{Baseline, Color, FontId, Paint};

impl Draw<'_> {
    // femtovg falls back per word, not per glyph. Split SMuFL symbols explicitly.
    fn text_runs<'t>(&self, text: &'t str) -> Vec<(&'t str, Option<FontId>)> {
        let mut runs = Vec::new();
        let mut start = 0;
        let mut previous = self.font;
        for (offset, ch) in text.char_indices() {
            let font = if ('\u{e000}'..='\u{f8ff}').contains(&ch) {
                self.fallback_font.or(self.font)
            } else {
                self.font
            };
            if offset > start && font != previous {
                runs.push((&text[start..offset], previous));
                start = offset;
            }
            previous = font;
        }
        if start < text.len() {
            runs.push((&text[start..], previous));
        }
        runs
    }

    fn text_paint(&self, font: Option<FontId>, size: f32, color: Color) -> Paint {
        let mut paint = Paint::color(self.color(color));
        if let Some(font) = font {
            paint.set_font(&[font]);
        }
        paint.set_font_size(size * self.s);
        paint
    }

    /// Width in logical pixels, using the same runs as the drawing path.
    pub fn text_width(&self, text: &str, size: f32) -> f32 {
        self.text_runs(text)
            .into_iter()
            .map(|(run, font)| {
                let paint = self.text_paint(font, size, Color::rgb(0, 0, 0));
                self.c
                    .measure_text(0.0, 0.0, run, &paint)
                    .map_or(run.chars().count() as f32 * size * 0.55, |m| {
                        m.width() / self.s
                    })
            })
            .sum()
    }

    fn draw_text_runs(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        color: Color,
        middle: bool,
    ) {
        let mut x = self.ox + (x + self.offset_x) * self.s;
        let y = self.oy + y * self.s;
        for (run, font) in self.text_runs(text) {
            let mut paint = self.text_paint(font, size, color);
            if middle {
                paint.set_text_baseline(Baseline::Middle);
            }
            let width = self
                .c
                .measure_text(0.0, 0.0, run, &paint)
                .map_or(run.chars().count() as f32 * size * self.s * 0.55, |m| {
                    m.width()
                });
            let _ = self.c.fill_text(x, y, run, &paint);
            x += width;
        }
    }

    pub fn text(&mut self, x: f32, y: f32, text: &str, size: f32, color: Color) {
        self.draw_text_runs(x, y, text, size, color, false);
    }

    /// Like [`text`](Self::text), but `y` is the vertical center of the glyph (Middle baseline).
    pub fn text_middle(&mut self, x: f32, y: f32, text: &str, size: f32, color: Color) {
        self.draw_text_runs(x, y, text, size, color, true);
    }

    pub fn text_centered(&mut self, cx: f32, y: f32, text: &str, size: f32, color: Color) {
        let width = self.text_width(text, size);
        self.text(cx - width * 0.5, y, text, size, color);
    }

    pub fn text_right(&mut self, x: f32, y: f32, text: &str, size: f32, color: Color) {
        let width = self.text_width(text, size);
        self.text(x - width, y, text, size, color);
    }
}
