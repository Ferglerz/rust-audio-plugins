//! Global performance controls; the whole control becomes a MIDI-learn target.
use super::*;
use crate::midi_learn as midi;
use pleasant_ui::draw::KnobLayout;

pub(super) const GLOBAL_IDS: [&str; 5] = [
    "base_inversion",
    "complexity",
    "transpose",
    "gate_scale",
    "velocity_scale",
];
const LABELS: [&str; 5] = ["Inversion", "Complexity", "Transpose", "Gate", "Velocity"];
const TARGETS: [usize; 5] = [
    midi::BASE_INVERSION,
    midi::COMPLEXITY,
    3,
    midi::GATE_SCALE,
    midi::VELOCITY_SCALE,
];

fn slot_rect(index: usize) -> Rect {
    let x = PIANO_SURFACE.0 + 6.0;
    match index {
        0 => (x, PIANO_SURFACE.1 + 46.0, 74.0, 112.0),
        1 => (x + 74.0, PIANO_SURFACE.1 + 8.0, 160.0, 170.0),
        2 => (x + 234.0, PIANO_SURFACE.1 + 46.0, 74.0, 112.0),
        _ => (
            GLOBAL_SURFACE.0 + 12.0,
            GLOBAL_SURFACE.1 + 48.0 + (4 - index) as f32 * 148.0,
            120.0,
            100.0,
        ),
    }
}

pub(super) fn is_global(id: &str) -> bool {
    GLOBAL_IDS.contains(&id)
}

pub(super) fn global_rect(id: &str) -> Rect {
    let index = GLOBAL_IDS
        .iter()
        .position(|candidate| *candidate == id)
        .unwrap_or(0);
    slot_rect(index)
}

pub(super) fn value_rect(id: &str) -> Rect {
    let r = global_rect(id);
    knob_layout(r).value_rect(r.0 + 12.0, r.2 - 24.0)
}

fn knob_layout(rect: Rect) -> KnobLayout {
    let mut layout = KnobLayout::new(rect);
    let complexity = rect == global_rect("complexity");
    layout.radius = if complexity { 30.0 } else { 26.0 };
    layout.cy = rect.1 + if complexity { 86.0 } else { 52.0 };
    layout.label_y = rect.1 + 12.0;
    layout.value_y = rect.1 + rect.3 - 10.0;
    layout.with_text_sizes(TEXT_LABEL, TEXT_LABEL)
}

/// Resolve the very same recipe as playback, in a safe middle register. This
/// keeps the extension depictions correct for altered, suspended and sixth chords.
fn complexity_steps(chord: SavedChord, global: harmony::GlobalHarmony) -> [(String, Vec<u8>); 6] {
    let recipe = SavedChord {
        root: 60 + chord.root % 12,
        inversion: 0,
        spread: 0,
        transpose: 0,
        ..chord
    };
    let mut previous = Vec::new();
    std::array::from_fn(|level| {
        let notes = harmony::resolve_chord(
            recipe,
            harmony::GlobalHarmony {
                complexity: level as u8,
                inversion: 0,
                transpose: 0,
                voicing: 1,
                ..global
            },
        );
        let pitches: Vec<_> = notes.as_slice().iter().map(|note| note % 12).collect();
        let added: Vec<_> = pitches
            .iter()
            .copied()
            .filter(|pc| level <= 1 || !previous.contains(pc))
            .collect();
        let label = match level {
            0 => "Saved".into(),
            1 => if chord.quality == 11 { "Power" } else { "Base" }.into(),
            _ => added.first().map_or_else(
                || "Same".into(),
                |pc| {
                    let interval = (*pc as i16 - recipe.root as i16).rem_euclid(12);
                    match (level, interval) {
                        (2, 9) => if matches!(chord.quality, 5 | 9) {
                            "bb7"
                        } else {
                            "6"
                        }
                        .into(),
                        (2, 10) => "b7".into(),
                        (2, _) => "maj7".into(),
                        (3, 1) => "b9".into(),
                        (3, 3) => "#9".into(),
                        (3, _) => "9".into(),
                        (4, 6) => "#11".into(),
                        (4, _) => "11".into(),
                        (5, 8) => "b13".into(),
                        (5, _) => "13".into(),
                        _ => unreachable!(),
                    }
                },
            ),
        };
        previous = pitches;
        let added = added
            .into_iter()
            .map(|pc| {
                (pc as i16 + chord.transpose as i16 + global.transpose as i16).rem_euclid(12) as u8
            })
            .collect();
        (label, added)
    })
}

pub(super) fn global_controls() -> Vec<(&'static str, Rect)> {
    GLOBAL_IDS
        .into_iter()
        .map(|id| (id, global_rect(id)))
        .collect()
}

pub(super) fn learn_rect(index: usize) -> Rect {
    let r = slot_rect(index);
    (
        r.0 + 4.0,
        r.1,
        r.2 - 8.0,
        r.3 + if index == 1 { 28.0 } else { 48.0 },
    )
}

fn mapping_rect(index: usize) -> Rect {
    let r = learn_rect(index);
    {
        let width = r.2;
        (
            r.0 + (r.2 - width) * 0.5,
            slot_rect(index).1 + slot_rect(index).3 + 2.0,
            width,
            if index == 1 { 24.0 } else { 44.0 },
        )
    }
}

fn mode_rect(index: usize) -> Rect {
    let r = mapping_rect(index);
    (
        r.0 + if index == 1 { r.2 - 78.0 } else { 0.0 },
        r.1 + r.3 - 22.0,
        (r.2 - 26.0).min(50.0),
        20.0,
    )
}

fn clear_rect(index: usize) -> Rect {
    let r = mapping_rect(index);
    (r.0 + r.2 - 24.0, r.1 + r.3 - 23.0, 22.0, 22.0)
}

impl ChordboardView {
    pub(super) fn draw_global_control(&self, d: &mut Draw, control: &Control, rect: Rect) {
        let index = GLOBAL_IDS
            .iter()
            .position(|id| *id == control.id)
            .unwrap_or(0);
        let routed = self.routed_control(control);
        let effective = routed.as_ref().unwrap_or(control);
        let color = if routed.is_some() { TEAL } else { GOLD };
        d.font = self.ui_font.get();
        let layout = knob_layout(rect);
        if control.id == "complexity" {
            self.draw_complexity_ring(d, &layout, effective.norm, color);
        }
        if matches!(control.id, "base_inversion" | "transpose") {
            d.knob_with_layout_bipolar(
                &layout,
                LABELS[index],
                &self.display_value(effective),
                effective.norm,
                color,
                false,
            );
        } else {
            d.knob_with_layout(
                &layout,
                LABELS[index],
                &self.display_value(effective),
                if control.id == "complexity" {
                    ((effective.norm * 5.0).round() + 0.5) / 6.0
                } else {
                    effective.norm
                },
                color,
                false,
            );
        }
        if routed.is_some() {
            self.live_highlight(d, rect);
        }
    }

    fn draw_complexity_ring(&self, d: &mut Draw, layout: &KnobLayout, norm: f32, color: Color) {
        let chord = SavedChord::decode(self.snapshot.captured)
            .or_else(|| self.params.arpeggiator.snapshot().rows[self.arp_ui.edit_page].chord);
        let steps = chord.map(|chord| {
            complexity_steps(
                chord,
                harmony::GlobalHarmony {
                    key: self.params.key.value() as u8,
                    scale: self.params.scale.value() as u8,
                    ..self.snapshot.global_harmony
                },
            )
        });
        let selected = (norm * 5.0).round() as usize;
        for i in 0..6 {
            // Six filled wedges around the smaller knob; gaps make every step distinct.
            let start = (135.0 + i as f32 * 45.0 + 2.5).to_radians();
            let end = start + 40.0_f32.to_radians();
            let points: Vec<_> = (0..=16)
                .map(|n| {
                    let a = start + (end - start) * n as f32 / 16.0;
                    (layout.cx + 55.0 * a.cos(), layout.cy + 55.0 * a.sin())
                })
                .collect();
            d.poly(
                &points,
                alpha(
                    color,
                    if i == selected {
                        0.92
                    } else if i <= selected && selected > 0 {
                        0.30
                    } else {
                        0.10
                    },
                ),
                26.0,
            );
            let a = (start + end) * 0.5;
            let x = layout.cx + 55.0 * a.cos();
            let y = layout.cy + 55.0 * a.sin();
            let ink = if i == selected { BG } else { TEXT };
            let label = steps
                .as_ref()
                .map_or(["Saved", "Base", "7", "9", "11", "13"][i], |s| {
                    s[i].0.as_str()
                });
            d.text_centered(x, y - 1.0, &music::label(label), 9.0, ink);
            if let Some(steps) = &steps {
                let notes = &steps[i].1;
                if i >= 2 && notes.len() == 1 {
                    d.text_centered(
                        x,
                        y + 10.0,
                        &music::label(&self.note_name(notes[0])),
                        9.0,
                        ink,
                    );
                } else {
                    let count = notes.len().min(7);
                    for n in 0..count {
                        d.circle(
                            x + (n as f32 - (count as f32 - 1.0) * 0.5) * 4.0,
                            y + 7.0,
                            1.2,
                            ink,
                            true,
                        );
                    }
                }
            }
        }
    }

    pub(super) fn draw_global_targets(&self, d: &mut Draw) {
        for (index, id) in GLOBAL_IDS.into_iter().enumerate() {
            if let Some(control) = self.control(id) {
                self.draw_global_control(d, &control, global_rect(id));
            }
            if !self.quick_midi.show {
                continue;
            }
            let target = TARGETS[index];
            let binding = &self.params.quick_midi[target];
            let mapping = crate::engine::Mapping::decode(binding.mapping.load(Ordering::Acquire));
            let armed = self.bridge.quick_learn.load(Ordering::Acquire) == target as u32 + 1;
            let label = if armed {
                "Play note / CC…".into()
            } else {
                match mapping.kind {
                    midi::NOTE => format!("Note {} · Ch {}", mapping.number, mapping.channel + 1),
                    1 => format!("CC{} Ch{}", mapping.number, mapping.channel + 1),
                    _ => "Unassigned".into(),
                }
            };
            // Color the entire knob face, not a separate Learn button.
            {
                let layout = knob_layout(global_rect(id));
                d.circle(
                    layout.cx,
                    layout.cy,
                    layout.radius,
                    alpha(GOLD, if armed { 0.24 } else { 0.10 }),
                    true,
                );
                d.circle(layout.cx, layout.cy, layout.radius + 5.0, GOLD, false);
            }
            let r = mapping_rect(index);
            let reserved = if index != 1 {
                0.0
            } else if mapping.kind == 1 && !armed {
                84.0
            } else if mapping.kind != 0 || armed {
                30.0
            } else {
                8.0
            };
            let label = self.fit_text(d, &label, (r.2 - reserved - 8.0).max(0.0), 10.0);
            d.text_middle(
                if index != 1 || (mapping.kind == 0 && !armed) {
                    r.0 + (r.2 - d.text_width(&label, 10.0)) * 0.5
                } else {
                    r.0 + 8.0
                },
                r.1 + 12.0,
                &label,
                10.0,
                if armed { GOLD } else { MUTED },
            );
            if mapping.kind == 1 && !armed {
                let step = binding.step_cc.load(Ordering::Relaxed);
                self.button(
                    d,
                    mode_rect(index),
                    if step { "STEP" } else { "VALUE" },
                    step,
                    GOLD,
                );
            }
            if mapping.kind != 0 || armed {
                self.close_icon(d, clear_rect(index), GOLD, false);
            }
        }
    }

    pub(super) fn press_global(
        &mut self,
        cx: &mut EventContext,
        x: f32,
        y: f32,
        clear: bool,
    ) -> bool {
        if self.quick_midi.show {
            if let Some(index) = (0..GLOBAL_IDS.len()).find(|&index| hit(learn_rect(index), x, y)) {
                let target = TARGETS[index];
                let binding = &self.params.quick_midi[target];
                let mapping =
                    crate::engine::Mapping::decode(binding.mapping.load(Ordering::Acquire));
                let armed = self.bridge.quick_learn.load(Ordering::Acquire) == target as u32 + 1;
                if clear || ((mapping.kind != 0 || armed) && hit(clear_rect(index), x, y)) {
                    binding.mapping.store(0, Ordering::Release);
                    if armed {
                        self.bridge.quick_learn.store(0, Ordering::Release);
                    }
                } else if mapping.kind == 1 && !armed && hit(mode_rect(index), x, y) {
                    binding.step_cc.fetch_xor(true, Ordering::AcqRel);
                } else {
                    self.request_learn(0);
                    self.bridge
                        .quick_learn
                        .store(if armed { 0 } else { target as u32 + 1 }, Ordering::Release);
                }
                cx.needs_redraw();
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complexity_depictions_follow_chord_quality_and_alterations() {
        for (symbol, expected) in [
            ("C", ["maj7", "9", "11", "13"]),
            ("Cm", ["b7", "9", "11", "13"]),
            ("C6", ["6", "9", "11", "Same"]),
            ("Cdim7", ["bb7", "9", "11", "Same"]),
            ("C7(addb9)", ["b7", "b9", "11", "13"]),
            ("C7(add#11)", ["b7", "9", "#11", "13"]),
        ] {
            let chord = harmony::parse_chord_symbol(symbol).unwrap();
            let steps = complexity_steps(chord, harmony::GlobalHarmony::default());
            assert_eq!(
                steps[2..].iter().map(|s| s.0.as_str()).collect::<Vec<_>>(),
                expected,
                "{symbol}"
            );
        }
        let c = harmony::parse_chord_symbol("C").unwrap();
        let steps = complexity_steps(c, harmony::GlobalHarmony::default());
        assert_eq!(steps[2].1, vec![11]);
        assert_eq!(steps[3].1, vec![2]);
        let transposed = complexity_steps(
            SavedChord { transpose: 2, ..c },
            harmony::GlobalHarmony {
                transpose: -1,
                ..Default::default()
            },
        );
        assert_eq!(transposed[2], ("maj7".into(), vec![0]));
        assert_eq!(transposed[3], ("9".into(), vec![3]));
        let lydian = complexity_steps(
            c,
            harmony::GlobalHarmony {
                key: 7,
                ..Default::default()
            },
        );
        assert_eq!(lydian[4], ("#11".into(), vec![6]));
    }

    #[test]
    fn global_learning_covers_controls_and_preserves_mapping_actions() {
        let mut view = ChordboardView::new(
            Arc::new(ChordboardParams::default()),
            Arc::new(Bridge::default()),
            Arc::new(PreviewContext),
        );
        let mut cx = Context::default();
        let target = Element::new(&mut cx).entity();
        let mut cx = EventContext::new_with_current(&mut cx, target);
        view.quick_midi.show = true;
        for (index, target) in TARGETS.into_iter().enumerate() {
            let r = learn_rect(index);
            assert!(view.press_global(&mut cx, r.0 + 8.0, r.1 + 10.0, false));
            assert_eq!(
                view.bridge.quick_learn.load(Ordering::Acquire),
                target as u32 + 1
            );
            assert!(view.press_global(&mut cx, r.0 + 8.0, r.1 + 10.0, false));
            assert_eq!(view.bridge.quick_learn.load(Ordering::Acquire), 0);
        }
        let r = global_rect("velocity_scale");
        assert!(view.press_global(&mut cx, r.0 + 20.0, r.1 + 20.0, false));
        assert_eq!(
            view.bridge.quick_learn.load(Ordering::Acquire),
            midi::VELOCITY_SCALE as u32 + 1
        );
        view.bridge.quick_learn.store(0, Ordering::Release);
        let target = midi::VELOCITY_SCALE;
        view.params.quick_midi[target].mapping.store(
            crate::engine::Mapping::cc(21, 9).encode(),
            Ordering::Release,
        );
        let r = mode_rect(4);
        view.press_global(&mut cx, r.0 + 2.0, r.1 + 2.0, false);
        assert!(view.params.quick_midi[target]
            .step_cc
            .load(Ordering::Acquire));
        let r = clear_rect(4);
        view.press_global(&mut cx, r.0 + 2.0, r.1 + 2.0, false);
        assert_eq!(
            view.params.quick_midi[target]
                .mapping
                .load(Ordering::Acquire),
            0
        );
        view.quick_midi.show = false;
        assert!(!is_global("voicing_override"));
        assert!(!global_controls()
            .iter()
            .any(|(id, _)| *id == "voicing_override"));
    }

    #[test]
    fn global_learn_targets_cover_knob_centers_and_mapping_actions() {
        for (index, id) in GLOBAL_IDS.into_iter().enumerate() {
            let control = global_rect(id);
            let learn = learn_rect(index);
            assert!(hit(
                learn,
                control.0 + control.2 * 0.5,
                control.1 + control.3 * 0.5
            ));
            for action in [mode_rect(index), clear_rect(index)] {
                assert!(hit(learn, action.0, action.1));
                assert!(hit(learn, action.0 + action.2, action.1 + action.3));
            }
            let surface = if index < 3 {
                PIANO_SURFACE
            } else {
                GLOBAL_SURFACE
            };
            assert!(hit(surface, learn.0, learn.1));
            assert!(hit(surface, learn.0 + learn.2, learn.1 + learn.3));
            {
                let value = value_rect(id);
                assert!(value.1 >= control.1 && value.1 + value.3 <= control.1 + control.3);
            }
        }
        assert_eq!(TARGETS, [9, 10, 3, 12, 14]);
    }
}
