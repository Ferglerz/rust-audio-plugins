use super::*;
impl ChordboardView {
    pub(super) fn restore_shift_latch(&mut self, cx: &mut EventContext) {
        if let Some((base, _, _)) = self.latch_shift.take() {
            Self::emit(cx, self.params.latch.as_ptr(), if base { 1.0 } else { 0.0 });
        }
        self.shift_keys.fill(false);
    }
    pub(super) fn shift_latch_event(&mut self, cx: &mut EventContext, event: &WindowEvent) -> bool {
        let (code, down) = match event {
            WindowEvent::KeyDown(code, _) => (*code, true),
            WindowEvent::KeyUp(code, _) => (*code, false),
            _ => return false,
        };
        let index = match code {
            Code::ShiftLeft => 0,
            Code::ShiftRight => 1,
            _ => {
                if down {
                    if let Some((_, _, used)) = self.latch_shift.as_mut() {
                        *used = true;
                    }
                }
                return false;
            }
        };
        if down {
            if !self.focused
                || self.edit.is_some()
                || self.arp_editor_editing()
                || self.arp_chooser_open()
                || self.fine_adjustment_active()
                || self.pointer.is_some_and(|(x, y)| self.fine_adjustment_at(x, y))
                || self.menu.is_some()
                || cx.modifiers().command()
            {
                return false;
            }
            if !self.shift_keys[index] {
                self.shift_keys[index] = true;
                if self.latch_shift.is_none() {
                    let base = self.params.latch.value();
                    self.latch_shift = Some((base, Instant::now(), false));
                    Self::emit(cx, self.params.latch.as_ptr(), if base { 0.0 } else { 1.0 });
                }
            }
            true
        } else if self.shift_keys[index] {
            self.shift_keys[index] = false;
            if !self.shift_keys.iter().any(|&held| held) {
                if let Some((base, pressed, used)) = self.latch_shift.take() {
                    if used || pressed.elapsed().as_secs_f32() >= 0.25 {
                        Self::emit(cx, self.params.latch.as_ptr(), if base { 1.0 } else { 0.0 });
                    }
                }
            }
            true
        } else {
            false
        }
    }
    pub(super) fn fine_adjustment_active(&self) -> bool {
        matches!(self.drag, Some(Drag::Control(_) | Drag::StrumBound(..)
            | Drag::RouteContour { .. } | Drag::RouteNode(..)
            | Drag::Split | Drag::BassSplit))
            || self.arp_editor_dragging()
    }
    fn fine_adjustment_at(&self, x: f32, y: f32) -> bool {
        self.placed_controls().iter().any(|(c, r)| !c.toggle && hit(*r, x, y))
            || self.strum_bound_at(x, y).is_some()
            || (self.panel == Some(Panel::Routes) && hit(ROUTE_GRAPH, x, y))
            || self.arp_editor_value_at(x, y)
    }
    fn press_expanded_strum(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        if hit(strum_bypass_rect(self.expand_t()), x, y) {
            self.toggle_strum_bypass(cx);
            return true;
        }
        if self.press_octaves(cx, x, y) {
            return true;
        }
        if hit(self.expand_button(), x, y) {
            self.end_pad_hover(cx);
            self.toggle_expand();
            return true;
        }
        if hit(strum_latch_rect(self.expand_t()), x, y) {
            self.set(
                cx,
                "strum_latch",
                if self.params.strum_latch.value() {
                    0.0
                } else {
                    1.0
                },
            );
            return true;
        }
        if self.manual_playing() {
            if let Some((y_axis, right)) = self.strum_bound_at(x, y) {
                self.end_pad_hover(cx);
                self.restore_shift_latch(cx);
                self.drag = Some(Drag::StrumBound(y_axis, right));
                cx.capture();
                cx.emit(RawParamEvent::BeginSetParameter(
                    self.bound_param(y_axis, right).as_ptr(),
                ));
                return true;
            }
            if hit(self.play_pad(), x, y) {
                self.begin_pad(cx, x, y, true);
                return true;
            }
        }
        true
    }
    fn press_octaves(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        let Some(i) = (0..4).find(|&i| hit(octave_button_rect(self.expand_t(), i), x, y)) else {
            return false;
        };
        if !self.explain_modulation("octaves") {
            Self::emit(
                cx,
                self.params.octaves.as_ptr(),
                self.params.octaves.preview_normalized(i as i32 + 1),
            );
        }
        true
    }
    pub(super) fn press_control(&mut self, cx: &mut EventContext, x: f32, y: f32) {
        for (c, r) in self.placed_controls() {
            if !hit(r, x, y) {
                continue;
            }
            if self.explain_modulation(c.id) {
                return;
            }
            if c.id == "root_on_select" {
                self.cycle_selection_mode(cx);
            } else if c.id == "voice_leading" {
                Self::emit(
                    cx,
                    c.ptr,
                    self.params
                        .voice_leading
                        .preview_normalized((self.params.voice_leading.value() + 1) % 3),
                );
            } else if c.id == "spread" {
                Self::emit(
                    cx,
                    c.ptr,
                    self.params.spread.preview_normalized(
                        (self.params.spread.value() + 1) % harmony::VOICING_NAMES.len() as i32,
                    ),
                );
            } else if c.toggle {
                Self::emit(cx, c.ptr, if c.norm >= 0.5 { 0.0 } else { 1.0 });
            } else {
                self.restore_shift_latch(cx);
                let value = pleasant_ui::slider_value_rect(r);
                self.drag = Some(Drag::Control(ControlDrag {
                    press: pleasant_ui::pointer::ValuePress::new(c.id, value, (x, y)),
                    ptr: c.ptr,
                    norm: c.norm,
                    editable: hit(value, x, y),
                    started: false,
                }));
                cx.capture();
            }
            return;
        }
    }
    pub(super) fn window_event(&mut self, cx: &mut EventContext, event: &WindowEvent) -> bool {
        if matches!(event, WindowEvent::FocusOut) {
            self.bridge.quick_learn.store(0, Ordering::Release);
            self.restore_shift_latch(cx);
            self.cancel_arp_editor();
            cx.release();
            self.pending_route_remove = None;
            self.focused = false;
            self.memory_held.fill(false);
            self.pointer = None;
            self.route_hover = None;
            cx.needs_redraw();
            self.end_pad_hover(cx);
            self.end_drag(cx);
            self.release_keys();
            self.edit = None;
            self.set_panel(None);
            return false;
        }
        let bounds = cx.bounds();
        let Some((x, y)) = pleasant_ui::local_xy(
            bounds.x,
            bounds.y,
            bounds.w,
            W,
            cx.mouse().cursorx,
            cx.mouse().cursory,
        ) else {
            return false;
        };
        // Vizia substitutes double/triple clicks for mouse-down. Every press
        // uses the same drag threshold and opens text only on value release.
        if self.edit.is_none() && matches!(
            event,
            WindowEvent::MouseDoubleClick(MouseButton::Left)
                | WindowEvent::MouseTripleClick(MouseButton::Left)
        ) {
            if self.menu.is_none()
                && self.edit.is_none()
                && self.drag.is_none()
                && self.arp_editor_event(cx, event, x, y)
            {
                return true;
            }
            return self.window_event(cx, &WindowEvent::MouseDown(MouseButton::Left));
        }
        // A modifier pressed before a value gesture belongs to that gesture.
        if self.fine_adjustment_active() && self.latch_shift.is_some() {
            self.restore_shift_latch(cx);
        }
        if self.shift_latch_event(cx, event) {
            return true;
        }
        if matches!(event, WindowEvent::MouseLeave) {
            if self.selected_modulator.is_none() {
                self.pending_route_remove = None;
            }
            self.pointer = None;
            self.route_hover = None;
            self.end_pad_hover(cx);
            cx.needs_redraw();
        } else if matches!(
            event,
            WindowEvent::MouseMove(_, _) | WindowEvent::MouseDown(_)
        ) {
            self.pointer = Some((x, y));
            self.update_route_hover(x, y);
            if matches!(event, WindowEvent::MouseDown(MouseButton::Left)) {
                self.flash_press(x, y);
            }
            cx.needs_redraw();
        }
        if self.menu.is_none()
            && self.edit.is_none()
            && self.drag.is_none()
            && self.arp_editor_event(cx, event, x, y)
        {
            return true;
        }
        // The ordinary value editor has priority over panel dismissal.
        if self.edit.is_some() && matches!(event, WindowEvent::KeyDown(Code::Escape, _)) {
            self.edit = None;
            cx.needs_redraw();
            return true;
        }
        // Dismiss the active interaction before leaving Escape to the host.
        if matches!(
            event,
            WindowEvent::KeyDown(Code::Escape, _) | WindowEvent::KeyUp(Code::Escape, _)
        ) {
            if matches!(event, WindowEvent::KeyDown(Code::Escape, _))
                && self.bridge.quick_learn.swap(0, Ordering::AcqRel) != 0
            {
                cx.needs_redraw();
                return true;
            }
            if matches!(event, WindowEvent::KeyDown(Code::Escape, _))
                && self.pending_route_remove.take().is_some()
            {
                cx.needs_redraw();
                return true;
            }
            if matches!(self.drag, Some(Drag::Route(_) | Drag::RouteNode(..))) {
                self.end_drag(cx);
                cx.needs_redraw();
                return true;
            }
            if self.menu.is_some() && matches!(event, WindowEvent::KeyDown(Code::Escape, _)) {
                self.menu = None;
                cx.needs_redraw();
                return true;
            }
            if self.panel.is_some() && matches!(event, WindowEvent::KeyDown(Code::Escape, _)) {
                self.set_panel(None);
                cx.needs_redraw();
                return true;
            }
            return false;
        }
        if let Some(Drag::Route(mut drag)) = self.drag {
            match event {
                WindowEvent::MouseMove(_, _) => {
                    drag.update(x, y);
                    if drag.active && self.panel == Some(Panel::Routes) {
                        self.set_panel(None);
                    }
                    self.drag = Some(Drag::Route(drag));
                    return true;
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    drag.update(x, y);
                    if drag.active && self.panel == Some(Panel::Routes) {
                        self.set_panel(None);
                    }
                    self.end_drag(cx);
                    if drag.active {
                        self.finish_route_drag(cx, drag.source, x, y);
                    } else {
                        self.toggle_modulator(drag.source);
                    }
                    cx.needs_redraw();
                    return true;
                }
                WindowEvent::MouseScroll(_, _) | WindowEvent::MouseDown(_) => return true,
                _ => {}
            }
        }
        if let Some(edit) = self.edit.as_mut() {
            use pleasant_ui::readout::{self, EditAction};
            match readout::handle_edit(edit, cx, event, (x, y)) {
                EditAction::Blocked | EditAction::Handled => return true,
                EditAction::Cancel => {
                    self.edit = None;
                    return true;
                }
                EditAction::Commit | EditAction::CommitAndContinue => {
                    self.commit_edit(cx);
                    // Invalid input stays open. A successful outside click may
                    // continue to its control; focus loss is cancelled above.
                    if self.edit.is_some()
                        || !matches!(event, WindowEvent::MouseDown(MouseButton::Left))
                    {
                        return true;
                    }
                }
            }
        }
        if let Some(menu) = self.menu {
            match event {
                WindowEvent::KeyDown(code, _) => {
                    match code {
                        Code::Enter | Code::NumpadEnter => {
                            self.select_menu(cx, menu, self.menu_cursor);
                        }
                        Code::ArrowRight | Code::ArrowLeft | Code::ArrowDown | Code::ArrowUp => {
                            self.menu_cursor = self.menu_neighbor(menu, self.menu_cursor, *code);
                            cx.needs_redraw();
                        }
                        _ => {}
                    }
                    return true;
                }
                WindowEvent::KeyUp(_, _) => return true,
                WindowEvent::MouseDown(MouseButton::Left) => {
                    if matches!(menu, Menu::Key | Menu::Scale) {
                        if let Some(index) = self.root_choice_at(x, y) {
                            self.select_menu(cx, Menu::Key, index);
                            return true;
                        }
                        if hit(menu::root_scale_rect(self.menu_bounds(menu)), x, y) {
                            self.open_menu(if menu == Menu::Scale { Menu::Key } else { Menu::Scale });
                            return true;
                        }
                    }
                    for index in 0..menu.items().len() {
                        if hit(self.menu_option_rect(menu, index), x, y) {
                            self.select_menu(cx, menu, index);
                            return true;
                        }
                    }
                    if hit(self.menu_bounds(menu), x, y) {
                        return true;
                    }
                    self.menu = None;
                    if hit(self.menu_trigger_rect(menu), x, y)
                        || (matches!(menu, Menu::Key | Menu::Scale) && hit(KEY_MENU, x, y)) {
                        return true;
                    }
                }
                WindowEvent::MouseDoubleClick(_) | WindowEvent::MouseScroll(_, _) => return true,
                _ => {}
            }
        }
        if self.panel.is_some()
            && matches!(
                event,
                WindowEvent::KeyDown(_, _) | WindowEvent::CharInput(_)
            )
        {
            return true;
        }
        if self.route_elapsed < pleasant_ui::page_slide::DURATION
            && hit(CHORDS_SURFACE, x, y)
            && matches!(
                event,
                WindowEvent::MouseDown(_) | WindowEvent::MouseScroll(_, _)
            )
        {
            return true;
        }
        match event {
            WindowEvent::KeyDown(code, _) if self.focused && !cx.modifiers().command() => {
                if let Some(slot) = MEMORY_CODES.iter().position(|c| c == code) {
                    if !self.memory_held[slot] {
                        self.memory_held[slot] = true;
                        if cx.modifiers().shift() || self.memory_ui.armed {
                            self.request_capture(slot);
                        } else {
                            self.recall_memory(cx, slot);
                        }
                    }
                    return true;
                }
                if let Some(key) = CODES.iter().position(|c| c == code) {
                    self.play_key(key);
                    return true;
                }
                let index = match code {
                    Code::ArrowUp => Some(0),
                    Code::ArrowDown => Some(1),
                    _ => None,
                };
                if let Some(index) = index {
                    if !self.special[index] {
                        self.special[index] = true;
                        match index {
                            0 => self.inversion(cx, 1),
                            1 => self.inversion(cx, -1),
                            _ => unreachable!(),
                        }
                    }
                    return true;
                }
            }
            WindowEvent::KeyUp(code, _) => {
                if let Some(slot) = MEMORY_CODES.iter().position(|c| c == code) {
                    let held = self.memory_held[slot];
                    self.memory_held[slot] = false;
                    return held;
                }
                if let Some(key) = CODES.iter().position(|c| c == code) {
                    if self.pressed[key] {
                        self.pressed[key] = false;
                        self.bridge.send(Command::KeyUp(key as u8));
                        return true;
                    }
                }
                let index = match code {
                    Code::ArrowUp => Some(0),
                    Code::ArrowDown => Some(1),
                    _ => None,
                };
                if let Some(i) = index {
                    let held = self.special[i];
                    self.special[i] = false;
                    return held;
                }
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                cx.focus();
                self.focused = true;
                if self.press_quick_midi(cx, x, y, false) { return true; }
                if self.strum_bound_at(x, y).is_some()
                    || !(self.manual_playing() && hit(self.play_pad(), x, y))
                {
                    self.end_pad_hover(cx);
                }
                if self.press_route_overlay(cx, x, y) {
                    return true;
                }
                if let Some(slot) = self.route_chip_at(x, y) {
                    self.route_slot = slot;
                    self.set_panel(Some(Panel::Routes));
                    self.selected_modulator = self.params.routes[slot]
                        .route()
                        .source
                        .checked_sub(1)
                        .map(|s| s as usize);
                    self.route_hover = None;
                    return true;
                }
                if hit(PIANO_SURFACE, x, y)
                    && self.panel.is_none_or(|p| !hit(self.panel_rect(p), x, y))
                {
                    return self.press_piano(cx, x, y);
                }
                if self.expand_t() > 0.0 && y >= HEADER_H && y < ARP_SURFACE.1 {
                    return self.press_expanded_strum(cx, x, y);
                }
                if hit(self.expand_button(), x, y) {
                    self.toggle_expand();
                    return true;
                }
                if let Some(source) = (0..crate::engine::routing::SOURCE_COUNT).find(|&i| {
                    hit(meter_rect(i), x, y)
                        && self.panel.is_none_or(|p| !hit(self.panel_rect(p), x, y))
                }) {
                    if hit(modulator_bypass_rect(source), x, y) {
                        let bypass = &self.params.modulators[source].bypass;
                        Self::emit(cx, bypass.as_ptr(), if bypass.value() { 0.0 } else { 1.0 });
                        return true;
                    }
                    self.end_drag(cx);
                    self.drag = Some(Drag::Route(RouteDrag::new(source, x, y)));
                    cx.capture();
                    return true;
                }
                if self.panel == Some(Panel::Routes) && hit(self.panel_rect(Panel::Routes), x, y) {
                    if hit(self.panel_close_rect(Panel::Routes), x, y) {
                        self.set_panel(None);
                        return true;
                    }
                    if self.press_routes(cx, x, y) {
                        return true;
                    }
                    self.press_control(cx, x, y);
                    return true;
                }

                if let Some(id) = self
                    .route_targets()
                    .into_iter()
                    .find(|(_, r)| hit(*r, x, y))
                    .map(|(target, _)| crate::engine::routing::TARGETS[target].id)
                {
                    if id != "x" && self.explain_modulation(id) {
                        return true;
                    }
                }
                if (0..TRANSPOSE_STEPS.len())
                        .any(|i| hit(transpose_control_rect(self.params.mpe_enabled(), i), x, y))
                {
                    return self.press_piano(cx, x, y);
                }
                if !(self.params.tempo_sync.value() && self.snapshot.time_sig_host)
                    && (hit(METER_NUM, x, y) || hit(METER_DEN, x, y)) {
                    self.open_menu(Menu::Meter(hit(METER_DEN, x, y)));
                    return true;
                }
                if (pleasant_ui::tempo::Header { source: TEMPO_SYNC, bpm: TEMPO_CONTROL }).hit(x, y) == Some(pleasant_ui::tempo::Hit::Source) {
                    self.set(
                        cx,
                        "tempo_sync",
                        if self.params.tempo_sync.value() {
                            0.0
                        } else {
                            1.0
                        },
                    );
                    return true;
                }
                if hit(strum_latch_rect(self.expand_t()), x, y) {
                    self.set(
                        cx,
                        "strum_latch",
                        if self.params.strum_latch.value() {
                            0.0
                        } else {
                            1.0
                        },
                    );
                    return true;
                }
                if self.press_octaves(cx, x, y) {
                    return true;
                }
                if hit(APPEARANCE, x, y) {
                    prefs().toggle();
                    return true;
                }
                if hit(strum_bypass_rect(self.expand_t()), x, y) {
                    self.toggle_strum_bypass(cx);
                    return true;
                }
                if hit(LATCH, x, y) {
                    self.set(
                        cx,
                        "latch",
                        if self.params.latch.value() { 0.0 } else { 1.0 },
                    );
                    return true;
                }
                if hit(ORDER, x, y) {
                    self.set_keyboard_layout(cx, (self.keyboard_layout() + 1) % 4);
                    return true;
                }
                for i in 0..KEY_COUNT {
                    if self.keyboard_chord(i).is_some()
                        && hit(self.keyboard_key_rect(i), x, y)
                    {
                        if let Some(chord) = self.keyboard_chord(i) {
                            self.bridge.send(Command::KeyDown(
                                i as u8 + POINTER_KEY_OFFSET,
                                (self.params.keyboard_octave.value() + chord.root as i32) as u8,
                                chord.quality,
                            ));
                        }
                        self.drag = Some(Drag::Key(i as u8 + POINTER_KEY_OFFSET));
                        cx.capture();
                        return true;
                    }
                }
                if hit(Menu::Key.trigger_rect(), x, y) {
                    self.open_menu(Menu::Key);
                    return true;
                }
                if self.memory_press(cx, x, y) {
                    return true;
                }
                if self.manual_playing() {
                    if let Some((y_axis, right)) = self.strum_bound_at(x, y) {
                        self.end_pad_hover(cx);
                        self.restore_shift_latch(cx);
                        self.drag = Some(Drag::StrumBound(y_axis, right));
                        cx.capture();
                        cx.emit(RawParamEvent::BeginSetParameter(
                            self.bound_param(y_axis, right).as_ptr(),
                        ));
                        return true;
                    }
                }
                if self.manual_playing() && hit(self.play_pad(), x, y) {
                    self.begin_pad(cx, x, y, true);
                    return true;
                }
                if hit(inversion_control_rect(self.params.mpe_enabled(), 0), x, y) {
                    self.inversion(cx, -1);
                    return true;
                }
                if hit(inversion_control_rect(self.params.mpe_enabled(), 1), x, y) {
                    self.inversion(cx, 1);
                    return true;
                }
                if hit(arp_direction_rect(), x, y) {
                    if !self.explain_modulation("arp_pattern") {
                        self.open_menu(Menu::ArpPattern);
                    }
                    return true;
                }
                self.press_control(cx, x, y);
                return true;
            }
            WindowEvent::MouseDown(MouseButton::Right) => {
                if self.press_quick_midi(cx, x, y, true) { return true; }
                if hit(PIANO_SURFACE, x, y)
                    && self.panel.is_none_or(|p| !hit(self.panel_rect(p), x, y))
                {
                    return self.press_piano_right(cx, x, y);
                }
            }
            WindowEvent::MouseMove(_, _) => match self.drag {
                Some(Drag::RouteContour {
                    origin,
                    min,
                    max,
                    slot,
                    mut axis,
                }) => {
                    let (min, max) = Self::route_contour_range(
                        origin,
                        (x, y),
                        min,
                        max,
                        cx.modifiers().shift(),
                        &mut axis,
                    );
                    cx.emit(RawParamEvent::SetParameterNormalized(
                        self.params.routes[slot].min.as_ptr(),
                        min,
                    ));
                    cx.emit(RawParamEvent::SetParameterNormalized(
                        self.params.routes[slot].max.as_ptr(),
                        max,
                    ));
                    if let Some(Drag::RouteContour { axis: saved, .. }) = self.drag.as_mut() {
                        *saved = axis;
                    }
                    return true;
                }
                Some(Drag::BassSplit) => {
                    self.move_bass_split(cx, x, y);
                    return true;
                }
                Some(Drag::Split) => {
                    self.move_split(cx, x, y);
                    return true;
                }
                Some(Drag::RouteNode(node, ptr)) => {
                    if let Some(value) = self.route_node_value(node, y) {
                        cx.emit(RawParamEvent::SetParameterNormalized(ptr, value));
                    }
                    return true;
                }
                Some(Drag::Memory(mut drag)) => {
                    drag.update(x, y);
                    self.drag = Some(Drag::Memory(drag));
                    return true;
                }
                Some(Drag::StrumBound(y_axis, right)) => {
                    let play = self.play_pad();
                    let (min, max, span, value) = if y_axis {
                        let (min, max) =
                            expression_bounds(self.params.y_min.value(), self.params.y_max.value());
                        (min, max, 0.01, 1.0 - (y - play.1) / play.3)
                    } else {
                        let (min, max) =
                            strum_bounds(self.params.x_min.value(), self.params.x_max.value());
                        (min, max, STRUM_MIN_SPAN, (x - play.0) / play.2)
                    };
                    let value = if right {
                        value.clamp(min + span, 1.0)
                    } else {
                        value.clamp(0.0, max - span)
                    };
                    let param = self.bound_param(y_axis, right);
                    cx.emit(RawParamEvent::SetParameterNormalized(
                        param.as_ptr(),
                        param.preview_normalized(value),
                    ));
                    return true;
                }
                Some(Drag::Pad) => {
                    self.set_pad_xy(cx, x, y);
                    return true;
                }
                Some(Drag::Control(mut drag)) => {
                    if drag.press.update(x, y) {
                        if !drag.started {
                            cx.emit(RawParamEvent::BeginSetParameter(drag.ptr));
                            drag.started = true;
                        }
                        let delta = pleasant_ui::pointer::readout_drag_delta(
                            x - drag.press.origin.0,
                            y - drag.press.origin.1,
                            cx.modifiers().shift(),
                        );
                        cx.emit(RawParamEvent::SetParameterNormalized(
                            drag.ptr,
                            (drag.norm + delta).clamp(0.0, 1.0),
                        ));
                    }
                    self.drag = Some(Drag::Control(drag));
                    return true;
                }
                _ => {
                    self.sync_pad_hover(cx);
                    if self.pad_hover {
                        self.set_pad_xy(cx, x, y);
                        return true;
                    }
                }
            },
            WindowEvent::MouseUp(MouseButton::Left) => {
                if let Some(Drag::Control(mut drag)) = self.drag {
                    let edit = drag.editable && drag.press.released_as_click(x, y);
                    self.end_drag(cx);
                    if edit {
                        self.release_keys();
                        self.memory_held.fill(false);
                        if let Some(c) = self.control(drag.press.target) {
                            self.edit = Some(ValueEdit::new(
                                c.id,
                                drag.press.rect,
                                self.display_value(&c),
                            ));
                        }
                    }
                    return true;
                }
                if let Some(Drag::Memory(drag)) = self.drag {
                    self.end_drag(cx);
                    self.finish_memory_drag(cx, drag, x, y);
                    return true;
                }
                if self.drag.is_some() {
                    self.end_drag(cx);
                    return true;
                }
            }
            WindowEvent::MouseScroll(_, dy) => {
                for (c, r) in self.placed_controls() {
                    if hit(r, x, y) {
                        if self.explain_modulation(c.id) {
                            return true;
                        }
                        if c.id == "quality" {
                            return true;
                        }
                        if c.id == "root_on_select" {
                            self.step_selection_mode(cx, if *dy > 0.0 { 1 } else { -1 });
                        } else {
                            Self::emit(cx, c.ptr, if *dy > 0.0 { c.next } else { c.prev });
                        }
                        return true;
                    }
                }
            }
            _ => {}
        }
        false
    }
}
