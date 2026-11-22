use super::*;

impl StripView {
    pub(super) fn handle_event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, meta| {
            let bounds = cx.bounds();
            let Some((x, y)) = pleasant_ui::local_xy(
                bounds.x,
                bounds.y,
                bounds.w,
                UI_W,
                cx.mouse().cursorx,
                cx.mouse().cursory,
            ) else {
                return;
            };
            let shift_down = !matches!(e, WindowEvent::FocusOut) && cx.modifiers().shift();
            if self.shift_down != shift_down {
                self.shift_down = shift_down;
                cx.needs_redraw();
            }
            if let WindowEvent::MouseScroll(_, dy) = e {
                if *dy != 0.0
                    && self.menu.is_none()
                    && !self.scale_menu
                    && self.processing_menu.is_none()
                {
                    if let Some((target, _)) = self.value_at(x, y) {
                        if self.edit.is_some() && !self.commit_edit(cx) {
                            return;
                        }
                        let delta = *dy * if cx.modifiers().shift() { 0.001 } else { 0.01 };
                        self.adjust_value(cx, target, delta);
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                        cx.needs_redraw();
                        return;
                    }
                }
            }
            if self.handle_value_press(cx, e, x, y) {
                nih_plug_vizia::consume_window_event(cx, e, meta);
                return;
            }
            if self.edit.is_some() {
                use pleasant_ui::readout::EditAction;

                // Tab order depends on the currently visible plugin controls.
                if matches!(e, WindowEvent::KeyDown(Code::Tab, _)) {
                    self.advance_edit(cx);
                    nih_plug_vizia::consume_window_event(cx, e, meta);
                    cx.needs_redraw();
                    return;
                }
                let action =
                    pleasant_ui::readout::handle_edit(self.edit.as_mut().unwrap(), cx, e, (x, y));
                match action {
                    EditAction::Handled => {
                        if matches!(e, WindowEvent::CharInput(_) | WindowEvent::KeyDown(_, _)) {
                            nih_plug_vizia::consume_window_event(cx, e, meta);
                        }
                        cx.needs_redraw();
                        return;
                    }
                    EditAction::Cancel => {
                        self.edit = None;
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                        cx.needs_redraw();
                        return;
                    }
                    EditAction::Commit => {
                        let committed = self.commit_edit(cx);
                        if matches!(e, WindowEvent::KeyDown(_, _)) {
                            // Invalid Enter keeps the editor open for correction.
                            nih_plug_vizia::consume_window_event(cx, e, meta);
                            cx.needs_redraw();
                            return;
                        }
                        // Outside clicks discard invalid input and continue to
                        // the underlying control after finishing this edit.
                        if !committed {
                            self.edit = None;
                            cx.needs_redraw();
                        }
                    }
                    EditAction::CommitAndContinue => {
                        if !self.commit_edit(cx) {
                            self.edit = None;
                        }
                    }
                    EditAction::Blocked => match e {
                        WindowEvent::MouseDoubleClick(_)
                        | WindowEvent::MouseScroll(_, _)
                        | WindowEvent::MouseDown(_) => return,
                        _ => {}
                    },
                }
            }
            match e {
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some((x, y));
                    if let Some(pending) = self.pending_create {
                        if (x - self.down.0).hypot(y - self.down.1) >= DRAG_START_THRESHOLD {
                            let (dx, dy) = self.down;
                            let curve = matches!(pending, PendingCreate::Curve);
                            self.create_band(dx, dy, curve, cx.modifiers().alt());
                            self.pending_create = None;
                        }
                    }
                    if let Some(id) = self.pending_solo {
                        if (x - self.down.0).hypot(y - self.down.1) >= DRAG_START_THRESHOLD {
                            self.begin_solo_audition_drag(id);
                        }
                    }
                    if self.drag.is_some() {
                        let shift = cx.modifiers().shift();
                        let cmd = cx.modifiers().command();
                        self.apply_drag(cx, x, y, shift, cmd, cx.modifiers().alt());
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseLeave => {
                    self.hover = None;
                    cx.needs_redraw();
                }
                WindowEvent::MouseDown(MouseButton::Left) => {
                    cx.focus();
                    if inside(x, y, THEME_BUTTON) {
                        preferences::toggle();
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.footer_comp_routing_rect()) {
                        self.toggle(cx, &self.params.comp_pre);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.footer_auto_rect()) {
                        self.toggle(cx, &self.params.auto_makeup);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.footer_link_rect()) {
                        self.toggle(cx, &self.params.stereo_link);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, output_gain_rect()) {
                        if let Some((target, rect)) = self.value_at(x, y) {
                            self.press_value(cx, target, rect, (x, y));
                            cx.needs_redraw();
                            return;
                        }
                        self.drag = Some(Target::Global(GlobalControl::Output));
                        self.down = (x, y);
                        self.last_drag = (x, y);
                        cx.capture();
                        let r = output_gain_rect();
                        let bar_x = r.0 + 8.0;
                        let bar_w = r.2 - 16.0;
                        let norm = ((x - bar_x) / bar_w).clamp(0.0, 1.0);
                        let p = self.param(GlobalControl::Output);
                        cx.emit(RawParamEvent::BeginSetParameter(p.as_ptr()));
                        cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                        cx.needs_redraw();
                        return;
                    }
                    if self.handle_dynamics_mouse_down(cx, x, y) {
                        return;
                    }
                    self.handle_eq_mouse_down(cx, x, y);
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    if self.menu.is_some() || self.scale_menu || self.processing_menu.is_some() {
                        return;
                    }
                    if self.try_reset_at(cx, x, y) {
                        return;
                    }
                    if inside(x, y, self.graph_area()) && self.drag.is_none() {
                        if self.is_lift_selected() {
                            let lift_dock = (self.gx(), GY, self.gw(), LIFT_DOCK_H);
                            if inside(x, y, lift_dock) {
                                return;
                            }
                        }
                        let over_hud = self.over_selected_hud(x, y);
                        if !over_hud {
                            let lift_hit = self.lift_hit_at(x, y);
                            if let Some(id) = lift_hit {
                                self.select(Some(id));
                                self.change_lift(|b| b.enabled = !b.enabled);
                            } else {
                                let bands = self.clone_page_bands();
                                if let Some(id) = self.find_range_hit(x, y, &bands) {
                                    self.select(Some(id));
                                    self.change(|b| {
                                        b.dynamic = true;
                                        b.range = 0.0;
                                    });
                                } else {
                                    let hit = bands
                                        .iter()
                                        .rev()
                                        .find(|b| {
                                            ((x - self.freq_x(b.freq)).powi(2)
                                                + (y - db_y(
                                                    if b.shape.has_gain() { b.gain } else { 0.0 },
                                                    self.graph_db,
                                                ))
                                                .powi(2))
                                            .sqrt()
                                                < 16.0
                                        })
                                        .map(|b| b.id);
                                    if let Some(id) = hit {
                                        self.select(Some(id));
                                        self.change(|b| b.enabled = !b.enabled);
                                    } else {
                                        self.pending_create = None;
                                        self.create_band(x, y, false, cx.modifiers().alt());
                                        cx.capture();
                                    }
                                }
                            }
                        }
                    }
                }
                WindowEvent::FocusOut => {
                    if let Some(Target::Global(i)) = self.drag {
                        cx.emit(RawParamEvent::EndSetParameter(self.param(i).as_ptr()));
                        self.drag = None;
                        cx.release();
                    } else if matches!(
                        self.drag,
                        Some(Target::Band(_) | Target::LiftBand(_) | Target::Value(_))
                    ) {
                        self.drag = None;
                        cx.release();
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    self.expand_graph_after_drag();
                    if let Some(restore) = self.alt_solo_restore.take() {
                        self.shared.solo_id.store(restore, Ordering::Relaxed);
                    }
                    if let Some(Target::Global(i)) = self.drag {
                        cx.emit(RawParamEvent::EndSetParameter(self.param(i).as_ptr()));
                    }
                    if let Some(Target::CompKnee { .. }) = self.drag {
                        cx.emit(RawParamEvent::EndSetParameter(
                            self.params.comp_knee.as_ptr(),
                        ));
                    }
                    if let Some(Target::PseKnee { .. }) = self.drag {
                        cx.emit(RawParamEvent::EndSetParameter(
                            self.params.pse_knee.as_ptr(),
                        ));
                    }
                    if let Some(Target::SoloAudition { restore, .. }) = self.drag {
                        self.shared.solo_id.store(restore, Ordering::Relaxed);
                    }
                    if self.pending_create.is_some() {
                        self.select(None);
                    }
                    if let Some(id) = self.pending_solo.take() {
                        let cur = self.shared.solo_id.load(Ordering::Relaxed);
                        if cur == id {
                            self.shared.solo_id.store(0, Ordering::Relaxed);
                        } else {
                            self.shared.solo_id.store(id, Ordering::Relaxed);
                        }
                    }
                    self.drag = None;
                    self.pending_create = None;
                    cx.release();
                    cx.needs_redraw();
                }
                WindowEvent::MouseDown(MouseButton::Right) => {
                    if (self.anim_progress.get() - self.anim_target.get()).abs() > 0.01
                        && inside(x, y, self.eq_bounds())
                    {
                        return;
                    }
                    if inside(x, y, self.graph_area()) {
                        let over_hud = self.over_selected_hud(x, y);
                        if !over_hud {
                            let lift_id = self.lift_hit_at(x, y);
                            if lift_id.is_some() {
                                self.select(lift_id);
                                self.delete();
                            } else {
                                let bands = self.clone_page_bands();
                                let id = bands
                                    .iter()
                                    .find(|b| {
                                        (x - self.freq_x(b.freq)).abs() < 15.0
                                            && (y - db_y(
                                                if b.shape.has_gain() { b.gain } else { 0.0 },
                                                self.graph_db,
                                            ))
                                            .abs()
                                                < 15.0
                                    })
                                    .map(|b| b.id);
                                if id.is_some() {
                                    self.select(id);
                                    self.delete();
                                }
                            }
                        }
                    }
                }
                WindowEvent::MouseScroll(_, dy) => {
                    if (self.anim_progress.get() - self.anim_target.get()).abs() > 0.01
                        && inside(x, y, self.eq_bounds())
                    {
                        return;
                    }
                    if inside(
                        x,
                        y,
                        (
                            self.gx() - EQ_GRAPH_PAD_LEFT,
                            GY,
                            EQ_GRAPH_PAD_LEFT,
                            GRAPH_BOTTOM - GY + 32.0,
                        ),
                    ) && *dy != 0.0
                    {
                        let cur_idx = SCALES
                            .iter()
                            .position(|&s| (s - self.graph_db).abs() < 0.1)
                            .expect("graph_db must be a valid scale");
                        let next_idx = if *dy > 0.0 {
                            cur_idx.saturating_sub(1)
                        } else {
                            (cur_idx + 1).min(SCALES.len() - 1)
                        };
                        self.set_graph_range(SCALES[next_idx]);
                        cx.needs_redraw();
                        return;
                    }
                    if self.menu.is_some() || self.scale_menu || self.processing_menu.is_some() {
                        return;
                    }
                    if self.dyn_page.get() == DynPage::Main {
                        let r = self.dyn_main_thresh_slider_rect();
                        let gr = self.dyn_main_gr_meter_rect();
                        if inside(
                            x,
                            y,
                            (r.0 - 10.0, r.1 - 10.0, r.2 + gr.2 + 20.0, r.3 + 20.0),
                        ) && *dy != 0.0
                        {
                            let p = self.param(GlobalControl::Compression);
                            let cur = p.unmodulated_normalized_value();
                            let norm = (cur + *dy * 0.02).clamp(0.0, 1.0);
                            cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                            cx.needs_redraw();
                            return;
                        }
                    }
                    if let Some((hx, hy)) = self.hover {
                        if let Some(i) = self
                            .global_hit_rects()
                            .into_iter()
                            .find(|(_, r)| inside(hx, hy, *r))
                            .map(|(i, _)| i)
                        {
                            let p = self.param(i);
                            let cur = p.unmodulated_normalized_value();
                            let norm = (cur + *dy * 0.03).clamp(0.0, 1.0);
                            cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                            cx.needs_redraw();
                            return;
                        }
                    }
                    if inside(x, y, self.graph_area()) {
                        if self.selected.is_none() {
                            let lift_hit = self.lift_hit_at(x, y);
                            if lift_hit.is_some() {
                                self.select(lift_hit);
                            } else {
                                let bands = self.clone_page_bands();
                                let hit = bands
                                    .iter()
                                    .rev()
                                    .find(|b| {
                                        ((x - self.freq_x(b.freq)).powi(2)
                                            + (y - db_y(
                                                if b.shape.has_gain() { b.gain } else { 0.0 },
                                                self.graph_db,
                                            ))
                                            .powi(2))
                                        .sqrt()
                                            < 18.0
                                    })
                                    .map(|b| b.id);
                                if hit.is_some() {
                                    self.select(hit);
                                }
                            }
                        }
                        if self.is_lift_selected() {
                            self.change_lift(|b| {
                                if !(b.shape.is_cut() && b.order == 1) {
                                    b.q = (b.q * (1.0 + *dy as f64 * 0.08)).clamp(0.15, 18.0);
                                }
                            });
                            cx.needs_redraw();
                        } else if self.selected.is_some() {
                            self.change(|b| {
                                if !(b.shape.is_cut() && b.order == 1) {
                                    b.q = (b.q * (1.0 + *dy as f64 * 0.08)).clamp(0.15, 18.0);
                                }
                            });
                            cx.needs_redraw();
                        }
                    }
                }
                WindowEvent::KeyDown(Code::Delete | Code::Backspace, _) => {
                    self.delete();
                    cx.needs_redraw();
                }
                WindowEvent::KeyDown(Code::Escape, _) => {
                    self.processing_menu = None;
                    self.menu = None;
                    self.scale_menu = false;
                    self.pending_create = None;
                    cx.needs_redraw();
                }
                _ => {}
            }
        });
    }
}

#[cfg(test)]
mod readout_tests {
    use super::*;
    use nih_plug_vizia::vizia::backend::BackendContext;

    fn context() -> (Context, Entity) {
        let mut cx = Context::default();
        let target = Element::new(&mut cx).entity();
        BackendContext::new(&mut cx)
            .cache()
            .set_bounds(target, BoundingBox::from_min_max(0.0, 0.0, UI_W, UI_H));
        (cx, target)
    }

    fn send(
        view: &mut StripView,
        cx: &mut Context,
        target: Entity,
        point: (f32, f32),
        event: WindowEvent,
    ) {
        let mut backend = BackendContext::new_with_event_manager(cx);
        backend.emit_origin(WindowEvent::MouseMove(point.0, point.1));
        backend.process_events();
        view.handle_event(
            &mut EventContext::new_with_current(cx, target),
            &mut Event::new(event),
        );
        BackendContext::new_with_event_manager(cx).process_events();
    }

    fn invalid_edit(view: &mut StripView) {
        let mut edit = ValueEdit::new(ValueTarget::Global(GlobalControl::Compression), (10.0, 10.0, 80.0, 20.0), "0".into());
        edit.insert("invalid");
        view.edit = Some(edit);
    }

    #[test]
    fn readout_invalid_enter_and_tab_stay_open_but_focus_loss_discards() {
        let mut view = super::super::tests::layout::test_view();
        let (mut cx, target) = context();
        invalid_edit(&mut view);
        for key in [Code::Enter, Code::Tab] {
            send(
                &mut view,
                &mut cx,
                target,
                (15.0, 15.0),
                WindowEvent::KeyDown(key, None),
            );
            let edit = view.edit.as_ref().unwrap();
            assert!(edit.invalid);
            assert_eq!(edit.target, ValueTarget::Global(GlobalControl::Compression));
        }
        view.drag = Some(Target::Value(ValueTarget::Global(GlobalControl::Compression)));
        send(
            &mut view,
            &mut cx,
            target,
            (15.0, 15.0),
            WindowEvent::FocusOut,
        );
        assert!(view.edit.is_none());
        assert!(
            view.drag.is_none(),
            "focus loss must continue through drag cleanup"
        );
    }

    #[test]
    fn readout_outside_click_discards_invalid_text_and_activates_underlying_control() {
        let mut view = super::super::tests::layout::test_view();
        let (mut cx, target) = context();
        invalid_edit(&mut view);
        let rect = view.dyn_cog_button_rect();
        send(
            &mut view,
            &mut cx,
            target,
            (rect.0 + rect.2 * 0.5, rect.1 + rect.3 * 0.5),
            WindowEvent::MouseDown(MouseButton::Left),
        );
        assert!(view.edit.is_none());
        assert_eq!(view.dyn_page.get(), DynPage::Controls);
    }

    #[test]
    fn readout_tab_order_and_character_delivery_preserve_local_editing_policy() {
        let mut view = super::super::tests::layout::test_view();
        let (mut cx, target) = context();
        view.start_edit(
            &mut EventContext::new_with_current(&mut cx, target),
            ValueTarget::Global(GlobalControl::Compression),
            (10.0, 10.0, 80.0, 20.0),
        );
        send(
            &mut view,
            &mut cx,
            target,
            (15.0, 15.0),
            WindowEvent::KeyDown(Code::Tab, None),
        );
        assert_eq!(view.edit.as_ref().unwrap().target, ValueTarget::Global(GlobalControl::Gate));
        BackendContext::new(&mut cx)
            .modifiers()
            .set(Modifiers::SHIFT, true);
        send(
            &mut view,
            &mut cx,
            target,
            (15.0, 15.0),
            WindowEvent::KeyDown(Code::Tab, None),
        );
        assert_eq!(view.edit.as_ref().unwrap().target, ValueTarget::Global(GlobalControl::Compression));
        BackendContext::new(&mut cx)
            .modifiers()
            .set(Modifiers::SHIFT, false);
        let original = view.edit.as_ref().unwrap().text.clone();
        send(
            &mut view,
            &mut cx,
            target,
            (15.0, 15.0),
            WindowEvent::KeyDown(Code::Digit3, Some(Key::Character("3".into()))),
        );
        assert_eq!(view.edit.as_ref().unwrap().text, original);
        send(
            &mut view,
            &mut cx,
            target,
            (15.0, 15.0),
            WindowEvent::CharInput('3'),
        );
        send(
            &mut view,
            &mut cx,
            target,
            (15.0, 15.0),
            WindowEvent::KeyDown(Code::Digit4, None),
        );
        assert_eq!(view.edit.as_ref().unwrap().text, "34");
        send(
            &mut view,
            &mut cx,
            target,
            (15.0, 15.0),
            WindowEvent::KeyDown(Code::Escape, None),
        );
        assert!(view.edit.is_none());
    }

    #[test]
    fn readout_press_distinguishes_click_drag_and_cancellation() {
        let point = (15.0, 15.0);
        for ending in [
            WindowEvent::MouseUp(MouseButton::Left),
            WindowEvent::MouseMove(25.0, 15.0),
            WindowEvent::FocusOut,
        ] {
            let mut view = super::super::tests::layout::test_view();
            let (mut cx, target) = context();
            view.press_value(
                &mut EventContext::new_with_current(&mut cx, target),
                ValueTarget::Global(GlobalControl::Compression),
                (10.0, 10.0, 80.0, 20.0),
                point,
            );
            let dragging = matches!(ending, WindowEvent::MouseMove(_, _));
            let clicking = matches!(ending, WindowEvent::MouseUp(_));
            send(
                &mut view,
                &mut cx,
                target,
                if dragging { (25.0, 15.0) } else { point },
                ending,
            );
            assert!(view.value_press.is_none());
            assert_eq!(view.edit.is_some(), clicking);
            assert_eq!(view.drag.is_some(), dragging);
            if dragging {
                assert_eq!(view.down, point);
                send(
                    &mut view,
                    &mut cx,
                    target,
                    (25.0, 15.0),
                    WindowEvent::MouseUp(MouseButton::Left),
                );
                assert!(view.drag.is_none());
            }
        }
    }
}
