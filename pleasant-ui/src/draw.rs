mod controls;
mod handles;
mod icons;
mod primitives;
mod text;

pub use controls::KnobLayout;
pub use primitives::TRACE_REST_PX;

use crate::theme;
use nih_plug_vizia::vizia::{
    prelude::*,
    vg::{Color, FontId},
};

/// Fits fixed-size editor artwork inside the host's current bounds. Rendering
/// and pointer handling must use the same transform, including the margins.
#[derive(Clone, Copy, Debug)]
pub struct EditorViewport {
    pub scale: f32,
    pub x: f32,
    pub y: f32,
}

impl EditorViewport {
    pub fn fit(bounds: (f32, f32, f32, f32), artwork: (f32, f32)) -> Option<Self> {
        let (x, y, w, h) = bounds;
        let (aw, ah) = artwork;
        if ![x, y, w, h, aw, ah].iter().all(|v| v.is_finite())
            || w <= 0.0
            || h <= 0.0
            || aw <= 0.0
            || ah <= 0.0
        {
            return None;
        }
        let scale = (w / aw).min(h / ah);
        Some(Self {
            scale,
            x: x + (w - aw * scale) * 0.5,
            y: y + (h - ah * scale) * 0.5,
        })
    }

    pub fn to_local(self, x: f32, y: f32) -> (f32, f32) {
        ((x - self.x) / self.scale, (y - self.y) / self.scale)
    }
}

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
    /// Desaturate and dim disabled module content without changing its geometry.
    pub disabled: bool,
    pub hover: Option<(f32, f32)>,
}

impl<'a> Draw<'a> {
    /// Paints an opaque, square background across all host bounds, then fits
    /// artwork to both axes. Other controls keep their existing rounded faces.
    pub fn new_fitted(
        c: &'a mut Canvas,
        light: bool,
        bounds: (f32, f32, f32, f32),
        artwork: (f32, f32),
        font: Option<FontId>,
    ) -> Option<Self> {
        let viewport = EditorViewport::fit(bounds, artwork)?;
        let draw = Self::new(c, light, viewport.scale, viewport.x, viewport.y, font);
        let mut background = nih_plug_vizia::vizia::vg::Path::new();
        background.rect(bounds.0, bounds.1, bounds.2, bounds.3);
        let paint = nih_plug_vizia::vizia::vg::Paint::color(draw.color(theme::BG));
        draw.c.fill_path(&background, &paint);
        Some(draw)
    }

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
            disabled: false,
            hover: None,
        }
    }

    pub fn with_hover(mut self, hover: Option<(f32, f32)>) -> Self {
        self.hover = hover;
        self
    }

    pub fn is_hovered(&self, r: (f32, f32, f32, f32)) -> bool {
        Self::is_hovered_rect(self.hover, self.offset_x, r)
    }

    pub fn is_hovered_rect(
        hover: Option<(f32, f32)>,
        offset_x: f32,
        r: (f32, f32, f32, f32),
    ) -> bool {
        hover.is_some_and(|(hx, hy)| {
            let rx = r.0 + offset_x;
            hx >= rx && hx <= rx + r.2 && hy >= r.1 && hy <= r.1 + r.3
        })
    }

    pub fn color(&self, c: Color) -> Color {
        let mut col = theme::transform_color(c, self.light);
        if self.disabled {
            let gray = (0.2126 * col.r + 0.7152 * col.g + 0.0722 * col.b) * 0.6;
            col.r = gray;
            col.g = gray;
            col.b = gray;
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_viewport_fits_short_and_narrow_hosts_and_maps_pointer_positions() {
        for artwork in [(720.0, 550.0), (812.0, 473.0)] {
            for (w, h) in [artwork, (720.0, 340.0), (812.0, 380.0), (300.0, 900.0)] {
                let bounds = (17.0, 23.0, w, h);
                let viewport = EditorViewport::fit(bounds, artwork).unwrap();
                let right = viewport.x + artwork.0 * viewport.scale;
                let bottom = viewport.y + artwork.1 * viewport.scale;
                assert!(viewport.x >= bounds.0 - 1e-3);
                assert!(viewport.y >= bounds.1 - 1e-3);
                assert!(right <= bounds.0 + w + 1e-3, "right edge clipped");
                assert!(bottom <= bounds.1 + h + 1e-3, "bottom edge clipped");
                for (x, y) in [(0.0, 0.0), (artwork.0 * 0.75, artwork.1 * 0.95)] {
                    let local = viewport.to_local(
                        viewport.x + x * viewport.scale,
                        viewport.y + y * viewport.scale,
                    );
                    assert!((local.0 - x).abs() < 1e-3);
                    assert!((local.1 - y).abs() < 1e-3);
                }
            }
        }
        for bounds in [(0.0, 0.0, 0.0, 100.0), (0.0, 0.0, 100.0, f32::NAN)] {
            assert!(EditorViewport::fit(bounds, (720.0, 550.0)).is_none());
        }
    }

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

    #[test]
    fn test_is_hovered() {
        let r = (10.0, 10.0, 50.0, 20.0);
        assert!(!Draw::is_hovered_rect(None, 0.0, r));

        assert!(Draw::is_hovered_rect(Some((20.0, 15.0)), 0.0, r));
        assert!(!Draw::is_hovered_rect(Some((70.0, 15.0)), 0.0, r));

        assert!(!Draw::is_hovered_rect(Some((20.0, 15.0)), 30.0, r));
        assert!(Draw::is_hovered_rect(Some((50.0, 15.0)), 30.0, r));
    }
}
