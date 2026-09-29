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
    draw::{ButtonAnim, Draw},
    math::linear_to_db,
    preferences::AppearanceStore,
    theme::{rgb, BG, COLORS, GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
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
// MODULE_RIGHT (~881.6) + OUTER_MARGIN, rounded to integer window width.
const UI_W: f32 = 902.0;
// Chrome bottom (DROP/TRIGGER readout) is 405; + OUTER_MARGIN (20) → 425.
const UI_H: f32 = 425.0;
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
    hover_thresh: Cell<bool>,
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
                hover_thresh: Cell::new(false),
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
        assert!((midi.1 - ret.1).abs() < f32::EPSILON);
        assert!((midi.3 - ret.3).abs() < f32::EPSILON);
        assert!(midi.0 + midi.2 <= ret.0);
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
        let trigger = trigger_column();
        assert!(trigger.0 >= DROP_TIME_SLIDER.0 + DROP_TIME_SLIDER.2);
        assert!((trigger.1 - DROP_TIME_SLIDER.1).abs() < f32::EPSILON);
        assert!((trigger.2 - DROP_TIME_SLIDER.2).abs() < f32::EPSILON);
        assert!((trigger.3 - DROP_TIME_SLIDER.3).abs() < f32::EPSILON);
        assert!(trigger.0 + trigger.2 <= UI_W);
        let drop_bar = TapeStopView::drop_bar_rect();
        let trig_bar = trigger_bar_rect();
        assert!((trig_bar.1 - drop_bar.1).abs() < f32::EPSILON);
        assert!((trig_bar.2 - drop_bar.2).abs() < f32::EPSILON);
        assert!((trig_bar.3 - drop_bar.3).abs() < f32::EPSILON);
        let title = trigger_title_rect();
        assert!(title.1 + title.3 <= trigger.1);
        let chrome = trigger_chrome_rect();
        assert!((chrome.0 - trigger.0).abs() < f32::EPSILON);
        assert!((chrome.1 - trigger.1).abs() < f32::EPSILON);
        assert!((chrome.2 - TRIGGER_W).abs() < f32::EPSILON);
        assert!(chrome.0 + chrome.2 <= UI_W);
        // Y-axis labels draw at GRAPH_X - 40; that left edge is the outer-margin reference.
        let outer_margin = GRAPH_X - 40.0;
        assert!((outer_margin - 20.0).abs() < f32::EPSILON);
        let right_pad = UI_W - (chrome.0 + chrome.2);
        assert!((right_pad - outer_margin).abs() < 1.0);
        assert!((THEME_BUTTON.0 + THEME_BUTTON.2 - (chrome.0 + chrome.2)).abs() < 0.5);
        let chrome_bottom = chrome.1 + chrome.3;
        let bottom_gap = UI_H - chrome_bottom;
        assert!((chrome_bottom - 405.0).abs() < f32::EPSILON);
        assert!((bottom_gap - outer_margin).abs() < f32::EPSILON);
        let bypass = trigger_bypass_rect();
        assert!((bypass.1 - (trigger.1 + 6.0)).abs() < f32::EPSILON);
        assert!((bypass.0 - (trigger.0 + DROP_SLIDER_W + 4.0)).abs() < f32::EPSILON);
        assert!(bypass.0 + bypass.2 <= chrome.0 + chrome.2);
        assert!(bypass.1 + bypass.3 <= chrome.1 + chrome.3);
        let value = trigger_value_rect();
        assert!((value.1 - DROP_TIME_READOUT.1).abs() < f32::EPSILON);
        assert!((value.3 - DROP_TIME_READOUT.3).abs() < f32::EPSILON);
        assert!((value.0 - trigger.0).abs() < f32::EPSILON);
        assert!((chrome.1 + chrome.3 - (value.1 + value.3)).abs() < f32::EPSILON);
        assert!(value.0 + value.2 <= chrome.0 + chrome.2 + f32::EPSILON);
        let cog = cog_rect();
        assert!(cog.0 + cog.2 <= DROP_TIME_READOUT.0);
        assert!(
            (cog.1 + cog.3 * 0.5 - (DROP_TIME_READOUT.1 + DROP_TIME_READOUT.3 * 0.5)).abs() < 1.0
        );
        let ret = axis_slot(0);
        let xfade = axis_slot(1);
        let stereo = axis_slot(2);
        assert!(midi_slot().0 + midi_slot().2 <= ret.0);
        assert!((ret.3 - DROP_READOUT_H).abs() < f32::EPSILON);
        assert!((ret.1 - DROP_TIME_READOUT.1).abs() < f32::EPSILON);
        assert!(ret.0 + ret.2 <= xfade.0);
        assert!(xfade.0 + xfade.2 <= stereo.0);
        assert!(stereo.0 + stereo.2 <= cog.0);
        assert!(axis_bar_rect(ret).2 > 8.0);
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
