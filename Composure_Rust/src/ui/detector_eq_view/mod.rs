//! Detector equalizer, using the same response model and node artwork as Damian.
use std::{
    cell::RefCell,
    sync::{atomic::Ordering, Arc},
};

use nih_plug::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::util::ModifiersExt;
use pleasant_eq::{BandCoefficients, BandSettings, EqShape, ResponseCache};
use pleasant_ui::theme::EQ_COLORS as COLORS;
use pleasant_ui::{
    pointer::ValuePress, point_in_rect, ButtonAnim, Draw, ValueEdit, GOLD, LINE, MUTED, PANEL,
    TEXT,
};

use super::display::UiDisplay;
use crate::{detector_eq::DetectorShape, params::ComposureParams};

mod axis;
mod draw;
mod events;
mod model;
#[cfg(test)]
mod tests;

use axis::*;
use model::*;

pub struct DetectorEqView {
    params: Arc<ComposureParams>,
    display: Arc<UiDisplay>,
    selected: Option<Node>,
    drag: Option<NodeDrag>,
    press: Option<ReadoutPress>,
    edit: Option<ValueEdit<Target>>,
    hover: Option<(f32, f32)>,
    shape_menu: bool,
    bypass_anim: ButtonAnim,
    response_cache: RefCell<ResponseCache>,
}

impl DetectorEqView {
    pub fn new(
        cx: &mut Context,
        params: Arc<ComposureParams>,
        display: Arc<UiDisplay>,
    ) -> Handle<'_, Self> {
        Self {
            params,
            display,
            selected: None,
            drag: None,
            press: None,
            edit: None,
            hover: None,
            shape_menu: false,
            bypass_anim: ButtonAnim::new(),
            response_cache: RefCell::new(ResponseCache::default()),
        }
        .build(cx, |cx| {
            // Reuse this timer: the pinned Vizia can spin when starting a second
            // timer during construction. Auto's system reads are cached for a second.
            pleasant_ui::start_frame_timer(cx, pleasant_ui::FRAME_INTERVAL, |cx| {
                cx.emit(super::appearance::RefreshAppearance);
                cx.needs_redraw();
            });
        })
        .focusable(true)
    }

    fn close_menu(&mut self, cx: &mut EventContext) {
        if self.shape_menu {
            self.shape_menu = false;
            cx.release();
            cx.needs_redraw();
        }
    }

    fn point(cx: &EventContext) -> (f32, f32) {
        let b = cx.bounds();
        pleasant_ui::local_xy(b.x, b.y, b.w, WIDTH, cx.mouse().cursorx, cx.mouse().cursory)
            .unwrap_or((-1.0, -1.0))
    }

    fn begin_node(&mut self, cx: &mut EventContext, node: Node, p: (f32, f32)) {
        self.begin_node_with_settings(cx, node, p, self.settings(node));
    }

    fn begin_node_with_settings(
        &mut self,
        cx: &mut EventContext,
        node: Node,
        p: (f32, f32),
        b: BandSettings,
    ) {
        let fields = node_fields(node, b.shape);
        for &field in &fields {
            pleasant_ui::param::begin(cx, self.param(Target { node, field }).as_ptr());
        }
        self.drag = Some(NodeDrag {
            node,
            fields,
            origin: p,
            frequency: b.frequency_hz as f32,
            gain: b.gain_db as f32,
            q: b.q as f32,
        });
        self.display.set_filter_preview_drag(true);
        cx.capture();
        cx.focus();
    }

    fn finish_gesture(&mut self, cx: &mut EventContext) {
        let captured = self.drag.is_some() || self.press.is_some();
        if let Some(drag) = self.drag.take() {
            for field in drag.fields {
                pleasant_ui::param::end(
                    cx,
                    self.param(Target {
                        node: drag.node,
                        field,
                    })
                    .as_ptr(),
                );
            }
        }
        if let Some(press) = self.press.take() {
            if press.active {
                pleasant_ui::param::end(cx, self.param(press.press.target).as_ptr());
            }
        }
        if captured {
            self.display.set_filter_preview_drag(false);
            cx.release();
        }
    }

    fn remove(&mut self, cx: &mut EventContext, node: Node) {
        match node {
            Node::Band(i) => set_once(cx, &self.params.detector_eq[i].active, 0.0),
            _ => {
                let p = self.param(Target {
                    node,
                    field: Field::Frequency,
                });
                set_once(cx, p, p.preview_normalized(0.0));
            }
        }
        self.selected = None;
        self.edit = None;
    }

    fn create(&mut self, cx: &mut EventContext, p: (f32, f32)) {
        if let Some(i) = self
            .params
            .detector_eq
            .iter()
            .position(|band| !band.active.value())
        {
            let b = &self.params.detector_eq[i];
            set_once(cx, &b.freq, b.freq.preview_normalized(x_frequency(p.0)));
            set_once(cx, &b.gain, b.gain.preview_normalized(y_gain(p.1)));
            set_once(cx, &b.q, b.q.preview_normalized(1.0));
            set_once(
                cx,
                &b.shape,
                b.shape.preview_normalized(DetectorShape::Bell),
            );
            set_once(cx, &b.enabled, 1.0);
            set_once(cx, &b.active, 1.0);
            self.selected = Some(Node::Band(i));
            // Parameter events are queued. Seed the drag from the values just
            // submitted, not the inactive band's previous parameter values.
            self.begin_node_with_settings(
                cx,
                Node::Band(i),
                p,
                BandSettings {
                    shape: EqShape::Bell,
                    frequency_hz: x_frequency(p.0) as f64,
                    gain_db: y_gain(p.1) as f64,
                    q: 1.0,
                    ..BandSettings::default()
                },
            );
        }
    }

    fn commit_edit(&mut self, cx: &mut EventContext) -> bool {
        if let Some(edit) = self.edit.as_ref() {
            let param = self.param(edit.target);
            let value = param.string_to_normalized_value(&edit.text).or_else(|| {
                pleasant_ui::value_edit::parse_number_with_units(
                    &edit.text,
                    &[("khz", 1000.0), ("hz", 1.0), ("k", 1000.0), ("db", 1.0)],
                )
                .filter(|v| v.is_finite())
                .map(|v| param.preview_normalized(v as f32))
            });
            if let Some(value) = value {
                set_once(cx, param, value);
                self.edit = None;
            } else {
                self.edit.as_mut().unwrap().invalid = true;
                return false;
            }
        }
        true
    }

    fn scroll(&mut self, cx: &mut EventContext, target: Target, delta: f32) {
        self.edit = None;
        let param = self.param(target);
        let step = if cx.modifiers().shift() { 0.001 } else { 0.01 };
        set_once(
            cx,
            param,
            param.unmodulated_normalized_value() + delta * step,
        );
    }
}

impl View for DetectorEqView {
    fn element(&self) -> Option<&'static str> {
        Some("detector-eq")
    }
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        self.draw_view(cx, canvas);
    }
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        self.handle_event(cx, event);
    }
}
