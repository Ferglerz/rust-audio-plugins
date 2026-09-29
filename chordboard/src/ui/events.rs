use super::*;
impl ChordboardView {
    fn press_expanded_strum(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        if hit(self.expand_button(), x, y) {
            self.end_pad_hover(cx);
            self.toggle_expand();
            return true;
        }
        if self.params.mode.value() == 1 {
            if hit(strum_sync_rect(self.expand_t()), x, y) {
                self.set(
                    cx,
                    "strum_sync",
                    if self.params.strum_sync.value() {
                        0.0
                    } else {
                        1.0
                    },
                );
                return true;
            }
            if self.params.strum_sync.value() && hit(strum_rate_rect(self.expand_t()), x, y) {
                self.open_menu(Menu::StrumRate);
                return true;
            }
        }
        if self.params.mode.value() == 2 && hit(strum_latch_rect(self.expand_t()), x, y) {
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
        if self.params.mode.value() == 2 {
            if let Some((y_axis, right)) = self.strum_bound_at(x, y) {
                self.end_pad_hover(cx);
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
    fn press_control(&mut self, cx: &mut EventContext, x: f32, y: f32) {
        for (c, r) in self.placed_controls() {
            if !hit(r, x, y) {
                continue;
            }
            if c.id == "quality" {
                self.open_menu(Menu::Quality);
            } else if c.id == "spread" {
                Self::emit(
                    cx,
                    c.ptr,
                    self.params
                        .spread
                        .preview_normalized((self.params.spread.value() + 1) % 3),
                );
            } else if c.toggle {
                Self::emit(cx, c.ptr, if c.norm >= 0.5 { 0.0 } else { 1.0 });
            } else {
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
        if matches!(
            event,
            WindowEvent::MouseDoubleClick(MouseButton::Left)
                | WindowEvent::MouseTripleClick(MouseButton::Left)
        ) {
            return self.window_event(cx, &WindowEvent::MouseDown(MouseButton::Left));
        }
        if matches!(event, WindowEvent::FocusOut) {
            self.focused = false;
            self.memory_held.fill(false);
            self.pointer = None;
            cx.needs_redraw();
            self.end_pad_hover(cx);
            self.end_drag(cx);
            self.release_keys();
            self.edit = None;
            self.set_panel(None);
            return false;
        }
        if matches!(event, WindowEvent::MouseLeave) {
            self.pointer = None;
            self.end_pad_hover(cx);
            cx.needs_redraw();
        } else if matches!(
            event,
            WindowEvent::MouseMove(_, _) | WindowEvent::MouseDown(_)
        ) {
            self.pointer = Some((x, y));
            if matches!(event, WindowEvent::MouseDown(MouseButton::Left)) {
                self.flash_press(x, y);
            }
            cx.needs_redraw();
        }
        // Escape belongs to the host; it is not a Chordboard shortcut.
        if matches!(
            event,
            WindowEvent::KeyDown(Code::Escape, _) | WindowEvent::KeyUp(Code::Escape, _)
        ) {
            return false;
        }
        if self.edit.is_some() {
            match event {
                WindowEvent::CharInput(c) => {
                    if !cx.modifiers().command() && c.is_ascii() && !c.is_control() {
                        if let Some(edit) = self.edit.as_mut() {
                            edit.insert(&c.to_string());
                        }
                    }
                    return true;
                }
                WindowEvent::KeyDown(code, _) => {
                    match code {
                        Code::Enter | Code::NumpadEnter => self.commit_edit(cx),
                        _ => {
                            if let Some(edit) = self.edit.as_mut() {
                                edit.handle_key(cx, *code);
                            }
                        }
                    }
                    return true;
                }
                WindowEvent::KeyUp(_, _) => return true,
                WindowEvent::MouseDown(MouseButton::Left) => {
                    self.commit_edit(cx);
                    if self.edit.is_some() {
                        return true;
                    }
                }
                _ => {}
            }
        }
        if let Some(menu) = self.menu {
            match event {
                WindowEvent::KeyDown(code, _) => {
                    match code {
                        Code::Enter | Code::NumpadEnter => {
                            self.select_menu(cx, menu, self.menu_cursor);
                        }
                        Code::ArrowRight => {
                            self.menu_cursor = (self.menu_cursor + 1) % menu.items().len();
                        }
                        Code::ArrowLeft => {
                            self.menu_cursor =
                                (self.menu_cursor + menu.items().len() - 1) % menu.items().len();
                        }
                        Code::ArrowDown => {
                            self.menu_cursor =
                                (self.menu_cursor + menu.columns()) % menu.items().len();
                        }
                        Code::ArrowUp => {
                            self.menu_cursor = (self.menu_cursor + menu.items().len()
                                - menu.columns())
                                % menu.items().len();
                        }
                        _ => {}
                    }
                    return true;
                }
                WindowEvent::KeyUp(_, _) => return true,
                WindowEvent::MouseDown(MouseButton::Left) => {
                    if hit(self.menu_close_rect(menu), x, y) {
                        self.menu = None;
                        return true;
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
                    if hit(self.menu_trigger_rect(menu), x, y) {
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
        if self.panel.is_none()
            && self.page_elapsed < pleasant_ui::page_slide::DURATION
            && hit(
                (
                    self.pad().0,
                    self.pad().1,
                    self.pad().2,
                    self.pad().3.max(360.0),
                ),
                x,
                y,
            )
            && matches!(
                event,
                WindowEvent::MouseDown(_) | WindowEvent::MouseScroll(_, _)
            )
        {
            return true;
        }
        match event {
            WindowEvent::KeyDown(code, _)
                if self.params.keyboard.value() && self.focused && !cx.modifiers().command() =>
            {
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
                if self.strum_bound_at(x, y).is_some()
                    || !(self.params.mode.value() == 2 && hit(self.play_pad(), x, y))
                {
                    self.end_pad_hover(cx);
                }
                if self.expand_t() > 0.0 && y >= HEADER_H {
                    return self.press_expanded_strum(cx, x, y);
                }
                if self.can_expand_strum() && hit(self.expand_button(), x, y) {
                    self.toggle_expand();
                    return true;
                }
                if hit(MPE, x, y) {
                    self.open_menu(Menu::Protocol);
                    return true;
                }
                for (r, panel) in [(OUTPUT, Panel::Output)] {
                    if hit(r, x, y) {
                        self.set_panel(if self.panel == Some(panel) {
                            None
                        } else {
                            Some(panel)
                        });
                        return true;
                    }
                }
                for axis in 0..2 {
                    if hit(mapping_summary_rect(axis), x, y) {
                        let close = self.panel == Some(Panel::Mapping) && self.mapping_axis == axis;
                        self.set_panel(if close { None } else { Some(Panel::Mapping) });
                        self.mapping_axis = axis;
                        return true;
                    }
                }
                if let Some(panel) = self.panel {
                    if hit(self.panel_close_rect(panel), x, y) {
                        self.set_panel(None);
                        return true;
                    }
                    if !hit(self.panel_rect(panel), x, y) {
                        self.set_panel(None);
                    } else {
                        if panel == Panel::Mapping {
                            if self.mapping_axis == 1
                                && hit(self.menu_trigger_rect(Menu::YTarget), x, y)
                            {
                                self.open_menu(Menu::YTarget);
                                return true;
                            }
                            if hit(mapping_learn_rect(self.mapping_axis), x, y) {
                                self.request_learn(
                                    if self.learning() == self.mapping_axis as u8 + 1 {
                                        0
                                    } else {
                                        self.mapping_axis as u8 + 1
                                    },
                                );
                                return true;
                            }
                            for menu in [
                                Menu::MappingKind,
                                Menu::MappingChannel(self.mapping().kind == 3),
                                Menu::MappingCc(self.mapping().kind == 2),
                            ] {
                                if matches!(menu, Menu::MappingChannel(_))
                                    && self.mapping_axis < 2
                                    && self.mapping().kind != 3
                                {
                                    continue;
                                }
                                if hit(self.menu_trigger_rect(menu), x, y) {
                                    if !matches!(menu, Menu::MappingCc(_))
                                        || matches!(self.mapping().kind, 1 | 2)
                                    {
                                        self.open_menu(menu);
                                    }
                                    return true;
                                }
                            }
                        }
                        self.press_control(cx, x, y);
                        return true;
                    }
                }
                if hit(TEMPO_SYNC, x, y) {
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
                if self.params.mode.value() == 1 {
                    if hit(strum_sync_rect(self.expand_t()), x, y) {
                        self.set(
                            cx,
                            "strum_sync",
                            if self.params.strum_sync.value() {
                                0.0
                            } else {
                                1.0
                            },
                        );
                        return true;
                    }
                    if self.params.strum_sync.value() && hit(strum_rate_rect(self.expand_t()), x, y)
                    {
                        self.open_menu(Menu::StrumRate);
                        return true;
                    }
                }
                if hit(SAVE_MEMORY, x, y) {
                    self.toggle_memory_save();
                    return true;
                }
                for i in 0..3 {
                    if self.params.mode.value() == 1 && hit(direction_rect(i), x, y) {
                        Self::emit(
                            cx,
                            self.params.direction.as_ptr(),
                            self.params.direction.preview_normalized(i as i32),
                        );
                        return true;
                    }
                }
                if self.params.mode.value() == 2 && hit(strum_latch_rect(self.expand_t()), x, y) {
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
                if hit(LEARN_OCTAVE, x, y) {
                    self.request_learn(if self.learning() == 4 { 0 } else { 4 });
                    return true;
                }
                for (i, step) in TRANSPOSE_STEPS.iter().enumerate() {
                    if hit(transpose_rect(i), x, y) {
                        let value = if *step == 0 {
                            0
                        } else {
                            (self.params.transpose.value() + step).clamp(-24, 24)
                        };
                        Self::emit(
                            cx,
                            self.params.transpose.as_ptr(),
                            self.params.transpose.preview_normalized(value),
                        );
                        return true;
                    }
                }
                if hit(APPEARANCE, x, y) {
                    prefs().toggle();
                    return true;
                }
                if hit(QWERTY, x, y) {
                    self.set(
                        cx,
                        "keyboard",
                        if self.params.keyboard.value() {
                            0.0
                        } else {
                            1.0
                        },
                    );
                    if self.params.keyboard.value() {
                        self.release_keys();
                    }
                    return true;
                }
                for i in 0..3 {
                    if hit(mode_rect(i as usize), x, y) {
                        Self::emit(
                            cx,
                            self.params.mode.as_ptr(),
                            self.params.mode.preview_normalized(i + 1),
                        );
                        self.end_drag(cx);
                        self.set_panel(None);
                        return true;
                    }
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
                    self.set(
                        cx,
                        "fifths",
                        if self.params.fifths.value() { 0.0 } else { 1.0 },
                    );
                    return true;
                }
                for i in 0..KEY_COUNT {
                    if hit(key_rect(i), x, y) {
                        if let Some(root) = self.keyboard_root(i % KEY_COLUMNS) {
                            self.bridge.send(Command::KeyDown(
                                i as u8 + POINTER_KEY_OFFSET,
                                (self.params.keyboard_octave.value() + root as i32).min(127) as u8,
                                harmony::row_quality(i / KEY_COLUMNS),
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
                if hit(Menu::Scale.trigger_rect(), x, y) {
                    self.open_menu(Menu::Scale);
                    return true;
                }
                if self.memory_press(cx, x, y) {
                    return true;
                }
                if self.params.mode.value() == 2 {
                    if let Some((y_axis, right)) = self.strum_bound_at(x, y) {
                        self.end_pad_hover(cx);
                        self.drag = Some(Drag::StrumBound(y_axis, right));
                        cx.capture();
                        cx.emit(RawParamEvent::BeginSetParameter(
                            self.bound_param(y_axis, right).as_ptr(),
                        ));
                        return true;
                    }
                }
                if self.params.mode.value() == 2 && hit(self.play_pad(), x, y) {
                    self.begin_pad(cx, x, y, true);
                    return true;
                }
                if hit(inversion_rect(0), x, y) {
                    self.inversion(cx, -1);
                    return true;
                }
                if hit(inversion_rect(1), x, y) {
                    self.inversion(cx, 1);
                    return true;
                }
                if self.arp_main() {
                    if hit(RATE_HEADER, x, y) {
                        if let Some(c) = self.control("rate") {
                            let r = (
                                RATE_HEADER.0 + RATE_HEADER.2 - 176.0,
                                RATE_HEADER.1,
                                176.0,
                                RATE_HEADER.3,
                            );
                            self.drag = Some(Drag::Control(ControlDrag {
                                press: pleasant_ui::pointer::ValuePress::new(c.id, r, (x, y)),
                                ptr: c.ptr,
                                norm: c.norm,
                                editable: hit(r, x, y),
                                started: false,
                            }));
                            cx.capture();
                        }
                        return true;
                    }
                    for i in 0..5 {
                        if hit(pattern_rect(i), x, y) {
                            Self::emit(
                                cx,
                                self.params.arp_pattern.as_ptr(),
                                self.params.arp_pattern.preview_normalized(i as i32),
                            );
                            return true;
                        }
                    }
                    for (i, (_, beats)) in ARP_RATES.iter().enumerate() {
                        if hit(rate_rect(i), x, y) {
                            Self::emit(
                                cx,
                                self.params.rate.as_ptr(),
                                self.params.rate.preview_normalized(*beats),
                            );
                            return true;
                        }
                    }
                    for i in 0..4 {
                        if hit(octave_rect(i), x, y) {
                            Self::emit(
                                cx,
                                self.params.octaves.as_ptr(),
                                self.params.octaves.preview_normalized(i as i32 + 1),
                            );
                            return true;
                        }
                    }
                }
                self.press_control(cx, x, y);
                return true;
            }
            WindowEvent::MouseMove(_, _) => match self.drag {
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
                        if c.id == "quality" {
                            return true;
                        }
                        Self::emit(cx, c.ptr, if *dy > 0.0 { c.next } else { c.prev });
                        return true;
                    }
                }
            }
            _ => {}
        }
        false
    }
}
