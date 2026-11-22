//! Bottom hover help, drawn separately from the parameter widgets.
use super::{appearance::*, layout::ControlLayout, theme};
use nih_plug_vizia::vizia::prelude::*;
use pleasant_ui::{draw::TOOLTIP_HEIGHT, point_in_rect, BG, MUTED};

struct TooltipView;

pub(super) fn build(cx: &mut Context) {
    TooltipView
        .build(cx, |cx| {
            // Observe sibling controls even when they capture pointer events.
            cx.add_listener(|_: &mut TooltipView, cx, event| {
                event.map(|event: &WindowEvent, _| {
                    if matches!(
                        event,
                        WindowEvent::MouseMove(..)
                            | WindowEvent::MouseEnter
                            | WindowEvent::MouseLeave
                    ) {
                        cx.needs_redraw();
                    }
                });
            });
        })
        .hoverable(false)
        .position_type(PositionType::SelfDirected)
        .left(Pixels(0.0))
        .top(Pixels(PLEASANT_EDITOR_HEIGHT as f32 - TOOLTIP_HEIGHT))
        .width(Pixels(PLEASANT_EDITOR_WIDTH as f32))
        .height(Pixels(TOOLTIP_HEIGHT));
}

impl View for TooltipView {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let b = cx.bounds();
        let scale = b.w / PLEASANT_EDITOR_WIDTH as f32;
        if scale <= 0.0 {
            return;
        }
        let x = (cx.mouse().cursorx - b.x) / scale;
        let y = (cx.mouse().cursory - b.y) / scale + PLEASANT_EDITOR_HEIGHT as f32 - TOOLTIP_HEIGHT;
        let mut d = super::appearance::painter(cx, canvas, PLEASANT_EDITOR_WIDTH as f32);
        d.rect(0.0, 0.0, PLEASANT_EDITOR_WIDTH as f32, TOOLTIP_HEIGHT, BG);
        d.bottom_tooltip(
            (
                32.0,
                0.0,
                PLEASANT_EDITOR_WIDTH as f32 - 64.0,
                TOOLTIP_HEIGHT,
            ),
            hover_help(x, y),
            MUTED,
        );
    }
}

fn hover_help(x: f32, screen_y: f32) -> &'static str {
    if screen_y < HEADER_H {
        return if x > PLEASANT_EDITOR_WIDTH as f32 - 160.0 {
            "Cycle Dark, Light, and Auto appearance."
        } else {
            "Hover a control for help · Drag to adjust · Click a value to type · Shift for fine adjustment"
        };
    }
    let y = screen_y - HEADER_H;
    let l = ControlLayout::pleasant();
    let standard = |pos: (f32, f32)| (pos.0, pos.1, theme::KNOB_SIZE, theme::KNOB_SIZE);
    let harmonic = |pos: (f32, f32)| (pos.0, pos.1, HARMONIC_KNOB_W, HARMONIC_KNOB_H);
    for (rect, text) in [
        (standard(l.rms_knob), "RMS: set the detector's averaging time. Drag to adjust; click the value to type."),
        ((l.strength_knob.0, l.strength_knob.1, STRENGTH_KNOB_SIZE, STRENGTH_KNOB_H), "Strength: scale the gain change produced by the transfer curve. Shift for fine adjustment."),
        (standard(l.offset_knob), "Offset: shift the input level used by the transfer curve. Click the value to type."),
        (standard((l.input_dependence_knob.0, l.input_dependence_knob.1)), "Input threshold dependence: vary the envelope response with the incoming level."),
        (standard((ENV_X + 136.0, PROG_KNOB_Y)), "Gain threshold dependence: vary the envelope response with gain reduction."),
        (standard((ENV_X + 258.0, PROG_KNOB_Y)), "Input slew: vary the envelope response with changes in the incoming level."),
        (harmonic(l.harmonic_drive), "Drive: set the amount of harmonic coloration. Enable Harmonics and raise Mix to hear it."),
        (harmonic(l.harmonic_mix), "Mix: blend the harmonic coloration with the clean signal."),
        (harmonic(l.harmonic_even), "Even: adjust the even-harmonic contribution."),
        (harmonic(l.harmonic_odd), "Odd: adjust the odd-harmonic contribution."),
        ((l.harmonic_type_switch.0, l.harmonic_type_switch.1, HARMONIC_BUTTON_W, 38.0), "Choose the harmonic circuit character."),
        ((l.makeup_knob.0, l.makeup_knob.1, GAIN_KNOB_SIZE, GAIN_KNOB_SIZE), "Gain: adjust output makeup gain. Drag to adjust; click the value to type."),
        ((l.ms_switch.0, l.ms_switch.1, MS_BUTTON_W, 28.0), "Switch between left/right and mid/side processing."),
        ((l.detection_btn.0, l.detection_btn.1, DETECTION_BUTTON_W, ADAPTIVE_BUTTON_H), "Switch the level detection mode."),
        ((l.norm_switch.0, l.norm_switch.1, theme::SWITCH_SLOT_W, 28.0), "Toggle adaptive RMS normalization."),
    ] {
        if point_in_rect(x, y, rect) { return text; }
    }
    if point_in_rect(x, y, (ENV_X, ENVELOPE_GRAPH_Y, ENV_W, ENVELOPE_GRAPH_H)) {
        return "Adjust the envelope graph to shape attack and release timing.";
    }
    if point_in_rect(x, y, (TRANS_X, ENV_Y, TRANS_W, SIDE_H)) {
        return "Edit the transfer curve to shape dynamics · Use SC EQ to edit detector filtering";
    }
    if point_in_rect(x, y, (HARM_X, ENV_Y, HARM_W, HARM_H)) {
        return "Harmonics: use the power icon to enable or bypass coloration; Mix blends it with the clean signal.";
    }
    if point_in_rect(x, y, (ENV_X, PROG_Y, ENV_W, PROG_H)) {
        return "Program dependence: use the power icon to enable or bypass the level-dependent envelope controls.";
    }
    "Hover a control for help · Drag to adjust · Click a value to type · Shift for fine adjustment"
}
