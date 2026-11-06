//! Editor scaffolding shared by every plugin editor.

use std::{cell::Cell, time::Duration};

use nih_plug_vizia::vizia::{
    prelude::{Canvas, Context, EventContext, Timer, TimerAction},
    vg::FontId,
};

/// Frame interval used by all editor redraw timers (about 60 fps).
pub const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Create a frame timer that runs `on_tick` on each tick. Not started yet;
/// call `cx.start_timer(timer)` when the timer should run. Some editors
/// store the timer and start it on demand.
pub fn frame_timer(
    cx: &mut Context,
    interval: Duration,
    on_tick: impl Fn(&mut EventContext) + 'static,
) -> Timer {
    cx.add_timer(interval, None, move |cx, action| {
        if let TimerAction::Tick(_) = action {
            on_tick(cx);
        }
    })
}

/// Create and start a frame timer. See [`frame_timer`].
pub fn start_frame_timer(
    cx: &mut Context,
    interval: Duration,
    on_tick: impl Fn(&mut EventContext) + 'static,
) {
    let timer = frame_timer(cx, interval, on_tick);
    cx.start_timer(timer);
}

/// Load a font into `slot` on first use and return the cached id afterwards.
///
/// Returns `None` if the font bytes fail to load; the next call retries.
pub fn load_font_once(
    canvas: &mut Canvas,
    slot: &Cell<Option<FontId>>,
    bytes: &[u8],
) -> Option<FontId> {
    if let Some(font) = slot.get() {
        return Some(font);
    }
    let font = canvas.add_font_mem(bytes).ok();
    slot.set(font);
    font
}
