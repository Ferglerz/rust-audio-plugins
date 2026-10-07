use super::*;

impl ChordboardView {

    pub(super) fn arp_items(&self, state: &State) -> Vec<Item> {
        let p = &state.lanes[0][self.sequencer_ui.edit_pages[0]];
        let mut items = Vec::new();
        item(
            &mut items,
            arp_interlock_rect(),
            format!(
                "Interlock: {}",
                field_value(Field::Interlock, p, &p.steps[0], &self.sequencer_ui)
            ),
            Action::Choice(Field::Interlock),
            p.interlock_override.is_some(),
        );
        items
    }

    pub(in crate::ui) fn draw_arp_pattern_settings(&self, d: &mut Draw) {
        let state = self.params.sequencer.snapshot();
        for item in self.arp_items(&state) {
            d.button(item.rect, &item.label, item.on, TEAL);
        }
    }

    pub(in crate::ui) fn draw_arp_pattern_edit(&self, d: &mut Draw) {
        if let Some(edit) = &self.sequencer_ui.edit {
            if hit(ARP_PAD, edit.rect.0, edit.rect.1) {
                d.value_edit(edit, TEAL);
            }
        }
    }

    pub(super) fn expression_mode_items(&self) -> Vec<Item> {
        if !self.sequencer_open() {
            return Vec::new();
        }
        let mut items = Vec::new();
        let mut add = |source, label: &str, field| {
            let r = meter_rect(source);
            item(
                &mut items,
                (r.0, MOD_SURFACE.1 + 10.0, r.2, 24.0),
                label,
                Action::Modifier(Some(field)),
                self.sequencer_ui.modifier == Some(field),
            );
        };
        for (source, label, field) in EXPRESSION_MODES {
            add(source, label, field);
        }
        let state = self.params.sequencer.snapshot();
        let lane = if self.sequencer_ui.lane == 1 { 1 } else { 0 };
        let page = &state.lanes[lane][self.sequencer_ui.edit_pages[lane]];
        for (source, number, label) in [(1, 1, "Mod"), (2, 11, "Expression"), (3, 2, "Breath")] {
            if let Some(track) = page.cc_numbers.iter().position(|&cc| cc == number) {
                add(source, label, Field::Cc(track));
            }
        }
        items
    }

    pub(in crate::ui) fn draw_expression_modes(&self, d: &mut Draw) {
        for c in self.expression_mode_items() {
            d.button(c.rect, &c.label, c.on, TEAL);
        }
    }

    pub(super) fn seq_items(&self, state: &State) -> Vec<Item> {
        let u = &self.sequencer_ui;
        let r = surface();
        let mut items = Vec::new();
        item(
            &mut items,
            lock_lengths_rect(),
            "Lock lengths",
            Action::LengthLock,
            self.params.seq_length_lock.value(),
        );
        for lane in VISIBLE_LANES {
            let y = lane_y(lane);
            item(
                &mut items,
                (r.0 + 8.0, y, 78.0, 20.0),
                LANES[lane],
                Action::LaneEnabled(lane),
                state.lanes[lane][u.edit_pages[lane]].enabled,
            );
            for page in 0..8 {
                item(
                    &mut items,
                    (
                        page_row_rect(lane).0 + page as f32 * 26.0,
                        y,
                        23.0,
                        27.0,
                    ),
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
        let mut fields = vec![
            ("Gate %", Field::Gate),
            ("Octave", Field::Octave),
            ("Chance %", Field::Probability),
            ("Repeats", Field::Ratchets),
            ("Timing %", Field::Micro),
        ];
        if lane != 1 {
            fields.push(("Note", Field::Tone));
        }
        for (label, field) in fields {
            entries.push((
                format!("{label}: {}", field_value(field, page, step, u)),
                Action::Field(field),
                u.modifier == Some(field),
            ));
        }
        entries.push((
            if step.enabled { "Step on" } else { "Step off" }.into(),
            Action::Enabled,
            step.enabled,
        ));
        let modifiers = MODIFIERS.len();
        let rects = header_slots(modifiers, entries.len() - modifiers);
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
                let beats = if lane == 0 {
                    self.params.rate.value()
                } else {
                    page.rate
                };
                let per_bar = steps_per_bar(
                    beats,
                    self.snapshot.time_sig_num,
                    self.snapshot.time_sig_den,
                );
                let shown = page.length as usize;
                let mut bar = 0;
                while bar < shown.min(visible_steps()) {
                    let end = (bar + per_bar).min(shown).min(visible_steps());
                    if (bar / per_bar) % 2 == 1 {
                        let a = step_rect(lane, bar);
                        let b = step_rect(lane, end - 1);
                        d.rounded_rect(
                            a.0 - 1.0,
                            a.1 - 3.0,
                            b.0 + b.2 - a.0 + 2.0,
                            a.3 + 6.0,
                            4.0,
                            alpha(TEXT, 0.06),
                        );
                    }
                    bar = end;
                }
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
                        let t = (slider_value(field, &p.steps[step]) - min) / (max - min);
                        d.rect(
                            c.rect.0,
                            c.rect.1 + c.rect.3 * (1.0 - t),
                            c.rect.2,
                            c.rect.3 * t,
                            alpha(lane_color(lane), if c.on { 0.55 } else { 0.2 }),
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
                            10.0,
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

    pub(in crate::ui) fn draw_sequencer(&self, d: &mut Draw) {
        let u = &self.sequencer_ui;
        if u.progress > 0.0 {
            let r = surface();
            d.scissor(r.0, r.1, r.2, r.3);
            let offset = d.offset_x;
            d.offset_x += (1.0 - u.progress) * r.2;
            d.rounded_rect(r.0, r.1, r.2, r.3, 8.0, PANEL);
            d.outline(r, LINE);
            d.text(r.0 + 16.0, r.1 + 29.0, "SEQUENCER", 15.0, TEXT);
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
                    9.0,
                    TEXT,
                );
            }
            let items = self.seq_items(&state);
            for c in items
                .iter()
                .filter(|c| !matches!(c.action, Action::SelectStep(..)))
            {
                let color = match c.action {
                    Action::SelectStep(l, _) | Action::EditPage(l, _) | Action::LaneEnabled(l) => {
                        lane_color(l)
                    }
                    _ => TEAL,
                };
                d.button(c.rect, &c.label, c.on, color);
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
