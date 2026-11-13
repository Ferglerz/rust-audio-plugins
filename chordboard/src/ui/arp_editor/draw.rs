use super::*;

impl ChordboardView {
    pub(in crate::ui) fn arp_button(
        &self,
        d: &mut Draw,
        r: Rect,
        label: &str,
        on: bool,
        color: Color,
    ) {
        if matches!(label, "+" | "−" | "-" | "×") {
            self.button_surface(d, r, on, color);
            let size = 20.0;
            d.text_centered(
                r.0 + r.2 * 0.5,
                r.1 + r.3 * 0.5 + size * 0.35,
                label,
                size,
                if on { color } else { TEXT },
            );
            return;
        }
        self.button(d, r, label, on, color);
    }

    pub(super) fn arp_icon_button(
        &self,
        d: &mut Draw,
        r: Rect,
        label: &str,
        on: bool,
        color: Color,
        icon: style::Icon,
    ) {
        self.icon_button(d, r, label, on, color, icon);
    }

    pub(in crate::ui) fn draw_adjustment_knob(&self, d: &mut Draw, control: &Control, rect: Rect) {
        let routed = self.routed_control(control);
        let c = routed.as_ref().unwrap_or(control);
        let color = if routed.is_some() { TEAL } else { GOLD };
        let label = if c.id == "swing" { "Swing" } else { "Humanize" };
        let layout = adjustment_knob_layout(rect);
        let value = self.display_value(c);
        if c.id == "swing" {
            d.knob_with_layout_bipolar(&layout, label, &value, c.norm, color, false);
        } else {
            d.knob_with_layout(&layout, label, &value, c.norm, color, false);
        }
        if routed.is_some() {
            self.live_highlight(d, rect);
        }
    }

    pub(super) fn arp_value(&self, d: &mut Draw, r: Rect, label: &str, value: f32) {
        self.arp_button(d, r, label, false, GOLD);
        let rail = (r.0 + 8.0, r.1 + r.3 - 4.0, r.2 - 16.0);
        d.line(rail.0, rail.1, rail.0 + rail.2, rail.1, LINE, 2.0);
        d.line(
            rail.0,
            rail.1,
            rail.0 + rail.2 * value.clamp(0.0, 1.0),
            rail.1,
            GOLD,
            2.0,
        );
    }

    fn draw_arp_lane_button(&self, d: &mut Draw, r: Rect, lane: Lane) {
        let on = self.arp_ui.lane == lane;
        let color = if on { GOLD } else { TEXT };
        self.button_surface(d, r, on, GOLD);
        style::Icon::Glyph(lane.glyph()).draw(d, r.0 + r.2 * 0.5, r.1 + 24.0, color);
        let lines = &[lane.label()];
        let center_y = r.1 + (r.3 + 24.0) * 0.5;
        for (i, line) in lines.iter().enumerate() {
            let y = center_y + (i as f32 - (lines.len() - 1) as f32 * 0.5) * 14.0;
            d.text_middle(
                r.0 + (r.2 - d.text_width(line, TEXT_LABEL)) * 0.5,
                y,
                line,
                TEXT_LABEL,
                color,
            );
        }
    }

    fn draw_arp_page_status(&self, d: &mut Draw, r: Rect, page: usize) {
        if (!self.programming || self.snapshot.arp_row_index as usize == page)
            && self.arp_playback().running
        {
            d.line(
                r.0 + 4.0,
                r.1 + r.3 - 2.0,
                r.0 + r.2 - 4.0,
                r.1 + r.3 - 2.0,
                GOLD,
                2.0,
            );
        }
        if self.snapshot.arp_pending as usize == page {
            d.circle(r.0 + r.2 - 4.0, r.1 + 4.0, 2.5, GOLD, true);
        }
    }

    pub(super) fn string_readout(
        &self,
        d: &mut Draw,
        x: f32,
        y: f32,
        g: Rect,
        label: &str,
        color: Color,
    ) {
        let width = d.text_width(label, 14.0);
        // Keep the value beside the bar, including at the top of its range.
        let left = if x + 18.0 + width <= g.0 + g.2 {
            x + 18.0
        } else {
            x - 18.0 - width
        };
        d.text(
            left,
            (y + 5.0).clamp(g.1 + 14.0, g.1 + g.3),
            label,
            14.0,
            color,
        );
    }

    pub(in crate::ui) fn draw_arp_editor(&self, d: &mut Draw) {
        let s = ARP_SURFACE;
        d.scissor(s.0, s.1, s.2, s.3);
        let saved_offset = d.offset_x;
        let saved_disabled = d.disabled;
        // Bypass stops playback; settings remain editable. Only the pattern dims.
        d.disabled = false;
        self.draw_arp_strings(d);
        let status = if self.params.daw_alignment.value() && !self.arp_playback().aligned {
            "DAW alignment · using local clock"
        } else if self.snapshot.arp_pending < 8 {
            "Row queued for next boundary"
        } else {
            ""
        };
        let graph = graph_rect();
        let status = self.fit_text(d, status, graph.2, TEXT_LABEL);
        d.text(graph.0, s.1 + s.3 - 7.0, &status, TEXT_LABEL, MUTED);

        if self.arp_ui.chooser_position > 0.0 {
            let r = chooser_area();
            d.scissor(r.0, r.1, r.2, r.3);
            d.offset_x = saved_offset + (1.0 - self.arp_ui.chooser_position) * r.2;
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            self.draw_arp_chooser(d);
            d.offset_x = saved_offset;
            d.scissor(s.0, s.1, s.2, s.3);
        }
        d.offset_x = saved_offset;
        if let Some(which @ (Dropdown::Pages | Dropdown::Paste(_))) = self.arp_ui.dropdown {
            self.draw_arp_dropdown(d, which);
        }
        d.disabled = saved_disabled;
        d.reset_scissor();
    }

    fn draw_tracker_voicing(&self, d: &mut Draw) {
        let state = self.arp_state();
        let row = state.editor_row(self.arp_ui.edit_page, false);
        for (inversion, name, value) in [
            (false, "Octave", row.arp_octave),
            (true, "Inversion", row.arp_inversion),
        ] {
            let r = tracker_voicing_rect(inversion);
            d.text_centered(
                r.0 + r.2 * 0.5,
                r.1 + r.3 * 0.5 + 4.5,
                &format!("{name} {value:+}"),
                TEXT_LABEL,
                if value == 0 { TEXT } else { GOLD },
            );
            for (plus, label) in [(false, "−"), (true, "+")] {
                self.arp_button(
                    d,
                    tracker_voicing_button(inversion, plus),
                    label,
                    false,
                    GOLD,
                );
            }
        }
    }

    fn draw_arp_strings(&self, d: &mut Draw) {
        let s = ARP_SURFACE;
        let p = self.arp_params();
        let power = power_rect();
        let inheritance = self.editor_arp_inheritance();
        let inherited = inheritance == Some(true);
        let enabled = inherited || self.editor_arp_enabled();
        if !inherited {
            self.draw_power_switch(d, power, enabled, TEAL, power);
        }
        d.text(
            if inherited {
                power.0
            } else {
                power.0 + power.2 + 9.0
            },
            s.1 + 29.0,
            if self.programming {
                "TRACKER ARP"
            } else {
                "PERFORMANCE ARP"
            },
            16.0,
            if enabled { TEXT } else { MUTED },
        );
        if self.programming {
            let disabled = d.disabled;
            d.disabled |= inheritance.is_none();
            self.arp_button(d, inherit_rect(), "Inherit", inherited, TEAL);
            d.disabled = disabled;
            self.draw_tracker_voicing(d);
        }
        if inherited {
            return;
        }
        let repeat = p.arp_mode.value() != 0;
        let r = mode_rect();
        self.button_surface(d, r, repeat, GOLD);
        let color = if repeat { GOLD } else { MUTED };
        // Compact repeat barline shares the header's button height.
        let x = r.0 + 18.0;
        let y = r.1 + r.3 * 0.5;
        d.rect(x, y - 8.0, 1.5, 16.0, color);
        d.rect(x + 4.0, y - 8.0, 3.0, 16.0, color);
        for dy in [-3.5, 3.5] {
            d.circle(x - 5.0, y + dy, 1.5, color, true);
        }
        self.arp_button(
            d,
            order_rect(),
            &format!(
                "{} ▾",
                ARP_PATTERN_NAMES[p.arp_pattern.value().clamp(0, 7) as usize]
            ),
            self.menu == Some(Menu::ArpPattern),
            GOLD,
        );
        if !self.programming {
            let saved_disabled = d.disabled;
            d.disabled |= self.visible_loops() >= LOOPS;
            let r = loop_add_rect();
            self.button_surface(d, r, false, TEAL);
            let (r, label) = self.button_content(d, r, "+ Loop");
            let cx = r.0 + r.2 * 0.5;
            let cy = r.1 + r.3 * 0.5;
            let px = d.ox + (cx + d.offset_x) * d.s;
            let py = d.oy + cy * d.s;
            let width = d.text_width(label, TEXT_LABEL);
            d.c.save();
            d.c.translate(px, py);
            d.c.rotate(-std::f32::consts::FRAC_PI_2);
            d.c.translate(-px, -py);
            d.text_middle(cx - width * 0.5, cy, label, TEXT_LABEL, TEXT);
            d.c.restore();
            d.disabled = saved_disabled;
        }
        self.arp_icon_button(
            d,
            sync_rect(),
            "",
            p.arp_sync.value(),
            TEAL,
            style::Icon::Host,
        );
        if p.arp_sync.value() {
            self.arp_button(
                d,
                rate_rect(),
                &format!("{} ▾", time_label(p.rate.value(), true)),
                self.menu == Some(Menu::ArpRate),
                GOLD,
            )
        } else if let Some(control) = self.control("arp_free_ms") {
            self.draw_control(d, &control, rate_rect(), GOLD);
        }
        d.line(
            s.0 + 12.0,
            s.1 + 43.0,
            s.0 + s.2 - 12.0,
            s.1 + 43.0,
            LINE,
            1.0,
        );
        let settings = self
            .arp_state()
            .editor_row(self.arp_ui.edit_page, !self.programming)
            .settings;
        let lock = lock_length_rect();
        self.button_surface(d, lock, settings.lock_length, GOLD);
        let (lock, _) = self.button_content(d, lock, "");
        let icon = if settings.lock_length {
            style::Icon::Lock
        } else {
            style::Icon::Unlock
        };
        icon.draw(
            d,
            lock.0 + lock.2 * 0.5,
            lock.1 + lock.3 * 0.5,
            if settings.lock_length { GOLD } else { MUTED },
        );
        let disabled = d.disabled;
        d.disabled |= !settings.lock_length;
        self.arp_button(
            d,
            fixed_steps_rect(),
            &format!("Steps: {}", settings.steps),
            false,
            GOLD,
        );
        let steps = fixed_steps_rect();
        for (x, symbol) in [(steps.0 + 12.0, "−"), (steps.0 + steps.2 - 12.0, "+")] {
            d.text_centered(x, steps.1 + steps.3 * 0.5 + 7.0, symbol, 20.0, TEXT);
        }
        self.arp_icon_button(
            d,
            fill_rect(),
            "",
            settings.fill == Fill::Evolve,
            TEAL,
            style::Icon::Glyph(if settings.fill == Fill::Evolve {
                style::GLYPH_REFRESH
            } else {
                style::GLYPH_REPEAT
            }),
        );
        d.disabled = disabled;
        if settings.pattern == 7 {
            self.arp_icon_button(
                d,
                custom_edit_rect(),
                "",
                self.panel == Some(Panel::Custom),
                TEAL,
                style::Icon::Glyph(style::GLYPH_PENCIL),
            );
        }
        for (i, lane) in LANES.iter().enumerate() {
            self.draw_arp_lane_button(d, lane_rect(i), *lane);
        }
        let disabled = d.disabled;
        d.disabled = !enabled;
        self.draw_arp_page_slide(d, content_rect(), Self::draw_arp_pattern);
        d.disabled = disabled;
    }

    fn draw_arp_page_slide(&self, d: &mut Draw, r: Rect, draw: impl Fn(&Self, &mut Draw, usize)) {
        d.scissor(r.0, r.1, r.2, r.3);
        let offset = d.offset_x;
        let progress = pleasant_ui::page_slide::position(0.0, 1.0, self.arp_ui.page_elapsed);
        let direction = if self.arp_ui.edit_page >= self.arp_ui.page_from {
            1.0
        } else {
            -1.0
        };
        if progress < 1.0 {
            d.offset_x = offset - direction * progress * r.2;
            draw(self, d, self.arp_ui.page_from);
        }
        d.offset_x = offset + direction * (1.0 - progress) * r.2;
        draw(self, d, self.arp_ui.edit_page);
        d.offset_x = offset;
        let surface = if self.fx_ui.open {
            DELAY_SURFACE
        } else {
            ARP_SURFACE
        };
        d.scissor(surface.0, surface.1, surface.2, surface.3);
    }

    fn draw_arp_pattern(&self, d: &mut Draw, page_index: usize) {
        let state = self.arp_state();
        let page = &state.editor_row(page_index, !self.programming).page;
        let (notes, len) = self.arp_cycle_for_page(page_index);
        let layout = self.arp_string_layout(page_index, len);
        let g = graph_rect();
        let count = self
            .maximum_routed_plain(
                "arp_octaves",
                state
                    .editor_row(page_index, !self.programming)
                    .settings
                    .octaves as f32,
            )
            .round()
            .clamp(1.0, LOOPS as f32) as usize;
        let active_loops =
            if !self.programming || self.snapshot.arp_row_index as usize == page_index {
                self.routed_plain("arp_octaves").unwrap_or(
                    state
                        .editor_row(page_index, !self.programming)
                        .settings
                        .octaves as f32,
                ) as usize
            } else {
                state
                    .editor_row(page_index, !self.programming)
                    .settings
                    .octaves as usize
            };
        for (i, r) in self.loop_headers(page_index, count) {
            let color = if i < active_loops { TEAL } else { MUTED };
            d.rounded_rect(r.0, r.1, r.2, r.3, 4.0, alpha(color, 0.08));
            let text = format!("{}", i + 1);
            let label = self.fit_text(d, &text, 44.0, TEXT_LABEL);
            d.text_middle(r.0 + 8.0, r.1 + r.3 * 0.5, &label, TEXT_LABEL, TEXT);
            let start = r.0 + 60.0;
            let inversion = format!("Inv {:+}", page.loops[i].inversion);
            d.text_centered(
                start + 59.0,
                r.1 + r.3 * 0.5 + 4.0,
                &inversion,
                TEXT_LABEL,
                TEXT,
            );
            style::Icon::Glyph(style::GLYPH_ARROWS_V).draw_sized(
                d,
                start + 176.0,
                r.1 + r.3 * 0.5,
                16.0,
                TEXT,
            );
            for button in 0..5 {
                if button != 2 || self.loop_remove_visible(i, count) {
                    let rect = loop_header_button(r, button);
                    self.button_surface(d, rect, false, color);
                    let cx = rect.0 + rect.2 * 0.5;
                    let cy = rect.1 + rect.3 * 0.5;
                    if button == 2 {
                        d.text_middle(cx - d.text_width("×", 20.0) * 0.5, cy, "×", 20.0, TEXT);
                    } else {
                        let glyph = if button == 0 || button == 3 {
                            style::GLYPH_MINUS
                        } else {
                            style::GLYPH_PLUS
                        };
                        style::Icon::Glyph(glyph).draw_sized(d, cx, cy, 14.0, TEXT);
                    }
                }
            }
            if !self.programming || self.snapshot.arp_row_index as usize == page_index {
                if let Some(live) = self.routed_plain(crate::engine::routing::LOOP_OCTAVE_IDS[i]) {
                    d.text_centered(
                        r.0 + r.2 * 0.5,
                        r.1 + r.3 + 11.0,
                        &format!("live {live:+.0}"),
                        10.0,
                        TEAL,
                    );
                }
            }
            if r.0 > g.0 + 2.0 {
                dotted(d, r.0 - 2.0, g.1, g.3, alpha(MUTED, 0.3));
            }
        }
        if len == 0 {
            d.text_centered(
                g.0 + g.2 * 0.5,
                g.1 + g.3 * 0.5,
                "Select a chord to adjust its arpeggio",
                13.0,
                MUTED,
            );
            return;
        }
        let lane = self.arp_ui.lane;
        let range = lane.range();
        let baseline = value_y(if range.0 < 0.0 { 0.0 } else { range.0 }, range, g);
        d.line(g.0, baseline, g.0 + g.2, baseline, alpha(LINE, 0.6), 1.0);
        for (i, &note) in notes.iter().enumerate().take(len) {
            let step = page.step(i, self.arp_pass_len_for_page(page_index));
            if layout.x[i] + layout.spacing < g.0 || layout.x[i] - layout.spacing > g.0 + g.2 {
                continue;
            }
            let nominal = layout.x[i];
            if layout.visual(i) == 0 {
                d.text_centered(nominal, g.1 + 13.0, "1", 11.0, GOLD);
            }
            let x = nominal + step.micro as f32 * layout.interval[i] / 100.0;
            let active = (!self.programming || self.snapshot.arp_row_index as usize == page_index)
                && self.arp_playback().running
                && self.arp_playback().step as usize == i;
            let color = if step.state == StepState::Skip {
                MUTED
            } else if active {
                GOLD
            } else {
                TEAL
            };
            let visible_active = if (!self.programming
                || self.snapshot.arp_row_index as usize == page_index)
                && self.arp_playback().running
            {
                i < self.arp_playback().len as usize
            } else {
                i < len / count * active_loops
            };
            let mute_opacity = if step.state == StepState::Mute {
                0.5
            } else {
                1.0
            };
            let strength = if step.state == StepState::Play && visible_active {
                1.0
            } else {
                0.38
            } * mute_opacity;
            let bar_width = (layout.spacing * 0.48).clamp(10.0, 26.0);
            dotted(d, nominal, g.1, g.3, alpha(MUTED, 0.25 * mute_opacity));
            let y = value_y(lane.get(step), range, g);
            let height = (baseline - y).abs().max(5.0);
            let top = baseline.min(y).min(g.1 + g.3 - height);
            d.rounded_rect(
                x - bar_width * 0.5,
                top,
                bar_width,
                height,
                5.0_f32.min(height * 0.5),
                alpha(color, strength),
            );
            // Hover or drag reveals exact values; note labels remain stable while timing moves.
            let hovered = self
                .pointer
                .is_some_and(|(px, py)| hit(g, px, py) && layout.at(px) == i);
            if hovered
                || self.arp_ui.drag.is_some_and(|drag| {
                    matches!(drag.kind, DragKind::Step(_) | DragKind::Timing) && drag.last == i
                })
            {
                let label = lane.format(lane.get(step));
                let width = d.text_width(&label, 14.0);
                let left = (x - width * 0.5).clamp(g.0, g.0 + g.2 - width);
                d.text(left, g.1 - 9.0, &label, 14.0, TEXT);
            }
            let strip = gate_rect();
            let handle_width = (layout.spacing - 2.0).clamp(8.0, 40.0);
            d.line(
                nominal,
                strip.1,
                nominal,
                strip.1 + strip.3,
                alpha(MUTED, 0.35),
                1.0,
            );
            d.rounded_rect(
                x - handle_width * 0.5,
                strip.1 + 2.0,
                handle_width,
                18.0,
                4.0,
                alpha(color, 0.18),
            );
            if handle_width >= 14.0 {
                style::Icon::Glyph(style::GLYPH_ARROWS_H).draw_sized(
                    d,
                    x,
                    strip.1 + 11.0,
                    13.0,
                    color,
                );
            } else {
                d.line(
                    x - handle_width * 0.25,
                    strip.1 + 11.0,
                    x + handle_width * 0.25,
                    strip.1 + 11.0,
                    color,
                    1.2,
                );
            }
            if self
                .arp_ui
                .drag
                .is_some_and(|drag| matches!(drag.kind, DragKind::Timing) && drag.origin == i)
            {
                self.string_readout(d, x, strip.1, strip, &format!("{:+}%", step.micro), TEXT);
            }
            let pitch = note + step.octave as i16 * 12;
            let label = if (0..=127).contains(&pitch) {
                self.note_name(pitch as u8)
            } else {
                "—".into()
            };
            d.text_centered(
                nominal,
                g.1 + g.3 + 43.0,
                &label,
                if layout.spacing < 23.0 { 11.0 } else { 13.0 },
                alpha(if active { GOLD } else { MUTED }, mute_opacity),
            );
            let r = layout.status(i);
            let (glyph, mark, selected) = match step.state {
                StepState::Play => (style::GLYPH_PLAY, TEAL, false),
                StepState::Mute => (style::GLYPH_VOLUME_OFF, TEAL, true),
                StepState::Skip => (style::GLYPH_SKIP, MUTED, true),
            };
            self.button_surface(d, r, selected, mark);
            style::Icon::Glyph(glyph).draw_sized(
                d,
                r.0 + r.2 * 0.5,
                r.1 + r.3 * 0.5,
                14.0,
                mark,
            );
        }
    }

    pub(in crate::ui) fn draw_keyboard_pages(&self, d: &mut Draw) {
        self.surface(d, PIANO_SURFACE);
        d.scissor(
            PIANO_SURFACE.0,
            PIANO_SURFACE.1,
            PIANO_SURFACE.2,
            PIANO_SURFACE.3,
        );
        self.draw_piano(d);
        d.reset_scissor();
    }

    pub(super) fn draw_arp_dropdown(&self, d: &mut Draw, which: Dropdown) {
        if which == Dropdown::Pages {
            let first = page_rect(0);
            d.rounded_rect(first.0 - 4.0, first.1 - 4.0, 236.0, 200.0, 4.0, PANEL);
            for page in 0..8 {
                let r = page_rect(page);
                self.arp_button(
                    d,
                    r,
                    &format!("Row {}", page + 1),
                    self.arp_ui.edit_page == page,
                    TEAL,
                );
                self.draw_arp_page_status(d, r, page);
                self.arp_icon_button(
                    d,
                    page_copy_rect(page),
                    "",
                    false,
                    TEAL,
                    style::Icon::Glyph(style::GLYPH_COPY),
                );
                let disabled = d.disabled;
                d.disabled |= self.arp_ui.clipboard.is_none();
                self.arp_icon_button(
                    d,
                    page_paste_rect(page),
                    " ▾",
                    false,
                    TEAL,
                    style::Icon::Glyph(style::GLYPH_CLIPBOARD),
                );
                d.disabled = disabled;
            }
            return;
        }
        if let Dropdown::Paste(_) = which {
            self.draw_arp_dropdown(d, Dropdown::Pages);
        }
        let count = which.len();
        for i in 0..count {
            let r = dropdown_rect(which, i);
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            let (label, on) = match which {
                Dropdown::TapLane => (TAP_LANES[i].label(), self.arp_ui.tap_lane == TAP_LANES[i]),
                Dropdown::Paste(_) => (["Complete row", "Pattern only"][i], false),
                Dropdown::Pages => unreachable!(),
            };
            self.arp_button(d, r, label, on, GOLD);
        }
    }

    fn draw_arp_chooser(&self, d: &mut Draw) {
        let u = &self.arp_ui;
        let selected = match u.chooser_draw {
            Chooser::Order => self.arp_params().arp_pattern.value() as usize,
        };
        for i in 0..self.chooser_count() {
            let r = self.chooser_hit_rect(i);
            let on = selected == i;
            self.button_surface(d, r, on, GOLD);
            let color = if on {
                GOLD
            } else if self.hover_amount(r) > 0.01 {
                TEXT
            } else {
                MUTED
            };
            let cx = r.0 + r.2 * 0.5;
            let cy = r.1 + r.3 * 0.36;
            let label = match u.chooser_draw {
                Chooser::Order => {
                    let w = (r.2 - 36.0).min(76.0);
                    let h = 30.0;
                    let points: Vec<_> = ORDER_CONTOURS[i]
                        .iter()
                        .enumerate()
                        .map(|(j, &n)| {
                            (
                                cx - w * 0.5 + j as f32 * w / 4.0,
                                cy + h * 0.5 - n as f32 * h / 4.0,
                            )
                        })
                        .collect();
                    d.poly(&points, color, 1.6);
                    for &(x, y) in &points {
                        d.circle(x, y, 2.5, color, true);
                    }
                    ARP_PATTERN_NAMES[i]
                }
            };
            let old_font = d.font;
            d.font = self.bold_font.get();
            d.text_centered(cx, r.1 + r.3 * 0.66, label, 14.0, color);
            d.font = old_font;
            if u.chooser_cursor == i && u.chooser.is_some() {
                d.outline_rounded(
                    r.0 + 2.0,
                    r.1 + 2.0,
                    r.2 - 4.0,
                    r.3 - 4.0,
                    6.0,
                    alpha(GOLD, 0.5),
                    1.0,
                );
            }
        }
    }
}
