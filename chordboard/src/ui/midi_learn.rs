use super::*;
use crate::midi_learn::{self as midi, COUNT, IDS, LABELS};
pub(super) const VISIBLE_TARGETS: [usize; 9] = [1, 4, 5, 6, 7, 8, midi::VOICING_OVERRIDE, midi::VELOCITY, midi::ALWAYS_CHORD];

fn target_label(index: usize) -> &'static str {
    match index {
        7 => "Arp Row",
        midi::BASE_INVERSION => "Inversion",
        midi::VELOCITY_SCALE => "Velocity",
        _ => LABELS[index],
    }
}

pub(super) struct QuickMidiUi {
    pub show: bool,
    pub progress: f32,
    start: f32,
    elapsed: f32,
    signature: [u32; COUNT],
    armed: u32,
}

impl Default for QuickMidiUi {
    fn default() -> Self {
        Self {
            show: false,
            progress: 0.0,
            start: 0.0,
            elapsed: pleasant_ui::page_slide::DURATION,
            signature: [0; COUNT],
            armed: 0,
        }
    }
}

pub(super) fn switch_rect() -> Rect {
    (
        MOD_SURFACE.0 + MOD_SURFACE.2 - 38.0,
        MOD_SURFACE.1 + 8.0,
        28.0,
        28.0,
    )
}

pub(super) fn exit_rect() -> Rect {
    let r = switch_rect();
    (r.0 - 26.0, r.1, r.2 + 26.0, r.3)
}

pub(super) fn chip_rect(index: usize) -> Rect {
    let Some(position) = VISIBLE_TARGETS.iter().position(|&target| target == index) else {
        return (-100.0, -100.0, 0.0, 0.0);
    };
    let count = VISIBLE_TARGETS.len();
    let height = (MOD_SURFACE.3 - MODULE_HEADER_H - 24.0 - (count - 1) as f32 * 8.0) / count as f32;
    (
        MOD_SURFACE.0 + 12.0,
        MOD_SURFACE.1 + MODULE_HEADER_H + 12.0 + position as f32 * (height + 8.0),
        MOD_SURFACE.2 - 24.0,
        height,
    )
}

pub(super) fn clear_rect(index: usize) -> Rect {
    let r = chip_rect(index);
    (r.0 + r.2 - 24.0, r.1 + r.3 - 25.0, 22.0, 22.0)
}

pub(super) fn mode_rect(index: usize) -> Rect {
    let r = chip_rect(index);
    (r.0 + r.2 - 56.0, r.1 + 26.0, 50.0, 18.0)
}

impl ChordboardView {
    /// Shared placement keeps the learn overlay and its pointer targets aligned.
    pub(super) fn surface_midi_targets(&self) -> Vec<(usize, Rect)> {
        let mut targets: Vec<_> = self.base_controls().into_iter()
            .filter(|(c, _)| !globals::is_global(c.id))
            .filter_map(|(c, r)| IDS.iter().position(|id| *id == c.id).map(|i| (i, r)))
            .collect();
        targets.extend([
            (midi::DAW_ALIGNMENT, DAW_ALIGNMENT),
            (midi::ALWAYS_BASS, ALWAYS_BASS),
            (midi::AFFECT_CHORDS, affect_chords_rect(self.params.mpe_enabled())),
            (midi::KEY_SPLIT, key_split_rect(self.params.mpe_enabled())),
            (midi::SPLIT_NOTE, melody_split_rect(self.params.mpe_enabled())),
            (midi::BASS_ENABLED, BASS_BYPASS),
            (midi::STRUM_ENABLED, strum_bypass_rect(self.expand_t())),
            (midi::OCTAVES, strum_octaves_rect(self.expand_t())),
            (1, self.voice_leading_rect()),
            (5, LATCH),
            (6, arp_power_rect()),
        ]);
        // The expanded performance surface covers the chord chooser and latch.
        if self.expand_t() > 0.0 {
            targets.retain(|(i, _)| !matches!(*i, midi::ROOT_ON_SELECT | 5));
        }
        if let Some(r) = self.voicing_bounds() { targets.push((0, r)); }
        targets
    }

    pub(super) fn draw_surface_midi_targets(&self, d: &mut Draw) {
        for (target, r) in self.surface_midi_targets() {
            let armed = self.bridge.quick_learn.load(Ordering::Acquire) == target as u32 + 1;
            let assigned = self.params.quick_midi[target].mapping.load(Ordering::Acquire) != 0;
            d.rounded_rect(r.0, r.1, r.2, r.3, 4.0, alpha(GOLD, if armed { 0.24 } else { 0.10 }));
            d.outline_rounded(r.0, r.1, r.2, r.3, 4.0,
                alpha(GOLD, if armed || assigned { 1.0 } else { 0.65 }), if armed { 2.0 } else { 1.0 });

        }
    }

    pub(super) fn press_surface_midi_target(&mut self, cx: &mut EventContext, x: f32, y: f32, clear: bool) {
        if let Some((target, _)) = self.surface_midi_targets().into_iter().find(|(_, r)| hit(*r, x, y)) {
            let armed = self.bridge.quick_learn.load(Ordering::Acquire) == target as u32 + 1;
            if clear {
                self.params.quick_midi[target].mapping.store(0, Ordering::Release);
                if armed { self.bridge.quick_learn.store(0, Ordering::Release); }
            } else {
                self.request_learn(0);
                self.bridge.quick_learn.store(if armed { 0 } else { target as u32 + 1 }, Ordering::Release);
            }
            cx.needs_redraw();
        }
    }

    pub(super) fn toggle_quick_midi(&mut self, cx: &mut EventContext) {
        self.end_drag(cx);
        self.restore_shift_latch(cx);
        self.end_pad_hover(cx);
        self.release_keys();
        self.menu = None;
        self.edit = None;
        self.tracker_selection = false;
        self.tracker_symbol_edit = false;
        self.cancel_arp_editor();
        self.set_panel(None);
        self.bridge.quick_learn.store(0, Ordering::Release);
        self.quick_midi.start = self.quick_midi.progress;
        self.quick_midi.elapsed = 0.0;
        self.quick_midi.show = !self.quick_midi.show;
        self.route_hover = None;
        cx.needs_redraw();
    }

    pub(super) fn press_quick_midi(
        &mut self,
        cx: &mut EventContext,
        x: f32,
        y: f32,
        clear: bool,
    ) -> bool {
        if !hit(MOD_SURFACE, x, y) {
            return false;
        }
        if hit(if self.quick_midi.show { exit_rect() } else { switch_rect() }, x, y) {
            if !clear {
                self.toggle_quick_midi(cx);
            }
            return true;
        }
        // Neither page accepts stale hit targets while it is sliding.
        if self.quick_midi.elapsed < pleasant_ui::page_slide::DURATION {
            return true;
        }
        if !self.quick_midi.show {
            return false;
        }
        if let Some(target) = VISIBLE_TARGETS
            .into_iter()
            .find(|&i| hit(chip_rect(i), x, y))
        {
            let binding = &self.params.quick_midi[target];
            let mapping = crate::engine::Mapping::decode(binding.mapping.load(Ordering::Acquire));
            let armed = self.bridge.quick_learn.load(Ordering::Acquire) == target as u32 + 1;
            if clear || ((mapping.kind != 0 || armed) && hit(clear_rect(target), x, y)) {
                self.params.quick_midi[target]
                    .mapping
                    .store(0, Ordering::Release);
                if self.bridge.quick_learn.load(Ordering::Acquire) == target as u32 + 1 {
                    self.bridge.quick_learn.store(0, Ordering::Release);
                }
            } else if target != midi::ROOT
                && mapping.kind == 1
                && !armed
                && hit(mode_rect(target), x, y)
            {
                binding.step_cc.fetch_xor(true, Ordering::AcqRel);
            } else {
                self.request_learn(0);
                let id = target as u32 + 1;
                let next = if self.bridge.quick_learn.load(Ordering::Acquire) == id {
                    0
                } else {
                    id
                };
                self.bridge.quick_learn.store(next, Ordering::Release);
            }
            cx.needs_redraw();
        }
        true
    }

    pub(super) fn tick_quick_midi(&mut self, cx: &mut EventContext, dt: f32) -> bool {
        let mut changed = self.quick_midi.elapsed < pleasant_ui::page_slide::DURATION;
        self.quick_midi.elapsed =
            (self.quick_midi.elapsed + dt).min(pleasant_ui::page_slide::DURATION);
        self.quick_midi.progress = pleasant_ui::page_slide::position(
            self.quick_midi.start,
            if self.quick_midi.show { 1.0 } else { 0.0 },
            self.quick_midi.elapsed,
        );
        let armed = self.bridge.quick_learn.load(Ordering::Acquire);
        changed |= armed != self.quick_midi.armed;
        self.quick_midi.armed = armed;
        for i in 0..COUNT {
            let signature = self.params.quick_midi[i].mapping.load(Ordering::Acquire)
                | (self.params.quick_midi[i].step_cc.load(Ordering::Relaxed) as u32) << 24;
            changed |= signature != self.quick_midi.signature[i];
            self.quick_midi.signature[i] = signature;
            if i == midi::ROOT {
                continue;
            }
            let Some(id) = IDS.get(i) else {
                continue;
            };
            let desired = midi::value(&self.params, i);
            if desired != midi::base(&self.params, i) {
                if let Some(control) = self.params.control(id) {
                    Self::emit(cx, control.ptr, midi::normalized(i, desired));
                    changed = true;
                }
            }
        }
        changed
    }

    pub(super) fn draw_modulator_pages(&self, d: &mut Draw) {
        let r = MOD_SURFACE;
        d.scissor(r.0, r.1, r.2, r.3);
        let progress = self.quick_midi.progress;
        if progress < 1.0 {
            d.offset_x = -progress * r.2;
            self.draw_meters(d);
            self.icon_button(
                d,
                switch_rect(),
                "",
                self.quick_midi.show,
                GOLD,
                style::Icon::MidiDin,
            );
        }
        if progress > 0.0 {
            d.offset_x = (1.0 - progress) * r.2;
            self.draw_quick_midi(d);
        }
        d.offset_x = 0.0;
        d.reset_scissor();
    }

    fn draw_quick_midi(&self, d: &mut Draw) {
        let icons = [
            style::Icon::Glyph("\u{ea03}"),
            style::Icon::Host,
            style::Icon::Glyph("\u{eb5b}"),
            style::Icon::TuningFork,
            style::Icon::Learn,
            style::Icon::Lock,
            style::Icon::Once,
            style::Icon::Loop,
            style::Icon::Touch,
        ];
        d.font = self.font.get();
        d.text(
            MOD_SURFACE.0 + 10.0,
            module_title_y(MOD_SURFACE.1, MODULE_TITLE_SIZE),
            "LEARN",
            MODULE_TITLE_SIZE,
            GOLD,
        );
        self.button(d, exit_rect(), "Done", false, GOLD);
        for i in VISIBLE_TARGETS {
            let label = target_label(i);
            let r = chip_rect(i);
            let mapping = crate::engine::Mapping::decode(
                self.params.quick_midi[i].mapping.load(Ordering::Acquire),
            );
            let armed = self.bridge.quick_learn.load(Ordering::Acquire) == i as u32 + 1;
            let hover = self.hover_amount(r);
            let assigned = mapping.kind != 0;
            d.rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                4.0,
                alpha(GOLD, if armed { 0.16 } else { 0.025 + hover * 0.12 }),
            );
            d.outline_rounded(
                r.0,
                r.1,
                r.2,
                r.3,
                4.0,
                alpha(
                    if assigned || armed || hover > 0.0 {
                        GOLD
                    } else {
                        LINE
                    },
                    0.55 + hover * 0.4,
                ),
                if armed { 2.0 } else { 1.0 },
            );
            icons.get(i).copied().unwrap_or(style::Icon::Learn).draw(d, r.0 + 15.0, r.1 + 16.0, GOLD);
            d.text(r.0 + 29.0, r.1 + 20.0, label, TEXT_SMALL, GOLD);
            let source = if armed {
                "Play note / CC…".into()
            } else if mapping.kind == midi::NOTE {
                format!("Note {} · Ch {}", mapping.number, mapping.channel + 1)
            } else if mapping.kind == 1 {
                format!("CC{} Ch{}", mapping.number, mapping.channel + 1)
            } else {
                "Unassigned".into()
            };
            d.text(
                r.0 + 8.0,
                r.1 + 39.0,
                &source,
                if mapping.kind == 1 && !armed {
                    10.0
                } else {
                    TEXT_SMALL
                },
                if armed { GOLD } else { MUTED },
            );
            if mapping.kind == 1 && !armed && i != midi::ROOT {
                let step = self.params.quick_midi[i].step_cc.load(Ordering::Relaxed);
                self.button(
                    d,
                    mode_rect(i),
                    if step { "STEP" } else { "VALUE" },
                    step,
                    GOLD,
                );
            }
            if i == midi::ROOT {
                d.text(r.0 + 8.0, r.1 + r.3 - 9.0, "Hold + play notes", 10.0, TEXT);
            } else if let Some(control) = self.params.control(IDS[i]) {
                let value = match i {
                    2 => self.snapshot.inversion.to_string(),
                    4 => harmony::QUALITY_NAMES[self.snapshot.quality.min(11) as usize].into(),
                    _ => control.value,
                };
                let text = self.fit_text(d, &value, r.2 - 38.0, TEXT_SMALL);
                d.text(r.0 + 8.0, r.1 + r.3 - 9.0, &text, TEXT_SMALL, TEXT);
            }
            if assigned || armed {
                self.close_icon(d, clear_rect(i), GOLD, false);
            }
        }
        d.font = self.ui_font.get();
    }

    pub(super) fn midi_learn_status(&self) -> String {
        let armed = self.bridge.quick_learn.load(Ordering::Acquire);
        if let Some(index) = armed.checked_sub(1).filter(|i| (*i as usize) < COUNT) {
            let label = target_label(index as usize);
            format!("MIDI Learn · Listening for {} · Play a note or move a CC", label)
        } else if let Some((target, _)) = self.pointer.and_then(|(x, y)|
            self.surface_midi_targets().into_iter().find(|(_, r)| hit(*r, x, y))) {
            let mapping = crate::engine::Mapping::decode(self.params.quick_midi[target].mapping.load(Ordering::Acquire));
            let source = match mapping.kind {
                midi::NOTE => format!("Note {} · Ch {}", mapping.number, mapping.channel + 1),
                1 => format!("CC{} · Ch {}", mapping.number, mapping.channel + 1),
                _ => "Unassigned".into(),
            };
            format!("{} · {} · Click to learn · Right-click to clear", target_label(target), source)
        } else {
            "MIDI Learn · Select a highlighted control".into()
        }
    }

    pub(super) fn quick_midi_hint(&self, x: f32, y: f32) -> Option<String> {
        if hit(if self.quick_midi.show { exit_rect() } else { switch_rect() }, x, y) {
            return Some(
                if self.quick_midi.show {
                    "Finish MIDI learn"
                } else {
                    "Open MIDI learn"
                }
                .into(),
            );
        }
        if !self.quick_midi.show || !hit(MOD_SURFACE, x, y) {
            return None;
        }
        VISIBLE_TARGETS.into_iter().find(|&i| hit(chip_rect(i), x, y)).map(|i| {
            if i == midi::ROOT {
                "Change Root · Learn a note or CC · Hold it, then play a root and optional alteration/extension notes · Selection remains after release · × or right-click clears".into()
            } else {
                format!("{} · Click, then play a note or move a CC · Notes step/toggle · CC: VALUE for knobs, STEP for pads · Click again or Escape cancels · × or right-click clears", target_label(i))
            }
        })
    }
}
