use super::*;

impl FlatteryView {
    pub(super) fn render(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        self.paint_editor(canvas, cx.bounds(), prefs().light(), prefs().label());
    }

    pub(super) fn paint_editor(
        &self,
        canvas: &mut Canvas,
        bounds: BoundingBox,
        light: bool,
        appearance: &str,
    ) {
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }
        load_font_once(canvas, &self.font, FONT_JETBRAINS_MONO);
        let label_font = load_font_once(
            canvas,
            &self.label_font,
            nih_plug_vizia::assets::fonts::NOTO_SANS_REGULAR,
        );

        let mut d = Draw::new(
            canvas,
            light,
            bounds.w / WINDOW_W,
            bounds.x,
            bounds.y,
            self.font.get(),
        )
        .with_hover(self.idle_hover());
        d.with_readable_style(label_font, |d| self.paint(d, appearance));
    }

    fn paint(&self, d: &mut Draw, appearance: &str) {
        d.rounded_rect(0.0, 0.0, WINDOW_W, WINDOW_H, 0.0, BG);
        d.rounded_rect(0.0, 0.0, WINDOW_W, HEADER_HEIGHT, 0.0, PANEL);
        d.text(36.0, 44.0, "FLATTERY", 24.0, GOLD);

        d.appearance_button(THEME_BUTTON, appearance);

        let module = (
            16.0,
            HEADER_HEIGHT + 16.0,
            WINDOW_W - 32.0,
            graph::CONTENT_H - HEADER_HEIGHT - 32.0,
        );
        d.panel(module, PANEL, LINE);

        let srate = self.shared.sample_rate.load(Ordering::Relaxed) as f64;
        let fft_size = self.params.fft_size.value().size();
        let bin_hz = srate / fft_size as f64;
        let bypassed =
            self.params.bypass.value() || self.shared.is_bypassed.load(Ordering::Relaxed);
        let mut layout = self.layout;
        layout.set_view(
            self.params.max_boost_db.value(),
            self.params.max_cut_db.value(),
            self.params.low_cut_hz.value() as f64,
            self.params.high_cut_hz.value() as f64,
            self.graph_zoomed,
        );

        layout.draw_background(d, fft_size, srate);
        layout.draw_limit_shade(
            d,
            self.params.max_boost_db.value(),
            self.params.max_cut_db.value(),
        );
        layout.draw_grid_and_labels(d, self.selected.is_none());

        let low_cut = self.params.low_cut_hz.value() as f64;
        let high_cut = self.params.high_cut_hz.value() as f64;
        let max_boost = self.params.max_boost_db.value();
        let max_cut = self.params.max_cut_db.value();
        let boost_pct = self.params.strength_boost.value();
        let cut_pct = self.params.strength_cut.value();

        layout.draw_operate_window(
            d,
            self.params.min_operate_db.value() as f64,
            self.params.max_operate_db.value() as f64,
        );

        let (zoom_in, zoom_out) = self.zoom_button_rects(&layout);
        let over =
            |rect: (f32, f32, f32, f32)| d.hover.is_some_and(|(x, y)| Self::inside(x, y, rect));
        let hover_in = zoom_in.is_some_and(over);
        let hover_out = zoom_out.is_some_and(over);
        let probe_x = self.probe_x(&layout, hover_in || hover_out);
        let reading = {
            let mut display = self.display.borrow_mut();
            display.refresh(&self.shared, fft_size, srate);
            layout.draw_spectrum(d, &display.spectrum, bin_hz, low_cut, high_cut);
            if !bypassed {
                layout.draw_filter_gains(d, &display.filters, fft_size, srate, low_cut, high_cut);
            }
            probe_x.and_then(|x| display.reading(&layout, x, fft_size, bin_hz, bypassed))
        };

        if let Some(reading) = &reading {
            reading.paint(d, &layout);
        }

        layout.draw_max_handles(
            d,
            max_boost,
            max_cut,
            low_cut,
            high_cut,
            self.hover_max_boost,
            self.hover_max_cut,
        );

        let boost_nodes = snapshot_nodes(&self.params, Polarity::Boost);
        let cut_nodes = snapshot_nodes(&self.params, Polarity::Cut);
        if let Some((polarity, id)) = self.selected {
            let (nodes, mut tint) = match polarity {
                Polarity::Boost => (&boost_nodes, COLOR_BOOST),
                Polarity::Cut => (&cut_nodes, COLOR_CUT),
            };
            if let Some(node) = nodes.iter().find(|node| node.id == id) {
                tint.a = 0.09;
                layout.draw_radius_bins(d, std::slice::from_ref(node), bin_hz, tint);
            }
        }
        let selected_boost =
            self.selected
                .and_then(|(p, id)| if p == Polarity::Boost { Some(id) } else { None });
        let selected_cut = self
            .selected
            .and_then(|(p, id)| if p == Polarity::Cut { Some(id) } else { None });

        d.scissor(layout.gx, layout.gy, layout.gw, layout.gh);
        layout.draw_strength_curve(
            d,
            &boost_nodes,
            Polarity::Boost,
            boost_pct,
            max_boost,
            max_cut,
            low_cut,
            high_cut,
            self.hover_curve == Some(Polarity::Boost)
                || self.hover_node.map(|h| h.0) == Some(Polarity::Boost),
            selected_boost,
        );
        layout.draw_strength_curve(
            d,
            &cut_nodes,
            Polarity::Cut,
            cut_pct,
            max_boost,
            max_cut,
            low_cut,
            high_cut,
            self.hover_curve == Some(Polarity::Cut)
                || self.hover_node.map(|h| h.0) == Some(Polarity::Cut),
            selected_cut,
        );
        if self.drag.is_none() && self.hover_node.is_none() {
            if let Some(polarity) = self.hover_curve {
                let pct = match polarity {
                    Polarity::Boost => boost_pct,
                    Polarity::Cut => cut_pct,
                };
                let freq = self.snap_hz(layout.x_to_freq(self.mouse.0));
                let nodes = match polarity {
                    Polarity::Boost => &boost_nodes,
                    Polarity::Cut => &cut_nodes,
                };
                let weight = weight_at(nodes, freq);
                let preview = StrengthNode::new(0, freq, weight);
                let stroke = match polarity {
                    Polarity::Boost => COLOR_BOOST_HOVER,
                    Polarity::Cut => COLOR_CUT_HOVER,
                };
                layout.draw_strength_node(
                    d, polarity, pct, &preview, stroke, false, low_cut, high_cut, 0.55,
                );
            }
        }
        d.reset_scissor();

        layout.draw_strength_handle(
            d,
            Polarity::Boost,
            boost_pct,
            self.hover_strength == Some(Polarity::Boost)
                || matches!(
                    self.drag,
                    Some(DragState::StrengthOffset {
                        polarity: Polarity::Boost,
                        ..
                    })
                ),
        );
        layout.draw_strength_handle(
            d,
            Polarity::Cut,
            cut_pct,
            self.hover_strength == Some(Polarity::Cut)
                || matches!(
                    self.drag,
                    Some(DragState::StrengthOffset {
                        polarity: Polarity::Cut,
                        ..
                    })
                ),
        );

        let hide_max_grab = self.hover_node.is_some() || self.hover_low_cut || self.hover_high_cut;
        let show_boost_grab = matches!(self.drag, Some(DragState::MaxBoost { .. }))
            || (self.drag.is_none() && !hide_max_grab && self.hover_max_boost);
        let show_cut_grab = matches!(self.drag, Some(DragState::MaxCut { .. }))
            || (self.drag.is_none() && !hide_max_grab && self.hover_max_cut);
        if show_boost_grab {
            layout.draw_line_grab(
                d,
                self.mouse.0,
                layout.db_to_y(max_boost as f64),
                COLOR_BOOST_HOVER,
            );
        }
        if show_cut_grab {
            layout.draw_line_grab(
                d,
                self.mouse.0,
                layout.db_to_y(-(max_cut as f64)),
                COLOR_CUT_HOVER,
            );
        }

        layout.draw_cut_handles(
            d,
            low_cut,
            high_cut,
            self.hover_low_cut,
            self.hover_high_cut,
        );

        let can_zoom_in = self.zoom_would_tighten(&layout);
        if let Some(r) = zoom_in {
            d.graph_zoom_button(r, true, hover_in && can_zoom_in, can_zoom_in);
        }
        if let Some(r) = zoom_out {
            d.graph_zoom_button(r, false, hover_out, true);
        }

        let fft_rect = self.fft_button_rect();
        d.button(fft_rect, &format!("FFT: {fft_size}"), false, GOLD);
        let domain_label = match self.params.ms_mode.value() {
            ProcessDomain::LR => "L/R",
            ProcessDomain::MS => "M/S",
        };
        d.button_tinted(self.domain_button_rect(), domain_label, true, TEAL);

        for &id in STACKED_SLIDERS {
            let r = Self::slider_rect(id);
            let knob = KnobLayout::new(r).with_text_sizes(READABLE_LABEL_SIZE, 13.0);
            let (label, val_str, color) = self.slider_info(id);
            let n = self.get_slider_norm(id);
            let val_r = Self::knob_value_rect(id);
            if let Some(edit) = &self.edit {
                if edit.target == id {
                    d.knob_with_layout(&knob, label, "", n, color, bypassed);
                    d.value_edit(edit, color);
                    continue;
                }
            }
            d.knob_with_layout(&knob, label, &val_str, n, color, bypassed);
            if self.edit.is_none() {
                if let Some((hx, hy)) = self.idle_hover() {
                    if Self::inside(hx, hy, val_r) {
                        d.value_underline(
                            val_r,
                            pleasant_ui::draw::KnobLayout::new(r).value_y,
                            color,
                        );
                    }
                }
            }
        }

        // Out Gain Knob
        let knob_r = Self::slider_rect(SliderId::OutputGain);
        let (label, val_str, color) = self.slider_info(SliderId::OutputGain);
        let n = self.get_slider_norm(SliderId::OutputGain);
        let val_r = Self::output_knob_value_rect();
        let knob = KnobLayout::new(knob_r).with_text_sizes(READABLE_LABEL_SIZE, 13.0);

        if let Some(edit) = &self.edit {
            if edit.target == SliderId::OutputGain {
                d.knob_with_layout_bipolar(&knob, label, "", n, color, bypassed);
                d.value_edit(edit, color);
            } else {
                d.knob_with_layout_bipolar(&knob, label, &val_str, n, color, bypassed);
            }
        } else {
            d.knob_with_layout_bipolar(&knob, label, &val_str, n, color, bypassed);
            if self.edit.is_none() {
                if let Some((hx, hy)) = self.idle_hover() {
                    if Self::inside(hx, hy, val_r) {
                        d.value_underline(
                            val_r,
                            pleasant_ui::draw::KnobLayout::new(knob_r).value_y,
                            color,
                        );
                    }
                }
            }
        }

        if self.selected.is_some() {
            for &id in NODE_SLIDERS {
                let r = Self::slider_rect(id);
                let (label, val_str, color) = self.slider_info(id);
                let n = self.get_slider_norm(id);
                let val_r = slider_value_rect(r);
                let baseline = r.1 + 16.0;
                d.with_label_font(|d| {
                    d.text(r.0 + 8.0, baseline, label, READABLE_LABEL_SIZE, MUTED)
                });
                let editing = self.edit.as_ref().is_some_and(|edit| edit.target == id);
                if editing {
                    if let Some(edit) = &self.edit {
                        d.value_edit(edit, color);
                    }
                } else {
                    d.text_centered(val_r.0 + val_r.2 * 0.5, baseline, &val_str, 13.0, TEXT);
                    if let Some((hx, hy)) = self.idle_hover() {
                        if Self::inside(hx, hy, val_r) {
                            d.value_underline(val_r, baseline, color);
                        }
                    }
                }
                let bar_x = r.0 + 8.0;
                let bar_w = r.2 - 16.0;
                let bar_y = r.1 + 24.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA;
                d.rect(bar_x, bar_y, bar_w, 5.0, LINE);
                d.rect(bar_x, bar_y, bar_w * n.clamp(0.0, 1.0), 5.0, color);
            }
        }

        let footer_y = WINDOW_H - pleasant_ui::draw::TOOLTIP_BASELINE_INSET;
        if let Some(reading) = reading {
            reading.paint_footer(d, GRAPH_X, footer_y);
        } else {
            let hint = if hover_in && can_zoom_in {
                "Zoom to the processing range."
            } else if hover_out {
                "Reset graph zoom."
            } else if self.edit.is_some() {
                "Enter applies the value. Escape cancels."
            } else if let Some(hint) = self.control_hint() {
                hint
            } else if probe_x.is_some() {
                "No analysis at this frequency."
            } else if bypassed {
                "Bypassed · Input analysis is still visible."
            } else {
                "Hover the graph to inspect input level and applied filter gain."
            };
            d.bottom_tooltip(
                (GRAPH_X, graph::CONTENT_H, WINDOW_W - GRAPH_X - 30.0, pleasant_ui::draw::TOOLTIP_HEIGHT),
                hint,
                MUTED,
            );
        }
    }

    fn probe_x(&self, layout: &GraphLayout, over_zoom: bool) -> Option<f32> {
        let (x, y) = self.idle_hover()?;
        if over_zoom
            || self.edit.is_some()
            || self.value_press.is_some()
            || self.hover_node.is_some()
            || self.hover_curve.is_some()
            || self.hover_strength.is_some()
            || self.hover_low_cut
            || self.hover_high_cut
            || self.hover_max_boost
            || self.hover_max_cut
            || !layout.in_graph(x, y)
            || layout.hit_op_min(x, y, self.params.min_operate_db.value())
            || layout.hit_op_max(x, y, self.params.max_operate_db.value())
        {
            return None;
        }
        Some(x)
    }
}
