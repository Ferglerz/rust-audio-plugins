//! [VIZIA](https://github.com/vizia/vizia) editor support for NIH plug.

// See the comment in the main `nih_plug` crate
#![allow(clippy::type_complexity)]

use crossbeam::atomic::AtomicCell;
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::{Editor, GuiContext};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vizia::prelude::*;

// Re-export for convenience
pub use vizia;

pub mod assets;
mod editor;
pub mod vizia_assets;
pub mod widgets;

/// Create an [`Editor`] instance using a [`vizia`][::vizia] GUI. The [`ViziaState`] passed to this
/// function contains the GUI's intitial size, and this is kept in sync whenever the GUI gets
/// resized. You can also use this to know if the GUI is open, so you can avoid performing
/// potentially expensive calculations while the GUI is not open. If you want this size to be
/// persisted when restoring a plugin instance, then you can store it in a `#[persist = "key"]`
/// field on your parameters struct.
///
/// The [`GuiContext`] is also passed to the app function. This is only meant for saving and
/// restoring state as part of your plugin's preset handling. You should not interact with this
/// directly to set parameters. Use the [`ParamEvent`][widgets::ParamEvent]s to change parameter
/// values, and [`GuiContextEvent`] to trigger window resizes.
///
/// The `theming` argument controls what level of theming to apply. If you use
/// [`ViziaTheming::Custom`], then you **need** to call
/// [`nih_plug_vizia::assets::register_noto_sans_light()`][assets::register_noto_sans_light()] at
/// the start of your app function. Vizia's included fonts are also not registered by default. If
/// you use the Roboto font that normally comes with Vizia or any of its emoji or icon fonts, you
/// also need to register those using the functions in
/// [`nih_plug_vizia::vizia_assets`][crate::vizia_assets].
///
/// See [VIZIA](https://github.com/vizia/vizia)'s repository for examples on how to use this.
pub fn create_vizia_editor<F>(
    vizia_state: Arc<ViziaState>,
    theming: ViziaTheming,
    app: F,
) -> Option<Box<dyn Editor>>
where
    F: Fn(&mut Context, Arc<dyn GuiContext>) + 'static + Send + Sync,
{
    Some(Box::new(editor::ViziaEditor {
        vizia_state,
        app: Arc::new(app),
        theming,

        // TODO: We can't get the size of the window when baseview does its own scaling, so if the
        //       host does not set a scale factor on Windows or Linux we should just use a factor of
        //       1. That may make the GUI tiny but it also prevents it from getting cut off.
        #[cfg(target_os = "macos")]
        scaling_factor: AtomicCell::new(None),
        #[cfg(not(target_os = "macos"))]
        scaling_factor: AtomicCell::new(Some(1.0)),

        emit_parameters_changed_event: Arc::new(AtomicBool::new(false)),
    }))
}

/// Controls what level of theming to apply to the editor.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub enum ViziaTheming {
    /// Disable both `nih_plug_vizia`'s and vizia's built-in theming.
    None,
    /// Disable `nih_plug_vizia`'s custom theming. Vizia's included fonts are also not registered by
    /// default. If you use the Roboto font that normally comes with Vizia or any of its emoji or
    /// icon fonts, you need to register those using the functions in
    /// [`nih_plug_vizia::vizia_assets`][crate::vizia_assets].
    Builtin,
    /// Apply `nih_plug_vizia`'s custom theming. This is the default. You **need** to call
    /// [`nih_plug_vizia::assets::register_noto_sans_light()`][assets::register_noto_sans_light()]
    /// at the start of your app function for the font to work correctly.
    #[default]
    Custom,
}

/// State for an `nih_plug_vizia` editor. The scale factor can be manipulated at runtime using
/// `cx.set_user_scale_factor()`.
#[derive(Serialize, Deserialize)]
pub struct ViziaState {
    /// A function that returns the window's current size in logical pixels, before any sort of
    /// scaling is applied. This size can be computed based on the plugin's current state.
    #[serde(skip, default = "empty_size_fn")]
    size_fn: Box<dyn Fn() -> (u32, u32) + Send + Sync>,
    /// A scale factor that should be applied to `size` separate from from any system HiDPI scaling.
    /// This can be used to allow GUIs to be scaled uniformly.
    #[serde(with = "nih_plug::params::persist::serialize_atomic_cell")]
    scale_factor: AtomicCell<f64>,
    /// Whether the editor's window is currently open.
    #[serde(skip)]
    open: AtomicBool,
    /// Fit before opening on the UI thread, including restored state and display changes.
    #[serde(skip)]
    fit_to_screen: AtomicBool,
}

/// A default implementation for `size_fn` needed to be able to derive the `Deserialize` trait.
fn empty_size_fn() -> Box<dyn Fn() -> (u32, u32) + Send + Sync> {
    Box::new(|| (0, 0))
}

impl Debug for ViziaState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (width, height) = (self.size_fn)();

        f.debug_struct("ViziaState")
            .field("size_fn", &format!("<fn> ({}, {})", width, height))
            .field("scale_factor", &self.scale_factor)
            .field("open", &self.open)
            .finish()
    }
}

impl<'a> PersistentField<'a, ViziaState> for Arc<ViziaState> {
    fn set(&self, new_value: ViziaState) {
        self.scale_factor.store(new_value.scale_factor.load());
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&ViziaState) -> R,
    {
        f(self)
    }
}

impl ViziaState {
    /// Initialize the GUI's state. This value can be passed to [`create_vizia_editor()`]. The
    /// callback always returns the window's current size is in logical pixels, so before it is
    /// multiplied by the DPI scaling factor. This size can be computed based on the plugin's
    /// current state.
    pub fn new(size_fn: impl Fn() -> (u32, u32) + Send + Sync + 'static) -> Arc<ViziaState> {
        Arc::new(ViziaState {
            size_fn: Box::new(size_fn),
            scale_factor: AtomicCell::new(1.0),
            open: AtomicBool::new(false),
            fit_to_screen: AtomicBool::new(false),
        })
    }

    /// The same as [`new()`][Self::new()], but with a separate initial scale factor. This scale
    /// factor gets applied on top of any HiDPI scaling, and it can be modified at runtime by
    /// changing `cx.set_user_scale_factor()`.
    pub fn new_with_default_scale_factor(
        size_fn: impl Fn() -> (u32, u32) + Send + Sync + 'static,
        default_scale_factor: f64,
    ) -> Arc<ViziaState> {
        Arc::new(ViziaState {
            size_fn: Box::new(size_fn),
            scale_factor: AtomicCell::new(default_scale_factor),
            open: AtomicBool::new(false),
            fit_to_screen: AtomicBool::new(false),
        })
    }

    /// Start at 100% relative to artwork originally drawn at 120%, leaving
    /// at least 25% of the available screen width free and room for host chrome.
    pub fn new_screen_sized(size_fn: impl Fn() -> (u32, u32) + Send + Sync + 'static) -> Arc<Self> {
        let state = Self::new_with_default_scale_factor(size_fn, 1.0 / 1.2);
        state.fit_to_screen.store(true, Ordering::Relaxed);
        state
    }

    fn fit_screen_if_available(&self) {
        if self.fit_to_screen.load(Ordering::Relaxed) && !self.is_open() {
            if let Some(screen) = baseview::available_screen_size() {
                self.scale_factor.store(screen_fit_scale(
                    self.inner_logical_size(),
                    self.scale_factor.load(),
                    (screen.width, screen.height),
                ));
            }
        }
    }

    /// Returns a `(width, height)` pair for the current size of the GUI in logical pixels, after
    /// applying the user scale factor.
    pub fn scaled_logical_size(&self) -> (u32, u32) {
        let (logical_width, logical_height) = self.inner_logical_size();
        let scale_factor = self.user_scale_factor();

        (
            (logical_width as f64 * scale_factor).round() as u32,
            (logical_height as f64 * scale_factor).round() as u32,
        )
    }

    /// Returns a `(width, height)` pair for the current size of the GUI in logical pixels before
    /// applying the user scale factor.
    pub fn inner_logical_size(&self) -> (u32, u32) {
        (self.size_fn)()
    }

    /// Get the non-DPI related uniform scaling factor the GUI's size will be multiplied with. This
    /// can be changed by changing `cx.user_scale_factor`.
    pub fn user_scale_factor(&self) -> f64 {
        self.fit_screen_if_available();
        self.scale_factor.load()
    }

    /// Whether the GUI is currently visible.
    // Called `is_open()` instead of `open()` to avoid the ambiguity.
    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }
}

fn screen_fit_scale(
    (width, height): (u32, u32),
    preferred: f64,
    (screen_w, screen_h): (f64, f64),
) -> f64 {
    if width == 0
        || height == 0
        || !screen_w.is_finite()
        || !screen_h.is_finite()
        || screen_w <= 0.0
        || screen_h <= 80.0
    {
        return preferred;
    }
    preferred
        .min(screen_w * 0.75 / width as f64)
        .min((screen_h - 80.0) / height as f64)
}

#[cfg(test)]
mod sizing_tests {
    use super::*;
    #[test]
    fn fits_width_and_height_without_upscaling() {
        assert_eq!(
            screen_fit_scale((1282, 656), 1.0 / 1.2, (1920.0, 1080.0)),
            1.0 / 1.2
        );
        let scale = screen_fit_scale((1282, 656), 1.0, (1024.0, 600.0));
        assert!(1282.0 * scale <= 768.0);
        assert!(656.0 * scale <= 520.0);
        assert_eq!(
            screen_fit_scale((1040, 660), 1.0, (1920.0, 600.0)),
            520.0 / 660.0
        );
    }
}
