use crate::{
    dsp::{inv_s_curve, s_curve},
    params::{MidiAssign, TapeStopParams},
    telemetry::TapeStopTelemetry,
};
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
    draw::{ButtonAnim, Draw},
    math::linear_to_db,
    preferences::AppearanceStore,
    theme::{rgb, BG, COLORS, GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
    value_edit::{parse_number_with_units, typed_char, ValueEdit},
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

const UI_W: f32 = 1112.0;
const UI_H: f32 = 438.0;
const HEADER_HEIGHT: f32 = 70.0;

const THEME_BUTTON: (f32, f32, f32, f32) = (866.0, 22.0, 72.0, 26.0);
const BYPASS_BUTTON: (f32, f32, f32, f32) = (954.0, 22.0, 26.0, 26.0);

const GRAPH_X: f32 = 60.0;
const GRAPH_Y: f32 = 108.0;
const GRAPH_W: f32 = 852.0;
const GRAPH_H: f32 = 252.0;

const DROP_SLIDER_GAP: f32 = 10.0;
const DROP_SLIDER_W: f32 = 40.0;
const DROP_SLIDER_EXTRA: f32 = 10.0;
const DROP_TIME_SLIDER: (f32, f32, f32, f32) = (
    GRAPH_X + GRAPH_W + DROP_SLIDER_GAP,
    GRAPH_Y,
    DROP_SLIDER_W,
    GRAPH_H + DROP_SLIDER_EXTRA,
);
const DROP_READOUT_W: f32 = 46.0 + DROP_SLIDER_GAP + DROP_SLIDER_W;
const DROP_READOUT_H: f32 = 36.0;
const DROP_CHROME: Color = PANEL;
const AXIS_LABEL_Y: f32 = GRAPH_Y + GRAPH_H + 32.0;
const DROP_READOUT_TEXT_Y: f32 = DROP_READOUT_H * 0.5 + 5.0;
const DROP_TIME_READOUT: (f32, f32, f32, f32) = (
    GRAPH_X + GRAPH_W - 46.0,
    AXIS_LABEL_Y - DROP_READOUT_TEXT_Y,
    DROP_READOUT_W,
    DROP_READOUT_H,
);

const GRAPH_OVERLAY_MARGIN: f32 = 14.0;
const STOP_SIZE: f32 = 68.0;
const STOP_BUTTON: (f32, f32, f32, f32) = (
    GRAPH_X + GRAPH_OVERLAY_MARGIN,
    GRAPH_Y + GRAPH_H - GRAPH_OVERLAY_MARGIN - STOP_SIZE,
    STOP_SIZE,
    STOP_SIZE,
);

const MIDI_SLOT_W: f32 = 136.0;

const TRIGGER_GAP: f32 = 10.0;
const TRIGGER_W: f32 = 80.0;
const TRIGGER_BYPASS: f32 = 22.0;
const COG_SIZE: f32 = 22.0;
const AXIS_GAP: f32 = 8.0;
const AXIS_LABEL_W: f32 = 70.0;
const AXIS_VALUE_W: f32 = 56.0;

const CURVE_NODE_HIT: f32 = 16.0;

fn trigger_column() -> (f32, f32, f32, f32) {
    (
        DROP_TIME_SLIDER.0 + DROP_TIME_SLIDER.2 + TRIGGER_GAP,
        DROP_TIME_SLIDER.1,
        DROP_SLIDER_W,
        DROP_TIME_SLIDER.3,
    )
}

fn trigger_title_rect() -> (f32, f32, f32, f32) {
    let column = trigger_column();
    (column.0, column.1 - 18.0, TRIGGER_W, 16.0)
}

fn trigger_bypass_rect() -> (f32, f32, f32, f32) {
    let column = trigger_column();
    (
        column.0 + TRIGGER_W - 6.0 - TRIGGER_BYPASS,
        column.1 + 6.0,
        TRIGGER_BYPASS,
        TRIGGER_BYPASS,
    )
}

fn trigger_value_rect() -> (f32, f32, f32, f32) {
    let column = trigger_column();
    (column.0, DROP_TIME_READOUT.1, TRIGGER_W, DROP_TIME_READOUT.3)
}

fn trigger_bar_rect() -> (f32, f32, f32, f32) {
    let column = trigger_column();
    (
        column.0 + 10.0,
        column.1 + 10.0,
        column.2 - 20.0,
        column.3 - 20.0,
    )
}

fn midi_slot() -> (f32, f32, f32, f32) {
    (GRAPH_X, DROP_TIME_READOUT.1, MIDI_SLOT_W, DROP_READOUT_H)
}

fn midi_label_rect() -> (f32, f32, f32, f32) {
    let slot = midi_slot();
    (slot.0, slot.1, 52.0, slot.3)
}

fn midi_value_rect() -> (f32, f32, f32, f32) {
    let slot = midi_slot();
    let label = midi_label_rect();
    (label.0 + label.2, slot.1, slot.2 - label.2, slot.3)
}

fn cog_rect() -> (f32, f32, f32, f32) {
    let readout = DROP_TIME_READOUT;
    (
        readout.0 - AXIS_GAP - COG_SIZE,
        readout.1 + (readout.3 - COG_SIZE) * 0.5,
        COG_SIZE,
        COG_SIZE,
    )
}

fn axis_slot(index: usize) -> (f32, f32, f32, f32) {
    let cog = cog_rect();
    let midi = midi_slot();
    let x0 = midi.0 + midi.2 + AXIS_GAP;
    let total = cog.0 - AXIS_GAP - x0;
    let width = (total - AXIS_GAP * 2.0) / 3.0;
    (
        x0 + index as f32 * (width + AXIS_GAP),
        DROP_TIME_READOUT.1,
        width,
        DROP_READOUT_H,
    )
}

fn axis_bar_rect(slot: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let x = slot.0 + 8.0 + AXIS_LABEL_W;
    let width = (slot.2 - 16.0 - AXIS_LABEL_W - AXIS_VALUE_W).max(8.0);
    let height = 8.0;
    (x, slot.1 + (slot.3 - height) * 0.5, width, height)
}

fn axis_value_rect(slot: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    (
        slot.0 + slot.2 - 8.0 - AXIS_VALUE_W,
        slot.1,
        AXIS_VALUE_W,
        slot.3,
    )
}

pub fn format_ms(seconds: f32) -> String {
    format!("{:.0}ms", (seconds * 1000.0).round())
}

pub fn snap_drop_time(val: f32) -> f32 {
    let val = val.clamp(0.05, 16.0);
    if val < 1.0 {
        let steps = (val / 0.05).round() as i32;
        ((steps * 5) as f32 / 100.0).clamp(0.05, 1.0)
    } else {
        let steps = (val / 0.25).round() as i32;
        (steps as f32 * 0.25).clamp(1.0, 16.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KnobId {
    DropTime,
    Return,
    Xfade,
    Curve,
    StereoDiv,
    RestartThresh,
    OverrideCc,
    OverrideNote,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum DragState {
    Slider {
        id: KnobId,
        start_x: f32,
        start_y: f32,
        start_norm: f32,
        vertical: bool,
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
    hover_stop: Cell<bool>,
    stop_click: Cell<f32>,
    hover_thresh: Cell<bool>,
    hover: Option<(f32, f32)>,
    curve_hover_anim: Cell<f32>,
    bypass_anim: ButtonAnim,
    auto_restart_anim: ButtonAnim,
    show_axis_controls: bool,
}

impl TapeStopView {
    fn midi_assign(&self) -> MidiAssign {
        self.params.midi_assign.value()
    }

    fn midi_number_id(&self) -> KnobId {
        match self.midi_assign() {
            MidiAssign::Cc => KnobId::OverrideCc,
            MidiAssign::Note => KnobId::OverrideNote,
        }
    }

    fn start_edit(
        &mut self,
        cx: &mut EventContext,
        id: KnobId,
        rect: (f32, f32, f32, f32),
        val_str: String,
    ) {
        cx.focus();
        self.edit = Some(ValueEdit::new(id, rect, val_str));
    }

    fn idle_hover(&self) -> Option<(f32, f32)> {
        pleasant_ui::idle_hover(self.hover, self.drag.is_some())
    }

    fn param(&self, id: KnobId) -> &FloatParam {
        match id {
            KnobId::DropTime => &self.params.drop_time,
            KnobId::Return => &self.params.return_sec,
            KnobId::Xfade => &self.params.xfade_ms,
            KnobId::Curve => &self.params.drop_curve,
            KnobId::StereoDiv => &self.params.stereo_div,
            KnobId::RestartThresh => &self.params.auto_restart_thresh,
            KnobId::OverrideCc | KnobId::OverrideNote => unreachable!("MIDI numbers are IntParams"),
        }
    }

    fn slider_rect(id: KnobId) -> (f32, f32, f32, f32) {
        match id {
            KnobId::DropTime => DROP_TIME_SLIDER,
            KnobId::Return => axis_slot(0),
            KnobId::Xfade => axis_slot(1),
            KnobId::StereoDiv => axis_slot(2),
            KnobId::RestartThresh => trigger_column(),
            _ => (0.0, 0.0, 0.0, 0.0),
        }
    }

    fn drop_bar_rect() -> (f32, f32, f32, f32) {
        let r = DROP_TIME_SLIDER;
        (r.0 + 10.0, r.1 + 10.0, r.2 - 20.0, r.3 - 20.0)
    }

    fn thresh_handle_rect(&self) -> (f32, f32, f32, f32) {
        let bar = trigger_bar_rect();
        let t_norm = self.get_knob_norm(KnobId::RestartThresh);
        let y = bar.1 + bar.3 * (1.0 - t_norm.clamp(0.0, 1.0));
        (bar.0 - 6.0, y - 4.0, bar.2 + 12.0, 8.0)
    }

    fn inside(px: f32, py: f32, rect: (f32, f32, f32, f32)) -> bool {
        px >= rect.0 && px <= rect.0 + rect.2 && py >= rect.1 && py <= rect.1 + rect.3
    }

    fn curve_node_pos() -> (f32, f32) {
        (GRAPH_X + GRAPH_W * 0.5, GRAPH_Y + GRAPH_H * 0.5)
    }

    fn live_playhead(speed: f32, curve_exp: f32) -> (f32, f32) {
        let y_n = 1.0 - speed.clamp(0.0, 1.0);
        let t = inv_s_curve(y_n, curve_exp);
        (GRAPH_X + t * GRAPH_W, GRAPH_Y + y_n * GRAPH_H)
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

    fn reset_knob(&self, cx: &mut EventContext, id: KnobId) {
        match id {
            KnobId::OverrideCc => self.emit_param_norm(
                cx,
                self.params.override_cc.as_ptr(),
                self.params.override_cc.default_normalized_value(),
            ),
            KnobId::OverrideNote => self.emit_param_norm(
                cx,
                self.params.override_note.as_ptr(),
                self.params.override_note.default_normalized_value(),
            ),
            _ => self.emit_knob_norm(cx, id, self.param(id).default_normalized_value()),
        }
    }

    fn emit_knob_norm(&self, cx: &mut EventContext, id: KnobId, norm: f32) {
        if id == KnobId::OverrideCc {
            self.emit_param_norm(cx, self.params.override_cc.as_ptr(), norm);
        } else if id == KnobId::OverrideNote {
            self.emit_param_norm(cx, self.params.override_note.as_ptr(), norm);
        } else if id == KnobId::DropTime {
            let plain = self.params.drop_time.preview_plain(norm);
            let snapped = snap_drop_time(plain);
            let snapped_norm = self.params.drop_time.preview_normalized(snapped);
            self.emit_param_norm(cx, self.params.drop_time.as_ptr(), snapped_norm);
        } else {
            self.emit_param_norm(cx, self.param(id).as_ptr(), norm);
        }
    }

    fn get_knob_norm(&self, id: KnobId) -> f32 {
        if id == KnobId::OverrideCc {
            self.params.override_cc.unmodulated_normalized_value()
        } else if id == KnobId::OverrideNote {
            self.params.override_note.unmodulated_normalized_value()
        } else {
            self.param(id).unmodulated_normalized_value()
        }
    }

    fn knob_info(&self, id: KnobId) -> (&'static str, String, Color) {
        match id {
            KnobId::DropTime => (
                "DROP TIME",
                format_ms(self.params.drop_time.value()),
                COLORS[4],
            ),
            KnobId::Return => (
                "RETURN",
                format_ms(self.params.return_sec.value()),
                COLORS[0],
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
                format!("{:+.1}%", self.params.stereo_div.value()),
                COLORS[3],
            ),
            KnobId::RestartThresh => (
                "THRESHOLD",
                format!("{:.1}dB", self.params.auto_restart_thresh.value()),
                COLORS[2],
            ),
            KnobId::OverrideCc => (
                "CC:",
                format!("{}", self.params.override_cc.value()),
                COLORS[4],
            ),
            KnobId::OverrideNote => (
                "Note:",
                format!("{}", self.params.override_note.value()),
                COLORS[4],
            ),
        }
    }

    fn commit_edit(&mut self, cx: &mut EventContext) {
        if let Some(edit) = self.edit.take() {
            let target = edit.target;
            let text = edit.text;
            let parsed = match target {
                KnobId::DropTime => parse_number_with_units(&text, &[("ms", 1.0), ("s", 1000.0)]),
                KnobId::Return => parse_number_with_units(&text, &[("ms", 1.0), ("s", 1000.0)]),
                KnobId::Xfade => parse_number_with_units(&text, &[("ms", 1.0), ("s", 1000.0)]),
                KnobId::StereoDiv => parse_number_with_units(&text, &[("%", 1.0)]),
                KnobId::RestartThresh => parse_number_with_units(&text, &[("db", 1.0)]),
                KnobId::OverrideCc => parse_number_with_units(&text, &[]),
                KnobId::OverrideNote => parse_number_with_units(&text, &[]),
                KnobId::Curve => parse_number_with_units(&text, &[]),
            };

            if let Some(v) = parsed {
                if target == KnobId::OverrideCc {
                    let norm = self.params.override_cc.preview_normalized(v as i32);
                    self.emit_param_norm(cx, self.params.override_cc.as_ptr(), norm);
                } else if target == KnobId::OverrideNote {
                    let norm = self.params.override_note.preview_normalized(v as i32);
                    self.emit_param_norm(cx, self.params.override_note.as_ptr(), norm);
                } else if target == KnobId::DropTime {
                    let snapped = snap_drop_time(v as f32 / 1000.0);
                    let norm = self.params.drop_time.preview_normalized(snapped);
                    self.emit_param_norm(cx, self.params.drop_time.as_ptr(), norm);
                } else if target == KnobId::Return {
                    let norm = self.params.return_sec.preview_normalized(v as f32 / 1000.0);
                    self.emit_param_norm(cx, self.params.return_sec.as_ptr(), norm);
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
        let prog_l = self.telemetry.brake_progress_l.load(Ordering::Relaxed);
        let prog_r = self.telemetry.brake_progress_r.load(Ordering::Relaxed);
        let speed_l = self.telemetry.speed_l.load(Ordering::Relaxed);
        let speed_r = self.telemetry.speed_r.load(Ordering::Relaxed);

        d.rect(gx, gy, gw, gh, rgb(23, 27, 33));

        // Draw Speed grid as percentages
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
            let pct = ((1.0 - i as f32 / 4.0) * 100.0).round() as i32;
            d.text(gx - 40.0, y + 4.0, &format!("{pct}%"), 10.5, MUTED);
        }
        for i in 0..=5 {
            let x = gx + gw * (i as f32 / 5.0);
            d.line(x, gy, x, gy + gh, LINE, 1.0);
            if i < 5 && !self.show_axis_controls {
                let t = drop_time * i as f32 / 5.0;
                d.text_centered(x, AXIS_LABEL_Y, &format_ms(t), 10.5, MUTED);
            }
        }
        d.text_centered(
            DROP_TIME_SLIDER.0 + DROP_TIME_SLIDER.2 * 0.5,
            gy - 8.0,
            "DROP",
            10.0,
            MUTED,
        );
        d.outline(
            (gx, gy, gw, gh),
            if braking || crossfading { TEAL } else { LINE },
        );

        // High-density curve sampling (2 samples per pixel) to ensure smooth curve without hard corners
        const CURVE_SAMPLES: usize = 1840;
        let mut curve = Vec::with_capacity(CURVE_SAMPLES + 1);
        for i in 0..=CURVE_SAMPLES {
            let t = i as f32 / CURVE_SAMPLES as f32;
            let y = gy + s_curve(t, curve_exp) * gh;
            curve.push((gx + t * gw, y));
        }

        // Grey line when bypassing
        let is_bypassed = self.params.bypass.value();
        let curve_color = if is_bypassed { MUTED } else { GOLD };
        let mut fill = curve_color;
        fill.a = 0.12;
        d.area(&curve, gy + gh, fill);
        d.poly(&curve, curve_color, 2.0);

        let (nx, ny) = Self::curve_node_pos();
        let node_hovered = self
            .idle_hover()
            .is_some_and(|(x, y)| Self::hit_curve_node(x, y))
            || matches!(self.drag, Some(DragState::Curve { .. }));
        let target = if node_hovered { 1.0 } else { 0.0 };
        let previous = self.curve_hover_anim.get();
        let hover = previous + (target - previous) * 0.2;
        self.curve_hover_anim.set(hover);
        let mut node_color = curve_color;
        node_color.r += (TEAL.r - node_color.r) * hover;
        node_color.g += (TEAL.g - node_color.g) * hover;
        node_color.b += (TEAL.b - node_color.b) * hover;
        if hover > 0.01 {
            let mut halo_color = node_color;
            halo_color.a = 0.15 * hover;
            d.circle(nx, ny, 12.0 + 3.0 * hover, halo_color, true);
        }
        d.circle(nx, ny, 8.0 + 2.0 * hover, node_color, false);
        d.circle(nx, ny, 3.5 + hover, node_color, true);

        if let Some(edit) = &self.edit {
            if edit.target == KnobId::Curve {
                d.value_edit(edit, GOLD);
            }
        }

        if braking || speed_l < 0.99 || speed_r < 0.99 {
            let (live_x_l, y_l) = Self::live_playhead(speed_l, curve_exp);
            d.circle(live_x_l, y_l, 6.0, TEAL, false);
            d.circle(live_x_l, y_l, 3.0, TEXT, true);
            if (speed_l - speed_r).abs() > 0.01 || (prog_l - prog_r).abs() > 0.01 {
                let (live_x_r, y_r) = Self::live_playhead(speed_r, curve_exp);
                d.circle(live_x_r, y_r, 6.0, COLORS[4], false);
                d.circle(live_x_r, y_r, 3.0, TEXT, true);
            }
        }

        self.draw_drop_time(d);
    }

    fn drop_l_points() -> [(f32, f32); 6] {
        let s = DROP_TIME_SLIDER;
        let r = DROP_TIME_READOUT;
        [
            (s.0, s.1),
            (s.0 + s.2, s.1),
            (s.0 + s.2, r.1 + r.3),
            (r.0, r.1 + r.3),
            (r.0, r.1),
            (s.0, r.1),
        ]
    }

    fn draw_drop_time(&self, d: &mut Draw) {
        let (_, val_str, color) = self.knob_info(KnobId::DropTime);
        let n = self.get_knob_norm(KnobId::DropTime);
        let readout = DROP_TIME_READOUT;
        let bar = Self::drop_bar_rect();

        d.fill_rounded_poly(&Self::drop_l_points(), 8.0, DROP_CHROME);
        d.stroke_rounded_poly(&Self::drop_l_points(), 8.0, LINE, 1.0);
        d.rect(bar.0, bar.1, bar.2, bar.3, LINE);
        let fill_h = bar.3 * n.clamp(0.0, 1.0);
        if fill_h > 0.5 {
            d.rect(bar.0, bar.1 + bar.3 - fill_h, bar.2, fill_h, color);
        }
        let thumb_y = bar.1 + bar.3 * (1.0 - n.clamp(0.0, 1.0));
        d.rect(bar.0 - 3.0, thumb_y - 3.0, bar.2 + 6.0, 6.0, color);

        if let Some(edit) = &self.edit {
            if edit.target == KnobId::DropTime {
                d.value_edit(edit, color);
                return;
            }
        }
        d.text_centered(
            readout.0 + readout.2 * 0.5,
            readout.1 + DROP_READOUT_TEXT_Y,
            &val_str,
            13.0,
            color,
        );
        if self.edit.is_none() {
            if let Some((hx, hy)) = self.idle_hover() {
                if Self::inside(hx, hy, readout) {
                    d.value_underline(readout, color);
                }
            }
        }
    }

    fn draw_audio_trigger(&self, d: &mut Draw) {
        let column = trigger_column();
        let auto_on = self.params.auto_restart.value();
        let flash = self.telemetry.transient_flash.load(Ordering::Relaxed);
        let (_, t_val, t_color) = self.knob_info(KnobId::RestartThresh);
        d.rounded_rect(column.0, column.1, column.2, column.3, 8.0, DROP_CHROME);
        let outline = if auto_on && flash > 0.05 {
            COLORS[5]
        } else if auto_on {
            t_color
        } else {
            LINE
        };
        d.outline_rounded(column.0, column.1, column.2, column.3, 8.0, outline, 1.0);

        let title = trigger_title_rect();
        let title_color = if !auto_on {
            MUTED
        } else if flash > 0.05 {
            COLORS[5]
        } else {
            t_color
        };
        d.text(title.0, title.1 + 12.0, "AUTO TRIGGER", 10.0, title_color);

        let bypass_r = trigger_bypass_rect();
        let bypass_hovered = self
            .idle_hover()
            .is_some_and(|(hx, hy)| Self::inside(hx, hy, bypass_r));
        let bypass_click = self.auto_restart_anim.step();
        d.bypass_button(bypass_r, !auto_on, t_color, bypass_hovered, bypass_click);

        let bar = trigger_bar_rect();
        d.rect(bar.0, bar.1, bar.2, bar.3, LINE);
        let peak_n = Self::peak_norm(self.telemetry.input_peak.load(Ordering::Relaxed));
        if auto_on && peak_n > 0.01 {
            let fill_h = bar.3 * peak_n;
            d.rect(bar.0, bar.1 + bar.3 - fill_h, bar.2, fill_h, TEAL);
        }

        let handle = self.thresh_handle_rect();
        let handle_hovered = self.hover_thresh.get()
            || matches!(
                self.drag,
                Some(DragState::Slider {
                    id: KnobId::RestartThresh,
                    ..
                })
            );
        let handle_col = if !auto_on {
            MUTED
        } else if handle_hovered {
            TEXT
        } else {
            t_color
        };
        d.rect(handle.0, handle.1, handle.2, handle.3, handle_col);
        d.grip_lines(handle, handle_col, false);

        let val_r = trigger_value_rect();
        let value_color = if auto_on { t_color } else { MUTED };
        if let Some(edit) = self
            .edit
            .as_ref()
            .filter(|edit| edit.target == KnobId::RestartThresh)
        {
            d.value_edit(edit, value_color);
        } else {
            d.text_centered(
                val_r.0 + val_r.2 * 0.5,
                val_r.1 + val_r.3 * 0.5 + 4.0,
                &t_val,
                11.0,
                value_color,
            );
            if auto_on && self.edit.is_none() {
                if let Some((hx, hy)) = self.idle_hover() {
                    if Self::inside(hx, hy, val_r) {
                        d.value_underline(val_r, t_color);
                    }
                }
            }
        }
    }

    fn draw_midi_slot(&self, d: &mut Draw) {
        let slot = midi_slot();
        let label_r = midi_label_rect();
        let val_r = midi_value_rect();
        let midi_id = self.midi_number_id();
        let (midi_label, midi_val, midi_color) = self.knob_info(midi_id);
        d.rounded_rect(slot.0, slot.1, slot.2, slot.3, 8.0, DROP_CHROME);
        d.outline_rounded(slot.0, slot.1, slot.2, slot.3, 8.0, LINE, 1.0);
        let label_hover = self
            .idle_hover()
            .is_some_and(|(hx, hy)| Self::inside(hx, hy, label_r));
        d.text(
            slot.0 + 10.0,
            slot.1 + slot.3 * 0.5 + 4.0,
            midi_label,
            10.0,
            if label_hover { GOLD } else { MUTED },
        );
        if let Some(edit) = self.edit.as_ref().filter(|edit| edit.target == midi_id) {
            d.value_edit(edit, midi_color);
        } else {
            d.text_centered(
                val_r.0 + val_r.2 * 0.5,
                val_r.1 + val_r.3 * 0.5 + 4.0,
                &midi_val,
                11.0,
                midi_color,
            );
            if self.edit.is_none() {
                if let Some((hx, hy)) = self.idle_hover() {
                    if Self::inside(hx, hy, val_r) {
                        d.value_underline(val_r, midi_color);
                    }
                }
            }
        }
    }

    fn draw_axis_cog(&self, d: &mut Draw) {
        let cog = cog_rect();
        let hovered = self
            .idle_hover()
            .is_some_and(|(hx, hy)| Self::inside(hx, hy, cog));
        let color = if self.show_axis_controls || hovered {
            GOLD
        } else {
            MUTED
        };
        d.cog_icon(cog.0 + cog.2 * 0.5, cog.1 + cog.3 * 0.5, color);
    }

    fn draw_axis_slider(&self, d: &mut Draw, id: KnobId) {
        let slot = Self::slider_rect(id);
        let (label, val_str, color) = self.knob_info(id);
        let n = self.get_knob_norm(id);
        let bar = axis_bar_rect(slot);
        let val_r = axis_value_rect(slot);
        d.rounded_rect(slot.0, slot.1, slot.2, slot.3, 8.0, DROP_CHROME);
        d.outline_rounded(slot.0, slot.1, slot.2, slot.3, 8.0, LINE, 1.0);
        d.text(
            slot.0 + 10.0,
            slot.1 + slot.3 * 0.5 + 4.0,
            label,
            10.0,
            MUTED,
        );
        d.rect(bar.0, bar.1, bar.2, bar.3, LINE);
        if id == KnobId::StereoDiv {
            let mid = bar.0 + bar.2 * 0.5;
            let x = bar.0 + bar.2 * n.clamp(0.0, 1.0);
            let (fill_x, fill_w) = if x >= mid {
                (mid, x - mid)
            } else {
                (x, mid - x)
            };
            if fill_w > 0.5 {
                d.rect(fill_x, bar.1, fill_w, bar.3, color);
            }
        } else {
            let fill_w = bar.2 * n.clamp(0.0, 1.0);
            if fill_w > 0.5 {
                d.rect(bar.0, bar.1, fill_w, bar.3, color);
            }
        }
        let thumb_x = bar.0 + bar.2 * n.clamp(0.0, 1.0);
        d.rect(thumb_x - 3.0, bar.1 - 3.0, 6.0, bar.3 + 6.0, color);

        if let Some(edit) = self.edit.as_ref().filter(|edit| edit.target == id) {
            d.value_edit(edit, color);
        } else {
            d.text_centered(
                val_r.0 + val_r.2 * 0.5,
                val_r.1 + val_r.3 * 0.5 + 4.0,
                &val_str,
                11.0,
                color,
            );
            if self.edit.is_none() {
                if let Some((hx, hy)) = self.idle_hover() {
                    if Self::inside(hx, hy, val_r) {
                        d.value_underline(val_r, color);
                    }
                }
            }
        }
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
                    WindowEvent::KeyDown(code, key) => {
                        match code {
                            Code::Enter | Code::NumpadEnter => {
                                self.commit_edit(cx);
                            }
                            Code::Escape => {
                                self.edit = None;
                            }
                            _ => {
                                self.edit.as_mut().unwrap().handle_key(cx, *code);
                                let has_char = matches!(key, Some(Key::Character(_)));
                                if !has_char && !cx.modifiers().command() {
                                    if let Some(c) = typed_char(*code, cx.modifiers().shift()) {
                                        self.edit.as_mut().unwrap().insert(&c.to_string());
                                    }
                                }
                            }
                        }
                        meta.consume();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::MouseDown(MouseButton::Left) => {
                        let edit_rect = self.edit.as_ref().unwrap().rect;
                        if Self::inside(mouse_x, mouse_y, edit_rect) {
                            self.edit.as_mut().unwrap().handle_mouse_down(mouse_x);
                            meta.consume();
                            cx.needs_redraw();
                            return;
                        }
                        self.commit_edit(cx);
                        meta.consume();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                        self.edit.as_mut().unwrap().select_all();
                        meta.consume();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::FocusOut => {
                        self.commit_edit(cx);
                    }
                    _ => return,
                }
            }

            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    // Circular STOP / REW overlay at the bottom-left of the graph
                    let (sx, sy, sw, sh) = STOP_BUTTON;
                    let scx = sx + sw * 0.5;
                    let scy = sy + sh * 0.5;
                    let dx = mouse_x - scx;
                    let dy = mouse_y - scy;
                    if dx * dx + dy * dy <= (sw * 0.5) * (sw * 0.5) {
                        let is_braking = self.telemetry.is_braking.load(Ordering::Relaxed);
                        self.telemetry.set_manual_trigger(!is_braking);
                        self.stop_click.set(1.0);
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, THEME_BUTTON) {
                        prefs().toggle();
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, BYPASS_BUTTON) {
                        let next = if self.params.bypass.value() { 0.0 } else { 1.0 };
                        self.emit_param_norm(cx, self.params.bypass.as_ptr(), next);
                        self.bypass_anim.trigger_click();
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, cog_rect()) {
                        self.show_axis_controls = !self.show_axis_controls;
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, trigger_bypass_rect()) {
                        let next = if self.params.auto_restart.value() {
                            0.0
                        } else {
                            1.0
                        };
                        self.emit_param_norm(cx, self.params.auto_restart.as_ptr(), next);
                        self.auto_restart_anim.trigger_click();
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, DROP_TIME_READOUT) {
                        let (_, val_str, _) = self.knob_info(KnobId::DropTime);
                        self.start_edit(cx, KnobId::DropTime, DROP_TIME_READOUT, val_str);
                        cx.needs_redraw();
                        return;
                    }

                    if Self::inside(mouse_x, mouse_y, DROP_TIME_SLIDER) {
                        let bar_r = Self::drop_bar_rect();
                        let new_norm = (1.0 - (mouse_y - bar_r.1) / bar_r.3).clamp(0.0, 1.0);
                        self.emit_knob_norm(cx, KnobId::DropTime, new_norm);
                        self.drag = Some(DragState::Slider {
                            id: KnobId::DropTime,
                            start_x: mouse_x,
                            start_y: mouse_y,
                            start_norm: new_norm,
                            vertical: true,
                        });
                        cx.needs_redraw();
                        return;
                    }

                    if self.show_axis_controls {
                        if Self::inside(mouse_x, mouse_y, midi_label_rect()) {
                            let next = match self.midi_assign() {
                                MidiAssign::Cc => 1.0,
                                MidiAssign::Note => 0.0,
                            };
                            self.emit_param_norm(cx, self.params.midi_assign.as_ptr(), next);
                            cx.needs_redraw();
                            return;
                        }
                        if Self::inside(mouse_x, mouse_y, midi_value_rect()) {
                            let id = self.midi_number_id();
                            let (_, val_str, _) = self.knob_info(id);
                            self.start_edit(cx, id, midi_value_rect(), val_str);
                            cx.needs_redraw();
                            return;
                        }
                        for &id in &[KnobId::Return, KnobId::Xfade, KnobId::StereoDiv] {
                            let r = Self::slider_rect(id);
                            let val_r = axis_value_rect(r);
                            let bar_r = axis_bar_rect(r);
                            if Self::inside(mouse_x, mouse_y, val_r) {
                                let (_, val_str, _) = self.knob_info(id);
                                self.start_edit(cx, id, val_r, val_str);
                                cx.needs_redraw();
                                return;
                            }
                            if Self::inside(mouse_x, mouse_y, r) {
                                let new_norm = ((mouse_x - bar_r.0) / bar_r.2).clamp(0.0, 1.0);
                                self.emit_knob_norm(cx, id, new_norm);
                                self.drag = Some(DragState::Slider {
                                    id,
                                    start_x: mouse_x,
                                    start_y: mouse_y,
                                    start_norm: new_norm,
                                    vertical: false,
                                });
                                cx.needs_redraw();
                                return;
                            }
                        }
                    }

                    if self.knob_enabled(KnobId::RestartThresh) {
                        let val_r = trigger_value_rect();
                        if Self::inside(mouse_x, mouse_y, val_r) {
                            let (_, val_str, _) = self.knob_info(KnobId::RestartThresh);
                            self.start_edit(cx, KnobId::RestartThresh, val_r, val_str);
                            cx.needs_redraw();
                            return;
                        }
                        if Self::inside(mouse_x, mouse_y, trigger_column())
                            && !Self::inside(mouse_x, mouse_y, trigger_bypass_rect())
                            && !Self::inside(mouse_x, mouse_y, trigger_title_rect())
                        {
                            let bar_r = trigger_bar_rect();
                            let new_norm = (1.0 - (mouse_y - bar_r.1) / bar_r.3).clamp(0.0, 1.0);
                            self.emit_knob_norm(cx, KnobId::RestartThresh, new_norm);
                            self.drag = Some(DragState::Slider {
                                id: KnobId::RestartThresh,
                                start_x: mouse_x,
                                start_y: mouse_y,
                                start_norm: new_norm,
                                vertical: true,
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
                    if Self::hit_curve_node(mouse_x, mouse_y) {
                        self.reset_knob(cx, KnobId::Curve);
                        self.drag = None;
                        cx.needs_redraw();
                        return;
                    }
                    if Self::inside(mouse_x, mouse_y, DROP_TIME_SLIDER) {
                        self.reset_knob(cx, KnobId::DropTime);
                        self.drag = None;
                        cx.needs_redraw();
                        return;
                    }
                    if self.show_axis_controls {
                        for &id in &[KnobId::Return, KnobId::Xfade, KnobId::StereoDiv] {
                            let r = Self::slider_rect(id);
                            let val_r = axis_value_rect(r);
                            if Self::inside(mouse_x, mouse_y, val_r) {
                                continue;
                            }
                            if Self::inside(mouse_x, mouse_y, r) {
                                self.reset_knob(cx, id);
                                self.drag = None;
                                cx.needs_redraw();
                                return;
                            }
                        }
                    }
                    if self.knob_enabled(KnobId::RestartThresh)
                        && Self::inside(mouse_x, mouse_y, trigger_column())
                        && !Self::inside(mouse_x, mouse_y, trigger_value_rect())
                        && !Self::inside(mouse_x, mouse_y, trigger_bypass_rect())
                    {
                        self.reset_knob(cx, KnobId::RestartThresh);
                        self.drag = None;
                        cx.needs_redraw();
                    }
                }

                WindowEvent::MouseUp(MouseButton::Left) => {
                    self.drag = None;
                    cx.needs_redraw();
                }

                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some((mouse_x, mouse_y));
                    cx.needs_redraw();

                    if self.drag.is_none() {
                        let (sx, sy, sw, sh) = STOP_BUTTON;
                        let scx = sx + sw * 0.5;
                        let scy = sy + sh * 0.5;
                        let dx = mouse_x - scx;
                        let dy = mouse_y - scy;
                        let hover_stop = dx * dx + dy * dy <= (sw * 0.5 + 4.0) * (sw * 0.5 + 4.0);
                        if hover_stop != self.hover_stop.get() {
                            self.hover_stop.set(hover_stop);
                            cx.needs_redraw();
                        }

                        let handle_hit = {
                            let h = self.thresh_handle_rect();
                            (h.0 - 4.0, h.1 - 2.0, h.2 + 8.0, h.3 + 4.0)
                        };
                        let hover_thresh = Self::inside(mouse_x, mouse_y, handle_hit);
                        if hover_thresh != self.hover_thresh.get() {
                            self.hover_thresh.set(hover_thresh);
                            cx.needs_redraw();
                        }
                    }

                    match self.drag {
                        Some(DragState::Slider {
                            id,
                            start_x,
                            start_y,
                            start_norm,
                            vertical,
                        }) => {
                            let bar_r = match id {
                                KnobId::DropTime => Self::drop_bar_rect(),
                                KnobId::RestartThresh => trigger_bar_rect(),
                                _ => axis_bar_rect(Self::slider_rect(id)),
                            };
                            let (delta, span) = if vertical {
                                (start_y - mouse_y, bar_r.3)
                            } else {
                                (mouse_x - start_x, bar_r.2)
                            };
                            let step = if cx.modifiers().shift() {
                                0.001
                            } else {
                                1.0 / span.max(1.0)
                            };
                            let new_norm = (start_norm + delta * step).clamp(0.0, 1.0);
                            self.emit_knob_norm(cx, id, new_norm);
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
                    }
                }

                WindowEvent::MouseLeave => {
                    self.hover = None;
                    self.hover_stop.set(false);
                    self.hover_thresh.set(false);
                    cx.needs_redraw();
                }

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
        let braking = self.telemetry.is_braking.load(Ordering::Relaxed);

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
        let bypass_hovered = self
            .idle_hover()
            .is_some_and(|(hx, hy)| Self::inside(hx, hy, BYPASS_BUTTON));
        let bypass_click = self.bypass_anim.step();
        d.bypass_button(BYPASS_BUTTON, bypassed, TEAL, bypass_hovered, bypass_click);

        self.draw_graph(&mut d);

        // Circular STOP / REW overlay in the bottom-left of the graph
        let (sx, sy, sw, sh) = STOP_BUTTON;
        let scx = sx + sw * 0.5;
        let scy = sy + sh * 0.5;
        let stop_active = braking || self.telemetry.get_manual_trigger();
        let stop_color = if stop_active { GOLD } else { MUTED };

        let click_amt = self.stop_click.get();
        if click_amt > 0.01 {
            self.stop_click.set((click_amt - 0.08).max(0.0));
        }
        let is_hovered = self.hover_stop.get();
        let now_sec = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            % 100_000) as f32
            * 0.001;
        let pulse = if is_hovered {
            (now_sec * 5.0).sin() * 0.5 + 0.5
        } else {
            0.0
        };

        if is_hovered || click_amt > 0.01 {
            let halo_r = sw * 0.5 + 2.0 + pulse * 3.0 + click_amt * 6.0;
            let mut halo_col = stop_color;
            halo_col.a = (0.25 + 0.25 * pulse + click_amt * 0.4).clamp(0.0, 1.0);
            d.circle(scx, scy, halo_r, halo_col, false);
        }
        if click_amt > 0.05 {
            let rip_r = sw * 0.5 + (1.0 - click_amt) * 14.0;
            let mut rip_col = stop_color;
            rip_col.a = (click_amt * 0.8).clamp(0.0, 1.0);
            d.circle(scx, scy, rip_r, rip_col, false);
        }

        let mut stop_fill = rgb(0, 0, 0);
        stop_fill.a = 0.5;
        d.circle(scx, scy, sw * 0.5, stop_fill, true);
        d.circle(scx, scy, sw * 0.5, stop_color, false);
        d.text_centered(
            scx,
            scy + 6.0,
            if stop_active { "REW" } else { "STOP" },
            16.0,
            if stop_active { GOLD } else { TEXT },
        );

        self.draw_audio_trigger(&mut d);
        self.draw_axis_cog(&mut d);
        if self.show_axis_controls {
            self.draw_midi_slot(&mut d);
            for &id in &[KnobId::Return, KnobId::Xfade, KnobId::StereoDiv] {
                self.draw_axis_slider(&mut d, id);
            }
        }
    }
}

pub fn default_editor_state() -> Arc<ViziaState> {
    ViziaState::new_screen_sized(|| (UI_W as u32, UI_H as u32))
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
                hover_stop: Cell::new(false),
                stop_click: Cell::new(0.0),
                hover_thresh: Cell::new(false),
                hover: None,
                curve_hover_anim: Cell::new(0.0),
                bypass_anim: ButtonAnim::new(),
                auto_restart_anim: ButtonAnim::new(),
                show_axis_controls: false,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_drop_time() {
        assert_eq!(snap_drop_time(0.01), 0.05);
        assert_eq!(snap_drop_time(0.05), 0.05);
        assert_eq!(snap_drop_time(0.07), 0.05);
        assert_eq!(snap_drop_time(0.08), 0.10);
        assert_eq!(snap_drop_time(0.50), 0.50);
        assert_eq!(snap_drop_time(0.99), 1.00);
        assert_eq!(snap_drop_time(1.00), 1.00);
        assert_eq!(snap_drop_time(1.10), 1.00);
        assert_eq!(snap_drop_time(1.13), 1.25);
        assert_eq!(snap_drop_time(1.25), 1.25);
        assert_eq!(snap_drop_time(2.40), 2.50);
        assert_eq!(snap_drop_time(16.0), 16.0);
        assert_eq!(snap_drop_time(20.0), 16.0);
    }

    #[test]
    fn test_stop_and_midi_overlay_layout() {
        assert_eq!(STOP_BUTTON.2, 68.0);
        assert_eq!(STOP_BUTTON.3, 68.0);
        assert!(STOP_BUTTON.0 >= GRAPH_X);
        assert!(STOP_BUTTON.1 >= GRAPH_Y);
        assert!(STOP_BUTTON.0 + STOP_BUTTON.2 <= GRAPH_X + GRAPH_W);
        assert!(STOP_BUTTON.1 + STOP_BUTTON.3 <= GRAPH_Y + GRAPH_H);
        let midi = midi_slot();
        let ret = axis_slot(0);
        assert!((midi.1 - ret.1).abs() < f32::EPSILON);
        assert!((midi.3 - ret.3).abs() < f32::EPSILON);
        assert!(midi.0 + midi.2 <= ret.0);
        assert!(midi.1 >= GRAPH_Y + GRAPH_H);
        assert!(midi_value_rect().0 >= midi_label_rect().0 + midi_label_rect().2 - 0.5);
    }

    #[test]
    fn test_drop_time_slider_is_right_of_graph() {
        assert_eq!(GRAPH_W + DROP_SLIDER_GAP + DROP_SLIDER_W, 902.0);
        assert!(DROP_TIME_SLIDER.0 >= GRAPH_X + GRAPH_W);
        assert!(DROP_TIME_SLIDER.0 + DROP_TIME_SLIDER.2 <= UI_W);
        assert!(DROP_TIME_SLIDER.3 > GRAPH_H);
        let bar = TapeStopView::drop_bar_rect();
        assert!((bar.1 + bar.3 - (GRAPH_Y + GRAPH_H)).abs() < f32::EPSILON);
        assert!(DROP_TIME_READOUT.1 >= GRAPH_Y + GRAPH_H);
        assert!((DROP_TIME_READOUT.1 + DROP_READOUT_TEXT_Y - AXIS_LABEL_Y).abs() < f32::EPSILON);
        let trigger = trigger_column();
        assert!(trigger.0 >= DROP_TIME_SLIDER.0 + DROP_TIME_SLIDER.2);
        assert!((trigger.1 - DROP_TIME_SLIDER.1).abs() < f32::EPSILON);
        assert!((trigger.2 - DROP_TIME_SLIDER.2).abs() < f32::EPSILON);
        assert!((trigger.3 - DROP_TIME_SLIDER.3).abs() < f32::EPSILON);
        assert!(trigger.0 + trigger.2 <= UI_W);
        let drop_bar = TapeStopView::drop_bar_rect();
        let trig_bar = trigger_bar_rect();
        assert!((trig_bar.1 - drop_bar.1).abs() < f32::EPSILON);
        assert!((trig_bar.2 - drop_bar.2).abs() < f32::EPSILON);
        assert!((trig_bar.3 - drop_bar.3).abs() < f32::EPSILON);
        let title = trigger_title_rect();
        assert!(title.1 + title.3 <= trigger.1);
        let bypass = trigger_bypass_rect();
        assert!((bypass.1 - (trigger.1 + 6.0)).abs() < f32::EPSILON);
        assert!(
            (bypass.0 - (trigger.0 + TRIGGER_W - 6.0 - TRIGGER_BYPASS)).abs() < f32::EPSILON
        );
        assert!(bypass.0 + bypass.2 <= UI_W);
        let value = trigger_value_rect();
        assert!((value.1 - DROP_TIME_READOUT.1).abs() < f32::EPSILON);
        assert!((value.3 - DROP_TIME_READOUT.3).abs() < f32::EPSILON);
        assert!((value.0 - trigger.0).abs() < f32::EPSILON);
        let cog = cog_rect();
        assert!(cog.0 + cog.2 <= DROP_TIME_READOUT.0);
        assert!(
            (cog.1 + cog.3 * 0.5 - (DROP_TIME_READOUT.1 + DROP_TIME_READOUT.3 * 0.5)).abs() < 1.0
        );
        let ret = axis_slot(0);
        let xfade = axis_slot(1);
        let stereo = axis_slot(2);
        assert!(midi_slot().0 + midi_slot().2 <= ret.0);
        assert!((ret.3 - DROP_READOUT_H).abs() < f32::EPSILON);
        assert!((ret.1 - DROP_TIME_READOUT.1).abs() < f32::EPSILON);
        assert!(ret.0 + ret.2 <= xfade.0);
        assert!(xfade.0 + xfade.2 <= stereo.0);
        assert!(stereo.0 + stereo.2 <= cog.0);
        assert!(axis_bar_rect(ret).2 > 8.0);
        let pts = TapeStopView::drop_l_points();
        let s = DROP_TIME_SLIDER;
        let r = DROP_TIME_READOUT;
        assert_eq!(pts[0], (s.0, s.1));
        assert_eq!(pts[1], (s.0 + s.2, s.1));
        assert_eq!(pts[2], (s.0 + s.2, r.1 + r.3));
        assert_eq!(pts[3], (r.0, r.1 + r.3));
        assert_eq!(pts[4], (r.0, r.1));
        assert_eq!(pts[5], (s.0, r.1));
    }

    #[test]
    fn test_format_ms() {
        assert_eq!(format_ms(0.0), "0ms");
        assert_eq!(format_ms(0.05), "50ms");
        assert_eq!(format_ms(0.50), "500ms");
        assert_eq!(format_ms(1.25), "1250ms");
        assert_eq!(format_ms(16.0), "16000ms");
    }
}
