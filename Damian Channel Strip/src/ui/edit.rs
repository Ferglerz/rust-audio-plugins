use super::*;

impl StripView {
    pub(super) fn set_graph_range(&mut self, range: f64) {
        self.graph_db = display_range(range);
        *self.params.graph_range.lock().unwrap() = self.graph_db;
    }
    pub(super) fn visible_value_fields(&self) -> Vec<(ValueTarget, (f32, f32, f32, f32))> {
        let mut fields = Vec::new();
        if let Some(b) = self.selected_lift() {
            for i in 0..3 {
                if i != 2 || !(b.shape.is_cut() && b.order == 1) {
                    fields.push((
                        ValueTarget::Lift(BandField::hud(i)),
                        hud_value_rect_lift_at(&b, self.graph_db, i, self.gx(), self.gw()),
                    ));
                }
            }
            fields.extend((0..5).map(|i| {
                (
                    ValueTarget::Lift(BandField::from_lift_slider(i)),
                    self.band_value_rect(i),
                )
            }));
        }
        if let Some(b) = self.selected.and_then(|id| self.find_band(id)) {
            if self.band_dyn_page.get() {
                if band_allows_dyn(&b) {
                    for i in 1..5 {
                        fields.push((
                            ValueTarget::Band(BandField::from_dyn_bar(i)),
                            self.hud_dyn_value_rect(&b, i),
                        ));
                    }
                }
            } else {
                for i in 0..3 {
                    if (i != 1 || b.shape.has_gain())
                        && (i != 2 || !(b.shape.is_cut() && b.order == 1))
                    {
                        fields.push((
                            ValueTarget::Band(BandField::hud(i)),
                            self.hud_value_rect(&b, i),
                        ));
                    }
                }
            }
        }
        fields.extend(self.global_value_hits());
        fields
    }
    pub(super) fn value_at(&self, x: f32, y: f32) -> Option<(ValueTarget, (f32, f32, f32, f32))> {
        let band_settled =
            (self.band_dyn_anim_progress.get() - self.band_dyn_anim_target.get()).abs() <= 0.01;
        self.visible_value_fields().into_iter().find(|(target, rect)| {
            let hidden_band = matches!(target, ValueTarget::Band(_))
                && (!self.hud_visible() || !band_settled);
            let hidden_lift = matches!(
                target,
                ValueTarget::Lift(BandField::Frequency | BandField::Gain | BandField::Q)
            ) && !self.hud_visible();
            !hidden_band && !hidden_lift && inside(x, y, *rect)
        })
    }
    pub(super) fn adjust_value(&self, cx: &mut EventContext, target: ValueTarget, delta: f32) {
        let d = delta as f64;
        match target {
            ValueTarget::Global(control) => {
                let p = self.param(control);
                let sign = if control.inverted() { -1.0 } else { 1.0 };
                let norm = (p.unmodulated_normalized_value() + delta * sign).clamp(0.0, 1.0);
                cx.emit(RawParamEvent::BeginSetParameter(p.as_ptr()));
                cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                cx.emit(RawParamEvent::EndSetParameter(p.as_ptr()));
            }
            ValueTarget::Band(field) => self.change(|b| adjust_band_value(b, field, d)),
            ValueTarget::Lift(field) => self.change_lift(|b| adjust_lift_value(b, field, d)),
        }
    }

    pub(super) fn press_value(
        &mut self,
        cx: &mut EventContext,
        target: ValueTarget,
        rect: (f32, f32, f32, f32),
        origin: (f32, f32),
    ) {
        pleasant_ui::begin_value_press(cx, &mut self.value_press, target, rect, origin);
    }

    pub(super) fn handle_value_press(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
    ) -> bool {
        use pleasant_ui::readout::PressAction;

        match pleasant_ui::readout::handle_press(&mut self.value_press, event, (x, y)) {
            PressAction::Inactive => return false,
            PressAction::BeginDrag(press) => {
                self.drag = Some(Target::Value(press.target));
                self.down = press.origin;
                self.last_drag = press.origin;
                // Continue through relative readout dragging.
                return false;
            }
            PressAction::BeginEdit(press) => {
                cx.release();
                self.start_edit(cx, press.target, press.rect);
                cx.needs_redraw();
            }
            PressAction::Released | PressAction::Cancelled => {
                cx.release();
                cx.needs_redraw();
            }
            PressAction::Blocked => {}
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
            ValueTarget::Global(control) => {
                let value = self.param(control).value() as f64;
                if control.inverted() { -value } else { value }
            }
            ValueTarget::Band(field) => {
                let Some(b) = self.selected.and_then(|id| self.find_band(id)) else {
                    return;
                };
                band_component(&b, field)
            }
            ValueTarget::Lift(field) => {
                let lift_bands = self.params.lift_bands.lock().unwrap();
                let Some(b) = lift_bands.iter().find(|b| Some(b.id) == self.selected) else {
                    return;
                };
                lift_component(b, field)
            }
        };
        let text = if let ValueTarget::Global(GlobalControl::PseTime) = target {
            format_pse_time(
                self.param(GlobalControl::PseTime).value() as f64,
                self.params.pse_peak.value(),
            )
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
            ValueTarget::Global(control) => {
                let p = self.param(control);
                let norm = p.preview_normalized(if control.inverted() {
                    -value as f32
                } else {
                    value as f32
                });
                cx.emit(RawParamEvent::BeginSetParameter(p.as_ptr()));
                cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                cx.emit(RawParamEvent::EndSetParameter(p.as_ptr()));
            }
            ValueTarget::Band(field) => self.change(|b| set_band_component(b, field, value)),
            ValueTarget::Lift(field) => self.change_lift(|b| set_lift_component(b, field, value)),
        }
        self.edit = None;
        true
    }
    pub(super) fn advance_edit(&mut self, cx: &mut EventContext) {
        let target = self.edit.as_ref().unwrap().target;
        if self.commit_edit(cx) {
            let fields = self.visible_value_fields();
            let index = fields.iter().position(|(t, _)| *t == target).unwrap_or(0);
            let next = if cx.modifiers().shift() {
                (index + fields.len() - 1) % fields.len()
            } else {
                (index + 1) % fields.len()
            };
            self.start_edit(cx, fields[next].0, fields[next].1);
        }
    }

    pub(super) fn global_controls(&self) -> &'static [GlobalControl] {
        match self.dyn_page.get() {
            DynPage::Main => &[GlobalControl::Compression],
            DynPage::Controls => &[
                GlobalControl::CompAttack,
                GlobalControl::CompRelease,
                GlobalControl::CompRatio,
                GlobalControl::Dry,
                GlobalControl::Wet,
            ],
        }
    }
    pub(super) fn param(&self, control: GlobalControl) -> &FloatParam {
        match control {
            GlobalControl::Compression => &self.params.compression,
            GlobalControl::Gate => &self.params.gate,
            GlobalControl::PseVoice => &self.params.pse_voice_det,
            GlobalControl::Dry => &self.params.dry,
            GlobalControl::Wet => &self.params.wet,
            GlobalControl::CompAttack => &self.params.comp_attack,
            GlobalControl::CompRatio => &self.params.comp_ratio,
            GlobalControl::PseDepth => &self.params.pse_depth,
            GlobalControl::PseHysteresis => &self.params.pse_hysteresis,
            GlobalControl::PseTime => &self.params.pse_time,
            GlobalControl::CompRelease => &self.params.comp_release,
            GlobalControl::CompDepth => &self.params.comp_depth,
            GlobalControl::Output => &self.params.output_gain,
            GlobalControl::WallEven => &self.params.wall_even,
            GlobalControl::WallOdd => &self.params.wall_odd,
            GlobalControl::WallThreshold => &self.params.wall_threshold,
        }
    }
}

fn band_component(b: &Band, field: BandField) -> f64 {
    match field {
        BandField::Frequency => b.freq,
        BandField::Gain => b.gain,
        BandField::Q => b.q,
        BandField::Threshold => b.threshold,
        BandField::Ratio => b.ratio,
        BandField::Attack => b.attack,
        BandField::Release => b.release,
        BandField::Range => b.range,
    }
}

fn lift_component(b: &crate::lift::LiftBand, field: BandField) -> f64 {
    match field {
        BandField::Frequency => b.freq,
        BandField::Gain => b.gain,
        BandField::Q => b.q,
        BandField::Threshold => b.threshold,
        BandField::Ratio => b.ratio,
        BandField::Attack => b.attack,
        BandField::Release => b.release,
        BandField::Range => b.range,
    }
}

fn set_band_component(b: &mut Band, field: BandField, value: f64) {
    match field {
        BandField::Frequency => b.freq = value,
        BandField::Gain => b.gain = value,
        BandField::Q => b.q = value,
        BandField::Threshold => b.threshold = value,
        BandField::Ratio => b.ratio = value,
        BandField::Attack => b.attack = value,
        BandField::Release => b.release = value,
        BandField::Range => b.range = value,
    }
}

fn set_lift_component(b: &mut crate::lift::LiftBand, field: BandField, value: f64) {
    match field {
        BandField::Frequency => b.freq = value.clamp(20.0, 20000.0),
        BandField::Gain => b.gain = value.clamp(-100.0, 0.0),
        BandField::Q => b.q = value,
        BandField::Threshold => b.threshold = value,
        BandField::Ratio => b.ratio = value,
        BandField::Attack => b.attack = value,
        BandField::Release => b.release = value,
        BandField::Range => b.range = value,
    }
}

fn adjust_lift_value(b: &mut crate::lift::LiftBand, field: BandField, delta: f64) {
    match field {
        BandField::Frequency => b.freq *= 1000.0_f64.powf(delta),
        BandField::Gain => b.gain += 100.0 * delta,
        BandField::Q => b.q *= 120.0_f64.powf(delta),
        BandField::Threshold => b.threshold += 60.0 * delta,
        BandField::Ratio => b.ratio += 19.0 * delta,
        BandField::Attack => b.attack *= 2000.0_f64.powf(delta),
        BandField::Release => b.release *= 200.0_f64.powf(delta),
        BandField::Range => b.range += 24.0 * delta,
    }
}

fn adjust_band_value(b: &mut Band, field: BandField, delta: f64) {
    match field {
        BandField::Frequency => b.freq *= 1000.0_f64.powf(delta),
        BandField::Gain => b.gain += 2.0 * crate::band::MAX_GAIN_DB * delta,
        BandField::Q => b.q *= 120.0_f64.powf(delta),
        BandField::Threshold => b.threshold += 60.0 * delta,
        BandField::Ratio => b.ratio += 19.0 * delta,
        BandField::Attack => b.attack *= 2000.0_f64.powf(delta),
        BandField::Release => b.release *= 200.0_f64.powf(delta),
        BandField::Range => b.range += 2.0 * MAX_RANGE_DB * delta,
    }
}

#[cfg(test)]
mod value_drag_tests {
    use super::*;

    #[test]
    fn readout_adjustments_change_only_the_addressed_parameter() {
        for field in [
            BandField::Frequency,
            BandField::Gain,
            BandField::Q,
            BandField::Threshold,
            BandField::Ratio,
            BandField::Attack,
            BandField::Release,
            BandField::Range,
        ] {
            let original = Band::default();
            let mut band = original.clone();
            adjust_band_value(&mut band, field, 0.01);
            assert_ne!(band, original);
            if field != BandField::Ratio {
                assert_eq!(band.ratio, original.ratio);
            }
            if field != BandField::Range {
                assert_eq!(band.range, original.range);
            }
            if field != BandField::Gain {
                assert_eq!(band.gain, original.gain);
            }
            adjust_band_value(&mut band, field, -0.01);
            assert!((band.gain - original.gain).abs() < 1e-8);
            assert!((band.freq - original.freq).abs() < 1e-8);
            assert!((band.attack - original.attack).abs() < 1e-8);
        }
    }
}
