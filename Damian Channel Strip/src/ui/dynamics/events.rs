use super::super::*;

impl StripView {
    pub(in crate::ui) fn handle_dynamics_mouse_down(
        &mut self,
        cx: &mut EventContext,
        x: f32,
        y: f32,
    ) -> bool {
        if inside(x, y, self.dyn_power_button_rect()) {
            self.dyn_bypass_anim.trigger_click();
            self.toggle(cx, &self.params.comp_on);
            cx.needs_redraw();
            return true;
        }
        if inside(x, y, self.pse_power_button_rect()) {
            self.pse_bypass_anim.trigger_click();
            self.toggle(cx, &self.params.pse_on);
            cx.needs_redraw();
            return true;
        }
        if inside(x, y, self.wall_power_button_rect()) {
            self.wall_bypass_anim.trigger_click();
            self.toggle(cx, &self.params.wall_on);
            cx.needs_redraw();
            return true;
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
            return true;
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
            return true;
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
            return true;
        }
        match self.pse_page.get() {
            PsePage::Main => {
                if (self.pse_anim_progress.get() - self.pse_anim_target.get()).abs() > 0.01
                    && inside(x, y, self.pse_bounds())
                {
                    return true;
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
                    return true;
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
                    return true;
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
                    return true;
                }
                if inside(x, y, (r.0 - 10.0, r.1 - 10.0, r.2 + 20.0, r.3 + 20.0)) {
                    self.drag = Some(Target::Global(1));
                    self.last_drag = (x, y);
                    let norm = 1.0 - ((y - r.1) / r.3).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::BeginSetParameter(gate_ptr));
                    cx.emit(RawParamEvent::SetParameterNormalized(gate_ptr, norm));
                    cx.capture();
                    cx.needs_redraw();
                    return true;
                }
            }
            PsePage::Controls => {
                if (self.pse_anim_progress.get() - self.pse_anim_target.get()).abs() > 0.01
                    && inside(x, y, self.pse_bounds())
                {
                    return true;
                }
                if inside(x, y, self.pse_detect_mode_rect()) {
                    self.toggle(cx, &self.params.pse_peak);
                    cx.needs_redraw();
                    return true;
                }
            }
        }
        match self.dyn_page.get() {
            DynPage::Main => {
                if (self.dyn_anim_progress.get() - self.dyn_anim_target.get()).abs() > 0.01
                    && inside(x, y, self.dyn_bounds())
                {
                    return true;
                }
                let r = self.dyn_main_thresh_slider_rect();
                let gr = self.dyn_main_gr_meter_rect();
                let comp_ptr = self.params.compression.as_ptr();
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
                if inside(x, y, handle_hit) {
                    self.drag = Some(Target::Global(0));
                    self.last_drag = (x, y);
                    cx.emit(RawParamEvent::BeginSetParameter(comp_ptr));
                    cx.capture();
                    cx.needs_redraw();
                    return true;
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
                    return true;
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
                    return true;
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
                    return true;
                }
            }
            DynPage::Controls => {
                if (self.dyn_anim_progress.get() - self.dyn_anim_target.get()).abs() > 0.01
                    && inside(x, y, self.dyn_bounds())
                {
                    return true;
                }
            }
        }
        match self.wall_page.get() {
            WallPage::Main => {
                if (self.wall_anim_progress.get() - self.wall_anim_target.get()).abs() > 0.01
                    && inside(x, y, self.wall_bounds())
                {
                    return true;
                }
                let r = self.wall_main_thresh_slider_rect();
                let wall_ptr = self.params.wall_threshold.as_ptr();
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
                    self.drag = Some(Target::Global(18));
                    self.last_drag = (x, y);
                    let norm = ((y - r.1) / r.3).clamp(0.0, 1.0);
                    cx.emit(RawParamEvent::BeginSetParameter(wall_ptr));
                    cx.emit(RawParamEvent::SetParameterNormalized(wall_ptr, norm));
                    cx.capture();
                    cx.needs_redraw();
                    return true;
                }
            }
            WallPage::Controls => {
                if (self.wall_anim_progress.get() - self.wall_anim_target.get()).abs() > 0.01
                    && inside(x, y, self.wall_bounds())
                {
                    return true;
                }
            }
        }
        false
    }
}
