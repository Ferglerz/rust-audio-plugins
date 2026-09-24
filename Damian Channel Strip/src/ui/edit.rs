use super::*;

impl StripView {
    pub(super) fn set_graph_range(&mut self, range: f64) {
        self.graph_db = display_range(range);
        *self.params.graph_range.lock().unwrap() = self.graph_db;
    }
    pub(super) fn value_at(&self, x: f32, y: f32) -> Option<(ValueTarget, (f32, f32, f32, f32))> {
        if let Some(b) = self.selected_lift() {
            if self.hud_visible() {
                for i in 0..3 {
                    let active = match i {
                        1 => true,
                        2 => !(b.shape.is_cut() && b.order == 1),
                        _ => true,
                    };
                    let r = self.hud_value_rect_lift(&b, i);
                    if active && inside(x, y, r) {
                        return Some((ValueTarget::Lift(i), r));
                    }
                }
            }
            for i in 0..5 {
                let r = self.band_value_rect(i);
                if inside(x, y, r) {
                    return Some((ValueTarget::Lift(i + 3), r));
                }
            }
        }
        if let Some(b) = self.selected.and_then(|id| self.find_band(id)) {
            if self.hud_visible()
                && (self.band_dyn_anim_progress.get() - self.band_dyn_anim_target.get()).abs()
                    <= 0.01
            {
                if self.band_dyn_page.get() {
                    if band_allows_dyn(&b) {
                        for i in 1..5 {
                            let r = self.hud_dyn_value_rect(&b, i);
                            if inside(x, y, r) {
                                let target = match i {
                                    0 => 3,
                                    1 => 7,
                                    2 => 4,
                                    3 => 5,
                                    _ => 6,
                                };
                                return Some((ValueTarget::Band(target), r));
                            }
                        }
                    }
                } else {
                    for i in 0..3 {
                        let active = match i {
                            1 => b.shape.has_gain(),
                            2 => !(b.shape.is_cut() && b.order == 1),
                            _ => true,
                        };
                        let r = self.hud_value_rect(&b, i);
                        if active && inside(x, y, r) {
                            return Some((ValueTarget::Band(i), r));
                        }
                    }
                }
            }
        }
        self.global_value_hits()
            .into_iter()
            .find(|(_, r)| inside(x, y, *r))
    }
    pub(super) fn adjust_value(&self, cx: &mut EventContext, target: ValueTarget, delta: f32) {
        let d = delta as f64;
        match target {
            ValueTarget::Global(i) => {
                let p = self.param(i);
                let sign = if i == 0 || i == 18 { -1.0 } else { 1.0 };
                let norm = (p.unmodulated_normalized_value() + delta * sign).clamp(0.0, 1.0);
                cx.emit(RawParamEvent::BeginSetParameter(p.as_ptr()));
                cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                cx.emit(RawParamEvent::EndSetParameter(p.as_ptr()));
            }
            ValueTarget::Band(i) => self.change(|b| adjust_band_value(b, i, d)),
            ValueTarget::Lift(i) => self.change_lift(|b| match i {
                0 => b.freq *= 1000.0_f64.powf(d),
                1 => b.gain += 100.0 * d,
                2 => b.q *= 120.0_f64.powf(d),
                3 => b.threshold += 60.0 * d,
                4 => b.ratio += 19.0 * d,
                5 => b.attack *= 2000.0_f64.powf(d),
                6 => b.release *= 200.0_f64.powf(d),
                _ => b.range += 24.0 * d,
            }),
        }
    }

    pub(super) fn press_value(
        &mut self,
        cx: &mut EventContext,
        target: ValueTarget,
        rect: (f32, f32, f32, f32),
        origin: (f32, f32),
    ) {
        self.value_press = Some(pleasant_ui::pointer::ValuePress::new(target, rect, origin));
        cx.focus();
        cx.capture();
    }

    pub(super) fn handle_value_press(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(mut press) = self.value_press else {
            return false;
        };
        match event {
            WindowEvent::MouseMove(_, _) => {
                if press.update(x, y) {
                    self.value_press = None;
                    self.drag = Some(Target::Value(press.target));
                    self.down = press.origin;
                    self.last_drag = press.origin;
                    // Continue through relative readout dragging.
                    return false;
                }
                self.value_press = Some(press);
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                self.value_press = None;
                cx.release();
                if press.released_as_click(x, y) {
                    self.start_edit(cx, press.target, press.rect);
                }
                cx.needs_redraw();
            }
            WindowEvent::FocusOut
            | WindowEvent::KeyDown(Code::Escape, _)
            | WindowEvent::MouseDown(MouseButton::Right) => {
                self.value_press = None;
                cx.release();
                cx.needs_redraw();
            }
            _ => {}
        }
        true
    }

    pub(super) fn start_edit(
        &mut self,
        cx: &mut EventContext,
        target: ValueTarget,
        rect: (f32, f32, f32, f32),
    ) {
        cx.focus();
        let value = match target {
            ValueTarget::Global(i) => {
                let value = self.param(i).value() as f64;
                if i == 0 || i == 18 {
                    -value
                } else {
                    value
                }
            }
            ValueTarget::Band(i) => {
                let Some(b) = self.selected.and_then(|id| self.find_band(id)) else {
                    return;
                };
                [
                    b.freq,
                    b.gain,
                    b.q,
                    b.threshold,
                    b.ratio,
                    b.attack,
                    b.release,
                    b.range,
                ][i]
            }
            ValueTarget::Lift(i) => {
                let lift_bands = self.params.lift_bands.lock().unwrap();
                let Some(b) = lift_bands.iter().find(|b| Some(b.id) == self.selected) else {
                    return;
                };
                [
                    b.freq,
                    b.gain,
                    b.q,
                    b.threshold,
                    b.ratio,
                    b.attack,
                    b.release,
                    b.range,
                ][i]
            }
        };
        let text = if let ValueTarget::Global(10) = target {
            format_pse_time(self.param(10).value() as f64, self.params.pse_peak.value())
        } else {
            format!("{value:.3}")
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        };
        self.edit = Some(ValueEdit::new(target, rect, text));
        self.menu = None;
        self.scale_menu = false;
    }
    pub(super) fn commit_edit(&mut self, cx: &mut EventContext) -> bool {
        let Some(edit) = self.edit.as_mut() else {
            return true;
        };
        if edit.text == edit.original {
            self.edit = None;
            return true;
        }
        let Some(value) = parse_value(&edit.text, edit.target) else {
            edit.invalid = true;
            return false;
        };
        match edit.target {
            ValueTarget::Global(i) => {
                let p = self.param(i);
                let norm = p.preview_normalized(if i == 0 || i == 18 {
                    -value as f32
                } else {
                    value as f32
                });
                cx.emit(RawParamEvent::BeginSetParameter(p.as_ptr()));
                cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                cx.emit(RawParamEvent::EndSetParameter(p.as_ptr()));
            }
            ValueTarget::Band(i) => self.change(|b| match i {
                0 => b.freq = value,
                1 => b.gain = value,
                2 => b.q = value,
                3 => b.threshold = value,
                4 => b.ratio = value,
                5 => b.attack = value,
                6 => b.release = value,
                _ => b.range = value,
            }),
            ValueTarget::Lift(i) => self.change_lift(|b| match i {
                0 => b.freq = value.clamp(20.0, 20000.0),
                1 => b.gain = value.clamp(-100.0, 0.0),
                2 => b.q = value,
                3 => b.threshold = value,
                4 => b.ratio = value,
                5 => b.attack = value,
                6 => b.release = value,
                _ => b.range = value,
            }),
        }
        self.edit = None;
        true
    }
    pub(super) fn edit_key(&mut self, cx: &mut EventContext, code: Code) {
        if code == Code::Escape {
            self.edit = None;
            return;
        }
        if matches!(code, Code::Enter | Code::NumpadEnter | Code::Tab) {
            let target = self.edit.as_ref().unwrap().target;
            if self.commit_edit(cx) && code == Code::Tab {
                let mut fields = Vec::new();
                if let Some(b) = self.selected_lift() {
                    for i in 0..3 {
                        if i != 2 || !(b.shape.is_cut() && b.order == 1) {
                            fields.push((
                                ValueTarget::Lift(i),
                                hud_value_rect_lift_at(&b, self.graph_db, i, self.gx(), self.gw()),
                            ));
                        }
                    }
                    fields.extend(
                        (0..5).map(|i| (ValueTarget::Lift(i + 3), self.band_value_rect(i))),
                    );
                }
                if let Some(b) = self.selected.and_then(|id| self.find_band(id)) {
                    if self.band_dyn_page.get() {
                        if band_allows_dyn(&b) {
                            for i in 1..5 {
                                let target = match i {
                                    0 => 3,
                                    1 => 7,
                                    2 => 4,
                                    3 => 5,
                                    _ => 6,
                                };
                                fields.push((
                                    ValueTarget::Band(target),
                                    self.hud_dyn_value_rect(&b, i),
                                ));
                            }
                        }
                    } else {
                        for i in 0..3 {
                            if (i != 1 || b.shape.has_gain())
                                && (i != 2 || !(b.shape.is_cut() && b.order == 1))
                            {
                                fields.push((ValueTarget::Band(i), self.hud_value_rect(&b, i)));
                            }
                        }
                    }
                }
                fields.extend(self.global_value_hits());
                let index = fields.iter().position(|(t, _)| *t == target).unwrap_or(0);
                let next = if cx.modifiers().shift() {
                    (index + fields.len() - 1) % fields.len()
                } else {
                    (index + 1) % fields.len()
                };
                self.start_edit(cx, fields[next].0, fields[next].1);
            }
            return;
        }
        let edit = self.edit.as_mut().unwrap();
        let command = cx.modifiers().command();
        match code {
            Code::KeyA if command => {
                edit.anchor = 0;
                edit.cursor = edit.text.len();
            }
            Code::KeyC | Code::KeyX if command => {
                let _ = cx.set_clipboard(edit.text[edit.selection()].to_string());
                if code == Code::KeyX {
                    edit.insert("");
                }
            }
            Code::KeyV if command => {
                if let Ok(text) = cx.get_clipboard() {
                    edit.insert(&text);
                }
            }
            Code::Backspace => edit.erase(true),
            Code::Delete => edit.erase(false),
            Code::ArrowLeft | Code::ArrowRight | Code::Home | Code::End => {
                let selecting = cx.modifiers().shift();
                edit.cursor = match code {
                    Code::Home => 0,
                    Code::End => edit.text.len(),
                    Code::ArrowLeft if !selecting && edit.cursor != edit.anchor => {
                        edit.selection().start
                    }
                    Code::ArrowRight if !selecting && edit.cursor != edit.anchor => {
                        edit.selection().end
                    }
                    Code::ArrowLeft => edit.cursor.saturating_sub(1),
                    _ => (edit.cursor + 1).min(edit.text.len()),
                };
                if !selecting {
                    edit.anchor = edit.cursor;
                }
            }
            _ => {}
        }
    }

    pub(super) fn global_controls(&self) -> &'static [usize] {
        match self.dyn_page.get() {
            DynPage::Main => &[0],
            DynPage::Controls => &[5, 11, 6, 3, 4],
        }
    }
    pub(super) fn param(&self, i: usize) -> &FloatParam {
        match i {
            0 => &self.params.compression,
            1 => &self.params.gate,
            2 => &self.params.pse_voice_det,
            3 => &self.params.dry,
            4 => &self.params.wet,
            5 => &self.params.comp_attack,
            6 => &self.params.comp_ratio,
            7 => &self.params.pse_depth,
            8 => &self.params.pse_hysteresis,
            9 => &self.params.pse_knee,
            10 => &self.params.pse_time,
            11 => &self.params.comp_release,
            12 => &self.params.comp_knee,
            14 => &self.params.comp_depth,
            15 => &self.params.output_gain,
            16 => &self.params.wall_even,
            17 => &self.params.wall_odd,
            18 => &self.params.wall_threshold,
            _ => &self.params.compression,
        }
    }
}

fn adjust_band_value(b: &mut Band, i: usize, delta: f64) {
    match i {
        0 => b.freq *= 1000.0_f64.powf(delta),
        1 => b.gain += 2.0 * crate::band::MAX_GAIN_DB * delta,
        2 => b.q *= 120.0_f64.powf(delta),
        3 => b.threshold += 60.0 * delta,
        4 => b.ratio += 19.0 * delta,
        5 => b.attack *= 2000.0_f64.powf(delta),
        6 => b.release *= 200.0_f64.powf(delta),
        _ => b.range += 2.0 * MAX_RANGE_DB * delta,
    }
}

#[cfg(test)]
mod value_drag_tests {
    use super::*;

    #[test]
    fn readout_adjustments_change_only_the_addressed_parameter() {
        for index in 0..8 {
            let original = Band::default();
            let mut band = original.clone();
            adjust_band_value(&mut band, index, 0.01);
            assert_ne!(band, original);
            if index != 4 {
                assert_eq!(band.ratio, original.ratio);
            }
            if index != 7 {
                assert_eq!(band.range, original.range);
            }
            if index != 1 {
                assert_eq!(band.gain, original.gain);
            }
            adjust_band_value(&mut band, index, -0.01);
            assert!((band.gain - original.gain).abs() < 1e-8);
            assert!((band.freq - original.freq).abs() < 1e-8);
            assert!((band.attack - original.attack).abs() < 1e-8);
        }
    }
}
