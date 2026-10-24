//! Performance bindings and a root-selection modifier, with allocation-free input.
use crate::{engine::Mapping, params::ChordboardParams};
use nih_plug::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub const CONTROL_COUNT: usize = 8;
pub const ROOT: usize = CONTROL_COUNT;
pub const COUNT: usize = CONTROL_COUNT + 1;
pub const IDS: [&str; CONTROL_COUNT] = [
    "spread",
    "voice_leading",
    "inversion",
    "transpose",
    "quality",
    "latch",
    "arp_enabled",
    "arp_page",
];
pub const LABELS: [&str; COUNT] = [
    "Voicing",
    "Voice Leading",
    "Inversion",
    "Transpose",
    "Chord Quality",
    "Latch",
    "Arp On/Off",
    "Arp Page",
    "Change Root",
];
const RANGES: [(i32, i32); CONTROL_COUNT] = [
    (0, crate::harmony::VOICING_NAMES.len() as i32 - 1),
    (0, 2),
    (0, 11),
    (-24, 24),
    (0, 11),
    (0, 1),
    (0, 1),
    (0, 7),
];
pub const NOTE: u8 = 3;

#[derive(Params, Default)]
pub struct Binding {
    #[persist = "quick-midi-map-v1"]
    pub mapping: AtomicU32,
    #[persist = "quick-midi-step-v1"]
    pub step_cc: AtomicBool,
    // Retain changes with the editor closed, until the GUI can notify the host.
    // Both values fit in one byte after biasing by 128; zero means no override.
    #[persist = "quick-midi-value-v1"]
    pub pending: AtomicU32,
}

impl Binding {
    pub fn value(&self, base: i32) -> i32 {
        let encoded = self.pending.load(Ordering::Acquire);
        if encoded == 0 {
            return base;
        }
        let previous = ((encoded >> 8) & 255) as i32 - 128;
        let desired = (encoded & 255) as i32 - 128;
        if base == desired || base != previous {
            // Acknowledgement or a new host/UI edit supersedes the MIDI value.
            let _ = self
                .pending
                .compare_exchange(encoded, 0, Ordering::AcqRel, Ordering::Acquire);
            base
        } else {
            desired
        }
    }

    fn set(&self, base: i32, desired: i32) {
        self.pending.store(
            1 << 16 | ((base + 128) as u32) << 8 | (desired + 128) as u32,
            Ordering::Release,
        );
    }
}

pub fn base(params: &ChordboardParams, target: usize) -> i32 {
    match target {
        0 => params.spread.value(),
        1 => params.voice_leading.value(),
        2 => params.inversion.value(),
        3 => params.transpose.value(),
        4 => params.quality.value(),
        5 => params.latch.value() as i32,
        6 => params.arp_enabled.value() as i32,
        _ => params.arp_page.value(),
    }
}

pub fn value(params: &ChordboardParams, target: usize) -> i32 {
    let (min, max) = RANGES[target];
    params.quick_midi[target]
        .value(base(params, target))
        .clamp(min, max)
}

pub fn normalized(target: usize, value: i32) -> f32 {
    let (min, max) = RANGES[target];
    (value.clamp(min, max) - min) as f32 / (max - min) as f32
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RootSelection {
    pub root: u8,
    pub second: Option<u8>,
    pub additions: u16,
}

pub struct Input {
    held: [bool; 2048],
    cc_high: [bool; 2048],
    root_gate: bool,
    root_mapping: u32,
    root_modifier: Option<usize>,
    selection_held: [bool; 2048],
    selection_down: usize,
    recipe: Option<RootSelection>,
    pub(crate) selection: Option<RootSelection>,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            held: [false; 2048],
            cc_high: [false; 2048],
            root_gate: false,
            root_mapping: 0,
            root_modifier: None,
            selection_held: [false; 2048],
            selection_down: 0,
            recipe: None,
            selection: None,
        }
    }
}

impl Input {
    /// Returns true when a controller event belongs to learning or a binding.
    pub fn consume(
        &mut self,
        event: NoteEvent<()>,
        params: &ChordboardParams,
        armed: &AtomicU32,
        harmony: [u8; 2],
    ) -> bool {
        let root_mapping = params.quick_midi[ROOT].mapping.load(Ordering::Acquire);
        if self.root_mapping != root_mapping {
            self.root_mapping = root_mapping;
            self.root_gate = false;
        }
        // Channel-mode messages retain their safety semantics and reset pad edges.
        if let NoteEvent::MidiCC {
            channel,
            cc: 120 | 123,
            ..
        } = event
        {
            if channel < 16 {
                let range = channel as usize * 128..(channel as usize + 1) * 128;
                self.held[range.clone()].fill(false);
                self.cc_high[range].fill(false);
            }
            self.root_gate = false;
            self.selection_held.fill(false);
            self.selection_down = 0;
            return false;
        }
        let (mapping, pressed, amount) = match event {
            NoteEvent::PolyPressure { channel, note, .. }
            | NoteEvent::PolyTuning { channel, note, .. }
            | NoteEvent::PolyBrightness { channel, note, .. }
                if channel < 16 && note < 128 =>
            {
                return self.held[channel as usize * 128 + note as usize];
            }
            NoteEvent::NoteOn {
                channel,
                note,
                velocity,
                ..
            } if channel < 16 && note < 128 && velocity.is_finite() => (
                Mapping {
                    kind: NOTE,
                    number: note,
                    channel,
                },
                velocity > 0.0,
                None,
            ),
            NoteEvent::NoteOff { channel, note, .. } if channel < 16 && note < 128 => (
                Mapping {
                    kind: NOTE,
                    number: note,
                    channel,
                },
                false,
                None,
            ),
            NoteEvent::MidiCC {
                channel, cc, value, ..
            } if channel < 16 && cc < 120 && value.is_finite() => {
                (Mapping::cc(cc, channel), true, Some(value.clamp(0.0, 1.0)))
            }
            _ => return false,
        };
        let note_id = mapping.channel as usize * 128 + mapping.number as usize;
        let cc_rising = if let Some(amount) = amount {
            let high = amount >= 0.5;
            let rising = high && !self.cc_high[note_id];
            self.cc_high[note_id] = high;
            rising
        } else {
            false
        };
        // Track ownership through unlearning/relearning so releases never leak.
        if mapping.kind == NOTE && self.held[note_id] {
            if !pressed {
                self.held[note_id] = false;
                if self.root_modifier == Some(note_id) {
                    self.root_gate = false;
                    self.root_modifier = None;
                }
                if self.selection_held[note_id] {
                    self.selection_held[note_id] = false;
                    self.selection_down = self.selection_down.saturating_sub(1);
                }
            }
            return true;
        }
        let target = armed.load(Ordering::Acquire);
        if pressed
            && (1..=COUNT as u32).contains(&target)
            && armed
                .compare_exchange(target, 0, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            let encoded = mapping.encode();
            for binding in &params.quick_midi {
                let _ = binding.mapping.compare_exchange(
                    encoded,
                    0,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                );
            }
            params.quick_midi[target as usize - 1]
                .mapping
                .store(encoded, Ordering::Release);
            if mapping.kind == NOTE {
                self.held[note_id] = true;
            }
            return true;
        }
        if root_mapping != 0 && mapping.encode() == root_mapping {
            let gate = pressed && amount.is_none_or(|v| v >= 0.5);
            if gate && !self.root_gate {
                self.recipe = None;
                self.selection_held.fill(false);
                self.selection_down = 0;
            }
            self.root_gate = gate;
            if mapping.kind == NOTE && pressed {
                self.held[note_id] = true;
                self.root_modifier = Some(note_id);
            }
            return true;
        }
        // The modifier owns subsequent notes before other performance mappings.
        // Releases retain the recipe; a new press after all releases starts a root.
        if self.root_gate && mapping.kind == NOTE && pressed {
            if self.selection_down == 0 {
                self.recipe = Some(RootSelection {
                    root: mapping.number,
                    second: None,
                    additions: 0,
                });
            } else if let Some(recipe) = self.recipe.as_mut() {
                if recipe.second.is_none() {
                    recipe.second = Some(mapping.number);
                } else {
                    recipe.additions |=
                        1 << (mapping.number as i16 - recipe.root as i16).rem_euclid(12);
                }
            }
            self.selection_down += 1;
            self.selection_held[note_id] = true;
            self.held[note_id] = true;
            self.selection = self.recipe;
            return true;
        }
        let Some(target) = params.quick_midi[..CONTROL_COUNT]
            .iter()
            .position(|binding| binding.mapping.load(Ordering::Acquire) == mapping.encode())
        else {
            return false;
        };
        if pressed {
            let step =
                amount.is_none() || params.quick_midi[target].step_cc.load(Ordering::Relaxed);
            if step && amount.is_some() && !cc_rising {
                return true;
            }
            let (min, max) = RANGES[target];
            let desired = if step {
                // Keyboard selection and memories can change these independently
                // of their host parameters; step from what is actually playing.
                let current = match target {
                    2 => harmony[1] as i32,
                    4 => harmony[0] as i32,
                    _ => value(params, target),
                };
                if current >= max {
                    min
                } else {
                    current + 1
                }
            } else {
                min + (amount.unwrap_or(0.0) * (max - min) as f32).round() as i32
            };
            params.quick_midi[target].set(base(params, target), desired);
            if mapping.kind == NOTE {
                self.held[note_id] = true;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engine::Out, Chordboard};

    fn note(channel: u8, note: u8, on: bool) -> NoteEvent<()> {
        NoteEvent::NoteOn {
            timing: 0,
            voice_id: None,
            channel,
            note,
            velocity: if on { 1.0 } else { 0.0 },
        }
    }
    fn cc(channel: u8, cc: u8, value: f32) -> NoteEvent<()> {
        NoteEvent::MidiCC {
            timing: 0,
            channel,
            cc,
            value,
        }
    }
    fn send(plugin: &mut Chordboard, event: NoteEvent<()>) -> Vec<Out> {
        let mut output = Vec::new();
        plugin.input(event, &mut |e| output.push(e));
        output
    }

    #[test]
    fn midi_learn_notes_are_channel_specific_cycle_all_targets_and_never_play() {
        for target in 0..CONTROL_COUNT {
            let mut plugin = Chordboard::default();
            plugin
                .engine
                .configure(plugin.current_config(), &mut |_| {});
            plugin
                .bridge
                .quick_learn
                .store(target as u32 + 1, Ordering::Release);
            let original = value(&plugin.params, target);
            assert!(send(&mut plugin, note(9, 40, true)).is_empty());
            assert_eq!(plugin.bridge.quick_learn.load(Ordering::Acquire), 0);
            assert_eq!(
                value(&plugin.params, target),
                original,
                "capture must not activate target {target}"
            );
            assert!(send(&mut plugin, note(9, 40, false)).is_empty());
            let (min, max) = RANGES[target];
            for step in 1..=(max - min + 1) {
                assert!(send(&mut plugin, note(9, 40, true))
                    .iter()
                    .all(|e| !matches!(e, Out::On(_, 40, _))));
                let expected = min + (original - min + step) % (max - min + 1);
                assert_eq!(
                    value(&plugin.params, target),
                    expected,
                    "target {target}, step {step}"
                );
                send(&mut plugin, note(9, 40, true));
                assert_eq!(
                    value(&plugin.params, target),
                    expected,
                    "repeated Note On must not advance twice"
                );
                assert!(plugin.engine.root.is_none());
                send(&mut plugin, note(9, 40, false));
            }
            // Another channel remains available for musical input.
            send(&mut plugin, note(0, 60, true));
            assert!(plugin.engine.down[60]);
        }
    }

    #[test]
    fn midi_learn_cc_value_and_pad_step_modes_reach_live_config_without_editor() {
        let mut plugin = Chordboard::default();
        for target in 0..CONTROL_COUNT {
            plugin
                .bridge
                .quick_learn
                .store(target as u32 + 1, Ordering::Release);
            send(&mut plugin, cc(9, 20 + target as u8, 0.25));
            for amount in [0.0, 0.5, 1.0] {
                send(&mut plugin, cc(9, 20 + target as u8, amount));
                let (min, max) = RANGES[target];
                assert_eq!(
                    value(&plugin.params, target),
                    min + (amount * (max - min) as f32).round() as i32
                );
            }
        }
        assert_eq!(plugin.engine.config.spread as i32, RANGES[0].1);
        assert_eq!(plugin.engine.config.voice_leading, 2);
        assert_eq!(plugin.engine.config.transpose, 24);
        assert_eq!(plugin.engine.config.quality, 11);
        assert!(plugin.engine.config.arp_enabled);
        assert_eq!(plugin.engine.config.arp_page, 7);
        plugin.params.quick_midi[0]
            .step_cc
            .store(true, Ordering::Relaxed);
        send(&mut plugin, cc(9, 20, 0.0));
        send(&mut plugin, cc(9, 20, 1.0));
        assert_eq!(plugin.engine.config.spread, 0);
        send(&mut plugin, cc(9, 20, 1.0));
        assert_eq!(
            plugin.engine.config.spread, 0,
            "held CC pad must not repeat"
        );
        send(&mut plugin, cc(9, 20, 0.0));
        assert_eq!(plugin.engine.config.spread, 0, "CC release must not step");
        send(&mut plugin, cc(9, 20, 1.0));
        assert_eq!(plugin.engine.config.spread, 1);
        send(&mut plugin, cc(0, 20, 0.0));
        assert_eq!(
            plugin.engine.config.spread, 1,
            "other channels must not control the binding"
        );
    }

    #[test]
    fn midi_learn_preset_round_trip_and_host_edits_preserve_parameter_ownership() {
        let mut plugin = Chordboard::default();
        plugin.bridge.quick_learn.store(4, Ordering::Release);
        send(&mut plugin, cc(9, 21, 0.0));
        send(&mut plugin, cc(9, 21, 1.0));
        plugin.params.quick_midi[3]
            .step_cc
            .store(true, Ordering::Relaxed);
        let fields = plugin.params.serialize_fields();
        let restored = ChordboardParams::default();
        restored.deserialize_fields(&fields);
        assert_eq!(restored.config().transpose, 24);
        assert_eq!(
            restored.quick_midi[3].mapping.load(Ordering::Acquire),
            Mapping::cc(21, 9).encode()
        );
        assert!(restored.quick_midi[3].step_cc.load(Ordering::Relaxed));
        assert_eq!(
            fields
                .keys()
                .filter(|key| key.starts_with("quick-midi-map-v1_"))
                .count(),
            COUNT
        );

        let mut edited = ChordboardParams::default();
        edited.deserialize_fields(&fields);
        edited.transpose = IntParam::new("Transpose", -7, IntRange::Linear { min: -24, max: 24 });
        assert_eq!(edited.config().transpose, -7, "new host/UI value must win");
        assert_eq!(edited.quick_midi[3].pending.load(Ordering::Acquire), 0);
        let mut acknowledged = ChordboardParams::default();
        acknowledged.deserialize_fields(&fields);
        acknowledged.transpose =
            IntParam::new("Transpose", 24, IntRange::Linear { min: -24, max: 24 });
        assert_eq!(acknowledged.config().transpose, 24);
        assert_eq!(
            acknowledged.quick_midi[3].pending.load(Ordering::Acquire),
            0
        );
    }

    #[test]
    fn midi_learn_reassignment_and_clearing_keep_note_releases_owned() {
        let params = ChordboardParams::default();
        let armed = AtomicU32::new(1);
        let mut input = Input::default();
        assert!(input.consume(note(9, 40, true), &params, &armed, [0; 2]));
        params.quick_midi[0].mapping.store(0, Ordering::Release);
        assert!(input.consume(note(9, 40, false), &params, &armed, [0; 2]));
        assert!(!input.consume(note(9, 40, true), &params, &armed, [0; 2]));
        armed.store(1, Ordering::Release);
        assert!(input.consume(cc(9, 21, 1.0), &params, &armed, [0; 2]));
        armed.store(2, Ordering::Release);
        assert!(input.consume(cc(9, 21, 1.0), &params, &armed, [0; 2]));
        assert_eq!(params.quick_midi[0].mapping.load(Ordering::Acquire), 0);
        assert_eq!(
            params.quick_midi[1].mapping.load(Ordering::Acquire),
            Mapping::cc(21, 9).encode()
        );
        armed.store(1, Ordering::Release);
        assert!(!input.consume(cc(9, 123, 0.0), &params, &armed, [0; 2]));
        assert_eq!(
            armed.load(Ordering::Acquire),
            1,
            "channel-mode messages must not be learned"
        );
        assert!(!input.consume(cc(9, 21, f32::NAN), &params, &armed, [0; 2]));
    }

    #[test]
    fn midi_learn_change_root_retains_dyads_and_extensions_without_playing_selection_notes() {
        let mut plugin = Chordboard::default();
        plugin
            .engine
            .configure(plugin.current_config(), &mut |_| {});
        plugin
            .bridge
            .quick_learn
            .store(ROOT as u32 + 1, Ordering::Release);
        send(&mut plugin, note(9, 36, true));
        send(&mut plugin, note(9, 36, false));
        // Press order selects the root; the second pitch uses the same resolver
        // as regular Chordboard input, including minor thirds and suspensions.
        for (root, interval) in [(60, 3), (62, 2), (65, 5), (55, 7), (60, 10), (60, 11)] {
            send(&mut plugin, note(9, 36, true));
            assert!(send(&mut plugin, note(0, root, true))
                .iter()
                .all(|event| !matches!(event, Out::On(..))));
            assert!(send(&mut plugin, note(0, root + interval, true))
                .iter()
                .all(|event| !matches!(event, Out::On(..))));
            let recipe = plugin.engine.memory.unwrap();
            assert_eq!(recipe.root, root);
            assert_eq!(recipe.second, Some(root + interval));
            let expected = crate::harmony::voice(root, 0, Some(root + interval), 0, 0, 0);
            assert_eq!(plugin.engine.full_notes, expected);
            send(&mut plugin, note(9, 36, false));
            send(&mut plugin, note(0, root, false));
            send(&mut plugin, note(0, root + interval, false));
            assert_eq!(
                plugin.engine.full_notes, expected,
                "release must retain selection"
            );
            assert_eq!(plugin.engine.memory.unwrap().root, root);
        }
        send(&mut plugin, note(9, 36, true));
        send(&mut plugin, note(0, 60, true));
        send(&mut plugin, note(0, 63, true));
        send(&mut plugin, note(0, 74, true));
        let recipe = plugin.engine.memory.unwrap();
        assert_eq!(recipe.additions, 1 << 2);
        assert_eq!(
            plugin.engine.full_notes,
            crate::harmony::voice_recipe(recipe, 0)
        );
        send(&mut plugin, note(0, 60, false));
        send(&mut plugin, note(0, 63, false));
        send(&mut plugin, note(0, 74, false));
        // All keys released while the modifier remains held: start a fresh root.
        send(&mut plugin, note(0, 67, true));
        assert_eq!(plugin.engine.memory.unwrap().root, 67);
        assert_eq!(plugin.engine.memory.unwrap().second, None);
        assert_eq!(plugin.engine.memory.unwrap().additions, 0);
        send(&mut plugin, note(9, 36, false));
        send(&mut plugin, note(0, 67, false));
        send(&mut plugin, note(0, 72, true));
        assert!(
            plugin.engine.down[72],
            "normal playing resumes when modifier is released"
        );
    }

    #[test]
    fn midi_learn_change_root_cc_gate_survives_clear_and_preserves_melody_and_bass() {
        let mut plugin = Chordboard::default();
        plugin
            .engine
            .configure(plugin.current_config(), &mut |_| {});
        // Bass below control octave and melody above 60 use independent paths.
        plugin.params = std::sync::Arc::new(ChordboardParams {
            key_split: BoolParam::new("Split", true),
            ..ChordboardParams::default()
        });
        plugin
            .engine
            .configure(plugin.current_config(), &mut |_| {});
        send(&mut plugin, note(1, 5, true));
        send(&mut plugin, note(2, 80, true));
        plugin
            .bridge
            .quick_learn
            .store(ROOT as u32 + 1, Ordering::Release);
        send(&mut plugin, cc(9, 21, 0.0));
        send(&mut plugin, cc(9, 21, 1.0));
        // Override the normal control octave too while root selection is held.
        send(&mut plugin, note(0, 12, true));
        assert_eq!(plugin.engine.memory.unwrap().root, 12);
        assert!(plugin.engine.melody_note_held(2, 80));
        assert!(plugin.engine.down[128 + 5]);
        plugin.params.quick_midi[ROOT]
            .mapping
            .store(0, Ordering::Release);
        send(&mut plugin, note(0, 12, false));
        assert_eq!(plugin.engine.memory.unwrap().root, 12);
        send(&mut plugin, note(0, 64, true));
        assert!(
            plugin.engine.melody_note_held(0, 64),
            "clearing a held CC modifier must end its gate"
        );
        let output = send(&mut plugin, note(1, 5, false));
        assert!(output
            .iter()
            .any(|event| matches!(event, Out::Off(_, 5, _))));
        assert!(send(&mut plugin, note(2, 80, false)).contains(&Out::Off(2, 80, 0.0)));
    }
}
