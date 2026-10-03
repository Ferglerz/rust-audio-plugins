use super::{
    graph::{GRAPH_H, GRAPH_W, GRAPH_X, GRAPH_Y},
    *,
};

const THEME_BUTTON_W: f32 = 72.0;
/// Align the theme button with the drop-time module.
pub(super) const MODULE_RIGHT: f32 = GRAPH_X + GRAPH_W + DROP_SLIDER_GAP + DROP_SLIDER_W;
pub(super) const THEME_BUTTON: (f32, f32, f32, f32) =
    (MODULE_RIGHT - THEME_BUTTON_W, 22.0, THEME_BUTTON_W, 26.0);

pub(super) const DROP_SLIDER_GAP: f32 = 10.0;
pub(super) const DROP_SLIDER_W: f32 = 40.0;
const DROP_SLIDER_EXTRA: f32 = 10.0;
pub(super) const DROP_TIME_SLIDER: (f32, f32, f32, f32) = (
    GRAPH_X + GRAPH_W + DROP_SLIDER_GAP,
    GRAPH_Y,
    DROP_SLIDER_W,
    GRAPH_H + DROP_SLIDER_EXTRA,
);
const DROP_READOUT_W: f32 = 46.0 + DROP_SLIDER_GAP + DROP_SLIDER_W;
pub(super) const DROP_READOUT_H: f32 = 36.0;
const DROP_CHROME: Color = PANEL;
pub(super) const AXIS_LABEL_Y: f32 = GRAPH_Y + GRAPH_H + 32.0;
pub(super) const DROP_READOUT_TEXT_Y: f32 = DROP_READOUT_H * 0.5 + 5.0;
pub(super) const DROP_TIME_READOUT: (f32, f32, f32, f32) = (
    GRAPH_X + GRAPH_W - 46.0,
    AXIS_LABEL_Y - DROP_READOUT_TEXT_Y,
    DROP_READOUT_W,
    DROP_READOUT_H,
);

const GRAPH_OVERLAY_MARGIN: f32 = 14.0;
const STOP_SIZE: f32 = 68.0;
pub(super) const STOP_BUTTON: (f32, f32, f32, f32) = (
    GRAPH_X + GRAPH_OVERLAY_MARGIN,
    GRAPH_Y + GRAPH_H - GRAPH_OVERLAY_MARGIN - STOP_SIZE,
    STOP_SIZE,
    STOP_SIZE,
);

const MIDI_SLOT_W: f32 = 66.0;

pub(super) const TRIGGER_BYPASS: f32 = 22.0;
const COG_SIZE: f32 = 22.0;
const AXIS_GAP: f32 = 8.0;
const AXIS_VALUE_W: f32 = 60.0;

pub(super) fn trigger_column() -> (f32, f32, f32, f32) {
    axis_slot(3)
}

pub(super) fn trigger_bypass_rect() -> (f32, f32, f32, f32) {
    let slot = trigger_column();
    (slot.0 + 8.0, slot.1, TRIGGER_BYPASS, TRIGGER_BYPASS)
}

pub(super) fn trigger_value_rect() -> (f32, f32, f32, f32) {
    axis_value_rect(trigger_column())
}

pub(super) fn trigger_bar_rect() -> (f32, f32, f32, f32) {
    axis_bar_rect(trigger_column())
}

pub(super) fn midi_slot() -> (f32, f32, f32, f32) {
    (GRAPH_X, DROP_TIME_READOUT.1, MIDI_SLOT_W, DROP_READOUT_H)
}

pub(super) fn midi_label_rect() -> (f32, f32, f32, f32) {
    let slot = midi_slot();
    (slot.0, slot.1, 42.0, slot.3)
}

pub(super) fn midi_value_rect() -> (f32, f32, f32, f32) {
    let slot = midi_slot();
    let label = midi_label_rect();
    (label.0 + label.2, slot.1, slot.2 - label.2, slot.3)
}

pub(super) fn cog_rect() -> (f32, f32, f32, f32) {
    let readout = DROP_TIME_READOUT;
    (
        readout.0 - AXIS_GAP - COG_SIZE,
        readout.1 + (readout.3 - COG_SIZE) * 0.5,
        COG_SIZE,
        COG_SIZE,
    )
}

pub(super) fn axis_slot(index: usize) -> (f32, f32, f32, f32) {
    let midi = midi_slot();
    let start = midi.0 + midi.2 + AXIS_GAP;
    // Preserve the trigger's extra label space while shortening all four sliders equally.
    let trigger_extra = 40.0;
    let width = (cog_rect().0 - AXIS_GAP - start - AXIS_GAP * 3.0 - trigger_extra) / 4.0;
    (
        start + index as f32 * (width + AXIS_GAP),
        DROP_TIME_READOUT.1,
        if index == 3 {
            width + trigger_extra
        } else {
            width
        },
        44.0,
    )
}

pub(super) fn axis_bar_rect(slot: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    (
        slot.0 + 8.0,
        slot.1 + 34.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
        slot.2 - 16.0,
        5.0,
    )
}

pub(super) fn axis_value_rect(slot: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    (
        slot.0 + (slot.2 - AXIS_VALUE_W) * 0.5,
        slot.1 + 14.0,
        AXIS_VALUE_W,
        20.0,
    )
}

impl TapeStopView {
    pub(super) fn midi_assign(&self) -> MidiAssign {
        self.params.midi_assign.value()
    }

    pub(super) fn midi_number_id(&self) -> KnobId {
        match self.midi_assign() {
            MidiAssign::Cc => KnobId::OverrideCc,
            MidiAssign::Note => KnobId::OverrideNote,
        }
    }

    pub(super) fn readout_at(&self, x: f32, y: f32) -> Option<KnobId> {
        if Self::inside(x, y, DROP_TIME_READOUT) {
            return Some(KnobId::DropTime);
        }
        if self.show_axis_controls
            && self.knob_enabled(KnobId::RestartThresh)
            && Self::inside(x, y, trigger_value_rect())
        {
            return Some(KnobId::RestartThresh);
        }
        if self.show_axis_controls {
            if Self::inside(x, y, midi_value_rect()) {
                return Some(self.midi_number_id());
            }
            return [KnobId::Return, KnobId::Xfade, KnobId::StereoDiv]
                .into_iter()
                .find(|id| Self::inside(x, y, axis_value_rect(Self::slider_rect(*id))));
        }
        None
    }

    pub(super) fn press_value(
        &mut self,
        cx: &mut EventContext,
        target: KnobId,
        rect: (f32, f32, f32, f32),
        origin: (f32, f32),
    ) {
        self.value_press = Some(pleasant_ui::pointer::ValuePress::new(target, rect, origin));
        cx.focus();
        cx.capture();
    }

    pub(super) fn start_edit(
        &mut self,
        cx: &mut EventContext,
        id: KnobId,
        rect: (f32, f32, f32, f32),
        val_str: String,
    ) {
        cx.focus();
        self.edit = Some(ValueEdit::new(id, rect, val_str));
    }

    pub(super) fn idle_hover(&self) -> Option<(f32, f32)> {
        pleasant_ui::idle_hover(self.hover, self.drag.is_some())
    }

    pub(super) fn param(&self, id: KnobId) -> &FloatParam {
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

    pub(super) fn slider_rect(id: KnobId) -> (f32, f32, f32, f32) {
        match id {
            KnobId::DropTime => DROP_TIME_SLIDER,
            KnobId::Return => axis_slot(0),
            KnobId::Xfade => axis_slot(1),
            KnobId::StereoDiv => axis_slot(2),
            KnobId::RestartThresh => trigger_column(),
            _ => (0.0, 0.0, 0.0, 0.0),
        }
    }

    pub(super) fn drop_bar_rect() -> (f32, f32, f32, f32) {
        let r = DROP_TIME_SLIDER;
        (r.0 + 10.0, r.1 + 10.0, r.2 - 20.0, r.3 - 20.0)
    }

    pub(super) fn inside(px: f32, py: f32, rect: (f32, f32, f32, f32)) -> bool {
        px >= rect.0 && px <= rect.0 + rect.2 && py >= rect.1 && py <= rect.1 + rect.3
    }

    pub(super) fn knob_enabled(&self, id: KnobId) -> bool {
        id != KnobId::RestartThresh || self.params.auto_restart.value()
    }

    pub(super) fn emit_param_norm(&self, cx: &mut EventContext, ptr: ParamPtr, norm: f32) {
        pleasant_ui::param::set_normalized_once(cx, ptr, norm);
    }

    pub(super) fn reset_knob(&self, cx: &mut EventContext, id: KnobId) {
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

    pub(super) fn emit_knob_norm(&self, cx: &mut EventContext, id: KnobId, norm: f32) {
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

    pub(super) fn get_knob_norm(&self, id: KnobId) -> f32 {
        if id == KnobId::OverrideCc {
            self.params.override_cc.unmodulated_normalized_value()
        } else if id == KnobId::OverrideNote {
            self.params.override_note.unmodulated_normalized_value()
        } else {
            self.param(id).unmodulated_normalized_value()
        }
    }

    pub(super) fn knob_info(&self, id: KnobId) -> (&'static str, String, Color) {
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
                "AUTO TRIGGER",
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

    pub(super) fn commit_edit(&mut self, cx: &mut EventContext) {
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

    pub(super) fn drop_l_points() -> [(f32, f32); 6] {
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

    pub(super) fn draw_drop_time(&self, d: &mut Draw) {
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
                    d.value_underline(readout, readout.1 + DROP_READOUT_TEXT_Y, color);
                }
            }
        }
    }

    pub(super) fn draw_audio_trigger(&self, d: &mut Draw) {
        self.draw_axis_slider(d, KnobId::RestartThresh);
        let bypass = trigger_bypass_rect();
        let hovered = self
            .idle_hover()
            .is_some_and(|(x, y)| Self::inside(x, y, bypass));
        d.bypass_button(
            bypass,
            !self.params.auto_restart.value(),
            COLORS[2],
            hovered,
            self.auto_restart_anim.step(),
        );
    }

    pub(super) fn draw_midi_slot(&self, d: &mut Draw) {
        let slot = midi_slot();
        let label_r = midi_label_rect();
        let val_r = midi_value_rect();
        let midi_id = self.midi_number_id();
        let (midi_label, midi_val, midi_color) = self.knob_info(midi_id);
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
                        d.value_underline(val_r, val_r.1 + val_r.3 * 0.5 + 4.0, midi_color);
                    }
                }
            }
        }
    }

    pub(super) fn draw_axis_cog(&self, d: &mut Draw) {
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

    pub(super) fn draw_axis_slider(&self, d: &mut Draw, id: KnobId) {
        let slot = Self::slider_rect(id);
        let (label, val_str, accent) = self.knob_info(id);
        let enabled = self.knob_enabled(id);
        let color = if enabled { accent } else { MUTED };
        let n = self.get_knob_norm(id).clamp(0.0, 1.0);
        let bar = axis_bar_rect(slot);
        let val_r = axis_value_rect(slot);
        let baseline = slot.1 + 30.0;
        let label_x = slot.0
            + 8.0
            + if id == KnobId::RestartThresh {
                TRIGGER_BYPASS + 6.0
            } else {
                0.0
            };
        let label_color = if id == KnobId::RestartThresh
            && enabled
            && self.telemetry.transient_flash.load(Ordering::Relaxed) > 0.05
        {
            COLORS[5]
        } else {
            MUTED
        };
        if id == KnobId::RestartThresh {
            d.text(label_x, slot.1 + 12.0, label, 12.0, label_color);
        } else {
            d.text_centered(
                slot.0 + slot.2 * 0.5,
                slot.1 + 12.0,
                label,
                12.0,
                label_color,
            );
        }
        d.rounded_rect(bar.0, bar.1, bar.2, bar.3, 2.5, LINE);
        let (fill_x, fill_w) = if id == KnobId::StereoDiv {
            let mid = bar.0 + bar.2 * 0.5;
            let x = bar.0 + bar.2 * n;
            (x.min(mid), (x - mid).abs())
        } else {
            (bar.0, bar.2 * n)
        };
        if fill_w > 0.5 {
            d.rounded_rect(fill_x, bar.1, fill_w, bar.3, 2.5, color);
        }
        if id == KnobId::StereoDiv {
            d.rect(bar.0 + bar.2 * 0.5 - 0.5, bar.1, 1.0, bar.3, color);
        }
        if let Some(edit) = self.edit.as_ref().filter(|edit| edit.target == id) {
            d.value_edit(edit, color);
        } else {
            d.text_centered(val_r.0 + val_r.2 * 0.5, baseline, &val_str, 12.0, color);
            if enabled && self.edit.is_none() {
                if let Some((hx, hy)) = self.idle_hover() {
                    if Self::inside(hx, hy, val_r) {
                        d.value_underline(val_r, baseline, color);
                    }
                }
            }
        }
    }
}
