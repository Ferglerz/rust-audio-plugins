use super::*;

impl StripView {
    pub(super) fn render(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let Some(scale) = view_scale(bounds) else {
            return;
        };
        if self.font.get().is_none() {
            self.font.set(canvas.add_font_mem(FONT_JETBRAINS_MONO).ok());
        }
        if self.signature.get().is_none() {
            self.signature.set(
                canvas
                    .add_font_mem(include_bytes!("assets/Allura-Regular.ttf"))
                    .ok(),
            );
        }
        let mut d = Draw::new(
            canvas,
            preferences::light(),
            scale,
            bounds.x,
            bounds.y,
            self.font.get(),
        );
        d.rect(0.0, 0.0, UI_W, UI_H, BG);
        d.rect(0.0, 0.0, UI_W, HEADER_H, PANEL);
        d.text(32.0, 46.0, "dB", 28.0, GOLD);
        d.text(88.0, 46.0, "SIGNATURE CHANNEL STRIP", 16.0, TEXT);
        draw_signature(&mut d, self.signature.get());

        let now = Instant::now();
        let dt = self
            .last_tick
            .get()
            .map(|prev| (now - prev).as_secs_f32())
            .unwrap_or(0.016)
            .clamp(0.001, 0.1);
        self.last_tick.set(Some(now));

        let cur_t = self.anim_time.get();
        let next_t = if cur_t < 1.0 {
            (cur_t + dt / 0.25).min(1.0)
        } else {
            1.0
        };
        self.anim_time.set(next_t);
        let eased_t = next_t * next_t * next_t * (next_t * (next_t * 6.0 - 15.0) + 10.0);
        let start_p = self.anim_start.get();
        let target_p = self.anim_target.get();
        let next_p = start_p + (target_p - start_p) * eased_t;
        self.anim_progress.set(next_p);
        tick_page_anim(&self.dyn_anim_progress, self.dyn_anim_target.get(), dt);
        tick_page_anim(&self.pse_anim_progress, self.pse_anim_target.get(), dt);
        tick_page_anim(&self.wall_anim_progress, self.wall_anim_target.get(), dt);
        tick_page_anim(
            &self.band_dyn_anim_progress,
            self.band_dyn_anim_target.get(),
            dt,
        );

        let dragging = self.drag.is_some();
        let hover = self.idle_hover();
        if module_settings_should_return(
            dragging || hover.is_some_and(|(x, y)| inside(x, y, self.dyn_bounds())),
            self.dyn_page.get() == DynPage::Controls,
            &self.dyn_away_since,
            now,
        ) {
            self.dyn_page.set(DynPage::Main);
            self.dyn_anim_target.set(0.0);
        }
        if module_settings_should_return(
            dragging || hover.is_some_and(|(x, y)| inside(x, y, self.pse_bounds())),
            self.pse_page.get() == PsePage::Controls,
            &self.pse_away_since,
            now,
        ) {
            self.pse_page.set(PsePage::Main);
            self.pse_anim_target.set(0.0);
        }
        if module_settings_should_return(
            dragging || hover.is_some_and(|(x, y)| inside(x, y, self.wall_bounds())),
            self.wall_page.get() == WallPage::Controls,
            &self.wall_away_since,
            now,
        ) {
            self.wall_page.set(WallPage::Main);
            self.wall_anim_target.set(0.0);
        }

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
            eq_x + 44.0,
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
            for db in (-(self.graph_db as i32)..=self.graph_db as i32).step_by(step) {
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
            let mut selected_curve: Option<(Vec<(f32, f32)>, C)> = None;
            for b in &bands {
                if !b.enabled {
                    continue;
                }
                let coeff = BandCoeffs::make(&plot_band(b, &dyn_uncapped), eq_sr);
                let color = if eq1_bypassed {
                    MUTED
                } else {
                    COLORS[(b.id as usize - 1) % COLORS.len()]
                };
                let dbs = eq_db_on_xs(&coeff, &xs, gx, gw, sr, eq_sr);
                for (i, db) in dbs.iter().enumerate() {
                    sum[i] += db;
                }
                if Some(b.id) == self.selected {
                    let points = eq_points(&xs, &dbs, self.graph_db);
                    let mut fill = color;
                    fill.a = if eq1_bypassed { 0.02 } else { 0.07 };
                    d.area(&points, db_y(0.0, self.graph_db), fill);
                    d.poly(&points, color, 1.2);
                    selected_curve = Some((points, color));
                }
            }
            let points = eq_points(&xs, &sum, self.graph_db);
            let sum_color = if eq1_bypassed { MUTED } else { GOLD };
            d.poly(&points, sum_color, 2.2);
            if let Some(b) = bands
                .iter()
                .find(|b| b.id == self.shared.solo_id.load(Ordering::Relaxed))
            {
                let fill = selected_curve.as_ref().map(|(pts, color)| {
                    let mut fill = *color;
                    fill.a = if eq1_bypassed { 0.02 } else { 0.07 };
                    (pts.as_slice(), db_y(0.0, self.graph_db), fill)
                });
                draw_solo_shade(
                    &mut d, b.shape, b.freq, b.q, gx, gw, eq_b, &points, sum_color, 2.2, fill,
                );
            }

            for b in &bands {
                let color = if !b.enabled || eq1_bypassed {
                    MUTED
                } else {
                    COLORS[(b.id as usize - 1) % COLORS.len()]
                };
                let x = freq_x_at(b.freq, gx, gw);
                let y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db);
                if b.dynamic && b.shape.has_gain() {
                    draw_dyn_range_stem(
                        &mut d,
                        b,
                        self.graph_db,
                        (gx, gw),
                        color,
                        uncapped_for(b.id, &dyn_uncapped),
                        Some(b.id) == self.selected,
                        self.input_level_for(b.id, &dyn_input),
                    );
                } else if Some(b.id) == self.selected {
                    d.circle(x, y, 12.0, color, false);
                }
                d.circle(x, y, 7.0, color, true);
            }

            if let Some(b) = bands.iter().find(|b| Some(b.id) == self.selected) {
                let color = if !b.enabled || eq1_bypassed {
                    MUTED
                } else {
                    COLORS[(b.id as usize - 1) % COLORS.len()]
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
                        );
                    }
                }
            }
        }

        // --- DRAW LIFT (if visible) ---
        if offset_lift > -GRAPH_CLIP_W && offset_lift < GRAPH_CLIP_W {
            d.offset_x = offset_lift;
            let lift_bypassed = !lift_on;
            for db in [0, -20, -40, -60, -80, -100] {
                let y = lift_gain_y(db as f64);
                d.line(
                    gx,
                    y,
                    gx + gw,
                    y,
                    if db == 0 { C::rgb(75, 81, 85) } else { LINE },
                    1.0,
                );
                d.text_centered(
                    gx - 22.0,
                    axis_label_y(y, 13.0),
                    &axis_db_text(db),
                    13.0,
                    MUTED,
                );
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
                if lift_bypassed {
                    C::rgba(80, 90, 100, 10)
                } else {
                    C::rgba(125, 143, 159, 28)
                },
            );
            d.poly_above(
                &spectrum,
                GY + GH,
                if lift_bypassed {
                    C::rgba(90, 100, 110, 20)
                } else {
                    C::rgba(144, 161, 175, 62)
                },
                1.0,
            );

            for b in &lift_bands {
                if !b.enabled {
                    continue;
                }
                let color = if lift_bypassed { MUTED } else { LIFT_COLOR };
                let points = lift_curve_points(b, &smooth_lift_gr, gx, gw, sr);
                draw_lift_influence(
                    &mut d,
                    b,
                    &points,
                    color,
                    (
                        if lift_bypassed { 0.05 } else { 0.22 },
                        if lift_bypassed { 0.25 } else { 0.90 },
                    ),
                    (gx, gw),
                );
                let overflow = !lift_bypassed
                    && smooth_lift_uncapped
                        .iter()
                        .zip(smooth_lift_gr.iter())
                        .any(|(uncapped, capped)| *uncapped > *capped + 0.15);
                if overflow {
                    let over_pts = lift_curve_points(b, &smooth_lift_uncapped, gx, gw, sr);
                    draw_lift_influence(&mut d, b, &over_pts, MUTED, (0.0, 0.55), (gx, gw));
                }
            }
            if let Some(b) = lift_bands
                .iter()
                .find(|b| b.id == self.shared.solo_id.load(Ordering::Relaxed))
            {
                let lift_pts = lift_curve_points(b, &smooth_lift_gr, gx, gw, sr);
                let lift_color = if lift_bypassed { MUTED } else { LIFT_COLOR };
                let mut fill = lift_color;
                fill.a = if lift_bypassed { 0.05 } else { 0.22 };
                draw_solo_shade(
                    &mut d,
                    b.shape,
                    b.freq,
                    b.q,
                    gx,
                    gw,
                    eq_b,
                    &lift_pts,
                    lift_color,
                    1.8,
                    Some((lift_pts.as_slice(), GY + GH, fill)),
                );
            }

            for (idx, b) in lift_bands.iter().enumerate() {
                let color = if !b.enabled || lift_bypassed {
                    MUTED
                } else {
                    LIFT_COLOR
                };
                let x = freq_x_at(b.freq, gx, gw);
                let y = lift_gain_y(b.gain);
                if Some(b.id) == self.selected {
                    d.circle(x, y, 13.0, color, false);
                }
                d.circle(x, y, 7.0, color, true);
                let label = if lift_bands.len() > 1 {
                    format!("{} LIFT {}", b.shape.uppercase_name(), idx + 1)
                } else {
                    format!("{} LIFT", b.shape.uppercase_name())
                };
                d.text(x - 12.0, y - 17.0, &label, 10.0, color);
            }

            if let Some(b) = lift_bands.iter().find(|b| Some(b.id) == self.selected) {
                let color = if !b.enabled || lift_bypassed {
                    MUTED
                } else {
                    LIFT_COLOR
                };
                let is_solo = self.is_auditioning(b.id);
                if self.hud_visible() {
                    let (bx, by, _, _) = self.hud_rect_for_lift(b);
                    let bypass_r = hud_bypass_rect(bx, by);
                    let bypass_hovered = self
                        .idle_hover()
                        .is_some_and(|(hx, hy)| inside(hx, hy, bypass_r));
                    band_hud_lift(
                        &mut d,
                        b,
                        is_solo,
                        color,
                        self.graph_db,
                        gx,
                        gw,
                        bypass_hovered,
                        self.band_bypass_anim.step(),
                    );
                }
                let c = if lift_on { LIFT_COLOR } else { MUTED };
                let badge_rect = (gx + 8.0, GRAPH_BOTTOM - 48.0, 32.0, 28.0);
                let badge_hovered = self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, badge_rect));
                let badge_click = self.lift_badge_anim.step();
                d.bypass_button(
                    badge_rect,
                    !b.enabled,
                    LIFT_COLOR,
                    badge_hovered,
                    badge_click,
                );
                for (i, label, val, n) in [
                    (
                        0,
                        "THRESHOLD",
                        format!("{:.1}", b.threshold),
                        (b.threshold + 60.0) / 60.0,
                    ),
                    (
                        1,
                        "RATIO",
                        format!("{:.1}:1", b.ratio),
                        (b.ratio - 1.0) / 19.0,
                    ),
                    (
                        2,
                        "ATTACK",
                        format!("{:.1} ms", b.attack),
                        (b.attack / 0.1).log(2000.0),
                    ),
                    (
                        3,
                        "RELEASE",
                        format!("{:.0} ms", b.release),
                        (b.release / 10.0).log(200.0),
                    ),
                    (4, "RANGE", format!("{:.1}", b.range), b.range / 24.0),
                ] {
                    d.control(band_rect_at(i, gx, gw), label, &val, n as f32, c);
                }
            }

            if page == EQ_PAGE_LIFT && (next_p - target_p).abs() < 0.05 {
                if let Some((x, y)) = self.idle_hover() {
                    if inside(x, y, (gx, GY, gw, GH))
                        && self.drag.is_none()
                        && self.menu.is_none()
                        && self.edit.is_none()
                        && !lift_bypassed
                        && !self.over_selected_hud(x, y)
                    {
                        let near = lift_bands.iter().any(|b| {
                            (x - freq_x_at(b.freq, gx, gw)).abs() < 15.0
                                && (y - lift_gain_y(b.gain)).abs() < 15.0
                        });
                        if !near {
                            let shape = infer_shape((x - gx) / gw, (y - GY) / GH, false);
                            if self.selected.is_none() {
                                let preview_band = LiftBand {
                                    id: 0,
                                    shape,
                                    freq: x_freq_at(x, gx, gw).clamp(20.0, 20000.0),
                                    gain: lift_y_gain(y),
                                    ..LiftBand::default()
                                };
                                let points =
                                    lift_curve_points(&preview_band, &smooth_lift_gr, gx, gw, sr);
                                draw_lift_influence(
                                    &mut d,
                                    &preview_band,
                                    &points,
                                    MUTED,
                                    (0.08, 0.40),
                                    (gx, gw),
                                );
                            }
                            draw_hover_preview(
                                &mut d,
                                x,
                                y,
                                gx,
                                gw,
                                hover_preview_label(self.selected.is_some(), shape),
                            );
                        }
                    }
                }
            }
        }

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
            let mut selected_curve: Option<(Vec<(f32, f32)>, C)> = None;
            for b in page_bands {
                if !b.enabled {
                    continue;
                }
                let coeff = BandCoeffs::make(&plot_band(b, &dyn_uncapped), eq_sr);
                let num = band_display_num(b.id);
                let color = if page_bypassed {
                    MUTED
                } else {
                    COLORS[(num as usize - 1) % COLORS.len()]
                };
                let dbs = eq_db_on_xs(&coeff, &xs, gx, gw, sr, eq_sr);
                for (i, db) in dbs.iter().enumerate() {
                    sum[i] += db;
                }
                if Some(b.id) == self.selected {
                    let points = eq_points(&xs, &dbs, self.graph_db);
                    let mut fill = color;
                    fill.a = if page_bypassed { 0.02 } else { 0.07 };
                    d.area(&points, db_y(0.0, self.graph_db), fill);
                    d.poly(&points, color, 1.2);
                    selected_curve = Some((points, color));
                }
            }
            let points = eq_points(&xs, &sum, self.graph_db);
            let sum_color = if page_bypassed { MUTED } else { GOLD };
            d.poly(&points, sum_color, 2.2);
            if let Some(b) = page_bands
                .iter()
                .find(|b| b.id == self.shared.solo_id.load(Ordering::Relaxed))
            {
                let fill = selected_curve.as_ref().map(|(pts, color)| {
                    let mut fill = *color;
                    fill.a = if page_bypassed { 0.02 } else { 0.07 };
                    (pts.as_slice(), db_y(0.0, self.graph_db), fill)
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
                    COLORS[(num as usize - 1) % COLORS.len()]
                };
                let x = freq_x_at(b.freq, gx, gw);
                let y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, self.graph_db);
                if allow_dyn && b.dynamic && b.shape.has_gain() {
                    draw_dyn_range_stem(
                        &mut d,
                        b,
                        self.graph_db,
                        (gx, gw),
                        color,
                        uncapped_for(b.id, &dyn_uncapped),
                        Some(b.id) == self.selected,
                        self.input_level_for(b.id, &dyn_input),
                    );
                } else if Some(b.id) == self.selected {
                    d.circle(x, y, 12.0, color, false);
                }
                d.circle(x, y, 7.0, color, true);
            }

            if let Some(b) = page_bands.iter().find(|b| Some(b.id) == self.selected) {
                let num = band_display_num(b.id);
                let color = if !b.enabled || page_bypassed {
                    MUTED
                } else {
                    COLORS[(num as usize - 1) % COLORS.len()]
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
            px + 44.0,
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
                if pse_dragging_knee {
                    let knee_x = (rect_center_x(self.pse_main_thresh_slider_rect())
                        + rect_center_x(self.pse_main_gr_meter_rect()))
                        * 0.5;
                    d.text_centered(
                        knee_x,
                        meter_value_y(),
                        &format_knee_readout(self.params.pse_knee.value()),
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
                    let pse_dragging_depth = self.drag == Some(Target::Global(7));
                    let pse_gr_text = format_gr_meter_readout(if pse_dragging_depth {
                        self.params.pse_depth.value()
                    } else {
                        self.shared.pse_gr_peak.load(Ordering::Relaxed)
                    });
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
                for (i, label) in [(8, "HYSTERESIS"), (2, "VOICE DET"), (10, "TIME")] {
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
                let det_hover = !pse_bypassed
                    && self
                        .idle_hover()
                        .is_some_and(|(hx, hy)| inside(hx, hy, det_r));
                d.button(
                    det_r,
                    if is_peak { "PEAK" } else { "RMS" },
                    true,
                    if pse_bypassed {
                        MUTED
                    } else if det_hover {
                        TEXT
                    } else {
                        PSE_BLUE
                    },
                );

                let speech_r = (px + 68.0, 356.0, 56.0, 86.0);
                d.text_centered(
                    speech_r.0 + speech_r.2 * 0.5,
                    speech_r.1 + 22.0,
                    "SPEECH",
                    9.2,
                    if pse_bypassed { MUTED } else { TEXT },
                );
                let speech_env = self
                    .shared
                    .speech_env
                    .load(Ordering::Relaxed)
                    .clamp(0.0, 1.0);
                let bar_x = speech_r.0 + 4.0;
                let bar_y = speech_r.1 + 40.0;
                let bar_w = speech_r.2 - 8.0;
                let bar_h = 10.0;
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
                    speech_r.0 + speech_r.2 * 0.5,
                    speech_r.1 + 68.0,
                    status_text,
                    10.0,
                    if pse_bypassed || status_text == "OFF" {
                        MUTED
                    } else if status_text == "VOICE" {
                        TEAL
                    } else {
                        PSE_BLUE
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
            dx + 44.0,
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
                if comp_dragging_knee {
                    let knee_x = (rect_center_x(self.dyn_main_thresh_slider_rect())
                        + rect_center_x(self.dyn_main_gr_meter_rect()))
                        * 0.5;
                    d.text_centered(
                        knee_x,
                        meter_value_y(),
                        &format_knee_readout(self.params.comp_knee.value()),
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
                    let comp_dragging_depth = self.drag == Some(Target::Global(14));
                    let comp_gr_text = format_gr_meter_readout(if comp_dragging_depth {
                        self.params.comp_depth.value()
                    } else {
                        self.shared.gr_peak.load(Ordering::Relaxed)
                    });
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
            wx + 44.0,
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

        d.line(32.0, FOOTER_LINE_Y, UI_W - 32.0, FOOTER_LINE_Y, LINE, 1.0);
        d.button(
            THEME_BUTTON,
            if d.light { "LIGHT" } else { "DARK" },
            false,
            TEXT,
        );
        d.button(
            PROCESS_BUTTON,
            &format!("{} ▾", config.mode.label()),
            false,
            GOLD,
        );
        if config.mode == ProcessingMode::LinearPhase {
            d.button(
                resolution_button_rect(),
                &format!("{} ▾", config.resolution.label()),
                false,
                GOLD,
            );
        }
        let active = Config::decode(self.shared.active_config.load(Ordering::Relaxed));
        let latency = self.shared.latency.load(Ordering::Relaxed);
        let status = if active != config {
            "APPLYING...".to_string()
        } else if config.mode == ProcessingMode::LinearPhase {
            format!("{:.1} ms / STATIC EQ", latency as f64 * 1000.0 / sr)
        } else if latency > 0 {
            format!("{:.2} ms", latency as f64 * 1000.0 / sr)
        } else {
            String::new()
        };
        if !status.is_empty() {
            let status_x = if config.mode == ProcessingMode::LinearPhase {
                400.0
            } else {
                288.0
            };
            d.text(status_x, FOOTER_BTN_Y + 18.0, &status, 10.0, MUTED);
        }
        d.text(
            self.footer_comp_label_x(),
            FOOTER_BTN_Y + 19.0,
            "Compression:",
            12.0,
            if comp_bypassed { MUTED } else { TEXT },
        );
        d.button(
            self.footer_comp_routing_rect(),
            if is_pre { "PRE" } else { "POST" },
            true,
            if is_pre { TEAL } else { GOLD },
        );
        d.button(
            self.footer_auto_rect(),
            "AUTO",
            self.params.auto_makeup.value(),
            if comp_bypassed { MUTED } else { GOLD },
        );
        d.button(
            self.footer_link_rect(),
            "LINK",
            self.params.stereo_link.value(),
            if comp_bypassed { MUTED } else { GOLD },
        );

        let r = output_gain_rect();
        d.rect(r.0, r.1, r.2, r.3, PANEL);
        let out_hover = self.idle_hover().is_some_and(|(hx, hy)| inside(hx, hy, r));
        let out_dragging = self.drag == Some(Target::Global(15));
        d.outline(
            r,
            if out_hover || out_dragging {
                GOLD
            } else {
                LINE
            },
        );

        let p_out = self.param(15);
        let out_norm = p_out.unmodulated_normalized_value();
        let out_val = p_out.value();
        let val_str = if out_val.abs() < 0.05 {
            "0.0 dB".to_string()
        } else {
            format!("{:+0.1} dB", out_val)
        };

        d.text(r.0 + 8.0, r.1 + 13.0, "OUTPUT", 9.5, MUTED);
        let val_r = output_gain_value_rect();
        if let Some(edit) = &self.edit {
            if edit.target == ValueTarget::Global(15) {
                d.value_edit(edit, GOLD);
            } else {
                d.text_centered(val_r.0 + val_r.2 * 0.5, r.1 + 13.0, &val_str, 10.5, TEXT);
            }
        } else {
            d.text_centered(val_r.0 + val_r.2 * 0.5, r.1 + 13.0, &val_str, 10.5, TEXT);
            if out_hover
                && self
                    .idle_hover()
                    .is_some_and(|(hx, hy)| inside(hx, hy, val_r))
            {
                d.value_underline(val_r, GOLD);
            }
        }

        let bar_x = r.0 + 8.0;
        let bar_w = r.2 - 16.0;
        let bar_y = r.1 + 19.0;
        let bar_h = 4.0;
        d.rect(bar_x, bar_y, bar_w, bar_h, LINE);
        let mid_x = bar_x + bar_w * 0.5;
        if out_norm > 0.5 {
            let fill_w = bar_w * (out_norm - 0.5);
            d.rect(mid_x, bar_y, fill_w, bar_h, GOLD);
        } else if out_norm < 0.5 {
            let fill_w = bar_w * (0.5 - out_norm);
            d.rect(mid_x - fill_w, bar_y, fill_w, bar_h, GOLD);
        }
        d.line(mid_x, bar_y - 1.0, mid_x, bar_y + bar_h + 1.0, TEXT, 1.0);
        if let Some(menu) = self.menu {
            let menu_info = self
                .selected
                .and_then(|id| self.find_band(id))
                .map(|b| {
                    (
                        menu.rect_at(&b, self.graph_db, gx, gw),
                        b.shape,
                        b.order as usize,
                    )
                })
                .or_else(|| {
                    self.selected_lift().map(|b| {
                        (
                            menu.rect_lift_at(&b, self.graph_db, gx, gw),
                            b.shape,
                            b.order as usize,
                        )
                    })
                });
            if let Some((r, shape, order)) = menu_info {
                let row_h = menu.row_h();
                let text_size = menu.text_size();
                d.rect(r.0, r.1, r.2, r.3, PANEL);
                for row in 0..menu.count() {
                    let (label, selected) = match menu {
                        BandMenu::Shape => {
                            (Shape::ALL[row].name().to_string(), shape == Shape::ALL[row])
                        }
                        BandMenu::Order => (
                            format!("{} dB/oct  /  order {}", (row + 1) * 6, row + 1),
                            order == row + 1,
                        ),
                    };
                    let row_y = r.1 + row as f32 * row_h;
                    if selected
                        || self
                            .idle_hover()
                            .is_some_and(|(x, y)| inside(x, y, (r.0, row_y, r.2, row_h)))
                    {
                        d.rect(r.0, row_y, r.2, row_h, LINE);
                    }
                    d.text(
                        r.0 + 10.0,
                        row_y + row_h * 0.5 + text_size * 0.32,
                        &label,
                        text_size,
                        if selected { GOLD } else { TEXT },
                    );
                }
            }
        }
        if let Some(resolution) = self.processing_menu {
            let r = processing_menu_rect(resolution);
            let row_h = dropdown_row_h(PROCESS_BUTTON_TEXT);
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            d.outline(r, LINE);
            let count = if resolution {
                RESOLUTIONS.len()
            } else {
                MODES.len()
            };
            for i in 0..count {
                let (label, selected) = if resolution {
                    (RESOLUTIONS[i].label(), config.resolution == RESOLUTIONS[i])
                } else {
                    (MODES[i].label(), config.mode == MODES[i])
                };
                let row = (r.0, r.1 + i as f32 * row_h, r.2, row_h);
                if selected || self.idle_hover().is_some_and(|(x, y)| inside(x, y, row)) {
                    d.rect(row.0, row.1, row.2, row.3, LINE);
                }
                d.text(
                    row.0 + 10.0,
                    row.1 + row_h * 0.5 + PROCESS_BUTTON_TEXT * 0.32,
                    label,
                    PROCESS_BUTTON_TEXT,
                    if selected { GOLD } else { TEXT },
                );
            }
        }
        if self.edit.is_none()
            && self.menu.is_none()
            && !self.scale_menu
            && self.processing_menu.is_none()
        {
            if let Some((target, r)) = self.idle_hover().and_then(|(x, y)| self.value_at(x, y)) {
                d.line(
                    r.0 + 4.0,
                    r.1 + r.3 - 1.0,
                    r.0 + r.2 - 4.0,
                    r.1 + r.3 - 1.0,
                    self.value_color(target),
                    1.0,
                );
            }
        }
        if let Some(edit) = &self.edit {
            let accent = self.value_color(edit.target);
            let r = edit.rect;
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            d.outline(
                r,
                if edit.invalid {
                    rgb(238, 110, 95)
                } else {
                    accent
                },
            );
            // Keep a long entry's caret in the field while editing.
            let capacity = ((r.2 - 8.0) / 6.6) as usize;
            let start = edit.cursor.saturating_sub(capacity);
            let end = (start + capacity).min(edit.text.len());
            let selection = edit.selection();
            let left = selection.start.max(start).min(end);
            let right = selection.end.min(end).max(left);
            d.rect(
                r.0 + 4.0 + (left - start) as f32 * 6.6,
                r.1 + 2.0,
                (right - left) as f32 * 6.6,
                r.3 - 4.0,
                C { a: 0.25, ..accent },
            );
            d.text(
                r.0 + 4.0,
                r.1 + r.3 * 0.5 + 4.0,
                &edit.text[start..end],
                11.0,
                TEXT,
            );
            let caret_x = r.0 + 4.0 + (edit.cursor - start) as f32 * 6.6;
            d.line(caret_x, r.1 + 3.0, caret_x, r.1 + r.3 - 3.0, accent, 1.0);
        }
    }
}

fn draw_signature(d: &mut Draw, font: Option<FontId>) {
    let shift = UI_W - 1120.0;
    let mut paint = Paint::linear_gradient(
        d.ox + (800.0 + shift) * d.s,
        d.oy + 13.0 * d.s,
        d.ox + (810.0 + shift) * d.s,
        d.oy + 63.0 * d.s,
        d.color(C::rgb(255, 230, 163)),
        d.color(C::rgb(176, 119, 33)),
    );
    if let Some(f) = font {
        paint.set_font(&[f]);
    }
    paint.set_font_size(44.0 * d.s);
    let _ = d.c.fill_text(
        d.ox + (794.0 + shift) * d.s,
        d.oy + 51.0 * d.s,
        "Damian Birdsey",
        &paint,
    );
    let mut underline = Path::new();
    for (i, (x, y)) in [
        (821.0 + shift, 67.0),
        (870.0 + shift, 64.0),
        (972.0 + shift, 65.0),
        (1073.0 + shift, 59.0),
    ]
    .into_iter()
    .enumerate()
    {
        if i == 0 {
            underline.move_to(d.ox + x * d.s, d.oy + y * d.s);
        } else {
            underline.line_to(d.ox + x * d.s, d.oy + y * d.s);
        }
    }
    let gold = d.color(GOLD);
    let mut paint = Paint::linear_gradient(
        d.ox + (821.0 + shift) * d.s,
        d.oy,
        d.ox + (1073.0 + shift) * d.s,
        d.oy,
        C { a: 0.0, ..gold },
        gold,
    );
    paint.set_line_width(d.s);
    d.c.stroke_path(&underline, &paint);
}
