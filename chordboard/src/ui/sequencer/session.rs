use super::*;

impl ChordboardView {

    pub(super) fn set_step_slider(&self, lane: usize, page: usize, step: usize, field: Field, y: f32) {
        let rect = step_rect(lane,
            step - first_step(),
        );
        let (min, max) = slider_range(field);
        let value = min + (1.0 - (y - rect.1) / rect.3).clamp(0.0, 1.0) * (max - min);
        self.params.sequencer.edit(|state| {
            set_slider_value(field, value, &mut state.lanes[lane][page], step);
        });
    }

    pub(in crate::ui) fn sequencer_route_targets(&self) -> Vec<(usize, Rect)> {
        if !self.sequencer_open() || self.sequencer_ui.progress < 1.0 || self.sequencer_editing() {
            return Vec::new();
        }
        VISIBLE_LANES
            .into_iter()
            .filter_map(|lane| {
                let id = ["seq_page_0", "seq_page_1", "seq_page_2", "seq_page_3"][lane];
                crate::engine::routing::TARGETS
                    .iter()
                    .position(|target| target.id == id)
                    .map(|target| (target, page_row_rect(lane)))
            })
            .collect()
    }

    pub(in crate::ui) fn sequencer_open(&self) -> bool {
        self.sequencer_ui.open
    }

    pub(super) fn arp_ready(&self) -> bool {
        self.arp_main() && self.page_elapsed >= pleasant_ui::page_slide::DURATION
    }

    pub(in crate::ui) fn sequencer_editing(&self) -> bool {
        self.sequencer_ui.edit.is_some()
    }

    pub(in crate::ui) fn tick_sequencer(&mut self, dt: f32) -> bool {
        let u = &self.sequencer_ui;
        let pointer = self.hover_pointer().filter(|&(x, y)| {
            u.open
                && u.progress == 1.0
                && VISIBLE_LANES.into_iter().any(|lane| {
                    (0..visible_steps())
                        .any(|visible| hit(step_rect(lane, visible), x, y))
                })
        });
        // Avoid cloning pattern data every frame when the pointer is outside the cells.
        let state = pointer.map(|_| self.params.sequencer.snapshot());
        let u = &mut self.sequencer_ui;
        let mut redraw = false;
        if u.elapsed < pleasant_ui::page_slide::DURATION {
            u.elapsed = (u.elapsed + dt).min(pleasant_ui::page_slide::DURATION);
            u.progress = pleasant_ui::page_slide::position(
                u.start,
                if u.open { 1.0 } else { 0.0 },
                u.elapsed,
            );
            redraw = true;
        }
        for lane in VISIBLE_LANES {
            if u.page_elapsed[lane] < pleasant_ui::page_slide::DURATION {
                u.page_elapsed[lane] =
                    (u.page_elapsed[lane] + dt).min(pleasant_ui::page_slide::DURATION);
                u.page_positions[lane] = pleasant_ui::page_slide::position(
                    u.page_starts[lane],
                    u.edit_pages[lane] as f32,
                    u.page_elapsed[lane],
                );
                redraw = true;
            } else {
                u.page_positions[lane] = u.edit_pages[lane] as f32;
            }
            let first = first_step();
            let hovered = pointer
                .filter(|_| {
                    u.open
                        && u.progress == 1.0
                        && u.page_elapsed[lane] >= pleasant_ui::page_slide::DURATION
                })
                .and_then(|(x, y)| {
                    let state = state.as_ref()?;
                    (0..visible_steps()).find(|&visible| {
                        first + visible < state.lanes[lane][u.edit_pages[lane]].length as usize
                            && hit(step_rect(lane, visible), x, y)
                    })
                });
            for step in 0usize..32 {
                // Neighbors stay on the same visual row.
                let nearby = hovered.is_some_and(|visible| {
                    let center = first + visible;
                    step.abs_diff(center) <= 1
                });
                let age = &mut u.number_age[lane][step];
                if nearby {
                    redraw |= *age != 0.0;
                    *age = 0.0;
                } else if *age < NUMBER_HOLD + NUMBER_FADE {
                    *age = (*age + dt).min(NUMBER_HOLD + NUMBER_FADE);
                    redraw = true;
                }
            }
        }
        redraw
    }

    pub(in crate::ui) fn set_sequencer_open(&mut self, cx: &mut EventContext, open: bool) {
        if self.sequencer_ui.open == open {
            return;
        }
        self.release_keys();
        self.memory_held.fill(false);
        self.menu = None;
        self.edit = None;
        self.cancel_sequencer();
        if !open {
            self.clear_loop();
        }
        let u = &mut self.sequencer_ui;
        u.start = u.progress;
        u.elapsed = 0.0;
        u.open = open;
        if !open {
            u.inspector = false;
        }
        cx.needs_redraw();
    }

    pub(super) fn loop_range(&self) -> Option<(usize, usize)> {
        let u = &self.sequencer_ui;
        self.params
            .sequencer
            .audition()
            .filter(|a| a.lane == u.lane && a.page == u.edit_pages[u.lane])
            .map(|a| (a.start as usize, a.end as usize))
    }

    pub(super) fn set_loop(&mut self, a: usize, b: usize) {
        let u = &self.sequencer_ui;
        let (lane, page) = (u.lane, u.edit_pages[u.lane]);
        let last = self.params.sequencer.snapshot().lanes[lane][page]
            .length
            .saturating_sub(1) as usize;
        self.params
            .sequencer
            .set_audition(Some(crate::sequencer::Audition {
                lane,
                page,
                start: a.min(b).min(last) as u8,
                end: a.max(b).min(last) as u8,
            }));
    }

    pub(in crate::ui) fn clear_loop(&mut self) {
        self.sequencer_ui.loop_anchor = None;
        self.params.sequencer.set_audition(None);
    }

    pub(super) fn release_foreign_loop(&mut self) {
        let u = &self.sequencer_ui;
        if self
            .params
            .sequencer
            .audition()
            .is_some_and(|a| a.lane != u.lane || a.page != u.edit_pages[u.lane])
        {
            self.clear_loop();
        }
    }

    pub(in crate::ui) fn loop_strip_hit(&self, x: f32, y: f32) -> bool {
        self.sequencer_open()
            && self.sequencer_ui.progress == 1.0
            && (hit(loop_strip_rect(), x, y) || hit(loop_all_rect(), x, y))
    }

    pub(super) fn loop_step_at(&self, x: f32) -> usize {
        let u = &self.sequencer_ui;
        let length = self.params.sequencer.snapshot().lanes[u.lane][u.edit_pages[u.lane]].length;
        step_at(0, x, 0.0).min(length.saturating_sub(1) as usize)
    }

    pub(super) fn select_loop_step(&mut self, step: usize) {
        let u = &mut self.sequencer_ui;
        u.step = step;
        if u.lane == 0 {
            u.arp_step = step;
        }
        u.inspector = true;
    }

    pub(in crate::ui) fn cancel_sequencer(&mut self) {
        self.sequencer_ui.edit = None;
        self.sequencer_ui.box_drag = None;
        self.sequencer_ui.slider_drag = None;
        self.sequencer_ui.length_drag = None;
        self.sequencer_ui.loop_anchor = None;
    }

    pub(super) fn seq_commit(&mut self, cx: &mut EventContext) -> bool {
        let Some(edit) = self.sequencer_ui.edit.as_ref() else {
            return true;
        };
        let field = edit.target;
        let text = edit.text.clone();
        let u = &self.sequencer_ui;
        let (lane, page, index, tone) = (u.lane, u.edit_pages[u.lane], u.step, 0);
        let mut valid = false;
        self.params.sequencer.edit(|state| {
            valid = set_field(field, &text, &mut state.lanes[lane][page], index, tone);
        });
        if valid {
            self.sequencer_ui.edit = None;
            let state = self.params.sequencer.snapshot();
            let u = &mut self.sequencer_ui;
            u.step = u
                .step
                .min(state.lanes[u.lane][u.edit_pages[u.lane]].length as usize - 1);
            u.arp_step = u
                .arp_step
                .min(state.lanes[0][u.edit_pages[0]].length as usize - 1);
        } else if let Some(edit) = self.sequencer_ui.edit.as_mut() {
            edit.invalid = true;
        }
        cx.needs_redraw();
        valid
    }

    pub(super) fn set_sequencer_length(&mut self, lane: usize, x: f32) {
        let rect = length_rect(lane);
        let length = (((x - rect.0) / rect.2 * 32.0).ceil() as i32).clamp(1, 32) as u8;
        let pages = self.sequencer_ui.edit_pages;
        self.params.sequencer.edit(|state| {
            for target in VISIBLE_LANES {
                if !self.params.seq_length_lock.value() && target != lane {
                    continue;
                }
                let page = pages[target];
                let p = &mut state.lanes[target][page];
                p.length = length;
                p.end = length - 1;
                p.start = p.start.min(p.end);
            }
        });
        let state = self.params.sequencer.snapshot();
        let u = &mut self.sequencer_ui;
        u.step = u
            .step
            .min(state.lanes[u.lane][u.edit_pages[u.lane]].length as usize - 1);
        u.arp_step = u
            .arp_step
            .min(state.lanes[0][u.edit_pages[0]].length as usize - 1);
    }

    pub(in crate::ui) fn sequencer_event(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
    ) -> bool {
        if matches!(event, WindowEvent::FocusOut) {
            self.cancel_sequencer();
            cx.release();
            return false;
        }
        if let Some(anchor) = self.sequencer_ui.loop_anchor {
            match event {
                WindowEvent::MouseMove(_, _) => {
                    let step = self.loop_step_at(x);
                    self.set_loop(anchor, step);
                    self.select_loop_step(step);
                }
                WindowEvent::MouseUp(MouseButton::Left) | WindowEvent::KeyDown(Code::Escape, _) => {
                    self.sequencer_ui.loop_anchor = None;
                    cx.release();
                }
                _ => {}
            }
            cx.needs_redraw();
            return true;
        }
        if let Some(lane) = self.sequencer_ui.length_drag {
            match event {
                WindowEvent::MouseMove(_, _) => self.set_sequencer_length(lane, x),
                WindowEvent::MouseUp(MouseButton::Left) | WindowEvent::KeyDown(Code::Escape, _) => {
                    self.sequencer_ui.length_drag = None;
                    cx.release();
                }
                _ => {}
            }
            cx.needs_redraw();
            return true;
        }
        if let Some((lane, page, step, field)) = self.sequencer_ui.slider_drag {
            match event {
                WindowEvent::MouseMove(_, _) => {
                    let length = self.params.sequencer.snapshot().lanes[lane][page].length as usize;
                    let next = step_at(lane, x,
                        y,
                    )
                    .min(length - 1);
                    for index in step.min(next)..=step.max(next) {
                        self.set_step_slider(lane, page, index, field, y);
                    }
                    self.sequencer_ui.slider_drag = Some((lane, page, next, field));
                    self.sequencer_ui.step = next;
                    if lane == 0 {
                        self.sequencer_ui.arp_step = next;
                    }
                    cx.needs_redraw();
                }
                WindowEvent::MouseUp(MouseButton::Left) | WindowEvent::KeyDown(Code::Escape, _) => {
                    self.sequencer_ui.slider_drag = None;
                    cx.release();
                }
                _ => {}
            }
            return true;
        }
        if let Some((page, start, end)) = self.sequencer_ui.box_drag {
            match event {
                WindowEvent::MouseMove(_, _) => {
                    let length = self.params.sequencer.snapshot().lanes[3][page].length as usize;
                    let step = step_at(3, x, y)
                        .min(length - 1)
                        .max(start);
                    let pattern = self.params.sequencer.snapshot();
                    let spans = pattern.lanes[3][page].box_spans();
                    let step = ((start + 1)..=step)
                        .find(|&i| spans[i] > 0)
                        .map_or(step, |next| next - 1);
                    self.sequencer_ui.box_drag = Some((page, start, step));
                    cx.needs_redraw();
                    return true;
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    self.params
                        .sequencer
                        .edit(|state| state.lanes[3][page].draw_box(start, end));
                    self.sequencer_ui.box_drag = None;
                    self.sequencer_ui.inspector = true;
                    cx.release();
                    cx.needs_redraw();
                    return true;
                }
                WindowEvent::KeyDown(Code::Escape, _) => {
                    self.sequencer_ui.box_drag = None;
                    cx.release();
                    cx.needs_redraw();
                    return true;
                }
                _ => return true,
            }
        }
        if self.sequencer_ui.edit.is_some() {
            match event {
                WindowEvent::CharInput(c) => {
                    if !cx.modifiers().command() {
                        self.sequencer_ui
                            .edit
                            .as_mut()
                            .unwrap()
                            .insert(&c.to_string());
                    }
                    cx.needs_redraw();
                    return true;
                }
                WindowEvent::KeyDown(code, _) => {
                    if *code == Code::Escape {
                        self.sequencer_ui.edit = None;
                    } else if matches!(code, Code::Enter | Code::NumpadEnter) {
                        self.seq_commit(cx);
                    } else {
                        self.sequencer_ui
                            .edit
                            .as_mut()
                            .unwrap()
                            .handle_key(cx, *code);
                    }
                    cx.needs_redraw();
                    return true;
                }
                WindowEvent::KeyUp(_, _) => return true,
                WindowEvent::MouseDown(MouseButton::Left) if !self.seq_commit(cx) => {
                    return true;
                }
                _ => {}
            }
        }
        let press = matches!(
            event,
            WindowEvent::MouseDown(MouseButton::Left)
                | WindowEvent::MouseDoubleClick(MouseButton::Left)
                | WindowEvent::MouseTripleClick(MouseButton::Left)
        );
        if press && self.loop_strip_hit(x, y) && hit(loop_strip_rect(), x, y) {
            let step = self.loop_step_at(x);
            self.set_loop(step, step);
            self.select_loop_step(step);
            self.sequencer_ui.loop_anchor = Some(step);
            if self.panel.is_some() {
                self.set_panel(None);
            }
            cx.capture();
            cx.needs_redraw();
            return true;
        }
        if press && self.sequencer_open() {
            if let Some(lane) = VISIBLE_LANES
                .into_iter()
                .find(|&lane| hit(length_rect(lane), x, y))
            {
                self.set_sequencer_length(lane, x);
                self.sequencer_ui.length_drag = Some(lane);
                cx.capture();
                cx.needs_redraw();
                return true;
            }
        }
        if press {
            if let Some(c) = self
                .expression_mode_items()
                .into_iter()
                .find(|c| hit(c.rect, x, y))
            {
                if let Action::Modifier(field) = c.action {
                    let u = &mut self.sequencer_ui;
                    u.modifier = if u.modifier == field { None } else { field };
                    if !u.open {
                        u.start = u.progress;
                        u.elapsed = 0.0;
                        u.open = true;
                    }
                    cx.needs_redraw();
                    return true;
                }
            }
        }
        if !self.sequencer_open() {
            return false;
        }
        if matches!(event, WindowEvent::KeyDown(Code::Escape, _))
            && self.sequencer_ui.inspector
        {
            self.sequencer_ui.inspector = false;
            cx.needs_redraw();
            return true;
        }
        let u = &self.sequencer_ui;
        let arp_ready = self.arp_ready();
        let in_surface = hit(surface(), x, y) || (arp_ready && hit(ARP_PAD, x, y));
        if !in_surface {
            return false;
        }
        let edit_only = matches!(event, WindowEvent::MouseDown(MouseButton::Right));
        if let WindowEvent::MouseScroll(_, dy) = event {
            if arp_ready && hit(ARP_PAD, x, y) {
                return false;
            }
            if *dy != 0.0 && u.progress == 1.0 {
                let state = self.params.sequencer.snapshot();
                if let Some((lane, step)) = self.seq_items(&state).iter().find_map(|item| {
                    if hit(item.rect, x, y) {
                        if let Action::SelectStep(lane @ 0..=2, step) = item.action {
                            if u.page_elapsed[lane] >= pleasant_ui::page_slide::DURATION {
                                return Some((lane, step));
                            }
                        }
                    }
                    None
                }) {
                    let field = u.modifier.unwrap_or(Field::Velocity);
                    let page = u.edit_pages[lane];
                    let value =
                        slider_value(field, &state.lanes[lane][page].steps[step]) + dy.signum();
                    self.params.sequencer.edit(|state| {
                        set_slider_value(field, value, &mut state.lanes[lane][page], step)
                    });
                    cx.needs_redraw();
                }
            }
            return true;
        }
        if !press && !edit_only {
            return matches!(
                event,
                WindowEvent::MouseDown(_)
                    | WindowEvent::MouseUp(_)
                    | WindowEvent::MouseScroll(_, _)
            );
        }
        if u.progress < 1.0 && !hit(ARP_PAD, x, y) {
            return true;
        }
        let state = self.params.sequencer.snapshot();
        let arp = arp_ready && hit(ARP_PAD, x, y) && !hit(ARP_EDITOR, x, y);
        let items = if arp {
            self.arp_items(&state)
        } else {
            self.seq_items(&state)
        };
        let Some(c) = items.iter().rev().find(|c| hit(c.rect, x, y)) else {
            return !arp;
        };
        if matches!(c.action, Action::SelectStep(..)) {
            if self.panel.is_some() {
                self.set_panel(None);
            }
        }
        if let Action::SelectStep(lane, _) = c.action {
            if self.sequencer_ui.page_elapsed[lane] < pleasant_ui::page_slide::DURATION {
                return true;
            }
        }
        if edit_only && !matches!(c.action, Action::SelectStep(..)) {
            return true;
        }
        let u = &mut self.sequencer_ui;
        if arp {
            u.lane = 0;
            u.step = u.arp_step;
        }
        let (lane, page, index) = (u.lane, u.edit_pages[u.lane], u.step);
        match c.action {
            Action::Tab(tab) => u.tab = tab,
            Action::Modifier(field) => u.modifier = field,
            Action::EditPage(l, p) => {
                let param = &self.params.seq_pages[l].page;
                Self::emit(cx, param.as_ptr(), param.preview_normalized(p as i32));
                if u.edit_pages[l] != p {
                    u.page_starts[l] = u.page_positions[l];
                    u.page_elapsed[l] = 0.0;
                    u.number_age[l].fill(NUMBER_HOLD + NUMBER_FADE);
                    u.edit_pages[l] = p;
                }
                u.lane = l;
                u.inspector = false;
                if l == 0 {
                    u.arp_step = u
                        .arp_step
                        .min(state.lanes[0][p].length.saturating_sub(1) as usize);
                }
                u.step = if l == 0 {
                    u.arp_step
                } else {
                    u.step
                        .min(state.lanes[l][p].length.saturating_sub(1) as usize)
                };
            }
            Action::PlayPage(l) => {
                let p = &self.params.seq_pages[l].page;
                Self::emit(cx, p.as_ptr(), p.preview_normalized(u.edit_pages[l] as i32));
            }
            Action::SelectStep(3, s) => {
                let p = &state.lanes[3][u.edit_pages[3]];
                if let Some((start, len)) = p.box_at(s) {
                    u.step = start;
                    let last_visible =
                        (start + len - 1).saturating_sub(first_step());
                    let edge = step_rect(3, last_visible);
                    if !edit_only && hit(edge, x, y) && x >= edge.0 + edge.2 - 7.0 {
                        u.box_drag = Some((u.edit_pages[3], start, start + len - 1));
                        cx.capture();
                    }
                    u.inspector = true;
                } else if !edit_only {
                    u.step = s;
                    u.box_drag = Some((u.edit_pages[3], s, s));
                    cx.capture();
                }
                u.lane = 3;
                u.tab = 0;
            }
            Action::SelectStep(l, s) => {
                if let Some(field) = u.modifier.filter(|_| !edit_only) {
                    let page = u.edit_pages[l];
                    u.lane = l;
                    u.step = s;
                    u.inspector = true;
                    if l == 0 {
                        u.arp_step = s;
                    }
                    if cx.modifiers().command() {
                        let value = slider_value(field, &Step::default());
                        self.params.sequencer.edit(|state| {
                            set_slider_value(field, value, &mut state.lanes[l][page], s)
                        });
                        cx.needs_redraw();
                        return true;
                    }
                    u.slider_drag = Some((l, page, s, field));
                    self.set_step_slider(l, page, s, field, y);
                    cx.capture();
                    cx.needs_redraw();
                    return true;
                }
                if !edit_only {
                    self.params.sequencer.edit(|state| {
                        let step = &mut state.lanes[l][u.edit_pages[l]].steps[s];
                        step.enabled = !step.enabled;
                    });
                }
                u.lane = l;
                u.step = s;
                if l == 0 {
                    u.arp_step = s;
                }
                u.inspector = true;
            }
            Action::LengthLock => {
                let enabled = !self.params.seq_length_lock.value();
                Self::emit(
                    cx,
                    self.params.seq_length_lock.as_ptr(),
                    if enabled { 1.0 } else { 0.0 },
                );
                if enabled {
                    let length = state.lanes[u.lane][u.edit_pages[u.lane]].length;
                    self.params.sequencer.edit(|state| {
                        for lane in VISIBLE_LANES {
                            let p = &mut state.lanes[lane][u.edit_pages[lane]];
                            p.length = length;
                            p.end = length - 1;
                            p.start = p.start.min(p.end);
                        }
                    });
                }
            }
            Action::LoopAll => {
                u.loop_anchor = None;
                self.params.sequencer.set_audition(None);
            }
            Action::Choice(field) => {
                self.params.sequencer.edit(|state| {
                    let p = &mut state.lanes[lane][page];
                    match field {
                        Field::Interlock => {
                            p.interlock_override = match p.interlock_override {
                                None => Some(0),
                                Some(v) if v < 2 => Some(v + 1),
                                Some(_) => None,
                            }
                        }
                        Field::Quality => {
                            let s = &mut p.steps[index];
                            s.quality = if s.quality >= 11 { -1 } else { s.quality + 1 };
                        }
                        Field::Inversion => {
                            let s = &mut p.steps[index];
                            s.inversion = if s.inversion >= 5 {
                                -1
                            } else {
                                s.inversion + 1
                            };
                        }
                        Field::Spread => {
                            let s = &mut p.steps[index];
                            s.spread = if s.spread >= 4 { -1 } else { s.spread + 1 };
                        }
                        _ => {}
                    }
                });
            }
            Action::Field(field)
                if is_step_value(field)
                    && lane != 3
                    && !matches!(
                        event,
                        WindowEvent::MouseDoubleClick(_) | WindowEvent::MouseTripleClick(_)
                    ) =>
            {
                u.modifier = Some(field);
                if !u.open {
                    u.start = u.progress;
                    u.elapsed = 0.0;
                    u.open = true;
                }
            }
            Action::Field(field) => {
                let p = &state.lanes[lane][page];
                u.edit = Some(ValueEdit::new(
                    field,
                    c.rect,
                    field_value(field, p, &p.steps[index], u),
                ));
                self.release_keys();
                self.memory_held.fill(false);
                self.end_pad_hover(cx);
            }
            Action::LaneEnabled(l) => self.params.sequencer.edit(|state| {
                let p = &mut state.lanes[l][u.edit_pages[l]];
                p.enabled = !p.enabled;
            }),
            action => {
                self.params.sequencer.edit(|state| {
                    let p = &mut state.lanes[lane][page];
                    match action {
                        Action::Enabled => p.steps[index].enabled = !p.steps[index].enabled,
                        Action::DeleteBox => p.delete_box(index),
                        Action::Tie => p.steps[index].tie = !p.steps[index].tie,
                        Action::Ghosts => {
                            for i in p.start as usize..=p.end as usize {
                                let s = &mut p.steps[i];
                                if !s.enabled {
                                    s.enabled = true;
                                    s.velocity = if (i - p.start as usize).is_multiple_of(4) {
                                        65
                                    } else {
                                        38
                                    };
                                }
                            }
                        }
                        _ => {}
                    }
                });
            }
        }
        if matches!(c.action, Action::EditPage(..) | Action::SelectStep(..)) {
            self.release_foreign_loop();
        }
        cx.needs_redraw();
        true
    }
}
