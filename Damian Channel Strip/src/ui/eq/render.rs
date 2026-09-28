use super::super::*;

impl StripView {
    pub(in crate::ui) fn render_eq_panel(
        &self,
        mut d: &mut Draw,
        next_p: f32,
        target_p: f32,
    ) -> (f64, Config, f32, f32) {
        let offset_1 = (0.0 - next_p) * GRAPH_CLIP_W;
        let offset_2 = (1.0 - next_p) * GRAPH_CLIP_W;
        let offset_lift = (EQ_PAGE_LIFT as f32 - next_p) * GRAPH_CLIP_W;
        let offset_sc = (EQ_PAGE_SC as f32 - next_p) * GRAPH_CLIP_W;

        let page = self.active_eq.get();
        let eq1_on = self.params.eq_on.value();
        let eq2_on = self.params.eq2_on.value();
        let lift_on = self.params.lift_on.value();
        let sc_eq_on = self.params.sc_eq_on.value();
        let active_bypassed = match page {
            EQ_PAGE_2 => !eq2_on,
            EQ_PAGE_LIFT => !lift_on,
            EQ_PAGE_SC => !sc_eq_on,
            _ => !eq1_on,
        };
        let bands = self.params.bands.lock().unwrap().clone();
        let eq2_bands = self.params.eq2_bands.lock().unwrap().clone();
        let sc_eq_bands = self.params.sc_eq_bands.lock().unwrap().clone();
        let lift_bands = self.params.lift_bands.lock().unwrap().clone();

        let eq_b = self.eq_bounds();
        let eq_x = eq_b.0;
        let gx = self.gx();
        let gw = self.gw();
        d.rect(eq_b.0, eq_b.1, eq_b.2, eq_b.3, PANEL);
        d.outline(eq_b, LINE);

        let eq_power_r = self.eq_power_rect();
        let eq_power_hovered = self
            .idle_hover()
            .is_some_and(|(hx, hy)| inside(hx, hy, eq_power_r));
        let eq_power_click = self.eq_bypass_anim.step();
        d.bypass_button(
            eq_power_r,
            active_bypassed,
            TEAL,
            eq_power_hovered,
            eq_power_click,
        );
        d.text(
            eq_x + MODULE_HEADER_INSET + MODULE_HEADER_CTRL + 6.0,
            module_title_y(16.0),
            match page {
                EQ_PAGE_LIFT => "LIFT",
                EQ_PAGE_SC => "SIDECHAIN EQ",
                _ => "PARAMETRIC EQ",
            },
            16.0,
            if active_bypassed { MUTED } else { TEXT },
        );
        d.tab_button(
            self.eq_tab_1_rect(),
            "EQ 1",
            page == EQ_PAGE_1,
            if eq1_on { TEAL } else { MUTED },
        );
        d.tab_button(
            self.eq_tab_2_rect(),
            "EQ 2",
            page == EQ_PAGE_2,
            if eq2_on { TEAL } else { MUTED },
        );
        d.tab_button(
            self.eq_tab_lift_rect(),
            "LIFT",
            page == EQ_PAGE_LIFT,
            if lift_on { LIFT_COLOR } else { MUTED },
        );
        d.tab_button(
            self.eq_tab_sc_rect(),
            "SC EQ",
            page == EQ_PAGE_SC,
            if sc_eq_on { TEAL } else { MUTED },
        );
        if page == EQ_PAGE_SC {
            d.button(
                self.eq_header_listen_sc_rect(),
                "LISTEN SC",
                self.params.pse_listen.value(),
                TEAL,
            );
        }

        if active_bypassed {
            d.text(
                eq_x + 478.0,
                module_title_y(12.0),
                match page {
                    EQ_PAGE_2 => "EQ 2 STAGE BYPASSED",
                    EQ_PAGE_LIFT => "LIFT STAGE BYPASSED",
                    EQ_PAGE_SC => "SC EQ STAGE BYPASSED",
                    _ => "EQ 1 STAGE BYPASSED",
                },
                12.0,
                GOLD,
            );
        } else if self.shared.solo_id.load(Ordering::Relaxed) != 0 {
            d.text(
                eq_x + 478.0,
                module_title_y(12.0),
                "SOLO AUDITION ACTIVE",
                12.0,
                GOLD,
            );
        }

        let spectrum: Vec<_> = (0..128)
            .map(|i| {
                let db = self.shared.spectrum[i].load(Ordering::Relaxed);
                (
                    gx + i as f32 / 127.0 * gw,
                    GY + GH - (db + 90.0).clamp(0.0, 90.0) / 90.0 * GH * 0.86,
                )
            })
            .collect();
        let sr = self.shared.sample_rate.load(Ordering::Relaxed) as f64;
        let config = self.params.processing_config();
        let eq_sr = config.mode.rate(sr);

        let mut raw_lift_gr = [0.0_f64; 256];
        let mut raw_lift_uncapped = [0.0_f64; 256];
        for k in 0..256 {
            raw_lift_gr[k] = self.shared.lift_bin_gr[k].load(Ordering::Relaxed) as f64;
            raw_lift_uncapped[k] =
                self.shared.lift_bin_gr_uncapped[k].load(Ordering::Relaxed) as f64;
        }
        let smooth_lift_gr = smooth_lift_bins(&raw_lift_gr);
        let smooth_lift_uncapped = smooth_lift_bins(&raw_lift_uncapped);
        let dyn_uncapped = self
            .shared
            .dyn_gr_uncapped
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|e| e.into_inner().clone());
        let dyn_input = self.dyn_band_input();

        let scale_btn = self.scale_button_rect();
        let scale_menu_r = self.scale_menu_rect();
        let draw_grid_and_spectrum = |d: &mut Draw, bypassed: bool, eq_idx: usize| {
            let step = if self.graph_db <= 6.0 { 6 } else { 12 };
            let steps = (self.graph_db / step as f64).floor() as i32;
            for db in (-steps..=steps).map(|i| i * step) {
                let y = db_y(db as f64, self.graph_db);
                d.line(
                    gx,
                    y,
                    gx + gw,
                    y,
                    if db == 0 { C::rgb(75, 81, 85) } else { LINE },
                    1.0,
                );
                if db != self.graph_db as i32 {
                    d.text_centered(
                        gx - EQ_GRAPH_PAD_LEFT * 0.5,
                        axis_label_y(y, 13.0),
                        &axis_db_text(db),
                        13.0,
                        MUTED,
                    );
                }
            }
            for (freq, label) in FREQ_AXIS {
                let x = freq_x_at(freq, gx, gw);
                d.line(x, GY, x, GRAPH_BOTTOM, LINE, 1.0);
                if !self.hud_visible() {
                    d.text_centered(x, EQ_AXIS_LABEL_Y, label, 12.0, MUTED);
                }
            }
            d.area(
                &spectrum,
                GY + GH,
                if bypassed {
                    C::rgba(80, 90, 100, 10)
                } else {
                    C::rgba(125, 143, 159, 28)
                },
            );
            d.poly_above(
                &spectrum,
                GY + GH,
                if bypassed {
                    C::rgba(90, 100, 110, 20)
                } else {
                    C::rgba(144, 161, 175, 62)
                },
                1.0,
            );
            let scale_hover = self
                .idle_hover()
                .is_some_and(|(hx, hy)| inside(hx, hy, scale_btn));
            d.rect(scale_btn.0, scale_btn.1, scale_btn.2, scale_btn.3, PANEL);
            d.text_centered(
                scale_btn.0 + scale_btn.2 * 0.5,
                axis_label_y(GY, SCALE_BUTTON_TEXT),
                &format!("±{} ▾", self.graph_db as i32),
                SCALE_BUTTON_TEXT,
                if scale_hover { TEXT } else { MUTED },
            );
            if self.scale_menu && self.active_eq.get() == eq_idx {
                let r = scale_menu_r;
                let row_h = dropdown_row_h(SCALE_BUTTON_TEXT);
                d.rect(r.0, r.1, r.2, r.3, PANEL);
                d.outline(r, LINE);
                for (i, range) in SCALES.iter().enumerate() {
                    let row = (r.0, r.1 + i as f32 * row_h, r.2, row_h);
                    if *range == self.graph_db {
                        d.rect(row.0, row.1, row.2, row.3, LINE);
                    }
                    d.text(
                        row.0 + 10.0,
                        row.1 + row_h * 0.5 + SCALE_BUTTON_TEXT * 0.32,
                        &format!("±{}", *range as i32),
                        SCALE_BUTTON_TEXT,
                        TEXT,
                    );
                }
            }
        };

        d.scissor(eq_b.0, eq_b.1, eq_b.2, eq_b.3);

        // --- DRAW EQ 1 (if visible) ---
        if offset_1 > -GRAPH_CLIP_W && offset_1 < GRAPH_CLIP_W {
            d.offset_x = offset_1;
            let eq1_bypassed = !eq1_on;
            draw_grid_and_spectrum(&mut d, eq1_bypassed, 0);

            let extras = bands.iter().filter(|b| b.enabled).map(|b| b.freq);
            let xs = eq_curve_xs(gx, gw, extras);
            let mut sum = vec![0.0; xs.len()];
            let mut selected_curve: Option<(Vec<(f32, f32)>, C, bool)> = None;
            for b in &bands {
                if !b.enabled {
                    continue;
                }
                let coeff = BandCoeffs::make(&plot_band(b, &dyn_uncapped), eq_sr);
                let color = if eq1_bypassed {
                    MUTED
                } else {
                    BAND_COLORS[(b.id as usize - 1) % BAND_COLORS.len()]
                };
                let dbs = eq_db_on_xs(&coeff, &xs, gx, gw, sr, eq_sr);
                for (i, db) in dbs.iter().enumerate() {
                    sum[i] += db;
                }
                if Some(b.id) == self.selected {
                    let points = eq_points(&xs, &dbs, self.graph_db);
                    let mut fill = color;
                    fill.a = if eq1_bypassed { 0.02 } else { 0.07 };
                    let between = b.dynamic && b.shape.has_gain();
                    let fill_points = if between {
                        let range = range_curve_points(
                            b, &xs, (gx, gw, self.graph_db), (sr, eq_sr),
                        );
                        let polygon = between_curve_polygon(&points, &range);
                        d.fill_poly(&polygon, fill);
                        polygon
                    } else {
                        d.area(&points, db_y(0.0, self.graph_db), fill);
                        points.clone()
                    };
                    d.poly(&points, color, 1.2);
                    selected_curve = Some((fill_points, color, between));
                }
            }
            let points = eq_points(&xs, &sum, self.graph_db);
            let sum_color = if eq1_bypassed { MUTED } else { GOLD };
            d.poly(&points, sum_color, 2.2);
            if let Some(b) = bands
                .iter()
                .find(|b| b.id == self.shared.solo_id.load(Ordering::Relaxed))
            {
                let fill = selected_curve.as_ref().map(|(pts, color, between)| {
                    let mut fill = *color;
                    fill.a = if eq1_bypassed { 0.02 } else { 0.07 };
                    (pts.as_slice(), db_y(0.0, self.graph_db), fill, *between)
                });
                draw_solo_shade(
                    &mut d, b.shape, b.freq, b.q, gx, gw, eq_b, &points, sum_color, 2.2, fill,
                );
            }

            for b in &bands {
                let color = if !b.enabled || eq1_bypassed {
                    MUTED
                } else {
                    BAND_COLORS[(b.id as usize - 1) % BAND_COLORS.len()]
                };
                let x = freq_x_at(b.freq, gx, gw);
                let y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db);
                if b.dynamic && b.shape.has_gain() {
                    if Some(b.id) == self.selected {
                        draw_dyn_range_curve(
                            &mut d,
                            b,
                            &xs,
                            (gx, gw, self.graph_db),
                            (sr, eq_sr),
                            color,
                        );
                    }
                    draw_dyn_range_stem(
                        &mut d,
                        b,
                        self.graph_db,
                        (gx, gw),
                        color,
                        uncapped_for(b.id, &dyn_uncapped),
                        Some(b.id) == self.selected,
                        self.input_level_for(b.id, &dyn_input),
                        self.shift_down,
                    );
                }
                d.eq_node(x, y, color, Some(b.id) == self.selected);
            }

            if let Some(b) = bands.iter().find(|b| Some(b.id) == self.selected) {
                let color = if !b.enabled || eq1_bypassed {
                    MUTED
                } else {
                    BAND_COLORS[(b.id as usize - 1) % BAND_COLORS.len()]
                };
                let is_solo = self.is_auditioning(b.id);
                let g = hud_geom_for_at(b, self.graph_db, gx, gw);
                let dyn_hover = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| hud_dyn_btn_hit(g, hx, hy, gx, gw));
                let cog_hover = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| hud_cog_hit(g, hx, hy, gx, gw));
                let bypass_r = hud_bypass_rect(g.bx, g.by);
                let bypass_hovered = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, bypass_r));
                if self.hud_visible() {
                    band_hud(
                        &mut d,
                        b,
                        is_solo,
                        color,
                        self.graph_db,
                        gx,
                        gw,
                        dyn_hover,
                        self.dyn_band_anim.step(),
                        cog_hover,
                        self.band_dyn_anim_progress.get(),
                        self.band_dyn_page.get(),
                        eq_b,
                        bypass_hovered,
                        self.band_bypass_anim.step(),
                        self.highlighted_dyn_field(b),
                    );
                }
            }

            // Hover cursor on EQ 1
            if page == 0 && (next_p - target_p).abs() < 0.05 {
                if let Some((x, y)) = self.idle_hover() {
                    if inside(x, y, (gx, GY, gw, GH))
                        && self.drag.is_none()
                        && self.menu.is_none()
                        && !self.scale_menu
                        && self.processing_menu.is_none()
                        && self.edit.is_none()
                        && !self.over_selected_hud(x, y)
                        && !eq1_bypassed
                    {
                        draw_band_graph_hover(
                            &mut d,
                            x,
                            y,
                            gx,
                            gw,
                            self.graph_db,
                            sr,
                            eq_sr,
                            self.selected,
                            &bands,
                            &dyn_uncapped,
                            self.shift_down,
                        );
                    }
                }
            }
        }

        self.render_lift_page(
            &mut d,
            LiftRenderFrame {
                offset_lift,
                lift_on,
                page,
                next_p,
                target_p,
                gx,
                gw,
                eq_b,
                spectrum: &spectrum,
                sr,
                lift_bands: &lift_bands,
                smooth_lift_gr: &smooth_lift_gr,
                smooth_lift_uncapped: &smooth_lift_uncapped,
            },
        );

        // --- DRAW EQ 2 / SC EQ (if visible) ---
        for (page_idx, offset, page_bands, page_on, allow_dyn) in [
            (EQ_PAGE_2, offset_2, eq2_bands.as_slice(), eq2_on, true),
            (
                EQ_PAGE_SC,
                offset_sc,
                sc_eq_bands.as_slice(),
                sc_eq_on,
                false,
            ),
        ] {
            if !(offset > -GRAPH_CLIP_W && offset < GRAPH_CLIP_W) {
                continue;
            }
            d.offset_x = offset;
            let page_bypassed = !page_on;
            draw_grid_and_spectrum(&mut d, page_bypassed, page_idx);

            let extras = page_bands.iter().filter(|b| b.enabled).map(|b| b.freq);
            let xs = eq_curve_xs(gx, gw, extras);
            let mut sum = vec![0.0; xs.len()];
            let mut selected_curve: Option<(Vec<(f32, f32)>, C, bool)> = None;
            for b in page_bands {
                if !b.enabled {
                    continue;
                }
                let coeff = BandCoeffs::make(&plot_band(b, &dyn_uncapped), eq_sr);
                let num = band_display_num(b.id);
                let color = if page_bypassed {
                    MUTED
                } else {
                    BAND_COLORS[(num as usize - 1) % BAND_COLORS.len()]
                };
                let dbs = eq_db_on_xs(&coeff, &xs, gx, gw, sr, eq_sr);
                for (i, db) in dbs.iter().enumerate() {
                    sum[i] += db;
                }
                if Some(b.id) == self.selected {
                    let points = eq_points(&xs, &dbs, self.graph_db);
                    let mut fill = color;
                    fill.a = if page_bypassed { 0.02 } else { 0.07 };
                    let between = allow_dyn && b.dynamic && b.shape.has_gain();
                    let fill_points = if between {
                        let range = range_curve_points(
                            b, &xs, (gx, gw, self.graph_db), (sr, eq_sr),
                        );
                        let polygon = between_curve_polygon(&points, &range);
                        d.fill_poly(&polygon, fill);
                        polygon
                    } else {
                        d.area(&points, db_y(0.0, self.graph_db), fill);
                        points.clone()
                    };
                    d.poly(&points, color, 1.2);
                    selected_curve = Some((fill_points, color, between));
                }
            }
            let points = eq_points(&xs, &sum, self.graph_db);
            let sum_color = if page_bypassed { MUTED } else { GOLD };
            d.poly(&points, sum_color, 2.2);
            if let Some(b) = page_bands
                .iter()
                .find(|b| b.id == self.shared.solo_id.load(Ordering::Relaxed))
            {
                let fill = selected_curve.as_ref().map(|(pts, color, between)| {
                    let mut fill = *color;
                    fill.a = if page_bypassed { 0.02 } else { 0.07 };
                    (pts.as_slice(), db_y(0.0, self.graph_db), fill, *between)
                });
                draw_solo_shade(
                    &mut d, b.shape, b.freq, b.q, gx, gw, eq_b, &points, sum_color, 2.2, fill,
                );
            }

            for b in page_bands {
                let num = band_display_num(b.id);
                let color = if !b.enabled || page_bypassed {
                    MUTED
                } else {
                    BAND_COLORS[(num as usize - 1) % BAND_COLORS.len()]
                };
                let x = freq_x_at(b.freq, gx, gw);
                let y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db);
                if allow_dyn && b.dynamic && b.shape.has_gain() {
                    if Some(b.id) == self.selected {
                        draw_dyn_range_curve(
                            &mut d,
                            b,
                            &xs,
                            (gx, gw, self.graph_db),
                            (sr, eq_sr),
                            color,
                        );
                    }
                    draw_dyn_range_stem(
                        &mut d,
                        b,
                        self.graph_db,
                        (gx, gw),
                        color,
                        uncapped_for(b.id, &dyn_uncapped),
                        Some(b.id) == self.selected,
                        self.input_level_for(b.id, &dyn_input),
                        self.shift_down,
                    );
                }
                d.eq_node(x, y, color, Some(b.id) == self.selected);
            }

            if let Some(b) = page_bands.iter().find(|b| Some(b.id) == self.selected) {
                let num = band_display_num(b.id);
                let color = if !b.enabled || page_bypassed {
                    MUTED
                } else {
                    BAND_COLORS[(num as usize - 1) % BAND_COLORS.len()]
                };
                let is_solo = self.is_auditioning(b.id);
                let g = hud_geom_for_at(b, self.graph_db, gx, gw);
                let dyn_hover = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| hud_dyn_btn_hit(g, hx, hy, gx, gw));
                let cog_hover = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| hud_cog_hit(g, hx, hy, gx, gw));
                let bypass_r = hud_bypass_rect(g.bx, g.by);
                let bypass_hovered = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, bypass_r));
                if self.hud_visible() {
                    band_hud(
                        &mut d,
                        b,
                        is_solo,
                        color,
                        self.graph_db,
                        gx,
                        gw,
                        dyn_hover,
                        self.dyn_band_anim.step(),
                        cog_hover,
                        self.band_dyn_anim_progress.get(),
                        self.band_dyn_page.get(),
                        eq_b,
                        bypass_hovered,
                        self.band_bypass_anim.step(),
                        self.highlighted_dyn_field(b),
                    );
                }
            }

            if page == page_idx && (next_p - target_p).abs() < 0.05 {
                if let Some((x, y)) = self.idle_hover() {
                    if inside(x, y, (gx, GY, gw, GH))
                        && self.drag.is_none()
                        && self.menu.is_none()
                        && !self.scale_menu
                        && self.processing_menu.is_none()
                        && self.edit.is_none()
                        && !self.over_selected_hud(x, y)
                        && !page_bypassed
                    {
                        draw_band_graph_hover(
                            &mut d,
                            x,
                            y,
                            gx,
                            gw,
                            self.graph_db,
                            sr,
                            eq_sr,
                            self.selected,
                            page_bands,
                            &dyn_uncapped,
                            self.shift_down,
                        );
                    }
                }
            }
        }

        let seam_frac = next_p - next_p.floor();
        if seam_frac > 0.005 && seam_frac < 0.995 {
            d.offset_x = 0.0;
            let seam_x = eq_b.0 + (next_p.floor() + 1.0 - next_p) * GRAPH_CLIP_W;
            d.line(seam_x, eq_b.1, seam_x, eq_b.1 + eq_b.3, LINE, 1.0);
        }

        d.reset_scissor();
        d.offset_x = 0.0;
        d.alpha_mul = 1.0;
        (sr, config, gx, gw)
    }
}
