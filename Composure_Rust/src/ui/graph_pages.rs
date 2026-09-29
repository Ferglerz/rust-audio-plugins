//! Expand Envelope into the curve space with a quarter-second quintic animation.
use super::*;
use std::time::{Duration, Instant};

pub(super) const CURVE_HEADER_H: f32 = 32.0;
fn header_height(_mode: u8) -> f32 {
    {
        CURVE_HEADER_H
    }
}
pub(super) const AXIS_W: f32 = 44.0;
pub(super) const PAGE_W: f32 = appearance::PLEASANT_GRAPH_SIZE + AXIS_W;
/// Only the module boundary moves; meters and all existing controls stay fixed.
pub(super) const EXPANSION_W: f32 =
    appearance::PLEASANT_GRAPH_X + appearance::PLEASANT_GRAPH_SIZE - appearance::TRANS_X;

#[derive(Lens, Clone)]
pub(super) struct GraphPages {
    pub(super) progress: f32,
    target: f32,
    start: f32,
    started: Instant,
    timer: Timer,
}

#[derive(Clone, Copy)]
enum PageEvent {
    Toggle,
    Tick,
}

impl Model for GraphPages {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e: &PageEvent, _| match e {
            PageEvent::Toggle => {
                self.start = self.progress;
                self.target = if self.target > 0.5 { 0.0 } else { 1.0 };
                self.started = Instant::now();
                cx.start_timer(self.timer);
                cx.needs_redraw();
            }
            PageEvent::Tick => {
                self.progress = pleasant_ui::page_slide::position(
                    self.start,
                    self.target,
                    self.started.elapsed().as_secs_f32(),
                );
                if self.progress == self.target {
                    cx.stop_timer(self.timer);
                }
                cx.needs_redraw();
            }
        });
    }
}

pub(super) fn build_model(cx: &mut Context) {
    let timer = cx.add_timer(Duration::from_millis(16), None, |cx, action| {
        if matches!(action, TimerAction::Tick(_)) {
            cx.emit(PageEvent::Tick);
        }
    });
    GraphPages {
        progress: 0.0,
        target: 0.0,
        start: 0.0,
        started: Instant::now(),
        timer,
    }
    .build(cx);
}

pub(super) fn detector_active(cx: &EventContext) -> bool {
    GraphPages::progress.get(cx) >= 1.0
}
pub(super) fn transfer_active(cx: &EventContext) -> bool {
    GraphPages::progress.get(cx) <= 0.0
}

pub(super) fn build(cx: &mut Context, params: Arc<ComposureParams>, display: Arc<UiDisplay>) {
    ZStack::new(cx, |cx| {
        ZStack::new(cx, |cx| {
            CurveTitle
                .build(cx, |_| {})
                .position_type(PositionType::SelfDirected)
                .left(Pixels(0.0))
                .top(Pixels(0.0))
                .width(Pixels(PAGE_W))
                .height(Pixels(CURVE_HEADER_H));
            ZStack::new(cx, |cx| {
                graph_chrome::build(cx, EditorData::params, EditorData::display);
                graph_view::GraphView::new(
                    cx,
                    params.graph_store.clone(),
                    display.clone(),
                    EditorData::display.map(|d| d.detector_db.load(Ordering::Relaxed)),
                    EditorData::display.map(|d| d.gr_db.load(Ordering::Relaxed)),
                    EditorData::params,
                )
                .position_type(PositionType::SelfDirected)
                .left(Pixels(AXIS_W))
                .top(Pixels(12.0))
                .width(EditorData::appearance.map(|mode| Pixels(appearance::graph_size(*mode))))
                .height(EditorData::appearance.map(|mode| Pixels(appearance::graph_size(*mode))));
                image_knob::ImageKnob::new(
                    cx,
                    EditorData::params,
                    |p| &p.input_offset_db,
                    display.clone(),
                    step_points::StepSet::None,
                )
                .class("production-knob")
                .position_type(PositionType::SelfDirected)
                .left(Pixels(AXIS_W + 12.0))
                .top(Pixels(24.0))
                .width(Pixels(96.0))
                .height(Pixels(96.0));
            })
            .position_type(PositionType::SelfDirected)
            .left(Pixels(0.0))
            .top(EditorData::appearance.map(|mode| Pixels(header_height(*mode))))
            .width(Pixels(PAGE_W))
            .height(
                EditorData::appearance.map(|mode| {
                    Pixels(appearance::graph_size(*mode) + 40.0 + header_height(*mode))
                }),
            );
        })
        .position_type(PositionType::SelfDirected)
        .left(Pixels(0.0))
        .display(GraphPages::progress.map(|p| {
            if *p == 0.0 {
                Display::Flex
            } else {
                Display::None
            }
        }))
        .top(Pixels(0.0))
        .width(Pixels(PAGE_W))
        .height(
            EditorData::appearance
                .map(|mode| Pixels(appearance::graph_size(*mode) + 40.0 + header_height(*mode))),
        );
    })
    .position_type(PositionType::SelfDirected)
    .left(EditorData::appearance.map(|mode| Pixels(appearance::graph_x(*mode) - AXIS_W)))
    .top(
        EditorData::appearance
            .map(|mode| Pixels(appearance::graph_y(*mode) - 12.0 - header_height(*mode))),
    )
    .width(
        EditorData::appearance
            .map(|mode| Pixels(appearance::graph_size(*mode) + AXIS_W + { 12.0 })),
    )
    .height(
        EditorData::appearance
            .map(|mode| Pixels(appearance::graph_size(*mode) + 40.0 + header_height(*mode))),
    )
    .overflow(Overflow::Hidden);

    ZStack::new(cx, |cx| {
        detector_eq_view::DetectorEqView::new(cx, params, display)
            .position_type(PositionType::SelfDirected)
            .left(Pixels(0.0))
            .top(Pixels(0.0))
            .width(Pixels(PAGE_W))
            .height(Pixels(appearance::PLEASANT_GRAPH_SIZE + CURVE_HEADER_H));
    })
    .position_type(PositionType::SelfDirected)
    .left(Pixels(appearance::ENV_X + appearance::ENV_W))
    .top(Pixels(appearance::ENV_Y))
    .width(GraphPages::progress.map(|p| Pixels(p * EXPANSION_W)))
    .height(Pixels(appearance::PLEASANT_GRAPH_SIZE + CURVE_HEADER_H))
    .overflow(Overflow::Hidden);
}

struct CurveTitle;
impl View for CurveTitle {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let mut d = appearance::painter(cx, canvas, PAGE_W);
        d.text(
            16.0,
            22.0,
            "CURVE",
            appearance::MODULE_TITLE_SIZE,
            pleasant_ui::TEXT,
        );
    }
}

struct EqToggle;
impl View for EqToggle {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, meta| {
            if matches!(e, WindowEvent::MouseDown(MouseButton::Left)) {
                cx.emit(PageEvent::Toggle);
                meta.consume();
            }
        });
    }
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let selected = GraphPages::target.get(cx) > 0.5;
        let width = cx.bounds().w / cx.scale_factor();
        let mut d = appearance::painter(cx, canvas, width);
        appearance::button(
            &mut d,
            (0.0, 0.0, width, 28.0),
            if selected { "SC EQ <" } else { "SC EQ >" },
            selected,
            pleasant_ui::GOLD,
        );
    }
}
pub(super) fn build_toggle(cx: &mut Context, rect: (f32, f32, f32)) {
    EqToggle
        .build(cx, |_| {})
        .position_type(PositionType::SelfDirected)
        .left(Pixels(rect.0))
        .top(Pixels(rect.1))
        .width(Pixels(rect.2))
        .height(Pixels(28.0));
}

#[cfg(test)]
mod tests {
    use super::*;
    use nih_plug_vizia::vizia::backend::BackendContext;

    #[test]
    fn timer_advances_and_settles_both_graph_pages() {
        let mut cx = Context::default();
        build_model(&mut cx);
        for target in [1.0, 0.0] {
            cx.emit(PageEvent::Toggle);
            BackendContext::new_with_event_manager(&mut cx).process_events();
            assert_eq!(GraphPages::target.get(&cx), target);
            let deadline = Instant::now() + Duration::from_secs(2);
            while GraphPages::progress.get(&cx) != target && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(17));
                let mut backend = BackendContext::new_with_event_manager(&mut cx);
                backend.process_timers();
                backend.process_events();
            }
            assert_eq!(
                GraphPages::progress.get(&cx),
                target,
                "page timer must reach its target"
            );
        }
    }
}
