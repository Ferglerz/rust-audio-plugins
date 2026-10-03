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
            .height(Pixels(appearance::SIDE_H));
    })
    .position_type(PositionType::SelfDirected)
    .left(Pixels(appearance::ENV_X + appearance::ENV_W))
    .top(Pixels(appearance::ENV_Y))
    .width(GraphPages::progress.map(|p| Pixels(p * EXPANSION_W)))
    .height(Pixels(appearance::SIDE_H))
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

use pleasant_eq::{BandCoefficients, BandSettings, EqShape};
use std::sync::atomic::Ordering;

struct EqToggle {
    hovered: bool,
}

impl View for EqToggle {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, meta| match e {
            WindowEvent::MouseEnter => {
                self.hovered = true;
                cx.needs_redraw();
            }
            WindowEvent::MouseLeave => {
                self.hovered = false;
                cx.needs_redraw();
            }
            WindowEvent::MouseMove(..) => {
                if !self.hovered {
                    self.hovered = true;
                    cx.needs_redraw();
                }
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                cx.emit(PageEvent::Toggle);
                meta.consume();
            }
            _ => {}
        });
    }
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let selected = GraphPages::target.get(cx) > 0.5;
        let width = cx.bounds().w / cx.scale_factor();
        let mut d = appearance::painter(cx, canvas, width);

        if selected {
            // When EQ is on, we won't show the diagram part of the button
            appearance::button(
                &mut d,
                (0.0, 0.0, width, 28.0),
                "SC EQ <",
                true,
                pleasant_ui::GOLD,
            );
            return;
        }

        // Button background
        d.button((0.0, 0.0, width, 28.0), "", false, pleasant_ui::GOLD);
        if self.hovered {
            d.rect(
                0.0,
                0.0,
                width,
                28.0,
                nih_plug_vizia::vizia::vg::Color::rgba(255, 255, 255, 18),
            );
        }

        let params = EditorData::params.get(cx);
        let display = EditorData::display.get(cx);
        let rate = (display.sample_rate.load(Ordering::Relaxed) as f64).max(44100.0);

        let mut active_coeffs = Vec::new();
        if params.hp_freq.value() > 0.0 {
            let settings = BandSettings {
                shape: EqShape::LowCut,
                order: 2,
                frequency_hz: params.hp_freq.value() as f64,
                gain_db: 0.0,
                q: std::f64::consts::FRAC_1_SQRT_2,
                enabled: true,
                ..BandSettings::default()
            };
            active_coeffs.push(BandCoefficients::prepare(&settings, rate));
        }
        if params.lp_freq.value() > 0.0 {
            let settings = BandSettings {
                shape: EqShape::HighCut,
                order: 2,
                frequency_hz: params.lp_freq.value() as f64,
                gain_db: 0.0,
                q: std::f64::consts::FRAC_1_SQRT_2,
                enabled: true,
                ..BandSettings::default()
            };
            active_coeffs.push(BandCoefficients::prepare(&settings, rate));
        }
        for band in params.detector_eq.iter() {
            if band.active.value() && band.enabled.value() {
                let settings = band.settings();
                active_coeffs.push(BandCoefficients::prepare(&settings, rate));
            }
        }

        let has_sc_eq = params.hp_freq.value() > 0.0
            || params.lp_freq.value() > 0.0
            || params.detector_eq.iter().any(|band| {
                band.active.value()
                    && band.enabled.value()
                    && (!band.settings().shape.has_gain() || band.gain.value().abs() > 0.01)
            });

        let (coeffs, curve_color) = if has_sc_eq && !active_coeffs.is_empty() {
            (active_coeffs, pleasant_ui::GOLD)
        } else {
            let default_hp = BandSettings {
                shape: EqShape::LowCut,
                order: 2,
                frequency_hz: 100.0,
                gain_db: 0.0,
                q: std::f64::consts::FRAC_1_SQRT_2,
                enabled: true,
                ..BandSettings::default()
            };
            let default_cut = BandSettings {
                shape: EqShape::Bell,
                order: 2,
                frequency_hz: 1800.0,
                gain_db: -4.5,
                q: 1.2,
                enabled: true,
                ..BandSettings::default()
            };
            (
                vec![
                    BandCoefficients::prepare(&default_hp, rate),
                    BandCoefficients::prepare(&default_cut, rate),
                ],
                if self.hovered {
                    pleasant_ui::TEXT
                } else {
                    pleasant_ui::MUTED
                },
            )
        };

        let diag_w = 26.0;
        let diag_h = 14.0;
        let gap = 6.0;
        let label = "SC EQ >";
        let label_size = 13.0;
        let label_color = if self.hovered {
            pleasant_ui::TEXT
        } else {
            pleasant_ui::MUTED
        };

        let text_w = {
            let mut p = nih_plug_vizia::vizia::vg::Paint::color(d.color(label_color));
            if let Some(font) = d.font {
                p.set_font(&[font]);
            }
            p.set_font_size(label_size * d.s);
            d.c.measure_text(0.0, 0.0, label, &p)
                .map(|m| m.width() / d.s)
                .unwrap_or(label.len() as f32 * label_size * 0.55)
        };

        let total_w = diag_w + gap + text_w;
        let diag_x = (width - total_w) * 0.5;
        let diag_y = (28.0 - diag_h) * 0.5;
        let text_x = diag_x + diag_w + gap;

        // Faint 0 dB reference line
        let cy = diag_y + diag_h * 0.5;
        let mut guide = pleasant_ui::LINE;
        guide.a = 0.45;
        d.line(diag_x, cy, diag_x + diag_w, cy, guide, 0.8);

        // Mini EQ curve
        let points: Vec<(f32, f32)> = (0..=32)
            .map(|i| {
                let t = i as f32 / 32.0;
                let freq = 20.0 * 1000.0_f64.powf(t as f64);
                let db: f64 = coeffs.iter().map(|c| c.response_db(freq, rate)).sum();
                let y = (diag_y + diag_h * (0.5 - (db.clamp(-18.0, 18.0) as f32 / 36.0)))
                    .clamp(diag_y, diag_y + diag_h);
                (diag_x + diag_w * t, y)
            })
            .collect();
        d.poly(&points, curve_color, 1.4);

        // Text label
        d.text_middle(text_x, 14.0, label, label_size, label_color);
    }
}
pub(super) fn build_toggle(cx: &mut Context, rect: (f32, f32, f32)) {
    EqToggle { hovered: false }
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
