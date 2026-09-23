mod controls;
mod handles;
mod icons;
mod primitives;
mod text;

pub use primitives::TRACE_REST_PX;

use crate::theme;
use nih_plug_vizia::vizia::{
    prelude::*,
    vg::{Color, FontId},
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
