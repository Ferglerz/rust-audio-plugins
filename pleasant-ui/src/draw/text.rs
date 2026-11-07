use std::borrow::Cow;

use nih_plug_vizia::vizia::vg::{Baseline, Color, FontId, Paint};

use super::Draw;

impl Draw<'_> {
    // femtovg falls back per word, not per glyph. Split SMuFL symbols explicitly.
    // Runs are visited in place: allocating a run list per text call would
    // allocate on every redraw.
    fn for_each_text_run(
        text: &str,
        font: Option<FontId>,
        fallback_font: Option<FontId>,
        mut visit: impl FnMut(&str, Option<FontId>),
    ) {
        let mut start = 0;
        let mut previous = font;
        for (offset, ch) in text.char_indices() {
            let run_font = if ('\u{e000}'..='\u{f8ff}').contains(&ch) {
                fallback_font.or(font)
            } else {
                font
            };
            if offset > start && run_font != previous {
                visit(&text[start..offset], previous);
                start = offset;
            }
            previous = run_font;
        }
        if start < text.len() {
            visit(&text[start..], previous);
        }
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
        let mut width = 0.0;
        Self::for_each_text_run(text, self.font, self.fallback_font, |run, font| {
            let paint = self.text_paint(font, size, Color::rgb(0, 0, 0));
            width += self
                .c
                .measure_text(0.0, 0.0, run, &paint)
                .map_or(run.chars().count() as f32 * size * 0.55, |m| {
                    m.width() / self.s
                });
        });
        width
    }

    /// Truncate `text` with an ellipsis so it fits `width` at `size`.
    ///
    /// Borrows the original text when it already fits. The search measures
    /// candidate prefixes plus the ellipsis instead of allocating a new
    /// candidate string for every dropped character.
    pub fn fit_text<'a>(&self, text: &'a str, width: f32, size: f32) -> Cow<'a, str> {
        if self.text_width(text, size) <= width {
            return Cow::Borrowed(text);
        }
        let ellipsis_width = self.text_width("…", size);
        if ellipsis_width > width {
            return Cow::Owned("…".to_string());
        }
        // Byte offset just past each character; prefixes end at these offsets.
        let ends: Vec<usize> = text
            .char_indices()
            .map(|(index, ch)| index + ch.len_utf8())
            .collect();
        let (mut lo, mut hi) = (0usize, ends.len());
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if self.text_width(&text[..ends[mid - 1]], size) + ellipsis_width <= width {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        if lo == 0 {
            return Cow::Owned("…".to_string());
        }
        let end = ends[lo - 1];
        let mut fitted = String::with_capacity(end + "…".len());
        fitted.push_str(&text[..end]);
        fitted.push('…');
        Cow::Owned(fitted)
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
        let transformed = self.color(color);
        let scale = self.s;
        let canvas = &mut *self.c;
        Self::for_each_text_run(text, self.font, self.fallback_font, |run, font| {
            let mut paint = Paint::color(transformed);
            if let Some(font) = font {
                paint.set_font(&[font]);
            }
            paint.set_font_size(size * scale);
            if middle {
                paint.set_text_baseline(Baseline::Middle);
            }
            let width = canvas
                .measure_text(0.0, 0.0, run, &paint)
                .map_or(run.chars().count() as f32 * size * scale * 0.55, |m| m.width());
            let _ = canvas.fill_text(x, y, run, &paint);
            x += width;
        });
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
