use super::*;

#[derive(Default)]
pub(super) struct MemoryUi {
    pub(super) armed: bool,
    pub(super) flash: [f32; MEMORY_COUNT],
    pending: Option<PendingCapture>,
}

struct PendingCapture {
    slot: usize,
    before: u64,
    elapsed: f32,
}

#[derive(Clone, Copy)]
pub(super) struct MemoryDrag {
    source: usize,
    word: u64,
    origin: (f32, f32),
    position: (f32, f32),
    active: bool,
}

#[derive(Debug, PartialEq)]
enum MemoryAction {
    Recall,
    Move(usize),
}

impl MemoryDrag {
    fn new(source: usize, word: u64, x: f32, y: f32) -> Self {
        Self {
            source,
            word,
            origin: (x, y),
            position: (x, y),
            active: false,
        }
    }

    pub(super) fn update(&mut self, x: f32, y: f32) {
        self.position = (x, y);
        self.active |= (x - self.origin.0).hypot(y - self.origin.1) >= 6.0;
    }

    fn action(self) -> Option<MemoryAction> {
        let target = memory_at(self.position.0, self.position.1)?;
        if self.active {
            (target != self.source).then_some(MemoryAction::Move(target))
        } else {
            (target == self.source).then_some(MemoryAction::Recall)
        }
    }
}

fn memory_at(x: f32, y: f32) -> Option<usize> {
    (0..MEMORY_COUNT).find(|&i| hit(memory_rect(i), x, y))
}

// Use the persisted slot atomics, just as capture does. Refuse stale drags if
// a capture or state restore has replaced the source since the mouse went down.
fn move_memory(params: &ChordboardParams, drag: MemoryDrag, target: usize) -> bool {
    if target >= MEMORY_COUNT || target == drag.source || SavedChord::decode(drag.word).is_none() {
        return false;
    }
    if params
        .slot(drag.source)
        .compare_exchange(drag.word, 0, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return false;
    }
    let displaced = params.slot(target).swap(drag.word, Ordering::Relaxed);
    if displaced != 0 {
        params.slot(drag.source).store(displaced, Ordering::Relaxed);
    }
    true
}

fn dashed_outline(d: &mut Draw, r: Rect, color: Color) {
    for edge in [r.1, r.1 + r.3] {
        let mut x = r.0;
        while x < r.0 + r.2 {
            d.line(x, edge, (x + 4.0).min(r.0 + r.2), edge, color, 1.0);
            x += 8.0;
        }
    }
    for edge in [r.0, r.0 + r.2] {
        let mut y = r.1;
        while y < r.1 + r.3 {
            d.line(edge, y, edge, (y + 4.0).min(r.1 + r.3), color, 1.0);
            y += 8.0;
        }
    }
}

impl ChordboardView {
    pub(super) fn can_capture(&self) -> bool {
        SavedChord::decode(self.snapshot.captured).is_some()
    }

    #[cfg(test)]
    pub(super) fn toggle_memory_save(&mut self) {
        if self.memory_ui.armed {
            self.memory_ui.armed = false;
            self.status = "Save cancelled".into();
        } else if self.can_capture() {
            self.memory_ui.armed = true;
            self.status = "Choose a memory to save this chord".into();
        } else {
            self.status = "Play a chord before saving".into();
        }
    }

    pub(super) fn request_capture(&mut self, slot: usize) {
        if slot >= MEMORY_COUNT {
            return;
        }
        self.memory_ui.armed = false;
        if !self.can_capture() {
            self.status = "Play a chord before saving".into();
            return;
        }
        if self.memory_ui.pending.is_some() {
            self.status = "Saving… wait for the current memory".into();
            return;
        }
        let expected = self.snapshot.captured;
        let before = self.params.slot(slot).load(Ordering::Relaxed);
        if before == expected {
            self.status = format!("Chord already stored in memory {}", slot + 1);
            self.memory_ui.flash[slot] = 1.0;
            return;
        }
        self.memory_ui.pending = Some(PendingCapture {
            slot,
            before,
            elapsed: 0.0,
        });
        self.bridge.send(Command::Capture(slot));
        self.status = format!("Saving to memory {}…", slot + 1);
    }

    pub(super) fn tick_memories(&mut self, dt: f32) -> bool {
        let mut changed = false;
        for flash in &mut self.memory_ui.flash {
            if *flash > 0.0 {
                *flash = (*flash - dt.max(0.0) * 1.8).max(0.0);
                changed = true;
            }
        }
        if let Some(pending) = &mut self.memory_ui.pending {
            pending.elapsed += dt.max(0.0);
            let stored = self.params.slot(pending.slot).load(Ordering::Relaxed);
            // Capture reads engine memory when processed, which can be newer
            // than the UI snapshot used to initiate the request.
            if stored != pending.before && SavedChord::decode(stored).is_some() {
                self.status = format!("Saved chord in memory {}", pending.slot + 1);
                self.memory_ui.flash[pending.slot] = 1.0;
                self.memory_ui.pending = None;
                changed = true;
            } else if pending.elapsed >= 2.0 {
                self.status = "Save was not confirmed; try again".into();
                self.memory_ui.pending = None;
                changed = true;
            }
        }
        changed
    }

    pub(super) fn memory_press(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        let Some(slot) = memory_at(x, y) else {
            return false;
        };
        let word = self.params.slot(slot).load(Ordering::Relaxed);
        if SavedChord::decode(word).is_some()
            && hit(memory_delete_rect(slot), x, y)
            && !cx.modifiers().shift()
            && !self.memory_ui.armed
        {
            self.params.slot(slot).store(0, Ordering::Relaxed);
            self.status = format!("Deleted memory {}", slot + 1);
        } else if cx.modifiers().shift() || self.memory_ui.armed {
            self.request_capture(slot);
        } else if SavedChord::decode(word).is_some() {
            self.drag = Some(Drag::Memory(MemoryDrag::new(slot, word, x, y)));
            cx.capture();
        } else {
            self.request_capture(slot);
        }
        true
    }

    pub(super) fn finish_memory_drag(
        &mut self,
        cx: &mut EventContext,
        mut drag: MemoryDrag,
        x: f32,
        y: f32,
    ) {
        drag.update(x, y);
        match drag.action() {
            Some(MemoryAction::Move(target)) => {
                let occupied =
                    SavedChord::decode(self.params.slot(target).load(Ordering::Relaxed)).is_some();
                if move_memory(&self.params, drag, target) {
                    self.status = if occupied {
                        format!("Swapped memories {} and {}", drag.source + 1, target + 1)
                    } else {
                        format!("Moved memory {} to {}", drag.source + 1, target + 1)
                    };
                } else {
                    self.status = "Memory changed during drag; move cancelled".into();
                }
            }
            Some(MemoryAction::Recall) => self.recall_memory(cx, drag.source),
            None => {}
        }
    }

    pub(super) fn recall_memory(&mut self, cx: &mut EventContext, slot: usize) {
        let word = self.params.slot(slot).load(Ordering::Relaxed);
        if let Some(chord) = SavedChord::decode(word) {
            self.release_keys();
            for (param, value) in [
                (&self.params.inversion, chord.inversion as i32),
                (&self.params.quality, chord.quality as i32),
                (&self.params.spread, chord.spread as i32),
                (&self.params.transpose, chord.transpose as i32),
            ] {
                Self::emit(cx, param.as_ptr(), param.preview_normalized(value));
            }
            self.bridge.send(Command::RecallMemory(slot, word));
            self.status = format!("Recalled slot {}", slot + 1);
        }
    }

    pub(super) fn draw_memories(&self, d: &mut Draw) {
        let drag = match self.drag {
            Some(Drag::Memory(drag)) if drag.active => Some(drag),
            _ => None,
        };
        let hovered = self.hover_pointer().and_then(|(x, y)| memory_at(x, y));
        for (i, shortcut) in MEMORY_HINTS.iter().enumerate() {
            let r = memory_rect(i);
            let chord = SavedChord::decode(self.params.slot(i).load(Ordering::Relaxed));
            let source = drag.is_some_and(|drag| drag.source == i);
            let target = drag.is_some_and(|drag| {
                drag.source != i && memory_at(drag.position.0, drag.position.1) == Some(i)
            });
            if chord.is_some() {
                d.rect(
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    alpha(GOLD, if source { 0.03 } else { 0.1 }),
                );
                d.outline(r, if target { TEAL } else { alpha(GOLD, 0.45) });
            } else {
                dashed_outline(
                    d,
                    r,
                    if target || hovered == Some(i) {
                        TEAL
                    } else {
                        alpha(MUTED, 0.35)
                    },
                );
            }
            if target || (self.memory_ui.armed && hovered == Some(i)) {
                d.rect(r.0, r.1, r.2, r.3, alpha(TEAL, 0.12));
                d.outline(r, TEAL);
            }
            let flash = self.memory_ui.flash[i];
            if flash > 0.0 {
                d.rect(r.0, r.1, r.2, r.3, alpha(TEAL, flash * 0.22));
                d.outline(r, TEAL);
            }
            d.text(
                r.0 + 7.0,
                r.1 + 14.0,
                &(i + 1).to_string(),
                TEXT_SMALL,
                MUTED,
            );
            let label = chord.map_or_else(|| "+".into(), harmony::chord_name);
            let label = self.fit_text(d, &label, r.2 - 4.0, 12.0);
            d.text_centered(
                r.0 + r.2 / 2.0,
                r.1 + 32.0,
                &label,
                12.0,
                if source {
                    MUTED
                } else if chord.is_some() {
                    GOLD
                } else {
                    MUTED
                },
            );
            d.text_right(
                r.0 + r.2 - 4.0,
                r.1 + r.3 - 4.0,
                shortcut,
                TEXT_SMALL,
                MUTED,
            );
            if chord.is_some() && hovered == Some(i) && drag.is_none() {
                let close = memory_delete_rect(i);
                let hot = self.hover_pointer().is_some_and(|(x, y)| hit(close, x, y));
                d.rect(close.0, close.1, close.2, close.3, PANEL);
                d.text_centered(
                    close.0 + close.2 / 2.0,
                    close.1 + 16.0,
                    "×",
                    16.0,
                    if hot { TEXT } else { MUTED },
                );
            }
        }
    }

    pub(super) fn draw_memory_drag(&self, d: &mut Draw) {
        if let Some(Drag::Memory(drag)) = self.drag {
            if drag.active {
                if let Some(chord) = SavedChord::decode(drag.word) {
                    let x = (drag.position.0 - 42.0).clamp(8.0, W - 92.0);
                    let y = (drag.position.1 - 46.0).clamp(8.0, H - 40.0);
                    d.rect(x, y, 84.0, 32.0, PANEL);
                    d.outline((x, y, 84.0, 32.0), GOLD);
                    d.text_centered(
                        x + 42.0,
                        y + 21.0,
                        &self.named_chord(chord),
                        TEXT_LABEL,
                        GOLD,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn chord(root: u8) -> u64 {
        SavedChord {
            root,
            second: None,
            quality: 0,
            inversion: 0,
            spread: 0,
            transpose: 0,
        }
        .encode()
    }
    fn view() -> ChordboardView {
        ChordboardView::new(
            Arc::new(ChordboardParams::default()),
            Arc::new(Bridge::default()),
            Arc::new(PreviewContext),
        )
    }

    #[test]
    fn saving_requires_a_valid_chord_and_can_be_cancelled() {
        let mut view = view();
        assert!(!view.can_capture());
        view.toggle_memory_save();
        assert!(!view.memory_ui.armed);
        view.request_capture(0);
        assert!(view.bridge.commands.pop().is_none());
        assert!(view.memory_ui.pending.is_none());
        assert_eq!(view.params.slot(0).load(Ordering::Relaxed), 0);

        view.snapshot.captured = chord(60);
        assert!(view.can_capture());
        view.toggle_memory_save();
        assert!(view.memory_ui.armed);
        view.toggle_memory_save();
        assert!(!view.memory_ui.armed);
        assert!(view.bridge.commands.pop().is_none());
    }

    #[test]
    fn capture_waits_for_a_changed_valid_persisted_chord() {
        let mut view = view();
        view.snapshot.captured = chord(60);
        view.params.slot(2).store(chord(64), Ordering::Relaxed);
        view.toggle_memory_save();
        view.request_capture(2);
        assert!(!view.memory_ui.armed);
        assert!(matches!(
            view.bridge.commands.pop(),
            Some(Command::Capture(2))
        ));
        assert!(view.status.starts_with("Saving"));
        assert_eq!(view.memory_ui.flash[2], 0.0);
        assert!(!view.tick_memories(0.1));
        assert!(view.memory_ui.pending.is_some());
        assert!(view.status.starts_with("Saving"));

        view.params.slot(2).store(0, Ordering::Relaxed);
        assert!(!view.tick_memories(0.1));
        assert!(view.memory_ui.pending.is_some());
        assert_eq!(view.memory_ui.flash[2], 0.0);

        view.params.slot(2).store(chord(60), Ordering::Relaxed);
        assert!(view.tick_memories(0.1));
        assert!(view.memory_ui.pending.is_none());
        assert_eq!(view.memory_ui.flash[2], 1.0);
        assert_eq!(view.status, "Saved chord in memory 3");
    }

    #[test]
    fn capture_accepts_engine_chord_changed_since_ui_snapshot() {
        let mut view = view();
        view.snapshot.captured = chord(60);
        view.params.slot(2).store(chord(64), Ordering::Relaxed);
        view.request_capture(2);
        assert!(matches!(
            view.bridge.commands.pop(),
            Some(Command::Capture(2))
        ));
        assert!(!view.tick_memories(0.1));
        assert_eq!(view.memory_ui.flash[2], 0.0);

        // The audio thread processes Capture after the musician changes chord.
        view.params.slot(2).store(chord(67), Ordering::Relaxed);
        assert!(view.tick_memories(0.1));
        assert!(view.memory_ui.pending.is_none());
        assert_eq!(view.memory_ui.flash[2], 1.0);
        assert_eq!(view.status, "Saved chord in memory 3");
    }

    #[test]
    fn identical_capture_reports_already_stored_without_a_command() {
        let mut view = view();
        view.snapshot.captured = chord(60);
        view.params.slot(1).store(chord(60), Ordering::Relaxed);
        view.toggle_memory_save();
        view.request_capture(1);
        assert!(!view.memory_ui.armed);
        assert!(view.memory_ui.pending.is_none());
        assert!(view.bridge.commands.pop().is_none());
        assert_eq!(view.status, "Chord already stored in memory 2");
        assert_eq!(view.memory_ui.flash[1], 1.0);
    }

    #[test]
    fn unconfirmed_capture_times_out_without_success_flash() {
        let mut view = view();
        view.snapshot.captured = chord(60);
        view.request_capture(0);
        assert!(matches!(
            view.bridge.commands.pop(),
            Some(Command::Capture(0))
        ));
        assert!(view.tick_memories(2.0));
        assert!(view.memory_ui.pending.is_none());
        assert_eq!(view.memory_ui.flash[0], 0.0);
        assert_eq!(view.status, "Save was not confirmed; try again");
        assert_eq!(view.params.slot(0).load(Ordering::Relaxed), 0);
        view.request_capture(1);
        assert!(matches!(
            view.bridge.commands.pop(),
            Some(Command::Capture(1))
        ));
    }

    #[test]
    fn click_jitter_recalls_but_drag_back_or_outside_cancels() {
        let first = memory_rect(0);
        let second = memory_rect(1);
        let x = first.0 + first.2 / 2.0;
        let y = first.1 + first.3 / 2.0;
        let mut drag = MemoryDrag::new(0, chord(60), x, y);
        drag.update(x + 2.0, y + 1.0);
        assert_eq!(drag.action(), Some(MemoryAction::Recall));
        drag.update(second.0 + second.2 / 2.0, y);
        assert_eq!(drag.action(), Some(MemoryAction::Move(1)));
        drag.update(x, y);
        assert_eq!(drag.action(), None);
        drag.update(CHORDS_SURFACE.0 + CHORDS_SURFACE.2 - 10.0, y);
        assert_eq!(drag.action(), None);
    }
    #[test]
    fn drop_moves_or_swaps_without_losing_chords() {
        let params = ChordboardParams::default();
        params.slot(0).store(chord(60), Ordering::Relaxed);
        let drag = MemoryDrag::new(0, chord(60), 50.0, 530.0);
        assert!(move_memory(&params, drag, 1));
        assert_eq!(params.slot(0).load(Ordering::Relaxed), 0);
        assert_eq!(params.slot(1).load(Ordering::Relaxed), chord(60));
        params.slot(0).store(chord(64), Ordering::Relaxed);
        let drag = MemoryDrag::new(0, chord(64), 50.0, 530.0);
        assert!(move_memory(&params, drag, 1));
        assert_eq!(params.slot(0).load(Ordering::Relaxed), chord(60));
        assert_eq!(params.slot(1).load(Ordering::Relaxed), chord(64));
    }
    #[test]
    fn stale_or_same_slot_drop_leaves_memories_untouched() {
        let params = ChordboardParams::default();
        params.slot(0).store(chord(64), Ordering::Relaxed);
        params.slot(1).store(chord(67), Ordering::Relaxed);
        let stale = MemoryDrag::new(0, chord(60), 50.0, 530.0);
        assert!(!move_memory(&params, stale, 1));
        let current = MemoryDrag::new(0, chord(64), 50.0, 530.0);
        assert!(!move_memory(&params, current, 0));
        assert!(!move_memory(&params, current, MEMORY_COUNT));
        assert_eq!(params.slot(0).load(Ordering::Relaxed), chord(64));
        assert_eq!(params.slot(1).load(Ordering::Relaxed), chord(67));
    }
}
