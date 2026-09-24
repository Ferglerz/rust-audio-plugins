//! Widgets and utilities for making widgets to integrate VIZIA with NIH-plug.
//!
//! # Note
//!
//! None of these widgets are finalized, and their sizes or looks can change at any point. Feel free
//! to copy the widgets and modify them to your personal taste.

use crossbeam::atomic::AtomicCell;
use nih_plug::prelude::{nih_error, GuiContext, Param, ParamPtr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use vizia::prelude::*;

use super::ViziaState;

mod generic_ui;
pub mod param_base;
mod param_button;
mod param_slider;
mod peak_meter;
mod resize_handle;
pub mod util;

pub use generic_ui::GenericUi;
pub use param_button::{ParamButton, ParamButtonExt};
pub use param_slider::{ParamSlider, ParamSliderExt, ParamSliderStyle};
pub use peak_meter::PeakMeter;
pub use resize_handle::ResizeHandle;

/// Register the default theme for the widgets exported by this module. This is automatically called
/// for you when using [`create_vizia_editor()`][super::create_vizia_editor()].
pub fn register_theme(cx: &mut Context) {
    if let Err(err) = cx.add_stylesheet(include_style!("assets/widgets.css")) {
        nih_error!("Failed to load stylesheet: {err:?}")
    }
}

/// An event that updates a parameter's value. Since NIH-plug manages the parameters, interacting
/// with parameter values with VIZIA works a little different from updating any other state. These
/// events are automatically handled by `nih_plug_vizia`.
///
/// Call the [`upcast()`][Self::upcast()] method to be able to emit this event through an
/// [`EventContext`][EventContext].
#[derive(Debug, Clone, Copy)]
pub enum ParamEvent<'a, P: Param> {
    /// Begin an automation gesture for a parameter.
    BeginSetParameter(&'a P),
    /// Set a parameter to a new normalized value. This needs to be surrounded by a matching
    /// `BeginSetParameter` and `EndSetParameter`.
    SetParameter(&'a P, P::Plain),
    /// Set a parameter to a new normalized value. This needs to be surrounded by a matching
    /// `BeginSetParameter` and `EndSetParameter`.
    SetParameterNormalized(&'a P, f32),
    /// End an automation gesture for a parameter.
    EndSetParameter(&'a P),
}

/// The same as [`ParamEvent`], but type erased. Use `ParamEvent` as an easier way to construct
/// these if you are working with regular parameter objects.
#[derive(Debug, Clone, Copy)]
pub enum RawParamEvent {
    /// Begin an automation gesture for a parameter.
    BeginSetParameter(ParamPtr),
    /// Set a parameter to a new normalized value. This needs to be surrounded by a matching
    /// `BeginSetParameter` and `EndSetParameter`.
    SetParameterNormalized(ParamPtr, f32),
    /// End an automation gesture for a parameter.
    EndSetParameter(ParamPtr),
    /// Sent by the wrapper to indicate that one or more parameter values have changed. Useful when
    /// using properties based on a parameter's value that are computed inside of an event handler.
    ParametersChanged,
}

/// Events that directly interact with the [`GuiContext`]. Used to trigger resizes.
pub enum GuiContextEvent {
    /// Resize the window to match the current size reported by the [`ViziaState`]'s size function.
    /// By changing the plugin's state that is used to determine the window's size before emitting
    /// this event, the window can be resized in a declarative and predictable way:
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use std::sync::atomic::{AtomicBool, Ordering};
    /// # use nih_plug_vizia::ViziaState;
    /// # use nih_plug_vizia::vizia::prelude::*;
    /// # use nih_plug_vizia::widgets::GuiContextEvent;
    /// // Assuming there is some kind of state variable passed to the editor, likely stored as a
    /// // `#[persist]` field in the `Params` struct:
    /// let window_state = Arc::new(AtomicBool::new(false));
    ///
    /// // And this is the `ViziaState` passed to `create_vizia_editor()`:
    /// ViziaState::new(move || {
    ///     if window_state.load(Ordering::Relaxed) {
    ///         (800, 400)
    ///     } else {
    ///         (400, 400)
    ///     }
    /// });
    ///
    /// // Then the window's size can be toggled between the two sizes like so:
    /// fn toggle_window_size(cx: &mut EventContext, window_state: Arc<AtomicBool>) {
    ///     window_state.fetch_xor(true, Ordering::Relaxed);
    ///
    ///     // This will cause NIH-plug to query the size from the `ViziaState` again and resize the
    ///     // windo to that size
    ///     cx.emit(GuiContextEvent::Resize);
    /// }
    /// ```
    Resize,
}

/// Handles parameter updates for VIZIA GUIs. Registered in
/// [`ViziaEditor::spawn()`][super::ViziaEditor::spawn()].
pub(crate) struct ParamModel {
    pub context: Arc<dyn GuiContext>,
}

/// Handles interactions through `WindowEvent` for VIZIA GUIs by updating the `ViziaState`.
/// Registered in [`ViziaEditor::spawn()`][super::ViziaEditor::spawn()].
#[derive(Lens)]
pub(crate) struct WindowModel {
    pub context: Arc<dyn GuiContext>,
    pub vizia_state: Arc<ViziaState>,

    /// The last known unscaled logical window size. Used to prevent sending duplicate resize
    /// requests.
    pub last_inner_window_size: AtomicCell<(u32, u32)>,
    pub accepted_scale: f64,
    pub pending_resize: Option<Instant>,
    /// Unscaled logical size after a host drag, applied once it settles.
    pub host_logical: Option<(u32, u32)>,
    pub host_resize_at: Option<Instant>,
}

impl Model for ParamModel {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        // `ParamEvent` gets downcast into `NormalizedParamEvent` by the `Message`
        // implementation below
        event.map(|param_event, _| match *param_event {
            RawParamEvent::BeginSetParameter(p) => unsafe {
                self.context.raw_begin_set_parameter(p)
            },
            RawParamEvent::SetParameterNormalized(p, v) => unsafe {
                self.context.raw_set_parameter_normalized(p, v)
            },
            RawParamEvent::EndSetParameter(p) => unsafe { self.context.raw_end_set_parameter(p) },
            // This can be used by widgets to be notified when parameter values have changed
            RawParamEvent::ParametersChanged => (),
        });
    }
}

/// Idle ticks flush the trailing edge of a resize without a background thread.
#[derive(Clone, Copy)]
pub(crate) struct ResizeTick;

impl Model for WindowModel {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|gui_context_event, meta| match gui_context_event {
            GuiContextEvent::Resize => {
                // This will trigger a `WindowEvent::GeometryChanged`, which in turn causes the
                // handler below this to be fired
                let (width, height) = self.vizia_state.inner_logical_size();
                cx.set_window_size(WindowSize { width, height });

                meta.consume();
            }
        });

        event.map(|_: &ResizeTick, _| {
            if self
                .host_resize_at
                .is_some_and(|at| at.elapsed() >= Duration::from_millis(120))
            {
                self.host_resize_at = None;
                if let Some((width, height)) = self.host_logical.take() {
                    let (artwork_w, artwork_h) = self.vizia_state.inner_logical_size();
                    let current_scale = cx.user_scale_factor();
                    let new_scale = crate::editor_scale::scale_from_host_resize(
                        artwork_w,
                        width,
                        current_scale,
                    )
                    .or_else(|| {
                        crate::editor_scale::scale_from_host_resize(
                            artwork_h,
                            height,
                            current_scale,
                        )
                    });
                    if let Some(scale) = new_scale {
                        // Keep the host's physical size while restoring the artwork's
                        // logical dimensions. Vizia then scales drawing and pointer
                        // coordinates together, including controls and graph edits.
                        self.vizia_state.scale_factor.store(scale);
                        self.last_inner_window_size.store((artwork_w, artwork_h));
                        self.accepted_scale = scale;
                        cx.set_window_size(WindowSize {
                            width: artwork_w,
                            height: artwork_h,
                        });
                        cx.set_user_scale_factor(scale);
                        self.vizia_state.remember_scale(scale);
                        self.vizia_state.flush_remembered_scale();
                    }
                }
            }

            if self
                .pending_resize
                .is_some_and(|at| at.elapsed() >= Duration::from_millis(120))
            {
                self.pending_resize = None;
                let size = (cx.window_size().width, cx.window_size().height);
                let scale = cx.user_scale_factor();
                if size == self.last_inner_window_size.load() && scale == self.accepted_scale {
                    return;
                }
                // The host queries Editor::size() synchronously during this request.
                self.vizia_state.scale_factor.store(scale);
                if self.context.request_resize() {
                    self.last_inner_window_size.store(size);
                    self.accepted_scale = scale;
                    self.vizia_state.remember_scale(scale);
                    self.vizia_state.flush_remembered_scale();
                } else {
                    let (width, height) = self.last_inner_window_size.load();
                    self.vizia_state.scale_factor.store(self.accepted_scale);
                    cx.set_window_size(WindowSize { width, height });
                    cx.set_user_scale_factor(self.accepted_scale);
                }
            }
        });

        // This gets fired whenever the inner window gets resized
        event.map(|window_event, _| {
            if let WindowEvent::GeometryChanged { .. } = window_event {
                let logical_size = (cx.window_size().width, cx.window_size().height);
                let scale = cx.user_scale_factor();
                let last = self.last_inner_window_size.load();
                // Compare against the scale the UI is using. A settled host
                // resize updates the persisted scale without changing this.
                if logical_size == last && (scale - self.accepted_scale).abs() < 1e-6 {
                    return;
                }

                // Host-driven: inner size moved away from both the last
                // accepted size and ViziaState. Scale-only changes are the
                // resize handle and still need a debounced request_resize.
                let expected = self.vizia_state.inner_logical_size();
                if logical_size != last && logical_size != expected {
                    self.last_inner_window_size.store(logical_size);
                    self.accepted_scale = scale;
                    self.host_logical = Some(logical_size);
                    self.host_resize_at = Some(Instant::now());
                    self.pending_resize = None;
                    return;
                }

                // Plugin-initiated scale (resize handle). Debounce so we do
                // not spam the host on every drag tick.
                self.vizia_state.scale_factor.store(scale);
                self.pending_resize = Some(Instant::now());
            }
        });
    }
}

impl<P: Param> From<ParamEvent<'_, P>> for RawParamEvent {
    fn from(event: ParamEvent<'_, P>) -> Self {
        match event {
            ParamEvent::BeginSetParameter(p) => RawParamEvent::BeginSetParameter(p.as_ptr()),
            ParamEvent::SetParameter(p, v) => {
                RawParamEvent::SetParameterNormalized(p.as_ptr(), p.preview_normalized(v))
            }
            ParamEvent::SetParameterNormalized(p, v) => {
                RawParamEvent::SetParameterNormalized(p.as_ptr(), v)
            }
            ParamEvent::EndSetParameter(p) => RawParamEvent::EndSetParameter(p.as_ptr()),
        }
    }
}

impl<P: Param> ParamEvent<'_, P> {
    /// Convert this event into a type erased version of itself that can be emitted through
    /// [`EventContext::emit()`][EventContext::emit()].
    ///
    /// TODO: Think of a better, clearer term for this
    pub fn upcast(self) -> RawParamEvent {
        self.into()
    }
}
