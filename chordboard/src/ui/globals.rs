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
    let width = (GLOBAL_SURFACE.2 - 24.0) / GLOBAL_IDS.len() as f32;
    (
        GLOBAL_SURFACE.0 + 12.0 + width * index as f32,
        GLOBAL_SURFACE.1 + 38.0,
        width,
        140.0,
    )
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
    layout.radius = 42.0;
    layout.cy = rect.1 + 70.0;
    layout.label_y = rect.1 + 16.0;
    layout.value_y = rect.1 + 128.0;
    layout.with_text_sizes(13.0, 16.0)
}

pub(super) fn global_controls() -> Vec<(&'static str, Rect)> {
    GLOBAL_IDS
        .into_iter()
        .map(|id| (id, global_rect(id)))
        .collect()
}

pub(super) fn learn_rect(index: usize) -> Rect {
    let r = slot_rect(index);
    (r.0 + 4.0, r.1, r.2 - 8.0, 162.0)
}

fn mapping_rect(index: usize) -> Rect {
    let r = learn_rect(index);
    {
        let width = (r.2 - 16.0).min(220.0);
        (r.0 + (r.2 - width) * 0.5, GLOBAL_SURFACE.1 + 176.0, width, 24.0)
    }
}

fn mode_rect(index: usize) -> Rect {
    let r = mapping_rect(index);
    (r.0 + r.2 - 78.0, r.1 + 2.0, 50.0, r.3 - 4.0)
}

fn clear_rect(index: usize) -> Rect {
    let r = mapping_rect(index);
    (r.0 + r.2 - 24.0, r.1 + 1.0, 22.0, 22.0)
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
                effective.norm,
                color,
                false,
            );
        }
        if routed.is_some() {
            self.live_highlight(d, rect);
        }
    }

    pub(super) fn draw_globals(&self, d: &mut Draw) {
        self.surface(d, GLOBAL_SURFACE);
        d.font = self.ui_font.get();
        if !self.quick_midi.show {
        d.text(
            GLOBAL_SURFACE.0 + 14.0,
            module_title_y(GLOBAL_SURFACE.1, MODULE_TITLE_SIZE),
            "GLOBAL",
            MODULE_TITLE_SIZE,
            TEXT,
        );
        }
        self.draw_global_targets(d);
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
            let reserved = if mapping.kind == 1 && !armed {
                84.0
            } else if mapping.kind != 0 || armed {
                30.0
            } else {
                8.0
            };
            let label = self.fit_text(d, &label, r.2 - reserved - 8.0, 10.0);
            d.text_middle(
                if mapping.kind == 0 && !armed { r.0 + (r.2 - d.text_width(&label, 10.0)) * 0.5 } else { r.0 + 8.0 },
                r.1 + r.3 * 0.5,
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
        if !hit(GLOBAL_SURFACE, x, y) {
            return false;
        }
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
            assert!(learn.1 + learn.3 <= GLOBAL_SURFACE.1 + GLOBAL_SURFACE.3);
            assert!(control.0 >= GLOBAL_SURFACE.0);
            assert!(control.0 + control.2 <= GLOBAL_SURFACE.0 + GLOBAL_SURFACE.2);
            {
                let value = value_rect(id);
                assert!(value.1 >= control.1 && value.1 + value.3 <= control.1 + control.3);
            }
        }
        assert_eq!(TARGETS, [9, 10, 3, 12, 14]);
    }
}
