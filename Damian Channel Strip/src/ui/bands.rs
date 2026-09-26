use super::*;

impl StripView {
    pub(super) fn hud_visible(&self) -> bool {
        self.selected.is_some() && !matches!(self.drag, Some(Target::Node(_)))
    }
    pub(super) fn set_band_dyn_page(&self, on: bool) {
        self.band_dyn_page.set(on);
        self.band_dyn_anim_target.set(if on { 1.0 } else { 0.0 });
    }
    /// Open cog settings when thresh/range adjust begins. No-op if already open or dyn off.
    pub(super) fn open_band_dyn_settings(&self) {
        if self.band_dyn_page.get() {
            return;
        }
        if band_keeps_dyn_page(self.selected.and_then(|id| self.find_band(id)).as_ref()) {
            self.set_band_dyn_page(true);
        }
    }
    pub(super) fn sync_band_dyn_page(&self) {
        if !band_keeps_dyn_page(self.selected.and_then(|id| self.find_band(id)).as_ref()) {
            self.set_band_dyn_page(false);
        }
    }
    pub(super) fn node_hit_at(&self, x: f32, y: f32) -> Option<u64> {
        if !inside(x, y, self.graph_area()) {
            return None;
        }
        self.lift_hit_at(x, y).or_else(|| {
            self.clone_page_bands()
                .iter()
                .rev()
                .find(|b| node_handle_hit(b, x, y, self.graph_db, self.gx(), self.gw()))
                .map(|b| b.id)
        })
    }
    pub(super) fn expand_graph_after_drag(&mut self) {
        let id = match self.drag {
            Some(Target::Node(id) | Target::Range(id) | Target::SoloAudition { id, .. }) => id,
            Some(Target::Value(ValueTarget::Band(1 | 7))) => match self.selected {
                Some(id) => id,
                None => return,
            },
            _ => return,
        };
        let Some(b) = self.find_band(id).filter(|b| b.shape.has_gain()) else {
            return;
        };
        let extent = if b.dynamic {
            b.gain.abs().max((b.gain - b.range).abs())
        } else {
            b.gain.abs()
        };
        if extent >= self.graph_db - 3.0 {
            self.set_graph_range(self.graph_db + 12.0);
        }
    }
    pub(super) fn over_selected_hud(&self, x: f32, y: f32) -> bool {
        if !self.hud_visible() || self.node_hit_at(x, y).is_some() {
            return false;
        }
        let gx = self.gx();
        let gw = self.gw();
        if let Some(sel_id) = self.selected {
            if sel_id >= LIFT_ID_BASE {
                if let Some(b) = self.selected_lift() {
                    if inside(x, y, self.hud_rect_for_lift(&b)) {
                        return true;
                    }
                    let node_x = self.freq_x(b.freq);
                    let node_y = lift_gain_y(b.gain);
                    let layout = if node_chrome_above(b.gain, None) {
                        NodeChromeLayout::Above
                    } else {
                        NodeChromeLayout::Below
                    };
                    return node_chrome_hit(x, y, node_x, node_y, layout, gx, gw).is_some()
                        || node_chrome_gap_hit(x, y, node_x, node_y, layout, gx, gw);
                }
            } else if let Some(b) = self.find_band(sel_id) {
                let g = self.hud_geom(&b);
                if inside(x, y, self.hud_bounds_for(&b))
                    || hud_dyn_btn_hit(g, x, y, gx, gw)
                    || hud_cog_hit(g, x, y, gx, gw)
                {
                    return true;
                }
                let node_x = self.freq_x(b.freq);
                let node_y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db);
                let layout = band_chrome_layout(&b, self.graph_db, gx, gw, true);
                return node_chrome_hit(x, y, node_x, node_y, layout, gx, gw).is_some()
                    || node_chrome_gap_hit(x, y, node_x, node_y, layout, gx, gw);
            }
        }
        false
    }
    pub(super) fn select(&mut self, id: Option<u64>) {
        self.menu = None;
        self.selected = id;
        self.shared
            .selected_id
            .store(id.unwrap_or(0), Ordering::Relaxed);
        if id.is_none() {
            self.shared.solo_id.store(0, Ordering::Relaxed);
        }
        self.sync_band_dyn_page();
    }
    pub(super) fn find_band(&self, id: u64) -> Option<Band> {
        if id >= LIFT_ID_BASE {
            None
        } else if id >= SC_EQ_ID_BASE {
            self.params
                .sc_eq_bands
                .lock()
                .unwrap()
                .iter()
                .find(|b| b.id == id)
                .cloned()
        } else if id >= EQ2_ID_BASE {
            self.params
                .eq2_bands
                .lock()
                .unwrap()
                .iter()
                .find(|b| b.id == id)
                .cloned()
        } else {
            self.params
                .bands
                .lock()
                .unwrap()
                .iter()
                .find(|b| b.id == id)
                .cloned()
        }
    }
    pub(super) fn start_eq_page(&mut self, page: usize) {
        self.anim_start.set(self.anim_progress.get());
        self.anim_target.set(page as f32);
        self.anim_time.set(0.0);
        self.active_eq.set(page);
        self.select(None);
        self.menu = None;
        self.edit = None;
    }
    pub(super) fn clone_page_bands(&self) -> Vec<Band> {
        match self.active_eq.get() {
            EQ_PAGE_2 => self.params.eq2_bands.lock().unwrap().clone(),
            EQ_PAGE_SC => self.params.sc_eq_bands.lock().unwrap().clone(),
            EQ_PAGE_LIFT => Vec::new(),
            _ => self.params.bands.lock().unwrap().clone(),
        }
    }
    pub(super) fn find_threshold_hit(&self, x: f32, y: f32, bands: &[Band]) -> Option<u64> {
        if self.active_eq.get() == EQ_PAGE_SC || self.active_eq.get() == EQ_PAGE_LIFT {
            return None;
        }
        // A node wins even when another band's dynamics handle overlaps it.
        if bands
            .iter()
            .any(|b| node_handle_hit(b, x, y, self.graph_db, self.gx(), self.gw()))
        {
            return None;
        }
        bands
            .iter()
            .rev()
            .find(|b| {
                threshold_handle_hit(
                    &dynamics_control_band(b, self.command_down),
                    x,
                    y,
                    self.graph_db,
                    self.gx(),
                    self.gw(),
                    self.selected,
                )
            })
            .map(|b| b.id)
    }
    pub(super) fn find_range_hit(&self, x: f32, y: f32, bands: &[Band]) -> Option<u64> {
        if self.active_eq.get() == EQ_PAGE_SC || self.active_eq.get() == EQ_PAGE_LIFT {
            return None;
        }
        // A node wins even when another band's dynamics handle overlaps it.
        if bands
            .iter()
            .any(|b| node_handle_hit(b, x, y, self.graph_db, self.gx(), self.gw()))
        {
            return None;
        }
        bands
            .iter()
            .rev()
            .find(|b| {
                range_handle_hit(b, x, y, self.graph_db, self.gx(), self.gw(), self.selected)
                    || (Some(b.id) == self.selected && {
                        let sr = self.shared.sample_rate.load(Ordering::Relaxed) as f64;
                        let eq_sr = self.params.processing_config().mode.rate(sr);
                        range_grip_hit(b, x, y, (self.gx(), self.gw(), self.graph_db), (sr, eq_sr))
                    })
            })
            .map(|b| b.id)
    }
    pub(super) fn begin_threshold_drag(&mut self, cx: &mut EventContext, id: u64, x: f32, y: f32) {
        self.select(Some(id));
        self.change(|b| {
            if !b.dynamic {
                b.dynamic = true;
            }
            if b.shape.has_gain() {
                let geom = dyn_meter_geom(b, self.graph_db, self.gx(), self.gw());
                if cx.modifiers().command() {
                    b.ratio = 1.0 + 19.0 * (geom.y_to_threshold(y) + 60.0) / 60.0;
                } else {
                    b.threshold = geom.y_to_threshold(y);
                }
            }
        });
        self.open_band_dyn_settings();
        self.drag = Some(Target::Threshold(id));
        self.last_drag = (x, y);
        cx.capture();
    }
    pub(super) fn begin_range_drag(&mut self, cx: &mut EventContext, id: u64, x: f32, y: f32) {
        self.select(Some(id));
        self.change(|b| {
            if !b.dynamic {
                b.dynamic = true;
            }
        });
        self.open_band_dyn_settings();
        self.drag = Some(Target::Range(id));
        self.last_drag = (x, y);
        cx.capture();
    }
    pub(super) fn change(&self, f: impl FnOnce(&mut Band)) {
        let Some(id) = self.selected else {
            return;
        };
        if id >= LIFT_ID_BASE {
            return;
        }
        let mut bands = if id >= SC_EQ_ID_BASE {
            self.params.sc_eq_bands.lock().unwrap()
        } else if id >= EQ2_ID_BASE {
            self.params.eq2_bands.lock().unwrap()
        } else {
            self.params.bands.lock().unwrap()
        };
        if let Some(b) = bands.iter_mut().find(|b| b.id == id) {
            f(b);
            b.sanitize();
            if id >= SC_EQ_ID_BASE {
                b.dynamic = false;
            }
        }
    }
    pub(super) fn is_lift_selected(&self) -> bool {
        self.selected.is_some_and(|id| id >= LIFT_ID_BASE)
    }
    pub(super) fn selected_lift(&self) -> Option<LiftBand> {
        if !self.is_lift_selected() {
            return None;
        }
        self.params
            .lift_bands
            .lock()
            .unwrap()
            .iter()
            .find(|b| Some(b.id) == self.selected)
            .cloned()
    }
    pub(super) fn change_lift(&self, f: impl FnOnce(&mut LiftBand)) {
        if let Some(b) = self
            .params
            .lift_bands
            .lock()
            .unwrap()
            .iter_mut()
            .find(|b| Some(b.id) == self.selected)
        {
            f(b);
            b.sanitize();
        }
    }
    pub(super) fn create_band(&mut self, x: f32, y: f32, curve: bool, dynamic: bool) {
        if self.active_eq.get() == EQ_PAGE_LIFT {
            self.create_lift_band(x, y);
            return;
        }
        let page = self.active_eq.get();
        let id_base = match page {
            EQ_PAGE_2 => EQ2_ID_BASE,
            EQ_PAGE_SC => SC_EQ_ID_BASE,
            _ => 0,
        };
        let mut bands = match page {
            EQ_PAGE_2 => self.params.eq2_bands.lock().unwrap(),
            EQ_PAGE_SC => self.params.sc_eq_bands.lock().unwrap(),
            _ => self.params.bands.lock().unwrap(),
        };
        let id = bands.iter().map(|b| b.id).max().unwrap_or(id_base) + 1;
        let shape = infer_shape((x - self.gx()) / self.gw(), (y - GY) / GH, curve);
        bands.push(Band {
            id,
            shape,
            freq: self.x_freq(x),
            gain: if shape.has_gain() {
                y_db(y, self.graph_db)
            } else {
                0.0
            },
            q: if matches!(shape, Shape::LowCut | Shape::HighCut) {
                0.707
            } else {
                1.0
            },
            dynamic: dynamic && shape.has_gain() && page != EQ_PAGE_SC,
            ..Band::default()
        });
        drop(bands);
        self.select(Some(id));
        self.drag = Some(Target::Node(id));
        self.last_drag = (x, y);
    }
    pub(super) fn create_lift_band(&mut self, x: f32, y: f32) {
        let mut lift_bands = self.params.lift_bands.lock().unwrap();
        let id = lift_bands
            .iter()
            .map(|b| b.id)
            .max()
            .unwrap_or(LIFT_ID_BASE)
            + 1;
        let shape = infer_shape((x - self.gx()) / self.gw(), (y - GY) / GH, false);
        lift_bands.push(LiftBand {
            id,
            shape,
            freq: self.x_freq(x).clamp(20.0, 20000.0),
            gain: lift_y_gain(y),
            ..LiftBand::default()
        });
        drop(lift_bands);
        self.select(Some(id));
        self.drag = Some(Target::Node(id));
        self.last_drag = (x, y);
    }
    pub(super) fn lift_hit_at(&self, x: f32, y: f32) -> Option<u64> {
        if self.active_eq.get() != EQ_PAGE_LIFT {
            return None;
        }
        self.params
            .lift_bands
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|b| {
                ((x - self.freq_x(b.freq)).powi(2) + (y - lift_gain_y(b.gain)).powi(2)).sqrt()
                    < 16.0
            })
            .map(|b| b.id)
    }
    pub(super) fn is_auditioning(&self, id: u64) -> bool {
        self.pending_solo == Some(id) || self.shared.solo_id.load(Ordering::Relaxed) == id
    }
    pub(super) fn node_xy(&self, id: u64) -> (f32, f32) {
        if id >= LIFT_ID_BASE {
            if let Some(b) = self.selected_lift() {
                return (self.freq_x(b.freq), lift_gain_y(b.gain));
            }
        } else if let Some(b) = self.find_band(id) {
            let y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db);
            return (self.freq_x(b.freq), y);
        }
        self.down
    }
    pub(super) fn begin_solo_audition_drag(&mut self, id: u64) {
        let restore = self.shared.solo_id.load(Ordering::Relaxed);
        self.shared.solo_id.store(id, Ordering::Relaxed);
        self.pending_solo = None;
        let (nx, ny) = self.node_xy(id);
        self.drag = Some(Target::SoloAudition {
            id,
            restore,
            grab_dx: nx - self.down.0,
            grab_dy: ny - self.down.1,
        });
        self.last_drag = self.down;
    }
    pub(super) fn sync_alt_solo(&mut self, id: u64, alt: bool) {
        if alt {
            if self.alt_solo_restore.is_none() {
                self.alt_solo_restore = Some(self.shared.solo_id.load(Ordering::Relaxed));
            }
            self.shared.solo_id.store(id, Ordering::Relaxed);
        } else if let Some(restore) = self.alt_solo_restore.take() {
            self.shared.solo_id.store(restore, Ordering::Relaxed);
        }
    }
    pub(super) fn apply_drag(
        &mut self,
        cx: &mut EventContext,
        x: f32,
        y: f32,
        shift: bool,
        cmd: bool,
        alt: bool,
    ) {
        let graph_db = self.graph_db;
        let (lx, ly) = self.last_drag;
        let dy = (ly - y) as f64;
        self.last_drag = (x, y);
        match self.drag {
            Some(Target::Value(target)) => {
                let delta = pleasant_ui::pointer::readout_drag_delta(x - lx, y - ly, shift);
                self.adjust_value(cx, target, delta);
            }

            Some(Target::SoloAudition {
                id,
                grab_dx,
                grab_dy,
                ..
            }) => {
                let nx = (x + grab_dx).clamp(self.gx(), self.gx() + self.gw());
                let ny = y + grab_dy;
                if id >= LIFT_ID_BASE {
                    self.change_lift(|b| {
                        b.freq = self.x_freq(nx).clamp(20.0, 20000.0);
                        b.gain = lift_y_gain(ny);
                    });
                } else {
                    self.change(|b| {
                        b.freq = self.x_freq(nx);
                        if b.shape.has_gain() {
                            b.gain = y_db(ny, graph_db);
                        }
                    });
                }
            }
            Some(Target::Node(id)) => {
                self.sync_alt_solo(id, alt);
                if id >= LIFT_ID_BASE {
                    if shift {
                        self.change_lift(|b| {
                            if !(b.shape.is_cut() && b.order == 1) {
                                b.q = q_from_shift_drag(b.q, dy);
                            }
                        });
                    } else if cmd {
                        self.change_lift(|b| {
                            b.threshold = threshold_from_drag(b.threshold, dy);
                        });
                    } else {
                        self.change_lift(|b| {
                            b.freq = self.x_freq(x).clamp(20.0, 20000.0);
                            if b.shape.is_cut() {
                                if b.order > 1 {
                                    b.q = q_from_shift_drag(b.q, dy);
                                }
                            } else {
                                b.gain = lift_y_gain(y);
                            }
                        });
                    }
                } else if shift {
                    self.change(|b| {
                        if !(b.shape.is_cut() && b.order == 1) {
                            b.q = q_from_shift_drag(b.q, dy);
                        }
                    });
                } else if cmd && self.active_eq.get() != EQ_PAGE_SC {
                    self.change(|b| {
                        if !b.shape.has_gain() {
                            return;
                        }
                        if !b.dynamic {
                            b.dynamic = true;
                            b.range = default_dyn_range(graph_db);
                        }
                        b.ratio = (b.ratio + dy * 0.05).clamp(1.0, 20.0);
                    });
                    self.open_band_dyn_settings();
                } else {
                    self.change(|b| {
                        b.freq = self.x_freq(x);
                        if b.shape.has_gain() {
                            b.gain = y_db(y, graph_db);
                        } else if b.shape.is_cut() && b.order > 1 {
                            b.q = q_from_shift_drag(b.q, dy);
                        }
                    });
                }
            }
            Some(Target::Global(i)) => {
                let p = self.param(i);
                if i == 0 && self.dyn_page.get() == DynPage::Main {
                    let r = self.dyn_main_thresh_slider_rect();
                    let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                } else if i == 18 && self.wall_page.get() == WallPage::Main {
                    let r = self.wall_main_thresh_slider_rect();
                    let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                } else if i == 1 && self.pse_page.get() == PsePage::Main {
                    let r = self.pse_main_thresh_slider_rect();
                    let norm = 1.0 - ((y - r.1) / r.3).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                } else if i == 14 && self.dyn_page.get() == DynPage::Main {
                    let r = self.dyn_main_gr_meter_rect();
                    let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                } else if i == 7 && self.pse_page.get() == PsePage::Main {
                    let r = self.pse_main_gr_meter_rect();
                    let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                } else if i == 15 {
                    let r = output_gain_rect();
                    let bar_x = r.0 + 8.0;
                    let bar_w = r.2 - 16.0;
                    let norm = if shift {
                        let cur = p.unmodulated_normalized_value();
                        let dx = x - self.last_drag.0;
                        (cur + (dx / bar_w) * 0.1).clamp(0.0, 1.0)
                    } else {
                        ((x - bar_x) / bar_w).clamp(0.0, 1.0)
                    };
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                } else {
                    let cur = p.unmodulated_normalized_value();
                    let delta = (dy * if shift { 0.0015 } else { 0.007 }) as f32;
                    let norm =
                        (cur + if i == 0 || i == 18 { -delta } else { delta }).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
                }
            }
            Some(Target::CompKnee { from_top }) => {
                let p = &self.params.comp_knee;
                let cur = p.unmodulated_normalized_value();
                let dir = if from_top { 1.0 } else { -1.0 };
                let delta = (dy * dir * if shift { 0.003 } else { 0.012 }) as f32;
                let norm = (cur + delta).clamp(0.0, 1.0);
                cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
            }
            Some(Target::PseKnee { from_top }) => {
                let p = &self.params.pse_knee;
                let cur = p.unmodulated_normalized_value();
                let dir = if from_top { 1.0 } else { -1.0 };
                let delta = (dy * dir * if shift { 0.003 } else { 0.012 }) as f32;
                let norm = (cur + delta).clamp(0.0, 1.0);
                cx.emit(RawParamEvent::SetParameterNormalized(p.as_ptr(), norm));
            }
            Some(Target::Range(_id)) => {
                self.change(|b| {
                    if !b.shape.has_gain() {
                        return;
                    }
                    b.dynamic = true;
                    if shift {
                        b.range = (b.range + dy * 0.1)
                            .clamp(b.gain - graph_db, b.gain + graph_db)
                            .clamp(-MAX_RANGE_DB, MAX_RANGE_DB);
                    } else {
                        b.range = snap_dyn_range(b.gain, y, graph_db);
                    }
                });
            }
            Some(Target::Threshold(_id)) => {
                self.change(|b| {
                    if !b.shape.has_gain() {
                        return;
                    }
                    b.dynamic = true;
                    let geom = dyn_meter_geom(b, graph_db, self.gx(), self.gw());
                    if cmd {
                        b.ratio = 1.0 + 19.0 * (geom.y_to_threshold(y) + 60.0) / 60.0;
                    } else {
                        b.threshold = geom.y_to_threshold(y);
                    }
                });
            }
            Some(Target::Band(i)) => {
                let Some(b) = self.selected.and_then(|id| self.find_band(id)) else {
                    return;
                };
                let r = self.hud_dyn_field_rect(&b, i);
                let bar_x = r.0 + 8.0;
                let bar_w = (r.2 - 16.0).max(1.0);
                let n = ((x - bar_x) / bar_w).clamp(0.0, 1.0) as f64;
                self.change(|b| match i {
                    0 => b.threshold = -60.0 + 60.0 * n,
                    1 => b.range = -MAX_RANGE_DB + 2.0 * MAX_RANGE_DB * n,
                    2 => b.ratio = 1.0 + 19.0 * n,
                    3 => b.attack = 0.1 * 2000.0_f64.powf(n),
                    _ => b.release = 10.0 * 200.0_f64.powf(n),
                });
            }
            Some(Target::LiftBand(i)) => {
                let r = self.band_rect(i);
                let n = ((x - r.0 - 12.0) / (r.2 - 24.0)).clamp(0.0, 1.0) as f64;
                self.change_lift(|b| match i {
                    0 => b.threshold = -60.0 + 60.0 * n,
                    1 => b.ratio = 1.0 + 19.0 * n,
                    2 => b.attack = 0.1 * 2000.0_f64.powf(n),
                    3 => b.release = 10.0 * 200.0_f64.powf(n),
                    _ => b.range = 24.0 * n,
                });
            }
            _ => {}
        }
    }
    pub(super) fn toggle(&self, cx: &mut EventContext, p: &BoolParam) {
        cx.emit(RawParamEvent::BeginSetParameter(p.as_ptr()));
        cx.emit(RawParamEvent::SetParameterNormalized(
            p.as_ptr(),
            if p.value() { 0.0 } else { 1.0 },
        ));
        cx.emit(RawParamEvent::EndSetParameter(p.as_ptr()));
    }
    pub(super) fn reset_float_param(&self, cx: &mut EventContext, p: &FloatParam) {
        let ptr = p.as_ptr();
        cx.emit(RawParamEvent::BeginSetParameter(ptr));
        cx.emit(RawParamEvent::SetParameterNormalized(
            ptr,
            p.default_normalized_value(),
        ));
        cx.emit(RawParamEvent::EndSetParameter(ptr));
    }
    pub(super) fn finish_param_reset(&mut self, cx: &mut EventContext) {
        self.drag = None;
        cx.release();
        cx.needs_redraw();
    }
    pub(super) fn try_reset_at(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        if self.value_at(x, y).is_some() {
            return false;
        }
        if inside(x, y, output_gain_rect()) {
            self.reset_float_param(cx, self.param(15));
            self.finish_param_reset(cx);
            return true;
        }
        if self.pse_page.get() == PsePage::Main
            && (self.pse_anim_progress.get() - self.pse_anim_target.get()).abs() <= 0.01
        {
            let r = self.pse_main_thresh_slider_rect();
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
            let gr = self.pse_main_gr_meter_rect();
            let depth_y = gr.1 + gr.3 * self.params.pse_depth.unmodulated_normalized_value();
            let depth_handle = self.pse_main_depth_handle_rect(depth_y);
            if inside(x, y, handle_hit) {
                self.reset_float_param(cx, self.param(1));
                self.finish_param_reset(cx);
                return true;
            }
            if inside(x, y, knee_rect) {
                self.reset_float_param(cx, &self.params.pse_knee);
                self.finish_param_reset(cx);
                return true;
            }
            if inside(x, y, depth_handle) {
                self.reset_float_param(cx, self.param(7));
                self.finish_param_reset(cx);
                return true;
            }
            if inside(x, y, (r.0 - 10.0, r.1 - 10.0, r.2 + 20.0, r.3 + 20.0)) {
                self.reset_float_param(cx, self.param(1));
                self.finish_param_reset(cx);
                return true;
            }
        }
        if self.dyn_page.get() == DynPage::Main
            && (self.dyn_anim_progress.get() - self.dyn_anim_target.get()).abs() <= 0.01
        {
            let r = self.dyn_main_thresh_slider_rect();
            let gr = self.dyn_main_gr_meter_rect();
            let thresh_norm = self.params.compression.unmodulated_normalized_value();
            let thresh_y = r.1 + r.3 * thresh_norm;
            let handle = self.dyn_main_thresh_handle_rect(thresh_y);
            let handle_hit = (
                handle.0 - 4.0,
                handle.1 - 2.0,
                handle.2 + 8.0,
                handle.3 + 4.0,
            );
            let knee_rect = self.dyn_main_knee_rect(thresh_y, 0.0);
            let depth_y = gr.1 + gr.3 * self.params.comp_depth.unmodulated_normalized_value();
            let depth_handle = self.dyn_main_depth_handle_rect(depth_y);
            if inside(x, y, handle_hit) {
                self.reset_float_param(cx, self.param(0));
                self.finish_param_reset(cx);
                return true;
            }
            if inside(x, y, knee_rect) {
                self.reset_float_param(cx, &self.params.comp_knee);
                self.finish_param_reset(cx);
                return true;
            }
            if inside(x, y, depth_handle) {
                self.reset_float_param(cx, self.param(14));
                self.finish_param_reset(cx);
                return true;
            }
            if inside(
                x,
                y,
                (r.0 - 10.0, r.1 - 10.0, r.2 + gr.2 + 20.0, r.3 + 20.0),
            ) {
                self.reset_float_param(cx, self.param(0));
                self.finish_param_reset(cx);
                return true;
            }
        }
        if self.wall_page.get() == WallPage::Main
            && (self.wall_anim_progress.get() - self.wall_anim_target.get()).abs() <= 0.01
        {
            let r = self.wall_main_thresh_slider_rect();
            let thresh_norm = self.params.wall_threshold.unmodulated_normalized_value();
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
                self.reset_float_param(cx, self.param(18));
                self.finish_param_reset(cx);
                return true;
            }
        }
        if let Some(i) = self
            .global_hit_rects()
            .into_iter()
            .find(|(_, r)| inside(x, y, *r))
            .map(|(i, _)| i)
        {
            self.reset_float_param(cx, self.param(i));
            self.finish_param_reset(cx);
            return true;
        }
        if self.is_lift_selected() {
            if let Some(i) = (1..5).find(|i| {
                inside(x, y, self.band_bar_rect(*i)) && !inside(x, y, self.band_value_rect(*i))
            }) {
                let defaults = LiftBand::default();
                self.change_lift(|b| match i {
                    0 => b.threshold = defaults.threshold,
                    1 => b.ratio = defaults.ratio,
                    2 => b.attack = defaults.attack,
                    3 => b.release = defaults.release,
                    _ => b.range = defaults.range,
                });
                self.finish_param_reset(cx);
                return true;
            }
        } else if let Some(b) = self
            .selected
            .and_then(|id| self.find_band(id))
            .filter(|b| self.band_dyn_page.get() && band_allows_dyn(b))
        {
            if let Some(i) = (0..5).find(|i| {
                inside(x, y, self.hud_dyn_field_rect(&b, *i))
                    && !inside(x, y, self.hud_dyn_value_rect(&b, *i))
            }) {
                let defaults = Band::default();
                self.change(|band| match i {
                    0 => band.threshold = defaults.threshold,
                    1 => band.range = defaults.range,
                    2 => band.ratio = defaults.ratio,
                    3 => band.attack = defaults.attack,
                    _ => band.release = defaults.release,
                });
                self.finish_param_reset(cx);
                return true;
            }
        }
        false
    }
    pub(super) fn delete(&mut self) {
        if let Some(id) = self.selected {
            if id >= LIFT_ID_BASE {
                self.params
                    .lift_bands
                    .lock()
                    .unwrap()
                    .retain(|b| b.id != id);
            } else if id >= SC_EQ_ID_BASE {
                self.params
                    .sc_eq_bands
                    .lock()
                    .unwrap()
                    .retain(|b| b.id != id);
            } else if id >= EQ2_ID_BASE {
                self.params.eq2_bands.lock().unwrap().retain(|b| b.id != id);
            } else {
                self.params.bands.lock().unwrap().retain(|b| b.id != id);
            }
            if self.shared.solo_id.load(Ordering::Relaxed) == id {
                self.shared.solo_id.store(0, Ordering::Relaxed);
            }
            self.select(None);
        }
    }
}
