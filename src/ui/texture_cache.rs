use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use nih_plug_vizia::vizia::prelude::*;

use super::image_draw::CachedTexture;

pub fn texture_cache() -> &'static Mutex<HashMap<&'static str, CachedTexture>> {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, CachedTexture>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Gfx `ImageId`s are per-editor; stale cache breaks second open.
pub fn clear_gpu_textures() {
    if let Ok(mut cache) = texture_cache().lock() {
        for tex in cache.values() {
            tex.invalidate();
        }
        cache.clear();
    }
}

pub fn draw_tex(
    canvas: &mut Canvas,
    key: &'static str,
    png: &'static [u8],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    alpha: f32,
) {
    if let Ok(mut cache) = texture_cache().lock() {
        let tex = cache
            .entry(key)
            .or_insert_with(|| CachedTexture::new(png));
        tex.draw(canvas, x, y, w, h, alpha);
    }
}
