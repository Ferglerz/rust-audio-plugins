//! Transfer and detector-EQ pages share Damian's quarter-second quintic slide.
use super::*;
use std::time::{Duration, Instant};

const PAGE_W: f32 = appearance::PLEASANT_GRAPH_SIZE + 32.0;

#[derive(Lens, Clone)]
pub(super) struct GraphPages {
    progress: f32,
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
        event.map(|mode: &appearance::SetAppearance, _| {
            if mode.0 == 2 {
                self.progress = 0.0;
                self.target = 0.0;
                cx.stop_timer(self.timer);
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
    EditorData::appearance.get(cx) != 2 && GraphPages::progress.get(cx) >= 1.0
}
pub(super) fn transfer_active(cx: &EventContext) -> bool {
    GraphPages::progress.get(cx) <= 0.0
}

pub(super) fn build(cx: &mut Context, params: Arc<ComposureParams>, display: Arc<UiDisplay>) {
    ZStack::new(cx, |cx| {
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
            .left(Pixels(32.0))
            .top(Pixels(12.0))
            .width(EditorData::appearance.map(|mode| Pixels(appearance::graph_size(*mode))))
            .height(EditorData::appearance.map(|mode| Pixels(appearance::graph_size(*mode))));
        })
        .position_type(PositionType::SelfDirected)
        .left(GraphPages::progress.map(|p| Pixels(-p * PAGE_W)))
        .top(Pixels(0.0))
        .width(Pixels(PAGE_W))
        .height(EditorData::appearance.map(|mode| Pixels(appearance::graph_size(*mode) + 40.0)));
        ZStack::new(cx, |cx| {
            detector_eq_view::DetectorEqView::new(cx, params.clone(), display.clone())
                .position_type(PositionType::SelfDirected)
                .left(Pixels(32.0))
                .top(Pixels(12.0))
                .width(Pixels(appearance::PLEASANT_GRAPH_SIZE))
                .height(Pixels(appearance::PLEASANT_GRAPH_SIZE));
        })
        .position_type(PositionType::SelfDirected)
        .left(GraphPages::progress.map(|p| Pixels((1.0 - p) * PAGE_W)))
        .top(Pixels(0.0))
        .width(Pixels(PAGE_W))
        .height(EditorData::appearance.map(|mode| Pixels(appearance::graph_size(*mode) + 40.0)))
        .display(EditorData::appearance.map(|mode| {
            if *mode == 2 {
                Display::None
            } else {
                Display::Flex
            }
        }));
    })
    .position_type(PositionType::SelfDirected)
    .left(EditorData::appearance.map(|mode| Pixels(appearance::graph_x(*mode) - 32.0)))
    .top(EditorData::appearance.map(|mode| Pixels(appearance::graph_y(*mode) - 12.0)))
    .width(EditorData::appearance.map(|mode| Pixels(appearance::graph_size(*mode) + 32.0)))
    .height(EditorData::appearance.map(|mode| Pixels(appearance::graph_size(*mode) + 40.0)))
    .overflow(Overflow::Hidden);
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
        let mut d = appearance::painter(cx, canvas, 130.0);
        d.button(
            (0.0, 0.0, 130.0, 28.0),
            if selected { "DETECTOR EQ" } else { "TRANSFER" },
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
        .height(Pixels(28.0))
        .display(EditorData::appearance.map(|mode| {
            if *mode == 2 {
                Display::None
            } else {
                Display::Flex
            }
        }));
}
