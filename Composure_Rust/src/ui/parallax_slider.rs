use std::sync::Arc;

use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::widgets::util;

use crate::params::ComposureParams;

use super::ui_assets as assets;
use super::display::UiDisplay;
use super::param_widget_ext::{self, ParamDragSession};
use super::step_points::StepSet;
use super::texture_cache::draw_tex;

const FADER_IMG_W: f32 = 36.0;
const FADER_IMG_H: f32 = 45.0;
const FADER_BG_W: f32 = 107.0;
const FADER_BG_H: f32 = 11.0;

// ── Parallax fader (harmonics) ───────────────────────────────────────────────

pub struct ParallaxSlider {
    param_base: ParamWidgetBase,
    display: Arc<UiDisplay>,
    drag: ParamDragSession,
    depth: f32,
    y_offset: f32,
    x_offset: f32,
    top_adjust: f32,
    step_set: StepSet,
}

impl ParallaxSlider {
    pub fn new<L, P, F>(
        cx: &mut Context,
        params: L,
        map: F,
        display: Arc<UiDisplay>,
        step_set: StepSet,
    ) -> Handle<'_, ParallaxSlider>
    where
        L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
        P: Param + 'static,
        F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
    {
        ParallaxSlider {
            param_base: ParamWidgetBase::new(cx, params, map),
            display,
            drag: ParamDragSession::INACTIVE,
            depth: 4.0,
            y_offset: 11.0,
            x_offset: 4.0,
            top_adjust: -2.0,
            step_set,
        }
        .build(cx, |_| {})
    }
}

impl View for ParallaxSlider {
    fn element(&self) -> Option<&'static str> {
        Some("parallax-slider")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let opacity = cx.opacity();
        let norm = param_widget_ext::ui_normalized(&self.param_base, self.step_set);
        let scaled_w = FADER_IMG_W;
        let scaled_h = FADER_IMG_H;

        let bg_x = bounds.x + (bounds.w - FADER_BG_W) * 0.5;
        let bg_y = bounds.y + (bounds.h - FADER_BG_H) * 0.5;
        draw_tex(
            canvas,
            "fader_bg",
            assets::FADER_BG,
            bg_x,
            bg_y,
            FADER_BG_W,
            FADER_BG_H,
            opacity,
        );

        let fader_x = norm * (bounds.w - scaled_w);
        let layer_y = bounds.y + (bounds.h - scaled_h) * 0.5 + self.y_offset;
        let bot_x = (bounds.x + fader_x + self.x_offset).ceil();
        let parallax_offset = norm * self.depth;
        let top_x = bot_x + parallax_offset.ceil() + self.top_adjust;

        draw_tex(
            canvas,
            "fader_bot",
            assets::FADER_BOT_LEFT,
            bot_x,
            layer_y,
            scaled_w,
            scaled_h,
            opacity,
        );
        draw_tex(
            canvas,
            "fader_top",
            assets::FADER_TOP,
            top_x,
            layer_y,
            scaled_w,
            scaled_h,
            opacity,
        );
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                if param_widget_ext::reset_to_default_on_command_click(cx, &self.param_base) {
                } else {
                    let ui_norm =
                        util::remap_current_entity_x_coordinate(cx, cx.mouse().cursorx);
                    param_widget_ext::param_drag_begin_horizontal(
                        &mut self.drag,
                        cx,
                        &self.param_base,
                        &self.display,
                        self.step_set,
                        ui_norm,
                        false,
                    );
                }
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if param_widget_ext::param_drag_end(
                    &mut self.drag,
                    cx,
                    &self.param_base,
                    &self.display,
                    self.step_set,
                    false,
                ) {
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(_, _) => {
                if self.drag.active {
                    let ui_norm =
                        util::remap_current_entity_x_coordinate(cx, cx.mouse().cursorx);
                    param_widget_ext::param_drag_move_horizontal(
                        cx,
                        &self.param_base,
                        &self.display,
                        self.step_set,
                        ui_norm,
                    );
                    meta.consume();
                }
            }
            _ => {}
        });
    }
}
