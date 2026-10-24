#![allow(clippy::type_complexity)]
mod application;
mod parent_window;
pub(crate) mod proxy;
mod window;

pub use parent_window::ParentWindow;

pub use application::Application;

pub use baseview::WindowScalePolicy;

/// Precise scroll motion in physical window pixels, delivered immediately before
/// the legacy line event so custom views can retain native trackpad momentum.
#[derive(Clone, Copy, Debug)]
pub struct PixelScroll {
    pub x: f32,
    pub y: f32,
}
use femtovg::renderer::OpenGl as Renderer;

mod keyboard;
pub use keyboard::KeyboardEventCaptured;
