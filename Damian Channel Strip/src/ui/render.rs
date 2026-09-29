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
            (cur_t + dt / pleasant_ui::page_slide::DURATION).min(1.0)
        } else {
            1.0
        };
        self.anim_time.set(next_t);
        let eased_t = pleasant_ui::page_slide::ease(next_t);
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

        let (sr, config, gx, gw) = self.render_eq_panel(&mut d, next_p, target_p);
        let (comp_bypassed, is_pre) = self.render_dynamics_panels(&mut d);
        d.line(32.0, FOOTER_LINE_Y, UI_W - 32.0, FOOTER_LINE_Y, LINE, 1.0);
        d.appearance_button(THEME_BUTTON, preferences::label());
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
                d.value_underline(val_r, r.1 + 13.0, GOLD);
            }
        }

        let bar_x = r.0 + 8.0;
        let bar_w = r.2 - 16.0;
        let bar_y = r.1 + 19.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA;
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
                    let text_x = if matches!(menu, BandMenu::Shape) {
                        d.filter_curve(
                            pleasant_eq::EqShape::from(Shape::ALL[row]).biquad_kind(),
                            (r.0 + 10.0, row_y + (row_h - 16.0) * 0.5, 30.0, 16.0),
                            if selected { GOLD } else { TEXT },
                        );
                        r.0 + 50.0
                    } else {
                        r.0 + 10.0
                    };
                    d.text(
                        text_x,
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
                d.value_underline(r, self.value_baseline(target), self.value_color(target));
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
