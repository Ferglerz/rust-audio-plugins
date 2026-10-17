use super::*;

impl ChordboardView {

    /// Relative drag: the full range takes `SLIDER_FRICTION` cell heights of travel.
    pub(super) fn drag_step_slider(&self, drag: &mut SliderDrag, y: f32, fine: bool) {
        let (min, max) = slider_range(drag.field);
        let scale = (max - min) / (STEP_H * SLIDER_FRICTION) * if fine { SLIDER_FINE } else { 1.0 };
        drag.value = (drag.value - (y - drag.last_y) * scale).clamp(min, max);
        drag.last_y = y;
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
                    .map(|target| (target, page_block_rect(lane)))
            })
            .chain(self.sequencer_rate_targets().into_iter().filter_map(|(id, r)| {
                crate::engine::routing::TARGETS
                    .iter()
                    .position(|target| target.available() && target.id == id)
                    .map(|target| (target, r))
            }))
            .collect()
    }

    fn sequencer_rate_targets(&self) -> Vec<(&'static str, Rect)> {
        vec![
            ("strum_sync", seq_rate_sync_rect()),
            (if self.sweep_synced() { "rate" } else { "strum_ms" }, seq_rate_rect()),
        ]
    }

    /// Arp rate controls in the sequencer header. Free mode uses the shared control drag.
    fn press_sequencer_rate(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        if hit(seq_rate_sync_rect(), x, y) {
            if !self.explain_modulation("strum_sync") {
                self.set(
                    cx,
                    "strum_sync",
                    if self.params.strum_sync.value() { 0.0 } else { 1.0 },
                );
            }
            return true;
        }
        if !hit(seq_rate_rect(), x, y) {
            return false;
        }
        if self.sweep_synced() {
            if !self.explain_modulation("rate") {
                self.open_menu(Menu::ArpRate);
            }
        } else {
            self.press_control(cx, x, y);
        }
        true
    }

    pub(in crate::ui) fn sequencer_hint(&self, x: f32, y: f32) -> Option<String> {
        if !self.sequencer_open() || self.sequencer_ui.progress < 1.0 {
            return None;
        }
        if self.panel == Some(Panel::Mapping) && hit(self.panel_rect(Panel::Mapping), x, y) {
            return None;
        }
        if hit(seq_power_rect(), x, y) {
            return Some("Enable or bypass sequencer playback".into());
        }
        if VISIBLE_LANES.into_iter().any(|lane| hit(length_rect(lane), x, y)) {
            return Some("Pattern length · Drag to choose 1–32 steps · Length lock applies the same length to both lanes".into());
        }
        if self.sequencer_open() {
            let state = self.params.sequencer.snapshot();
            if let Some(item) = self
                .seq_items(&state)
                .into_iter()
                .find(|item| hit(item.rect, x, y))
            {
                match item.action {
                    Action::PageRate => return Some("Chords page rate · Click to cycle note divisions · Independent of the Arp rate".into()),
                    Action::SelectStep(_, _) => return Some(if self.sequencer_ui.modifier.is_some() {
                        "Drag up or down to set the value · Drag across to paint · Shift for fine · Ctrl/Cmd-drag resets cells".into()
                    } else {
                        "Click to toggle hit/rest · Right-click to select a step for editing".into()
                    }),
                    Action::EditPage(_, _) => return Some("Pattern page · Click to edit and play this lane’s page".into()),
                    Action::LaneEnabled(_) => return Some("Click to enable or mute this sequencer lane".into()),
                    Action::LengthLock => return Some("Length lock · Keep both lanes at the same number of steps".into()),
                    Action::Modifier(None) => return Some("Hits/rests · Click cells to toggle steps".into()),
                    Action::Modifier(Some(_)) => return Some(format!("{} · Select this value, then drag cells vertically or paint across", item.label)),
                    Action::Field(_) => return Some(format!("{} · Click to select a cell value · Double-click to type", item.label)),
                    Action::Enabled => return Some("Click to toggle the selected step’s hit/rest".into()),
                }
            }
        }
        if hit(seq_rate_sync_rect(), x, y) {
            return Some("Arp rate: use the header tempo in beat divisions, or run free in milliseconds".into());
        }
        if hit(seq_rate_rect(), x, y) {
            return Some(if self.sweep_synced() {
                "Arp step rate · Click to choose a note value".into()
            } else {
                "Free arp rate · Drag anywhere to adjust · Click value to type · Shift for fine adjustment".into()
            });
        }
        None
    }

    pub(in crate::ui) fn sequencer_open(&self) -> bool {
        self.sequencer_ui.open
    }

    pub(in crate::ui) fn sequencer_value_at(&self, x: f32, y: f32) -> bool {
        self.sequencer_open() && VISIBLE_LANES.into_iter().any(|lane| {
            hit(length_rect(lane), x, y)
                || (self.sequencer_ui.modifier.is_some()
                    && (0..visible_steps()).any(|step| hit(step_rect(lane, step), x, y)))
        })
    }

    pub(in crate::ui) fn sequencer_dragging(&self) -> bool {
        self.sequencer_ui.slider_drag.is_some() || self.sequencer_ui.length_drag.is_some()
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

    #[allow(dead_code)] // Tests still close the module. The window no longer has a hide control.
    pub(in crate::ui) fn set_sequencer_open(&mut self, cx: &mut EventContext, open: bool) {
        if self.sequencer_ui.open == open {
            return;
        }
        self.release_keys();
        self.memory_held.fill(false);
        self.pending_route_remove = None;
        self.menu = None;
        self.edit = None;
        self.cancel_sequencer();
        cx.release();
        let u = &mut self.sequencer_ui;
        u.start = u.progress;
        u.elapsed = 0.0;
        u.open = open;
        if !open {
            u.inspector = false;
        }
        cx.needs_redraw();
    }

    pub(in crate::ui) fn cancel_sequencer(&mut self) {
        self.sequencer_ui.edit = None;
        self.sequencer_ui.slider_drag = None;
        self.sequencer_ui.length_drag = None;
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

    pub(super) fn inherit_sequencer_length(&self, lane: usize, page: usize) {
        if !self.params.seq_length_lock.value() { return; }
        let mut pages = self.sequencer_ui.edit_pages;
        self.params.sequencer.edit(|state| {
            let length = VISIBLE_LANES.into_iter()
                .map(|l| state.lanes[l][pages[l]].length)
                .chain(std::iter::once(state.lanes[lane][page].length))
                .max().unwrap_or(1);
            pages[lane] = page;
            for l in VISIBLE_LANES {
                let p = &mut state.lanes[l][pages[l]];
                p.length = length;
                p.end = length - 1;
                p.start = p.start.min(p.end);
            }
        });
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
        if let Some(mut drag) = self.sequencer_ui.slider_drag {
            match event {
                WindowEvent::MouseMove(_, _) => {
                    let (lane, page, step, field) = (drag.lane, drag.page, drag.step, drag.field);
                    let length = self.params.sequencer.snapshot().lanes[lane][page].length as usize;
                    let next = step_at(lane, x, y).min(length - 1);
                    if !drag.reset {
                        self.drag_step_slider(&mut drag, y, cx.modifiers().shift());
                    }
                    let value = drag.value;
                    self.params.sequencer.edit(|state| {
                        for index in step.min(next)..=step.max(next) {
                            set_slider_value(field, value, &mut state.lanes[lane][page], index);
                        }
                    });
                    drag.step = next;
                    self.sequencer_ui.slider_drag = Some(drag);
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
        // Controller popovers are painted over the lanes in the taller layout.
        if self.panel == Some(Panel::Mapping) && hit(self.panel_rect(Panel::Mapping), x, y) {
            return false;
        }
        let press = matches!(
            event,
            WindowEvent::MouseDown(MouseButton::Left)
                | WindowEvent::MouseDoubleClick(MouseButton::Left)
                | WindowEvent::MouseTripleClick(MouseButton::Left)
        );
        let in_settings = x >= SEQ_SURFACE.0
            && x < SEQ_SURFACE.0 + SEQ_SURFACE.2
            && y >= SEQ_SURFACE.1
            && y < SEQ_SURFACE.1 + SEQ_SETTINGS_H;
        if in_settings {
            if press
                && self.sequencer_open()
                && self.sequencer_ui.progress == 1.0
                && self.press_sequencer_rate(cx, x, y)
            {
                cx.focus();
                self.focused = true;
                cx.needs_redraw();
                return true;
            }
            if press || matches!(event, WindowEvent::MouseDown(MouseButton::Right)) {
                return false;
            }
        }
        if press && self.sequencer_open() {
            if let Some(lane) = VISIBLE_LANES
                .into_iter()
                .find(|&lane| hit(length_rect(lane), x, y))
            {
                cx.focus();
                self.focused = true;
                self.restore_shift_latch(cx);
                self.set_sequencer_length(lane, x);
                self.sequencer_ui.length_drag = Some(lane);
                cx.capture();
                cx.needs_redraw();
                return true;
            }
        }
        if !self.sequencer_open() {
            return false;
        }
        let u = &self.sequencer_ui;
        let step_controls = self.arp_main()
            && self.page_position == 1.0
            && self.route_progress == 0.0
            && self.expand_t() == 0.0
            && self.seq_inspector_items(&self.params.sequencer.snapshot(), 0, inspector_rect(), true)
                .iter().any(|item| hit(item.rect, x, y));
        if self.route_chip_at(x, y).is_some() || (!hit(surface(), x, y) && !step_controls) {
            return false;
        }
        let edit_only = matches!(event, WindowEvent::MouseDown(MouseButton::Right));
        if let WindowEvent::MouseScroll(_, dy) = event {
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
        cx.focus();
        self.focused = true;
        if self.sequencer_ui.progress < 1.0 {
            return true;
        }
        let state = self.params.sequencer.snapshot();
        let items = self.seq_items(&state);
        let Some(c) = items.iter().rev().find(|c| hit(c.rect, x, y)) else {
            return true;
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
        if let Action::EditPage(lane, page) = c.action {
            self.inherit_sequencer_length(lane, page);
        }
        if matches!(c.action, Action::SelectStep(..)) && self.sequencer_ui.modifier.is_some() && !edit_only {
            self.restore_shift_latch(cx);
        }
        let u = &mut self.sequencer_ui;
        let (lane, page, index) = (u.lane, u.edit_pages[u.lane], u.step);
        match c.action {
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
                u.inspector = true;
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
            Action::SelectStep(l, s) => {
                if let Some(field) = u.modifier.filter(|_| !edit_only) {
                    let page = u.edit_pages[l];
                    u.lane = l;
                    u.step = s;
                    u.inspector = true;
                    if l == 0 {
                        u.arp_step = s;
                    }
                    let reset = cx.modifiers().intersects(Modifiers::CTRL | Modifiers::LOGO);
                    if reset {
                        let value = slider_value(field, &Step::default());
                        self.params.sequencer.edit(|state| {
                            set_slider_value(field, value, &mut state.lanes[l][page], s)
                        });
                    }
                    u.slider_drag = Some(SliderDrag {
                        lane: l,
                        page,
                        step: s,
                        field,
                        last_y: y,
                        value: if reset { slider_value(field, &Step::default()) } else { slider_value(field, &state.lanes[l][page].steps[s]) },
                        reset,
                    });
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
                    let length = VISIBLE_LANES.into_iter().map(|lane| state.lanes[lane][u.edit_pages[lane]].length).max().unwrap_or(1);
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
            Action::Field(field)
                if is_step_value(field)
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
            Action::Enabled => self.params.sequencer.edit(|state| {
                let p = &mut state.lanes[lane][page];
                p.steps[index].enabled = !p.steps[index].enabled;
            }),
            Action::PageRate => self.params.sequencer.edit(|state| {
                let p = &mut state.lanes[1][u.edit_pages[1]];
                let next = ARP_RATES
                    .iter()
                    .position(|(_, beats)| (*beats - p.rate).abs() < 0.0001)
                    .map_or(0, |index| (index + 1) % ARP_RATES.len());
                p.rate = ARP_RATES[next].1;
            }),
        }
        cx.needs_redraw();
        true
    }
}
