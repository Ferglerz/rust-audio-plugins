//! Production controls at fixed JSFX positions (no scroll, no page nav).

use std::sync::Arc;

use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;

use crate::params::ComposureParams;

use super::display::UiDisplay;
use super::image_knob::ImageKnob;
use super::image_switch::{DetectionModeButton, HarmonicTypeSwitch, ImageSwitch, ProgModeSwitch};
use super::layout::ControlLayout;
use super::parallax_slider::ParallaxSlider;
use super::readout_controls::{JsfxButtonLabel, JsfxParamButton, ReadoutSlider, SliderFill};
use super::step_points::StepSet;
use super::theme;

pub fn build_positioned_controls<L>(cx: &mut Context, params: L, display: Arc<UiDisplay>)
where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
{
    let analog = ControlLayout::from_jsfx();
    let pleasant = ControlLayout::pleasant();

    placed_knob(
        cx,
        params.clone(),
        |p| &p.attack,
        display.clone(),
        analog.attack_knob,
        pleasant.attack_knob,
        StepSet::Attack,
    );
    placed_knob(
        cx,
        params.clone(),
        |p| &p.release,
        display.clone(),
        analog.release_knob,
        pleasant.release_knob,
        StepSet::Release,
    );
    placed_knob(
        cx,
        params.clone(),
        |p| &p.hold_ms,
        display.clone(),
        analog.hold_knob,
        pleasant.hold_knob,
        StepSet::None,
    );

    placed_slider(
        cx,
        params.clone(),
        |p| &p.hp_freq,
        display.clone(),
        analog.hp_slider,
        pleasant.hp_slider,
        SliderFill::LeftToRight,
        true,
        StepSet::HpFreq,
        "High Pass",
    );
    placed_jsfx_button(
        cx,
        params.clone(),
        |p| &p.sc_adjust_preview,
        JsfxButtonLabel::Listen,
        analog.listen_btn,
        pleasant.listen_btn,
    );
    placed_jsfx_button(
        cx,
        params.clone(),
        |p| &p.use_sidechain,
        JsfxButtonLabel::Sidechain,
        analog.sc_btn,
        pleasant.sc_btn,
    );
    placed_slider(
        cx,
        params.clone(),
        |p| &p.lp_freq,
        display.clone(),
        analog.lp_slider,
        pleasant.lp_slider,
        SliderFill::RightToLeft,
        true,
        StepSet::LpFreq,
        "Low Pass",
    );

    placed_slider(
        cx,
        params.clone(),
        |p| &p.lookahead_ms,
        display.clone(),
        analog.lookahead_slider,
        pleasant.lookahead_slider,
        SliderFill::LeftToRight,
        false,
        StepSet::LookaheadMs,
        "Lookahead",
    );
    placed_jsfx_button(
        cx,
        params.clone(),
        |p| &p.brickwall_limiter,
        JsfxButtonLabel::Brickwall,
        analog.brickwall_btn,
        pleasant.brickwall_btn,
    );
    placed_slider(
        cx,
        params.clone(),
        |p| &p.prog_release_blend,
        display.clone(),
        analog.prog_blend_slider,
        pleasant.prog_blend_slider,
        SliderFill::LeftToRight,
        false,
        StepSet::Percent0_100,
        "Program Release Blend",
    );
    placed_jsfx_button(
        cx,
        params.clone(),
        |p| &p.prog_release_inverse,
        JsfxButtonLabel::Inverse,
        analog.inverse_btn,
        pleasant.inverse_btn,
    );
    placed_slider(
        cx,
        params.clone(),
        |p| &p.attack_curve,
        display.clone(),
        analog.attack_curve_slider,
        pleasant.attack_curve_slider,
        SliderFill::CenterOut,
        false,
        StepSet::None,
        "Attack Curve",
    );
    placed_slider(
        cx,
        params.clone(),
        |p| &p.release_curve,
        display.clone(),
        analog.release_curve_slider,
        pleasant.release_curve_slider,
        SliderFill::CenterOut,
        false,
        StepSet::None,
        "Release Curve",
    );
    placed_detection(
        cx,
        params.clone(),
        |p| &p.detection_mode,
        analog.detection_btn,
        pleasant.detection_btn,
    );

    placed_harmonic_switch(
        cx,
        params.clone(),
        |p| &p.harmonic_type,
        analog.harmonic_type_switch,
        pleasant.harmonic_type_switch,
    );
    placed_parallax(
        cx,
        params.clone(),
        |p| &p.harmonic_drive,
        display.clone(),
        analog.harmonic_drive,
        pleasant.harmonic_drive,
        StepSet::Percent0_100,
        "Drive",
    );
    placed_parallax(
        cx,
        params.clone(),
        |p| &p.harmonic_mix,
        display.clone(),
        analog.harmonic_mix,
        pleasant.harmonic_mix,
        StepSet::Percent0_100,
        "Mix",
    );
    placed_parallax(
        cx,
        params.clone(),
        |p| &p.harmonic_even_boost,
        display.clone(),
        analog.harmonic_even,
        pleasant.harmonic_even,
        StepSet::Percent0_200,
        "Even",
    );
    placed_parallax(
        cx,
        params.clone(),
        |p| &p.harmonic_odd_boost,
        display.clone(),
        analog.harmonic_odd,
        pleasant.harmonic_odd,
        StepSet::Percent0_200,
        "Odd",
    );
    placed_knob(
        cx,
        params.clone(),
        |p| &p.makeup_gain_db,
        display.clone(),
        analog.makeup_knob,
        pleasant.makeup_knob,
        StepSet::None,
    );

    placed_knob(
        cx,
        params.clone(),
        |p| &p.strength,
        display.clone(),
        analog.strength_knob,
        pleasant.strength_knob,
        StepSet::None,
    );
    placed_prog_mode(
        cx,
        params.clone(),
        |p| &p.prog_release_mode,
        analog.prog_mode_switch,
        pleasant.prog_mode_switch,
    );
    placed_bool_switch(
        cx,
        params.clone(),
        |p| &p.mid_side_mode,
        analog.ms_switch,
        pleasant.ms_switch,
    );
    placed_bool_switch(
        cx,
        params.clone(),
        |p| &p.rms_normalization,
        analog.norm_switch,
        pleasant.norm_switch,
    );
    placed_knob(
        cx,
        params.clone(),
        |p| &p.rms_size_ms,
        display.clone(),
        analog.rms_knob,
        pleasant.rms_knob,
        StepSet::None,
    );
    placed_knob(
        cx,
        params,
        |p| &p.input_offset_db,
        display,
        analog.offset_knob,
        pleasant.offset_knob,
        StepSet::None,
    );
}

fn xy(analog: f32, pleasant: f32) -> impl Lens<Target = Units> {
    super::EditorData::appearance.map(move |mode| {
        Pixels(if *mode == 2 { analog } else { pleasant })
    })
}

fn placed_knob<L, P, F>(
    cx: &mut Context,
    params: L,
    map: F,
    display: Arc<UiDisplay>,
    analog: (f32, f32),
    pleasant: (f32, f32),
    step_set: StepSet,
) where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    ImageKnob::new(cx, params, map, display, step_set)
        .class("production-knob")
        .position_type(PositionType::SelfDirected)
        .left(xy(analog.0, pleasant.0))
        .top(xy(analog.1, pleasant.1))
        .width(Pixels(theme::KNOB_SIZE))
        .height(Pixels(theme::KNOB_SIZE));
}

/// `PARALLAX_LABEL_Y_OFFSET` in FerglerUI rendering constants.
const PARALLAX_LABEL_Y_OFFSET: f32 = 4.0;

fn placed_parallax<L, P, F>(
    cx: &mut Context,
    params: L,
    map: F,
    display: Arc<UiDisplay>,
    analog: (f32, f32),
    pleasant: (f32, f32),
    step_set: StepSet,
    label: &'static str,
) where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    Label::new(cx, label)
        .class("parallax-label")
        .position_type(PositionType::SelfDirected)
        .left(xy(analog.0, pleasant.0))
        .top(super::EditorData::appearance.map(move |mode| {
            Pixels(if *mode == 2 {
                analog.1 + PARALLAX_LABEL_Y_OFFSET
            } else {
                pleasant.1 - 14.0
            })
        }))
        .width(Pixels(theme::PARALLAX_SLIDER_W))
        .height(Pixels(14.0));

    ParallaxSlider::new(cx, params, map, display, step_set)
        .class("parallax-slider")
        .position_type(PositionType::SelfDirected)
        .left(xy(analog.0, pleasant.0))
        .top(xy(analog.1, pleasant.1))
        .width(Pixels(theme::PARALLAX_SLIDER_W))
        .height(Pixels(theme::PARALLAX_SLIDER_H));
}

fn placed_slider<L, P, F>(
    cx: &mut Context,
    params: L,
    map: F,
    display: Arc<UiDisplay>,
    analog: (f32, f32, f32),
    pleasant: (f32, f32, f32),
    fill: SliderFill,
    filter_preview: bool,
    step_set: StepSet,
    label: &'static str,
) where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    Label::new(cx, label)
        .class("readout-slider-label")
        .position_type(PositionType::SelfDirected)
        .left(xy(analog.0, pleasant.0))
        .top(super::EditorData::appearance.map(move |mode| {
            let (y, h) = if *mode == 2 {
                (analog.1, theme::READOUT_SLIDER_LABEL_H)
            } else {
                (pleasant.1, 14.0)
            };
            Pixels(y - h)
        }))
        .width(xy(analog.2, pleasant.2))
        .height(Pixels(14.0));

    ReadoutSlider::new(cx, params, map, display, fill, filter_preview, step_set)
        .class("production-slider")
        .position_type(PositionType::SelfDirected)
        .left(xy(analog.0, pleasant.0))
        .top(xy(analog.1, pleasant.1))
        .width(xy(analog.2, pleasant.2))
        .height(Pixels(theme::PARALLAX_SLOT_H));
}

fn placed_jsfx_button<L, P, F>(
    cx: &mut Context,
    params: L,
    map: F,
    label: JsfxButtonLabel,
    analog: (f32, f32, f32),
    pleasant: (f32, f32, f32),
) where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    JsfxParamButton::new(cx, params, map, label)
        .class("jsfx-button")
        .position_type(PositionType::SelfDirected)
        .left(xy(analog.0, pleasant.0))
        .top(xy(analog.1, pleasant.1))
        .width(xy(analog.2, pleasant.2))
        .height(Pixels(theme::BUTTON_H));
}

fn placed_image_switch<'v, W>(
    widget: Handle<'v, W>,
    analog: (f32, f32),
    pleasant: (f32, f32),
) -> Handle<'v, W>
where
    W: View,
{
    let x_off = theme::align_in_slot(theme::SWITCH_SLOT_W, theme::SWITCH_W);

    widget
        .class("image-switch")
        .position_type(PositionType::SelfDirected)
        .left(super::EditorData::appearance.map(move |mode| {
            Pixels(if *mode == 2 {
                analog.0 + x_off - theme::SWITCH_LEFT_OVERHANG
            } else {
                pleasant.0
            })
        }))
        .top(xy(analog.1, pleasant.1))
        .width(super::EditorData::appearance.map(|mode| {
            Pixels(if *mode == 2 {
                theme::SWITCH_WIDGET_W
            } else {
                theme::SWITCH_SLOT_W
            })
        }))
        .height(super::EditorData::appearance.map(|mode| {
            Pixels(if *mode == 2 {
                theme::SWITCH_H
            } else {
                56.0
            })
        }))
}

fn placed_bool_switch<L, P, F>(
    cx: &mut Context,
    params: L,
    map: F,
    analog: (f32, f32),
    pleasant: (f32, f32),
) where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    placed_image_switch(ImageSwitch::new(cx, params, map), analog, pleasant);
}

fn placed_harmonic_switch<L, P, F>(
    cx: &mut Context,
    params: L,
    map: F,
    analog: (f32, f32),
    pleasant: (f32, f32),
) where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    placed_image_switch(HarmonicTypeSwitch::new(cx, params, map), analog, pleasant);
}

fn placed_prog_mode<L, P, F>(
    cx: &mut Context,
    params: L,
    map: F,
    analog: (f32, f32),
    pleasant: (f32, f32),
) where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    placed_image_switch(ProgModeSwitch::new(cx, params, map), analog, pleasant);
}

fn placed_detection<L, P, F>(
    cx: &mut Context,
    params: L,
    map: F,
    analog: (f32, f32),
    pleasant: (f32, f32),
) where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    DetectionModeButton::new(cx, params, map)
        .class("detection-mode-button")
        .position_type(PositionType::SelfDirected)
        .left(xy(analog.0, pleasant.0))
        .top(xy(analog.1, pleasant.1))
        .width(Pixels(100.0))
        .height(Pixels(75.0));
}
