use crate::{params::TapeStopParams, telemetry::TapeStopTelemetry};
use nih_plug::prelude::*;
use nih_plug_vizia::widgets::util::ModifiersExt;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{
        prelude::*,
        vg::{Color, FontId},
    },
    widgets::RawParamEvent,
    ViziaState, ViziaTheming,
};
use pleasant_ui::{
    draw::Draw,
    math::linear_to_db,
    preferences::AppearanceStore,
    theme::{rgb, BG, COLORS, GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
    value_edit::{parse_number_with_units, ValueEdit},
    FONT_JETBRAINS_MONO,
};
use std::{
    cell::Cell,
    sync::{atomic::Ordering, Arc, OnceLock},
    time::Duration,
};

static PREFS: OnceLock<AppearanceStore> = OnceLock::new();
fn prefs() -> &'static AppearanceStore {
    PREFS.get_or_init(|| AppearanceStore::new("Tape Stop"))
}

const UI_W: f32 = 1040.0;
const UI_H: f32 = 520.0;
const HEADER_HEIGHT: f32 = 70.0;
const THEME_BUTTON: (f32, f32, f32, f32) = (870.0, 22.0, 72.0, 26.0);
const BYPASS_BUTTON: (f32, f32, f32, f32) = (954.0, 22.0, 32.0, 26.0);

const GRAPH_X: f32 = 60.0;
const GRAPH_Y: f32 = 90.0;
const GRAPH_W: f32 = 920.0;
const GRAPH_H: f32 = 280.0;

const AUTO_RESTART_BUTTON: (f32, f32, f32, f32) = (716.0, 390.0, 148.0, 28.0);
const POWER_BUTTON: (f32, f32, f32, f32) = (716.0, 426.0, 148.0, 28.0);
const CC_FIELD: (f32, f32, f32, f32) = (548.0, 390.0, 148.0, 28.0);
const METER_IN: (f32, f32, f32, f32) = (876.0, 390.0, 104.0, 28.0);
const METER_OUT: (f32, f32, f32, f32) = (876.0, 426.0, 104.0, 28.0);
const CURVE_NODE_HIT: f32 = 16.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KnobId {
    DropTime,
    Xfade,
    Curve,
    StereoDiv,
    RestartThresh,
    OverrideCc,
}

const ALL_KNOBS: &[KnobId] = &[
    KnobId::DropTime,
    KnobId::Xfade,
    KnobId::StereoDiv,
    KnobId::RestartThresh,
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum DragState {
    Knob {
        id: KnobId,
        start_y: f32,
        start_norm: f32,
    },
    Curve {
        start_x: f32,
        start_y: f32,
        start_norm: f32,
    },
}

pub struct TapeStopView {
    params: Arc<TapeStopParams>,
    telemetry: Arc<TapeStopTelemetry>,
    font: Cell<Option<FontId>>,
    drag: Option<DragState>,
    edit: Option<ValueEdit<KnobId>>,
}

fn s_curve(t: f32, exp: f32) -> f32 {
    if t < 0.5 {
        0.5 * (2.0 * t).powf(exp)
    } else {
        1.0 - 0.5 * (2.0 * (1.0 - t)).powf(exp)
    }
}

impl TapeStopView {
    fn param(&self, id: KnobId) -> &FloatParam {
        match id {
            KnobId::DropTime => &self.params.drop_time,
            KnobId::Xfade => &self.params.xfade_ms,
            KnobId::Curve => &self.params.drop_curve,
            KnobId::StereoDiv => &self.params.stereo_div,
            KnobId::RestartThresh => &self.params.auto_restart_thresh,
            KnobId::OverrideCc => unreachable!("CC is an IntParam"),
        }
    }

    fn knob_rect(id: KnobId) -> (f32, f32, f32, f32) {
        let col = match id {
            KnobId::DropTime => 0,
            KnobId::Xfade => 1,
            KnobId::StereoDiv => 2,
            KnobId::RestartThresh => 3,
            KnobId::Curve | KnobId::OverrideCc => 0,
        };
        let x = 36.0 + col as f32 * (98.0 + 14.0);
        (x, 390.0, 98.0, 92.0)
    }

    fn inside(px: f32, py: f32, rect: (f32, f32, f32, f32)) -> bool {
        px >= rect.0 && px <= rect.0 + rect.2 && py >= rect.1 && py <= rect.1 + rect.3
    }

    fn curve_node_pos() -> (f32, f32) {
        (GRAPH_X + GRAPH_W * 0.5, GRAPH_Y + GRAPH_H * 0.5)
    }

    fn curve_edit_rect() -> (f32, f32, f32, f32) {
        let (cx, cy) = Self::curve_node_pos();
        (cx - 40.0, cy - 16.0, 80.0, 32.0)
    }

    fn hit_curve_node(px: f32, py: f32) -> bool {
        let (cx, cy) = Self::curve_node_pos();
        let dx = px - cx;
        let dy = py - cy;
        dx * dx + dy * dy <= CURVE_NODE_HIT * CURVE_NODE_HIT
    }

    fn knob_enabled(&self, id: KnobId) -> bool {
        id != KnobId::RestartThresh || self.params.auto_restart.value()
    }

    fn emit_param_norm(&self, cx: &mut EventContext, ptr: ParamPtr, norm: f32) {
        let norm = norm.clamp(0.0, 1.0);
        cx.emit(RawParamEvent::BeginSetParameter(ptr));
        cx.emit(RawParamEvent::SetParameterNormalized(ptr, norm));
        cx.emit(RawParamEvent::EndSetParameter(ptr));
    }

    fn emit_knob_norm(&self, cx: &mut EventContext, id: KnobId, norm: f32) {
        if id == KnobId::OverrideCc {
            self.emit_param_norm(cx, self.params.override_cc.as_ptr(), norm);
        } else {
            self.emit_param_norm(cx, self.param(id).as_ptr(), norm);
        }
    }

    fn get_knob_norm(&self, id: KnobId) -> f32 {
        if id == KnobId::OverrideCc {
            self.params.override_cc.unmodulated_normalized_value()
        } else {
            self.param(id).unmodulated_normalized_value()
        }
    }

    fn knob_info(&self, id: KnobId) -> (&'static str, String, Color) {
        match id {
            KnobId::DropTime => (
                "DROP TIME",
                format!("{:.2}s", self.params.drop_time.value()),
                COLORS[4],
            ),
            KnobId::Xfade => (
                "CROSSFADE",
                format!("{:.1}ms", self.params.xfade_ms.value()),
                COLORS[5],
            ),
            KnobId::Curve => (
                "CURVE",
                format!("{:.2}", self.params.drop_curve.value()),
                COLORS[1],
            ),
            KnobId::StereoDiv => (
                "STEREO DIV",
                format!("{:.1}%", self.params.stereo_div.value()),
                COLORS[3],
            ),
            KnobId::RestartThresh => (
                "THRESHOLD",
                format!("{:.1}dB", self.params.auto_restart_thresh.value()),
                COLORS[2],
            ),
            KnobId::OverrideCc => (
                "CC #",
                format!("{}", self.params.override_cc.value()),
                COLORS[4],
            ),
        }
    }

    fn commit_edit(&mut self, cx: &mut EventContext) {
        if let Some(edit) = self.edit.take() {
            let target = edit.target;
            let text = edit.text;
            let parsed = match target {
                KnobId::DropTime => parse_number_with_units(&text, &[("ms", 0.001), ("s", 1.0)]),
                KnobId::Xfade => parse_number_with_units(&text, &[("ms", 1.0), ("s", 1000.0)]),
                KnobId::StereoDiv => parse_number_with_units(&text, &[("%", 1.0)]),
                KnobId::RestartThresh => parse_number_with_units(&text, &[("db", 1.0)]),
                KnobId::OverrideCc => parse_number_with_units(&text, &[]),
                KnobId::Curve => parse_number_with_units(&text, &[]),
            };

            if let Some(v) = parsed {
                if target == KnobId::OverrideCc {
                    let norm = self.params.override_cc.preview_normalized(v as i32);
                    self.emit_param_norm(cx, self.params.override_cc.as_ptr(), norm);
                } else {
                    let p = self.param(target);
                    let norm = p.preview_normalized(v as f32);
                    self.emit_param_norm(cx, p.as_ptr(), norm);
                }
            }
        }
    }

    fn peak_norm(peak: f32) -> f32 {
        let db = linear_to_db(peak as f64);
        ((db + 60.0) / 60.0).clamp(0.0, 1.0) as f32
    }

    fn draw_graph(&self, d: &mut Draw) {
        let gx = GRAPH_X;
        let gy = GRAPH_Y;
        let gw = GRAPH_W;
        let gh = GRAPH_H;
        let curve_exp = self.params.drop_curve.value();
        let drop_time = self.params.drop_time.value();
        let braking = self.telemetry.is_braking.load(Ordering::Relaxed);
        let crossfading = self.telemetry.is_crossfading.load(Ordering::Relaxed);
        let progress = self.telemetry.brake_progress.load(Ordering::Relaxed);
        let speed_l = self.telemetry.speed_l.load(Ordering::Relaxed);
        let speed_r = self.telemetry.speed_r.load(Ordering::Relaxed);

        d.rect(gx, gy, gw, gh, rgb(23, 27, 33));

        for i in 0..=4 {
            let y = gy + gh * (i as f32 / 4.0);
            d.line(
                gx,
                y,
                gx + gw,
                y,
                if i == 2 { rgb(75, 82, 92) } else { LINE },
                1.0,
            );
            let speed = 1.0 - i as f32 / 4.0;
            d.text(gx - 36.0, y + 4.0, &format!("{speed:.2}"), 10.5, MUTED);
        }
        for i in 0..=5 {
            let x = gx + gw * (i as f32 / 5.0);
            d.line(x, gy, x, gy + gh, LINE, 1.0);
            let t = drop_time * i as f32 / 5.0;
            let label = if drop_time >= 1.5 {
                format!("{t:.1}")
            } else {
                format!("{t:.2}")
            };
            d.text(x - 10.0, gy + gh + 18.0, &label, 10.5, MUTED);
        }
        d.text(gx - 36.0, gy - 8.0, "SPEED", 10.0, MUTED);
        d.outline(
            (gx, gy, gw, gh),
            if braking || crossfading { TEAL } else { LINE },
        );

        let mut curve = Vec::with_capacity(101);
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let y = gy + s_curve(t, curve_exp) * gh;
            curve.push((gx + t * gw, y));
        }
        let mut fill = GOLD;
        fill.a = 0.12;
        d.area(&curve, gy + gh, fill);
        d.poly(&curve, GOLD, 2.0);

        d.text(gx + 8.0, gy + 22.0, "START", 11.0, TEXT);
        d.text(gx + gw - 44.0, gy + gh - 10.0, "STOP", 11.0, TEXT);

        let (nx, ny) = Self::curve_node_pos();
        let node_color = if self.params.bypass.value() || !self.params.power.value() {
            MUTED
        } else {
            GOLD
        };
        d.circle(nx, ny, 8.0, node_color, false);
        d.circle(nx, ny, 3.5, node_color, true);

        if let Some(edit) = &self.edit {
            if edit.target == KnobId::Curve {
                let r = edit.rect;
                d.rect(r.0, r.1, r.2, r.3, PANEL);
                d.outline(r, GOLD);
                d.text_centered(r.0 + r.2 * 0.5, r.1 + 22.0, &edit.text, 12.0, GOLD);
            }
        }

        if braking || speed_l < 0.99 || speed_r < 0.99 {
            let live_x = gx + progress.clamp(0.0, 1.0) * gw;
            let y_l = gy + (1.0 - speed_l.clamp(0.0, 1.0)) * gh;
            let y_r = gy + (1.0 - speed_r.clamp(0.0, 1.0)) * gh;
            d.circle(live_x, y_l, 6.0, TEAL, false);
            d.circle(live_x, y_l, 3.0, TEXT, true);
            if (speed_l - speed_r).abs() > 0.01 {
                d.circle(live_x, y_r, 6.0, COLORS[4], false);
                d.circle(live_x, y_r, 3.0, TEXT, true);
            }
        }
    }

    fn draw_meter(d: &mut Draw, rect: (f32, f32, f32, f32), label: &str, norm: f32, color: Color) {
        d.rect(rect.0, rect.1, rect.2, rect.3, PANEL);
        d.rect(rect.0 + 28.0, rect.1 + 8.0, 68.0, 12.0, LINE);
        if norm > 0.01 {
            d.rect(
                rect.0 + 28.0,
                rect.1 + 8.0,
                68.0 * norm.clamp(0.0, 1.0),
                12.0,
                color,
            );
        }
        d.text(rect.0 + 8.0, rect.1 + 18.0, label, 10.0, MUTED);
    }
}

impl View for TapeStopView {
    fn element(&self) -> Option<&'static str> {
        Some("tape-stop-view")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| {
            let bounds = cx.bounds();
            let scale = bounds.w / UI_W;
            let mouse_x = (cx.mouse().cursorx - bounds.x) / scale;
            let mouse_y = (cx.mouse().cursory - bounds.y) / scale;

            if self.edit.is_some() {
                match window_event {
                    WindowEvent::CharInput(c) => {
                        if !cx.modifiers().command() && c.is_ascii() && !c.is_control() {
                            if let Some(edit) = self.edit.as_mut() {
                                edit.insert(&c.to_string());
                            }
                        }
                        meta.consume();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::KeyDown(Code::Enter, _) => {
                        self.commit_edit(cx);
                        meta.consume();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::KeyDown(Code::Escape, _) => {
                        self.edit = None;
                        meta.consume();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::KeyDown(Code::Backspace, _) => {
                        if let Some(edit) = self.edit.as_mut() {
                            edit.erase(true);
                        }
                        meta.consume();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::MouseDown(MouseButton::Left) => {
                        self.commit_edit(cx);
                        meta.consume();
                        cx.needs_redraw();
                        return;
                    }
                    _ => return,
                }
            }

            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    if Self::inside(mouse_x, mouse_y, THEME_BUTTON) {
                        prefs().toggle();
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, BYPASS_BUTTON) {
                        let next = if self.params.bypass.value() { 0.0 } else { 1.0 };
                        self.emit_param_norm(cx, self.params.bypass.as_ptr(), next);
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, AUTO_RESTART_BUTTON) {
                        let next = if self.params.auto_restart.value() {
                            0.0
                        } else {
                            1.0
                        };
                        self.emit_param_norm(cx, self.params.auto_restart.as_ptr(), next);
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, POWER_BUTTON) {
                        let next = if self.params.power.value() { 0.0 } else { 1.0 };
                        self.emit_param_norm(cx, self.params.power.as_ptr(), next);
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, CC_FIELD) {
                        let val_str = format!("{}", self.params.override_cc.value());
                        self.edit = Some(ValueEdit::new(KnobId::OverrideCc, CC_FIELD, val_str));
                        cx.needs_redraw();
                        return;
                    }

                    for &id in ALL_KNOBS {
                        if !self.knob_enabled(id) {
                            continue;
                        }
                        let r = Self::knob_rect(id);
                        if Self::inside(mouse_x, mouse_y, r) {
                            self.drag = Some(DragState::Knob {
                                id,
                                start_y: mouse_y,
                                start_norm: self.get_knob_norm(id),
                            });
                            cx.needs_redraw();
                            return;
                        }
                    }

                    if Self::hit_curve_node(mouse_x, mouse_y) {
                        self.drag = Some(DragState::Curve {
                            start_x: mouse_x,
                            start_y: mouse_y,
                            start_norm: self.get_knob_norm(KnobId::Curve),
                        });
                        cx.needs_redraw();
                    }
                }

                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    if Self::inside(mouse_x, mouse_y, CC_FIELD) {
                        let val_str = format!("{}", self.params.override_cc.value());
                        self.edit = Some(ValueEdit::new(KnobId::OverrideCc, CC_FIELD, val_str));
                        cx.needs_redraw();
                        return;
                    }

                    for &id in ALL_KNOBS {
                        if !self.knob_enabled(id) {
                            continue;
                        }
                        let r = Self::knob_rect(id);
                        if Self::inside(mouse_x, mouse_y, r) {
                            let (_, val_str, _) = self.knob_info(id);
                            self.edit = Some(ValueEdit::new(id, r, val_str));
                            cx.needs_redraw();
                            return;
                        }
                    }

                    if Self::hit_curve_node(mouse_x, mouse_y) {
                        let (_, val_str, _) = self.knob_info(KnobId::Curve);
                        self.edit = Some(ValueEdit::new(
                            KnobId::Curve,
                            Self::curve_edit_rect(),
                            val_str,
                        ));
                        cx.needs_redraw();
                    }
                }

                WindowEvent::MouseUp(MouseButton::Left) => {
                    self.drag = None;
                    cx.needs_redraw();
                }

                WindowEvent::MouseMove(_, _) => match self.drag {
                    Some(DragState::Knob {
                        id,
                        start_y,
                        start_norm,
                    }) => {
                        let delta_y = start_y - mouse_y;
                        let step = if cx.modifiers().shift() { 0.001 } else { 0.005 };
                        let new_norm = (start_norm + delta_y * step).clamp(0.0, 1.0);
                        self.emit_knob_norm(cx, id, new_norm);
                        cx.needs_redraw();
                    }
                    Some(DragState::Curve {
                        start_x,
                        start_y,
                        start_norm,
                    }) => {
                        let dx = mouse_x - start_x;
                        let dy = start_y - mouse_y;
                        let step = if cx.modifiers().shift() { 0.001 } else { 0.004 };
                        let new_norm = (start_norm + (dx + dy) * step).clamp(0.0, 1.0);
                        self.emit_knob_norm(cx, KnobId::Curve, new_norm);
                        cx.needs_redraw();
                    }
                    None => {}
                },

                _ => {}
            }
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }
        if self.font.get().is_none() {
            self.font.set(canvas.add_font_mem(FONT_JETBRAINS_MONO).ok());
        }

        let mut d = Draw::new(
            canvas,
            prefs().light(),
            bounds.w / UI_W,
            bounds.x,
            bounds.y,
            self.font.get(),
        );

        let bypassed = self.params.bypass.value();
        let powered = self.params.power.value();
        let inactive = bypassed || !powered;

        d.rect(0.0, 0.0, UI_W, UI_H, BG);
        d.rect(0.0, 0.0, UI_W, HEADER_HEIGHT, PANEL);
        d.text(36.0, 44.0, "TAPE STOP", 24.0, GOLD);
        d.text(200.0, 44.0, "MIDI TAPE BRAKE", 13.0, TEXT);

        let is_light = prefs().light();
        d.button(
            THEME_BUTTON,
            if is_light { "DARK" } else { "LIGHT" },
            false,
            MUTED,
        );
        d.bypass_button(BYPASS_BUTTON, bypassed, TEAL);

        self.draw_graph(&mut d);

        let auto_on = self.params.auto_restart.value();

        for &id in ALL_KNOBS {
            let r = Self::knob_rect(id);
            let (label, val_str, color) = self.knob_info(id);
            let n = self.get_knob_norm(id);
            let greyed = inactive || !self.knob_enabled(id);

            if let Some(edit) = &self.edit {
                if edit.target == id {
                    d.rect(r.0, r.1, r.2, r.3, PANEL);
                    d.outline(r, GOLD);
                    d.text_centered(r.0 + r.2 * 0.5, r.1 + 22.0, label, 9.2, TEXT);
                    d.text_centered(r.0 + r.2 * 0.5, r.1 + 50.0, &edit.text, 12.0, GOLD);
                    continue;
                }
            }

            d.knob(r, label, &val_str, n, color, greyed);
        }

        let cc_edit = self
            .edit
            .as_ref()
            .filter(|edit| edit.target == KnobId::OverrideCc);
        d.rect(CC_FIELD.0, CC_FIELD.1, CC_FIELD.2, CC_FIELD.3, PANEL);
        d.outline(CC_FIELD, if cc_edit.is_some() { GOLD } else { LINE });
        d.text(
            CC_FIELD.0 + 10.0,
            CC_FIELD.1 + 18.0,
            "CC #",
            11.0,
            if inactive { MUTED } else { TEXT },
        );
        let cc_val;
        let (cc_text, cc_color) = if let Some(edit) = cc_edit {
            (edit.text.as_str(), GOLD)
        } else {
            cc_val = format!("{}", self.params.override_cc.value());
            (cc_val.as_str(), if inactive { MUTED } else { COLORS[4] })
        };
        d.text_centered(
            CC_FIELD.0 + CC_FIELD.2 - 28.0,
            CC_FIELD.1 + 18.0,
            cc_text,
            12.0,
            cc_color,
        );
        let flash = self.telemetry.transient_flash.load(Ordering::Relaxed);
        d.button(
            AUTO_RESTART_BUTTON,
            "AUTO RESTART",
            auto_on,
            if auto_on && flash > 0.05 {
                COLORS[5]
            } else {
                COLORS[2]
            },
        );
        if auto_on {
            let led = if flash > 0.05 {
                COLORS[5]
            } else {
                rgb(46, 89, 51)
            };
            d.circle(
                AUTO_RESTART_BUTTON.0 + AUTO_RESTART_BUTTON.2 - 12.0,
                AUTO_RESTART_BUTTON.1 + AUTO_RESTART_BUTTON.3 * 0.5,
                4.0,
                led,
                true,
            );
        }

        d.button(
            POWER_BUTTON,
            if powered { "POWER" } else { "OFF" },
            powered,
            TEAL,
        );

        let in_peak = self.telemetry.input_peak.load(Ordering::Relaxed);
        let out_peak = self.telemetry.output_peak.load(Ordering::Relaxed);
        Self::draw_meter(&mut d, METER_IN, "IN", Self::peak_norm(in_peak), TEAL);
        Self::draw_meter(&mut d, METER_OUT, "OUT", Self::peak_norm(out_peak), GOLD);
    }
}

pub fn default_editor_state() -> Arc<ViziaState> {
    ViziaState::new(|| (UI_W as u32, UI_H as u32))
}

pub fn create(
    params: Arc<TapeStopParams>,
    telemetry: Arc<TapeStopTelemetry>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            nih_plug_vizia::assets::register_noto_sans_light(cx);
            TapeStopView {
                params: params.clone(),
                telemetry: telemetry.clone(),
                font: Cell::new(None),
                drag: None,
                edit: None,
            }
            .build(cx, |cx| {
                let timer = cx.add_timer(Duration::from_millis(16), None, |cx, action| {
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
