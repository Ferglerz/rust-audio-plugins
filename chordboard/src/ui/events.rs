use super::*;
impl ChordboardView {
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
        if matches!(event, WindowEvent::FocusOut) {
            self.focused = false;
            self.pointer = None;
            cx.needs_redraw();
            self.end_drag(cx);
            self.release_keys();
            self.edit = None;
            self.menu = None;
            return false;
        }
        if matches!(event, WindowEvent::MouseLeave) {
            self.pointer = None;
            cx.needs_redraw();
        } else if matches!(
            event,
            WindowEvent::MouseMove(_, _) | WindowEvent::MouseDown(_)
        ) {
            self.pointer = Some((x, y));
            cx.needs_redraw();
        }
        if matches!(event, WindowEvent::KeyDown(Code::Escape, _))
            && matches!(self.drag, Some(Drag::Memory(_)))
        {
            self.end_drag(cx);
            self.status = "Memory drag cancelled".into();
            return true;
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
                        Code::Escape => self.edit = None,
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
                        Code::Escape => self.menu = None,
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
                        if hit(menu.option_rect(index), x, y) {
                            self.select_menu(cx, menu, index);
                            return true;
                        }
                    }
                    self.menu = None;
                    if hit(menu.trigger_rect(), x, y) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        match event {
            WindowEvent::KeyDown(code, _)
                if self.params.keyboard.value() && self.focused && !cx.modifiers().command() =>
            {
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
                if *code == Code::Escape {
                    self.bridge.panic.store(true, Ordering::Release);
                    self.release_keys();
                    return true;
                }
            }
            WindowEvent::KeyUp(code, _) => {
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
                if hit((936.0, 24.0, 70.0, 30.0), x, y) {
                    prefs().toggle();
                    return true;
                }
                if hit((814.0, 24.0, 112.0, 30.0), x, y) {
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
                for i in 0..4 {
                    if hit(mode_rect(i as usize), x, y) {
                        Self::emit(
                            cx,
                            self.params.mode.as_ptr(),
                            self.params.mode.preview_normalized(i),
                        );
                        self.end_drag(cx);
                        self.set_setup(false);
                        self.group = if i == 1 || i == 2 { 1 } else { 0 };
                        self.page = if i == 1 || i == 2 { 1 } else { 0 };
                        return true;
                    }
                }
                if self.group == 6 && hit(LEARN_OCTAVE, x, y) {
                    self.bridge
                        .send(Command::Learn(if self.snapshot.learning == 4 {
                            0
                        } else {
                            4
                        }));
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
                if hit((32.0, 190.0, 154.0, 28.0), x, y) {
                    self.set(
                        cx,
                        "fifths",
                        if self.params.fifths.value() { 0.0 } else { 1.0 },
                    );
                    return true;
                }
                for i in 0..KEY_COUNT {
                    if hit(key_rect(i), x, y) {
                        if let Some(root) =
                            harmony::keyboard_root(self.params.fifths.value(), i % KEY_COLUMNS)
                        {
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
                if hit((434.0, 441.0, 130.0, 28.0), x, y) {
                    self.set(
                        cx,
                        "highlight",
                        if self.params.highlight.value() {
                            0.0
                        } else {
                            1.0
                        },
                    );
                    return true;
                }
                if self.memory_press(cx, x, y) {
                    return true;
                }
                if !self.arp_main() && hit(PLAY_PAD, x, y) {
                    self.drag = Some(Drag::Pad);
                    cx.capture();
                    let px = ((x - PLAY_PAD.0) / PLAY_PAD.2).clamp(0.0, 1.0);
                    let py = (1.0 - (y - PLAY_PAD.1) / PLAY_PAD.3).clamp(0.0, 1.0);
                    for ptr in [self.params.x.as_ptr(), self.params.y.as_ptr()] {
                        cx.emit(RawParamEvent::BeginSetParameter(ptr));
                    }
                    cx.emit(RawParamEvent::SetParameterNormalized(
                        self.params.x.as_ptr(),
                        px,
                    ));
                    cx.emit(RawParamEvent::SetParameterNormalized(
                        self.params.y.as_ptr(),
                        py,
                    ));
                    self.bridge.send(Command::BeginGesture(px, py));
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
                for axis in 0..3 {
                    if hit(mapping_summary_rect(axis), x, y) {
                        if self.mapping_axis != axis {
                            self.bridge.send(Command::Learn(0));
                        }
                        self.set_setup(true);
                        self.mapping_axis = axis;
                        return true;
                    }
                }
                if self.group == 6 {
                    for axis in 0..3 {
                        if hit(axis_rect(axis), x, y) {
                            if self.mapping_axis != axis
                                && (1..=3).contains(&self.snapshot.learning)
                            {
                                self.bridge.send(Command::Learn(0));
                            }
                            self.mapping_axis = axis;
                            return true;
                        }
                    }
                    if hit(LEARN, x, y) {
                        self.bridge.send(Command::Learn(
                            if self.snapshot.learning == self.mapping_axis as u8 + 1 {
                                0
                            } else {
                                self.mapping_axis as u8 + 1
                            },
                        ));
                        return true;
                    }
                    for menu in [
                        Menu::MappingKind,
                        Menu::MappingChannel(self.mapping().kind == 3),
                        Menu::MappingCc(self.mapping().kind == 2),
                    ] {
                        if hit(menu.trigger_rect(), x, y) {
                            if matches!(menu, Menu::MappingCc(_))
                                && !matches!(self.mapping().kind, 1 | 2)
                            {
                                return true;
                            }
                            self.open_menu(menu);
                            return true;
                        }
                    }
                }
                if hit(SETUP, x, y) {
                    self.set_setup(!self.setup_open);
                    return true;
                }
                for (i, &(group, _)) in self.groups().iter().enumerate() {
                    if hit(group_rect(i), x, y) {
                        if self.group == 6 {
                            self.bridge.send(Command::Learn(0));
                        }
                        self.group = group;
                        self.page = 0;
                        return true;
                    }
                }
                if self.pages() > 1 && hit(PAGE, x, y) {
                    self.page = (self.page + 1) % self.pages();
                    return true;
                }
                if self.arp_visible() {
                    for i in 0..5 {
                        if hit(pattern_rect(i, self.arp_main()), x, y) {
                            Self::emit(
                                cx,
                                self.params.arp_pattern.as_ptr(),
                                self.params.arp_pattern.preview_normalized(i as i32),
                            );
                            return true;
                        }
                    }
                    for (i, (_, beats)) in ARP_RATES.iter().enumerate() {
                        if hit(rate_rect(i, self.arp_main()), x, y) {
                            Self::emit(
                                cx,
                                self.params.rate.as_ptr(),
                                self.params.rate.preview_normalized(*beats),
                            );
                            return true;
                        }
                    }
                    for i in 0..4 {
                        if hit(octave_rect(i, self.arp_main()), x, y) {
                            Self::emit(
                                cx,
                                self.params.octaves.as_ptr(),
                                self.params.octaves.preview_normalized(i as i32 + 1),
                            );
                            return true;
                        }
                    }
                }
                for (c, r) in self.placed_controls() {
                    if hit(r, x, y) {
                        if c.toggle {
                            Self::emit(cx, c.ptr, if c.norm >= 0.5 { 0.0 } else { 1.0 });
                            return true;
                        }
                        // Leave value labels stable for double-click text entry.
                        if y < r.1 + 22.0 {
                            return true;
                        }
                        self.drag = Some(Drag::Control(c.ptr, r));
                        cx.capture();
                        cx.emit(RawParamEvent::BeginSetParameter(c.ptr));
                        cx.emit(RawParamEvent::SetParameterNormalized(
                            c.ptr,
                            ((x - r.0 - 12.0) / (r.2 - 24.0)).clamp(0.0, 1.0),
                        ));
                        return true;
                    }
                }
                return true;
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                self.end_drag(cx);
                if self.arp_visible() && hit(rate_header(self.arp_main()), x, y) {
                    self.release_keys();
                    let r = rate_header(self.arp_main());
                    self.edit = Some(ValueEdit::new(
                        "rate",
                        (r.0 + r.2 - 176.0, r.1, 176.0, 24.0),
                        format!("{}", self.params.rate.value()),
                    ));
                    return true;
                }
                for (c, r) in self.placed_controls() {
                    if hit(r, x, y) {
                        if c.toggle {
                            return true;
                        }
                        self.release_keys();
                        self.edit = Some(ValueEdit::new(
                            c.id,
                            (r.0 + 8.0, r.1 + 3.0, r.2 - 16.0, 24.0),
                            c.value.clone(),
                        ));
                        return true;
                    }
                }
            }
            WindowEvent::MouseMove(_, _) => match self.drag {
                Some(Drag::Memory(mut drag)) => {
                    drag.update(x, y);
                    self.drag = Some(Drag::Memory(drag));
                    return true;
                }
                Some(Drag::Pad) => {
                    cx.emit(RawParamEvent::SetParameterNormalized(
                        self.params.x.as_ptr(),
                        ((x - PLAY_PAD.0) / PLAY_PAD.2).clamp(0.0, 1.0),
                    ));
                    cx.emit(RawParamEvent::SetParameterNormalized(
                        self.params.y.as_ptr(),
                        (1.0 - (y - PLAY_PAD.1) / PLAY_PAD.3).clamp(0.0, 1.0),
                    ));
                    return true;
                }
                Some(Drag::Control(ptr, r)) => {
                    cx.emit(RawParamEvent::SetParameterNormalized(
                        ptr,
                        ((x - r.0 - 12.0) / (r.2 - 24.0)).clamp(0.0, 1.0),
                    ));
                    return true;
                }
                _ => {}
            },
            WindowEvent::MouseUp(MouseButton::Left) => {
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
