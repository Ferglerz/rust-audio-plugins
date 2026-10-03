use crate::{
    dsp::{inv_s_curve, s_curve},
    params::{MidiAssign, TapeStopParams},
    telemetry::TapeStopTelemetry,
};
use nih_plug::prelude::*;
use nih_plug_vizia::widgets::util::ModifiersExt;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{
        prelude::*,
        vg::{Color, FontId},
    },
    widgets::RawParamEvent,
    ViziaState, ViziaTheming,
};
use pleasant_ui::{
    draw::{ButtonAnim, Draw, EditorViewport},
    preferences::AppearanceStore,
    theme::{rgb, COLORS, GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
    value_edit::{parse_number_with_units, typed_char, ValueEdit},
    FONT_JETBRAINS_MONO,
};
use std::{
    cell::Cell,
    sync::{atomic::Ordering, Arc, OnceLock},
    time::Duration,
};

mod controls;
mod events;
mod graph;
mod render;

static PREFS: OnceLock<AppearanceStore> = OnceLock::new();
fn prefs() -> &'static AppearanceStore {
    PREFS.get_or_init(|| AppearanceStore::new("Tape Stop"))
}

// Outer margin matches y-axis % label left edge (GRAPH_X - 40 = 20).
// Align the outer edge with the remaining drop-time module.
const UI_W: f32 = 812.0;
// Leave room beneath the graph for the cog section’s horizontal slider row.
const UI_H: f32 = 473.0;
const HEADER_HEIGHT: f32 = 70.0;

pub fn format_ms(seconds: f32) -> String {
    format!("{:.0}ms", (seconds * 1000.0).round())
}

pub fn snap_drop_time(val: f32) -> f32 {
    let val = val.clamp(0.05, 16.0);
    if val < 1.0 {
        let steps = (val / 0.05).round() as i32;
        ((steps * 5) as f32 / 100.0).clamp(0.05, 1.0)
    } else {
        let steps = (val / 0.25).round() as i32;
        (steps as f32 * 0.25).clamp(1.0, 16.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KnobId {
    DropTime,
    Return,
    Xfade,
    Curve,
    StereoDiv,
    RestartThresh,
    OverrideCc,
    OverrideNote,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum DragState {
    Value {
        id: KnobId,
        start_x: f32,
        start_y: f32,
        start_norm: f32,
    },
    Slider {
        id: KnobId,
        start_x: f32,
        start_y: f32,
        start_norm: f32,
        vertical: bool,
    },
    Curve {
        start_x: f32,
        start_y: f32,
        start_norm: f32,
    },
}

pub struct TapeStopView {
    params: Arc<TapeStopParams>,
    telemetry: Arc<TapeStopTelemetry>,
    font: Cell<Option<FontId>>,
    drag: Option<DragState>,
    edit: Option<ValueEdit<KnobId>>,
    value_press: Option<pleasant_ui::pointer::ValuePress<KnobId>>,
    hover_stop: Cell<bool>,
    stop_click: Cell<f32>,
    hover: Option<(f32, f32)>,
    curve_hover_anim: Cell<f32>,
    auto_restart_anim: ButtonAnim,
    show_axis_controls: bool,
}

impl View for TapeStopView {
    fn element(&self) -> Option<&'static str> {
        Some("tape-stop-view")
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
    ViziaState::new_screen_sized("Tape Stop", || (UI_W as u32, UI_H as u32))
}

pub fn create(
    params: Arc<TapeStopParams>,
    telemetry: Arc<TapeStopTelemetry>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            nih_plug_vizia::assets::register_noto_sans_light(cx);
            TapeStopView {
                params: params.clone(),
                telemetry: telemetry.clone(),
                font: Cell::new(None),
                drag: None,
                edit: None,
                value_press: None,
                hover_stop: Cell::new(false),
                stop_click: Cell::new(0.0),
                hover: None,
                curve_hover_anim: Cell::new(0.0),
                auto_restart_anim: ButtonAnim::new(),
                show_axis_controls: false,
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

#[cfg(test)]
mod tests {
    use super::{controls::*, graph::*, *};

    #[test]
    fn test_snap_drop_time() {
        assert_eq!(snap_drop_time(0.01), 0.05);
        assert_eq!(snap_drop_time(0.05), 0.05);
        assert_eq!(snap_drop_time(0.07), 0.05);
        assert_eq!(snap_drop_time(0.08), 0.10);
        assert_eq!(snap_drop_time(0.50), 0.50);
        assert_eq!(snap_drop_time(0.99), 1.00);
        assert_eq!(snap_drop_time(1.00), 1.00);
        assert_eq!(snap_drop_time(1.10), 1.00);
        assert_eq!(snap_drop_time(1.13), 1.25);
        assert_eq!(snap_drop_time(1.25), 1.25);
        assert_eq!(snap_drop_time(2.40), 2.50);
        assert_eq!(snap_drop_time(16.0), 16.0);
        assert_eq!(snap_drop_time(20.0), 16.0);
    }

    #[test]
    fn test_stop_and_midi_overlay_layout() {
        assert_eq!(STOP_BUTTON.2, 68.0);
        assert_eq!(STOP_BUTTON.3, 68.0);
        assert!(STOP_BUTTON.0 >= GRAPH_X);
        assert!(STOP_BUTTON.1 >= GRAPH_Y);
        assert!(STOP_BUTTON.0 + STOP_BUTTON.2 <= GRAPH_X + GRAPH_W);
        assert!(STOP_BUTTON.1 + STOP_BUTTON.3 <= GRAPH_Y + GRAPH_H);
        let midi = midi_slot();
        let ret = axis_slot(0);
        assert!(midi.1 + midi.3 < ret.1);
        assert!(midi.1 >= GRAPH_Y + GRAPH_H);
        assert_eq!(midi.2, 96.0);
        assert!(midi_value_rect().2 >= 50.0);
        assert!(midi_value_rect().0 >= midi_label_rect().0 + midi_label_rect().2 - 0.5);
    }

    #[test]
    fn test_drop_time_slider_is_right_of_graph() {
        assert!((GRAPH_W - 852.0 * 0.8).abs() < 1.0e-3);
        assert!((GRAPH_W + DROP_SLIDER_GAP + DROP_SLIDER_W - 731.6).abs() < 1.0e-3);
        assert!(DROP_TIME_SLIDER.0 >= GRAPH_X + GRAPH_W);
        assert!(DROP_TIME_SLIDER.0 + DROP_TIME_SLIDER.2 <= UI_W);
        assert!(DROP_TIME_SLIDER.3 > GRAPH_H);
        let bar = TapeStopView::drop_bar_rect();
        assert!((bar.1 + bar.3 - (GRAPH_Y + GRAPH_H)).abs() < f32::EPSILON);
        assert!(DROP_TIME_READOUT.1 >= GRAPH_Y + GRAPH_H);
        assert!((DROP_TIME_READOUT.1 + DROP_READOUT_TEXT_Y - AXIS_LABEL_Y).abs() < f32::EPSILON);
        let outer_margin = GRAPH_X - 40.0;
        assert!((UI_W - MODULE_RIGHT - outer_margin).abs() < 1.0);
        assert!((THEME_BUTTON.0 + THEME_BUTTON.2 - MODULE_RIGHT).abs() < 0.5);
        let cog = cog_rect();
        assert!(cog.0 + cog.2 <= DROP_TIME_READOUT.0);
        let slots = [axis_slot(0), axis_slot(1), axis_slot(2), trigger_column()];
        for (i, slot) in slots.iter().enumerate() {
            assert!(slot.1 > DROP_TIME_READOUT.1 + DROP_TIME_READOUT.3);
            assert!(slot.1 + slot.3 < UI_H);
            assert!(slot.0 + slot.2 <= MODULE_RIGHT);
            let bar = axis_bar_rect(*slot);
            let value = axis_value_rect(*slot);
            assert!(bar.2 > 100.0);
            assert!(bar.1 > value.1 + value.3);
            assert!(bar.1 + bar.3 <= slot.1 + slot.3);
            if i > 0 {
                assert!(slots[i - 1].0 + slots[i - 1].2 < slot.0);
            }
        }
        let bypass = trigger_bypass_rect();
        let value = trigger_value_rect();
        assert!(bypass.0 + bypass.2 < value.0);
        assert!(bypass.1 + bypass.3 < trigger_bar_rect().1);
        let pts = TapeStopView::drop_l_points();
        let s = DROP_TIME_SLIDER;
        let r = DROP_TIME_READOUT;
        assert_eq!(pts[0], (s.0, s.1));
        assert_eq!(pts[1], (s.0 + s.2, s.1));
        assert_eq!(pts[2], (s.0 + s.2, r.1 + r.3));
        assert_eq!(pts[3], (r.0, r.1 + r.3));
        assert_eq!(pts[4], (r.0, r.1));
        assert_eq!(pts[5], (s.0, r.1));
    }

    #[test]
    fn stereo_curves_share_a_start_and_drift_to_zero() {
        let div = 50.0;
        let left_scale = stereo_time_scale(div, false);
        let right_scale = stereo_time_scale(div, true);
        assert!((left_scale - 0.75).abs() < 1.0e-5);
        assert!((right_scale - 1.25).abs() < 1.0e-5);
        let left = stereo_brake_points(GRAPH_X, GRAPH_Y, GRAPH_W, GRAPH_H, 1.0, left_scale, 200);
        let right = stereo_brake_points(GRAPH_X, GRAPH_Y, GRAPH_W, GRAPH_H, 1.0, right_scale, 200);
        assert!((left[0].0 - right[0].0).abs() < 1.0e-3);
        assert!((left[0].1 - right[0].1).abs() < 1.0e-3);
        assert!((left[0].0 - GRAPH_X).abs() < 1.0e-3);
        let end_l = left.last().copied().unwrap();
        assert!((end_l.0 - (GRAPH_X + GRAPH_W * left_scale)).abs() < 1.0);
        assert!((end_l.1 - (GRAPH_Y + GRAPH_H)).abs() < 1.0e-3);
        let end_r = right.last().copied().unwrap();
        assert!((end_r.0 - (GRAPH_X + GRAPH_W)).abs() < 1.0e-2);
        assert!(end_r.1 < end_l.1 - 4.0);
    }

    #[test]
    fn test_format_ms() {
        assert_eq!(format_ms(0.0), "0ms");
        assert_eq!(format_ms(0.05), "50ms");
        assert_eq!(format_ms(0.50), "500ms");
        assert_eq!(format_ms(1.25), "1250ms");
        assert_eq!(format_ms(16.0), "16000ms");
    }
}
