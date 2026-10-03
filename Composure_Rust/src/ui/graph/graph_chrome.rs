//! Graph chrome — point count + axis labels.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug_vizia::vizia::prelude::*;

use crate::dsp::constants::MAX_POINTS;
use crate::params::ComposureParams;

use super::display::UiDisplay;
use super::graph_display;
use super::graph_pages::AXIS_W;

const MAX_AXIS_LABELS: usize = 12;
const RANGE_MENU_W: f32 = 76.0;
const RANGE_MENU_H: f32 = 216.0;
// Vizia's Popup adds 4px of translation; retain a 4px visible gap above the button.
const RANGE_MENU_TOP: f32 = -RANGE_MENU_H - 8.0;

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

fn build_axis_labels<P>(cx: &mut Context, params: P)
where
    P: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
{
    for index in 1..=MAX_AXIS_LABELS {
        {
            let mode = 0;
            let top_lens = params.clone().map(move |p: &Arc<ComposureParams>| {
                Pixels(axis_label_y(p, index, mode) - super::appearance::graph_y(mode) + 12.0)
            });
            let label_lens = params.map(move |p: &Arc<ComposureParams>| {
                let labels = graph_display::axis_label_db_values(graph_range_db(p));
                if index == labels.len() {
                    String::new()
                } else {
                    axis_label_text(p, index).unwrap_or_default()
                }
            });
            Label::new(cx, label_lens)
                .class("graph-axis-label")
                .position_type(PositionType::SelfDirected)
                .left(Pixels(AXIS_W - 33.10345))
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
            if matches!(e, WindowEvent::MouseMove(_, _) | WindowEvent::MouseLeave) {
                cx.needs_redraw();
            }
        });
    }
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let range = graph_range_db(&super::EditorData::params.get(cx));
        let mut d = super::appearance::painter(cx, canvas, AXIS_W);
        let highlighted = d.hover.is_some() || PopupData::is_open.get(cx);
        let tint = if highlighted {
            pleasant_ui::TEAL
        } else {
            pleasant_ui::TEXT
        };
        d.rounded_rect(1.0, 0.0, AXIS_W - 2.0, 20.0, 6.0, pleasant_ui::PANEL);
        d.rounded_rect(
            1.0,
            0.0,
            AXIS_W - 2.0,
            20.0,
            6.0,
            nih_plug_vizia::vizia::vg::Color {
                a: if highlighted { 0.14 } else { 0.045 },
                ..tint
            },
        );
        d.outline_rounded(
            1.0,
            0.0,
            AXIS_W - 2.0,
            20.0,
            6.0,
            if highlighted {
                pleasant_ui::TEAL
            } else {
                pleasant_ui::LINE
            },
            1.0,
        );
        d.text_centered(
            AXIS_W * 0.5,
            14.0,
            &format!("-{}", range as i32),
            13.0,
            if highlighted {
                pleasant_ui::TEXT
            } else {
                pleasant_ui::MUTED
            },
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
                .width(Pixels(RANGE_MENU_W))
                .height(Pixels(RANGE_MENU_H));
            })
            .on_blur(|cx| cx.emit(PopupEvent::Close))
            .left(Pixels(0.0))
            .top(Pixels(RANGE_MENU_TOP))
            .width(Pixels(RANGE_MENU_W))
            .height(Pixels(RANGE_MENU_H));
        })
        .z_index(20)
        .position_type(PositionType::SelfDirected)
        .left(Pixels(0.0))
        .top(super::EditorData::params.map(|p| Pixels(range_button_y(p))))
        .width(Pixels(AXIS_W))
        .height(Pixels(20.0));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn range_menu_stays_anchored_above_the_bottom_marker() {
        let params = ComposureParams::default();
        let button_y = range_button_y(&params);
        assert!(button_y + RANGE_MENU_TOP > 12.0);
        assert_eq!(RANGE_MENU_TOP + RANGE_MENU_H + 4.0, -4.0);
    }
}
