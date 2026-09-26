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
            self.end_drag(cx);
            self.release_keys();
            self.edit = None;
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
                    Code::Digit1 => Some(2),
                    Code::Digit2 => Some(3),
                    _ => None,
                };
                if let Some(index) = index {
                    if !self.special[index] {
                        self.special[index] = true;
                        match index {
                            0 => self.inversion(cx, 1),
                            1 => self.inversion(cx, -1),
                            2 => self.set(cx, "bank", 0.0),
                            _ => self.set(cx, "bank", 1.0),
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
                    Code::Digit1 => Some(2),
                    Code::Digit2 => Some(3),
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
                if hit((1016.0, 24.0, 72.0, 30.0), x, y) {
                    self.bridge.panic.store(true, Ordering::Release);
                    self.release_keys();
                    self.pulse.trigger_click();
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
                if hit((280.0, 24.0, 194.0, 30.0), x, y) {
                    self.preset = (self.preset + 1) % PRESETS.len();
                    return true;
                }
                if hit((482.0, 24.0, 62.0, 30.0), x, y) {
                    self.load_preset(cx);
                    return true;
                }
                if hit((552.0, 24.0, 62.0, 30.0), x, y) {
                    self.save_preset();
                    return true;
                }
                for i in 0..4 {
                    if hit((32.0 + i as f32 * 142.0, 91.0, 134.0, 34.0), x, y) {
                        Self::emit(
                            cx,
                            self.params.mode.as_ptr(),
                            self.params.mode.preview_normalized(i),
                        );
                        return true;
                    }
                }
                if hit((632.0, 91.0, 144.0, 34.0), x, y) {
                    self.bridge
                        .send(Command::Learn(if self.snapshot.learning == 4 {
                            0
                        } else {
                            4
                        }));
                    return true;
                }
                if hit((784.0, 91.0, 144.0, 34.0), x, y) {
                    self.set(
                        cx,
                        "latch",
                        if self.params.latch.value() { 0.0 } else { 1.0 },
                    );
                    return true;
                }
                if hit((936.0, 91.0, 152.0, 34.0), x, y) {
                    self.set(cx, "mpe", if self.params.mpe.value() { 0.0 } else { 1.0 });
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
                for i in 0..2 {
                    if hit((194.0 + i as f32 * 76.0, 190.0, 68.0, 28.0), x, y) {
                        self.set(cx, "bank", i as f32);
                        return true;
                    }
                }
                for i in 0..21 {
                    if hit(key_rect(i), x, y) {
                        if let Some(root) = harmony::keyboard_root(
                            self.params.fifths.value(),
                            self.params.bank.value() as u8,
                            i % 7,
                        ) {
                            self.bridge.send(Command::KeyDown(
                                i as u8 + 32,
                                (self.params.keyboard_octave.value() + root as i32).min(127) as u8,
                                harmony::row_quality(i / 7),
                            ));
                        }
                        self.drag = Some(Drag::Key(i as u8 + 32));
                        cx.capture();
                        return true;
                    }
                }
                if hit((32.0, 441.0, 158.0, 28.0), x, y) {
                    let next = (self.params.key.value() + 1) % 12;
                    Self::emit(
                        cx,
                        self.params.key.as_ptr(),
                        self.params.key.preview_normalized(next),
                    );
                    return true;
                }
                if hit((198.0, 441.0, 228.0, 28.0), x, y) {
                    let next = (self.params.scale.value() + 1) % 8;
                    Self::emit(
                        cx,
                        self.params.scale.as_ptr(),
                        self.params.scale.preview_normalized(next),
                    );
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
                for i in 0..8 {
                    if hit((32.0 + i as f32 * 67.0, 504.0, 61.0, 46.0), x, y) {
                        if cx.modifiers().shift() {
                            self.bridge.send(Command::Capture(i));
                            self.status = format!("Captured chord in slot {}", i + 1);
                        } else {
                            let word = self.params.slot(i).load(Ordering::Relaxed);
                            if let Some(chord) = SavedChord::decode(word) {
                                self.release_keys();
                                Self::emit(
                                    cx,
                                    self.params.inversion.as_ptr(),
                                    self.params
                                        .inversion
                                        .preview_normalized(chord.inversion as i32),
                                );
                                Self::emit(
                                    cx,
                                    self.params.quality.as_ptr(),
                                    self.params.quality.preview_normalized(chord.quality as i32),
                                );
                                Self::emit(
                                    cx,
                                    self.params.spread.as_ptr(),
                                    self.params.spread.preview_normalized(chord.spread as i32),
                                );
                                Self::emit(
                                    cx,
                                    self.params.transpose.as_ptr(),
                                    self.params
                                        .transpose
                                        .preview_normalized(chord.transpose as i32),
                                );
                                self.bridge.send(Command::Recall(word));
                                self.status = format!("Recalled slot {}", i + 1);
                            } else {
                                self.status =
                                    "Hold Shift and click a slot to capture the current chord"
                                        .into();
                            }
                        }
                        return true;
                    }
                }
                if hit(PAD, x, y) {
                    self.drag = Some(Drag::Pad);
                    cx.capture();
                    let px = ((x - PAD.0) / PAD.2).clamp(0.0, 1.0);
                    let py = (1.0 - (y - PAD.1) / PAD.3).clamp(0.0, 1.0);
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
                if hit((600.0, 516.0, 40.0, 34.0), x, y) {
                    self.inversion(cx, -1);
                    return true;
                }
                if hit((648.0, 516.0, 40.0, 34.0), x, y) {
                    self.inversion(cx, 1);
                    return true;
                }
                if hit((856.0, 516.0, 112.0, 34.0), x, y) {
                    self.mapping_axis = (self.mapping_axis + 1) % 3;
                    return true;
                }
                if hit((976.0, 516.0, 112.0, 34.0), x, y) {
                    self.bridge.send(Command::Learn(
                        if self.snapshot.learning == self.mapping_axis as u8 + 1 {
                            0
                        } else {
                            self.mapping_axis as u8 + 1
                        },
                    ));
                    return true;
                }
                if hit((600.0, 556.0, 250.0, 22.0), x, y) {
                    let field = self.params.mapping(self.mapping_axis);
                    let mut m = crate::engine::Mapping::decode(field.load(Ordering::Relaxed));
                    m.kind = (m.kind + 1) % 4;
                    if m.kind == 2 {
                        m.number %= 32;
                    }
                    if m.kind == 3 && m.channel == 16 {
                        m.channel = 0;
                    }
                    field.store(m.encode(), Ordering::Relaxed);
                    return true;
                }
                if hit((856.0, 556.0, 110.0, 22.0), x, y) {
                    let field = self.params.mapping(self.mapping_axis);
                    let mut m = crate::engine::Mapping::decode(field.load(Ordering::Relaxed));
                    m.channel = (m.channel + 1) % 17;
                    field.store(m.encode(), Ordering::Relaxed);
                    return true;
                }
                if hit((976.0, 556.0, 112.0, 22.0), x, y) {
                    let field = self.params.mapping(self.mapping_axis);
                    let mut m = crate::engine::Mapping::decode(field.load(Ordering::Relaxed));
                    m.number = (m.number + 1) % if m.kind == 2 { 32 } else { 128 };
                    field.store(m.encode(), Ordering::Relaxed);
                    return true;
                }
                for i in 0..6 {
                    if hit((32.0 + i as f32 * 157.0, 585.0, 149.0, 28.0), x, y) {
                        self.group = i;
                        self.page = 0;
                        return true;
                    }
                }
                if hit((982.0, 585.0, 106.0, 28.0), x, y) {
                    let count = self
                        .params
                        .controls()
                        .iter()
                        .filter(|c| c.group == self.group)
                        .count();
                    self.page = (self.page + 1) % count.div_ceil(8).max(1);
                    return true;
                }
                for (i, c) in self.shown_controls().iter().enumerate() {
                    let r = control_rect(i);
                    if hit(r, x, y) {
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
                for (i, c) in self.shown_controls().iter().enumerate() {
                    let r = control_rect(i);
                    if hit(r, x, y) {
                        self.release_keys();
                        self.edit = Some(ValueEdit::new(
                            c.id,
                            (r.0 + 100.0, r.1 + 3.0, r.2 - 108.0, 24.0),
                            c.value.clone(),
                        ));
                        return true;
                    }
                }
            }
            WindowEvent::MouseMove(_, _) => match self.drag {
                Some(Drag::Pad) => {
                    cx.emit(RawParamEvent::SetParameterNormalized(
                        self.params.x.as_ptr(),
                        ((x - PAD.0) / PAD.2).clamp(0.0, 1.0),
                    ));
                    cx.emit(RawParamEvent::SetParameterNormalized(
                        self.params.y.as_ptr(),
                        (1.0 - (y - PAD.1) / PAD.3).clamp(0.0, 1.0),
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
                if self.drag.is_some() {
                    self.end_drag(cx);
                    return true;
                }
            }
            WindowEvent::MouseScroll(_, dy) => {
                for (i, c) in self.shown_controls().iter().enumerate() {
                    if hit(control_rect(i), x, y) {
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
