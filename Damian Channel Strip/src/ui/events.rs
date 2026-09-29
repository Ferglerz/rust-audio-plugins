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
                match e {
                    WindowEvent::CharInput(c) => {
                        if !cx.modifiers().command() && c.is_ascii() && !c.is_control() {
                            self.edit.as_mut().unwrap().insert(&c.to_string());
                        }
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::KeyDown(code, key) => {
                        self.edit_key(cx, *code);
                        // Some hosts deliver only KeyDown (empty characters / keyCode).
                        let has_char = matches!(key, Some(Key::Character(_)));
                        if !has_char && !cx.modifiers().command() {
                            if let Some(c) = typed_char(*code, cx.modifiers().shift()) {
                                self.edit.as_mut().unwrap().insert(&c.to_string());
                            }
                        }
                        nih_plug_vizia::consume_window_event(cx, e, meta);
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::MouseDown(MouseButton::Left) => {
                        if inside(x, y, self.edit.as_ref().unwrap().rect) {
                            let edit = self.edit.as_mut().unwrap();
                            let capacity = ((edit.rect.2 - 8.0) / 6.6) as usize;
                            let start = edit.cursor.saturating_sub(capacity);
                            edit.cursor = (start
                                + (((x - edit.rect.0 - 4.0) / 6.6).round().max(0.0) as usize))
                                .min(edit.text.len());
                            edit.anchor = edit.cursor;
                            cx.needs_redraw();
                            return;
                        }
                        if !self.commit_edit(cx) {
                            self.edit = None;
                            cx.needs_redraw();
                        }
                    }
                    WindowEvent::FocusOut => {
                        if !self.commit_edit(cx) {
                            self.edit = None;
                        }
                    }
                    WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                        let edit = self.edit.as_mut().unwrap();
                        edit.anchor = 0;
                        edit.cursor = edit.text.len();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::MouseDoubleClick(_)
                    | WindowEvent::MouseScroll(_, _)
                    | WindowEvent::MouseDown(_) => return,
                    _ => {}
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
                        self.drag = Some(Target::Global(15));
                        self.down = (x, y);
                        self.last_drag = (x, y);
                        cx.capture();
                        let r = output_gain_rect();
                        let bar_x = r.0 + 8.0;
                        let bar_w = r.2 - 16.0;
                        let norm = ((x - bar_x) / bar_w).clamp(0.0, 1.0);
                        let p = self.param(15);
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
                            .unwrap_or(2);
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
                            let p = self.param(0);
                            let cur = p.unmodulated_normalized_value();
                            let norm = (cur - *dy * 0.02).clamp(0.0, 1.0);
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
                            let norm =
                                (cur + *dy * if i == 0 { -0.03 } else { 0.03 }).clamp(0.0, 1.0);
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
