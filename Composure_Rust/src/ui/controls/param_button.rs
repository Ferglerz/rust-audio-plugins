use super::param_widget_ext;
use crate::params::ComposureParams;
use nih_plug::prelude::Param;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use std::sync::Arc;

#[allow(dead_code)]
pub struct ParamButton {
    param_base: ParamWidgetBase,
}
#[allow(dead_code)]
impl ParamButton {
    pub fn new<L, P, F>(cx: &mut Context, params: L, map: F) -> Handle<'_, Self>
    where
        L: Lens<Target = Arc<ComposureParams>> + Clone + 'static,
        P: Param + 'static,
        F: Fn(&Arc<ComposureParams>) -> &P + Copy + 'static,
    {
        Self {
            param_base: ParamWidgetBase::new(cx, params, map),
        }
        .build(cx, |_| {})
    }
}
impl View for ParamButton {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        super::appearance::draw_control(
            cx,
            canvas,
            &self.param_base,
            super::appearance::Control::Button,
            self.param_base.modulated_normalized_value(),
        );
    }
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        if super::appearance::program_control_inactive(
            &super::EditorData::params.get(cx),
            self.param_base.name(),
        ) {
            return;
        }
        event.map(|e, meta| match e {
            WindowEvent::MouseEnter | WindowEvent::MouseLeave | WindowEvent::MouseMove(..) => {
                cx.needs_redraw();
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                param_widget_ext::toggle_bool_param(cx, &self.param_base);
                cx.needs_redraw();
                meta.consume();
            }
            _ => {}
        });
    }
}
