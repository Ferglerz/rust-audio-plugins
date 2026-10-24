use super::{controls::*, *};

#[derive(Clone, Copy)]
pub(super) struct PerformanceTick;

impl TapeStopView {
    pub(super) fn division_param(&self, id: KnobId) -> Option<&IntParam> {
        if !self.params.time_sync.value() {
            return None;
        }
        match id {
            KnobId::DropTime => Some(&self.params.drop_division),
            KnobId::Return => Some(&self.params.return_division),
            _ => None,
        }
    }

    pub(super) fn knob_ptr(&self, id: KnobId) -> ParamPtr {
        if let Some(p) = self.division_param(id) {
            return p.as_ptr();
        }
        match id {
            KnobId::OverrideCc => self.params.override_cc.as_ptr(),
            KnobId::OverrideNote => self.params.override_note.as_ptr(),
            _ => self.param(id).as_ptr(),
        }
    }

    pub(super) fn end_drag(&mut self, cx: &mut EventContext) {
        pleasant_ui::param::end_all(cx, &mut self.gesture_params);
        self.drag = None;
        self.pad_binding = None;
        cx.release();
    }

    pub(super) fn begin_gesture(&mut self, cx: &mut EventContext, ids: &[KnobId]) {
        self.end_drag(cx);
        for (i, id) in ids.iter().enumerate() {
            let ptr = self.knob_ptr(*id);
            pleasant_ui::param::begin(cx, ptr);
            self.gesture_params[i] = Some(ptr);
        }
        cx.focus();
        cx.capture();
    }

    pub(super) fn x_id(&self) -> KnobId {
        if self.params.xy_x_axis.value() == XAxis::Curve {
            KnobId::Curve
        } else {
            KnobId::Return
        }
    }

    pub(super) fn pad_values(&self) -> (f32, f32) {
        (
            self.get_knob_norm(self.x_id()),
            self.get_knob_norm(KnobId::DropTime),
        )
    }

    pub(super) fn set_pad(&self, cx: &mut EventContext, value: (f32, f32)) {
        self.emit_knob_norm(cx, self.x_id(), value.0);
        self.emit_knob_norm(cx, KnobId::DropTime, value.1);
    }

    pub(super) fn begin_pad(&mut self, cx: &mut EventContext, pointer: (f32, f32)) {
        let mode = if self.params.xy_mode.value() == PadMode::Absolute {
            pleasant_ui::xy_pad::Interaction::Absolute
        } else {
            pleasant_ui::xy_pad::Interaction::Relative
        };
        let drag = pleasant_ui::xy_pad::Drag::begin(XY_PAD, pointer, self.pad_values(), mode);
        self.begin_gesture(cx, &[self.x_id(), KnobId::DropTime]);
        self.pad_binding = Some((
            self.params.xy_x_axis.value(),
            self.params.xy_mode.value(),
            self.params.time_sync.value(),
        ));
        self.set_pad(cx, drag.value());
        self.drag = Some(DragState::Pad(drag));
    }

    pub(super) fn draw_performance(&self, d: &mut Draw) {
        let sync = self.params.time_sync.value();
        d.button_surface(TIME_SYNC, TEAL, sync as u8 as f32, 0.0, 0.0);
        d.text_centered(
            TIME_SYNC.0 + TIME_SYNC.2 * 0.5,
            TIME_SYNC.1 + 20.0,
            if sync { "Sync" } else { "Free" },
            13.0,
            if sync { TEAL } else { TEXT },
        );
        let host = self.params.tempo_sync.value();
        let bpm = if host {
            self.telemetry.tempo.load(Ordering::Relaxed)
        } else {
            self.params.tempo.value()
        };
        TEMPO_HEADER.draw(
            d,
            host,
            bpm as f64,
            self.params.tempo.unmodulated_normalized_value(),
            self.idle_hover().is_some_and(|(x, y)| {
                TEMPO_HEADER.hit(x, y) == Some(pleasant_ui::tempo::Hit::Source)
            }) as u8 as f32,
        );
        if let Some(edit) = self.edit.as_ref().filter(|e| e.target == KnobId::Tempo) {
            d.value_edit(edit, GOLD);
        }
        d.text(XY_PAD.0, XY_PAD.1 - 8.0, "XY · Y: Drop Time", 11.0, MUTED);
        pleasant_ui::xy_pad::draw(d, XY_PAD, self.pad_values());
        for (rect, label) in [
            (
                XY_AXIS,
                if self.x_id() == KnobId::Curve {
                    "X: Curve"
                } else {
                    "X: Return"
                },
            ),
            (
                XY_MODE,
                if self.params.xy_mode.value() == PadMode::Absolute {
                    "Absolute"
                } else {
                    "Relative"
                },
            ),
        ] {
            d.button_surface(
                rect,
                TEAL,
                0.0,
                self.idle_hover()
                    .is_some_and(|(x, y)| Self::inside(x, y, rect)) as u8 as f32,
                0.0,
            );
            d.text_centered(rect.0 + rect.2 * 0.5, rect.1 + 19.0, label, 12.0, TEAL);
        }
    }
}
