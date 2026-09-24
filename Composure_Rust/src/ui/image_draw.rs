//! Femtovg texture cache for custom image controls.

use std::cell::Cell;

use nih_plug_vizia::vizia::image::{self, ImageFormat};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::{ImageFlags, ImageId, ImageSource, Paint, Path};

pub struct CachedTexture {
    id: Cell<Option<ImageId>>,
    png: &'static [u8],
}

impl CachedTexture {
    pub const fn new(png: &'static [u8]) -> Self {
        Self {
            id: Cell::new(None),
            png,
        }
    }

    pub fn invalidate(&self) {
        self.id.set(None);
    }

    pub fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, alpha: f32) {
        let id = self.ensure_id(canvas);
        let mut path = Path::new();
        path.rect(x, y, w, h);
        canvas.fill_path(&path, &Paint::image(id, x, y, w, h, 0.0, alpha));
    }

    fn ensure_id(&self, canvas: &mut Canvas) -> ImageId {
        if let Some(id) = self.id.get() {
            return id;
        }
        let img = image::load_from_memory_with_format(self.png, ImageFormat::Png).expect("ui png");
        let id = canvas
            .create_image(
                ImageSource::try_from(&img).expect("ui png source"),
                ImageFlags::empty(),
            )
            .expect("ui texture");
        self.id.set(Some(id));
        id
    }
}
