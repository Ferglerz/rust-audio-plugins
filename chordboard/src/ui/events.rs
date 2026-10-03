use super::*;
impl ChordboardView {
    fn press_expanded_strum(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        if self.can_expand_strum() && hit(self.expand_button(), x, y) {
            self.end_pad_hover(cx);
            self.toggle_expand();
            return true;
        }
        if self.mode() == 2 && hit(strum_latch_rect(self.expand_t()), x, y) {
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
        if self.mode() == 2 {
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
                if c.id == "length_ms"
                    && self.playback_mode() == 1
                    && self.params.strum_hold.value()
                {
                    return;
                }
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
            self.route_hover = None;
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
        // Dismiss the active interaction before leaving Escape to the host.
        if matches!(
            event,
            WindowEvent::KeyDown(Code::Escape, _) | WindowEvent::KeyUp(Code::Escape, _)
        ) {
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
        if self.route_elapsed < pleasant_ui::page_slide::DURATION
            && hit(CHORDS_SURFACE, x, y)
            && matches!(
                event,
                WindowEvent::MouseDown(_) | WindowEvent::MouseScroll(_, _)
            )
        {
            return true;
        }
        if self.page_elapsed < pleasant_ui::page_slide::DURATION
            && hit(
                (self.pad().0, self.pad().1, self.pad().2, self.pad().3),
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
                if self.strum_bound_at(x, y).is_some()
                    || !(self.mode() == 2 && hit(self.play_pad(), x, y))
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
                    if self.panel == Some(Panel::Mapping) {
                        self.set_panel(None);
                    }
                    return self.press_piano(cx, x, y);
                }
                if self.expand_t() > 0.0 && y >= HEADER_H {
                    return self.press_expanded_strum(cx, x, y);
                }
                if self.can_expand_strum() && hit(self.expand_button(), x, y) {
                    self.toggle_expand();
                    return true;
                }
                if let Some(source) = (0..crate::engine::routing::SOURCE_COUNT).find(|&i| {
                    hit(meter_rect(i), x, y)
                        && self.panel.is_none_or(|p| !hit(self.panel_rect(p), x, y))
                }) {
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
                if let Some(panel) = self.panel {
                    if hit(self.panel_close_rect(panel), x, y) {
                        self.set_panel(None);
                        return true;
                    }
                    if !hit(self.panel_rect(panel), x, y) {
                        if panel == Panel::Mapping {
                            self.set_panel(None);
                        }
                    } else {
                        if panel == Panel::Mapping {
                            if self.mapping_axis == 1
                                && hit(self.menu_trigger_rect(Menu::YTarget), x, y)
                            {
                                self.open_menu(Menu::YTarget);
                                return true;
                            }
                            for menu in [
                                Menu::MappingKind,
                                Menu::MappingChannel(self.mapping().kind == 3),
                                Menu::MappingCc(false),
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
                if self.arp_main() && hit(RATE_SYNC, x, y) {
                    if !self.explain_modulation("strum_sync") {
                        self.set(
                            cx,
                            "strum_sync",
                            if self.params.strum_sync.value() {
                                0.0
                            } else {
                                1.0
                            },
                        );
                    }
                    return true;
                }
                if self.mode() == 1 && hit(STRUM_HOLD, x, y) {
                    if !self.explain_modulation("strum_hold") {
                        self.set(
                            cx,
                            "strum_hold",
                            if self.params.strum_hold.value() {
                                0.0
                            } else {
                                1.0
                            },
                        );
                    }
                    return true;
                }
                if self.mode() == 2 && hit(strum_latch_rect(self.expand_t()), x, y) {
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
                for (i, step) in TRANSPOSE_STEPS.iter().enumerate() {
                    if hit(transpose_control_rect(self.params.mpe_enabled(), i), x, y) {
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
                for i in 0..MODE_LABELS.len() {
                    if hit(mode_rect(i), x, y) {
                        Self::emit(
                            cx,
                            self.params.mode.as_ptr(),
                            self.params.mode.preview_normalized(if i == 0 {
                                if self.params.mode.value() == 3 {
                                    3
                                } else {
                                    1
                                }
                            } else {
                                2
                            }),
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
                    self.set_keyboard_layout(cx, (self.keyboard_layout() + 1) % 4);
                    return true;
                }
                for i in 0..KEY_COUNT {
                    if self.keyboard_chord(i).is_some() && hit(self.keyboard_key_rect(i), x, y) {
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
                if hit(Menu::Scale.trigger_rect(), x, y) {
                    self.open_menu(Menu::Scale);
                    return true;
                }
                if self.memory_press(cx, x, y) {
                    return true;
                }
                if self.mode() == 2 {
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
                if self.mode() == 2 && hit(self.play_pad(), x, y) {
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
                if self.arp_main() {
                    for looping in [false, true] {
                        if hit(repeat_rect(looping), x, y) {
                            Self::emit(
                                cx,
                                self.params.mode.as_ptr(),
                                self.params
                                    .mode
                                    .preview_normalized(if looping { 3 } else { 1 }),
                            );
                            return true;
                        }
                    }
                    if self.sweep_synced() && hit(RATE_HEADER, x, y) {
                        if self.explain_modulation("rate") {
                            return true;
                        }
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
                    for i in 0..7 {
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
                        if self.sweep_synced() && hit(rate_rect(i), x, y) {
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
