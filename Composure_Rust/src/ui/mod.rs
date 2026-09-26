//! Vizia UI — single production page (1182×504). No multi-page nav.

mod appearance;
#[path = "controls/controls.rs"]
mod controls;
mod debug_view;
mod detector_eq_view;
mod display;
mod draw_helpers;
mod gr_meter_view;
#[path = "graph/graph_chrome.rs"]
mod graph_chrome;
#[path = "graph/graph_display.rs"]
pub mod graph_display;
#[path = "graph/graph_hit.rs"]
mod graph_hit;
mod graph_pages;
#[path = "graph/graph_view.rs"]
mod graph_view;
#[path = "graph/graph_view_draw.rs"]
mod graph_view_draw;
mod image_draw;
#[path = "controls/image_knob.rs"]
mod image_knob;
#[path = "controls/image_switch.rs"]
mod image_switch;
mod layout;
#[path = "controls/parallax_slider.rs"]
mod parallax_slider;
#[path = "controls/param_widget_ext.rs"]
mod param_widget_ext;
#[path = "controls/readout_controls.rs"]
mod readout_controls;
#[path = "controls/step_points.rs"]
mod step_points;
mod texture_cache;
mod theme;
#[path = "graph/threshold_lines.rs"]
mod threshold_lines;
mod ui_assets;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug::debug::nih_error;
use nih_plug::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::ResizeHandle;
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
            self.params
                .editor_width
                .store(appearance::editor_width(mode.0), Ordering::Relaxed);
            cx.emit(nih_plug_vizia::widgets::GuiContextEvent::Resize);
            cx.needs_redraw();
        });
    }
}

fn place_gr_meter<L, R, P, X>(
    cx: &mut Context,
    x: X,
    second: bool,
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
        .top(EditorData::appearance.map(|mode| Pixels(appearance::graph_y(*mode))))
        .width(EditorData::appearance.map(move |mode| {
            Pixels(if *mode == 2 || second {
                theme::METER_W
            } else {
                appearance::PLEASANT_METERS_W
            })
        }))
        .height(EditorData::appearance.map(|mode| {
            Pixels(if *mode == 2 {
                appearance::graph_size(*mode)
            } else {
                appearance::PLEASANT_METER_HEIGHT
            })
        }))
        .display(EditorData::appearance.map(move |mode| {
            if second && *mode != 2 {
                Display::None
            } else {
                Display::Flex
            }
        }));
}

pub fn preferred_editor_width() -> u32 {
    let mode = appearance::encode(pleasant_ui::preferences::Appearance::read("Composure"));
    appearance::editor_width(mode)
}

pub fn default_editor_state(width: Arc<std::sync::atomic::AtomicU32>) -> Arc<ViziaState> {
    ViziaState::new_screen_sized("Composure", move || {
        let width = width.load(Ordering::Relaxed);
        let mode = if width == EDITOR_WIDTH { 2 } else { 0 };
        (width, appearance::editor_height(mode))
    })
}

pub fn create(params: Arc<ComposureParams>, display: Arc<UiDisplay>) -> Option<Box<dyn Editor>> {
    params
        .editor_width
        .store(preferred_editor_width(), Ordering::Relaxed);
    let editor_state = params.editor_state.clone();
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        build_editor_contents(cx, params.clone(), display.clone());
    })
}

fn build_editor_contents(cx: &mut Context, params: Arc<ComposureParams>, display: Arc<UiDisplay>) {
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
    graph_pages::build_model(cx);

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
            .width(Stretch(1.0))
            .height(Stretch(1.0));

        ZStack::new(cx, |cx| {
            controls::build_positioned_controls(cx, EditorData::params, display.clone());
            appearance::build_harmonics_bypass(cx);
            debug_view::build(cx, EditorData::display);

            let initial_points = params.graph_store.load().graph.num_points;
            display.sync_graph_points(initial_points);

            let gr_lens = EditorData::display.map(|d| d.gr_db.load(Ordering::Relaxed));
            let range_lens =
                EditorData::params.map(|p| p.graph_range_mode.value().range_db() as f32);
            graph_pages::build(cx, params.clone(), display.clone());
            graph_chrome::build_analog_readout(cx, EditorData::display);

            place_gr_meter(
                cx,
                EditorData::appearance.map(|mode| Pixels(appearance::meter_x(*mode).0)),
                false,
                gr_lens.clone(),
                range_lens.clone(),
                EditorData::params,
                display.clone(),
            );
            place_gr_meter(
                cx,
                EditorData::appearance.map(|mode| Pixels(appearance::meter_x(*mode).1)),
                true,
                gr_lens,
                range_lens,
                EditorData::params,
                display.clone(),
            );
        })
        .position_type(PositionType::SelfDirected)
        .top(EditorData::appearance.map(|mode| Pixels(appearance::content_offset(*mode))))
        .width(EditorData::appearance.map(|mode| Pixels(appearance::editor_width(*mode) as f32)))
        .height(EditorData::appearance.map(|mode| Pixels(appearance::editor_height(*mode) as f32)));

        appearance::build_selector(cx);
    })
    .id("root")
    .class("composure")
    .toggle_class("pleasant", EditorData::appearance.map(|m| *m != 2))
    .toggle_class("light", EditorData::appearance.map(|m| *m == 1))
    .width(Stretch(1.0))
    .height(Stretch(1.0));

    ResizeHandle::new(cx)
        .position_type(PositionType::SelfDirected)
        .right(Pixels(0.0))
        .bottom(Pixels(0.0))
        .width(Pixels(16.0))
        .height(Pixels(16.0));
}
