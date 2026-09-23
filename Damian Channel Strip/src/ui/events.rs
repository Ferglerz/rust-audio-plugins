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
            if self.edit.is_some() {
                match e {
                    WindowEvent::CharInput(c) => {
                        if !cx.modifiers().command() && c.is_ascii() && !c.is_control() {
                            self.edit.as_mut().unwrap().insert(&c.to_string());
                        }
                        meta.consume();
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
                        meta.consume();
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
                            self.start_edit(cx, target, rect);
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
                    if inside(x, y, self.dyn_power_button_rect()) {
                        self.dyn_bypass_anim.trigger_click();
                        self.toggle(cx, &self.params.comp_on);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.pse_power_button_rect()) {
                        self.pse_bypass_anim.trigger_click();
                        self.toggle(cx, &self.params.pse_on);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.wall_power_button_rect()) {
                        self.wall_bypass_anim.trigger_click();
                        self.toggle(cx, &self.params.wall_on);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.dyn_cog_button_rect()) {
                        match self.dyn_page.get() {
                            DynPage::Main => {
                                self.dyn_page.set(DynPage::Controls);
                                self.dyn_anim_target.set(1.0);
                            }
                            DynPage::Controls => {
                                self.dyn_page.set(DynPage::Main);
                                self.dyn_anim_target.set(0.0);
                            }
                        }
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.pse_cog_button_rect()) {
                        match self.pse_page.get() {
                            PsePage::Main => {
                                self.pse_page.set(PsePage::Controls);
                                self.pse_anim_target.set(1.0);
                            }
                            PsePage::Controls => {
                                self.pse_page.set(PsePage::Main);
                                self.pse_anim_target.set(0.0);
                            }
                        }
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.wall_cog_button_rect()) {
                        match self.wall_page.get() {
                            WallPage::Main => {
                                self.wall_page.set(WallPage::Controls);
                                self.wall_anim_target.set(1.0);
                            }
                            WallPage::Controls => {
                                self.wall_page.set(WallPage::Main);
                                self.wall_anim_target.set(0.0);
                            }
                        }
                        cx.needs_redraw();
                        return;
                    }
                    match self.pse_page.get() {
                        PsePage::Main => {
                            if (self.pse_anim_progress.get() - self.pse_anim_target.get()).abs()
                                > 0.01
                                && inside(x, y, self.pse_bounds())
                            {
                                return;
                            }
                            let r = self.pse_main_thresh_slider_rect();
                            let gate_ptr = self.params.gate.as_ptr();
                            let thresh_norm = self.params.gate.unmodulated_normalized_value();
                            let thresh_y = r.1 + r.3 * (1.0 - thresh_norm);
                            let handle = self.pse_main_thresh_handle_rect(thresh_y);
                            let handle_hit = (
                                handle.0 - 4.0,
                                handle.1 - 2.0,
                                handle.2 + 8.0,
                                handle.3 + 4.0,
                            );
                            let knee_rect = self.pse_main_knee_rect(thresh_y, 0.0);
                            if inside(x, y, handle_hit) {
                                self.drag = Some(Target::Global(1));
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(gate_ptr));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                            if inside(x, y, knee_rect) {
                                let from_top = y < thresh_y;
                                self.drag = Some(Target::PseKnee { from_top });
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(
                                    self.params.pse_knee.as_ptr(),
                                ));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                            let gr = self.pse_main_gr_meter_rect();
                            let pse_depth_ptr = self.params.pse_depth.as_ptr();
                            let depth_norm = self.params.pse_depth.unmodulated_normalized_value();
                            let depth_y = gr.1 + gr.3 * depth_norm;
                            let depth_handle = self.pse_main_depth_handle_rect(depth_y);
                            if inside(x, y, depth_handle) {
                                self.drag = Some(Target::Global(7));
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(pse_depth_ptr));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                            if inside(x, y, (r.0 - 10.0, r.1 - 10.0, r.2 + 20.0, r.3 + 20.0)) {
                                self.drag = Some(Target::Global(1));
                                self.last_drag = (x, y);
                                let norm = 1.0 - ((y - r.1) / r.3).clamp(0.0, 1.0);
                                cx.emit(RawParamEvent::BeginSetParameter(gate_ptr));
                                cx.emit(RawParamEvent::SetParameterNormalized(gate_ptr, norm));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                        }
                        PsePage::Controls => {
                            if (self.pse_anim_progress.get() - self.pse_anim_target.get()).abs()
                                > 0.01
                                && inside(x, y, self.pse_bounds())
                            {
                                return;
                            }
                            if inside(x, y, self.pse_detect_mode_rect()) {
                                self.toggle(cx, &self.params.pse_peak);
                                cx.needs_redraw();
                                return;
                            }
                        }
                    }
                    match self.dyn_page.get() {
                        DynPage::Main => {
                            if (self.dyn_anim_progress.get() - self.dyn_anim_target.get()).abs()
                                > 0.01
                                && inside(x, y, self.dyn_bounds())
                            {
                                return;
                            }
                            let r = self.dyn_main_thresh_slider_rect();
                            let gr = self.dyn_main_gr_meter_rect();
                            let comp_ptr = self.params.compression.as_ptr();
                            let thresh_norm =
                                self.params.compression.unmodulated_normalized_value();
                            let thresh_y = r.1 + r.3 * thresh_norm;
                            let handle = self.dyn_main_thresh_handle_rect(thresh_y);
                            let handle_hit = (
                                handle.0 - 4.0,
                                handle.1 - 2.0,
                                handle.2 + 8.0,
                                handle.3 + 4.0,
                            );
                            let knee_rect = self.dyn_main_knee_rect(thresh_y, 0.0);
                            if inside(x, y, handle_hit) {
                                self.drag = Some(Target::Global(0));
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(comp_ptr));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                            if inside(x, y, knee_rect) && !inside(x, y, handle_hit) {
                                let from_top = y < thresh_y;
                                self.drag = Some(Target::CompKnee { from_top });
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(
                                    self.params.comp_knee.as_ptr(),
                                ));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                            let comp_depth_ptr = self.params.comp_depth.as_ptr();
                            let depth_norm = self.params.comp_depth.unmodulated_normalized_value();
                            let depth_y = gr.1 + gr.3 * depth_norm;
                            let depth_handle = self.dyn_main_depth_handle_rect(depth_y);
                            if inside(x, y, depth_handle) {
                                self.drag = Some(Target::Global(14));
                                self.last_drag = (x, y);
                                cx.emit(RawParamEvent::BeginSetParameter(comp_depth_ptr));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                            let combined = (r.0 - 10.0, r.1 - 10.0, r.2 + gr.2 + 20.0, r.3 + 20.0);
                            if inside(x, y, combined) {
                                self.drag = Some(Target::Global(0));
                                self.last_drag = (x, y);
                                let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                                cx.emit(RawParamEvent::BeginSetParameter(comp_ptr));
                                cx.emit(RawParamEvent::SetParameterNormalized(comp_ptr, norm));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                        }
                        DynPage::Controls => {
                            if (self.dyn_anim_progress.get() - self.dyn_anim_target.get()).abs()
                                > 0.01
                                && inside(x, y, self.dyn_bounds())
                            {
                                return;
                            }
                        }
                    }
                    match self.wall_page.get() {
                        WallPage::Main => {
                            if (self.wall_anim_progress.get() - self.wall_anim_target.get()).abs()
                                > 0.01
                                && inside(x, y, self.wall_bounds())
                            {
                                return;
                            }
                            let r = self.wall_main_thresh_slider_rect();
                            let wall_ptr = self.params.wall_threshold.as_ptr();
                            let thresh_norm =
                                self.params.wall_threshold.unmodulated_normalized_value();
                            let thresh_y = r.1 + r.3 * thresh_norm;
                            let handle = self.wall_main_thresh_handle_rect(thresh_y);
                            let handle_hit = (
                                handle.0 - 4.0,
                                handle.1 - 2.0,
                                handle.2 + 8.0,
                                handle.3 + 4.0,
                            );
                            if inside(x, y, handle_hit)
                                || inside(x, y, (r.0 - 10.0, r.1 - 10.0, r.2 + 20.0, r.3 + 20.0))
                            {
                                self.drag = Some(Target::Global(18));
                                self.last_drag = (x, y);
                                let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                                cx.emit(RawParamEvent::BeginSetParameter(wall_ptr));
                                cx.emit(RawParamEvent::SetParameterNormalized(wall_ptr, norm));
                                cx.capture();
                                cx.needs_redraw();
                                return;
                            }
                        }
                        WallPage::Controls => {
                            if (self.wall_anim_progress.get() - self.wall_anim_target.get()).abs()
                                > 0.01
                                && inside(x, y, self.wall_bounds())
                            {
                                return;
                            }
                        }
                    }
                    if let Some(resolution) = self.processing_menu.take() {
                        let r = processing_menu_rect(resolution);
                        let row_h = dropdown_row_h(PROCESS_BUTTON_TEXT);
                        if inside(x, y, r) {
                            let row = ((y - r.1) / row_h) as usize;
                            let (ptr, normalized) = if resolution {
                                (self.params.linear_resolution.as_ptr(), row as f32 / 4.0)
                            } else {
                                (self.params.processing_mode.as_ptr(), row as f32 / 2.0)
                            };
                            cx.emit(RawParamEvent::BeginSetParameter(ptr));
                            cx.emit(RawParamEvent::SetParameterNormalized(ptr, normalized));
                            cx.emit(RawParamEvent::EndSetParameter(ptr));
                        }
                        cx.needs_redraw();
                        return;
                    }
                    let resolution = self.params.processing_mode.value()
                        == ProcessingMode::LinearPhase
                        && inside(x, y, resolution_button_rect());
                    if inside(x, y, PROCESS_BUTTON) || resolution {
                        self.processing_menu = Some(resolution);
                        self.menu = None;
                        self.scale_menu = false;
                        cx.needs_redraw();
                        return;
                    }
                    if self.scale_menu {
                        self.scale_menu = false;
                        if inside(x, y, self.scale_menu_rect()) {
                            let row_h = dropdown_row_h(SCALE_BUTTON_TEXT);
                            let index = ((y - self.scale_menu_rect().1) / row_h) as usize;
                            if let Some(range) = SCALES.get(index) {
                                self.set_graph_range(*range);
                            }
                        }
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.scale_button_rect()) {
                        self.scale_menu = true;
                        self.menu = None;
                        cx.needs_redraw();
                        return;
                    }
                    if let Some(menu) = self.menu.take() {
                        let handled = if let Some(b) = self.selected_lift() {
                            let r = menu.rect_lift_at(&b, self.graph_db, self.gx(), self.gw());
                            if inside(x, y, r) {
                                let row = ((y - r.1) / menu.row_h()) as usize;
                                if row < menu.count() {
                                    self.change_lift(|b| match menu {
                                        BandMenu::Shape => {
                                            let shape = Shape::ALL[row];
                                            if shape.is_cut() && !b.shape.is_cut() {
                                                b.q = std::f64::consts::FRAC_1_SQRT_2;
                                            }
                                            b.shape = shape;
                                        }
                                        BandMenu::Order => b.order = row as u8 + 1,
                                    });
                                }
                            }
                            true
                        } else {
                            false
                        };
                        if !handled {
                            let band = self.selected.and_then(|id| self.find_band(id));
                            if let Some(b) = band {
                                let r = menu.rect_at(&b, self.graph_db, self.gx(), self.gw());
                                if inside(x, y, r) {
                                    let row = ((y - r.1) / menu.row_h()) as usize;
                                    if row < menu.count() {
                                        self.change(|b| match menu {
                                            BandMenu::Shape => {
                                                let shape = Shape::ALL[row];
                                                if shape.is_cut() && !b.shape.is_cut() {
                                                    b.q = std::f64::consts::FRAC_1_SQRT_2;
                                                }
                                                b.shape = shape;
                                                if !shape.has_gain() {
                                                    b.dynamic = false;
                                                }
                                            }
                                            BandMenu::Order => b.order = row as u8 + 1,
                                        });
                                        self.sync_band_dyn_page();
                                    }
                                }
                            }
                        }
                        cx.needs_redraw();
                        return;
                    }
                    if let Some((target, rect)) = self.value_at(x, y) {
                        self.start_edit(cx, target, rect);
                        cx.needs_redraw();
                        return;
                    }
                    if inside(x, y, self.eq_tab_1_rect()) {
                        if self.active_eq.get() != EQ_PAGE_1 {
                            self.start_eq_page(EQ_PAGE_1);
                            cx.needs_redraw();
                        }
                        return;
                    }
                    if inside(x, y, self.eq_tab_2_rect()) {
                        if self.active_eq.get() != EQ_PAGE_2 {
                            self.start_eq_page(EQ_PAGE_2);
                            cx.needs_redraw();
                        }
                        return;
                    }
                    if inside(x, y, self.eq_tab_sc_rect()) {
                        if self.active_eq.get() != EQ_PAGE_SC {
                            self.start_eq_page(EQ_PAGE_SC);
                            cx.needs_redraw();
                        }
                        return;
                    }
                    if inside(x, y, self.eq_tab_lift_rect()) {
                        if self.active_eq.get() != EQ_PAGE_LIFT {
                            self.start_eq_page(EQ_PAGE_LIFT);
                            cx.needs_redraw();
                        }
                        return;
                    }
                    if self.active_eq.get() == EQ_PAGE_SC
                        && inside(x, y, self.eq_header_listen_sc_rect())
                    {
                        self.toggle(cx, &self.params.pse_listen);
                        cx.needs_redraw();
                        return;
                    }
                    self.down = (x, y);
                    self.last_drag = (x, y);
                    if inside(x, y, self.eq_power_rect()) {
                        self.eq_bypass_anim.trigger_click();
                        match self.active_eq.get() {
                            EQ_PAGE_2 => self.toggle(cx, &self.params.eq2_on),
                            EQ_PAGE_LIFT => self.toggle(cx, &self.params.lift_on),
                            EQ_PAGE_SC => self.toggle(cx, &self.params.sc_eq_on),
                            _ => self.toggle(cx, &self.params.eq_on),
                        }
                    } else {
                        if (self.anim_progress.get() - self.anim_target.get()).abs() > 0.01
                            && inside(x, y, self.eq_bounds())
                        {
                            return;
                        }
                        // Lift dock sits on top of HUD and graph.
                        let mut bottom_control_handled = false;
                        let bottom_dock = (
                            self.gx(),
                            GRAPH_BOTTOM - LIFT_DOCK_H,
                            self.gw(),
                            LIFT_DOCK_H,
                        );
                        if let Some(i) = self
                            .global_hit_rects()
                            .into_iter()
                            .find(|(_, r)| inside(x, y, *r))
                            .map(|(i, _)| i)
                        {
                            bottom_control_handled = true;
                            self.drag = Some(Target::Global(i));
                            self.last_drag = (x, y);
                            cx.emit(RawParamEvent::BeginSetParameter(self.param(i).as_ptr()));
                            cx.capture();
                        } else if self.is_lift_selected() {
                            let badge_rect = (self.gx() + 8.0, GRAPH_BOTTOM - 48.0, 32.0, 28.0);
                            if inside(x, y, badge_rect) {
                                self.lift_badge_anim.trigger_click();
                                bottom_control_handled = true;
                                self.change_lift(|b| b.enabled = !b.enabled);
                            } else if let Some(i) =
                                (0..5).find(|i| inside(x, y, self.band_bar_rect(*i)))
                            {
                                bottom_control_handled = true;
                                self.drag = Some(Target::LiftBand(i));
                                self.last_drag = (x, y);
                                cx.capture();
                                self.apply_drag(cx, x, y, false, false, false);
                            } else if inside(x, y, bottom_dock) {
                                bottom_control_handled = true;
                            }
                        }

                        if !bottom_control_handled {
                            // Check if click hits selected band HUD
                            let mut hud_consumed = false;
                            if self.hud_visible() {
                                if let Some(sel_id) = self.selected {
                                    if sel_id >= LIFT_ID_BASE {
                                        if let Some(b) = self.selected_lift() {
                                            let (bx, by, _, _) = self.hud_rect_for_lift(&b);
                                            let node_x = self.freq_x(b.freq);
                                            let node_y = lift_gain_y(b.gain);
                                            if let Some(chrome) = node_chrome_hit(
                                                x,
                                                y,
                                                node_x,
                                                node_y,
                                                if node_chrome_above(b.gain, None) {
                                                    NodeChromeLayout::Above
                                                } else {
                                                    NodeChromeLayout::Below
                                                },
                                                self.gx(),
                                                self.gw(),
                                            ) {
                                                hud_consumed = true;
                                                match chrome {
                                                    NodeChrome::Solo => {
                                                        self.pending_solo = Some(b.id);
                                                        self.down = (x, y);
                                                        self.last_drag = (x, y);
                                                        cx.capture();
                                                    }
                                                    NodeChrome::Close => self.delete(),
                                                }
                                            } else if node_chrome_gap_hit(
                                                x,
                                                y,
                                                node_x,
                                                node_y,
                                                if node_chrome_above(b.gain, None) {
                                                    NodeChromeLayout::Above
                                                } else {
                                                    NodeChromeLayout::Below
                                                },
                                                self.gx(),
                                                self.gw(),
                                            ) {
                                                hud_consumed = true;
                                            } else if inside(x, y, self.hud_rect_for_lift(&b)) {
                                                hud_consumed = true;
                                                match hud_row_hit(x, y, bx, by, b.shape.is_cut()) {
                                                    Some(HudChrome::Bypass) => {
                                                        self.band_bypass_anim.trigger_click();
                                                        self.change_lift(|b| {
                                                            b.enabled = !b.enabled
                                                        });
                                                    }
                                                    Some(HudChrome::Shape) => {
                                                        self.menu = Some(BandMenu::Shape);
                                                    }
                                                    Some(HudChrome::Order) => {
                                                        self.menu = Some(BandMenu::Order);
                                                    }
                                                    None => {}
                                                }
                                            }
                                        }
                                    } else if let Some(b) = self.find_band(sel_id) {
                                        let g = self.hud_geom(&b);
                                        let gx = self.gx();
                                        let gw = self.gw();
                                        let node_x = self.freq_x(b.freq);
                                        let node_y = db_y(
                                            if b.shape.has_gain() { b.gain } else { 0.0 },
                                            self.graph_db,
                                        );
                                        let animating = (self.band_dyn_anim_progress.get()
                                            - self.band_dyn_anim_target.get())
                                        .abs()
                                            > 0.01;
                                        let layout =
                                            band_chrome_layout(&b, self.graph_db, gx, gw, true);
                                        if let Some(chrome) =
                                            node_chrome_hit(x, y, node_x, node_y, layout, gx, gw)
                                        {
                                            hud_consumed = true;
                                            match chrome {
                                                NodeChrome::Solo => {
                                                    self.pending_solo = Some(b.id);
                                                    self.down = (x, y);
                                                    self.last_drag = (x, y);
                                                    cx.capture();
                                                }
                                                NodeChrome::Close => self.delete(),
                                            }
                                        } else if node_chrome_gap_hit(
                                            x, y, node_x, node_y, layout, gx, gw,
                                        ) {
                                            hud_consumed = true;
                                        } else if hud_dyn_btn_hit(g, x, y, gx, gw) {
                                            hud_consumed = true;
                                            self.dyn_band_anim.trigger_click();
                                            self.change(|b| b.dynamic = !b.dynamic);
                                            self.sync_band_dyn_page();
                                        } else if hud_cog_hit(g, x, y, gx, gw) {
                                            hud_consumed = true;
                                            self.set_band_dyn_page(!self.band_dyn_page.get());
                                        } else if inside(x, y, self.hud_bounds_for(&b)) {
                                            hud_consumed = true;
                                            if animating {
                                            } else if self.band_dyn_page.get() {
                                                if band_allows_dyn(&b) {
                                                    if let Some(i) = (0..5).find(|i| {
                                                        *i != 1
                                                            && inside(
                                                                x,
                                                                y,
                                                                self.hud_dyn_field_rect(&b, *i),
                                                            )
                                                    }) {
                                                        self.drag = Some(Target::Band(i));
                                                        self.last_drag = (x, y);
                                                        cx.capture();
                                                        self.apply_drag(
                                                            cx, x, y, false, false, false,
                                                        );
                                                    }
                                                }
                                            } else {
                                                match hud_row_hit(
                                                    x,
                                                    y,
                                                    g.bx,
                                                    g.by,
                                                    b.shape.is_cut(),
                                                ) {
                                                    Some(HudChrome::Bypass) => {
                                                        self.band_bypass_anim.trigger_click();
                                                        self.change(|b| b.enabled = !b.enabled);
                                                    }
                                                    Some(HudChrome::Shape) => {
                                                        self.menu = Some(BandMenu::Shape);
                                                    }
                                                    Some(HudChrome::Order) => {
                                                        self.menu = Some(BandMenu::Order);
                                                    }
                                                    None => {}
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if !hud_consumed && inside(x, y, self.graph_area()) {
                                let lift_hit = self.lift_hit_at(x, y);
                                if let Some(id) = lift_hit {
                                    self.select(Some(id));
                                    self.drag = Some(Target::Node(id));
                                    self.last_drag = (x, y);
                                    cx.capture();
                                } else {
                                    let bands = self.clone_page_bands();
                                    if let Some(id) = self.find_threshold_hit(x, y, &bands) {
                                        self.begin_threshold_drag(cx, id, x, y);
                                    } else if let Some(id) = self.find_range_hit(x, y, &bands) {
                                        self.begin_range_drag(cx, id, x, y);
                                    } else {
                                        let hit = bands
                                            .iter()
                                            .rev()
                                            .find(|b| {
                                                ((x - self.freq_x(b.freq)).powi(2)
                                                    + (y - db_y(
                                                        if b.shape.has_gain() {
                                                            b.gain
                                                        } else {
                                                            0.0
                                                        },
                                                        self.graph_db,
                                                    ))
                                                    .powi(2))
                                                .sqrt()
                                                    < 16.0
                                            })
                                            .map(|b| b.id);
                                        if let Some(id) = hit {
                                            self.select(Some(id));
                                            self.drag = Some(Target::Node(id));
                                            self.last_drag = (x, y);
                                            cx.capture();
                                        } else {
                                            let sr = self.shared.sample_rate.load(Ordering::Relaxed)
                                                as f64;
                                            let config = self.params.processing_config();
                                            let eq_sr = config.mode.rate(sr);
                                            let meters = self.dyn_gr_uncapped();
                                            let db = bands
                                                .iter()
                                                .filter(|b| b.enabled)
                                                .map(|b| {
                                                    BandCoeffs::make(&plot_band(b, &meters), eq_sr)
                                                        .response(self.x_freq(x), eq_sr)
                                                })
                                                .sum::<f64>();
                                            if (y - db_y(db, self.graph_db)).abs() < 12.0 {
                                                self.pending_create = Some(PendingCreate::Curve);
                                                cx.capture();
                                            } else if self.selected.is_none() {
                                                self.create_band(x, y, false, cx.modifiers().alt());
                                                cx.capture();
                                            } else {
                                                self.pending_create =
                                                    Some(PendingCreate::Background);
                                                cx.capture();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    cx.needs_redraw();
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
                            let bottom_dock = (
                                self.gx(),
                                GRAPH_BOTTOM - LIFT_DOCK_H,
                                self.gw(),
                                LIFT_DOCK_H,
                            );
                            if inside(x, y, bottom_dock) {
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
                WindowEvent::MouseUp(MouseButton::Left) => {
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
