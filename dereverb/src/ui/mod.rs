mod curve;
mod events;
mod footer;
mod history;
mod layout;

use crate::{
    params::{DereverbParams, EngineKind, Quality},
    telemetry::{self, Telemetry, POINTS},
    weighting::{LevelMode, LOUDER, NODES},
};
use layout::*;
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{prelude::*, vg::FontId},
    widgets::util::ModifiersExt,
    ViziaTheming,
};
use pleasant_ui::{
    draw::{Draw, EditorViewport, READABLE_AXIS_SIZE, READABLE_LABEL_SIZE},
    graph::Viewport,
    load_font_once, param, point_in_rect,
    pointer::ValuePress,
    readout::{self, EditAction, PressAction},
    start_frame_timer,
    value_edit::ValueEdit,
    FONT_JETBRAINS_MONO, GOLD, LINE, MUTED, PANEL, TEAL, TEXT,
};
use std::{
    cell::Cell,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

pleasant_ui::appearance_store!("Dereverb");

struct DereverbView {
    params: Arc<DereverbParams>,
    shared: Arc<Telemetry>,
    font: Cell<Option<FontId>>,
    label_font: Cell<Option<FontId>>,
    hover: Option<(f32, f32)>,
    drag: Option<(f32, f32, f32)>,
    curve_drag: Option<(usize, f32, f32)>,
    guard_drag: Option<(f32, f32)>,
    press: Option<ValuePress<()>>,
    edit: Option<ValueEdit<()>>,
}

impl DereverbView {
    fn render(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let font = load_font_once(canvas, &self.font, FONT_JETBRAINS_MONO);
        let b = cx.bounds();
        let Some(mut d) =
            Draw::new_fitted(canvas, prefs().light(), (b.x, b.y, b.w, b.h), SIZE, font)
        else {
            return;
        };
        self.paint(&mut d);
    }

    fn paint(&self, d: &mut Draw) {
        let label_font = load_font_once(
            d.c,
            &self.label_font,
            nih_plug_vizia::assets::fonts::NOTO_SANS_REGULAR,
        );
        d.with_readable_style(label_font, |d| self.paint_content(d));
    }

    fn idle_hover(&self) -> Option<(f32, f32)> {
        self.hover.filter(|_| {
            self.drag.is_none()
                && self.curve_drag.is_none()
                && self.guard_drag.is_none()
                && self.press.is_none()
                && self.edit.is_none()
        })
    }

    fn paint_content(&self, d: &mut Draw) {
        d.hover = self.idle_hover();
        d.rect(0.0, 0.0, SIZE.0, 72.0, PANEL);
        d.text(36.0, 46.0, "DEREVERB", 24.0, GOLD);
        let engine = self.params.engine.value();
        let neural = engine == EngineKind::DpdfNet;
        for (rect, label, kind) in ENGINE_BUTTONS {
            d.button_tinted(rect, label, engine == kind, GOLD);
        }
        if engine.has_quality() {
            d.button_tinted(
                LIVE,
                if neural { "Live" } else { "10 ms" },
                self.params.quality.value() == Quality::Live,
                TEAL,
            );
            d.button_tinted(
                HIGH_QUALITY,
                if neural { "High quality" } else { "20 ms" },
                self.params.quality.value() == Quality::HighQuality,
                GOLD,
            );
        }
        d.appearance_button(THEME, prefs().label());
        self.paint_curve(d);
        let (input, output) = self.shared.snapshot();
        let input_power = recent_power(&input);
        let output_power = recent_power(&output);
        let history_hover = self.paint_history(d, &input, &output, input_power, output_power);
        let value = format!("{:.1} %", self.params.strength.value());
        d.knob_with_layout(
            &pleasant_ui::draw::KnobLayout::new(KNOB).with_text_sizes(READABLE_LABEL_SIZE, 13.0),
            "Strength",
            if self.edit.is_some() { "" } else { &value },
            self.params.strength.unmodulated_normalized_value(),
            GOLD,
            false,
        );
        if let Some(edit) = &self.edit {
            d.value_edit(edit, GOLD);
        } else if self
            .hover
            .is_some_and(|p| point_in_rect(p.0, p.1, value_rect()))
        {
            d.value_underline(
                value_rect(),
                pleasant_ui::draw::KnobLayout::new(KNOB).value_y,
                GOLD,
            );
        }
        let change = (input_power > 0.0 && output_power > 0.0)
            .then(|| 10.0 * (output_power / input_power).log10());
        let label = if change.is_some_and(|value| value > 0.05) {
            "Level increase"
        } else {
            "Level reduction"
        };
        d.with_label_font(|d| d.text(600.0, 478.0, label, READABLE_LABEL_SIZE, MUTED));
        let change = change.map_or_else(|| "—".into(), |value| format!("{:.1} dB", value.abs()));
        d.text(
            600.0,
            502.0,
            &change,
            18.0,
            if d.light { TEXT } else { TEAL },
        );

        let guard = &self.params.transient_guard;
        let amount = guard.unmodulated_normalized_value();
        let track_end = GUARD_TRACK_LEFT + GUARD_TRACK_WIDTH;
        let thumb = GUARD_TRACK_LEFT + GUARD_TRACK_WIDTH * amount;
        d.with_label_font(|d| {
            d.text(
                TRANSIENT_GUARD.0,
                529.0,
                "Transient guard",
                READABLE_LABEL_SIZE,
                MUTED,
            )
        });
        d.text_right(
            TRANSIENT_GUARD.0 + TRANSIENT_GUARD.2,
            529.0,
            &format!("{:.1} %", guard.unmodulated_plain_value()),
            12.0,
            if d.light { TEXT } else { TEAL },
        );
        d.line(GUARD_TRACK_LEFT, 544.0, track_end, 544.0, LINE, 3.0);
        if amount > 0.0 {
            d.line(GUARD_TRACK_LEFT, 544.0, thumb, 544.0, TEAL, 3.0);
        }
        d.circle(thumb, 544.0, 4.0, TEAL, true);
        self.paint_footer(d, engine, neural, history_hover);
    }
}

impl View for DereverbView {
    fn element(&self) -> Option<&'static str> {
        Some("dereverb-view")
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

pub fn create(params: Arc<DereverbParams>, shared: Arc<Telemetry>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            DereverbView {
                params: params.clone(),
                shared: shared.clone(),
                font: Cell::new(None),
                label_font: Cell::new(None),
                hover: None,
                drag: None,
                curve_drag: None,
                guard_drag: None,
                press: None,
                edit: None,
            }
            .build(cx, |cx| {
                start_frame_timer(cx, Duration::from_millis(33), |cx| cx.needs_redraw());
            })
            .width(Stretch(1.0))
            .height(Stretch(1.0));
        },
    )
}

#[cfg(all(test, target_os = "macos"))]
#[path = "../ui_preview.rs"]
mod preview;
