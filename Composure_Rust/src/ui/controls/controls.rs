//! Controls for the supported Dark and Light interfaces.
use super::display::UiDisplay;
use super::image_knob::ImageKnob;
use super::image_switch::{DetectionModeButton, ImageSwitch, ProgramEnableButton};
use super::layout::ControlLayout;
use super::param_button::ParamButton;
use super::step_points::StepSet;
use super::{appearance, theme};
use crate::params::ComposureParams;
use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;
use std::sync::Arc;

pub fn build_positioned_controls<L>(cx: &mut Context, params: L, display: Arc<UiDisplay>)
where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
{
    let l = ControlLayout::pleasant();
    super::envelope_view::EnvelopeView::new(cx, params.clone(), display.clone())
        .class("envelope-graph")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(appearance::ENVELOPE_GRAPH_X))
        .top(Pixels(appearance::ENVELOPE_GRAPH_Y))
        .width(Pixels(appearance::ENVELOPE_GRAPH_W))
        .height(Pixels(appearance::ENVELOPE_GRAPH_H));
    super::graph_pages::build_toggle(
        cx,
        (
            appearance::ENV_X + appearance::ENV_W - 10.0 - appearance::SC_EQ_BUTTON_W,
            appearance::ENV_Y + (appearance::MODULE_HEADER_H - 28.0) * 0.5,
            appearance::SC_EQ_BUTTON_W,
        ),
    );
    knob(
        cx,
        params.clone(),
        |p| &p.prog_release_blend,
        display.clone(),
        (l.prog_blend_slider.0, l.prog_blend_slider.1),
        (theme::KNOB_SIZE, theme::KNOB_SIZE),
        StepSet::Percent0_100,
    );
    knob(
        cx,
        params.clone(),
        |p| &p.input_rate_amount,
        display.clone(),
        (appearance::ENV_X + 136.0, l.prog_blend_slider.1),
        (theme::KNOB_SIZE, theme::KNOB_SIZE),
        StepSet::None,
    );
    for (input, y) in [
        (true, l.prog_blend_slider.1),
        (false, l.prog_blend_slider.1 + 36.0),
    ] {
        ProgramEnableButton::new(cx, input)
            .position_type(PositionType::SelfDirected)
            .left(Pixels(appearance::ENV_X + appearance::ENV_W - 130.0))
            .top(Pixels(y))
            .width(Pixels(120.0))
            .height(Pixels(28.0));
    }
    ParamButton::new(cx, params.clone(), |p| &p.prog_release_inverse)
        .class("jsfx-button")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(l.inverse_btn.0))
        .top(Pixels(l.inverse_btn.1))
        .width(Pixels(l.inverse_btn.2))
        .height(Pixels(28.0));
    DetectionModeButton::new(cx, params.clone(), |p| &p.detection_mode)
        .class("detection-mode-button")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(l.detection_btn.0))
        .top(Pixels(l.detection_btn.1))
        .width(Pixels(appearance::DETECTION_BUTTON_W))
        .height(Pixels(28.0));
    ImageSwitch::new(cx, params.clone(), |p| &p.harmonic_type)
        .class("image-switch")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(l.harmonic_type_switch.0))
        .top(Pixels(l.harmonic_type_switch.1))
        .width(Pixels(appearance::HARMONIC_BUTTON_W))
        .height(Pixels(38.0));
    let harmonic_size = (appearance::HARMONIC_KNOB_W, appearance::HARMONIC_KNOB_H);
    knob(
        cx,
        params.clone(),
        |p| &p.harmonic_drive,
        display.clone(),
        l.harmonic_drive,
        harmonic_size,
        StepSet::Percent0_100,
    );
    knob(
        cx,
        params.clone(),
        |p| &p.harmonic_mix,
        display.clone(),
        l.harmonic_mix,
        harmonic_size,
        StepSet::Percent0_100,
    );
    knob(
        cx,
        params.clone(),
        |p| &p.harmonic_even_boost,
        display.clone(),
        l.harmonic_even,
        harmonic_size,
        StepSet::Percent0_200,
    );
    knob(
        cx,
        params.clone(),
        |p| &p.harmonic_odd_boost,
        display.clone(),
        l.harmonic_odd,
        harmonic_size,
        StepSet::Percent0_200,
    );
    let standard_size = (theme::KNOB_SIZE, theme::KNOB_SIZE);
    knob(
        cx,
        params.clone(),
        |p| &p.makeup_gain_db,
        display.clone(),
        l.makeup_knob,
        standard_size,
        StepSet::None,
    );
    knob(
        cx,
        params.clone(),
        |p| &p.strength,
        display.clone(),
        l.strength_knob,
        (
            appearance::STRENGTH_KNOB_SIZE,
            appearance::STRENGTH_KNOB_SIZE,
        ),
        StepSet::None,
    );
    switch(cx, params.clone(), |p| &p.mid_side_mode, l.ms_switch);
    switch(cx, params.clone(), |p| &p.rms_normalization, l.norm_switch);
    knob(
        cx,
        params,
        |p| &p.rms_size_ms,
        display,
        l.rms_knob,
        standard_size,
        StepSet::RmsMs,
    );
}

fn knob<L, P, F>(
    cx: &mut Context,
    params: L,
    map: F,
    display: Arc<UiDisplay>,
    pos: (f32, f32),
    size: (f32, f32),
    steps: StepSet,
) where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    ImageKnob::new(cx, params, map, display, steps)
        .class("production-knob")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(pos.0))
        .top(Pixels(pos.1))
        .width(Pixels(size.0))
        .height(Pixels(size.1));
}

fn switch<L, P, F>(cx: &mut Context, params: L, map: F, pos: (f32, f32))
where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    ImageSwitch::new(cx, params, map)
        .class("image-switch")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(pos.0))
        .top(Pixels(pos.1))
        .width(Pixels(theme::SWITCH_SLOT_W))
        .height(Pixels(28.0));
}
