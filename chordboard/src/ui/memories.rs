use super::*;

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
    (0..8).find(|&i| hit(memory_rect(i), x, y))
}

// Use the persisted slot atomics, just as capture does. Refuse stale drags if
// a capture or state restore has replaced the source since the mouse went down.
fn move_memory(params: &ChordboardParams, drag: MemoryDrag, target: usize) -> bool {
    if target >= 8 || target == drag.source || SavedChord::decode(drag.word).is_none() {
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
    pub(super) fn memory_press(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        let Some(slot) = memory_at(x, y) else {
            return false;
        };
        let word = self.params.slot(slot).load(Ordering::Relaxed);
        if SavedChord::decode(word).is_some()
            && hit(memory_delete_rect(slot), x, y)
            && !cx.modifiers().shift()
        {
            self.params.slot(slot).store(0, Ordering::Relaxed);
            self.status = format!("Deleted memory {}", slot + 1);
        } else if cx.modifiers().shift() {
            self.bridge.send(Command::Capture(slot));
            self.status = format!("Captured chord in slot {}", slot + 1);
        } else if SavedChord::decode(word).is_some() {
            self.drag = Some(Drag::Memory(MemoryDrag::new(slot, word, x, y)));
            cx.capture();
        } else {
            self.status = "Shift-click to capture a chord, or drag a filled memory here".into();
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

    fn recall_memory(&mut self, cx: &mut EventContext, slot: usize) {
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
            self.bridge.send(Command::Recall(word));
            self.status = format!("Recalled slot {}", slot + 1);
        }
    }

    pub(super) fn draw_memories(&self, d: &mut Draw) {
        d.text(32.0, 492.0, "CHORD MEMORIES", TEXT_SMALL, MUTED);
        d.text_right(
            564.0,
            492.0,
            "SHIFT-CLICK TO CAPTURE · DRAG TO MOVE",
            TEXT_SMALL,
            MUTED,
        );
        let drag = match self.drag {
            Some(Drag::Memory(drag)) if drag.active => Some(drag),
            _ => None,
        };
        let hovered = self.pointer.and_then(|(x, y)| memory_at(x, y));
        for i in 0..8 {
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
                        MUTED
                    },
                );
            }
            if target {
                d.rect(r.0, r.1, r.2, r.3, alpha(TEAL, 0.12));
            }
            d.text(
                r.0 + 7.0,
                r.1 + 14.0,
                &(i + 1).to_string(),
                TEXT_SMALL,
                MUTED,
            );
            let label = chord.map_or_else(|| "+".into(), harmony::chord_name);
            d.text_centered(
                r.0 + r.2 / 2.0,
                r.1 + 35.0,
                &label,
                TEXT_LABEL,
                if source {
                    MUTED
                } else if chord.is_some() {
                    GOLD
                } else {
                    MUTED
                },
            );
            if chord.is_some() && hovered == Some(i) && drag.is_none() {
                let close = memory_delete_rect(i);
                let hot = self.pointer.is_some_and(|(x, y)| hit(close, x, y));
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
                        &harmony::chord_name(chord),
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
    #[test]
    fn click_jitter_recalls_but_drag_back_or_outside_cancels() {
        let mut drag = MemoryDrag::new(0, chord(60), 50.0, 530.0);
        drag.update(52.0, 531.0);
        assert_eq!(drag.action(), Some(MemoryAction::Recall));
        drag.update(120.0, 530.0);
        assert_eq!(drag.action(), Some(MemoryAction::Move(1)));
        drag.update(50.0, 530.0);
        assert_eq!(drag.action(), None);
        drag.update(590.0, 530.0);
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
        assert!(!move_memory(&params, current, 8));
        assert_eq!(params.slot(0).load(Ordering::Relaxed), chord(64));
        assert_eq!(params.slot(1).load(Ordering::Relaxed), chord(67));
    }
}
