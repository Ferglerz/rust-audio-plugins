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
fn cpu_mode_restore_initializes_selected_audio() {
    with_circuit_stack(|| {
        let mut plugin = OpenWurliUi::default();
        for (saved_id, expected) in [("heavy", CircuitMode::Heavy), ("fast", CircuitMode::Fast)] {
            let mut state = PluginState {
                version: "0.1.0".into(),
                params: Default::default(),
                fields: Default::default(),
            };
            state.params.insert(
                "cpu_mode".into(),
                nih_plug::wrapper::state::ParamValue::String(saved_id.into()),
            );
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
            reference.set_mlp_enabled(true);
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
fn default_output_uses_upstream_fourteen_db_alignment() {
    with_circuit_stack(|| {
        let mut plugin = OpenWurliUi::default();
        plugin.params.volume.smoothed.reset(0.5);
        let mut output = [0.1, -0.1, 0.0];
        plugin.finish_output(&mut output);
        assert!((output[0] - 0.5011872).abs() < 1e-6);
        assert!((output[1] + 0.5011872).abs() < 1e-6);
        assert_eq!(output[2], 0.0);
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
    // At a sustained 0.2 level, Extra Sag should settle near -10.6 dB.
    // The former strength settled near -1.9 dB and fails this bound.
    let mut sustained = [0.2; 44_100];
    finish.process(&mut sustained, true);
    let gain = sustained[44_099] / 0.2;
    assert!((0.29..0.30).contains(&gain), "sustained sag gain: {gain}");
    finish.process(&mut sustained, false);
    assert_eq!(finish.sag_envelope, 0.0);
    finish.reset();
    assert_eq!(finish.sag_envelope, 0.0);
}

#[test]
fn drive_and_volume_drive_range_ends_at_1_04() {
    let params = OpenWurliUiParams::default();
    assert_eq!(params.pickup_drive.preview_plain(0.0), 0.5);
    assert_eq!(params.pickup_drive.preview_plain(1.0), 1.04);
    assert!((params.pickup_drive.preview_plain(0.5) - 0.77).abs() < 1e-6);
    assert_eq!(params.pickup_drive.default_plain_value(), 1.0);
    // The other character controls retain their existing ranges.
    assert_eq!(params.hammer_hardness.preview_plain(1.0), 2.0);
    assert_eq!(params.tremolo_response.preview_plain(1.0), 2.0);
}

#[test]
fn drive_and_volume_saved_drive_values_respect_the_new_ceiling() {
    for (saved, expected) in [(0.5, 0.5), (1.0, 1.0), (1.04, 1.04), (2.0, 1.04)] {
        let mut state = PluginState {
            version: "0.1.1".into(),
            params: Default::default(),
            fields: Default::default(),
        };
        state.params.insert(
            "pickup_drive".into(),
            nih_plug::wrapper::state::ParamValue::F32(saved),
        );
        OpenWurliUi::filter_state(&mut state);
        let nih_plug::wrapper::state::ParamValue::F32(actual) = state.params["pickup_drive"] else {
            panic!("Pickup Drive must remain a float");
        };
        assert_eq!(actual, expected);
    }
}

#[test]
fn drive_and_volume_output_scales_linearly_even_with_extra_sag() {
    with_circuit_stack(|| {
        for mode in [CpuMode::Fast, CpuMode::Heavy] {
            for sag in [false, true] {
                let render = |volume| {
                    let mut plugin = OpenWurliUi::default();
                    let params = Arc::get_mut(&mut plugin.params).unwrap();
                    params.cpu_mode = EnumParam::new("CPU Mode", mode);
                    params.rail_sag = BoolParam::new("Extra Sag", sag);
                    params.volume = FloatParam::new(
                        "Volume",
                        volume,
                        FloatRange::Linear { min: 0.0, max: 1.0 },
                    )
                    .with_smoother(SmoothingStyle::Linear(5.0));
                    params.volume.smoothed.reset(volume);
                    plugin.sync_params();
                    plugin.engine.warm_up();
                    for note in [48, 60, 64, 67] {
                        plugin.handle_event(&note_on(0, note));
                    }
                    let mut audio = vec![0.0; 4096];
                    for block in audio.chunks_mut(256) {
                        plugin.render_block(block, || None);
                        plugin.finish_output(block);
                    }
                    assert!(audio.iter().all(|s| s.is_finite()));
                    (audio, plugin.finish.sag_envelope)
                };
                let (quiet, quiet_sag) = render(0.5);
                let (loud, loud_sag) = render(1.0);
                let (muted, muted_sag) = render(0.0);
                assert!(quiet.iter().any(|s| s.abs() > 1e-6));
                assert!(
                    quiet
                        .iter()
                        .zip(&loud)
                        .all(|(a, b)| (2.0 * a - b).abs() < 1e-6),
                    "{mode:?}, sag={sag}: Volume must scale output without changing tone"
                );
                assert_eq!(quiet_sag, loud_sag, "Volume changed the sag envelope");
                assert_eq!(quiet_sag, muted_sag, "Mute changed the sag envelope");
                assert!(muted.iter().all(|s| *s == 0.0));
            }
        }
    });
}

#[test]
fn extra_sag_reduces_rendered_chords_in_both_modes() {
    with_circuit_stack(|| {
        for mode in [CircuitMode::Fast, CircuitMode::Heavy] {
            let mut engine = WurliEngine::new(44_100.0);
            engine.set_circuit_mode(mode);
            engine.set_noise_enabled(false);
            for note in [48, 60, 64, 67] {
                engine.note_on_extended(note, 0.8);
            }
            let mut clean = vec![0.0; 44_100];
            for chunk in clean.chunks_mut(256) {
                engine.render(chunk);
            }
            let mut sagged = clean.clone();
            FastFinish::new(44_100.0).process(&mut sagged, true);
            let energy = |samples: &[f32]| samples.iter().map(|s| s * s).sum::<f32>();
            let reduction_db = 10.0 * (energy(&clean) / energy(&sagged)).log10();
            eprintln!("{mode:?} Extra Sag chord reduction: {reduction_db:.2} dB");
            assert!(sagged.iter().all(|s| s.is_finite()));
            assert!(
                reduction_db > 1.0,
                "{mode:?}: sag reduction {reduction_db} dB"
            );
        }
    });
}

#[test]
fn restored_editor_scale_keeps_the_new_artwork_dimensions() {
    let params = OpenWurliUiParams::default();
    let mut fields = params.serialize_fields();
    fields.insert("editor-state".into(), r#"{"scale_factor":0.91}"#.into());
    params.deserialize_fields(&fields);
    assert_eq!(params.editor_state.inner_logical_size(), (720, 556));
    assert_eq!(params.editor_state.user_scale_factor(), 0.91);
    assert_eq!(params.editor_state.scaled_logical_size(), (655, 506));
}

#[test]
fn host_parameter_ids_and_control_defaults_are_stable() {
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
            "reed_decay",
            "hammer_hardness",
            "pickup_drive",
            "trem_response",
            "extended_notes",
            "rail_sag",
        ]
    );
    assert_eq!(params.volume.default_plain_value(), 0.5);
    assert_eq!(params.tremolo_depth.default_plain_value(), 0.5);
    assert_eq!(params.speaker_character.default_plain_value(), 0.0);
    assert_eq!(params.cpu_mode.default_plain_value(), CpuMode::Fast);
    assert_eq!(params.reed_decay.default_plain_value(), 1.0);
    assert_eq!(params.hammer_hardness.default_plain_value(), 1.0);
    assert_eq!(params.pickup_drive.default_plain_value(), 1.0);
    assert_eq!(params.tremolo_response.default_plain_value(), 1.0);
    assert!(!params.extended_notes.default_plain_value());
    assert!(!params.rail_sag.default_plain_value());
    assert!(params.serialize_fields().contains_key("editor-state"));
}

#[test]
fn test_clap_wildcard_note_on_is_ignored() {
    with_circuit_stack(|| {
        // CLAP key -1 arrives as 255; it must not play the clamped top key.
        let mut plugin = OpenWurliUi::default();
        plugin.handle_event(&note_on(0, 255));
        assert_eq!(plugin.engine.active_voice_count(), 0);
    });
}

#[test]
fn test_clap_wildcard_note_off_releases_every_key() {
    with_circuit_stack(|| {
        let mut plugin = OpenWurliUi::default();
        for n in [48u8, 60, 64, 67, 96] {
            plugin.handle_event(&note_on(0, n));
        }
        assert_eq!(plugin.engine.held_voice_count(), 5);
        plugin.handle_event(&note_off(0, 255));
        assert_eq!(plugin.engine.held_voice_count(), 0);
    });
}

#[test]
fn test_all_notes_off_cc_releases_every_key() {
    with_circuit_stack(|| {
        let mut plugin = OpenWurliUi::default();
        for n in [48u8, 60, 64] {
            plugin.handle_event(&note_on(0, n));
        }
        plugin.handle_event(&cc(control_change::ALL_NOTES_OFF, 0.0));
        assert_eq!(plugin.engine.held_voice_count(), 0);
    });
}

fn cc(cc: u8, value: f32) -> NoteEvent<()> {
    NoteEvent::MidiCC {
        timing: 0,
        channel: 0,
        cc,
        value,
    }
}

fn choke(note: u8) -> NoteEvent<()> {
    NoteEvent::Choke {
        timing: 0,
        voice_id: None,
        channel: 0,
        note,
    }
}

#[test]
fn test_choke_stops_only_its_note() {
    with_circuit_stack(|| {
        let mut plugin = OpenWurliUi::default();
        plugin.handle_event(&note_on(0, 60));
        plugin.handle_event(&note_on(0, 64));
        plugin.handle_event(&choke(60));
        assert_eq!(plugin.engine.active_voice_count(), 1);
    });
}

#[test]
fn test_wildcard_choke_and_all_sound_off_stop_everything() {
    with_circuit_stack(|| {
        let mut plugin = OpenWurliUi::default();
        for n in [48u8, 60, 64] {
            plugin.handle_event(&note_on(0, n));
        }
        plugin.handle_event(&choke(255));
        assert_eq!(plugin.engine.active_voice_count(), 0);

        for n in [48u8, 60, 64] {
            plugin.handle_event(&note_on(0, n));
        }
        plugin.handle_event(&cc(control_change::ALL_SOUND_OFF, 0.0));
        assert_eq!(plugin.engine.active_voice_count(), 0);
    });
}

#[test]
fn test_all_notes_off_cc_keeps_pedal_held_notes_until_pedal_up() {
    with_circuit_stack(|| {
        let mut plugin = OpenWurliUi::default();
        plugin.handle_event(&cc(control_change::DAMPER_PEDAL, 1.0));
        for n in [48u8, 60] {
            plugin.handle_event(&note_on(0, n));
        }
        plugin.handle_event(&cc(control_change::ALL_NOTES_OFF, 0.0));
        assert_eq!(plugin.engine.held_voice_count(), 0);
        assert_eq!(plugin.engine.sustained_voice_count(), 2);
        plugin.handle_event(&cc(control_change::DAMPER_PEDAL, 0.0));
        assert_eq!(plugin.engine.sustained_voice_count(), 0);
    });
}

#[test]
fn test_mode_messages_cause_all_notes_off() {
    with_circuit_stack(|| {
        for mode_cc in 124u8..=127 {
            let mut plugin = OpenWurliUi::default();
            for n in [48u8, 60, 64] {
                plugin.handle_event(&note_on(0, n));
            }
            plugin.handle_event(&cc(mode_cc, 0.0));
            assert_eq!(plugin.engine.held_voice_count(), 0, "CC {mode_cc}");
        }
    });
}

#[test]
fn test_reset_all_controllers_lifts_the_pedal() {
    with_circuit_stack(|| {
        let mut plugin = OpenWurliUi::default();
        plugin.handle_event(&cc(control_change::DAMPER_PEDAL, 1.0));
        plugin.handle_event(&note_on(0, 60));
        plugin.handle_event(&note_off(0, 60)); // → Sustained
        plugin.handle_event(&cc(control_change::RESET_ALL_CONTROLLERS, 0.0));
        assert!(!plugin.engine.is_sustain_held());
        assert_eq!(plugin.engine.sustained_voice_count(), 0);
    });
}

#[test]
fn extended_choke_keeps_pitch_identity_after_range_is_disabled() {
    with_circuit_stack(|| {
        for mode in [CircuitMode::Fast, CircuitMode::Heavy] {
            let mut plugin = OpenWurliUi::default();
            plugin.engine.set_circuit_mode(mode);
            set_extended(&mut plugin, true);
            for note in [0, 33, 96, 127] {
                plugin.handle_event(&note_on(0, note));
            }
            set_extended(&mut plugin, false);
            plugin.handle_event(&choke(0));
            plugin.handle_event(&choke(127));
            assert_eq!(plugin.engine.held_voice_count(), 2);
            plugin.handle_event(&note_off(0, 33));
            plugin.handle_event(&note_off(0, 96));
            assert_eq!(plugin.engine.held_voice_count(), 0);
        }
    });
}

#[test]
fn all_sound_off_is_sample_accurate_and_fades_without_resetting_the_chain() {
    with_circuit_stack(|| {
        for mode in [CircuitMode::Fast, CircuitMode::Heavy] {
            let mut plugin = OpenWurliUi::default();
            let mut reference = OpenWurliUi::default();
            for instrument in [&mut plugin, &mut reference] {
                instrument.engine = WurliEngine::new_with_circuit_mode(44100.0, mode);
                instrument.sync_params();
                instrument.engine.warm_up();
                instrument.handle_event(&note_on(0, 60));
                instrument.render_block(&mut [0.0; 2048], || None);
            }
            let mut event = cc(control_change::ALL_SOUND_OFF, 0.0);
            if let NoteEvent::MidiCC { timing, .. } = &mut event {
                *timing = 128;
            }
            let mut events = [event].into_iter();
            let mut actual = [0.0; 2048];
            let mut unchoked = [0.0; 2048];
            plugin.render_block(&mut actual, || events.next());
            reference.render_block(&mut unchoked, || None);
            assert_eq!(&actual[..128], &unchoked[..128]);
            assert!(actual[128..]
                .iter()
                .zip(&unchoked[128..])
                .any(|(a, b)| (a - b).abs() > 1e-6));
            assert!(actual.iter().all(|s| s.is_finite()));
            assert_eq!(plugin.engine.active_voice_count(), 0);
            assert_eq!(plugin.engine.nan_guard_fires(), 0);
        }
    });
}
