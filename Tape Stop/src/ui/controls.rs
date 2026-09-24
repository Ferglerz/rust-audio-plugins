use super::{
    graph::{GRAPH_H, GRAPH_W, GRAPH_X, GRAPH_Y},
    *,
};

const THEME_BUTTON_W: f32 = 72.0;
/// Right edge of the auto-trigger module (theme button aligns here).
pub(super) const MODULE_RIGHT: f32 =
    GRAPH_X + GRAPH_W + DROP_SLIDER_GAP + DROP_SLIDER_W + TRIGGER_GAP + TRIGGER_W;
pub(super) const THEME_BUTTON: (f32, f32, f32, f32) = (
    MODULE_RIGHT - THEME_BUTTON_W,
    22.0,
    THEME_BUTTON_W,
    26.0,
);

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

const MIDI_SLOT_W: f32 = 96.0;

/// Horizontal gap between DROP slider and AUTO TRIGGER.
pub(super) const TRIGGER_GAP: f32 = 10.0;
pub(super) const TRIGGER_W: f32 = 80.0;
pub(super) const TRIGGER_BYPASS: f32 = 22.0;
const COG_SIZE: f32 = 22.0;
const AXIS_GAP: f32 = 8.0;
const AXIS_LABEL_W: f32 = 66.0;
const AXIS_VALUE_W: f32 = 52.0;

pub(super) fn trigger_column() -> (f32, f32, f32, f32) {
    (
        DROP_TIME_SLIDER.0 + DROP_TIME_SLIDER.2 + TRIGGER_GAP,
        DROP_TIME_SLIDER.1,
        DROP_SLIDER_W,
        DROP_TIME_SLIDER.3,
    )
}

pub(super) fn trigger_title_rect() -> (f32, f32, f32, f32) {
    let column = trigger_column();
    (column.0, column.1 - 18.0, TRIGGER_W, 16.0)
}

pub(super) fn trigger_chrome_rect() -> (f32, f32, f32, f32) {
    let column = trigger_column();
    let value = trigger_value_rect();
    (
        column.0,
        column.1,
        TRIGGER_W,
        value.1 + value.3 - column.1,
    )
}

pub(super) fn trigger_bypass_rect() -> (f32, f32, f32, f32) {
    let column = trigger_column();
    (
        column.0 + DROP_SLIDER_W + 4.0,
        column.1 + 6.0,
        TRIGGER_BYPASS,
        TRIGGER_BYPASS,
    )
}

pub(super) fn trigger_value_rect() -> (f32, f32, f32, f32) {
    let column = trigger_column();
    (
        column.0,
        DROP_TIME_READOUT.1,
        TRIGGER_W,
        DROP_TIME_READOUT.3,
    )
}

pub(super) fn trigger_bar_rect() -> (f32, f32, f32, f32) {
    let column = trigger_column();
    (
        column.0 + 10.0,
        column.1 + 10.0,
        column.2 - 20.0,
        column.3 - 20.0,
    )
}

pub(super) fn midi_slot() -> (f32, f32, f32, f32) {
    (GRAPH_X, DROP_TIME_READOUT.1, MIDI_SLOT_W, DROP_READOUT_H)
}

pub(super) fn midi_label_rect() -> (f32, f32, f32, f32) {
    let slot = midi_slot();
    (slot.0, slot.1, 46.0, slot.3)
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

pub(super) fn axis_bar_rect(slot: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let x = slot.0 + 8.0 + AXIS_LABEL_W;
    let width = (slot.2 - 16.0 - AXIS_LABEL_W - AXIS_VALUE_W).max(8.0);
    let height = 8.0;
    (x, slot.1 + (slot.3 - height) * 0.5, width, height)
}

pub(super) fn axis_value_rect(slot: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    (
        slot.0 + slot.2 - 8.0 - AXIS_VALUE_W,
        slot.1,
        AXIS_VALUE_W,
        slot.3,
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
        if Self::inside(x, y, DROP_TIME_READOUT) { return Some(KnobId::DropTime); }
        if self.knob_enabled(KnobId::RestartThresh) && Self::inside(x, y, trigger_value_rect()) { return Some(KnobId::RestartThresh); }
        if self.show_axis_controls {
            if Self::inside(x, y, midi_value_rect()) { return Some(self.midi_number_id()); }
            return [KnobId::Return, KnobId::Xfade, KnobId::StereoDiv].into_iter()
                .find(|id| Self::inside(x, y, axis_value_rect(Self::slider_rect(*id))));
        }
        None
    }

    pub(super) fn press_value(
        &mut self, cx: &mut EventContext, target: KnobId,
        rect: (f32, f32, f32, f32), origin: (f32, f32),
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

    pub(super) fn thresh_handle_rect(&self) -> (f32, f32, f32, f32) {
        let bar = trigger_bar_rect();
        let t_norm = self.get_knob_norm(KnobId::RestartThresh);
        let y = bar.1 + bar.3 * (1.0 - t_norm.clamp(0.0, 1.0));
        (bar.0 - 5.0, y - 8.0, bar.2 + 10.0, 16.0)
    }

    pub(super) fn inside(px: f32, py: f32, rect: (f32, f32, f32, f32)) -> bool {
        px >= rect.0 && px <= rect.0 + rect.2 && py >= rect.1 && py <= rect.1 + rect.3
    }

    pub(super) fn knob_enabled(&self, id: KnobId) -> bool {
        id != KnobId::RestartThresh || self.params.auto_restart.value()
    }

    pub(super) fn emit_param_norm(&self, cx: &mut EventContext, ptr: ParamPtr, norm: f32) {
        let norm = norm.clamp(0.0, 1.0);
        cx.emit(RawParamEvent::BeginSetParameter(ptr));
        cx.emit(RawParamEvent::SetParameterNormalized(ptr, norm));
        cx.emit(RawParamEvent::EndSetParameter(ptr));
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

    fn peak_norm(peak: f32) -> f32 {
        let db = linear_to_db(peak as f64);
        ((db + 60.0) / 60.0).clamp(0.0, 1.0) as f32
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

    pub(super) fn draw_audio_trigger(&self, d: &mut Draw) {
        let auto_on = self.params.auto_restart.value();
        let flash = self.telemetry.transient_flash.load(Ordering::Relaxed);
        let (_, t_val, t_color) = self.knob_info(KnobId::RestartThresh);
        let chrome = trigger_chrome_rect();
        d.rounded_rect(chrome.0, chrome.1, chrome.2, chrome.3, 8.0, DROP_CHROME);
        d.outline_rounded(chrome.0, chrome.1, chrome.2, chrome.3, 8.0, LINE, 1.0);

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
        d.grab_bar(handle, handle_col);

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

    pub(super) fn draw_midi_slot(&self, d: &mut Draw) {
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
