//! Graph chrome — point count + axis labels.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug_vizia::vizia::prelude::*;

use crate::dsp::constants::MAX_POINTS;
use crate::params::ComposureParams;

use super::display::UiDisplay;
use super::graph_display;
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
        .left(Pixels(32.0 + theme::sx(8.0)))
        .top(Pixels(21.0));

    build_axis_labels(cx, params);

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
        .left(
            super::EditorData::appearance
                .map(|mode| Pixels(32.0 + super::appearance::graph_size(*mode) - theme::sx(168.0))),
        )
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
        let label_lens = params
            .map(move |p: &Arc<ComposureParams>| axis_label_text(p, index).unwrap_or_default());
        for mode in [0_u8, 2_u8] {
            let top_lens = params.clone().map(move |p: &Arc<ComposureParams>| {
                Pixels(axis_label_y(p, index, mode) - super::appearance::graph_y(mode) + 12.0)
            });
            Label::new(cx, label_lens.clone())
                .class("graph-axis-label")
                .display(super::EditorData::appearance.map(move |current| {
                    if (*current == 2) == (mode == 2) {
                        Display::Flex
                    } else {
                        Display::None
                    }
                }))
                .position_type(PositionType::SelfDirected)
                .left(Pixels(32.0 - theme::sx(30.0)))
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
