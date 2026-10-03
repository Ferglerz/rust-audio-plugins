use super::super::*;

impl StripView {
    pub(in crate::ui) fn render_dynamics_panels(&self, mut d: &mut Draw) -> (bool, bool) {
        let comp_bypassed = !self.params.comp_on.value();
        let pse_bypassed = !self.params.pse_on.value();
        let (px, py, pw, ph) = self.pse_bounds();
        let (dx, dy, dw, dh) = self.dyn_bounds();
        let is_pre = self.is_pre();

        d.rect(px, py, pw, ph, PANEL);
        d.outline((px, py, pw, ph), LINE);
        let pse_power_r = self.pse_power_button_rect();
        let pse_hovered = self
            .idle_hover()
            .is_some_and(|(hx, hy)| inside(hx, hy, pse_power_r));
        let pse_click = self.pse_bypass_anim.step();
        d.bypass_button(pse_power_r, pse_bypassed, PSE_BLUE, pse_hovered, pse_click);
        d.text(
            px + MODULE_HEADER_INSET + MODULE_HEADER_CTRL + 6.0,
            module_title_y(MODULE_TITLE_SIZE),
            "PSE",
            MODULE_TITLE_SIZE,
            if pse_bypassed { MUTED } else { TEXT },
        );
        let pse_cog_r = self.pse_cog_button_rect();
        draw_module_page_button(
            &mut d,
            pse_cog_r,
            self.pse_page.get() == PsePage::Controls,
            pse_bypassed,
            !pse_bypassed
                && self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, pse_cog_r)),
            PSE_BLUE,
        );

        d.scissor(px, py, pw, ph);
        let pse_eased = quintic_page_progress(self.pse_anim_progress.get());
        let pse_main_off = (0.0 - pse_eased) * pw;
        let pse_ctrl_off = (1.0 - pse_eased) * pw;
        if pse_main_off > -pw && pse_main_off < pw {
            d.offset_x = pse_main_off;
            {
                let (sx, sy, sw, sh) = self.pse_main_thresh_slider_rect();
                let (gx_m, gy_m, gw_m, gh_m) = self.pse_main_gr_meter_rect();
                d.rect(sx, sy, sw, sh, LINE);
                d.rect(gx_m, gy_m, gw_m, gh_m, LINE);

                let sc_level = if pse_bypassed {
                    -90.0
                } else {
                    self.shared.sc_level.load(Ordering::Relaxed) as f64
                };
                let sc_db = sc_level.clamp(-48.0, 0.0);
                let sig_frac = ((sc_db + 48.0) / 48.0) as f32;
                let sig_h = sh * sig_frac;
                let sig_top_y = sy + sh - sig_h;

                let p1 = self.param(1);
                let thresh_norm = p1.unmodulated_normalized_value();
                let thresh_y = sy + sh * (1.0 - thresh_norm);

                if !pse_bypassed && sig_h > 0.5 {
                    let below_thresh_top = sig_top_y.max(thresh_y);
                    let below_thresh_h = (sy + sh) - below_thresh_top;
                    if below_thresh_h > 0.0 {
                        d.rect(
                            sx,
                            below_thresh_top,
                            sw,
                            below_thresh_h,
                            C::rgb(45, 120, 165),
                        );
                    }
                    if sig_top_y < thresh_y {
                        d.rect(sx, sig_top_y, sw, thresh_y - sig_top_y, PSE_BLUE);
                    }
                }

                let pse_gr = if pse_bypassed {
                    0.0
                } else {
                    self.shared.pse_gr.load(Ordering::Relaxed)
                };
                let depth_norm = self.param(7).unmodulated_normalized_value();
                let pse_gr = catch_gr_to_depth(pse_gr, depth_norm * 20.0);
                let gr_bar_h = gh_m * (pse_gr / 20.0).clamp(0.0, 1.0);
                if !pse_bypassed && gr_bar_h > 0.5 {
                    d.rect(gx_m, gy_m, gw_m, gr_bar_h, PSE_BLUE);
                }

                let depth_y = gy_m + gh_m * depth_norm;
                let depth_handle = self.pse_main_depth_handle_rect(depth_y);
                let depth_hover = !pse_bypassed
                    && (self
                        .idle_hover()
                        .is_some_and(|(hx, hy)| inside(hx, hy, depth_handle))
                        || self.drag == Some(Target::Global(7)));
                let handle_color = if pse_bypassed {
                    MUTED
                } else if depth_hover {
                    TEXT
                } else {
                    PSE_BLUE
                };
                draw_gr_depth_handle(&mut d, gx_m, gw_m, depth_y, handle_color);

                let handle = self.pse_main_thresh_handle_rect(thresh_y);
                let handle_hit = (
                    handle.0 - 4.0,
                    handle.1 - 2.0,
                    handle.2 + 8.0,
                    handle.3 + 4.0,
                );
                let knee_norm = (self.params.pse_knee.value() / 18.0).clamp(0.0, 1.0);
                let knee_hover = !pse_bypassed
                    && (self.idle_hover().is_some_and(|(hx, hy)| {
                        inside(hx, hy, self.pse_main_knee_rect(thresh_y, 0.0))
                            && !inside(hx, hy, handle_hit)
                    }) || matches!(self.drag, Some(Target::PseKnee { .. })));
                let bulge_w = knee_pulse_bulge(&self.pse_knee_bulge, knee_hover);
                draw_knee_zone(
                    &mut d,
                    self.pse_main_knee_rect(thresh_y, 0.0),
                    bulge_w,
                    knee_norm,
                    knee_hover,
                    if pse_bypassed { MUTED } else { PSE_BLUE },
                    pse_bypassed,
                );
                let handle_hover = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, handle_hit))
                    || self.drag == Some(Target::Global(1));
                d.grab_bar(
                    handle,
                    if pse_bypassed {
                        MUTED
                    } else if handle_hover {
                        TEXT
                    } else {
                        PSE_BLUE
                    },
                );

                let gate_val_str = if p1.value() <= -79.9 {
                    "OFF".to_owned()
                } else {
                    p1.normalized_value_to_string(p1.unmodulated_normalized_value(), true)
                };
                let pse_dragging_knee = matches!(self.drag, Some(Target::PseKnee { .. }));
                let pse_dragging_depth = self.drag == Some(Target::Global(7));
                if pse_dragging_knee || pse_dragging_depth {
                    let readout = if pse_dragging_depth {
                        format!(
                            "MAX: {}",
                            format_gr_meter_readout(self.params.pse_depth.value())
                        )
                    } else {
                        format_knee_readout(self.params.pse_knee.value())
                    };
                    let knee_x = (rect_center_x(self.pse_main_thresh_slider_rect())
                        + rect_center_x(self.pse_main_gr_meter_rect()))
                        * 0.5;
                    d.text_centered(
                        knee_x,
                        meter_value_y(),
                        &readout,
                        HUD_VALUE_SIZE,
                        if pse_bypassed { MUTED } else { PSE_BLUE },
                    );
                } else {
                    d.text_centered(
                        rect_center_x(self.pse_main_thresh_slider_rect()),
                        meter_value_y(),
                        &gate_val_str,
                        HUD_VALUE_SIZE,
                        if pse_bypassed { MUTED } else { PSE_BLUE },
                    );
                    let pse_gr_text =
                        format_gr_meter_readout(self.shared.pse_gr_peak.load(Ordering::Relaxed));
                    d.text_centered(
                        rect_center_x(self.pse_main_gr_meter_rect()),
                        meter_value_y(),
                        &pse_gr_text,
                        HUD_VALUE_SIZE,
                        if pse_bypassed { MUTED } else { PSE_BLUE },
                    );
                }
            }
        }
        if pse_ctrl_off > -pw && pse_ctrl_off < pw {
            d.offset_x = pse_ctrl_off;
            {
                for (i, label) in [(8, "HYSTERESIS"), (10, "TIME"), (2, "VOICE DET")] {
                    let p = self.param(i);
                    let value = if i == 10 {
                        format_pse_time(p.value() as f64, self.params.pse_peak.value())
                    } else {
                        p.normalized_value_to_string(p.unmodulated_normalized_value(), true)
                    };
                    d.knob(
                        self.pse_knob_rect(i),
                        label,
                        &value,
                        p.unmodulated_normalized_value(),
                        if i == 2 { TEAL } else { PSE_BLUE },
                        pse_bypassed,
                    );
                }

                let time_r = self.pse_knob_rect(10);
                let det_r = self.pse_detect_mode_rect();
                let joined_card = (
                    time_r.0 - 2.0,
                    time_r.1 - 2.0,
                    time_r.2 + 4.0,
                    (det_r.1 + det_r.3) - time_r.1 + 4.0,
                );
                d.outline(joined_card, LINE);
                let is_peak = self.params.pse_peak.value();
                d.button_tinted(
                    det_r,
                    if is_peak { "PEAK" } else { "RMS" },
                    true,
                    if pse_bypassed {
                        MUTED
                    } else {
                        PSE_BLUE
                    },
                );

                let speech_env = self
                    .shared
                    .speech_env
                    .load(Ordering::Relaxed)
                    .clamp(0.0, 1.0);
                let (bar_x, bar_y, bar_w, bar_h) = self.pse_vad_meter_rect();
                d.rect(bar_x, bar_y, bar_w, bar_h, rgb(28, 33, 40));
                d.outline((bar_x, bar_y, bar_w, bar_h), LINE);
                if !pse_bypassed && speech_env > 0.02 {
                    d.rect(
                        bar_x + 1.0,
                        bar_y + 1.0,
                        (bar_w - 2.0) * speech_env,
                        bar_h - 2.0,
                        TEAL,
                    );
                }
                let status_text = if pse_bypassed || self.params.pse_voice_det.value() <= 0.0 {
                    "OFF"
                } else if speech_env > 0.35 {
                    "VOICE"
                } else if speech_env > 0.05 {
                    "DETECT"
                } else {
                    "IDLE"
                };
                d.text_centered(
                    bar_x + bar_w * 0.5,
                    bar_y + bar_h + 18.0,
                    status_text,
                    10.0,
                    if pse_bypassed || status_text == "OFF" {
                        MUTED
                    } else {
                        TEAL
                    },
                );
            }
        }
        d.reset_scissor();
        d.offset_x = 0.0;

        d.rect(dx, dy, dw, dh, PANEL);
        d.outline((dx, dy, dw, dh), LINE);
        let dyn_power_r = self.dyn_power_button_rect();
        let dyn_hovered = self
            .idle_hover()
            .is_some_and(|(hx, hy)| inside(hx, hy, dyn_power_r));
        let dyn_click = self.dyn_bypass_anim.step();
        d.bypass_button(dyn_power_r, comp_bypassed, GOLD, dyn_hovered, dyn_click);
        d.text(
            dx + MODULE_HEADER_INSET + MODULE_HEADER_CTRL + 6.0,
            module_title_y(MODULE_TITLE_SIZE),
            "COMP",
            MODULE_TITLE_SIZE,
            if comp_bypassed { MUTED } else { TEXT },
        );
        let dyn_cog_r = self.dyn_cog_button_rect();
        draw_module_page_button(
            &mut d,
            dyn_cog_r,
            self.dyn_page.get() == DynPage::Controls,
            comp_bypassed,
            !comp_bypassed
                && self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, dyn_cog_r)),
            GOLD,
        );

        d.scissor(dx, dy, dw, dh);
        let dyn_eased = quintic_page_progress(self.dyn_anim_progress.get());
        let dyn_main_off = (0.0 - dyn_eased) * dw;
        let dyn_ctrl_off = (1.0 - dyn_eased) * dw;
        if dyn_main_off > -dw && dyn_main_off < dw {
            d.offset_x = dyn_main_off;
            {
                let (sx, sy, sw, sh) = self.dyn_main_thresh_slider_rect();
                let (gx_m, gy_m, gw_m, gh_m) = self.dyn_main_gr_meter_rect();
                d.rect(sx, sy, sw, sh, LINE);
                d.rect(gx_m, gy_m, gw_m, gh_m, LINE);

                let sc_level = if comp_bypassed {
                    -90.0
                } else {
                    self.shared.sc_level.load(Ordering::Relaxed) as f64
                };
                let sc_db = sc_level.clamp(-48.0, 0.0);
                let sig_frac = ((sc_db + 48.0) / 48.0) as f32;
                let sig_h = sh * sig_frac;
                let sig_top_y = sy + sh - sig_h;

                let p0 = self.param(0);
                let thresh_norm = p0.unmodulated_normalized_value();
                let thresh_y = sy + sh * thresh_norm;

                if !comp_bypassed && sig_h > 0.5 {
                    let below_thresh_top = sig_top_y.max(thresh_y);
                    let below_thresh_h = (sy + sh) - below_thresh_top;
                    if below_thresh_h > 0.0 {
                        d.rect(
                            sx,
                            below_thresh_top,
                            sw,
                            below_thresh_h,
                            C::rgb(70, 160, 140),
                        );
                    }
                    if sig_top_y < thresh_y {
                        d.rect(sx, sig_top_y, sw, thresh_y - sig_top_y, GOLD);
                    }
                }

                let gr = if comp_bypassed {
                    0.0
                } else {
                    self.shared.gr.load(Ordering::Relaxed)
                };
                let gr_uncapped = if comp_bypassed {
                    0.0
                } else {
                    self.shared.gr_uncapped.load(Ordering::Relaxed)
                };
                let depth_norm = self.param(14).unmodulated_normalized_value();
                let gr = catch_gr_to_depth(gr, depth_norm * GR_METER_DB);
                let (gr_bar_h, gr_uncapped_h) = gr_meter_bar_heights(gr, gr_uncapped, gh_m);
                if !comp_bypassed && gr_uncapped_h > gr_bar_h + 0.5 {
                    d.rect(gx_m, gy_m + gr_bar_h, gw_m, gr_uncapped_h - gr_bar_h, MUTED);
                }
                if !comp_bypassed && gr_bar_h > 0.5 {
                    d.rect(gx_m, gy_m, gw_m, gr_bar_h, GOLD);
                }

                let depth_y = gy_m + gh_m * depth_norm;
                let depth_handle = self.dyn_main_depth_handle_rect(depth_y);
                let depth_hover = !comp_bypassed
                    && (self
                        .idle_hover()
                        .is_some_and(|(hx, hy)| inside(hx, hy, depth_handle))
                        || self.drag == Some(Target::Global(14)));
                let handle_color = if comp_bypassed {
                    MUTED
                } else if depth_hover {
                    TEXT
                } else {
                    GOLD
                };
                draw_gr_depth_handle(&mut d, gx_m, gw_m, depth_y, handle_color);

                let handle = self.dyn_main_thresh_handle_rect(thresh_y);
                let handle_hit = (
                    handle.0 - 4.0,
                    handle.1 - 2.0,
                    handle.2 + 8.0,
                    handle.3 + 4.0,
                );
                let knee_norm = (self.params.comp_knee.value() / 20.0).clamp(0.0, 1.0);
                let knee_hover = !comp_bypassed
                    && (self.idle_hover().is_some_and(|(hx, hy)| {
                        inside(hx, hy, self.dyn_main_knee_rect(thresh_y, 0.0))
                            && !inside(hx, hy, handle_hit)
                    }) || matches!(self.drag, Some(Target::CompKnee { .. })));
                let bulge_w = knee_pulse_bulge(&self.knee_bulge, knee_hover);
                draw_knee_zone(
                    &mut d,
                    self.dyn_main_knee_rect(thresh_y, 0.0),
                    bulge_w,
                    knee_norm,
                    knee_hover,
                    if comp_bypassed { MUTED } else { GOLD },
                    comp_bypassed,
                );
                let handle_hover = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, handle_hit))
                    || self.drag == Some(Target::Global(0));
                d.grab_bar(
                    handle,
                    if comp_bypassed {
                        MUTED
                    } else if handle_hover {
                        TEXT
                    } else {
                        GOLD
                    },
                );

                let thresh_val_str =
                    p0.normalized_value_to_string(p0.unmodulated_normalized_value(), true);
                let comp_dragging_knee = matches!(self.drag, Some(Target::CompKnee { .. }));
                let comp_dragging_depth = self.drag == Some(Target::Global(14));
                if comp_dragging_knee || comp_dragging_depth {
                    let readout = if comp_dragging_depth {
                        format!(
                            "MAX: {}",
                            format_gr_meter_readout(self.params.comp_depth.value())
                        )
                    } else {
                        format_knee_readout(self.params.comp_knee.value())
                    };
                    let knee_x = (rect_center_x(self.dyn_main_thresh_slider_rect())
                        + rect_center_x(self.dyn_main_gr_meter_rect()))
                        * 0.5;
                    d.text_centered(
                        knee_x,
                        meter_value_y(),
                        &readout,
                        HUD_VALUE_SIZE,
                        if comp_bypassed { MUTED } else { GOLD },
                    );
                } else {
                    d.text_centered(
                        rect_center_x(self.dyn_main_thresh_slider_rect()),
                        meter_value_y(),
                        &thresh_val_str,
                        HUD_VALUE_SIZE,
                        if comp_bypassed { MUTED } else { GOLD },
                    );
                    let comp_gr_text =
                        format_gr_meter_readout(self.shared.gr_peak.load(Ordering::Relaxed));
                    d.text_centered(
                        rect_center_x(self.dyn_main_gr_meter_rect()),
                        meter_value_y(),
                        &comp_gr_text,
                        HUD_VALUE_SIZE,
                        if comp_bypassed { MUTED } else { GOLD },
                    );
                }
            }
        }
        if dyn_ctrl_off > -dw && dyn_ctrl_off < dw {
            d.offset_x = dyn_ctrl_off;
            {
                for (i, label, color) in [
                    (5, "ATTACK", GOLD),
                    (11, "RELEASE", GOLD),
                    (6, "RATIO", GOLD),
                    (3, "DRY", TEAL),
                    (4, "WET", GOLD),
                ] {
                    let p = self.param(i);
                    let value =
                        p.normalized_value_to_string(p.unmodulated_normalized_value(), true);
                    d.knob(
                        self.global_rect(i),
                        label,
                        &value,
                        p.unmodulated_normalized_value(),
                        color,
                        comp_bypassed,
                    );
                }
            }
        }
        d.reset_scissor();
        d.offset_x = 0.0;

        let wall_bypassed = !self.params.wall_on.value();
        let (wx, wy, ww, wh) = self.wall_bounds();
        d.rect(wx, wy, ww, wh, PANEL);
        d.outline((wx, wy, ww, wh), LINE);
        let wall_power_r = self.wall_power_button_rect();
        let wall_hovered = self
            .idle_hover()
            .is_some_and(|(hx, hy)| inside(hx, hy, wall_power_r));
        let wall_click = self.wall_bypass_anim.step();
        d.bypass_button(
            wall_power_r,
            wall_bypassed,
            WALL_COLOR,
            wall_hovered,
            wall_click,
        );
        d.text(
            wx + MODULE_HEADER_INSET + MODULE_HEADER_CTRL + 6.0,
            module_title_y(MODULE_TITLE_SIZE),
            "WALL",
            MODULE_TITLE_SIZE,
            if wall_bypassed { MUTED } else { TEXT },
        );
        let wall_cog_r = self.wall_cog_button_rect();
        draw_module_page_button(
            &mut d,
            wall_cog_r,
            self.wall_page.get() == WallPage::Controls,
            wall_bypassed,
            !wall_bypassed
                && self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, wall_cog_r)),
            WALL_COLOR,
        );

        d.scissor(wx, wy, ww, wh);
        let wall_eased = quintic_page_progress(self.wall_anim_progress.get());
        let wall_main_off = (0.0 - wall_eased) * ww;
        let wall_ctrl_off = (1.0 - wall_eased) * ww;
        if wall_main_off > -ww && wall_main_off < ww {
            d.offset_x = wall_main_off;
            {
                let (sx, sy, sw, sh) = self.wall_main_thresh_slider_rect();
                d.rect(sx, sy, sw, sh, LINE);

                let wall_level = if wall_bypassed {
                    -90.0
                } else {
                    self.shared.wall_level.load(Ordering::Relaxed) as f64
                };
                let wall_db = wall_level.clamp(-48.0, 0.0);
                let sig_frac = ((wall_db + 48.0) / 48.0) as f32;
                let sig_h = sh * sig_frac;
                let sig_top_y = sy + sh - sig_h;

                let p18 = self.param(18);
                let thresh_norm = p18.unmodulated_normalized_value();
                let thresh_y = sy + sh * thresh_norm;

                if !wall_bypassed && sig_h > 0.5 {
                    let below_thresh_top = sig_top_y.max(thresh_y);
                    let below_thresh_h = (sy + sh) - below_thresh_top;
                    if below_thresh_h > 0.0 {
                        d.rect(
                            sx,
                            below_thresh_top,
                            sw,
                            below_thresh_h,
                            C::rgb(70, 160, 140),
                        );
                    }
                    if sig_top_y < thresh_y {
                        d.rect(sx, sig_top_y, sw, thresh_y - sig_top_y, WALL_COLOR);
                    }
                }

                let handle = self.wall_main_thresh_handle_rect(thresh_y);
                let handle_hit = (
                    handle.0 - 4.0,
                    handle.1 - 2.0,
                    handle.2 + 8.0,
                    handle.3 + 4.0,
                );
                let handle_hover = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, handle_hit))
                    || self.drag == Some(Target::Global(18));
                d.grab_bar(
                    handle,
                    if wall_bypassed {
                        MUTED
                    } else if handle_hover {
                        TEXT
                    } else {
                        WALL_COLOR
                    },
                );

                let thresh_val =
                    p18.normalized_value_to_string(p18.unmodulated_normalized_value(), true);
                d.text_centered(
                    rect_center_x(self.wall_main_thresh_slider_rect()),
                    meter_value_y(),
                    &thresh_val,
                    HUD_VALUE_SIZE,
                    if wall_bypassed { MUTED } else { WALL_COLOR },
                );
            }
        }
        if wall_ctrl_off > -ww && wall_ctrl_off < ww {
            d.offset_x = wall_ctrl_off;
            {
                for (i, label) in [(16, "EVEN"), (17, "ODD")] {
                    let p = self.param(i);
                    let value =
                        p.normalized_value_to_string(p.unmodulated_normalized_value(), true);
                    d.knob(
                        self.wall_knob_rect(i),
                        label,
                        &value,
                        p.unmodulated_normalized_value(),
                        WALL_COLOR,
                        wall_bypassed,
                    );
                }
            }
        }
        d.reset_scissor();
        d.offset_x = 0.0;

        (comp_bypassed, is_pre)
    }
}
