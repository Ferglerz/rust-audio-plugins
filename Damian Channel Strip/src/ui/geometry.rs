use super::*;

impl StripView {
    pub(super) fn is_pre(&self) -> bool {
        self.params.comp_pre.value()
    }
    pub(super) fn idle_hover(&self) -> Option<(f32, f32)> {
        pleasant_ui::idle_hover(self.hover, self.drag.is_some())
    }
    pub(super) fn eq_bounds(&self) -> (f32, f32, f32, f32) {
        let x = if self.is_pre() {
            MARGIN + PSE_W + GAP + DYN_W + GAP
        } else {
            MARGIN + PSE_W + GAP
        };
        (x, MODULE_Y, EQ_W, MODULE_H)
    }
    pub(super) fn dyn_bounds(&self) -> (f32, f32, f32, f32) {
        let x = if self.is_pre() {
            MARGIN + PSE_W + GAP
        } else {
            UI_W - MARGIN - WALL_W - GAP - DYN_W
        };
        (x, MODULE_Y, DYN_W, MODULE_H)
    }
    pub(super) fn pse_bounds(&self) -> (f32, f32, f32, f32) {
        (MARGIN, MODULE_Y, PSE_W, MODULE_H)
    }
    pub(super) fn wall_bounds(&self) -> (f32, f32, f32, f32) {
        (UI_W - MARGIN - WALL_W, MODULE_Y, WALL_W, MODULE_H)
    }
    pub(super) fn gx(&self) -> f32 {
        self.eq_bounds().0 + EQ_GRAPH_PAD_LEFT
    }
    pub(super) fn gw(&self) -> f32 {
        self.eq_bounds().2 - EQ_GRAPH_PAD_LEFT - EQ_GRAPH_PAD_RIGHT
    }
    pub(super) fn graph_area(&self) -> (f32, f32, f32, f32) {
        (self.gx(), GY, self.gw(), GH)
    }
    pub(super) fn freq_x(&self, freq: f64) -> f32 {
        freq_x_at(freq, self.gx(), self.gw())
    }
    pub(super) fn x_freq(&self, x: f32) -> f64 {
        x_freq_at(x, self.gx(), self.gw())
    }
    pub(super) fn hud_bounds_for(&self, b: &Band) -> (f32, f32, f32, f32) {
        hud_bounds_for_at(b, self.graph_db, self.gx(), self.gw())
    }
    pub(super) fn hud_rect_for_lift(&self, b: &LiftBand) -> (f32, f32, f32, f32) {
        hud_rect_for_lift_at(b, self.graph_db, self.gx(), self.gw())
    }
    pub(super) fn hud_value_rect(&self, b: &Band, i: usize) -> (f32, f32, f32, f32) {
        hud_value_rect_at(b, self.graph_db, i, self.gx(), self.gw())
    }
    pub(super) fn hud_value_rect_lift(&self, b: &LiftBand, i: usize) -> (f32, f32, f32, f32) {
        hud_value_rect_lift_at(b, self.graph_db, i, self.gx(), self.gw())
    }
    pub(super) fn highlighted_dyn_field(&self, b: &Band) -> Option<usize> {
        if !self.band_dyn_page.get() {
            return None;
        }
        (2..5).find(|&i| {
            let target = ValueTarget::Band(i + 2);
            self.drag == Some(Target::Band(i))
                || self.drag == Some(Target::Value(target))
                || self.edit.as_ref().is_some_and(|edit| edit.target == target)
                || self.value_press.is_some_and(|press| press.target == target)
                || self
                    .idle_hover()
                    .is_some_and(|(x, y)| inside(x, y, self.hud_dyn_field_rect(b, i)))
        })
    }

    pub(super) fn hud_dyn_field_rect(&self, b: &Band, i: usize) -> (f32, f32, f32, f32) {
        hud_dyn_field_rect_at(b, self.graph_db, i, self.gx(), self.gw())
    }
    pub(super) fn hud_dyn_value_rect(&self, b: &Band, i: usize) -> (f32, f32, f32, f32) {
        hud_dyn_value_rect_at(b, self.graph_db, i, self.gx(), self.gw())
    }
    pub(super) fn hud_geom(&self, b: &Band) -> HudGeom {
        hud_geom_for_at(b, self.graph_db, self.gx(), self.gw())
    }
    pub(super) fn band_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        band_rect_at(i, self.gx(), self.gw())
    }
    pub(super) fn band_bar_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        band_bar_rect_at(i, self.gx(), self.gw())
    }
    pub(super) fn band_value_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        band_value_rect_at(i, self.gx(), self.gw())
    }
    pub(super) fn dyn_gr_uncapped(&self) -> Vec<(u64, f32)> {
        self.shared
            .dyn_gr_uncapped
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }
    pub(super) fn dyn_band_input(&self) -> Vec<(u64, f32)> {
        self.shared
            .dyn_band_input
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }
    pub(super) fn input_level_for(&self, id: u64, levels: &[(u64, f32)]) -> Option<f64> {
        levels
            .iter()
            .find(|(band_id, _)| *band_id == id)
            .map(|(_, level)| *level as f64)
    }
    pub(super) fn eq_tab_1_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (
            eq_x + 200.0,
            module_header_ctrl_y(),
            48.0,
            MODULE_HEADER_CTRL,
        )
    }
    pub(super) fn eq_tab_2_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (
            eq_x + 252.0,
            module_header_ctrl_y(),
            48.0,
            MODULE_HEADER_CTRL,
        )
    }
    pub(super) fn eq_tab_lift_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (
            eq_x + 304.0,
            module_header_ctrl_y(),
            48.0,
            MODULE_HEADER_CTRL,
        )
    }
    pub(super) fn eq_tab_sc_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (
            eq_x + 356.0,
            module_header_ctrl_y(),
            56.0,
            MODULE_HEADER_CTRL,
        )
    }
    pub(super) fn eq_power_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (
            eq_x + MODULE_HEADER_INSET,
            module_header_ctrl_y(),
            24.0,
            MODULE_HEADER_CTRL,
        )
    }
    pub(super) fn scale_button_rect(&self) -> (f32, f32, f32, f32) {
        let eq_x = self.eq_bounds().0;
        (eq_x, GY - 10.0, EQ_GRAPH_PAD_LEFT, 20.0)
    }
    pub(super) fn eq_header_listen_sc_rect(&self) -> (f32, f32, f32, f32) {
        let eq_b = self.eq_bounds();
        (
            eq_b.0 + eq_b.2 - MODULE_HEADER_INSET - 90.0,
            module_header_ctrl_y(),
            90.0,
            MODULE_HEADER_CTRL,
        )
    }
    pub(super) fn footer_link_rect(&self) -> (f32, f32, f32, f32) {
        let out = output_gain_rect();
        (out.0 - GAP - 56.0, FOOTER_BTN_Y, 56.0, 28.0)
    }
    pub(super) fn footer_auto_rect(&self) -> (f32, f32, f32, f32) {
        let link = self.footer_link_rect();
        (link.0 - 8.0 - 56.0, FOOTER_BTN_Y, 56.0, 28.0)
    }
    pub(super) fn footer_comp_routing_rect(&self) -> (f32, f32, f32, f32) {
        let auto = self.footer_auto_rect();
        (auto.0 - 8.0 - 56.0, FOOTER_BTN_Y, 56.0, 28.0)
    }
    pub(super) fn footer_comp_label_x(&self) -> f32 {
        self.footer_comp_routing_rect().0 - 108.0
    }
    pub(super) fn scale_menu_rect(&self) -> (f32, f32, f32, f32) {
        let btn = self.scale_button_rect();
        let row = dropdown_row_h(SCALE_BUTTON_TEXT);
        (btn.0, btn.1 + btn.3 + 2.0, 72.0, SCALES.len() as f32 * row)
    }
    pub(super) fn dyn_cog_button_rect(&self) -> (f32, f32, f32, f32) {
        let b = self.dyn_bounds();
        module_header_cog_rect(b.0, b.2)
    }
    pub(super) fn pse_cog_button_rect(&self) -> (f32, f32, f32, f32) {
        let b = self.pse_bounds();
        module_header_cog_rect(b.0, b.2)
    }
    pub(super) fn wall_cog_button_rect(&self) -> (f32, f32, f32, f32) {
        let b = self.wall_bounds();
        module_header_cog_rect(b.0, b.2)
    }
    pub(super) fn wall_main_thresh_slider_rect(&self) -> (f32, f32, f32, f32) {
        let (wx, _, ww, _) = self.wall_bounds();
        let sw = 38.0;
        (wx + (ww - sw) * 0.5, METER_TOP, sw, METER_H)
    }
    pub(super) fn wall_main_thresh_handle_rect(&self, thresh_y: f32) -> (f32, f32, f32, f32) {
        let (sx, _, sw, _) = self.wall_main_thresh_slider_rect();
        (sx - 5.0, thresh_y - 8.0, sw + 10.0, 16.0)
    }
    pub(super) fn dyn_power_button_rect(&self) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        (
            dx + MODULE_HEADER_INSET,
            module_header_ctrl_y(),
            24.0,
            MODULE_HEADER_CTRL,
        )
    }
    pub(super) fn pse_power_button_rect(&self) -> (f32, f32, f32, f32) {
        let px = self.pse_bounds().0;
        (
            px + MODULE_HEADER_INSET,
            module_header_ctrl_y(),
            24.0,
            MODULE_HEADER_CTRL,
        )
    }
    pub(super) fn wall_power_button_rect(&self) -> (f32, f32, f32, f32) {
        let wx = self.wall_bounds().0;
        (
            wx + MODULE_HEADER_INSET,
            module_header_ctrl_y(),
            24.0,
            MODULE_HEADER_CTRL,
        )
    }
    pub(super) fn pse_detect_mode_rect(&self) -> (f32, f32, f32, f32) {
        let r = self.pse_knob_rect(10);
        (r.0, r.1 + r.3 + 2.0, r.2, 24.0)
    }
    pub(super) fn dyn_main_thresh_slider_rect(&self) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        (dx + 30.0, METER_TOP, 38.0, METER_H)
    }
    pub(super) fn dyn_main_gr_meter_rect(&self) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        (dx + 84.0, METER_TOP, 16.0, METER_H)
    }
    pub(super) fn dyn_main_thresh_handle_rect(&self, thresh_y: f32) -> (f32, f32, f32, f32) {
        let (sx, _, sw, _) = self.dyn_main_thresh_slider_rect();
        (sx - 5.0, thresh_y - 8.0, sw + 10.0, 16.0)
    }
    pub(super) fn dyn_main_depth_handle_rect(&self, depth_y: f32) -> (f32, f32, f32, f32) {
        let (gx, _, gw, _) = self.dyn_main_gr_meter_rect();
        (gx - 6.0, depth_y - 8.0, gw + 12.0, 16.0)
    }
    pub(super) fn dyn_main_knee_offset(&self) -> f32 {
        let knee_val = self.params.comp_knee.value();
        10.0 + (knee_val / 20.0).clamp(0.0, 1.0) * 45.0
    }
    pub(super) fn dyn_main_knee_rect(&self, thresh_y: f32, bulge_w: f32) -> (f32, f32, f32, f32) {
        let handle = self.dyn_main_thresh_handle_rect(thresh_y);
        let knee_offset = self.dyn_main_knee_offset();
        clamp_knee_to_meter(
            handle.0 - 2.0 - bulge_w,
            handle.1 - knee_offset,
            handle.2 + 4.0 + 2.0 * bulge_w,
            handle.3 + 2.0 * knee_offset,
        )
    }
    pub(super) fn pse_main_thresh_slider_rect(&self) -> (f32, f32, f32, f32) {
        let px = self.pse_bounds().0;
        (px + 30.0, METER_TOP, 38.0, METER_H)
    }
    pub(super) fn pse_main_gr_meter_rect(&self) -> (f32, f32, f32, f32) {
        let px = self.pse_bounds().0;
        (px + 84.0, METER_TOP, 16.0, METER_H)
    }
    pub(super) fn pse_main_thresh_handle_rect(&self, thresh_y: f32) -> (f32, f32, f32, f32) {
        let (sx, _, sw, _) = self.pse_main_thresh_slider_rect();
        (sx - 5.0, thresh_y - 8.0, sw + 10.0, 16.0)
    }
    pub(super) fn pse_main_depth_handle_rect(&self, depth_y: f32) -> (f32, f32, f32, f32) {
        let (gx, _, gw, _) = self.pse_main_gr_meter_rect();
        (gx - 6.0, depth_y - 8.0, gw + 12.0, 16.0)
    }
    pub(super) fn pse_main_knee_offset(&self) -> f32 {
        let knee_val = self.params.pse_knee.value();
        10.0 + (knee_val / 18.0).clamp(0.0, 1.0) * 45.0
    }
    pub(super) fn pse_main_knee_rect(&self, thresh_y: f32, bulge_w: f32) -> (f32, f32, f32, f32) {
        let handle = self.pse_main_thresh_handle_rect(thresh_y);
        let knee_offset = self.pse_main_knee_offset();
        clamp_knee_to_meter(
            handle.0 - 2.0 - bulge_w,
            handle.1 - knee_offset,
            handle.2 + 4.0 + 2.0 * bulge_w,
            handle.3 + 2.0 * knee_offset,
        )
    }
    pub(super) fn pse_knob_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        let slots = stacked_knob_slots(self.pse_bounds().0, PSE_W, 4);
        match i {
            10 => slots[1],
            2 => (slots[2].0, slots[2].1 + 30.0, slots[2].2, slots[2].3),
            _ => slots[0],
        }
    }
    pub(super) fn pse_vad_meter_rect(&self) -> (f32, f32, f32, f32) {
        let r = self.pse_knob_rect(2);
        (r.0 + 8.0, r.1 + r.3 + 12.0, r.2 - 16.0, 10.0)
    }
    pub(super) fn wall_knob_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        let wx = self.wall_bounds().0;
        let slots = stacked_knob_slots(wx, WALL_W, 2);
        match i {
            16 => slots[0],
            17 => slots[1],
            _ => slots[0],
        }
    }
    pub(super) fn global_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        let dx = self.dyn_bounds().0;
        match self.dyn_page.get() {
            DynPage::Controls => {
                let slots = stacked_knob_slots(dx, DYN_W, 5);
                match i {
                    5 => slots[0],
                    11 => slots[1],
                    6 => slots[2],
                    3 => slots[3],
                    4 => slots[4],
                    _ => (0.0, -1000.0, 0.0, 0.0),
                }
            }
            DynPage::Main => {
                if i == 14 {
                    meter_value_rect(self.dyn_main_gr_meter_rect())
                } else if i == 0 {
                    meter_value_rect(self.dyn_main_thresh_slider_rect())
                } else {
                    (dx + 6.0, 226.0, 56.0, 86.0)
                }
            }
        }
    }
    pub(super) fn knob_value_rect(r: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        pleasant_ui::draw::KnobLayout::new(r).value_rect(r.0 + 8.0, r.2 - 16.0)
    }
    pub(super) fn global_value_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        if self.dyn_page.get() == DynPage::Main && matches!(i, 0 | 14) {
            self.global_rect(i)
        } else {
            Self::knob_value_rect(self.global_rect(i))
        }
    }
    pub(super) fn pse_value_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        if self.pse_page.get() == PsePage::Main && i == 1 {
            meter_value_rect(self.pse_main_thresh_slider_rect())
        } else {
            Self::knob_value_rect(self.pse_knob_rect(i))
        }
    }
    pub(super) fn wall_value_rect(&self, i: usize) -> (f32, f32, f32, f32) {
        if self.wall_page.get() == WallPage::Main && i == 18 {
            meter_value_rect(self.wall_main_thresh_slider_rect())
        } else {
            Self::knob_value_rect(self.wall_knob_rect(i))
        }
    }
    pub(super) fn pse_controls(&self) -> &'static [usize] {
        match self.pse_page.get() {
            PsePage::Main => &[1],
            PsePage::Controls => &PSE_CONTROLS_KNOBS,
        }
    }
    pub(super) fn wall_controls(&self) -> &'static [usize] {
        match self.wall_page.get() {
            WallPage::Main => &WALL_MAIN_KNOBS,
            WallPage::Controls => &WALL_CONTROLS_KNOBS,
        }
    }
    pub(super) fn global_hit_rects(&self) -> Vec<(usize, (f32, f32, f32, f32))> {
        let mut rects: Vec<_> = self
            .global_controls()
            .iter()
            .copied()
            .map(|i| (i, self.global_rect(i)))
            .collect();
        rects.extend(self.pse_controls().iter().copied().map(|i| {
            (
                i,
                if self.pse_page.get() == PsePage::Main {
                    self.pse_value_rect(i)
                } else {
                    self.pse_knob_rect(i)
                },
            )
        }));
        if self.wall_page.get() == WallPage::Controls {
            rects.extend(
                self.wall_controls()
                    .iter()
                    .copied()
                    .map(|i| (i, self.wall_knob_rect(i))),
            );
        }
        rects
    }
    pub(super) fn global_value_hits(&self) -> Vec<(ValueTarget, (f32, f32, f32, f32))> {
        let mut fields: Vec<_> = self
            .global_controls()
            .iter()
            .copied()
            .map(|i| (ValueTarget::Global(i), self.global_value_rect(i)))
            .collect();
        fields.extend(
            self.pse_controls()
                .iter()
                .copied()
                .map(|i| (ValueTarget::Global(i), self.pse_value_rect(i))),
        );
        fields.extend(
            self.wall_controls()
                .iter()
                .copied()
                .map(|i| (ValueTarget::Global(i), self.wall_value_rect(i))),
        );
        fields.push((ValueTarget::Global(15), output_gain_value_rect()));
        fields
    }
    pub(super) fn value_baseline(&self, target: ValueTarget) -> f32 {
        match target {
            ValueTarget::Band(_) | ValueTarget::Lift(0..=2) => hud_control_text_y(),
            ValueTarget::Lift(i) => band_rect_at(i - 3, self.gx(), self.gw()).1 + 18.0,
            ValueTarget::Global(15) => output_gain_rect().1 + 13.0,
            ValueTarget::Global(i) => {
                if (matches!(i, 0 | 14) && self.dyn_page.get() == DynPage::Main)
                    || (i == 1 && self.pse_page.get() == PsePage::Main)
                    || (i == 18 && self.wall_page.get() == WallPage::Main)
                {
                    meter_value_y()
                } else {
                    let r = if self.pse_controls().contains(&i) {
                        self.pse_knob_rect(i)
                    } else if self.wall_controls().contains(&i) {
                        self.wall_knob_rect(i)
                    } else {
                        self.global_rect(i)
                    };
                    pleasant_ui::draw::KnobLayout::new(r).value_y
                }
            }
        }
    }

    pub(super) fn value_color(&self, target: ValueTarget) -> C {
        match target {
            ValueTarget::Global(1 | 7 | 8 | 9 | 10) => PSE_BLUE,
            ValueTarget::Global(2 | 3) => TEAL,
            ValueTarget::Global(16 | 17 | 18) => WALL_COLOR,
            ValueTarget::Global(_) => GOLD,
            ValueTarget::Band(_) => self
                .selected
                .map(|id| BAND_COLORS[(band_display_num(id) as usize - 1) % BAND_COLORS.len()])
                .unwrap_or(TEAL),
            ValueTarget::Lift(_) => LIFT_COLOR,
        }
    }
}
