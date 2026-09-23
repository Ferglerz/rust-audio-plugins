//! Production controls at fixed JSFX positions (no scroll, no page nav).

use std::sync::Arc;

use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;

use crate::params::ComposureParams;

use super::display::UiDisplay;
use super::image_knob::ImageKnob;
use super::image_switch::{DetectionModeButton, HarmonicTypeSwitch, ImageSwitch, ProgModeSwitch};
use super::parallax_slider::ParallaxSlider;
use super::layout::ControlLayout;
use super::readout_controls::{JsfxButtonLabel, JsfxParamButton, ReadoutSlider, SliderFill};
use super::step_points::StepSet;
use super::theme;

pub fn build_positioned_controls<L>(cx: &mut Context, params: L, display: Arc<UiDisplay>)
where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
{
    let l = ControlLayout::from_jsfx();

    placed_knob(
        cx,
        params.clone(),
        |p| &p.attack,
        display.clone(),
        l.attack_knob.0,
        l.attack_knob.1,
        StepSet::Attack,
    );
    placed_knob(
        cx,
        params.clone(),
        |p| &p.release,
        display.clone(),
        l.release_knob.0,
        l.release_knob.1,
        StepSet::Release,
    );
    placed_knob(
        cx,
        params.clone(),
        |p| &p.hold_ms,
        display.clone(),
        l.hold_knob.0,
        l.hold_knob.1,
        StepSet::None,
    );

    placed_slider(
        cx,
        params.clone(),
        |p| &p.hp_freq,
        display.clone(),
        l.hp_slider.0,
        l.hp_slider.1,
        l.hp_slider.2,
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
        l.listen_btn.0,
        l.listen_btn.1,
        l.listen_btn.2,
    );
    placed_jsfx_button(
        cx,
        params.clone(),
        |p| &p.use_sidechain,
        JsfxButtonLabel::Sidechain,
        l.sc_btn.0,
        l.sc_btn.1,
        l.sc_btn.2,
    );
    placed_slider(
        cx,
        params.clone(),
        |p| &p.lp_freq,
        display.clone(),
        l.lp_slider.0,
        l.lp_slider.1,
        l.lp_slider.2,
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
        l.lookahead_slider.0,
        l.lookahead_slider.1,
        l.lookahead_slider.2,
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
        l.brickwall_btn.0,
        l.brickwall_btn.1,
        l.brickwall_btn.2,
    );
    placed_slider(
        cx,
        params.clone(),
        |p| &p.prog_release_blend,
        display.clone(),
        l.prog_blend_slider.0,
        l.prog_blend_slider.1,
        l.prog_blend_slider.2,
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
        l.inverse_btn.0,
        l.inverse_btn.1,
        l.inverse_btn.2,
    );
    placed_slider(
        cx,
        params.clone(),
        |p| &p.attack_curve,
        display.clone(),
        l.attack_curve_slider.0,
        l.attack_curve_slider.1,
        l.attack_curve_slider.2,
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
        l.release_curve_slider.0,
        l.release_curve_slider.1,
        l.release_curve_slider.2,
        SliderFill::CenterOut,
        false,
        StepSet::None,
        "Release Curve",
    );
    placed_detection(
        cx,
        params.clone(),
        |p| &p.detection_mode,
        l.detection_btn.0,
        l.detection_btn.1,
    );

    placed_harmonic_switch(
        cx,
        params.clone(),
        |p| &p.harmonic_type,
        l.harmonic_type_switch.0,
        l.harmonic_type_switch.1,
    );
    placed_parallax(
        cx,
        params.clone(),
        |p| &p.harmonic_drive,
        display.clone(),
        l.harmonic_drive.0,
        l.harmonic_drive.1,
        StepSet::Percent0_100,
        "Drive",
    );
    placed_parallax(
        cx,
        params.clone(),
        |p| &p.harmonic_mix,
        display.clone(),
        l.harmonic_mix.0,
        l.harmonic_mix.1,
        StepSet::Percent0_100,
        "Mix",
    );
    placed_parallax(
        cx,
        params.clone(),
        |p| &p.harmonic_even_boost,
        display.clone(),
        l.harmonic_even.0,
        l.harmonic_even.1,
        StepSet::Percent0_200,
        "Even",
    );
    placed_parallax(
        cx,
        params.clone(),
        |p| &p.harmonic_odd_boost,
        display.clone(),
        l.harmonic_odd.0,
        l.harmonic_odd.1,
        StepSet::Percent0_200,
        "Odd",
    );
    placed_knob(
        cx,
        params.clone(),
        |p| &p.makeup_gain_db,
        display.clone(),
        l.makeup_knob.0,
        l.makeup_knob.1,
        StepSet::None,
    );

    placed_knob(
        cx,
        params.clone(),
        |p| &p.strength,
        display.clone(),
        l.strength_knob.0,
        l.strength_knob.1,
        StepSet::None,
    );
    placed_prog_mode(
        cx,
        params.clone(),
        |p| &p.prog_release_mode,
        l.prog_mode_switch.0,
        l.prog_mode_switch.1,
    );
    placed_bool_switch(cx, params.clone(), |p| &p.mid_side_mode, l.ms_switch.0, l.ms_switch.1);
    placed_bool_switch(
        cx,
        params.clone(),
        |p| &p.rms_normalization,
        l.norm_switch.0,
        l.norm_switch.1,
    );
    placed_knob(
        cx,
        params.clone(),
        |p| &p.rms_size_ms,
        display.clone(),
        l.rms_knob.0,
        l.rms_knob.1,
        StepSet::None,
    );
    placed_knob(
        cx,
        params,
        |p| &p.input_offset_db,
        display,
        l.offset_knob.0,
        l.offset_knob.1,
        StepSet::None,
    );
}

fn placed_knob<L, P, F>(
    cx: &mut Context,
    params: L,
    map: F,
    display: Arc<UiDisplay>,
    x: f32,
    y: f32,
    step_set: StepSet,
)
where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    ImageKnob::new(cx, params, map, display, step_set)
        .class("production-knob")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(x))
        .top(Pixels(y))
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
    x: f32,
    y: f32,
    step_set: StepSet,
    label: &'static str,
)
where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    Label::new(cx, label)
        .class("parallax-label")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(x))
        .top(Pixels(y + PARALLAX_LABEL_Y_OFFSET))
        .width(Pixels(theme::PARALLAX_SLIDER_W))
        .height(Pixels(11.0));

    ParallaxSlider::new(cx, params, map, display, step_set)
        .class("parallax-slider")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(x))
        .top(Pixels(y))
        .width(Pixels(theme::PARALLAX_SLIDER_W))
        .height(Pixels(theme::PARALLAX_SLIDER_H));
}

fn placed_slider<L, P, F>(
    cx: &mut Context,
    params: L,
    map: F,
    display: Arc<UiDisplay>,
    x: f32,
    y: f32,
    w: f32,
    fill: SliderFill,
    filter_preview: bool,
    step_set: StepSet,
    label: &'static str,
)
where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    Label::new(cx, label)
        .class("readout-slider-label")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(x))
        .top(Pixels(y - theme::READOUT_SLIDER_LABEL_H))
        .width(Pixels(w))
        .height(Pixels(theme::READOUT_SLIDER_LABEL_H));

    ReadoutSlider::new(cx, params, map, display, fill, filter_preview, step_set)
        .class("production-slider")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(x))
        .top(Pixels(y))
        .width(Pixels(w))
        .height(Pixels(theme::PARALLAX_SLOT_H));
}

fn placed_jsfx_button<L, P, F>(
    cx: &mut Context,
    params: L,
    map: F,
    label: JsfxButtonLabel,
    x: f32,
    y: f32,
    w: f32,
)
where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    JsfxParamButton::new(cx, params, map, label)
        .class("jsfx-button")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(x))
        .top(Pixels(y))
        .width(Pixels(w))
        .height(Pixels(theme::BUTTON_H));
}

fn placed_image_switch<'v, W>(widget: Handle<'v, W>, x: f32, y: f32) -> Handle<'v, W>
where
    W: View,
{
    let x_off = theme::align_in_slot(theme::SWITCH_SLOT_W, theme::SWITCH_W);

    widget
        .class("image-switch")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(x + x_off - theme::SWITCH_LEFT_OVERHANG))
        .top(Pixels(y))
        .width(Pixels(theme::SWITCH_WIDGET_W))
        .height(Pixels(theme::SWITCH_H))
}

fn placed_bool_switch<L, P, F>(cx: &mut Context, params: L, map: F, x: f32, y: f32)
where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    placed_image_switch(ImageSwitch::new(cx, params, map), x, y);
}

fn placed_harmonic_switch<L, P, F>(cx: &mut Context, params: L, map: F, x: f32, y: f32)
where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    placed_image_switch(HarmonicTypeSwitch::new(cx, params, map), x, y);
}

fn placed_prog_mode<L, P, F>(cx: &mut Context, params: L, map: F, x: f32, y: f32)
where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    placed_image_switch(ProgModeSwitch::new(cx, params, map), x, y);
}

fn placed_detection<L, P, F>(cx: &mut Context, params: L, map: F, x: f32, y: f32)
where
    L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
    P: Param + 'static,
    F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
{
    DetectionModeButton::new(cx, params, map)
        .class("detection-mode-button")
        .position_type(PositionType::SelfDirected)
        .left(Pixels(x))
        .top(Pixels(y))
        .width(Pixels(100.0))
        .height(Pixels(75.0));
}
