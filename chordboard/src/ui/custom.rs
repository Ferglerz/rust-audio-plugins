//! Piano roll for the seven-step, chord-relative program stored on each tracker row.
use super::*;
use crate::arpeggiator::{default_custom, CustomStep, CUSTOM_STEPS};

const ROWS: i16 = 21;
fn grid() -> Rect {
    let r = CUSTOM_SURFACE;
    (r.0 + 96.0, r.1 + 100.0, r.2 - 128.0, r.3 - 164.0)
}
fn action_rect(index: usize) -> Rect {
    (
        CUSTOM_SURFACE.0 + 24.0 + index as f32 * 120.0,
        CUSTOM_SURFACE.1 + 50.0,
        112.0,
        28.0,
    )
}
pub(super) fn custom_button_rects() -> Vec<Rect> {
    (0..6).map(action_rect).collect()
}
fn pitch(step: CustomStep) -> i16 {
    step.octave as i16 * 7 + step.degree as i16 - 1
}
fn set_pitch(step: &mut CustomStep, value: i16) {
    let value = value.clamp(-28, 34);
    step.degree = value.rem_euclid(7) as u8 + 1;
    step.octave = value.div_euclid(7) as i8;
}
fn note_rect(index: usize, step: CustomStep, bottom: i16) -> Rect {
    let r = grid();
    let cell = r.2 / 8.0;
    let height = r.3 / ROWS as f32;
    (
        r.0 + index as f32 * cell + 2.0,
        r.1 + (ROWS - 1 - (pitch(step) - bottom)) as f32 * height + 2.0,
        (cell * step.gate as f32 / 100.0 - 4.0).max(8.0),
        height - 4.0,
    )
}
#[derive(Clone, Copy)]
pub(super) struct NoteDrag {
    row: usize,
    index: usize,
    x: f32,
    y: f32,
    resize: bool,
    steps: [CustomStep; CUSTOM_STEPS],
}
fn dragged_steps(drag: NoteDrag, x: f32, y: f32) -> [CustomStep; CUSTOM_STEPS] {
    let mut steps = drag.steps;
    let cell = grid().2 / 8.0;
    if drag.resize {
        steps[drag.index].gate = (steps[drag.index].gate as f32 + (x - drag.x) / cell * 100.0)
            .round()
            .clamp(1.0, 200.0) as u8;
    } else {
        let target = (drag.index as f32 + (x - drag.x) / cell)
            .round()
            .clamp(0.0, 6.0) as usize;
        let value =
            pitch(steps[drag.index]) + ((drag.y - y) / (grid().3 / ROWS as f32)).round() as i16;
        set_pitch(&mut steps[drag.index], value);
        // Move the note in time, shifting intervening notes without deleting any.
        if target > drag.index {
            steps[drag.index..=target].rotate_left(1);
        } else {
            steps[target..=drag.index].rotate_right(1);
        }
    }
    steps
}
impl ChordboardView {
    fn custom_note_at(&self, x: f32, y: f32) -> Option<usize> {
        let state = self.params.arpeggiator.snapshot();
        let row = state.editor_row(self.arp_ui.edit_page, !self.programming);
        (0..CUSTOM_STEPS).rev().find(|&i| {
            hit(
                note_rect(i, row.settings.custom[i], self.custom_bottom),
                x,
                y,
            )
        })
    }
    pub(super) fn draw_custom(&self, d: &mut Draw) {
        let state = self.params.arpeggiator.snapshot();
        let row = state.editor_row(self.arp_ui.edit_page, !self.programming);
        let r = CUSTOM_SURFACE;
        d.font = self.ui_font.get();
        d.text(
            r.0 + 24.0,
            r.1 + 29.0,
            &format!("CUSTOM PIANO ROLL · ROW {}", self.arp_ui.edit_page + 1),
            MODULE_TITLE_SIZE,
            TEAL,
        );
        for (i, label) in [
            "Rotate ←",
            "Rotate →",
            "Reverse",
            "Reset",
            "Octave ↓",
            "Octave ↑",
        ]
        .iter()
        .enumerate()
        {
            self.button(d, action_rect(i), label, false, TEAL);
        }
        let g = grid();
        let cell = g.2 / 8.0;
        let height = g.3 / ROWS as f32;
        let chord = row
            .chord
            .or_else(|| SavedChord::decode(self.snapshot.captured));
        let mut globals = self.params.global_harmony();
        if row.chord.is_none() {
            globals.transpose = 0;
        }
        for lane in 0..ROWS {
            let value = self.custom_bottom + lane;
            let degree = value.rem_euclid(7) as u8 + 1;
            let octave = value.div_euclid(7);
            let y = g.1 + (ROWS - 1 - lane) as f32 * height;
            let note = chord
                .and_then(|c| harmony::custom_degree_note(c, globals, 0, degree))
                .map(|n| n as i16 + octave * 12)
                .filter(|n| (0..=127).contains(n));
            let color = if degree == 1 {
                alpha(TEAL, 0.12)
            } else if lane % 2 == 0 {
                alpha(TEXT, 0.035)
            } else {
                BG
            };
            d.rect(g.0, y, g.2, height, color);
            d.line(g.0, y, g.0 + g.2, y, LINE, 1.0);
            let label = note.map_or_else(
                || format!("{degree} {octave:+}"),
                |n| format!("{} · {degree}", self.note_name(n as u8)),
            );
            d.text_middle(
                r.0 + 12.0,
                y + height * 0.5,
                &label,
                10.0,
                if degree == 1 { TEAL } else { MUTED },
            );
        }
        for i in 0..=8 {
            let x = g.0 + i as f32 * cell;
            d.line(x, g.1, x, g.1 + g.3, LINE, 1.0);
            if i < 7 {
                d.text(x + 8.0, g.1 - 8.0, &format!("{}", i + 1), TEXT_LABEL, MUTED);
            }
        }
        d.text(g.0 + 7.0 * cell + 8.0, g.1 - 8.0, "Tail", TEXT_LABEL, MUTED);
        let active = (self.snapshot.arp_running
            && self.snapshot.arp_row_index as usize == self.arp_ui.edit_page
            && row.settings.pattern == 7)
            .then(|| {
                let len = if row.settings.lock_length {
                    row.settings.steps.clamp(1, 24) as usize
                } else {
                    7
                };
                crate::engine::arpeggiator::program_index(
                    row.settings,
                    self.snapshot.arp_step as usize,
                    self.snapshot.arp_phase,
                    len,
                ) % 7
            });
        if let Some(i) = active {
            d.rect(g.0 + i as f32 * cell, g.1, 2.0, g.3, TEAL);
        }
        for (i, &step) in row.settings.custom.iter().enumerate() {
            if !(self.custom_bottom..self.custom_bottom + ROWS).contains(&pitch(step)) {
                continue;
            }
            let n = note_rect(i, step, self.custom_bottom);
            let color = if step.rest { MUTED } else { TEAL };
            d.rounded_rect(
                n.0,
                n.1,
                n.2,
                n.3,
                4.0,
                alpha(color, if active == Some(i) { 0.65 } else { 0.30 }),
            );
            d.outline_rounded(n.0, n.1, n.2, n.3, 4.0, color, 1.0);
            if n.2 > 60.0 {
                d.text_middle(
                    n.0 + 7.0,
                    n.1 + n.3 * 0.5,
                    &format!(
                        "{} · {}%",
                        if step.rest { "Rest" } else { "Note" },
                        step.velocity
                    ),
                    10.0,
                    TEXT,
                );
            }
            d.line(
                n.0 + n.2 - 5.0,
                n.1 + 4.0,
                n.0 + n.2 - 5.0,
                n.1 + n.3 - 4.0,
                color,
                2.0,
            );
        }
        d.text(r.0 + 24.0, r.1 + r.3 - 36.0, "Drag notes to reorder / change degree & octave · Drag right edges for duration · Right-click toggles rest · Scroll a note for velocity", TEXT_LABEL, MUTED);
        d.text(r.0 + 24.0, r.1 + r.3 - 16.0, "Seven chord-relative steps · Duration and velocity multiply the arp lane and global controls · Cmd/Ctrl-click restores a step", TEXT_LABEL, MUTED);
    }
    pub(super) fn custom_event(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
    ) -> bool {
        if let Some(drag) = self.custom_drag {
            match event {
                WindowEvent::MouseMove(_, _) => {
                    let steps = dragged_steps(drag, x, y);
                    self.params.arpeggiator.edit(|s| {
                        let row = s.editor_row_mut(drag.row, !self.programming);
                        row.settings.custom = steps;
                    });
                    cx.needs_redraw();
                    return true;
                }
                WindowEvent::MouseUp(_)
                | WindowEvent::FocusOut
                | WindowEvent::KeyDown(Code::Escape, _) => {
                    self.custom_drag = None;
                    cx.release();
                    cx.needs_redraw();
                    return true;
                }
                _ => {}
            }
        }
        if self.slide_panel != Panel::Custom
            || (self.panel != Some(Panel::Custom) && self.route_progress == 0.0)
        {
            return false;
        }
        if matches!(event, WindowEvent::KeyDown(Code::Escape, _)) {
            self.set_panel(None);
            cx.needs_redraw();
            return true;
        }
        if !hit(CUSTOM_SURFACE, x, y) {
            return false;
        }
        if self.route_elapsed < pleasant_ui::page_slide::DURATION {
            return true;
        }
        match event {
            WindowEvent::MouseDown(MouseButton::Left)
            | WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                if hit(self.panel_close_rect(Panel::Custom), x, y) {
                    self.set_panel(None);
                } else if let Some(action) = (0..6).find(|&i| hit(action_rect(i), x, y)) {
                    if action >= 4 {
                        self.custom_bottom =
                            (self.custom_bottom + if action == 4 { -7 } else { 7 }).clamp(-28, 14);
                    } else {
                        let index = self.arp_ui.edit_page;
                        self.params.arpeggiator.edit(|s| {
                            let row = s.editor_row_mut(index, !self.programming);
                            match action {
                                0 => row.settings.custom.rotate_left(1),
                                1 => row.settings.custom.rotate_right(1),
                                2 => row.settings.custom.reverse(),
                                _ => row.settings.custom = default_custom(),
                            }
                        });
                    }
                } else if let Some(index) = self.custom_note_at(x, y) {
                    let row_index = self.arp_ui.edit_page;
                    if cx.modifiers().intersects(Modifiers::CTRL | Modifiers::LOGO) {
                        self.params.arpeggiator.edit(|s| {
                            let row = s.editor_row_mut(row_index, !self.programming);
                            row.settings.custom[index] = default_custom()[index];
                        });
                    } else {
                        let steps = self
                            .params
                            .arpeggiator
                            .snapshot()
                            .editor_row(row_index, !self.programming)
                            .settings
                            .custom;
                        let n = note_rect(index, steps[index], self.custom_bottom);
                        self.custom_drag = Some(NoteDrag {
                            row: row_index,
                            index,
                            x,
                            y,
                            resize: x >= n.0 + n.2 - (n.2 / 3.0).min(12.0),
                            steps,
                        });
                        cx.capture();
                    }
                }
            }
            WindowEvent::MouseDown(MouseButton::Right) => {
                if let Some(i) = self.custom_note_at(x, y) {
                    let index = self.arp_ui.edit_page;
                    self.params.arpeggiator.edit(|s| {
                        let row = s.editor_row_mut(index, !self.programming);
                        row.settings.custom[i].rest ^= true;
                    });
                }
            }
            WindowEvent::MouseScroll(_, dy) => {
                if let Some(i) = self.custom_note_at(x, y) {
                    let index = self.arp_ui.edit_page;
                    self.params.arpeggiator.edit(|s| {
                        let row = s.editor_row_mut(index, !self.programming);
                        let step = &mut row.settings.custom[i];
                        step.velocity = (step.velocity as i16 + if *dy > 0.0 { 5 } else { -5 })
                            .clamp(0, 200) as u8;
                    });
                }
            }
            _ => {}
        }
        cx.needs_redraw();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn piano_roll_move_preserves_steps_and_changes_pitch() {
        let drag = NoteDrag {
            row: 2,
            index: 0,
            x: 0.0,
            y: 0.0,
            resize: false,
            steps: default_custom(),
        };
        let steps = dragged_steps(drag, grid().2 / 8.0 * 2.0, -grid().3 / ROWS as f32 * 7.0);
        assert_eq!(steps[2].degree, 1);
        assert_eq!(steps[2].octave, 1);
        assert_eq!(steps[0], drag.steps[1]);
        assert_eq!(steps[1], drag.steps[2]);
        assert_eq!(steps[3..], drag.steps[3..]);
    }
    #[test]
    fn piano_roll_edges_resize_without_changing_pitch_or_order() {
        let drag = NoteDrag {
            row: 0,
            index: 3,
            x: 0.0,
            y: 0.0,
            resize: true,
            steps: default_custom(),
        };
        let steps = dragged_steps(drag, -grid().2 / 16.0, 100.0);
        assert_eq!(steps[3].gate, 50);
        assert_eq!(steps[3].degree, 4);
        assert_eq!(dragged_steps(drag, 99999.0, 0.0)[3].gate, 200);
        assert_eq!(dragged_steps(drag, -99999.0, 0.0)[3].gate, 1);
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;

    #[test]
    fn custom_overlay_covers_modules_and_tracker_and_keeps_edits_on_row() {
        let mut view = ChordboardView::new(
            Arc::new(ChordboardParams::default()),
            Arc::new(Bridge::default()),
            Arc::new(PreviewContext),
        );
        view.programming = true;
        view.params.arpeggiator.edit(|s| {
            s.add_row_after(0);
        });
        view.arp_ui.edit_page = 1;
        view.set_panel(Some(Panel::Custom));
        view.route_elapsed = pleasant_ui::page_slide::DURATION;
        view.route_progress = 1.0;
        let mut cx = Context::default();
        let target = Element::new(&mut cx).entity();
        let mut cx = EventContext::new_with_current(&mut cx, target);
        assert!(view.custom_event(
            &mut cx,
            &WindowEvent::MouseDown(MouseButton::Left),
            TRACKER_SURFACE.0 + 10.0,
            TRACKER_SURFACE.1 + 10.0
        ));
        for surface in [
            CHORDS_SURFACE,
            PERF_SURFACE,
            ARP_SURFACE,
            MOD_SURFACE,
            PIANO_SURFACE,
        ] {
            assert!(hit(
                CUSTOM_SURFACE,
                surface.0 + surface.2 * 0.5,
                surface.1 + surface.3 * 0.5
            ));
        }
        let n = note_rect(0, default_custom()[0], view.custom_bottom);
        view.custom_event(
            &mut cx,
            &WindowEvent::MouseDown(MouseButton::Left),
            n.0 + 8.0,
            n.1 + 8.0,
        );
        view.custom_event(
            &mut cx,
            &WindowEvent::MouseMove(0.0, 0.0),
            n.0 + 8.0,
            n.1 + 8.0 - grid().3 / ROWS as f32,
        );
        view.custom_event(
            &mut cx,
            &WindowEvent::MouseUp(MouseButton::Left),
            n.0 + 8.0,
            n.1 + 8.0,
        );
        let state = view.params.arpeggiator.snapshot();
        assert_eq!(state.rows[0].settings.custom, default_custom());
        assert_eq!(state.rows[1].settings.custom[0].degree, 2);
        assert!(view.custom_drag.is_none());
    }

    #[test]
    fn direction_dropdowns_share_modes_except_custom() {
        assert_eq!(Menu::GlissDirection.items(), Menu::ArpPattern.items()[..7]);
        assert_eq!(Menu::ArpPattern.items()[7], "Custom");
    }
}
