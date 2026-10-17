use super::*;

impl ChordboardView {
    pub(in crate::ui) fn sequencer_button_rects(&self) -> Vec<(Rect, bool)> {
        if self.sequencer_ui.progress <= 0.0 {
            return Vec::new();
        }
        let state = self.params.sequencer.snapshot();
        self.seq_items(&state).into_iter()
            .filter(|item| !matches!(item.action, Action::SelectStep(..)))
            .map(|item| (item.rect, item.on))
            .collect()
    }

    pub(super) fn seq_items(&self, state: &State) -> Vec<Item> {
        let u = &self.sequencer_ui;
        let mut items = Vec::new();
        item(
            &mut items,
            lock_lengths_rect(),
            "Lock lengths",
            Action::LengthLock,
            self.params.seq_length_lock.value(),
        );
        for lane in VISIBLE_LANES {
            item(
                &mut items,
                lane_label_rect(lane),
                LANES[lane],
                Action::LaneEnabled(lane),
                state.lanes[lane][u.edit_pages[lane]].enabled,
            );
            for page in 0..8 {
                item(
                    &mut items,
                    page_rect(lane, page),
                    (page + 1).to_string(),
                    Action::EditPage(lane, page),
                    u.edit_pages[lane] == page,
                );
            }
            let p = &state.lanes[lane][u.edit_pages[lane]];
            for visible in 0..visible_steps() {
                let step = visible + first_step();
                if step >= p.length as usize {
                    continue;
                }
                let s = &p.steps[step];
                let label = if lane == 3 && s.live && s.enabled {
                    "Live".into()
                } else {
                    (step + 1).to_string()
                };
                item(
                    &mut items,
                    step_rect(lane, visible),
                    label,
                    Action::SelectStep(lane, step),
                    s.enabled && step >= p.start as usize && step <= p.end as usize,
                );
            }
        }
        let lane = if VISIBLE_LANES.contains(&u.lane) {
            u.lane
        } else {
            0
        };
        items.extend(self.seq_inspector_items(state, lane, inspector_rect(), true));
        items
    }

    pub(super) fn seq_inspector_items(&self, state: &State, _tab: usize, _i: Rect, _tabs: bool) -> Vec<Item> {
        let u = &self.sequencer_ui;
        let lane = if VISIBLE_LANES.contains(&u.lane) {
            u.lane
        } else {
            0
        };
        let page = &state.lanes[lane][u.edit_pages[lane]];
        let step = &page.steps[u.step.min(page.steps.len() - 1)];
        let mut entries = Vec::new();
        for (label, field) in MODIFIERS {
            entries.push(((*label).to_string(), Action::Modifier(field), u.modifier == field));
        }
        // Modifier buttons already expose the other step values on the cells.
        if lane != 1 {
            entries.push((
                "Note".into(),
                Action::Field(Field::Tone),
                u.modifier == Some(Field::Tone),
            ));
        }
        entries.push((
            if step.enabled { "Step on" } else { "Step off" }.into(),
            Action::Enabled,
            step.enabled,
        ));
        if lane != 0 {
            let label = ARP_RATES
                .iter()
                .find(|(_, beats)| (*beats - page.rate).abs() < 0.0001)
                .map_or_else(
                    || format!("{:.3}", page.rate),
                    |(label, _)| label.to_string(),
                );
            entries.push((format!("Rate: {label}"), Action::PageRate, false));
        }
        let rects = header_slots(entries.len());
        entries
            .into_iter()
            .zip(rects)
            .map(|((label, action, on), rect)| Item {
                rect,
                label,
                action,
                on,
            })
            .collect()
    }

    pub(super) fn draw_sequencer_rows(&self, d: &mut Draw, state: &State) {
        let u = &self.sequencer_ui;
        let offset = d.offset_x;
        for lane in VISIBLE_LANES {
            let first = step_rect(lane, 0);
            let last = step_rect(lane, visible_steps() - 1);
            let width = last.0 + last.2 - first.0;
            let surface = surface();
            let clip_left = (first.0 + offset).max(surface.0);
            let clip_right = (first.0 + offset + width).min(surface.0 + surface.2);
            if clip_right <= clip_left {
                continue;
            }
            d.scissor(
                clip_left,
                first.1,
                clip_right - clip_left,
                last.1 + last.3 - first.1,
            );
            for page_index in 0..8 {
                let slide = (page_index as f32 - u.page_positions[lane]) * (width + 3.0);
                if slide.abs() >= width + 3.0 {
                    continue;
                }
                d.offset_x = offset + slide;
                let page = &state.lanes[lane][page_index];
                let per_group = cells_per_group(self.meter().0);
                for visible in 0..visible_steps() {
                    let step = first_step() + visible;
                    if step >= page.length as usize {
                        let rect = step_rect(lane, visible);
                        d.rounded_rect(rect.0, rect.1, rect.2, rect.3, 4.0, alpha(BG, 0.85));
                        d.outline(rect, alpha(LINE, 0.3));
                        continue;
                    }
                    let number_alpha = if page_index == u.edit_pages[lane] {
                        number_alpha(u.number_age[lane][step])
                    } else {
                        0.0
                    };
                    let rect = step_rect(lane, visible);
                    let c = Item {
                        rect,
                        label: (step + 1).to_string(),
                        action: Action::SelectStep(lane, step),
                        on: page.steps[step].enabled
                            && step >= page.start as usize
                            && step <= page.end as usize,
                    };
                    d.button(rect, "", c.on, lane_color(lane));
                    // Paint on the opaque face, before values and selection indicators.
                    // Preserve the lower 2px strike accent and keep rests shaded too.
                    if light_bar(step, per_group) {
                        let tint = if d.light { BG } else { TEXT };
                        // Transform the opaque palette color first: changing its alpha
                        // before theme lookup bypasses the exact light-theme palette.
                        let opacity = d.alpha_mul;
                        d.alpha_mul *= if d.light { 0.3 } else { 0.14 };
                        d.rect(rect.0, rect.1, rect.2, rect.3 - 2.0, tint);
                        d.alpha_mul = opacity;
                    }
                    if u.modifier.is_none() {
                        d.text_centered(
                            rect.0 + rect.2 / 2.0,
                            rect.1 + rect.3 / 2.0 + 4.0,
                            &c.label,
                            11.0,
                            alpha(if c.on { lane_color(lane) } else { MUTED }, number_alpha),
                        );
                    }
                    if let Some(field) = u.modifier {
                        let p = page;
                        let value = field_value(field, p, &p.steps[step], u);
                        let (min, max) = slider_range(field);
                        let (fill_start, fill_end) = slider_fill(field, &p.steps[step]);
                        d.rect(
                            c.rect.0,
                            c.rect.1 + c.rect.3 * fill_start,
                            c.rect.2,
                            c.rect.3 * (fill_end - fill_start),
                            alpha(if matches!(field, Field::Mod(_)) { GOLD } else { lane_color(lane) }, if c.on { 0.55 } else { 0.2 }),
                        );
                        if min < 0.0 {
                            let zero = c.rect.1 + c.rect.3 * (1.0 + min / (max - min));
                            d.line(
                                c.rect.0,
                                zero,
                                c.rect.0 + c.rect.2,
                                zero,
                                alpha(TEXT, 0.4),
                                1.0,
                            );
                        }
                        d.text_centered(
                            c.rect.0 + c.rect.2 / 2.0,
                            c.rect.1 + 20.0,
                            &value,
                            11.0,
                            alpha(TEXT, number_alpha),
                        );
                    }

                    if page_index == u.edit_pages[lane] && u.lane == lane && u.step == step {
                        d.outline(c.rect, TEXT);
                    }
                    if self.snapshot.seq_running
                        && self.snapshot.seq_pages[lane] as usize == page_index
                        && self.snapshot.seq_steps[lane] as usize == step
                    {
                        d.line(
                            c.rect.0,
                            c.rect.1 + c.rect.3 - 1.0,
                            c.rect.0 + c.rect.2,
                            c.rect.1 + c.rect.3 - 1.0,
                            GOLD,
                            3.0,
                        );
                    }
                }
            }
        }
        d.offset_x = offset;
        for lane in VISIBLE_LANES {
            let length = state.lanes[lane][u.edit_pages[lane]].length as usize;
            let first = first_step();
            if length > first && length <= first + visible_steps() {
                let end = step_rect(lane, length - first - 1);
                d.line(
                    end.0 + end.2 + 1.5,
                    end.1,
                    end.0 + end.2 + 1.5,
                    end.1 + end.3,
                    lane_color(lane),
                    2.0,
                );
            }
        }
        d.offset_x = offset;
        let r = surface();
        d.scissor(r.0, r.1, r.2, r.3);
    }

    pub(super) fn arp_rate_label(&self) -> String {
        let beats = self
            .routed_plain("rate")
            .unwrap_or(self.params.rate.value());
        ARP_RATES
            .iter()
            .find(|(_, b)| (b - beats).abs() < 0.0001)
            .map_or_else(|| format!("{beats:.3} beats"), |(label, _)| label.to_string())
    }

    fn draw_arp_settings(&self, d: &mut Draw) {
        let power = seq_power_rect();
        self.button(
            d,
            power,
            "",
            self.params.seq_enabled.value(),
            TEAL,
        );
        d.power_icon(
            power.0 + power.2 * 0.5,
            power.1 + power.3 * 0.5,
            if self.params.seq_enabled.value() { TEAL } else { MUTED },
        );
        let font = d.font;
        d.font = self.font.get();
        d.text(
            power.0 + power.2 + 10.0,
            module_title_y(SEQ_SURFACE.1, MODULE_TITLE_SIZE),
            "ARPEGGIATOR",
            MODULE_TITLE_SIZE,
            TEXT,
        );
        d.font = font;
        let looping = self.mode() == 3;
        for loop_choice in [false, true] {
            self.icon_button(
                d,
                repeat_rect(loop_choice),
                if loop_choice { "Loop" } else { "Once" },
                looping == loop_choice,
                GOLD,
                if loop_choice { style::Icon::Loop } else { style::Icon::Once },
            );
        }
        let index = self.params.arp_pattern.value().clamp(0, 6) as usize;
        let direction = arp_direction_rect();
        self.button(
            d,
            direction,
            &format!("{} ▾", ARP_PATTERN_NAMES[index]),
            self.menu == Some(Menu::ArpPattern),
            GOLD,
        );
        self.live_choice(d, "arp_pattern", index as f32, direction);
        for (c, r) in self.base_controls() {
            if matches!(c.id, "humanize" | "swing") {
                self.draw_control(d, &c, r, GOLD);
            }
        }
        self.draw_sequencer_rate(d);
        self.draw_interlock(d);
    }

    fn draw_sequencer_rate(&self, d: &mut Draw) {
        let synced = self.sweep_synced();
        self.button(
            d,
            seq_rate_sync_rect(),
            if synced { "Tempo sync" } else { "Free · ms" },
            synced,
            TEAL,
        );
        self.live_choice(d, "strum_sync", 1.0, seq_rate_sync_rect());
        if synced {
            self.button(
                d,
                seq_rate_rect(),
                &format!("Rate: {} ▾", self.arp_rate_label()),
                self.menu == Some(Menu::ArpRate),
                GOLD,
            );
            if self.routed_plain("rate").is_some() {
                self.live_highlight(d, seq_rate_rect());
            }
        } else if let Some((c, r)) = self
            .base_controls()
            .into_iter()
            .find(|(c, _)| c.id == "strum_ms")
        {
            d.rounded_rect(r.0, r.1, r.2, r.3, 6.0, alpha(TEXT, 0.025));
            self.draw_control(d, &c, r, GOLD);
        }
    }

    pub(in crate::ui) fn draw_sequencer(&self, d: &mut Draw) {
        let u = &self.sequencer_ui;
        if u.progress > 0.0 {
            let r = surface();
            d.scissor(r.0, r.1, r.2, r.3);
            let offset = d.offset_x;
            d.offset_x += (1.0 - u.progress) * r.2;
            self.draw_arp_settings(d);
            let state = self.params.sequencer.snapshot();
            for lane in VISIBLE_LANES {
                let length = state.lanes[lane][u.edit_pages[lane]].length;
                let track = length_rect(lane);
                d.rounded_rect(track.0, track.1, track.2, track.3, 4.0, BG);
                let end = track.0 + track.2 * length as f32 / 32.0;
                d.rounded_rect(
                    track.0,
                    track.1,
                    end - track.0,
                    track.3,
                    4.0,
                    alpha(lane_color(lane), 0.3),
                );
                d.line(end, track.1, end, track.1 + track.3, lane_color(lane), 2.0);
                d.text_centered(
                    track.0 + track.2 * 0.5,
                    track.1 + track.3 * 0.5 + 3.0,
                    &format!("Length · {length}"),
                    11.0,
                    TEXT,
                );
            }
            let items = self.seq_items(&state);
            for c in items
                .iter()
                .filter(|c| !matches!(c.action, Action::SelectStep(..)) && hit(surface(), c.rect.0, c.rect.1))
            {
                let color = match c.action {
                    Action::SelectStep(l, _) | Action::EditPage(l, _) | Action::LaneEnabled(l) => {
                        lane_color(l)
                    }
                    Action::Modifier(Some(Field::Mod(_))) => GOLD,
                    _ => TEAL,
                };
                self.button(d, c.rect, &c.label, c.on, color);
                if matches!(c.action, Action::Modifier(Some(Field::Mod(_)))) && !c.on {
                    d.text_centered(c.rect.0 + c.rect.2 * 0.5, c.rect.1 + c.rect.3 * 0.5 + 4.0, &c.label, 11.0, GOLD);
                }
                if let Action::EditPage(lane, page) = c.action {
                    if self.snapshot.seq_pages[lane] as usize == page {
                        d.line(
                            c.rect.0 + 3.0,
                            c.rect.1 + c.rect.3 - 2.0,
                            c.rect.0 + c.rect.2 - 3.0,
                            c.rect.1 + c.rect.3 - 2.0,
                            lane_color(lane),
                            2.0,
                        );
                    }
                    if self.snapshot.seq_pending[lane] as usize == page {
                        d.rect(c.rect.0 + c.rect.2 - 5.0, c.rect.1 + 2.0, 3.0, 3.0, GOLD);
                    }
                }
            }
            self.draw_sequencer_rate(d);
            self.draw_sequencer_rows(d, &state);
            if let Some(edit) = &u.edit {
                if hit(r, edit.rect.0, edit.rect.1) {
                    d.value_edit(edit, TEAL);
                }
            }

            d.offset_x = offset;
            d.reset_scissor();
        }
    }
}
