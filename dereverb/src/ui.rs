use crate::{
    params::{DereverbParams, EngineKind, Quality},
    telemetry::{self, Telemetry, POINTS},
    weighting::{LevelMode, LOUDER, NODES},
};
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{prelude::*, vg::FontId},
    widgets::util::ModifiersExt,
    ViziaTheming,
};
use pleasant_ui::{
    draw::{Draw, EditorViewport, KnobLayout},
    graph::Viewport,
    param,
    pointer::ValuePress,
    preferences::AppearanceStore,
    readout::{self, EditAction, PressAction},
    value_edit::ValueEdit,
    FONT_JETBRAINS_MONO, GOLD, LINE, MUTED, PANEL, TEAL,
};
use std::{
    cell::Cell,
    sync::{atomic::Ordering, Arc, OnceLock},
    time::Duration,
};

type Rect = (f32, f32, f32, f32);
const SIZE: (f32, f32) = (800.0, 620.0);
const KNOB: Rect = (600.0, 146.0, 160.0, 198.0);
const THEME: Rect = (688.0, 23.0, 76.0, 28.0);
const DPDFNET: Rect = (200.0, 23.0, 86.0, 28.0);
const NARA_WPE: Rect = (294.0, 23.0, 99.0, 28.0);
const DTLN: Rect = (401.0, 23.0, 62.0, 28.0);
const LIVE: Rect = (478.0, 23.0, 62.0, 28.0);
const HIGH_QUALITY: Rect = (548.0, 23.0, 120.0, 28.0);
const ENGINE_BUTTONS: [(Rect, &str, EngineKind); 3] = [
    (DPDFNET, "DPDFNET", EngineKind::DpdfNet),
    (NARA_WPE, "NARA-WPE", EngineKind::NaraWpe),
    (DTLN, "DTLN", EngineKind::Dtln),
];
const GRAPH: Rect = (64.0, 432.0, 496.0, 88.0);
const HISTORY_FLOOR: f32 = -90.0;
const CURVE: Rect = (64.0, 142.0, 496.0, 210.0);
const RELATIVE: Rect = (302.0, 86.0, 120.0, 28.0);
const ABSOLUTE: Rect = (432.0, 86.0, 128.0, 28.0);
const FLAT: Rect = (600.0, 370.0, 70.0, 28.0);
const TRANSIENT_GUARD: Rect = (600.0, 517.0, 164.0, 34.0);
const GUARD_TRACK_LEFT: f32 = TRANSIENT_GUARD.0 + 4.0;
const GUARD_TRACK_WIDTH: f32 = TRANSIENT_GUARD.2 - 8.0;
const LOUDER_BUTTON: Rect = (680.0, 370.0, 84.0, 28.0);
fn inside(point: (f32, f32), r: Rect) -> bool {
    point.0 >= r.0 && point.0 <= r.0 + r.2 && point.1 >= r.1 && point.1 <= r.1 + r.3
}
fn prefs() -> &'static AppearanceStore {
    static STORE: OnceLock<AppearanceStore> = OnceLock::new();
    STORE.get_or_init(|| AppearanceStore::new("Dereverb"))
}
fn value_rect() -> Rect {
    KnobLayout::new(KNOB).value_rect(KNOB.0 + 20.0, KNOB.2 - 40.0)
}

fn recent_power(history: &[f32; POINTS]) -> f64 {
    let recent = &history[POINTS - 20..];
    recent
        .iter()
        .map(|&rms| f64::from(rms).powi(2))
        .sum::<f64>()
        / recent.len() as f64
}

fn history_level(power: f64) -> String {
    if power > 0.0 {
        format!("{:.1} dBFS", 10.0 * power.log10())
    } else {
        "−∞ dBFS".into()
    }
}

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
    fn end_drag(&mut self, cx: &mut EventContext) {
        if self.drag.take().is_some() {
            param::end(cx, self.params.strength.as_ptr());
        }
        if let Some((index, _, _)) = self.curve_drag.take() {
            param::end(cx, self.params.weights()[index].as_ptr());
        }
        if self.guard_drag.take().is_some() {
            param::end(cx, self.params.transient_guard.as_ptr());
        }
        cx.release();
    }
    fn node_at(&self, point: (f32, f32)) -> Option<usize> {
        let viewport = Viewport::from_tuple(CURVE);
        self.params
            .weighting()
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, &weight)| {
                let (x, y) = viewport.from_normalized(i as f32 / (NODES - 1) as f32, weight);
                let distance = (x - point.0).hypot(y - point.1);
                (distance <= 13.0).then_some((i, distance))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(index, _)| index)
    }

    fn set_curve(&mut self, cx: &mut EventContext, nodes: [f32; NODES]) {
        self.end_drag(cx);
        for (parameter, value) in self.params.weights().into_iter().zip(nodes) {
            param::set_normalized_once(cx, parameter.as_ptr(), value);
        }
    }
    fn commit(&mut self, cx: &mut EventContext) {
        if let Some(edit) = self.edit.take() {
            if let Some(value) =
                pleasant_ui::value_edit::parse_number_with_units(&edit.text, &[("%", 1.0)])
            {
                if value.is_finite() {
                    param::set_normalized_once(
                        cx,
                        self.params.strength.as_ptr(),
                        value as f32 / 100.0,
                    );
                }
            }
        }
    }
    fn handle_event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window: &WindowEvent, meta| {
            let b = cx.bounds();
            let Some(viewport) = EditorViewport::fit((b.x, b.y, b.w, b.h), SIZE) else {
                return;
            };
            let point = viewport.to_local(cx.mouse().cursorx, cx.mouse().cursory);
            match readout::handle_press(&mut self.press, window, point) {
                PressAction::Inactive => {}
                PressAction::Blocked => return,
                PressAction::BeginDrag(press) => {
                    param::begin(cx, self.params.strength.as_ptr());
                    self.drag = Some((
                        press.origin.0,
                        press.origin.1,
                        self.params.strength.unmodulated_normalized_value(),
                    ));
                }
                PressAction::BeginEdit(_) => {
                    cx.release();
                    self.edit = Some(ValueEdit::new(
                        (),
                        value_rect(),
                        format!("{:.1}", self.params.strength.value()),
                    ));
                    nih_plug_vizia::consume_window_event(cx, window, meta);
                    cx.needs_redraw();
                    return;
                }
                PressAction::Released | PressAction::Cancelled => {
                    cx.release();
                    return;
                }
            }
            if let Some(edit) = &mut self.edit {
                let action = readout::handle_edit(edit, cx, window, point);
                match action {
                    EditAction::Commit | EditAction::CommitAndContinue => self.commit(cx),
                    EditAction::Cancel => self.edit = None,
                    EditAction::Blocked => return,
                    EditAction::Handled => {}
                }
                if action != EditAction::CommitAndContinue {
                    nih_plug_vizia::consume_window_event(cx, window, meta);
                    cx.needs_redraw();
                    return;
                }
            }
            match window {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    cx.focus();
                    if let Some(index) = self.node_at(point) {
                        let parameter = self.params.weights()[index];
                        param::begin(cx, parameter.as_ptr());
                        self.curve_drag =
                            Some((index, point.1, parameter.unmodulated_normalized_value()));
                        cx.capture();
                    } else if inside(point, RELATIVE) || inside(point, ABSOLUTE) {
                        param::set_normalized_once(
                            cx,
                            self.params.level_mode.as_ptr(),
                            if inside(point, RELATIVE) { 0.0 } else { 1.0 },
                        );
                    } else if inside(point, FLAT) {
                        self.set_curve(cx, [1.0; NODES]);
                    } else if inside(point, LOUDER_BUTTON) {
                        self.set_curve(cx, LOUDER);
                    } else if inside(point, TRANSIENT_GUARD) {
                        self.end_drag(cx);
                        let guard = &self.params.transient_guard;
                        param::begin(cx, guard.as_ptr());
                        let value = if cx.modifiers().shift() {
                            guard.unmodulated_normalized_value()
                        } else {
                            let value =
                                ((point.0 - GUARD_TRACK_LEFT) / GUARD_TRACK_WIDTH).clamp(0.0, 1.0);
                            param::set_normalized(cx, guard.as_ptr(), value);
                            value
                        };
                        self.guard_drag = Some((point.0, value));
                        cx.capture();
                    } else if inside(point, THEME) {
                        prefs().toggle();
                    } else if let Some((_, _, engine)) = ENGINE_BUTTONS
                        .iter()
                        .find(|(rect, _, _)| inside(point, *rect))
                    {
                        self.end_drag(cx);
                        param::set_normalized_once(
                            cx,
                            self.params.engine.as_ptr(),
                            self.params.engine.preview_normalized(*engine),
                        );
                    } else if self.params.engine.value().has_quality()
                        && (inside(point, LIVE) || inside(point, HIGH_QUALITY))
                    {
                        self.end_drag(cx);
                        let quality = if inside(point, LIVE) {
                            Quality::Live
                        } else {
                            Quality::HighQuality
                        };
                        param::set_normalized_once(
                            cx,
                            self.params.quality.as_ptr(),
                            self.params.quality.preview_normalized(quality),
                        );
                    } else if inside(point, value_rect()) {
                        self.press = Some(ValuePress::new((), value_rect(), point));
                        cx.capture();
                    } else if inside(point, KNOB) {
                        param::begin(cx, self.params.strength.as_ptr());
                        self.drag = Some((
                            point.0,
                            point.1,
                            self.params.strength.unmodulated_normalized_value(),
                        ));
                        cx.capture();
                    } else {
                        return;
                    }
                }
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some(point);
                    if let Some((index, y, value)) = self.curve_drag {
                        let scale = if cx.modifiers().shift() { 0.1 } else { 1.0 };
                        param::set_normalized(
                            cx,
                            self.params.weights()[index].as_ptr(),
                            value + (y - point.1) / CURVE.3 * scale,
                        );
                    }
                    if let Some((x, value)) = self.guard_drag {
                        let scale = if cx.modifiers().shift() { 0.1 } else { 1.0 };
                        param::set_normalized(
                            cx,
                            self.params.transient_guard.as_ptr(),
                            value + (point.0 - x) / GUARD_TRACK_WIDTH * scale,
                        );
                    }
                    if let Some((x, y, value)) = self.drag {
                        let sensitivity = if cx.modifiers().shift() {
                            0.0005
                        } else {
                            0.005
                        };
                        param::set_normalized(
                            cx,
                            self.params.strength.as_ptr(),
                            value + (point.0 - x + y - point.1) * sensitivity,
                        );
                    }
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left)
                    if self.node_at(point).is_some() =>
                {
                    let index = self.node_at(point).unwrap();
                    self.end_drag(cx);
                    param::set_normalized_once(cx, self.params.weights()[index].as_ptr(), 1.0);
                }
                WindowEvent::MouseScroll(_, dy)
                    if self.node_at(point).is_some() && self.curve_drag.is_none() =>
                {
                    let parameter = self.params.weights()[self.node_at(point).unwrap()];
                    let step = if cx.modifiers().shift() { 0.001 } else { 0.01 };
                    param::set_normalized_once(
                        cx,
                        parameter.as_ptr(),
                        parameter.unmodulated_normalized_value() + dy * step,
                    );
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left)
                    if inside(point, TRANSIENT_GUARD) =>
                {
                    self.end_drag(cx);
                    let guard = &self.params.transient_guard;
                    param::set_normalized_once(
                        cx,
                        guard.as_ptr(),
                        guard.default_normalized_value(),
                    );
                }
                WindowEvent::MouseScroll(_, dy)
                    if inside(point, TRANSIENT_GUARD) && self.guard_drag.is_none() =>
                {
                    let guard = &self.params.transient_guard;
                    let step = if cx.modifiers().shift() { 0.001 } else { 0.01 };
                    param::set_normalized_once(
                        cx,
                        guard.as_ptr(),
                        guard.unmodulated_normalized_value() + dy * step,
                    );
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) if inside(point, KNOB) => {
                    self.end_drag(cx);
                    param::set_normalized_once(cx, self.params.strength.as_ptr(), 1.0);
                }
                WindowEvent::MouseScroll(_, dy) if inside(point, KNOB) && self.drag.is_none() => {
                    let step = if cx.modifiers().shift() { 0.001 } else { 0.01 };
                    param::set_normalized_once(
                        cx,
                        self.params.strength.as_ptr(),
                        self.params.strength.unmodulated_normalized_value() + dy * step,
                    );
                }
                WindowEvent::MouseUp(MouseButton::Left)
                | WindowEvent::FocusOut
                | WindowEvent::KeyDown(Code::Escape, _)
                | WindowEvent::MouseDown(MouseButton::Right) => self.end_drag(cx),
                WindowEvent::MouseLeave => self.hover = None,
                _ => return,
            }
            nih_plug_vizia::consume_window_event(cx, window, meta);
            cx.needs_redraw();
        });
    }
    fn render(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        if self.font.get().is_none() {
            self.font.set(canvas.add_font_mem(FONT_JETBRAINS_MONO).ok());
        }
        let b = cx.bounds();
        let Some(mut d) = Draw::new_fitted(
            canvas,
            prefs().light(),
            (b.x, b.y, b.w, b.h),
            SIZE,
            self.font.get(),
        ) else {
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
        } else if self.hover.is_some_and(|p| inside(p, value_rect())) {
            d.value_underline(value_rect(), KnobLayout::new(KNOB).value_y, GOLD);
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

        let status = self.shared.status.load(Ordering::Relaxed);
        let rate = self.shared.sample_rate.load(Ordering::Relaxed);
        let latency = self.shared.latency.load(Ordering::Relaxed);
        let active_quality = self.shared.quality.load(Ordering::Relaxed);
        let active_engine = self.shared.engine.load(Ordering::Relaxed);
        let active_slot = EngineKind::MODES.iter().position(|&(kind, quality)| {
            kind as u32 == active_engine
                && (!kind.has_quality() || quality.index() as u32 == active_quality)
        });
        let switching = active_slot != Some(engine.index(self.params.quality.value()));
        let mode = match active_slot.map(|index| EngineKind::MODES[index]) {
            Some((EngineKind::Dtln, _)) => "DTLN DENOISE",
            Some((EngineKind::NaraWpe, Quality::Live)) => "NARA-WPE · SHORT",
            Some((EngineKind::NaraWpe, Quality::HighQuality)) => "NARA-WPE · 20 MS",
            Some((EngineKind::DpdfNet, Quality::HighQuality)) => "DPDFNET HIGH QUALITY",
            _ => "DPDFNET LIVE",
        };
        let state = if switching {
            format!("{mode} · SWITCHING MODE")
        } else {
            match status {
                telemetry::ACTIVE => format!(
                    "{mode}   {:.1} kHz   {:.1} ms",
                    rate / 1000.0,
                    1000.0 * latency as f32 / rate.max(1.0)
                ),
                telemetry::RECOVERING => "RECOVERING · ALIGNED PASSTHROUGH".into(),
                telemetry::FAILED => "BYPASSED · ENGINE UNAVAILABLE".into(),
                _ => "WAITING FOR AUDIO".into(),
            }
        };
        d.line(36.0, 559.0, 764.0, 559.0, LINE, 1.0);
        d.text(
            36.0,
            581.0,
            &state,
            11.0,
            if status == telemetry::ACTIVE {
                TEAL
            } else {
                GOLD
            },
        );
        let tooltip = if let Some(index) = active_slot.filter(|_| status == telemetry::FAILED) {
            self.shared.errors.lock().unwrap_or_else(|e| e.into_inner())[index]
                .chars()
                .take(105)
                .collect::<String>()
        } else if self.hover.is_some_and(|p| inside(p, THEME)) {
            "Cycle Dark, Light, and Auto appearance.".into()
        } else if self.hover.is_some_and(|p| inside(p, DPDFNET)) {
            "DPDFNet: neural voice restoration with Live and High Quality modes.".into()
        } else if self.hover.is_some_and(|p| inside(p, NARA_WPE)) {
            "NARA-WPE: adaptive late reverb reduction. The filter learns from the incoming audio."
                .into()
        } else if self.hover.is_some_and(|p| inside(p, DTLN)) {
            "DTLN: speech denoising. At full strength, wet audio is limited to about 8 kHz.".into()
        } else if engine == EngineKind::NaraWpe && self.hover.is_some_and(|p| inside(p, LIVE)) {
            "10 ms analysis window. A half-second average of the learned filter steadies removal."
                .into()
        } else if engine == EngineKind::NaraWpe
            && self.hover.is_some_and(|p| inside(p, HIGH_QUALITY))
        {
            "20 ms analysis window with finer frequency resolution and faster filter response."
                .into()
        } else if neural && self.hover.is_some_and(|p| inside(p, LIVE)) {
            "Live uses lighter neural spectral correction.".into()
        } else if neural && self.hover.is_some_and(|p| inside(p, HIGH_QUALITY)) {
            "High Quality uses the full neural model with deep filtering.".into()
        } else if self.hover.is_some_and(|p| inside(p, RELATIVE)) {
            "Relative: level against a 1-second RMS reference; weighting follows whole-recording gain changes.".into()
        } else if self.hover.is_some_and(|p| inside(p, ABSOLUTE)) {
            "Absolute: original input RMS in dBFS; gain before Dereverb intentionally changes the weighting.".into()
        } else if self.hover.is_some_and(|p| inside(p, TRANSIENT_GUARD)) {
            "Drag or scroll to set protection for attacks and s, sh, and h. Shift for fine adjustment.".into()
        } else if self.hover.is_some_and(|p| inside(p, FLAT)) {
            "Flat: apply the same Strength at every input level.".into()
        } else if self.hover.is_some_and(|p| inside(p, LOUDER_BUTTON)) {
            "Louder: reduce processing on quieter sounds. Drag the curve points to fine-tune."
                .into()
        } else if self.curve_drag.is_some()
            || self
                .hover
                .is_some_and(|p| self.node_at(p).is_some() || inside(p, CURVE))
        {
            "Drag points vertically; Shift for fine control. Double-click resets. The teal dot follows the loudest channel.".into()
        } else if self.edit.is_some() {
            "Type a percentage. Enter applies; Escape cancels.".into()
        } else if self.hover.is_some_and(|p| inside(p, KNOB)) {
            "Drag or scroll Strength; Shift for fine control. Click its value to type; double-click to reset.".into()
        } else if self.hover.is_some_and(|p| inside(p, GRAPH)) {
            "Input and output RMS are time-aligned. Readouts average the latest 200 ms across all channels.".into()
        } else {
            "Input level is normalized internally, then restored. The curve weights the selected Strength.".into()
        };
        d.text(36.0, 605.0, &tooltip, 10.0, MUTED);
    }

    fn paint_curve(&self, d: &mut Draw) {
        let curve = self.params.weighting();
        let viewport = Viewport::from_tuple(CURVE);
        d.text(64.0, 104.0, "LEVEL WEIGHTING", 11.0, MUTED);
        d.button(
            RELATIVE,
            "RELATIVE",
            curve.mode == LevelMode::Relative,
            GOLD,
        );
        d.button(
            ABSOLUTE,
            "ABSOLUTE",
            curve.mode == LevelMode::Absolute,
            GOLD,
        );
        d.text(64.0, 130.0, "WEIGHT %", 10.0, MUTED);
        for value in [0, 25, 50, 75, 100] {
            let (_, y) = viewport.from_normalized(0.0, value as f32 / 100.0);
            d.line(CURVE.0, y, CURVE.0 + CURVE.2, y, LINE, 1.0);
            d.text_right(CURVE.0 - 12.0, y + 4.0, &value.to_string(), 10.0, MUTED);
        }
        for i in 0..NODES {
            let position = i as f32 / (NODES - 1) as f32;
            let (x, _) = viewport.from_normalized(position, 0.0);
            d.line(x, CURVE.1, x, CURVE.1 + CURVE.3, LINE, 1.0);
            d.text_centered(
                x,
                372.0,
                &format!("{:.0}", curve.mode.db(position)),
                10.0,
                MUTED,
            );
        }
        d.text_centered(
            CURVE.0 + CURVE.2 * 0.5,
            391.0,
            if curve.mode == LevelMode::Relative {
                "LEVEL RELATIVE TO AVERAGE · dB"
            } else {
                "INPUT LEVEL · dBFS"
            },
            10.0,
            MUTED,
        );
        let points: [(f32, f32); 129] = std::array::from_fn(|i| {
            let x = i as f32 / 128.0;
            viewport.from_normalized(x, curve.at(x))
        });
        let mut fill = GOLD;
        fill.a = 0.06;
        d.area(&points, CURVE.1 + CURVE.3, fill);
        d.poly(&points, GOLD, 2.0);
        for (i, &value) in curve.nodes.iter().enumerate() {
            let (x, y) = viewport.from_normalized(i as f32 / (NODES - 1) as f32, value);
            let selected = self.curve_drag.is_some_and(|(n, _, _)| n == i)
                || self.hover.is_some_and(|p| self.node_at(p) == Some(i));
            d.circle(x, y, if selected { 6.5 } else { 5.0 }, GOLD, true);
            if selected {
                d.circle(x, y, 10.0, GOLD, false);
                d.text_right(
                    560.0,
                    130.0,
                    &format!(
                        "{:.0} dB / {:.1} %",
                        curve.mode.db(i as f32 / 4.0),
                        value * 100.0
                    ),
                    10.0,
                    GOLD,
                );
            }
        }
        let db = match curve.mode {
            LevelMode::Relative => self.shared.relative_db.load(Ordering::Relaxed),
            LevelMode::Absolute => self.shared.absolute_db.load(Ordering::Relaxed),
        };
        let active = self.shared.active_level.load(Ordering::Relaxed) != 0;
        if active {
            let (x, y) = viewport.from_normalized(
                curve.mode.position(db),
                self.shared.weight.load(Ordering::Relaxed),
            );
            let mut glow = TEAL;
            glow.a = 0.16;
            d.circle(x, y, 11.0, glow, true);
            d.circle(x, y, 5.5, TEAL, true);
            d.circle(x, y, 8.0, TEAL, false);
        }
        d.button(FLAT, "FLAT", curve.nodes == [1.0; NODES], GOLD);
        d.button(
            LOUDER_BUTTON,
            "LOUDER",
            curve
                .nodes
                .iter()
                .zip(LOUDER)
                .all(|(a, b)| (a - b).abs() < 1e-5),
            GOLD,
        );
        d.text(600.0, 430.0, "EFFECTIVE STRENGTH", 10.0, MUTED);
        d.text(
            600.0,
            454.0,
            &format!(
                "{:.1} %",
                if active {
                    self.shared.effective.load(Ordering::Relaxed) * 100.0
                } else {
                    0.0
                }
            ),
            18.0,
            TEAL,
        );
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
                let timer = cx.add_timer(Duration::from_millis(33), None, |cx, action| {
                    if let TimerAction::Tick(_) = action {
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

#[cfg(all(test, target_os = "macos"))]
#[path = "ui_preview.rs"]
mod preview;
