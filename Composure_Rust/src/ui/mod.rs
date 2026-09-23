//! Vizia UI — single production page (1182×504). No multi-page nav.

mod appearance;
mod controls;
mod debug_view;
mod display;
mod draw_helpers;
mod gr_meter_view;
mod graph_chrome;
pub mod graph_display;
mod graph_view;
mod graph_view_draw;
mod image_draw;
mod image_knob;
mod image_switch;
mod layout;
mod parallax_slider;
mod param_widget_ext;
mod readout_controls;
mod step_points;
mod texture_cache;
mod theme;
mod threshold_lines;
mod ui_assets;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug::debug::nih_error;
use nih_plug::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
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
    pub appearance: u8,
    pub font: Arc<std::sync::OnceLock<nih_plug_vizia::vizia::vg::FontId>>,
}

impl Model for EditorData {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|mode: &appearance::SetAppearance, _| {
            self.appearance = mode.0;
            appearance::decode(mode.0).write("Composure");
            cx.needs_redraw();
        });
    }
}

fn place_gr_meter<L, R, P, X>(
    cx: &mut Context,
    x: X,
    gr_db: L,
    range_db: R,
    params: P,
    display: Arc<UiDisplay>,
) where
    L: Lens<Target = f32> + Clone,
    R: Lens<Target = f32> + Clone,
    P: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    X: Lens<Target = Units>,
{
    gr_meter_view::GrMeterView::new(cx, gr_db, range_db, params, display)
        .position_type(PositionType::SelfDirected)
        .left(x)
        .top(Pixels(theme::METER_Y))
        .width(Pixels(theme::METER_W))
        .height(Pixels(theme::METER_H));
}

pub fn default_editor_state() -> Arc<ViziaState> {
    ViziaState::new_screen_sized("Composure", || (EDITOR_WIDTH, EDITOR_HEIGHT))
}

pub fn create(params: Arc<ComposureParams>, display: Arc<UiDisplay>) -> Option<Box<dyn Editor>> {
    let editor_state = params.editor_state.clone();
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        nih_plug_vizia::assets::register_noto_sans_light(cx);
        nih_plug_vizia::assets::register_noto_sans_thin(cx);
        cx.add_font_mem(pleasant_ui::FONT_JETBRAINS_MONO);

        if let Err(err) = cx.add_stylesheet(include_style!("src/ui/theme.css")) {
            nih_error!("Failed to load stylesheet: {err:?}");
        }

        ui_assets::register_editor_images(cx);
        texture_cache::clear_gpu_textures();

        EditorData {
            params: params.clone(),
            display: display.clone(),
            appearance: appearance::encode(pleasant_ui::preferences::Appearance::read("Composure")),
            font: Arc::new(std::sync::OnceLock::new()),
        }
        .build(cx);

        ZStack::new(cx, |cx| {
            Element::new(cx)
                .class("editor-bg")
                .display(EditorData::appearance.map(|m| {
                    if *m == 2 {
                        Display::Flex
                    } else {
                        Display::None
                    }
                }))
                .position_type(PositionType::SelfDirected)
                .left(Pixels(0.0))
                .top(Pixels(0.0))
                .width(Pixels(EDITOR_WIDTH as f32))
                .height(Pixels(EDITOR_HEIGHT as f32));

            appearance::Background::new(cx)
                .position_type(PositionType::SelfDirected)
                .width(Pixels(EDITOR_WIDTH as f32))
                .height(Pixels(EDITOR_HEIGHT as f32));

            ZStack::new(cx, |cx| {
                controls::build_positioned_controls(cx, EditorData::params, display.clone());
                debug_view::build(cx, EditorData::display);

                let initial_points = params.graph_store.load().graph.num_points;
                display.sync_graph_points(initial_points);

                graph_chrome::build(cx, EditorData::params, EditorData::display);

                let gr_lens = EditorData::display.map(|d| d.gr_db.load(Ordering::Relaxed));
                let range_lens =
                    EditorData::params.map(|p| p.graph_range_mode.value().range_db() as f32);

                graph_view::GraphView::new(
                    cx,
                    params.graph_store.clone(),
                    display.clone(),
                    EditorData::display.map(|d| d.detector_db.load(Ordering::Relaxed)),
                    gr_lens.clone(),
                    EditorData::params,
                )
                .position_type(PositionType::SelfDirected)
                .left(EditorData::appearance.map(|mode| Pixels(appearance::graph_x(*mode))))
                .top(Pixels(theme::GRAPH_Y))
                .width(Pixels(theme::GRAPH_SIZE_X))
                .height(Pixels(theme::GRAPH_SIZE));

                place_gr_meter(
                    cx,
                    EditorData::appearance.map(|mode| Pixels(appearance::meter_x(*mode).0)),
                    gr_lens.clone(),
                    range_lens.clone(),
                    EditorData::params,
                    display.clone(),
                );
                place_gr_meter(
                    cx,
                    EditorData::appearance.map(|mode| Pixels(appearance::meter_x(*mode).1)),
                    gr_lens,
                    range_lens,
                    EditorData::params,
                    display.clone(),
                );
            })
            .position_type(PositionType::SelfDirected)
            .top(EditorData::appearance.map(|mode| Pixels(appearance::content_offset(*mode))))
            .width(Pixels(EDITOR_WIDTH as f32))
            .height(Pixels(EDITOR_HEIGHT as f32));

            appearance::build_selector(cx);
        })
        .id("root")
        .class("composure")
        .toggle_class("pleasant", EditorData::appearance.map(|m| *m != 2))
        .toggle_class("light", EditorData::appearance.map(|m| *m == 1))
        .width(Pixels(EDITOR_WIDTH as f32))
        .height(Pixels(EDITOR_HEIGHT as f32));
    })
}
