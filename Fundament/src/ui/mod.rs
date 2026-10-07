//! Custom canvas editor for Fundament.

use crate::{params::FundamentParams, telemetry::FundamentTelemetry};
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{prelude::*, vg::FontId},
    ViziaState, ViziaTheming,
};
use pleasant_ui::{draw::ButtonAnim, preferences::AppearanceStore, value_edit::ValueEdit};
use std::{
    cell::Cell,
    sync::{Arc, OnceLock},
    time::Duration,
};

mod controls;
mod events;
mod render;
mod spectrum;

pub const UI_W: u32 = 860;
pub const UI_H: u32 = 460;
const VIEW_W: f32 = UI_W as f32;
const VIEW_H: f32 = UI_H as f32;

static PREFS: OnceLock<AppearanceStore> = OnceLock::new();

fn prefs() -> &'static AppearanceStore {
    PREFS.get_or_init(|| AppearanceStore::new("Fundament"))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ControlId {
    Voices,
    Harmonics,
    Low,
    High,
    Sensitivity,
    CutDepth,
    CutWidth,
    SynthLevel,
    Tone,
    Waveform,
    Attack,
    Release,
    Glide,
    Mix,
    Output,
}

#[derive(Clone, Copy, Debug)]
struct Drag {
    start_y: f32,
    start_norm: f32,
}

/// Mouse-down on a control body. Becomes a drag after a few pixels of vertical movement.
#[derive(Clone, Copy, Debug)]
struct Arm {
    id: ControlId,
    origin_y: f32,
}

struct FundamentView {
    params: Arc<FundamentParams>,
    telemetry: Arc<FundamentTelemetry>,
    font: Cell<Option<FontId>>,
    drag: Option<Drag>,
    gesture: Option<ParamPtr>,
    arm: Option<Arm>,
    edit: Option<ValueEdit<ControlId>>,
    value_press: Option<pleasant_ui::pointer::ValuePress<ControlId>>,
    hover: Option<(f32, f32)>,
    bypass_anim: ButtonAnim,
}

impl View for FundamentView {
    fn element(&self) -> Option<&'static str> {
        Some("fundament-view")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let was_editing = self.edit.is_some();
        self.handle_event(cx, event);
        pleasant_ui::value_edit::sync_text_input(cx, was_editing, self.edit.is_some());
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        self.render(cx, canvas);
    }
}

pub fn default_editor_state() -> Arc<ViziaState> {
    ViziaState::new_screen_sized("Fundament", || (UI_W, UI_H))
}

pub fn create(
    params: Arc<FundamentParams>,
    telemetry: Arc<FundamentTelemetry>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            nih_plug_vizia::assets::register_noto_sans_light(cx);
            FundamentView {
                params: params.clone(),
                telemetry: telemetry.clone(),
                font: Cell::new(None),
                drag: None,
                gesture: None,
                arm: None,
                edit: None,
                value_press: None,
                hover: None,
                bypass_anim: ButtonAnim::new(),
            }
            .build(cx, |cx| {
                let timer = cx.add_timer(Duration::from_millis(16), None, |cx, action| {
                    if let TimerAction::Tick(_) = action {
                        cx.needs_redraw();
                    }
                });
                cx.start_timer(timer);
            })
            .width(Stretch(1.0))
            .height(Stretch(1.0));
        },
    )
}
