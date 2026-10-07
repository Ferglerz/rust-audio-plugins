//! Chordboard: realtime MIDI harmony, strumming, and expression routing.
mod bridge;
pub mod engine;
pub mod harmony;
pub mod params;
pub mod sequencer;
mod ui;
use bridge::Bridge;
use engine::{Engine, Out};
use nih_plug::prelude::*;
use params::ChordboardParams;
use std::sync::{atomic::Ordering, Arc};

// Stay within both wrappers' preallocated outgoing event capacities. If a host
// delivers a pathological event storm, drop the whole new batch and release
// the voices that existed at entry, rather than allocating or losing Note Offs.
struct EventBatch {
    events: [Option<(u32, Out)>; 512],
    len: usize,
    overflow: bool,
}
impl Default for EventBatch {
    fn default() -> Self {
        Self {
            events: [None; 512],
            len: 0,
            overflow: false,
        }
    }
}
impl EventBatch {
    fn push(&mut self, timing: u32, event: Out) {
        if self.len == self.events.len() {
            self.overflow = true;
        } else {
            self.events[self.len] = Some((timing, event));
            self.len += 1;
        }
    }
}

pub struct Chordboard {
    params: Arc<ChordboardParams>,
    bridge: Arc<Bridge>,
    engine: Engine,
    expected_position: Option<i64>,
    last_axes: [f32; 2],
    pending_reset: bool,
}
impl Default for Chordboard {
    fn default() -> Self {
        Self {
            params: Arc::new(ChordboardParams::default()),
            bridge: Arc::new(Bridge::default()),
            engine: Engine::default(),
            expected_position: None,
            last_axes: [0.0, 0.8],
            pending_reset: false,
        }
    }
}
fn output(event: Out, timing: u32) -> NoteEvent<()> {
    match event {
        Out::On(channel, note, velocity) => NoteEvent::NoteOn {
            timing,
            voice_id: None,
            channel,
            note,
            velocity,
        },
        Out::Off(channel, note, velocity) => NoteEvent::NoteOff {
            timing,
            voice_id: None,
            channel,
            note,
            velocity,
        },
        Out::Pressure(channel, pressure) => NoteEvent::MidiChannelPressure {
            timing,
            channel,
            pressure,
        },
        Out::Bend(channel, value) => NoteEvent::MidiPitchBend {
            timing,
            channel,
            value,
        },
        Out::Cc(channel, cc, value) => NoteEvent::MidiCC {
            timing,
            channel,
            cc,
            value,
        },
    }
}
impl Chordboard {
    fn process_events(
        &mut self,
        count: usize,
        mut next_event: impl FnMut() -> Option<NoteEvent<()>>,
        out: &mut impl FnMut(u32, Out),
    ) {
        let mut next = next_event();
        let mut sample = 0;
        while sample < count {
            // Host MIDI wins ties with scheduled note-offs and strikes.
            while next.is_some_and(|event| event.timing() as usize <= sample) {
                if let Some(event) = next.take() {
                    self.input(event, &mut |event| out(sample as u32, event));
                }
                next = next_event();
            }
            let end = next.map_or(count, |event| (event.timing() as usize).min(count));
            self.engine.advance(end - sample, &mut |offset, event| {
                out((sample + offset) as u32, event);
            });
            sample = end;
        }
        // Preserve zero-length buffers and hosts' end-boundary/trailing MIDI:
        // apply those inputs after the span, clamping only their output timing.
        while let Some(event) = next.take() {
            self.input(event, &mut |event| {
                out(count.saturating_sub(1) as u32, event);
            });
            next = next_event();
        }
    }

    fn current_config(&self) -> engine::Config {
        let mut config = self.params.config();
        let encoded = self.bridge.learned_split.load(Ordering::Acquire);
        if let Some(learned) = engine::LearnedSplit::decode(encoded) {
            if config.bass_split == learned.bass as i16
                && config.split_note == learned.melody
                && (if learned.target == 5 {
                    config.bass_enabled
                } else {
                    config.key_split
                })
            {
                let _ = self.bridge.learned_split.compare_exchange(
                    encoded,
                    0,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                );
            } else {
                learned.apply(&mut config);
            }
        }
        config
    }
    fn playback_tempo(&self, host_tempo: Option<f64>) -> f64 {
        if self.params.tempo_sync.value() {
            host_tempo.unwrap_or(120.0)
        } else {
            self.params.tempo.value() as f64
        }
    }
    fn detect_mpe_input(&self, event: &NoteEvent<()>) -> Option<bool> {
        match *event {
            NoteEvent::MidiCC {
                channel,
                cc: 6,
                value,
                ..
            } if matches!(channel, 0 | 15)
                && value.is_finite()
                && self.engine.rpn[channel as usize] == [0, 6] =>
            {
                Some(value > 0.0)
            }
            NoteEvent::PolyTuning { channel, note, .. }
            | NoteEvent::PolyBrightness { channel, note, .. }
                if channel < 16
                    && note < 128
                    && self.engine.down[channel as usize * 128 + note as usize] =>
            {
                Some(true)
            }
            NoteEvent::MidiPitchBend { channel, .. }
            | NoteEvent::MidiChannelPressure { channel, .. }
            | NoteEvent::MidiCC {
                channel, cc: 74, ..
            } if self.engine.member_expression_is_mpe(channel) => Some(true),
            _ => None,
        }
    }
    fn input(&mut self, event: NoteEvent<()>, out: &mut impl FnMut(Out)) {
        if let Some(detected) = self.detect_mpe_input(&event) {
            self.params.detected_mpe.store(detected, Ordering::Relaxed);
            self.engine.configure(self.current_config(), out);
        }
        match event {
            NoteEvent::NoteOn {
                channel,
                note,
                velocity,
                ..
            } => self.engine.midi_note(true, channel, note, velocity, out),
            NoteEvent::NoteOff {
                channel,
                note,
                velocity,
                ..
            } => self.engine.midi_note(false, channel, note, velocity, out),
            NoteEvent::MidiCC {
                channel, cc, value, ..
            } => self.engine.control(channel, cc, value, out),
            NoteEvent::MidiChannelPressure {
                channel, pressure, ..
            } => self.engine.pressure(channel, None, pressure, out),
            NoteEvent::PolyPressure {
                channel,
                note,
                pressure,
                ..
            } => self.engine.pressure(channel, Some(note), pressure, out),
            NoteEvent::MidiPitchBend { channel, value, .. } => {
                self.engine.bend(channel, value, out)
            }
            NoteEvent::PolyTuning {
                channel,
                note,
                tuning,
                ..
            } => {
                if self.engine.melody_note_held(channel, note)
                    || self
                        .engine
                        .root
                        .is_some_and(|s| s.held && s.channel == channel && s.note == note)
                {
                    let range = self.engine.input_bend[channel as usize].max(1.0);
                    self.engine
                        .bend(channel, (0.5 + tuning / (2.0 * range)).clamp(0.0, 1.0), out);
                }
            }
            NoteEvent::PolyBrightness {
                channel,
                note,
                brightness,
                ..
            } => {
                if self.engine.melody_note_held(channel, note)
                    || self
                        .engine
                        .root
                        .is_some_and(|s| s.held && s.channel == channel && s.note == note)
                {
                    self.engine.control(channel, 74, brightness, out);
                }
            }
            NoteEvent::Choke { .. } => self.engine.panic(out),
            _ => {}
        }
        if let Some(learned) = self.engine.learned_split.take() {
            self.bridge
                .learned_split
                .store(learned.encode(), Ordering::Release);
        }
        // Publish learning before another event can reconfigure the engine (MPE
        // detection also reads these parameters within the current audio block).
        if let Some(base) = self.engine.learned_base.take() {
            self.params
                .control_base
                .store(base as i32, Ordering::Relaxed);
        }
    }
}
impl Plugin for Chordboard {
    fn filter_state(state: &mut PluginState) {
        use nih_plug::wrapper::state::ParamValue;
        if !state.fields.contains_key("sequencer-v1") {
            state.fields.insert(
                "sequencer-v1".into(),
                serde_json::to_string(&sequencer::State::default()).unwrap_or_default(),
            );
            state
                .params
                .insert("seq_enabled".into(), ParamValue::Bool(false));
        }
        state
            .params
            .entry("bass_split".into())
            .or_insert(ParamValue::I32(-1));
        if state
            .fields
            .get("schema-version")
            .is_none_or(|v| !v.parse::<u32>().is_ok_and(|version| version >= 2))
        {
            let destination = engine::routing::default_routes()[0].target as i32;
            let existing = (1..=engine::routing::ROUTE_COUNT).any(|slot| {
                matches!(state.params.get(&format!("route_target_{slot}")), Some(ParamValue::I32(v)) if *v == destination)
            });
            if existing {
                state.fields.insert("schema-version".into(), "2".into());
            } else if let Some(slot) = (1..=engine::routing::ROUTE_COUNT).find(|slot| {
                state
                    .params
                    .get(&format!("route_source_{slot}"))
                    .is_none_or(|v| matches!(v, ParamValue::I32(0)))
            }) {
                state
                    .params
                    .insert(format!("route_source_{slot}"), ParamValue::I32(8));
                state
                    .params
                    .insert(format!("route_target_{slot}"), ParamValue::I32(destination));
                state
                    .params
                    .insert(format!("route_enabled_{slot}"), ParamValue::Bool(true));
                state
                    .params
                    .insert(format!("route_min_{slot}"), ParamValue::F32(0.0));
                state
                    .params
                    .insert(format!("route_max_{slot}"), ParamValue::F32(1.0));
                state
                    .params
                    .insert(format!("route_curve_{slot}"), ParamValue::F32(0.0));
                state.fields.insert("schema-version".into(), "2".into());
            } else {
                // A full legacy matrix retains its direct X behavior until a
                // slot is freed, without replacing any saved assignments.
                state.fields.insert("schema-version".into(), "1".into());
            }
        }
        if !state.params.contains_key("output_mode") {
            if let Some(ParamValue::Bool(enabled)) = state.params.get("mpe") {
                state.params.insert(
                    "output_mode".into(),
                    ParamValue::I32(if *enabled { 1 } else { 2 }),
                );
            }
        }
    }
    const NAME: &'static str = "Chordboard";
    const VENDOR: &'static str = "Fergler";
    const URL: &'static str = "https://github.com/Ferglerz/Chordboard";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = ();
    type BackgroundTask = ();
    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn editor(&mut self, _: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone(), self.bridge.clone())
    }
    fn initialize(
        &mut self,
        _: &AudioIOLayout,
        config: &BufferConfig,
        _: &mut impl InitContext<Self>,
    ) -> bool {
        self.engine.sample_rate = config.sample_rate;
        self.pending_reset = true;
        true
    }
    fn reset(&mut self) {
        self.bridge.learned_split.store(0, Ordering::Release);
        self.pending_reset = true;
        self.engine.quality = self.params.selected_quality.load(Ordering::Relaxed).min(11) as u8;
        self.expected_position = None;
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let count = buffer.samples();
        let starting_notes = self.engine.owned_output_notes();
        let mut emitted = EventBatch::default();
        let transport = context.transport();
        let playing = transport.playing;
        let tempo = self.playback_tempo(transport.tempo);
        let position = transport.pos_samples();
        let jump = playing
            && self
                .expected_position
                .zip(position)
                .is_some_and(|(a, b)| (a - b).abs() > 1);
        self.expected_position = if playing {
            position.map(|p| p + count as i64)
        } else {
            None
        };
        let panic_requested = self.bridge.panic.swap(false, Ordering::AcqRel);
        let reset_requested = self.bridge.reset.swap(false, Ordering::AcqRel);
        if self.pending_reset || panic_requested || reset_requested {
            self.engine.panic(&mut |e| emitted.push(0, e));
            if reset_requested {
                self.engine.quality = self.params.quality.value() as u8;
                self.engine.inversion = self.params.inversion.value() as u8;
            }
            self.pending_reset = false;
            for _ in 0..128 {
                if self.bridge.commands.pop().is_none() {
                    break;
                }
            }
        }
        let mut sequence_updated = false;
        for _ in 0..2 {
            if !self
                .params
                .sequencer
                .apply_pending(&mut self.engine.seq.data)
            {
                break;
            }
            sequence_updated = true;
        }
        if sequence_updated {
            self.engine
                .accept_sequence_generation(&mut |e| emitted.push(0, e));
        }
        self.engine
            .configure(self.current_config(), &mut |e| emitted.push(0, e));
        self.engine
            .transport(playing, tempo, jump, &mut |e| emitted.push(0, e));
        let axes = [self.params.x.value(), self.params.y.value()];
        for (axis, &value) in axes.iter().enumerate() {
            if value != self.last_axes[axis] {
                self.engine
                    .position(value, axis, &mut |e| emitted.push(0, e));
            }
        }
        self.last_axes = axes;
        // Bounded work even if a UI producer continuously enqueues commands.
        for _ in 0..128 {
            let Some(command) = self.bridge.commands.pop() else {
                break;
            };
            self.engine.command(command, &mut |e| emitted.push(0, e));
        }
        self.process_events(count, || context.next_event(), &mut |timing, event| {
            emitted.push(timing, event);
        });
        if let Some((axis, mapping)) = self.engine.learned.take() {
            self.params
                .mapping(axis)
                .store(mapping.encode(), Ordering::Relaxed);
        }
        if let Some((index, chord)) = self.engine.saved.take() {
            self.params.slot(index).store(chord, Ordering::Relaxed);
        }
        if let Some(base) = self.engine.learned_base.take() {
            self.params
                .control_base
                .store(base as i32, Ordering::Relaxed);
        }
        if emitted.overflow {
            self.engine.panic(&mut |_| {});
            // A pedal-up earlier in the dropped batch may have cleared the
            // internal flag already. Release sustain on every output channel.
            for channel in 0..16 {
                context.send_event(output(Out::Cc(channel, 64, 0.0), 0));
            }
            let owned_count = starting_notes
                .iter()
                .flatten()
                .filter(|&&owned| owned)
                .count();
            if owned_count > 480 {
                // Bound emergency recovery even after many transparent notes
                // accumulated across prior blocks.
                for channel in 0..16 {
                    context.send_event(output(Out::Cc(channel, 123, 0.0), 0));
                }
            } else {
                for (channel, notes) in starting_notes.iter().enumerate() {
                    for (note, &owned) in notes.iter().enumerate() {
                        if owned {
                            context.send_event(output(Out::Off(channel as u8, note as u8, 0.0), 0));
                        }
                    }
                }
            }
        } else {
            for &(timing, event) in emitted.events[..emitted.len].iter().flatten() {
                context.send_event(output(event, timing));
            }
        }
        self.params
            .selected_quality
            .store(self.engine.quality as u32, Ordering::Relaxed);
        self.bridge.publish(self.engine.snapshot());
        ProcessStatus::Normal
    }
}
impl ClapPlugin for Chordboard {
    const CLAP_ID: &'static str = "com.fergler.chordboard";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Expressive harmony and gesture-driven MIDI strumming");
    const CLAP_MANUAL_URL: Option<&'static str> = Some("https://github.com/Ferglerz/Chordboard");
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::NoteEffect, ClapFeature::Utility];
}
impl Vst3Plugin for Chordboard {
    const VST3_CLASS_ID: [u8; 16] = *b"FergChordboard01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}
nih_export_clap!(Chordboard);
nih_export_vst3!(Chordboard);

#[cfg(feature = "ui-preview")]
pub use ui::preview;

#[cfg(test)]
mod protocol_tests {
    use super::*;
    use nih_plug::wrapper::state::ParamValue;
    fn cc(plugin: &mut Chordboard, channel: u8, cc: u8, value: u8) {
        plugin.input(
            NoteEvent::MidiCC {
                timing: 0,
                channel,
                cc,
                value: value as f32 / 127.0,
            },
            &mut |_| {},
        );
    }
    #[test]
    fn split_melody_survives_auto_mpe_detection_and_poly_expression() {
        let mut plugin = Chordboard {
            params: Arc::new(ChordboardParams {
                output_mode: IntParam::new(
                    "Output protocol",
                    0,
                    IntRange::Linear { min: 0, max: 2 },
                ),
                key_split: BoolParam::new("Key split", true),
                always_chord: BoolParam::new("Always play full chord", true),
                mode: IntParam::new("Play mode", 2, IntRange::Linear { min: 1, max: 3 }),
                ..ChordboardParams::default()
            }),
            ..Chordboard::default()
        };
        plugin.engine.configure(plugin.params.config(), &mut |_| {});
        for (channel, note) in [(0, 48), (1, 72), (2, 76)] {
            plugin.input(
                NoteEvent::NoteOn {
                    timing: 0,
                    voice_id: None,
                    channel,
                    note,
                    velocity: 0.8,
                },
                &mut |_| {},
            );
        }
        let mut events = Vec::new();
        plugin.input(
            NoteEvent::PolyTuning {
                timing: 0,
                voice_id: None,
                channel: 1,
                note: 72,
                tuning: 2.0,
            },
            &mut |e| events.push(e),
        );
        assert!(plugin.engine.config.mpe);
        assert!(plugin.engine.melody_note_held(1, 72));
        assert!(plugin.engine.melody_note_held(2, 76));
        assert!(!events
            .iter()
            .any(|e| matches!(e, Out::Off(1, 72, _) | Out::Off(2, 76, _))));
        assert!(events
            .iter()
            .any(|e| matches!(e, Out::Bend(1, value) if *value > 0.5)));
        events.clear();
        plugin.input(
            NoteEvent::PolyBrightness {
                timing: 0,
                voice_id: None,
                channel: 2,
                note: 76,
                brightness: 0.8,
            },
            &mut |e| events.push(e),
        );
        assert!(events.contains(&Out::Cc(2, 74, 0.8)));
    }

    #[test]
    fn learned_control_octave_survives_same_block_mpe_configuration() {
        let mut plugin = Chordboard::default();
        plugin
            .engine
            .command(engine::Command::Learn(4), &mut |_| {});
        plugin.input(
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 36,
                velocity: 0.8,
            },
            &mut |_| {},
        );
        assert_eq!(plugin.params.control_base.load(Ordering::Relaxed), 36);
        cc(&mut plugin, 0, 101, 0);
        cc(&mut plugin, 0, 100, 6);
        cc(&mut plugin, 0, 6, 8);
        let mut events = Vec::new();
        plugin.input(
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 1,
                note: 39,
                velocity: 0.8,
            },
            &mut |e| events.push(e),
        );
        assert!(!events.iter().any(|e| matches!(e, Out::On(..))));
        assert!(plugin.engine.root.is_none());
        assert_eq!(plugin.engine.quality, 1);
    }
    #[test]
    fn auto_detects_announced_mpe_zones_and_respects_forced_modes() {
        for mode in 0..3 {
            for channel in [0, 15] {
                let mut plugin = Chordboard {
                    params: Arc::new(ChordboardParams {
                        output_mode: IntParam::new(
                            "Output protocol",
                            mode,
                            IntRange::Linear { min: 0, max: 2 },
                        ),
                        ..ChordboardParams::default()
                    }),
                    ..Chordboard::default()
                };
                cc(&mut plugin, channel, 101, 0);
                cc(&mut plugin, channel, 100, 6);
                cc(&mut plugin, channel, 6, 8);
                assert!(plugin.params.detected_mpe.load(Ordering::Relaxed));
                assert_eq!(plugin.engine.config.mpe, mode != 2);
                cc(&mut plugin, channel, 6, 0);
                assert_eq!(plugin.engine.config.mpe, mode == 1);
            }
        }
    }
    #[test]
    fn auto_ignores_notes_and_master_expression_but_detects_member_expression() {
        let mut plugin = Chordboard {
            params: Arc::new(ChordboardParams {
                output_mode: IntParam::new(
                    "Output protocol",
                    0,
                    IntRange::Linear { min: 0, max: 2 },
                ),
                ..ChordboardParams::default()
            }),
            ..Chordboard::default()
        };
        for (channel, note) in [(1, 60), (2, 64)] {
            plugin.input(
                NoteEvent::NoteOn {
                    timing: 0,
                    voice_id: None,
                    channel,
                    note,
                    velocity: 0.8,
                },
                &mut |_| {},
            );
        }
        assert!(!plugin.params.mpe_enabled());
        plugin.input(
            NoteEvent::MidiPitchBend {
                timing: 0,
                channel: 0,
                value: 0.7,
            },
            &mut |_| {},
        );
        assert!(!plugin.params.mpe_enabled());
        plugin.input(
            NoteEvent::MidiPitchBend {
                timing: 0,
                channel: 1,
                value: 0.7,
            },
            &mut |_| {},
        );
        assert!(plugin.params.mpe_enabled());
        assert!(plugin.engine.config.mpe);
    }
    #[test]
    fn old_protocol_presets_migrate_once_and_new_instances_default_off() {
        assert_eq!(ChordboardParams::default().output_mode.value(), 2);
        for enabled in [false, true] {
            let mut state = PluginState {
                version: "0.1.0".into(),
                params: Default::default(),
                fields: Default::default(),
            };
            state.params.insert("mpe".into(), ParamValue::Bool(enabled));
            Chordboard::filter_state(&mut state);
            assert!(
                matches!(state.params["output_mode"], ParamValue::I32(v) if v == if enabled { 1 } else { 2 })
            );
            state
                .params
                .insert("output_mode".into(), ParamValue::I32(0));
            Chordboard::filter_state(&mut state);
            assert!(matches!(state.params["output_mode"], ParamValue::I32(0)));
        }
    }
    #[test]
    fn legacy_strumfield_migration_preserves_links_and_is_idempotent() {
        let mut state = PluginState {
            version: "0.1.0".into(),
            params: Default::default(),
            fields: Default::default(),
        };
        state
            .params
            .insert("route_source_1".into(), ParamValue::I32(2));
        state
            .params
            .insert("route_target_1".into(), ParamValue::I32(0));
        Chordboard::filter_state(&mut state);
        assert!(matches!(state.params["route_source_1"], ParamValue::I32(2)));
        assert!(matches!(state.params["route_target_1"], ParamValue::I32(0)));
        assert!(matches!(state.params["route_source_2"], ParamValue::I32(8)));
        assert!(
            matches!(state.params["route_target_2"], ParamValue::I32(v) if v == engine::routing::default_routes()[0].target as i32)
        );
        assert_eq!(state.fields["schema-version"], "2");
        state
            .params
            .insert("route_source_2".into(), ParamValue::I32(0));
        Chordboard::filter_state(&mut state);
        assert!(matches!(state.params["route_source_2"], ParamValue::I32(0)));
    }

    #[test]
    fn full_legacy_matrix_keeps_every_link_and_its_direct_strumfield_behavior() {
        let mut state = PluginState {
            version: "0.1.0".into(),
            params: Default::default(),
            fields: Default::default(),
        };
        for slot in 1..=engine::routing::ROUTE_COUNT {
            state
                .params
                .insert(format!("route_source_{slot}"), ParamValue::I32(2));
            state
                .params
                .insert(format!("route_target_{slot}"), ParamValue::I32(0));
        }
        Chordboard::filter_state(&mut state);
        assert_eq!(state.fields["schema-version"], "1");
        assert_eq!(state.params.len(), engine::routing::ROUTE_COUNT * 2 + 2);
        assert!(matches!(
            state.params["seq_enabled"],
            ParamValue::Bool(false)
        ));
        let mut config = engine::Config {
            mode: engine::MANUAL,
            legacy_direct_x: true,
            ..engine::Config::default()
        };
        config.routes = [engine::routing::Route::default(); engine::routing::ROUTE_COUNT];
        let mut engine = engine::Engine::default();
        engine.configure(config, &mut |_| {});
        engine.position(0.7, 0, &mut |_| {});
        assert_eq!(engine.x, 0.7);
    }

    #[test]
    fn tempo_source_switches_between_host_and_manual_bpm() {
        let mut plugin = Chordboard::default();
        assert_eq!(plugin.playback_tempo(Some(93.0)), 93.0);
        assert_eq!(plugin.playback_tempo(None), 120.0);
        plugin.params = Arc::new(ChordboardParams {
            tempo_sync: BoolParam::new("Host sync", false),
            tempo: FloatParam::new(
                "Tempo BPM",
                87.0,
                FloatRange::Linear {
                    min: 20.0,
                    max: 400.0,
                },
            ),
            ..ChordboardParams::default()
        });
        assert_eq!(plugin.playback_tempo(Some(93.0)), 87.0);
    }
    #[test]
    fn learned_split_stays_active_until_host_parameters_acknowledge_it() {
        let mut plugin = Chordboard::default();
        plugin
            .engine
            .configure(plugin.current_config(), &mut |_| {});
        plugin
            .engine
            .command(engine::Command::Learn(6), &mut |_| {});
        plugin.input(
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 20,
                velocity: 0.8,
            },
            &mut |_| {},
        );
        let config = plugin.current_config();
        assert_eq!((config.bass_split, config.split_note), (12, 20));
        assert!(config.key_split);
        assert_ne!(plugin.bridge.learned_split.load(Ordering::Acquire), 0);
        plugin.params = Arc::new(ChordboardParams {
            bass_split: IntParam::new("Bass", 12, IntRange::Linear { min: -1, max: 127 }),
            split_note: IntParam::new("Melody", 20, IntRange::Linear { min: 0, max: 127 }),
            key_split: BoolParam::new("Melody enabled", true),
            ..ChordboardParams::default()
        });
        assert_eq!(plugin.current_config().split_note, 20);
        assert_eq!(plugin.bridge.learned_split.load(Ordering::Acquire), 0);
    }
}
