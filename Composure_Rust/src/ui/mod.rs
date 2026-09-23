//! Vizia UI — single production page (1182×504). No multi-page nav.

mod ui_assets;
mod controls;
mod debug_view;
mod display;
mod draw_helpers;
mod graph_chrome;
pub mod graph_display;
mod texture_cache;
mod image_knob;
mod image_switch;
mod parallax_slider;
mod image_draw;
mod readout_controls;
mod graph_view;
mod graph_view_draw;
mod gr_meter_view;
mod layout;
mod theme;
mod step_points;
mod param_widget_ext;
mod threshold_lines;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug::debug::nih_error;
use nih_plug::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{create_vizia_editor, ViziaState, ViziaTheming};

/// Scaled from `../Composure/Images/BG.png` at 32% (1182×504).
const EDITOR_BG_IMAGE: &str = "composure_bg.png";

use crate::params::ComposureParams;

pub use display::UiDisplay;
pub use theme::{EDITOR_HEIGHT, EDITOR_WIDTH};

#[derive(Lens, Clone)]
pub struct EditorData {
    pub params: Arc<ComposureParams>,
    pub display: Arc<UiDisplay>,
}

impl Model for EditorData {}

fn place_gr_meter<L, R, P>(
    cx: &mut Context,
    x: f32,
    gr_db: L,
    range_db: R,
    params: P,
    display: Arc<UiDisplay>,
) where
    L: Lens<Target = f32> + Clone,
    R: Lens<Target = f32> + Clone,
    P: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
{
    gr_meter_view::GrMeterView::new(cx, gr_db, range_db, params, display)
        .position_type(PositionType::SelfDirected)
        .left(Pixels(x))
        .top(Pixels(theme::METER_Y))
        .width(Pixels(theme::METER_W))
        .height(Pixels(theme::METER_H));
}

pub fn default_editor_state() -> Arc<ViziaState> {
    ViziaState::new(|| (EDITOR_WIDTH, EDITOR_HEIGHT))
}

pub fn create(params: Arc<ComposureParams>, display: Arc<UiDisplay>) -> Option<Box<dyn Editor>> {
    let editor_state = params.editor_state.clone();
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        nih_plug_vizia::assets::register_noto_sans_light(cx);
        nih_plug_vizia::assets::register_noto_sans_thin(cx);

        if let Err(err) = cx.add_stylesheet(include_style!("src/ui/theme.css")) {
            nih_error!("Failed to load stylesheet: {err:?}");
        }

        ui_assets::register_editor_images(cx);
        texture_cache::clear_gpu_textures();

        EditorData {
            params: params.clone(),
            display: display.clone(),
        }
        .build(cx);

        ZStack::new(cx, |cx| {
            Element::new(cx)
                .class("editor-bg")
                .position_type(PositionType::SelfDirected)
                .left(Pixels(0.0))
                .top(Pixels(0.0))
                .width(Pixels(EDITOR_WIDTH as f32))
                .height(Pixels(EDITOR_HEIGHT as f32));

            controls::build_positioned_controls(cx, EditorData::params, display.clone());
            debug_view::build(cx, EditorData::display);

            let initial_points = params.graph_store.load().graph.num_points;
            display.sync_graph_points(initial_points);

            graph_chrome::build(cx, EditorData::params, EditorData::display);

            let gr_lens = EditorData::display.map(|d| d.gr_db.load(Ordering::Relaxed));
            let range_lens = EditorData::params.map(|p| p.graph_range_mode.value().range_db() as f32);

            graph_view::GraphView::new(
                cx,
                params.graph_store.clone(),
                display.clone(),
                EditorData::display.map(|d| d.detector_db.load(Ordering::Relaxed)),
                gr_lens.clone(),
                EditorData::params,
            )
            .position_type(PositionType::SelfDirected)
            .left(Pixels(theme::GRAPH_X))
            .top(Pixels(theme::GRAPH_Y))
            .width(Pixels(theme::GRAPH_SIZE_X))
            .height(Pixels(theme::GRAPH_SIZE));

            place_gr_meter(
                cx,
                theme::METER_X,
                gr_lens.clone(),
                range_lens.clone(),
                EditorData::params,
                display.clone(),
            );
            place_gr_meter(
                cx,
                theme::METER_X_RIGHT,
                gr_lens,
                range_lens,
                EditorData::params,
                display.clone(),
            );
        })
        .id("root")
        .width(Pixels(EDITOR_WIDTH as f32))
        .height(Pixels(EDITOR_HEIGHT as f32));

        ResizeHandle::new(cx);
    })
}
