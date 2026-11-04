//! Seven-step chord-relative program, stored with the selected tracker row.
use super::*;
use crate::arpeggiator::{default_custom, CustomStep, CUSTOM_STEPS};

#[derive(Clone, Copy)]
enum Field {
    Degree,
    Octave,
    Rest,
    Velocity,
    Gate,
}
const FIELDS: [Field; 5] = [
    Field::Degree,
    Field::Octave,
    Field::Rest,
    Field::Velocity,
    Field::Gate,
];

fn column_rect(index: usize) -> Rect {
    let width = (CHORDS_SURFACE.2 - 32.0) / CUSTOM_STEPS as f32;
    (
        CHORDS_SURFACE.0 + 16.0 + index as f32 * width,
        CHORDS_SURFACE.1 + 48.0,
        width,
        CHORDS_SURFACE.3 - 60.0,
    )
}

fn field_rect(index: usize, field: Field) -> Rect {
    let r = column_rect(index);
    let (y, height) = match field {
        Field::Degree => (80.0, 50.0),
        Field::Octave => (180.0, 32.0),
        Field::Rest => (222.0, 28.0),
        Field::Velocity => (282.0, 32.0),
        Field::Gate => (350.0, 32.0),
    };
    (r.0 + 8.0, CHORDS_SURFACE.1 + y, r.2 - 16.0, height)
}

fn action_rect(index: usize) -> Rect {
    (
        CHORDS_SURFACE.0 + CHORDS_SURFACE.2 - 382.0 + index as f32 * 80.0,
        CHORDS_SURFACE.1 + 8.0,
        74.0,
        28.0,
    )
}

pub(super) fn custom_button_rects() -> Vec<Rect> {
    (0..4)
        .map(action_rect)
        .chain((0..CUSTOM_STEPS).flat_map(|index| FIELDS.map(|field| field_rect(index, field))))
        .collect()
}

fn adjust(step: &mut CustomStep, index: usize, field: Field, delta: i16, reset: bool) {
    match field {
        Field::Degree => {
            step.degree = if reset {
                index as u8 + 1
            } else {
                (step.degree as i16 + delta).clamp(1, 7) as u8
            }
        }
        Field::Octave => {
            step.octave = if reset {
                0
            } else {
                (step.octave as i16 + delta).clamp(-4, 4) as i8
            }
        }
        Field::Rest => step.rest = !reset && !step.rest,
        Field::Velocity => {
            step.velocity = if reset {
                100
            } else {
                (step.velocity as i16 + delta * 5).clamp(0, 200) as u8
            }
        }
        Field::Gate => {
            step.gate = if reset {
                100
            } else {
                (step.gate as i16 + delta * 5).clamp(1, 200) as u8
            }
        }
    }
}

impl ChordboardView {
    pub(super) fn draw_custom(&self, d: &mut Draw) {
        let state = self.params.arpeggiator.snapshot();
        let Some(row) = state.rows.get(self.arp_ui.edit_page) else {
            return;
        };
        d.font = self.ui_font.get();
        d.text(
            CHORDS_SURFACE.0 + 14.0,
            module_title_y(CHORDS_SURFACE.1, MODULE_TITLE_SIZE),
            &format!("CUSTOM · ROW {}", self.arp_ui.edit_page + 1),
            MODULE_TITLE_SIZE,
            TEAL,
        );
        for (index, label) in ["Rotate ←", "Rotate →", "Reverse", "Reset"]
            .into_iter()
            .enumerate()
        {
            self.button(d, action_rect(index), label, false, TEAL);
        }
        let chord = row
            .chord
            .or_else(|| SavedChord::decode(self.snapshot.captured));
        let mut globals = self.params.global_harmony();
        if row.chord.is_none() {
            globals.transpose = 0;
        }
        let pass_len = if row.settings.lock_length {
            row.settings.steps.clamp(1, 24) as usize
        } else {
            CUSTOM_STEPS
        };
        let active = (self.snapshot.arp_running
            && self.snapshot.arp_row_index as usize == self.arp_ui.edit_page
            && row.settings.pattern == 7)
            .then(|| {
                crate::engine::arpeggiator::program_index(
                    row.settings,
                    self.snapshot.arp_step as usize,
                    self.snapshot.arp_phase,
                    pass_len,
                ) % CUSTOM_STEPS
            });
        for (index, step) in row.settings.custom.iter().enumerate() {
            let column = column_rect(index);
            let sounding = active == Some(index);
            d.rounded_rect(
                column.0 + 2.0,
                column.1,
                column.2 - 4.0,
                column.3,
                5.0,
                alpha(TEAL, if sounding { 0.12 } else { 0.025 }),
            );
            d.text_centered(
                column.0 + column.2 * 0.5,
                CHORDS_SURFACE.1 + 67.0,
                &format!("STEP {}", index + 1),
                10.0,
                if sounding { TEAL } else { MUTED },
            );
            for field in FIELDS {
                let r = field_rect(index, field);
                let (label, caption) = match field {
                    Field::Degree => (format!("‹  {}  ›", step.degree), None),
                    Field::Octave => (format!("‹ {:+} ›", step.octave), Some("OCTAVE")),
                    Field::Rest => (
                        if step.rest {
                            "REST".into()
                        } else {
                            "PLAY".into()
                        },
                        None,
                    ),
                    Field::Velocity => (format!("‹ {}% ›", step.velocity), Some("VELOCITY ×")),
                    Field::Gate => (format!("‹ {}% ›", step.gate), Some("GATE ×")),
                };
                if let Some(caption) = caption {
                    d.text_centered(column.0 + column.2 * 0.5, r.1 - 9.0, caption, 10.0, MUTED);
                }
                self.button(
                    d,
                    r,
                    &label,
                    match field {
                        Field::Degree => sounding,
                        Field::Rest => step.rest,
                        _ => false,
                    },
                    if step.rest { MUTED } else { TEAL },
                );
            }
            let note = chord
                .and_then(|chord| harmony::custom_degree_note(chord, globals, 0, step.degree))
                .map(|note| note as i16 + step.octave as i16 * 12)
                .filter(|&note| (0..=127).contains(&note));
            let label = note.map_or_else(
                || "—".into(),
                |note| {
                    harmony::midi_note_name_in_key(
                        note as u8,
                        self.params.key.value() as u8,
                        self.params.scale.value() as u8,
                        self.params.key_spelling.value(),
                    )
                },
            );
            let label = self.fit_text(d, &music::label(&label), column.2 - 16.0, 14.0);
            d.text_centered(
                column.0 + column.2 * 0.5,
                CHORDS_SURFACE.1 + 153.0,
                &label,
                14.0,
                if step.rest { MUTED } else { TEXT },
            );
        }
        d.text(
            CHORDS_SURFACE.0 + 20.0,
            CHORDS_SURFACE.1 + CHORDS_SURFACE.3 - 10.0,
            if row.chord.is_some() {
                "Degrees follow the chord root · Scroll or click ‹ / › · Cmd/Ctrl-click resets"
            } else {
                "Previewing the live chord · Assign a chord to this tracker row to play it"
            },
            10.0,
            MUTED,
        );
    }

    pub(super) fn press_custom(
        &mut self,
        cx: &mut EventContext,
        x: f32,
        y: f32,
        reset: bool,
    ) -> bool {
        if !hit(CHORDS_SURFACE, x, y) {
            return false;
        }
        let row_index = self.arp_ui.edit_page;
        if let Some(action) = (0..4).find(|&index| hit(action_rect(index), x, y)) {
            self.params.arpeggiator.edit(|state| {
                let Some(row) = state.rows.get_mut(row_index) else {
                    return;
                };
                match action {
                    0 => row.settings.custom.rotate_left(1),
                    1 => row.settings.custom.rotate_right(1),
                    2 => row.settings.custom.reverse(),
                    _ => row.settings.custom = default_custom(),
                }
            });
            cx.needs_redraw();
            return true;
        }
        for index in 0..CUSTOM_STEPS {
            for field in FIELDS {
                let r = field_rect(index, field);
                if hit(r, x, y) {
                    let delta = if x < r.0 + r.2 * 0.5 { -1 } else { 1 };
                    self.params.arpeggiator.edit(|state| {
                        if let Some(row) = state.rows.get_mut(row_index) {
                            adjust(&mut row.settings.custom[index], index, field, delta, reset);
                        }
                    });
                    cx.needs_redraw();
                    return true;
                }
            }
        }
        true
    }

    pub(super) fn scroll_custom(&mut self, x: f32, y: f32, dy: f32) -> bool {
        if !hit(CHORDS_SURFACE, x, y) {
            return false;
        }
        if dy == 0.0 {
            return true;
        }
        for index in 0..CUSTOM_STEPS {
            for field in FIELDS {
                if hit(field_rect(index, field), x, y) {
                    let row_index = self.arp_ui.edit_page;
                    self.params.arpeggiator.edit(|state| {
                        if let Some(row) = state.rows.get_mut(row_index) {
                            adjust(
                                &mut row.settings.custom[index],
                                index,
                                field,
                                if dy > 0.0 { 1 } else { -1 },
                                false,
                            );
                        }
                    });
                    return true;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_edits_and_actions_stay_with_the_selected_row() {
        let mut view = ChordboardView::new(
            Arc::new(ChordboardParams::default()),
            Arc::new(Bridge::default()),
            Arc::new(PreviewContext),
        );
        view.params.arpeggiator.edit(|state| {
            state.add_row_after(0);
        });
        view.arp_ui.edit_page = 1;
        let mut cx = Context::default();
        let target = Element::new(&mut cx).entity();
        let mut cx = EventContext::new_with_current(&mut cx, target);
        let r = field_rect(0, Field::Degree);
        view.press_custom(&mut cx, r.0 + r.2 - 2.0, r.1 + 2.0, false);
        let r = field_rect(0, Field::Velocity);
        view.scroll_custom(r.0 + 2.0, r.1 + 2.0, -1.0);
        let state = view.params.arpeggiator.snapshot();
        assert_eq!(state.rows[0].settings.custom, default_custom());
        assert_eq!(
            (
                state.rows[1].settings.custom[0].degree,
                state.rows[1].settings.custom[0].velocity
            ),
            (2, 95)
        );
        let r = action_rect(1);
        view.press_custom(&mut cx, r.0 + 2.0, r.1 + 2.0, false);
        assert_eq!(
            view.params.arpeggiator.snapshot().rows[1].settings.custom[1],
            state.rows[1].settings.custom[0]
        );
        let r = action_rect(3);
        view.press_custom(&mut cx, r.0 + 2.0, r.1 + 2.0, false);
        assert_eq!(
            view.params.arpeggiator.snapshot().rows[1].settings.custom,
            default_custom()
        );
    }

    #[test]
    fn custom_controls_respect_bounds_and_reset_to_neutral() {
        let mut step = CustomStep {
            degree: 7,
            octave: 4,
            rest: true,
            velocity: 200,
            gate: 1,
        };
        adjust(&mut step, 2, Field::Degree, 1, false);
        adjust(&mut step, 2, Field::Octave, 1, false);
        adjust(&mut step, 2, Field::Velocity, 1, false);
        adjust(&mut step, 2, Field::Gate, -1, false);
        assert_eq!(
            (step.degree, step.octave, step.velocity, step.gate),
            (7, 4, 200, 1)
        );
        for field in FIELDS {
            adjust(&mut step, 2, field, -1, true);
        }
        assert_eq!(step, default_custom()[2]);
    }

    #[test]
    fn custom_targets_fit_the_chord_panel_without_covering_close() {
        let right = CHORDS_SURFACE.0 + CHORDS_SURFACE.2;
        for r in custom_button_rects() {
            assert!(r.0 >= CHORDS_SURFACE.0 && r.0 + r.2 <= right);
            assert!(r.1 >= CHORDS_SURFACE.1 && r.1 + r.3 < CHORDS_SURFACE.1 + CHORDS_SURFACE.3);
        }
        assert!(action_rect(3).0 + action_rect(3).2 < right - 48.0);
        for index in 0..CUSTOM_STEPS - 1 {
            let a = field_rect(index, Field::Degree);
            let b = field_rect(index + 1, Field::Degree);
            assert!(a.0 + a.2 < b.0);
        }
    }
}
