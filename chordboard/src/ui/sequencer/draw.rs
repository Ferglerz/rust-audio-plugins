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

    pub(in crate::ui) fn draw_sequencer_editor(&self, d: &mut Draw) {
        let u = &self.sequencer_ui;
        let state = self.params.sequencer.snapshot();
        let i = ARP_EDITOR;
        d.rounded_rect(i.0, i.1, i.2, i.3, 8.0, PANEL);
        d.outline(i, LINE);
        d.text(
            i.0 + 12.0,
            i.1 + 24.0,
            &format!("{} · Step {}", LANES[u.lane], u.step + 1),
            13.0,
            TEXT,
        );
        for c in self.seq_inspector_items(&state, u.tab, i, true) {
            d.button(c.rect, &c.label, c.on, lane_color(u.lane));
        }
        if let Some(edit) = &u.edit {
            if hit(i, edit.rect.0, edit.rect.1) {
                d.value_edit(edit, TEAL);
            }
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
        item(
            &mut items,
            loop_all_rect(),
            "Loop all",
            Action::LoopAll,
            self.loop_range().is_none(),
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
        let page = &state.lanes[3][u.edit_pages[3]];
        let first_visible = first_step();
        let last_visible = (first_visible + visible_steps()).min(page.length as usize);
        for (start, len) in page
            .box_spans()
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, n)| *n > 0)
        {
            let first = start.max(first_visible);
            let end = (start + len as usize).min(last_visible);
            if first >= end {
                continue;
            }
            for rect in span_rects(first, end) {
                item(&mut items, rect, "", Action::SelectStep(3, start), true);
            }
        }
        items.extend(self.seq_inspector_items(state, u.tab, inspector_rect(), true));
        items
    }

    pub(super) fn seq_inspector_items(&self, state: &State, tab: usize, i: Rect, tabs: bool) -> Vec<Item> {
        let u = &self.sequencer_ui;
        let mut items = Vec::new();
        for (index, (label, field)) in MODIFIERS.iter().enumerate() {
            item(
                &mut items,
                (
                    i.0 + 12.0 + (index % 3) as f32 * 80.0,
                    i.1 + 64.0 + (index / 3) as f32 * 26.0,
                    76.0,
                    22.0,
                ),
                *label,
                Action::Modifier(*field),
                u.modifier == *field,
            );
        }
        if tabs {
            for (position, (tab, name)) in TABS.iter().enumerate().enumerate() {
                item(
                    &mut items,
                    (
                        i.0 + 12.0 + position as f32 * 120.0,
                        i.1 + 36.0,
                        114.0,
                        24.0,
                    ),
                    *name,
                    Action::Tab(tab),
                    u.tab == tab,
                );
            }
        } else {
            item(
                &mut items,
                (i.0 + 12.0, i.1 + 18.0, 180.0, 28.0),
                TABS[tab],
                Action::Tab(tab),
                true,
            );
        }
        let p = &state.lanes[u.lane][u.edit_pages[u.lane]];
        let s = &p.steps[u.step];
        let fields: Vec<(&str, Field)> = match tab {
            0 if u.lane == 3 => vec![
                ("Box length", Field::Span),
                ("Root offset", Field::Root),
                ("Quality", Field::Quality),
                ("Inversion", Field::Inversion),
                ("Voicing", Field::Spread),
            ],
            0 => {
                let mut fields = vec![
                    ("Gate %", Field::Gate),
                    ("Octave", Field::Octave),
                    ("Chance %", Field::Probability),
                    ("Repeats", Field::Ratchets),
                    ("Timing %", Field::Micro),
                ];
                if u.lane != 1 {
                    fields.push(("Note", Field::Tone));
                }
                fields
            }
            1 => {
                let mut fields = vec![("Loop first", Field::Start)];
                if u.lane != 0 {
                    fields.insert(0, ("Step beats", Field::Rate));
                }
                fields
            }
            _ => Vec::new(),
        };
        let cols = 2;
        let fw = (i.2 - 24.0) / cols as f32;
        for (index, (label, field)) in fields.iter().enumerate() {
            let rect = (
                i.0 + 12.0 + (index % cols) as f32 * fw,
                i.1 + 122.0 + (index / cols) as f32 * 28.0,
                fw - 8.0,
                24.0,
            );
            item(
                &mut items,
                rect,
                format!("{label}: {}", field_value(*field, p, s, u)),
                if matches!(
                    field,
                    Field::Quality
                        | Field::Inversion
                        | Field::Spread
                        | Field::Interlock
                ) {
                    Action::Choice(*field)
                } else {
                    Action::Field(*field)
                },
                u.modifier == Some(*field),
            );
        }
        let y = i.1 + i.3 - 42.0;
        match tab {
            0 if u.lane == 3 => {
                item(
                    &mut items,
                    (i.0 + 12.0, y, 120.0, 27.0),
                    "Delete box",
                    Action::DeleteBox,
                    false,
                );
            }
            0 => {
                item(
                    &mut items,
                    (i.0 + 12.0, y, 102.0, 27.0),
                    if s.enabled { "Step on" } else { "Step off" },
                    Action::Enabled,
                    s.enabled,
                );
                item(
                    &mut items,
                    (i.0 + 124.0, y, 102.0, 27.0),
                    "Tie",
                    Action::Tie,
                    s.tie,
                );
            }
            1 => {
                let actions = [
                    ("Pattern on", Action::LaneEnabled(u.lane), p.enabled),
                    (
                        "Play pattern",
                        Action::PlayPage(u.lane),
                        self.snapshot.seq_pages[u.lane] as usize == u.edit_pages[u.lane],
                    ),
                    ("Ghosts", Action::Ghosts, false),
                ];
                for (index, (label, action, on)) in actions
                    .into_iter()
                    .enumerate()
                    .take(if u.lane == 3 { 2 } else { 5 })
                {
                    item(
                        &mut items,
                        (
                            i.0 + 12.0 + (index % 2) as f32 * 120.0,
                            y - 30.0 + (index / 2) as f32 * 30.0,
                            114.0,
                            27.0,
                        ),
                        label,
                        action,
                        on,
                    );
                }
            }
            _ => {}
        }
        items
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
                if lane == 3 {
                    let visible_start = first_step();
                    let visible_end =
                        (visible_start + visible_steps()).min(page.length as usize);
                    for index in visible_start..visible_end {
                        let rect = step_rect(3, index - visible_start);
                        d.outline(rect, alpha(COLORS[1], 0.15));
                    }
                    for (start, len) in page
                        .box_spans()
                        .iter()
                        .copied()
                        .enumerate()
                        .filter(|(_, n)| *n > 0)
                    {
                        let end = (start + len as usize).min(visible_end);
                        let first = start.max(visible_start);
                        if first >= end {
                            continue;
                        }
                        for rect in span_rects(first, end) {
                            d.button(rect, "", true, COLORS[1]);
                            let last = step_rect(3, end - 1 - visible_start);
                            if rect.1 == last.1 {
                                d.line(
                                    rect.0 + rect.2 - 4.0,
                                    rect.1 + 5.0,
                                    rect.0 + rect.2 - 4.0,
                                    rect.1 + rect.3 - 5.0,
                                    TEXT,
                                    2.0,
                                );
                            }
                            if u.lane == 3 && u.step == start {
                                d.outline(rect, TEXT);
                            }
                        }
                    }
                    if let Some((_, start, end)) = u.box_drag.filter(|(p, _, _)| *p == page_index) {
                        for rect in span_rects(start, end + 1) {
                            d.outline(rect, GOLD);
                        }
                    }
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
                    if lane == 3 {
                        if let Some((start, _)) = page.box_at(step) {
                            d.text_centered(
                                rect.0 + rect.2 / 2.0,
                                rect.1 + rect.3 / 2.0 + 4.0,
                                &format!("{:+}", page.steps[start].root_offset),
                                11.0,
                                alpha(COLORS[1], number_alpha),
                            );
                        }
                        continue;
                    }
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
            for c in items.iter().filter(|c| {
                !hit(ARP_EDITOR, c.rect.0, c.rect.1)
                    && !matches!(c.action, Action::SelectStep(..))
                    && !matches!(
                        c.action,
                        Action::Tab(_)
                            | Action::Field(_)
                            | Action::Choice(_)
                            | Action::Enabled
                            | Action::DeleteBox
                            | Action::Tie
                            | Action::Ghosts
                    )
            }) {
                let color = match c.action {
                    Action::SelectStep(l, _)
                    | Action::EditPage(l, _)
                    | Action::PlayPage(l)
                    | Action::LaneEnabled(l) => lane_color(l),
                    Action::LoopAll => lane_color(u.lane),
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
            self.draw_loop_strip(d, &state);
            self.draw_sequencer_rows(d, &state);

            d.offset_x = offset;
            d.reset_scissor();
        }
    }

    pub(super) fn draw_loop_strip(&self, d: &mut Draw, state: &State) {
        let u = &self.sequencer_ui;
        let length = state.lanes[u.lane][u.edit_pages[u.lane]].length as usize;
        let range = self.loop_range();
        let color = lane_color(u.lane);
        for step in 0..length.min(visible_steps()) {
            let r = loop_step_rect(step);
            let inside = range.is_some_and(|(a, b)| (a..=b).contains(&step));
            let fill = if inside {
                alpha(color, 0.32)
            } else {
                alpha(TEXT, 0.03 + d.is_hovered(r) as u8 as f32 * 0.04)
            };
            d.rounded_rect(r.0, r.1, r.2, r.3, 5.0, fill);
            let edge = range.is_some_and(|(a, b)| step == a || step == b);
            if step % 4 == 0 || edge {
                d.text_centered(
                    r.0 + r.2 * 0.5,
                    r.1 + r.3 * 0.5 + 3.5,
                    &(step + 1).to_string(),
                    TEXT_SMALL,
                    if inside { TEXT } else { MUTED },
                );
            }
        }
    }
}
