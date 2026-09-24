use super::super::*;

impl StripView {
    pub(in crate::ui) fn handle_eq_mouse_down(&mut self, cx: &mut EventContext, x: f32, y: f32) {
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
        let resolution = self.params.processing_mode.value() == ProcessingMode::LinearPhase
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
            self.press_value(cx, target, rect, (x, y));
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
        if self.active_eq.get() == EQ_PAGE_SC && inside(x, y, self.eq_header_listen_sc_rect()) {
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
            let mut dock_control_handled = false;
            let lift_dock = (self.gx(), GY, self.gw(), LIFT_DOCK_H);
            if let Some(i) = self
                .global_hit_rects()
                .into_iter()
                .find(|(_, r)| inside(x, y, *r))
                .map(|(i, _)| i)
            {
                dock_control_handled = true;
                self.drag = Some(Target::Global(i));
                self.last_drag = (x, y);
                cx.emit(RawParamEvent::BeginSetParameter(self.param(i).as_ptr()));
                cx.capture();
            } else if self.is_lift_selected() {
                if let Some(i) = (0..5).find(|i| inside(x, y, self.band_rect(*i))) {
                    dock_control_handled = true;
                    self.drag = Some(Target::LiftBand(i));
                    self.last_drag = (x, y);
                    cx.capture();
                    self.apply_drag(cx, x, y, false, false, false);
                } else if inside(x, y, lift_dock) {
                    dock_control_handled = true;
                }
            }

            if !dock_control_handled {
                // Check if click hits selected band HUD
                let mut hud_consumed = false;
                // Nodes sit above the Solo/X buttons and their gap blocker.
                // Keep the blocker active everywhere outside a node grab area.
                if self.hud_visible() && self.node_hit_at(x, y).is_none() {
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
                                            self.change_lift(|b| b.enabled = !b.enabled);
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
                            let node_y =
                                db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db);
                            let animating = (self.band_dyn_anim_progress.get()
                                - self.band_dyn_anim_target.get())
                            .abs()
                                > 0.01;
                            let layout = band_chrome_layout(&b, self.graph_db, gx, gw, true);
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
                            } else if node_chrome_gap_hit(x, y, node_x, node_y, layout, gx, gw) {
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
                                        if let Some(i) = (2..5).find(|i| {
                                            *i != 1 && inside(x, y, self.hud_dyn_field_rect(&b, *i))
                                        }) {
                                            self.drag = Some(Target::Band(i));
                                            self.last_drag = (x, y);
                                            cx.capture();
                                            self.apply_drag(cx, x, y, false, false, false);
                                        }
                                    }
                                } else {
                                    match hud_row_hit(x, y, g.bx, g.by, b.shape.is_cut()) {
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
                                    node_handle_hit(b, x, y, self.graph_db, self.gx(), self.gw())
                                })
                                .map(|b| b.id);
                            if let Some(id) = hit {
                                self.select(Some(id));
                                self.drag = Some(Target::Node(id));
                                self.last_drag = (x, y);
                                cx.capture();
                            } else {
                                let sr = self.shared.sample_rate.load(Ordering::Relaxed) as f64;
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
                                    self.pending_create = Some(PendingCreate::Background);
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
}
