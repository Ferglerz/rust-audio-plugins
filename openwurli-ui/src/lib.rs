//! OpenWurli UI: an independent nih-plug host shell for upstream OpenWurli DSP.

use nih_plug::{midi::control_change, prelude::*};
use openwurli_dsp::{CircuitMode, WurliEngine};
use std::sync::Arc;

mod engine_info;
pub mod params;
pub mod ui;

use params::{CpuMode, OpenWurliUiParams};

pub struct OpenWurliUi {
    params: Arc<OpenWurliUiParams>,
    engine: WurliEngine,
    finish: FastFinish,
}

/// Optional post-engine extra sag, shared by both circuit modes.
/// The Heavy circuit also includes its native physical rail behavior.
struct FastFinish {
    sag_envelope: f32,
    sag_attack: f32,
    sag_release: f32,
}

impl FastFinish {
    fn new(sample_rate: f32) -> Self {
        let mut finish = Self {
            sag_envelope: 0.0,
            sag_attack: 0.0,
            sag_release: 0.0,
        };
        finish.set_sample_rate(sample_rate);
        finish
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        let sample_rate = sample_rate.max(1.0);
        self.sag_attack = 1.0 - (-1.0 / (0.008 * sample_rate)).exp();
        self.sag_release = 1.0 - (-1.0 / (0.150 * sample_rate)).exp();
    }

    fn reset(&mut self) {
        self.sag_envelope = 0.0;
    }

    fn process(&mut self, output: &mut [f32], sag_enabled: bool) {
        for sample in output {
            if sag_enabled {
                let magnitude = sample.abs();
                let coefficient = if magnitude > self.sag_envelope {
                    self.sag_attack
                } else {
                    self.sag_release
                };
                self.sag_envelope += coefficient * (magnitude - self.sag_envelope);
                *sample /= 1.0 + 1.2 * self.sag_envelope;
            }
        }
        if !sag_enabled {
            self.sag_envelope = 0.0;
        }
    }
}

impl Default for OpenWurliUi {
    fn default() -> Self {
        Self {
            params: Arc::new(OpenWurliUiParams::default()),
            engine: WurliEngine::new(44_100.0),
            finish: FastFinish::new(44_100.0),
        }
    }
}

impl OpenWurliUi {
    fn sync_params(&mut self) {
        self.engine
            .set_circuit_mode(match self.params.cpu_mode.value() {
                CpuMode::Fast => CircuitMode::Fast,
                CpuMode::Heavy => CircuitMode::Heavy,
            });
        self.engine.set_volume(self.params.volume.value() as f64);
        self.engine
            .set_tremolo_depth(self.params.tremolo_depth.value() as f64);
        self.engine
            .set_speaker_character(self.params.speaker_character.value() as f64);
        self.engine.set_mlp_enabled(self.params.mlp_enabled.value());
        // Hiss is removed, including native preamp noise in both circuit modes.
        self.engine.set_noise_enabled(false);
        self.engine
            .set_reed_decay(self.params.reed_decay.value() as f64);
        self.engine
            .set_hammer_hardness(self.params.hammer_hardness.value() as f64);
        self.engine
            .set_pickup_drive(self.params.pickup_drive.value() as f64);
        self.engine
            .set_tremolo_response(self.params.tremolo_response.value() as f64);
    }

    fn handle_event(&mut self, event: &NoteEvent<()>) {
        match event {
            NoteEvent::NoteOn { note, velocity, .. } if *velocity == 0.0 => {
                self.engine.note_off_extended(*note)
            }
            NoteEvent::NoteOn { note, velocity, .. }
                if self.params.extended_notes.value()
                    || (openwurli_dsp::tables::MIDI_LO..=openwurli_dsp::tables::MIDI_HI)
                        .contains(note) =>
            {
                self.engine.note_on_extended(*note, *velocity)
            }
            // Always release the exact pitch, even if Extended Notes was
            // disabled while it was held. Ignored pitches cannot release an end key.
            NoteEvent::NoteOff { note, .. } => self.engine.note_off_extended(*note),
            NoteEvent::MidiCC { cc, value, .. } if *cc == control_change::DAMPER_PEDAL => {
                self.engine.set_sustain(*value >= 0.5)
            }
            _ => {}
        }
    }

    fn render_block(
        &mut self,
        output: &mut [f32],
        mut next_event: impl FnMut() -> Option<NoteEvent<()>>,
    ) {
        let num_samples = output.len();
        let mut block_start = 0;
        let mut event = next_event();
        while block_start < num_samples {
            while let Some(ref current) = event {
                if current.timing() as usize > block_start {
                    break;
                }
                self.handle_event(current);
                event = next_event();
            }
            let block_end = event.as_ref().map_or(num_samples, |current| {
                (current.timing() as usize).min(num_samples)
            });
            if block_end > block_start {
                self.engine.render(&mut output[block_start..block_end]);
            }
            block_start = block_end;
        }
        while let Some(current) = event {
            self.handle_event(&current);
            event = next_event();
        }
    }
}

impl Plugin for OpenWurliUi {
    const NAME: &'static str = "OpenWurli UI";
    const VENDOR: &'static str = "Fergler";
    const URL: &'static str = "https://github.com/Ferglerz/rust-audio-plugins";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn filter_state(state: &mut PluginState) {
        // Missing parameters retain their current value in NIH-plug. Explicitly
        // restore the old Fast sound even when loading into a Heavy instance.
        state
            .params
            .entry("extended_notes".into())
            .or_insert(nih_plug::wrapper::state::ParamValue::Bool(false));
        state
            .params
            .entry("cpu_mode".into())
            .or_insert_with(|| nih_plug::wrapper::state::ParamValue::String("fast".into()));
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Restore the selected circuit before preparation so a saved Heavy
        // session starts in Heavy without an initial transition from Fast.
        self.engine
            .set_circuit_mode(match self.params.cpu_mode.value() {
                CpuMode::Fast => CircuitMode::Fast,
                CpuMode::Heavy => CircuitMode::Heavy,
            });
        self.engine
            .set_sample_rate(buffer_config.sample_rate as f64);
        self.finish.set_sample_rate(buffer_config.sample_rate);
        self.engine
            .ensure_buffer_capacity(buffer_config.max_buffer_size as usize);
        self.sync_params();
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
        self.finish.reset();
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone())
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.sync_params();
        let num_samples = buffer.samples();
        if num_samples == 0 {
            return ProcessStatus::Normal;
        }

        // OpenWurli renders mono. Split at MIDI events, then duplicate the
        // finished signal to the second output channel.
        let channels = buffer.as_slice();
        self.render_block(&mut channels[0][..num_samples], || context.next_event());
        self.finish.process(
            &mut channels[0][..num_samples],
            self.params.rail_sag.value(),
        );
        let (left, rest) = channels.split_at_mut(1);
        for channel in rest {
            channel[..num_samples].copy_from_slice(&left[0][..num_samples]);
        }
        ProcessStatus::Normal
    }
}

impl ClapPlugin for OpenWurliUi {
    const CLAP_ID: &'static str = "com.fergler.openwurli-ui";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("OpenWurli electric piano with Pleasant UI");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Custom("electric-piano"),
    ];
}

impl Vst3Plugin for OpenWurliUi {
    const VST3_CLASS_ID: [u8; 16] = *b"FergOpenWurliUI_";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(OpenWurliUi);
nih_export_vst3!(OpenWurliUi);

#[cfg(test)]
mod tests {
    use super::*;

    fn with_circuit_stack(test: impl FnOnce() + Send + 'static) {
        // The generated full-circuit solvers use large debug stack frames.
        // Release builds do not overflow the normal test thread stack.
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(test)
            .expect("spawn circuit test")
            .join()
            .expect("circuit test completed");
    }

    fn note_on(timing: u32, note: u8) -> NoteEvent<()> {
        NoteEvent::NoteOn {
            timing,
            voice_id: None,
            channel: 0,
            note,
            velocity: 0.8,
        }
    }

    fn note_off(timing: u32, note: u8) -> NoteEvent<()> {
        NoteEvent::NoteOff {
            timing,
            voice_id: None,
            channel: 0,
            note,
            velocity: 0.0,
        }
    }

    fn pedal(value: f32) -> NoteEvent<()> {
        NoteEvent::MidiCC {
            timing: 0,
            channel: 0,
            cc: control_change::DAMPER_PEDAL,
            value,
        }
    }

    struct TestInitContext;

    impl InitContext<OpenWurliUi> for TestInitContext {
        fn plugin_api(&self) -> PluginApi {
            PluginApi::Clap
        }
        fn execute(&self, _: ()) {}
        fn set_latency_samples(&self, _: u32) {}
        fn set_current_voice_capacity(&self, _: u32) {}
    }

    fn restore_cpu_mode(plugin: &OpenWurliUi, state: &mut PluginState) {
        OpenWurliUi::filter_state(state);
        let nih_plug::wrapper::state::ParamValue::String(id) = &state.params["cpu_mode"] else {
            panic!("CPU mode should use stable saved IDs");
        };
        let (_, ptr, _) = plugin
            .params
            .param_map()
            .into_iter()
            .find(|(id, _, _)| id == "cpu_mode")
            .expect("saved CPU parameter");
        let ParamPtr::EnumParam(ptr) = ptr else {
            panic!("CPU mode must be an enum")
        };
        // Same enum-ID restoration used by NIH-plug. The plugin's Arc keeps
        // this parameter alive throughout the call.
        assert!(unsafe { (*ptr).set_from_id(id) });
    }

    #[test]
    fn cpu_mode_restore_initializes_selected_audio_and_old_presets_use_fast() {
        with_circuit_stack(|| {
            let mut plugin = OpenWurliUi::default();
            // Restore Heavy first, then load an old state into that instance.
            // An absent cpu_mode must actively select Fast, not retain Heavy.
            for (saved_id, expected) in [
                (Some("heavy"), CircuitMode::Heavy),
                (None, CircuitMode::Fast),
            ] {
                let mut state = PluginState {
                    version: "0.1.0".into(),
                    params: Default::default(),
                    fields: Default::default(),
                };
                if let Some(id) = saved_id {
                    state.params.insert(
                        "cpu_mode".into(),
                        nih_plug::wrapper::state::ParamValue::String(id.into()),
                    );
                }
                restore_cpu_mode(&plugin, &mut state);
                plugin.reset();
                assert!(plugin.initialize(
                    &OpenWurliUi::AUDIO_IO_LAYOUTS[0],
                    &BufferConfig {
                        sample_rate: 48_000.0,
                        min_buffer_size: None,
                        max_buffer_size: 256,
                        process_mode: ProcessMode::Realtime,
                    },
                    &mut TestInitContext
                ));
                assert_eq!(plugin.engine.circuit_mode(), expected);
                let mut reference = WurliEngine::new_with_circuit_mode(48_000.0, expected);
                reference.warm_up();
                plugin.engine.note_on(60, 0.8);
                reference.note_on(60, 0.8);
                let mut actual = [0.0; 256];
                let mut wanted = [0.0; 256];
                plugin.render_block(&mut actual, || None);
                reference.render(&mut wanted);
                assert_eq!(
                    actual.map(f32::to_bits),
                    wanted.map(f32::to_bits),
                    "restored mode must already be settled before the first note"
                );
            }
        });
    }

    #[test]
    fn midi_event_is_applied_at_its_sample_boundary() {
        with_circuit_stack(|| {
            let mut plugin = OpenWurliUi::default();
            let mut reference = OpenWurliUi::default();
            plugin.sync_params();
            reference.sync_params();
            let mut output = [0.0; 2048];
            let mut no_note = [0.0; 2048];
            let mut events = [note_on(128, 60)].into_iter();
            plugin.render_block(&mut output, || events.next());
            reference.render_block(&mut no_note, || None);
            assert!(output[..128]
                .iter()
                .zip(&no_note[..128])
                .all(|(a, b)| (a - b).abs() < 1e-6));
            assert!(output[128..]
                .iter()
                .zip(&no_note[128..])
                .any(|(a, b)| (a - b).abs() > 1e-6));
            assert_eq!(plugin.engine.held_voice_count(), 1);
        });
    }

    #[test]
    fn out_of_range_midi_notes_do_not_strike_or_release_either_end_key() {
        with_circuit_stack(|| {
            for mode in [CircuitMode::Fast, CircuitMode::Heavy] {
                let mut plugin = OpenWurliUi::default();
                plugin.engine.set_circuit_mode(mode);
                let low = openwurli_dsp::tables::MIDI_LO;
                let top = openwurli_dsp::tables::MIDI_HI;
                for note in (0..low).chain(top + 1..=127) {
                    plugin.handle_event(&note_on(0, note));
                    assert_eq!(
                        plugin.engine.held_voice_count(),
                        0,
                        "out-of-range note {note} must not allocate a voice"
                    );
                    plugin.handle_event(&note_off(0, note));
                }
                plugin.handle_event(&note_on(0, low));
                plugin.handle_event(&note_on(0, top));
                assert_eq!(plugin.engine.held_voice_count(), 2);
                for note in (0..low).chain(top + 1..=127) {
                    plugin.handle_event(&note_off(0, note));
                    let mut zero_velocity = note_on(0, note);
                    if let NoteEvent::NoteOn { velocity, .. } = &mut zero_velocity {
                        *velocity = 0.0;
                    }
                    plugin.handle_event(&zero_velocity);
                    assert_eq!(
                        plugin.engine.held_voice_count(),
                        2,
                        "out-of-range release must leave both end keys held"
                    );
                }
                plugin.handle_event(&note_off(0, low));
                plugin.handle_event(&pedal(1.0));
                plugin.handle_event(&note_off(0, top));
                assert_eq!(plugin.engine.held_voice_count(), 0);
                assert_eq!(plugin.engine.sustained_voice_count(), 1);
                plugin.handle_event(&pedal(0.0));
                assert_eq!(plugin.engine.sustained_voice_count(), 0);
            }
        });
    }

    fn set_extended(plugin: &mut OpenWurliUi, enabled: bool) {
        Arc::get_mut(&mut plugin.params).unwrap().extended_notes =
            BoolParam::new("Extended Notes", enabled);
    }

    #[test]
    fn extended_notes_keep_their_identity_and_release_after_disabling() {
        with_circuit_stack(|| {
            for mode in [CircuitMode::Fast, CircuitMode::Heavy] {
                let mut plugin = OpenWurliUi::default();
                plugin.engine.set_circuit_mode(mode);
                for note in 0..=127 {
                    set_extended(&mut plugin, true);
                    plugin.handle_event(&note_on(0, note));
                    assert_eq!(
                        plugin.engine.count_voices_with_note_in_state(
                            note,
                            openwurli_dsp::engine::VoiceState::Held
                        ),
                        1,
                        "{mode:?}: MIDI {note} must retain its actual pitch"
                    );
                    set_extended(&mut plugin, false);
                    plugin.handle_event(&pedal(1.0));
                    if note % 2 == 0 {
                        plugin.handle_event(&note_off(0, note));
                    } else {
                        let mut event = note_on(0, note);
                        if let NoteEvent::NoteOn { velocity, .. } = &mut event {
                            *velocity = 0.0;
                        }
                        plugin.handle_event(&event);
                    }
                    assert_eq!(plugin.engine.held_voice_count(), 0);
                    assert_eq!(plugin.engine.sustained_voice_count(), 1);
                    plugin.handle_event(&pedal(0.0));
                    assert_eq!(plugin.engine.sustained_voice_count(), 0);
                }
            }
        });
    }

    #[test]
    fn extended_notes_render_finite_distinct_audio_in_both_modes() {
        with_circuit_stack(|| {
            for mode in [CircuitMode::Fast, CircuitMode::Heavy] {
                for mlp in [false, true] {
                    let render = |note| {
                        let mut plugin = OpenWurliUi::default();
                        set_extended(&mut plugin, true);
                        plugin.sync_params();
                        plugin.engine.set_circuit_mode(mode);
                        plugin.engine.set_mlp_enabled(mlp);
                        plugin.engine.warm_up();
                        plugin.handle_event(&note_on(0, note));
                        let mut audio = vec![0.0; 16_384];
                        for block in audio.chunks_mut(256) {
                            plugin.render_block(block, || None);
                        }
                        assert!(audio.iter().all(|s| s.is_finite()), "{mode:?}: MIDI {note}");
                        assert!(
                            audio.iter().any(|s| s.abs() > 1e-6),
                            "{mode:?}: MIDI {note} silent"
                        );
                        assert_eq!(plugin.engine.nan_guard_fires(), 0);
                        audio
                    };
                    for (edge, notes) in [(33, [0, 21, 32]), (96, [97, 108, 127])] {
                        let boundary = render(edge);
                        for note in notes {
                            let extended = render(note);
                            let difference: f32 = boundary
                                .iter()
                                .zip(extended)
                                .map(|(a, b)| (a - b).abs())
                                .sum();
                            assert!(difference > 0.01, "{mode:?}: MIDI {note} still clamped");
                        }
                    }
                }
            }
        });
    }

    #[test]
    fn old_states_disable_extended_notes_and_saved_values_survive() {
        let mut state = PluginState {
            version: "0.1.1".into(),
            params: Default::default(),
            fields: Default::default(),
        };
        OpenWurliUi::filter_state(&mut state);
        assert!(matches!(
            state.params["extended_notes"],
            nih_plug::wrapper::state::ParamValue::Bool(false)
        ));
        state.params.insert(
            "extended_notes".into(),
            nih_plug::wrapper::state::ParamValue::Bool(true),
        );
        OpenWurliUi::filter_state(&mut state);
        assert!(matches!(
            state.params["extended_notes"],
            nih_plug::wrapper::state::ParamValue::Bool(true)
        ));
    }

    #[test]
    fn sustain_holds_and_releases_a_note() {
        with_circuit_stack(|| {
            let mut plugin = OpenWurliUi::default();
            plugin.handle_event(&note_on(0, 60));
            plugin.handle_event(&pedal(1.0));
            plugin.handle_event(&note_off(0, 60));
            assert_eq!(plugin.engine.sustained_voice_count(), 1);
            plugin.handle_event(&pedal(0.0));
            assert_eq!(plugin.engine.sustained_voice_count(), 0);
        });
    }

    #[test]
    fn extended_reed_decay_reaches_the_audio_path_in_both_modes() {
        with_circuit_stack(|| {
            let params = OpenWurliUiParams::default();
            assert_eq!(params.reed_decay.preview_plain(0.0), 0.5);
            assert_eq!(params.reed_decay.preview_plain(1.0), 20.0);
            for mode in [CircuitMode::Fast, CircuitMode::Heavy] {
                let render = |decay| {
                    let mut engine = WurliEngine::new(44_100.0);
                    engine.set_circuit_mode(mode);
                    engine.set_noise_enabled(false);
                    engine.set_reed_decay(decay);
                    engine.note_on(60, 0.8);
                    let mut samples = vec![0.0; 16_384];
                    for chunk in samples.chunks_mut(256) {
                        engine.render(chunk);
                    }
                    assert!(samples.iter().all(|s| s.is_finite()));
                    samples
                };
                let previous_max = render(2.0);
                let extended = render(20.0);
                let difference: f32 = previous_max
                    .iter()
                    .zip(&extended)
                    .map(|(a, b)| (a - b).abs())
                    .sum();
                assert!(difference > 0.01, "{mode:?}: decay still clamped at 2x");
            }
        });
    }

    #[test]
    fn fast_finish_preserves_silence_and_controls_sag() {
        let mut finish = FastFinish::new(44_100.0);
        let original = [0.2; 1024];
        let mut clean = original;
        finish.process(&mut clean, false);
        assert_eq!(clean, original);

        for sag in [false, true] {
            let mut silent = [0.0; 1024];
            finish.process(&mut silent, sag);
            assert_eq!(silent, [0.0; 1024]);
        }

        let mut sagged = original;
        finish.process(&mut sagged, true);
        assert!(sagged[1023] < original[1023]);
        assert!(sagged[1023] > 0.0);
        finish.reset();
        assert_eq!(finish.sag_envelope, 0.0);
    }

    #[test]
    fn host_parameter_ids_and_upstream_defaults_are_stable() {
        let params = OpenWurliUiParams::default();
        let ids: Vec<_> = params
            .param_map()
            .into_iter()
            .map(|(id, _, _)| id)
            .collect();
        assert_eq!(
            ids,
            [
                "volume",
                "trem_depth",
                "speaker",
                "cpu_mode",
                "mlp",
                "reed_decay",
                "hammer_hardness",
                "pickup_drive",
                "trem_response",
                "noise_enable",
                "noise_gain",
                "extended_notes",
                "rail_sag",
            ]
        );
        assert_eq!(params.volume.default_plain_value(), 0.5);
        assert_eq!(params.tremolo_depth.default_plain_value(), 0.5);
        assert_eq!(params.speaker_character.default_plain_value(), 0.0);
        assert!(!params.mlp_enabled.default_plain_value());
        assert_eq!(params.cpu_mode.default_plain_value(), CpuMode::Fast);
        assert_eq!(params.reed_decay.default_plain_value(), 1.0);
        assert_eq!(params.hammer_hardness.default_plain_value(), 1.0);
        assert_eq!(params.pickup_drive.default_plain_value(), 1.0);
        assert_eq!(params.tremolo_response.default_plain_value(), 1.0);
        assert!(!params.noise_enabled.default_plain_value());
        assert_eq!(params.noise_gain.default_plain_value(), 1.0);
        assert!(!params.extended_notes.default_plain_value());
        assert!(!params.rail_sag.default_plain_value());
        assert!(params.serialize_fields().contains_key("editor-state"));
    }
}
