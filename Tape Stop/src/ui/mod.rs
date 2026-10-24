use crate::{
    dsp::{inv_s_curve, s_curve},
    params::{MidiAssign, PadMode, TapeStopParams, XAxis},
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
    ViziaState, ViziaTheming,
};
use pleasant_ui::{
    draw::{ButtonAnim, Draw, EditorViewport},
    preferences::AppearanceStore,
    theme::{rgb, COLORS, GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
    value_edit::{parse_number_with_units, ValueEdit},
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
mod performance;
mod render;
use performance::PerformanceTick;

static PREFS: OnceLock<AppearanceStore> = OnceLock::new();
fn prefs() -> &'static AppearanceStore {
    PREFS.get_or_init(|| AppearanceStore::new("Tape Stop"))
}

// Outer margin matches y-axis % label left edge (GRAPH_X - 40 = 20).
// Fit the graph, equally spaced time controls, and square performance pad.
const UI_W: f32 = controls::XY_PAD.0 + controls::XY_PAD.2 + 20.0;
// The expanded cog controls share the graph's readout row.
const UI_H: f32 = 430.0;
const HEADER_HEIGHT: f32 = 70.0;

pub fn format_ms(seconds: f32) -> String {
    let ms = seconds * 1000.0;
    if seconds < 1.0 && (ms - ms.round()).abs() >= 0.05 {
        format!("{ms:.1}ms")
    } else {
        format!("{ms:.0}ms")
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
    Tempo,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum DragState {
    Pad(pleasant_ui::xy_pad::Drag),
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
    axis_controls_progress: f32,
    gesture_params: [Option<ParamPtr>; 2],
    pad_binding: Option<(XAxis, PadMode, bool)>,
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
                axis_controls_progress: 0.0,
                gesture_params: [None; 2],
                pad_binding: None,
            }
            .build(cx, |cx| {
                let timer = cx.add_timer(Duration::from_millis(16), None, |cx, action| {
                    if let TimerAction::Tick(_) = action {
                        cx.emit(PerformanceTick);
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
mod interaction_tests;

#[cfg(test)]
mod tests {
    use super::{controls::*, graph::*, *};

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
        assert_eq!(midi.1, ret.1);
        assert!(midi.0 + midi.2 < ret.0);
        assert!(midi.1 >= GRAPH_Y + GRAPH_H);
        assert_eq!(midi.2, 66.0);
        assert!(midi_value_rect().2 >= 24.0);
        assert!(midi_value_rect().0 >= midi_label_rect().0 + midi_label_rect().2 - 0.5);
    }

    #[test]
    fn performance_layout_has_narrower_graph_and_equal_section_gaps() {
        assert!((GRAPH_W - 852.0 * 0.8 * 0.75).abs() < 1e-3);
        for gap in [
            DROP_TIME_READOUT.0 - (GRAPH_X + GRAPH_W),
            RETURN_TIME_READOUT.0 - (DROP_TIME_READOUT.0 + DROP_TIME_READOUT.2),
            XY_PAD.0 - (RETURN_TIME_READOUT.0 + RETURN_TIME_READOUT.2),
        ] {
            assert!((gap - SECTION_GAP).abs() < 1e-3);
        }
        assert!(DROP_TIME_SLIDER.0 > GRAPH_X + GRAPH_W);
        assert!(RETURN_TIME_SLIDER.0 > DROP_TIME_READOUT.0 + DROP_TIME_READOUT.2);
        assert_eq!(DROP_TIME_SLIDER.1, RETURN_TIME_SLIDER.1);
        assert_eq!(DROP_TIME_SLIDER.2, RETURN_TIME_SLIDER.2);
        assert_eq!(DROP_TIME_SLIDER.3, RETURN_TIME_SLIDER.3);
        assert!(XY_PAD.0 > RETURN_TIME_READOUT.0 + RETURN_TIME_READOUT.2);
        assert_eq!(XY_PAD.1, GRAPH_Y);
        assert_eq!(XY_PAD.2, XY_PAD.3);
        assert!(XY_PAD.0 + XY_PAD.2 <= UI_W - 20.0);
        assert!(TIME_SYNC.0 + TIME_SYNC.2 < TEMPO_HEADER.source.0);
        assert!(TEMPO_HEADER.bpm.0 + TEMPO_HEADER.bpm.2 < THEME_BUTTON.0);
        let slots = [axis_slot(0), axis_slot(1), trigger_column()];
        assert_eq!(slots[0].2, slots[1].2);
        for (i, slot) in slots.iter().enumerate() {
            assert!(slot.1 + slot.3 < UI_H);
            assert!(slot.0 + slot.2 < cog_rect().0);
            if i > 0 {
                assert!(slots[i - 1].0 + slots[i - 1].2 < slot.0);
            }
        }
        for bounds in [
            (0.0, 0.0, UI_W, UI_H),
            (12.0, 8.0, 812.0, 430.0),
            (0.0, 0.0, UI_W * 1.5, UI_H * 1.5),
        ] {
            let viewport = EditorViewport::fit(bounds, (UI_W, UI_H)).unwrap();
            let point = (XY_PAD.0 + XY_PAD.2 * 0.25, XY_PAD.1 + XY_PAD.3 * 0.75);
            let local = viewport.to_local(
                viewport.x + point.0 * viewport.scale,
                viewport.y + point.1 * viewport.scale,
            );
            let (x, y) = pleasant_ui::xy_pad::normalized(XY_PAD, local);
            assert!((x - 0.25).abs() < 1e-5 && (y - 0.25).abs() < 1e-5);
        }
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
    fn stereo_playheads_follow_their_channel_curves() {
        for div in [-100.0, -44.7, 0.0, 50.0, 100.0] {
            for right in [false, true] {
                let scale = stereo_time_scale(div, right);
                for exponent in [0.25, 1.0, 4.0] {
                    for graph_time in [0.0_f32, 0.1, 0.25, 0.5, 0.75, 1.0] {
                        if graph_time > scale {
                            continue;
                        }
                        let progress = graph_time / scale;
                        let speed = 1.0 - crate::dsp::s_curve(progress, exponent);
                        let (x, y) = TapeStopView::live_playhead(speed, exponent, scale);
                        assert!((x - (GRAPH_X + graph_time * GRAPH_W)).abs() < 0.1);
                        assert!((y - (GRAPH_Y + (1.0 - speed) * GRAPH_H)).abs() < 1.0e-3);
                    }
                }
                let (x, y) = TapeStopView::live_playhead(0.0, 1.0, scale);
                assert!((x - (GRAPH_X + GRAPH_W * scale.min(1.0))).abs() < 1.0e-3);
                assert!((y - (GRAPH_Y + GRAPH_H * (1.0 / scale).min(1.0))).abs() < 1.0e-3);
            }
        }
    }

    #[test]
    fn test_format_ms() {
        assert_eq!(format_ms(0.0), "0ms");
        assert_eq!(format_ms(0.05), "50ms");
        assert_eq!(format_ms(0.50), "500ms");
        assert_eq!(format_ms(1.25), "1250ms");
        assert_eq!(format_ms(16.0), "16000ms");
        assert_eq!(format_ms(0.1374), "137.4ms");
    }
}
