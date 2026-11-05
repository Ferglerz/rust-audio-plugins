//! Composite the module stack as one page, leaving the tracker and header fixed.
//! Three reusable GPU images retain the visible frame, the outgoing frame, and
//! the live destination. Retargeting freezes the current composite, so fast row
//! clicks never jump back to a previously selected row or duplicate parameters.
use super::*;
use nih_plug_vizia::vizia::vg::{ImageFlags, ImageId, Paint, Path, PixelFormat, RenderTarget};

pub(super) struct RowSlide {
    selected: Option<(u64, usize)>,
    generation: u64,
    elapsed: f32,
    direction: f32,
}

impl Default for RowSlide {
    fn default() -> Self {
        Self {
            selected: None,
            generation: 0,
            elapsed: pleasant_ui::page_slide::DURATION,
            direction: 1.0,
        }
    }
}

impl RowSlide {
    pub(super) fn select(&mut self, id: u64, index: usize) -> bool {
        let previous = self.selected.replace((id, index));
        let Some((previous_id, previous_index)) = previous else {
            return false;
        };
        if id == previous_id {
            return false;
        }
        self.direction = if index >= previous_index { 1.0 } else { -1.0 };
        self.elapsed = 0.0;
        self.generation = self.generation.wrapping_add(1);
        true
    }

    pub(super) fn active(&self) -> bool {
        self.elapsed < pleasant_ui::page_slide::DURATION
    }

    pub(super) fn tick(&mut self, dt: f32) -> bool {
        let active = self.active();
        self.elapsed = (self.elapsed + dt).min(pleasant_ui::page_slide::DURATION);
        active
    }

    fn offsets(&self, width: f32) -> (f32, f32) {
        let progress = pleasant_ui::page_slide::position(0.0, 1.0, self.elapsed);
        (
            -self.direction * progress * width,
            self.direction * (1.0 - progress) * width,
        )
    }
}

pub(super) struct RowFrames {
    size: (usize, usize),
    visible: ImageId,
    outgoing: ImageId,
    incoming: ImageId,
    generation: u64,
    populated: bool,
    transition: bool,
}

impl RowFrames {
    fn new(canvas: &mut Canvas, size: (usize, usize), generation: u64) -> Option<Self> {
        let mut images = Vec::with_capacity(3);
        for _ in 0..3 {
            match canvas.create_image_empty(
                size.0,
                size.1,
                PixelFormat::Rgba8,
                ImageFlags::FLIP_Y | ImageFlags::PREMULTIPLIED,
            ) {
                Ok(image) => images.push(image),
                Err(_) => {
                    for image in images {
                        canvas.delete_image(image);
                    }
                    return None;
                }
            }
        }
        Some(Self {
            size,
            visible: images[0],
            outgoing: images[1],
            incoming: images[2],
            generation,
            populated: false,
            transition: false,
        })
    }

    fn delete(self, canvas: &mut Canvas) {
        for image in [self.visible, self.outgoing, self.incoming] {
            canvas.delete_image(image);
        }
    }
}

fn image(canvas: &mut Canvas, id: ImageId, x: f32, y: f32, w: f32, h: f32) {
    let mut path = Path::new();
    path.rect(x, y, w, h);
    canvas.fill_path(&path, &Paint::image(id, x, y, w, h, 0.0, 1.0));
}

impl ChordboardView {
    pub(super) fn draw_row_modules(&self, d: &mut Draw) {
        let width = EDITOR_W * d.s;
        let height = (H - HEADER_H - 22.0) * d.s;
        let size = (width.ceil() as usize, height.ceil() as usize);
        let mut cache = self.row_frames.borrow_mut();
        if cache.as_ref().is_some_and(|frames| frames.size != size) {
            if let Some(frames) = cache.take() {
                frames.delete(d.c);
            }
        }
        if cache.is_none() {
            *cache = RowFrames::new(d.c, size, self.row_slide.generation);
        }
        let Some(frames) = cache.as_mut() else {
            // Keep the editor usable if the GPU cannot allocate page textures.
            self.draw_modules(d);
            return;
        };
        if frames.generation != self.row_slide.generation {
            std::mem::swap(&mut frames.visible, &mut frames.outgoing);
            frames.generation = self.row_slide.generation;
            frames.transition = frames.populated;
        }
        let sliding = self.row_slide.active() && frames.transition;
        d.c.save();
        d.c.reset();
        d.c.set_render_target(RenderTarget::Image(if sliding {
            frames.incoming
        } else {
            frames.visible
        }));
        d.c.clear_rect(0, 0, size.0 as u32, size.1 as u32, d.color(BG));
        {
            let mut page = Draw::new(d.c, d.light, d.s, CONTENT_X * d.s, -HEADER_H * d.s, d.font)
                .with_hover(if sliding { None } else { d.hover });
            page.fallback_font = d.fallback_font;
            page.icon_font = d.icon_font;
            self.draw_modules(&mut page);
        }
        if sliding {
            d.c.reset();
            d.c.set_render_target(RenderTarget::Image(frames.visible));
            d.c.clear_rect(0, 0, size.0 as u32, size.1 as u32, d.color(BG));
            let (outgoing, incoming) = self.row_slide.offsets(width);
            image(
                d.c,
                frames.outgoing,
                outgoing,
                0.0,
                size.0 as f32,
                size.1 as f32,
            );
            image(
                d.c,
                frames.incoming,
                incoming,
                0.0,
                size.0 as f32,
                size.1 as f32,
            );
        }
        d.c.set_render_target(RenderTarget::Screen);
        d.c.restore();
        image(
            d.c,
            frames.visible,
            d.ox - CONTENT_X * d.s,
            d.oy + HEADER_H * d.s,
            size.0 as f32,
            size.1 as f32,
        );
        frames.populated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_slide_tracks_identity_direction_and_shared_easing() {
        let mut slide = RowSlide::default();
        assert!(!slide.select(10, 0));
        assert!(slide.select(20, 1));
        assert_eq!(slide.offsets(100.0), (0.0, 100.0));
        slide.tick(pleasant_ui::page_slide::DURATION / 2.0);
        assert_eq!(slide.offsets(100.0), (-50.0, 50.0));
        assert!(slide.select(10, 0));
        assert_eq!(slide.offsets(100.0), (0.0, -100.0));
        slide.tick(pleasant_ui::page_slide::DURATION);
        assert!(!slide.active());
        assert_eq!(slide.offsets(100.0), (100.0, 0.0));
        // Reordering the selected row does not change its settings page.
        assert!(!slide.select(10, 4));
        assert!(!slide.active());
        // Deleting the selected row can replace it at the same list index.
        assert!(slide.select(30, 4));
    }
}
