//! JSFX debug toggle (invisible 30×30 top-left) + runtime telemetry panel.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::{Color as VgColor, Paint, Path};

use super::display::UiDisplay;

const TOGGLE_SIZE: f32 = 30.0;

pub fn build<D>(cx: &mut Context, display: D)
where
    D: Lens<Target = Arc<UiDisplay>> + Clone + 'static,
{
    DebugToggle::new(cx, display.clone())
        .position_type(PositionType::SelfDirected)
        .left(Pixels(0.0))
        .top(Pixels(0.0))
        .width(Pixels(TOGGLE_SIZE))
        .height(Pixels(TOGGLE_SIZE));

    DebugPanel::new(cx, display)
        .position_type(PositionType::SelfDirected)
        .left(Pixels(8.0))
        .top(Pixels(390.0))
        .width(Pixels(200.0))
        .height(Pixels(108.0));
}

struct DebugToggle<D> {
    display: D,
}

impl DebugToggle<()> {
    fn new<D>(cx: &mut Context, display: D) -> Handle<'_, DebugToggle<D>>
    where
        D: Lens<Target = Arc<UiDisplay>> + Clone + 'static,
    {
        DebugToggle { display }.build(cx, |_| {})
    }
}

impl<D> View for DebugToggle<D>
where
    D: Lens<Target = Arc<UiDisplay>> + Clone,
{
    fn element(&self) -> Option<&'static str> {
        Some("debug-toggle")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let d = self.display.get(cx);
        if !d.is_debug_enabled() {
            return;
        }
        let b = cx.bounds();
        let mut path = Path::new();
        path.rect(b.x, b.y, b.w, b.h);
        let mut paint = Paint::color(VgColor::rgbaf(0.6, 0.6, 0.2, cx.opacity() * 0.4));
        paint.set_line_width(1.0);
        canvas.stroke_path(&path, &paint);
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.display.get(cx).toggle_debug();
                cx.needs_redraw();
                meta.consume();
            }
            _ => {}
        });
    }
}

struct DebugPanel<D> {
    display: D,
}

impl DebugPanel<()> {
    fn new<D>(cx: &mut Context, display: D) -> Handle<'_, DebugPanel<D>>
    where
        D: Lens<Target = Arc<UiDisplay>> + Clone + 'static,
    {
        let text_lens = display.map(|d: &Arc<UiDisplay>| {
            if !d.is_debug_enabled() {
                return String::new();
            }
            let _v = d.debug_revision();
            let det = d.detector_db.load(Ordering::Relaxed);
            let gr = d.gr_db.load(Ordering::Relaxed);
            d.debug_overlay_text(det, gr)
        });

        DebugPanel { display }.build(cx, |cx| {
            Label::new(cx, text_lens).class("debug-panel-text");
        })
    }
}

impl<D> View for DebugPanel<D>
where
    D: Lens<Target = Arc<UiDisplay>> + Clone,
{
    fn element(&self) -> Option<&'static str> {
        Some("debug-panel-root")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let d = self.display.get(cx);
        if !d.is_debug_enabled() {
            return;
        }
        let b = cx.bounds();
        let opacity = cx.opacity();
        let mut bg = Path::new();
        bg.rect(b.x, b.y, b.w, b.h);
        canvas.fill_path(
            &bg,
            &Paint::color(VgColor::rgbaf(0.1, 0.1, 0.1, opacity * 0.85)),
        );
        let mut border = Path::new();
        border.rect(b.x, b.y, b.w, b.h);
        let mut border_paint = Paint::color(VgColor::rgbaf(0.6, 0.6, 0.6, opacity));
        border_paint.set_line_width(1.0);
        canvas.stroke_path(&border, &border_paint);
    }
}
