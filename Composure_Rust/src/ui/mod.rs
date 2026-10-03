//! Vizia UI — single production page (1182×504). No multi-page nav.

mod appearance;
#[path = "controls/controls.rs"]
mod controls;
mod debug_view;
mod detector_eq_view;
mod display;
mod draw_helpers;
mod envelope_view;
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
#[path = "controls/image_knob.rs"]
mod image_knob;
#[path = "controls/image_switch.rs"]
mod image_switch;
mod layout;
#[path = "controls/param_button.rs"]
mod param_button;
#[path = "controls/param_widget_ext.rs"]
mod param_widget_ext;
#[path = "controls/step_points.rs"]
mod step_points;
mod theme;
#[path = "graph/threshold_lines.rs"]
mod threshold_lines;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug::debug::nih_error;
use nih_plug::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::ResizeHandle;
use nih_plug_vizia::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::params::ComposureParams;

pub use appearance::{
    PLEASANT_EDITOR_HEIGHT as EDITOR_HEIGHT, PLEASANT_EDITOR_WIDTH as EDITOR_WIDTH,
};
pub use display::UiDisplay;

#[derive(Lens, Clone)]
pub struct EditorData {
    pub params: Arc<ComposureParams>,
    pub display: Arc<UiDisplay>,
    pub appearance: u8,
    pub appearance_choice: pleasant_ui::preferences::Appearance,
    pub font: Arc<std::sync::OnceLock<nih_plug_vizia::vizia::vg::FontId>>,
}

impl Model for EditorData {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|mode: &appearance::SetAppearance, _| {
            self.appearance_choice = appearance::decode(mode.0);
            self.appearance_choice.write("Composure");
            self.appearance = appearance::encode(self.appearance_choice.resolved());
            self.params
                .editor_width
                .store(appearance::editor_width(self.appearance), Ordering::Relaxed);
            cx.emit(nih_plug_vizia::widgets::GuiContextEvent::Resize);
            cx.needs_redraw();
        });
        event.map(|_: &appearance::RefreshAppearance, _| {
            if self.appearance_choice == pleasant_ui::preferences::Appearance::Auto {
                let resolved = appearance::encode(self.appearance_choice.resolved());
                if self.appearance != resolved {
                    self.appearance = resolved;
                    cx.needs_redraw();
                }
            }
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
        .top(EditorData::appearance.map(|mode| Pixels(appearance::graph_y(*mode))))
        .width(Pixels(appearance::PLEASANT_METERS_W))
        .height(Pixels(appearance::PLEASANT_METER_HEIGHT));
}

pub fn preferred_editor_width() -> u32 {
    let mode = appearance::encode(appearance::read_choice());
    appearance::editor_width(mode)
}

pub fn default_editor_state(width: Arc<std::sync::atomic::AtomicU32>) -> Arc<ViziaState> {
    ViziaState::new_screen_sized("Composure", move || {
        let width = width.load(Ordering::Relaxed);
        (width, EDITOR_HEIGHT)
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

    EditorData {
        params: params.clone(),
        display: display.clone(),
        appearance: appearance::encode(appearance::read_choice().resolved()),
        appearance_choice: appearance::read_choice(),
        font: Arc::new(std::sync::OnceLock::new()),
    }
    .build(cx);
    graph_pages::build_model(cx);

    ZStack::new(cx, |cx| {
        appearance::Background::new(cx)
            .position_type(PositionType::SelfDirected)
            .width(Stretch(1.0))
            .height(Stretch(1.0));

        ZStack::new(cx, |cx| {
            controls::build_positioned_controls(cx, EditorData::params, display.clone());
            appearance::build_harmonics_bypass(cx);
            appearance::build_program_bypass(cx);
            debug_view::build(cx, EditorData::display);

            let initial_points = params.graph_store.load().graph.num_points;
            display.sync_graph_points(initial_points);

            let gr_lens = EditorData::display.map(|d| d.gr_db.load(Ordering::Relaxed));
            let range_lens =
                EditorData::params.map(|p| p.graph_range_mode.value().range_db() as f32);
            graph_pages::build(cx, params.clone(), display.clone());

            place_gr_meter(
                cx,
                EditorData::appearance.map(|mode| Pixels(appearance::meter_x(*mode).0)),
                gr_lens.clone(),
                range_lens.clone(),
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
    .class("pleasant")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_construction_finishes_without_hanging() {
        let (finished, result) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut cx = Context::new(WindowSize::new(1182, 504), 1.0);
            build_editor_contents(
                &mut cx,
                Arc::new(ComposureParams::default()),
                Arc::new(UiDisplay::default()),
            );
            finished.send(()).unwrap();
        });

        result
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("Composure editor construction must finish within five seconds");
        worker.join().unwrap();
    }
}
