//! Graph chrome — point count + axis labels.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug_vizia::vizia::prelude::*;

use crate::dsp::constants::MAX_POINTS;
use crate::params::ComposureParams;

use super::display::UiDisplay;
use super::graph_display;
use super::graph_pages::AXIS_W;
use super::theme;

const MAX_AXIS_LABELS: usize = 12;

pub fn build<P, D>(cx: &mut Context, params: P, display: D)
where
    P: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    D: Lens<Target = Arc<UiDisplay>> + Clone + 'static,
{
    let point_lens = display.map(|d: &Arc<UiDisplay>| {
        let _v = d.graph_points_version.load(Ordering::Relaxed);
        let interior = d.graph_interior_points.load(Ordering::Relaxed);
        format!("{interior}/{}", MAX_POINTS - 2)
    });

    Label::new(cx, point_lens)
        .class("graph-point-count")
        .position_type(PositionType::SelfDirected)
        .left(
            super::EditorData::appearance
                .map(|mode| Pixels(AXIS_W + super::appearance::graph_size(*mode) - 64.0)),
        )
        .top(
            super::EditorData::appearance
                .map(|mode| Pixels(12.0 + super::appearance::graph_size(*mode) - 36.0)),
        )
        .width(Pixels(44.0))
        .height(Pixels(16.0));

    build_axis_labels(cx, params);
    build_range_selector(cx);

    let hint_lens = display.map(|d: &Arc<UiDisplay>| {
        if d.graph_hint_visible.load(Ordering::Relaxed) {
            "Ctrl/cmd to fine adjust".to_string()
        } else {
            String::new()
        }
    });

    Label::new(cx, hint_lens)
        .class("graph-ctrl-hint")
        .position_type(PositionType::SelfDirected)
        .left(super::EditorData::appearance.map(|_| Pixels(AXIS_W + 16.0)))
        .top(
            super::EditorData::appearance
                .map(|mode| Pixels(12.0 + super::appearance::graph_size(*mode) - 20.0)),
        )
        .width(Pixels(160.0))
        .height(Pixels(14.0));
}

pub fn build_analog_readout<D>(cx: &mut Context, display: D)
where
    D: Lens<Target = Arc<UiDisplay>> + Clone + 'static,
{
    let readout_lens = display.map(|d: &Arc<UiDisplay>| d.active_readout());
    Label::new(cx, readout_lens)
        .class("control-readout")
        .display(super::EditorData::appearance.map(|mode| {
            if *mode == 2 {
                Display::Flex
            } else {
                Display::None
            }
        }))
        .position_type(PositionType::SelfDirected)
        .left(Pixels(194.0))
        .top(Pixels(278.0))
        .width(Pixels(280.0))
        .height(Pixels(24.0));
}

fn build_axis_labels<P>(cx: &mut Context, params: P)
where
    P: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
{
    for index in 1..=MAX_AXIS_LABELS {
        for mode in [0_u8, 2_u8] {
            let top_lens = params.clone().map(move |p: &Arc<ComposureParams>| {
                Pixels(axis_label_y(p, index, mode) - super::appearance::graph_y(mode) + 12.0)
            });
            let label_lens = params.map(move |p: &Arc<ComposureParams>| {
                let labels = graph_display::axis_label_db_values(graph_range_db(p));
                if mode != 2 && index == labels.len() {
                    String::new()
                } else {
                    axis_label_text(p, index).unwrap_or_default()
                }
            });
            Label::new(cx, label_lens)
                .class("graph-axis-label")
                .display(super::EditorData::appearance.map(move |current| {
                    if (*current == 2) == (mode == 2) {
                        Display::Flex
                    } else {
                        Display::None
                    }
                }))
                .position_type(PositionType::SelfDirected)
                .left(Pixels(AXIS_W - theme::sx(30.0)))
                .top(top_lens)
                .width(Pixels(26.0))
                .height(Pixels(12.0));
        }
    }
}

fn graph_range_db(p: &ComposureParams) -> f64 {
    p.graph_range_mode.value().range_db()
}

fn axis_label_text(p: &ComposureParams, index: usize) -> Option<String> {
    let labels = graph_display::axis_label_db_values(graph_range_db(p));
    let idx = index.checked_sub(1)?;
    labels.get(idx).map(|db| db.to_string())
}

fn axis_label_y(p: &ComposureParams, index: usize, mode: u8) -> f32 {
    let labels = graph_display::axis_label_db_values(graph_range_db(p));
    let idx = match index.checked_sub(1) {
        Some(i) if i < labels.len() => i,
        _ => return super::appearance::graph_y(mode),
    };
    let db = labels[idx] as f64;
    let op_min = -graph_range_db(p);
    graph_display::axis_label_y(
        db,
        op_min,
        graph_range_db(p),
        super::appearance::graph_size(mode),
    ) + super::appearance::graph_y(mode)
        - 5.0
}

fn range_button_y(p: &ComposureParams) -> f32 {
    12.0 + graph_display::axis_label_y(
        -graph_range_db(p),
        -graph_range_db(p),
        graph_range_db(p),
        super::appearance::PLEASANT_GRAPH_SIZE,
    ) - 10.0
}

struct RangeButton;
impl View for RangeButton {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, meta| {
            if matches!(e, WindowEvent::MouseDown(MouseButton::Left)) {
                cx.emit(PopupEvent::Switch);
                meta.consume();
            }
        });
    }
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let range = graph_range_db(&super::EditorData::params.get(cx));
        let mut d = super::appearance::painter(cx, canvas, AXIS_W);
        d.rect(0.0, 0.0, AXIS_W, 20.0, pleasant_ui::PANEL);
        d.text_centered(
            AXIS_W * 0.5,
            14.0,
            &format!("-{} ▾", range as i32),
            11.0,
            pleasant_ui::MUTED,
        );
    }
}

struct RangeMenu {
    param: nih_plug_vizia::widgets::param_base::ParamWidgetBase,
}
impl View for RangeMenu {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        use nih_plug::prelude::Enum;
        event.map(|e, meta| {
            if matches!(e, WindowEvent::MouseDown(MouseButton::Left)) {
                let row =
                    ((cx.mouse().cursory - cx.bounds().y) / (18.0 * cx.scale_factor())) as usize;
                if let Some(mode) = crate::params::GraphRangeMode::PLEASANT.get(row) {
                    self.param.begin_set_parameter(cx);
                    self.param.set_normalized_value(
                        cx,
                        self.param.preview_normalized(mode.to_index() as f32),
                    );
                    self.param.end_set_parameter(cx);
                    cx.emit(PopupEvent::Close);
                }
                meta.consume();
            }
        });
    }
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let range = graph_range_db(&super::EditorData::params.get(cx));
        let mut d = super::appearance::painter(cx, canvas, 76.0);
        d.rect(0.0, 0.0, 76.0, 216.0, pleasant_ui::PANEL);
        d.outline((0.0, 0.0, 76.0, 216.0), pleasant_ui::LINE);
        for (i, mode) in crate::params::GraphRangeMode::PLEASANT.iter().enumerate() {
            let y = i as f32 * 18.0;
            if range == mode.range_db() {
                d.rect(1.0, y, 74.0, 18.0, pleasant_ui::LINE);
            }
            d.text(
                10.0,
                y + 13.0,
                &format!("-{} dB", mode.range_db() as i32),
                11.0,
                pleasant_ui::TEXT,
            );
        }
    }
}

fn build_range_selector(cx: &mut Context) {
    RangeButton
        .build(cx, |cx| {
            PopupData::default().build(cx);
            Popup::new(cx, PopupData::is_open, true, |cx| {
                RangeMenu {
                    param: nih_plug_vizia::widgets::param_base::ParamWidgetBase::new(
                        cx,
                        super::EditorData::params,
                        |p| &p.graph_range_mode,
                    ),
                }
                .build(cx, |_| {})
                .width(Pixels(76.0))
                .height(Pixels(216.0));
            })
            .on_blur(|cx| cx.emit(PopupEvent::Close))
            .left(Pixels(0.0))
            .top(super::EditorData::params.map(|p| Pixels(12.0 - range_button_y(p))))
            .width(Pixels(76.0))
            .height(Pixels(216.0));
        })
        .z_index(20)
        .position_type(PositionType::SelfDirected)
        .left(Pixels(0.0))
        .top(super::EditorData::params.map(|p| Pixels(range_button_y(p))))
        .width(Pixels(AXIS_W))
        .height(Pixels(20.0))
        .display(super::EditorData::appearance.map(|mode| {
            if *mode == 2 {
                Display::None
            } else {
                Display::Flex
            }
        }));
}
