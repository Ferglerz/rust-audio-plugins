mod curve;
mod events;
mod footer;
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
    draw::{Draw, EditorViewport},
    graph::Viewport,
    load_font_once, param,
    pointer::ValuePress,
    point_in_rect,
    readout::{self, EditAction, PressAction},
    start_frame_timer,
    value_edit::ValueEdit,
    FONT_JETBRAINS_MONO, GOLD, LINE, MUTED, PANEL, TEAL,
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
        let Some(mut d) = Draw::new_fitted(canvas, prefs().light(), (b.x, b.y, b.w, b.h), SIZE, font)
        else {
            return;
        };
        self.paint(&mut d);
    }

    fn paint(&self, d: &mut Draw) {
        d.hover = if self.drag.is_none() && self.curve_drag.is_none() && self.edit.is_none() {
            self.hover
        } else {
            None
        };
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
                if neural { "LIVE" } else { "10 MS" },
                self.params.quality.value() == Quality::Live,
                TEAL,
            );
            d.button_tinted(
                HIGH_QUALITY,
                if neural { "HIGH QUALITY" } else { "20 MS" },
                self.params.quality.value() == Quality::HighQuality,
                GOLD,
            );
        }
        d.appearance_button(THEME, prefs().label());
        self.paint_curve(d);
        d.text(64.0, 414.0, "LEVEL HISTORY", 11.0, MUTED);
        let (input, output) = self.shared.snapshot();
        let input_power = recent_power(&input);
        let output_power = recent_power(&output);
        d.text_right(
            416.0,
            414.0,
            &format!("IN {}", history_level(input_power)),
            11.0,
            MUTED,
        );
        d.text_right(
            560.0,
            414.0,
            &format!("OUT {}", history_level(output_power)),
            11.0,
            TEAL,
        );
        let level_y = |db: f32| {
            GRAPH.1 + 2.0 + (GRAPH.3 - 4.0) * db.clamp(HISTORY_FLOOR, 0.0) / HISTORY_FLOOR
        };
        for db in [-90, -60, -30, 0] {
            let y = level_y(db as f32);
            d.line(GRAPH.0, y, GRAPH.0 + GRAPH.2, y, LINE, 1.0);
            d.text_right(GRAPH.0 - 10.0, y + 4.0, &db.to_string(), 10.0, MUTED);
        }
        d.scissor(GRAPH.0, GRAPH.1, GRAPH.2, GRAPH.3);
        for (history, color, width) in [(&input, MUTED, 3.4), (&output, TEAL, 1.5)] {
            let points: [(f32, f32); POINTS] = std::array::from_fn(|i| {
                (
                    GRAPH.0 + GRAPH.2 * i as f32 / (POINTS - 1) as f32,
                    level_y(20.0 * history[i].log10()),
                )
            });
            d.poly(&points, color, width);
        }
        d.reset_scissor();
        d.text(GRAPH.0, 538.0, "-1.6 s", 10.0, MUTED);
        d.text_right(GRAPH.0 + GRAPH.2, 538.0, "NOW", 10.0, MUTED);
        let value = format!("{:.1} %", self.params.strength.value());
        d.knob(
            KNOB,
            "STRENGTH",
            if self.edit.is_some() { "" } else { &value },
            self.params.strength.unmodulated_normalized_value(),
            GOLD,
            false,
        );
        if let Some(edit) = &self.edit {
            d.value_edit(edit, GOLD);
        } else if self.hover.is_some_and(|p| point_in_rect(p.0, p.1, value_rect())) {
            d.value_underline(value_rect(), pleasant_ui::draw::KnobLayout::new(KNOB).value_y, GOLD);
        }
        let change = (input_power > 0.0).then(|| 10.0 * (output_power / input_power).log10());
        let change = change.unwrap_or(0.0);
        let label = if change > 0.05 {
            "LEVEL INCREASE"
        } else {
            "LEVEL REDUCTION"
        };
        d.text(600.0, 478.0, label, 10.0, MUTED);
        d.text(600.0, 502.0, &format!("{:.1} dB", change.abs()), 18.0, TEAL);

        let guard = &self.params.transient_guard;
        let amount = guard.unmodulated_normalized_value();
        let track_end = GUARD_TRACK_LEFT + GUARD_TRACK_WIDTH;
        let thumb = GUARD_TRACK_LEFT + GUARD_TRACK_WIDTH * amount;
        d.text(TRANSIENT_GUARD.0, 529.0, "TRANSIENT GUARD", 10.0, MUTED);
        d.text_right(
            TRANSIENT_GUARD.0 + TRANSIENT_GUARD.2,
            529.0,
            &format!("{:.1} %", guard.unmodulated_plain_value()),
            12.0,
            TEAL,
        );
        d.line(GUARD_TRACK_LEFT, 544.0, track_end, 544.0, LINE, 3.0);
        if amount > 0.0 {
            d.line(GUARD_TRACK_LEFT, 544.0, thumb, 544.0, TEAL, 3.0);
        }
        d.circle(thumb, 544.0, 4.0, TEAL, true);
        self.paint_footer(d, engine, neural);
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
