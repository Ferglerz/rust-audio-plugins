use super::*;

impl OpenWurliUi {
    // Exercise the same stereo path the host calls; most reed tests inspect only the left channel.
    fn render_block(&mut self, left: &mut [f32], next: impl FnMut() -> Option<NoteEvent<()>>) {
        self.params
            .volume
            .smoothed
            .reset(self.params.volume.value());
        self.render_stereo(left, &mut vec![0.; left.len()], next);
    }
}

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

#[test]
fn old_heavy_state_loads_as_wurli() {
    use nih_plug::wrapper::state::ParamValue;
    let mut state = PluginState {
        version: "0.2.1".into(),
        params: [("cpu_mode".into(), ParamValue::String("heavy".into()))].into(),
        fields: Default::default(),
    };
    OpenWurliUi::filter_state(&mut state);
    assert!(!state.params.contains_key("cpu_mode"));
    for m in ["c", "l", "n", "t", "duo"] {
        for field in ["enable", "pedal"] {
            assert!(matches!(
                state.params[&format!("smod_{m}_{field}")],
                ParamValue::Bool(false)
            ));
        }
    }
    assert!(matches!(
        state.params["vv64_enable"],
        ParamValue::Bool(false)
    ));
    assert!(matches!(
        state.params["elepian_enable"],
        ParamValue::Bool(false)
    ));
    assert!(matches!(
        state.params["neo_enable"],
        ParamValue::Bool(false)
    ));
    assert!(matches!(
        state.params["pickup_d6_bridge"],
        ParamValue::F32(0.)
    ));
    assert!(matches!(
        state.params["pickup_pn_return"],
        ParamValue::F32(1.)
    ));
    assert!(matches!(
        state.params["storytone_enable"],
        ParamValue::Bool(false)
    ));
    assert!(matches!(&state.params["clavinet_model"], ParamValue::String(v) if v == "d6"));
    assert!(matches!(&state.params["early_reed_model"], ParamValue::String(v) if v == "pianet_n"));
    assert!(matches!(
        state.params["clavinet_e7_power"],
        ParamValue::Bool(true)
    ));
    assert!(matches!(&state.params["grand_model"], ParamValue::String(v) if v == "cp80"));
    assert!(matches!(
        state.params["grand_70_original"],
        ParamValue::Bool(false)
    ));
    assert!(matches!(&state.params["piano_type"], ParamValue::String(v) if v == "wurli"));
}

#[test]
fn older_mk8_and_vintage_vibe_presets_recall_their_original_boards() {
    use nih_plug::wrapper::state::ParamValue;
    let params = OpenWurliUiParams::default();
    let parameter_ids = params.param_map();
    for (legacy_id, option_id) in [("mk8_enable", "mk8_fx_fitted"), ("vv64_enable", "stereovibe_enable")] {
        assert!(parameter_ids.iter().any(|(id, _, _)| id == option_id));
        let mut state = PluginState {
            version: "0.3.0".into(),
            params: [(legacy_id.to_owned(), ParamValue::Bool(true))].into(),
            fields: Default::default(),
        };
        OpenWurliUi::filter_state(&mut state);
        assert!(matches!(state.params[legacy_id], ParamValue::Bool(true)));
        assert!(matches!(state.params[option_id], ParamValue::Bool(false)));
        state.params.insert(option_id.to_owned(), ParamValue::Bool(true));
        OpenWurliUi::filter_state(&mut state);
        assert!(matches!(state.params[option_id], ParamValue::Bool(true)));
    }
}

#[test]
fn recalled_jack_controls_reach_each_instrument() {
    use nih_plug::wrapper::state::ParamValue;
    let mut state = PluginState {
        version: "0.3.0".into(),
        params: Default::default(),
        fields: Default::default(),
    };
    OpenWurliUi::filter_state(&mut state);
    for (id, value) in [
        ("stage_panel_vol", 1.),
        ("stage_cable_pf", 300.),
        ("grand_jack_vol", 1.),
    ] {
        assert!(matches!(state.params[id], ParamValue::F32(x) if x == value));
    }
    state
        .params
        .insert("grand_jack_vol".into(), ParamValue::F32(0.25));
    OpenWurliUi::filter_state(&mut state);
    assert!(matches!(
        state.params["grand_jack_vol"],
        ParamValue::F32(0.25)
    ));
    let mut p = OpenWurliUiParams::default();
    let pot = |v| FloatParam::new("Volume", v, FloatRange::Linear { min: 0., max: 1. });
    p.stage_panel_volume = pot(0.5);
    p.grand_jack_volume = pot(0.25);
    p.stage_cable = FloatParam::new(
        "Cable",
        750.,
        FloatRange::Linear {
            min: 100.,
            max: 1000.,
        },
    );
    assert_eq!(
        (p.stage_controls().volume, p.stage_controls().cable_pf),
        (0.5, 750.)
    );
    assert_eq!(p.grand_controls().volume, 0.25);
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
        let mut plugin = OpenWurliUi::default();
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
    });
}

fn set_extended(plugin: &mut OpenWurliUi, enabled: bool) {
    Arc::get_mut(&mut plugin.params).unwrap().extended_notes =
        BoolParam::new("Extended Notes", enabled);
}

#[test]
fn extended_notes_keep_their_identity_and_release_after_disabling() {
    with_circuit_stack(|| {
        let mut plugin = OpenWurliUi::default();
        for note in 0..=127 {
            set_extended(&mut plugin, true);
            plugin.handle_event(&note_on(0, note));
            assert_eq!(
                plugin
                    .engine
                    .count_voices_with_note_in_state(note, openwurli_dsp::engine::VoiceState::Held),
                1,
                "MIDI {note} must retain its actual pitch"
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
    });
}

#[test]
fn extended_notes_render_finite_distinct_audio() {
    with_circuit_stack(|| {
        for mlp in [false, true] {
            let render = |note| {
                let mut plugin = OpenWurliUi::default();
                set_extended(&mut plugin, true);
                plugin.sync_params();
                plugin.engine.set_mlp_enabled(mlp);
                plugin.engine.warm_up();
                plugin.handle_event(&note_on(0, note));
                let mut audio = vec![0.0; 16_384];
                for block in audio.chunks_mut(256) {
                    plugin.render_block(block, || None);
                }
                assert!(audio.iter().all(|s| s.is_finite()), "MIDI {note}");
                assert!(audio.iter().any(|s| s.abs() > 1e-6), "MIDI {note} silent");
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
                    assert!(difference > 0.01, "MIDI {note} still clamped");
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
fn extended_reed_decay_reaches_the_audio_path() {
    with_circuit_stack(|| {
        let params = OpenWurliUiParams::default();
        assert_eq!(params.reed_decay.preview_plain(0.0), 0.5);
        assert_eq!(params.reed_decay.preview_plain(1.0), 20.0);
        let render = |decay| {
            let mut engine = WurliEngine::new(44_100.0);
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
        assert!(difference > 0.01, "decay still clamped at 2x");
    });
}

#[test]
fn pickup_height_voicing_changes_new_strikes() {
    use nih_plug::wrapper::state::ParamValue;
    let mut legacy = PluginState {
        version: "0.2.1".into(),
        params: Default::default(),
        fields: Default::default(),
    };
    OpenWurliUi::filter_state(&mut legacy);
    assert!(matches!(
        legacy.params["pickup_height"],
        ParamValue::F32(0.0)
    ));
    with_circuit_stack(|| {
        let limit = openwurli_dsp::pickup::MAX_HEIGHT_TRIM_MM as f32;
        for note in [52, 53] {
            let render = |height| {
                let mut plugin = OpenWurliUi::default();
                let params = Arc::get_mut(&mut plugin.params).unwrap();
                params.pickup_height = FloatParam::new(
                    "Pickup Height",
                    height,
                    FloatRange::Linear {
                        min: -limit,
                        max: limit,
                    },
                );
                plugin.sync_params();
                plugin.engine.warm_up();
                plugin.handle_event(&note_on(0, note));
                let mut samples = vec![0.; 4096];
                for chunk in samples.chunks_mut(128) {
                    plugin.render_block(chunk, || None);
                }
                assert!(samples.iter().all(|s| s.is_finite()));
                assert_eq!(plugin.engine.nan_guard_fires(), 0);
                samples
            };
            let stock = render(0.);
            for height in [-limit, limit] {
                let voiced = render(height);
                let distance: f32 = voiced.iter().zip(&stock).map(|(a, b)| (a - b).abs()).sum();
                assert!(distance > 0.01, "note {note} height {height}");
            }
        }
    });
}

#[test]
fn wurli_controls_revoice_a_pedal_held_note() {
    with_circuit_stack(|| {
        let render = |note, depth, height, response| {
            let mut plugin = OpenWurliUi::default();
            Arc::get_mut(&mut plugin.params).unwrap().tremolo_depth =
                FloatParam::new("Depth", depth, FloatRange::Linear { min: 0., max: 1. });
            plugin.sync_params();
            plugin.engine.warm_up();
            plugin.handle_event(&pedal(1.));
            plugin.handle_event(&note_on(0, note));
            plugin.render_block(&mut [0.; 4096], || None);
            plugin.handle_event(&note_off(0, note));
            let params = Arc::get_mut(&mut plugin.params).unwrap();
            params.pickup_height = FloatParam::new(
                "Height",
                height,
                FloatRange::Linear {
                    min: -0.12,
                    max: 0.12,
                },
            );
            params.tremolo_response = FloatParam::new(
                "Response",
                response,
                FloatRange::Linear {
                    min: 0.125,
                    max: 8.,
                },
            );
            let mut audio = vec![0.; 24_576];
            for chunk in audio.chunks_mut(128) {
                plugin.render_block(chunk, || None);
            }
            assert!(audio.iter().all(|s| s.is_finite()));
            audio
        };
        let difference = |a: &[f32], b: &[f32]| {
            let energy: f32 = a.iter().map(|x| x * x).sum();
            (a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f32>() / energy).sqrt()
        };
        for note in [48, 64] {
            let low = render(note, 0., -0.12, 1.);
            let high = render(note, 0., 0.12, 1.);
            assert!(
                difference(&low[512..], &high[512..]) > 0.05,
                "pickup height on {note}"
            );
        }
        let fast = render(60, 0.5, 0., 0.125);
        let slow = render(60, 0.5, 0., 8.);
        assert!(
            difference(&fast[8192..], &slow[8192..]) > 0.08,
            "photocell response at half depth"
        );
        assert_eq!(render(60, 0., 0., 0.125), render(60, 0., 0., 8.));
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
fn repeated_inactive_resets_preserve_wurli_pcm_after_notes_and_pedal_history() {
    with_circuit_stack(|| {
        let return_to_wurli = |force_legacy_reset: bool| {
            let mut p = OpenWurliUi::default();
            let mut left = [0.; 512];
            let mut history =
                [pedal(1.), note_on(7, 55), note_off(80, 55), note_on(200, 64)].into_iter();
            p.render_block(&mut left, || history.next());
            assert!(left.iter().any(|x| x.abs() > 1e-6));
            Arc::get_mut(&mut p.params).unwrap().select_model(Instrument::Mk8);
            let mut other = [cc(11, 0.4), pedal(0.), note_on(0, 48)].into_iter();
            p.render_block(&mut left, || other.next());
            assert!(!p.inactive_wurli_settled);
            p.reset();
            assert!(p.inactive_wurli_settled);
            for _ in 0..2 {
                p.render_block(&mut left, || None);
                // Model what the old wrapper did to Wurli on every transport reset.
                if force_legacy_reset {
                    p.inactive_wurli_settled = false;
                }
                p.reset();
            }
            Arc::get_mut(&mut p.params).unwrap().select_model(Instrument::Piano(PianoType::Wurli));
            let mut result = vec![0.; 4096];
            let mut notes =
                [note_on(16, 60), note_on(93, 67), note_off(900, 60)].into_iter();
            p.render_block(&mut result, || notes.next());
            assert!(result.iter().any(|x| x.abs() > 1e-6));
            p.reset();
            assert!(!p.inactive_wurli_settled);
            result
        };
        assert_eq!(return_to_wurli(false), return_to_wurli(true));
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
    assert_eq!(params.hammer_hardness.preview_plain(1.0), 2.0);
    assert_eq!(params.tremolo_response.preview_plain(0.0), 0.125);
    assert_eq!(params.tremolo_response.preview_plain(1.0), 8.0);
    assert!((params.tremolo_response.default_normalized_value() - 1.0 / 3.0).abs() < 1e-6);
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
        for sag in [false, true] {
            let render = |volume| {
                let mut plugin = OpenWurliUi::default();
                let params = Arc::get_mut(&mut plugin.params).unwrap();
                params.rail_sag = BoolParam::new("Extra Sag", sag);
                params.volume =
                    FloatParam::new("Volume", volume, FloatRange::Linear { min: 0.0, max: 1.0 })
                        .with_smoother(SmoothingStyle::Linear(5.0));
                params.volume.smoothed.reset(volume);
                plugin.sync_params();
                plugin.engine.warm_up();
                for note in [48, 60, 64, 67] {
                    plugin.handle_event(&note_on(0, note));
                }
                let mut audio = vec![0.0; 4096];
                for block in audio.chunks_mut(256) {
                    plugin.render_stereo(block, &mut [0.; 256], || None);
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
                "sag={sag}: Volume must scale output without changing tone"
            );
            assert_eq!(quiet_sag, loud_sag, "Volume changed the sag envelope");
            assert_eq!(quiet_sag, muted_sag, "Mute changed the sag envelope");
            assert!(muted.iter().all(|s| *s == 0.0));
        }
    });
}

#[test]
fn extra_sag_reduces_rendered_chords() {
    with_circuit_stack(|| {
        let mut engine = WurliEngine::new(44_100.0);
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
        eprintln!("Extra Sag chord reduction: {reduction_db:.2} dB");
        assert!(sagged.iter().all(|s| s.is_finite()));
        assert!(reduction_db > 1.0, "sag reduction {reduction_db} dB");
    });
}

#[test]
fn restored_editor_scale_keeps_the_new_artwork_dimensions() {
    let params = OpenWurliUiParams::default();
    let mut fields = params.serialize_fields();
    fields.insert("editor-state".into(), r#"{"scale_factor":0.91}"#.into());
    params.deserialize_fields(&fields);
    assert_eq!(params.editor_state.inner_logical_size(), (728, 588));
    assert_eq!(params.editor_state.user_scale_factor(), 0.91);
    assert_eq!(params.editor_state.scaled_logical_size(), (662, 535));
}

#[test]
fn host_parameter_ids_and_control_defaults_are_stable() {
    let params = OpenWurliUiParams::default();
    let ids: Vec<_> = params
        .param_map()
        .into_iter()
        .map(|(id, _, _)| id)
        .collect();
    for id in [
        "volume",
        "trem_depth",
        "speaker",
        "piano_type",
        "reed_decay",
        "hammer_hardness",
        "pickup_drive",
        "trem_response",
        "extended_notes",
        "rail_sag",
        "pickup_height",
    ] {
        assert!(ids.iter().any(|name| name == id));
    }
    assert!(!ids.iter().any(|name| name == "cpu_mode"));
    assert_eq!(
        ids.len(),
        ids.iter().collect::<std::collections::HashSet<_>>().len()
    );
    assert_eq!(params.volume.default_plain_value(), 0.5);
    assert_eq!(params.tremolo_depth.default_plain_value(), 0.5);
    assert_eq!(params.speaker_character.default_plain_value(), 0.0);
    assert_eq!(params.piano.default_plain_value(), PianoType::Wurli);
    assert_eq!(params.reed_decay.default_plain_value(), 1.0);
    assert_eq!(params.hammer_hardness.default_plain_value(), 1.0);
    assert_eq!(params.pickup_drive.default_plain_value(), 1.0);
    assert_eq!(params.pickup_height.default_plain_value(), 0.0);
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
        let mut plugin = OpenWurliUi::default();
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
    });
}

#[test]
fn all_sound_off_is_sample_accurate_and_fades_without_resetting_the_chain() {
    with_circuit_stack(|| {
        let mut plugin = OpenWurliUi::default();
        let mut reference = OpenWurliUi::default();
        for instrument in [&mut plugin, &mut reference] {
            instrument.engine = WurliEngine::new(44100.0);
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
    });
}

#[test]
fn keyboard_audition_renders_and_releases_only_the_mouse_voice() {
    with_circuit_stack(|| {
        let mut plugin = OpenWurliUi::default();
        let mut reference = OpenWurliUi::default();
        plugin.sync_params();
        reference.sync_params();
        let mut audio = [0.0; 512];
        let mut midi_audio = [0.0; 512];
        let mut midi = Some(note_on(0, 61));
        reference.render_block(&mut midi_audio, || midi.take());
        plugin.params.keyboard.audition(Some(61));
        plugin.render_block(&mut audio, || None);
        assert_eq!(audio, midi_audio);
        assert!(audio.iter().any(|s| s.abs() > 1e-5));

        // An unmatched host release cannot stop an editor-owned voice.
        plugin.handle_event(&note_off(0, 61));
        assert_eq!(plugin.engine.held_voice_count(), 1);
        plugin.handle_event(&note_on(0, 61));
        plugin.handle_event(&pedal(1.0));
        plugin.params.keyboard.audition(None);
        plugin.render_block(&mut audio, || None);
        assert_eq!(plugin.engine.held_voice_count(), 1);
        assert_eq!(plugin.engine.sustained_voice_count(), 1);
        plugin.handle_event(&note_off(0, 61));
        assert_eq!(plugin.engine.playing_notes(), 1 << 61);
        plugin.handle_event(&pedal(0.0));
        assert_eq!(plugin.engine.playing_notes(), 0);

        plugin.params.keyboard.audition(Some(127));
        plugin.render_block(&mut audio, || None);
        assert_eq!(plugin.engine.playing_notes(), 0);
        set_extended(&mut plugin, true);
        plugin.params.keyboard.audition(Some(127));
        plugin.render_block(&mut audio, || None);
        assert_eq!(plugin.engine.playing_notes(), 1 << 127);
        set_extended(&mut plugin, false);
        plugin.params.keyboard.audition(None);
        plugin.render_block(&mut audio, || None);
        assert_eq!(plugin.engine.playing_notes(), 0);
    });
}

mod calibration;

#[test]
fn switching_models_routes_notes_and_preserves_shared_controls() {
    with_circuit_stack(|| {
        let mut p = OpenWurliUi::default();
        p.params.volume.smoothed.reset(0.5);
        let (mut l, mut r) = ([0.; 1024], [0.; 1024]);
        let mut renders = Vec::new();
        for model in PianoType::ALL {
            Arc::get_mut(&mut p.params).unwrap().piano = EnumParam::new("Piano", model);
            let mut events = [note_on(300, 60)].into_iter();
            p.render_stereo(&mut l, &mut r, || events.next());
            assert!(l[300..].iter().any(|x| x.abs() > 0.001));
            assert!(l[128..300].iter().all(|x| x.abs() < 1e-5));
            assert_eq!(l, r);
            renders.push(l);
            assert_eq!(
                p.engine.playing_notes(),
                if model == PianoType::Wurli {
                    1 << 60
                } else {
                    0
                }
            );
            assert_eq!(
                p.stage.playing_notes(),
                if model == PianoType::Stage {
                    1 << 60
                } else {
                    0
                }
            );
            assert_eq!(
                p.grand.pressed(),
                if model == PianoType::Grand {
                    1 << 60
                } else {
                    0
                }
            );
        }
        assert!(renders.windows(2).all(|pair| pair[0] != pair[1]));
        Arc::get_mut(&mut p.params).unwrap().grand.tremolo = BoolParam::new("Tremolo", true);
        for _ in 0..4 {
            p.render_stereo(&mut l, &mut r, || None);
        }
        assert!(l.iter().zip(r).any(|(l, r)| (l - r).abs() > 0.001));
        p.params.volume.smoothed.reset(0.);
        p.render_stereo(&mut l, &mut r, || None);
        assert_eq!((l, r), ([0.; 1024], [0.; 1024]));
    });
}

#[test]
fn extended_range_routes_extreme_notes_and_releases_them_after_disabling() {
    with_circuit_stack(|| {
        let mut p = OpenWurliUi::default();
        p.params.volume.smoothed.reset(0.5);
        for model in PianoType::ALL {
            Arc::get_mut(&mut p.params).unwrap().piano = EnumParam::new("Piano", model);
            for note in [0, 127] {
                let keys = |p: &OpenWurliUi| match model {
                    PianoType::Wurli => p.engine.playing_notes(),
                    PianoType::Stage => p.stage.playing_notes(),
                    PianoType::Grand => p.grand.pressed(),
                };
                let (mut l, mut r) = ([0.; 2048], [0.; 2048]);
                let mut on = Some(note_on(0, note));
                p.render_stereo(&mut l, &mut r, || on.take());
                assert_eq!(keys(&p), 0);
                set_extended(&mut p, true);
                let mut on = Some(note_on(0, note));
                p.render_stereo(&mut l, &mut r, || on.take());
                assert_eq!(keys(&p), 1 << note);
                assert!(
                    l.iter().all(|v| v.is_finite()) && l.iter().any(|v| v.abs() > 1e-6),
                    "{model:?} MIDI {note}"
                );
                set_extended(&mut p, false);
                let mut off = Some(note_off(0, note));
                p.render_stereo(&mut l, &mut r, || off.take());
                assert_eq!(keys(&p), 0);
            }
        }
    });
}

#[test]
fn cp70b_switch_releases_old_course_and_uses_its_native_compass() {
    with_circuit_stack(|| {
        let mut piano = OpenWurliUi::default();
        Arc::get_mut(&mut piano.params).unwrap().piano = EnumParam::new("Piano", PianoType::Grand);
        let mut events = [note_on(0, 21), pedal(1.)].into_iter();
        piano.render_block(&mut [0.; 256], || events.next());
        assert_eq!(piano.grand.pressed(), 1 << 21);
        Arc::get_mut(&mut piano.params).unwrap().grand_model =
            EnumParam::new("Grand Model", GrandModel::Cp70B);
        let mut events = [
            note_on(0, 27),
            note_on(32, 28),
            note_on(64, 100),
            note_on(96, 101),
        ]
        .into_iter();
        let mut sound = [0.; 1024];
        piano.render_block(&mut sound, || events.next());
        assert_eq!(piano.grand.pressed(), (1 << 28) | (1 << 100));
        assert!(piano.pedal && sound.iter().any(|x| x.abs() > 0.0001));
        assert_eq!(piano.other_grand.pressed(), 0);
        Arc::get_mut(&mut piano.params).unwrap().grand_model =
            EnumParam::new("Grand Model", GrandModel::Cp80);
        piano.render_block(&mut sound, || None);
        assert_eq!(piano.grand.pressed(), 0);
        assert_eq!(piano.params.key_range(), 21..=108);
    });
}

#[test]
fn original_cp70_selection_preserves_the_two_legacy_automation_endpoints() {
    with_circuit_stack(|| {
        let mut piano = OpenWurliUi::default();
        let parameters = Arc::get_mut(&mut piano.params).unwrap();
        parameters.piano = EnumParam::new("Piano", PianoType::Grand);
        assert_eq!(
            parameters.grand_model.preview_normalized(GrandModel::Cp80),
            0.
        );
        assert_eq!(
            parameters.grand_model.preview_normalized(GrandModel::Cp70B),
            1.
        );
        let sequence = [
            (GrandModel::Cp70B, false),
            (GrandModel::Cp70B, true),
            (GrandModel::Cp80, true),
            (GrandModel::Cp70B, true),
            (GrandModel::Cp70B, false),
        ];
        let mut signatures = Vec::new();
        for (model, original) in sequence {
            let params = Arc::get_mut(&mut piano.params).unwrap();
            params.grand_model = EnumParam::new("Grand Model", model);
            params.grand_70_original = BoolParam::new("Original CP-70", original);
            let mut silence = [0.; 1024];
            piano.render_block(&mut silence, || None);
            assert_eq!(piano.grand.pressed(), 0);
            assert_eq!(piano.using_original, model == GrandModel::Cp70B && original);
            let mut events = [note_on(0, 93)].into_iter();
            let mut signal = [0.; 4096];
            piano.render_block(&mut signal, || events.next());
            signatures.push(
                signal[512..]
                    .iter()
                    .map(|v| f64::from(*v).powi(2))
                    .sum::<f64>(),
            );
        }
        assert!(signatures.iter().all(|s| s.is_finite() && *s > 0.));
        assert!((signatures[0] / signatures[1] - 1.).abs() > 0.1);
        assert!((signatures[0] / signatures[4] - 1.).abs() < 0.001);
        assert!((signatures[1] / signatures[3] - 1.).abs() < 0.001);
    });
}

#[test]
fn passive_rail_pedals_sustain_while_cc11_leaves_the_hardware_panels_alone() {
    with_circuit_stack(|| {
        for instrument in [Instrument::Elepian, Instrument::Vv64, Instrument::Valente] {
            let take = |pedal: u8| {
                let mut piano = OpenWurliUi::default();
                let params = Arc::get_mut(&mut piano.params).unwrap();
                params.select_model(instrument);
                params.volume.smoothed.reset(OUTPUT_VOLUME_REFERENCE);
                let mut messages = [cc(pedal, 1.), note_on(128, 65)].into_iter();
                let (mut l, mut r) = ([0.; 4096], [0.; 4096]);
                piano.render_stereo(&mut l, &mut r, || messages.next());
                let attack = l;
                let mut up = Some(NoteEvent::NoteOff {
                    timing: 32,
                    voice_id: None,
                    channel: 0,
                    note: 65,
                    velocity: 0.,
                });
                piano.render_stereo(&mut l, &mut r, || up.take());
                piano.render_stereo(&mut l, &mut r, || None);
                let release = l.iter().map(|x| x * x).sum::<f32>();
                (attack, release)
            };
            let normal = take(1);
            assert_eq!(take(11), normal);
            let sustained = take(64);
            // Raising Valente's shared damper rail also excites neighboring reeds.
            assert_eq!(normal.0 == sustained.0, instrument != Instrument::Valente);
            assert!(sustained.1 > normal.1, "{instrument:?}");
        }
    });
}

#[test]
fn claviset_expression_and_damper_follow_the_selected_hardware() {
    with_circuit_stack(|| {
        for model in Instrument::CLAVISET {
            let Instrument::Claviset(which) = model else {
                unreachable!()
            };
            let mut piano = OpenWurliUi::default();
            let p = Arc::get_mut(&mut piano.params).unwrap();
            p.select_model(model);
            p.volume.smoothed.reset(OUTPUT_VOLUME_REFERENCE);
            assert_eq!(p.key_range(), 36..=96);
            let mut notes = [cc(11, 0.4), cc(64, 1.), note_on(64, 60)].into_iter();
            let (mut left, mut right) = ([0.; 3072], [0.; 3072]);
            piano.render_stereo(&mut left, &mut right, || notes.next());
            assert_eq!(left, right);
            assert!(left.iter().all(|x| x.is_finite()) && left.iter().any(|x| x.abs() > 1e-4));
            assert_eq!(piano.clavisets[which as usize].pressed(), 1 << 60);
            let (controls, pedal) = piano.params.claviset.display_state(which);
            assert!((controls.swell - 0.4).abs() < 1e-6);
            assert_eq!(pedal, which == claviset_dsp::Instrument::Claviset200);
            piano.handle_event(&cc(121, 0.));
            let (controls, pedal) = piano.params.claviset.display_state(which);
            assert_eq!((controls.swell, pedal), (1., false));
        }
    });
}

#[test]
fn storytone_pedals_and_swell_reach_the_live_engine() {
    with_circuit_stack(|| {
        let stroke = |controller| {
            let mut piano = OpenWurliUi::default();
            let params = Arc::get_mut(&mut piano.params).unwrap();
            params.select_model(Instrument::Storytone);
            params.volume.smoothed.reset(OUTPUT_VOLUME_REFERENCE);
            assert_eq!(piano.params.key_range(), 21..=108);
            let mut events = [cc(controller, 1.), note_on(10, 64)].into_iter();
            let (mut l, mut r) = ([0.; 4096], [0.; 4096]);
            piano.render_stereo(&mut l, &mut r, || events.next());
            assert_eq!(l, r);
            assert_eq!(piano.storytone.held_notes(), 1 << 64);
            let energy = |b: &[f32]| b.iter().map(|v| v * v).sum::<f32>();
            let attack = energy(&l);
            let mut release = Some(NoteEvent::NoteOff {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 64,
                velocity: 0.,
            });
            piano.render_stereo(&mut l, &mut r, || release.take());
            piano.render_stereo(&mut l, &mut r, || None);
            let tail = energy(&l);
            assert_eq!(piano.storytone.held_notes(), 0);
            let (swell, soft, sustain) = piano.params.storytone.display();
            assert_eq!(swell > 0., controller == 11);
            assert_eq!(soft, controller == 67);
            assert_eq!(sustain, controller == 64);
            piano.handle_event(&cc(121, 0.));
            assert_eq!(piano.params.storytone.display(), (0., false, false));
            (attack, tail)
        };
        let base = stroke(1);
        assert!(base.0 > 0.);
        assert!(stroke(11).0 > base.0);
        assert!(stroke(67).0 < base.0);
        assert!(stroke(64).1 > base.1);
    });
}

#[test]
fn duo_split_and_recording_jacks_route_held_notes_across_the_tap() {
    with_circuit_stack(|| {
        let mut piano = OpenWurliUi::default();
        let params = Arc::get_mut(&mut piano.params).unwrap();
        params.select_model(Instrument::Duo);
        params.duo.split = EnumParam::new(
            "Duo Keyboard Selector",
            crate::duo::Registration::PianetTreble,
        );
        params.duo.recording = BoolParam::new("Duo Separate Recording Outputs", true);
        params.volume.smoothed.reset(OUTPUT_VOLUME_REFERENCE);
        for note in [hohner_duo::LAST_BASS, hohner_duo::LAST_BASS + 1] {
            piano.reset();
            let mut key = Some(note_on(256, note));
            let (mut l, mut r) = ([0.; 4096], [0.; 4096]);
            piano.render_stereo(&mut l, &mut r, || key.take());
            let energy = |a: &[f32]| a.iter().map(|v| v * v).sum::<f32>();
            if note == hohner_duo::LAST_BASS {
                assert!(energy(&l) > 4. * energy(&r));
            } else {
                assert!(energy(&r) > 4. * energy(&l));
            }
            let mut cc = Some(pedal(1.));
            piano.render_stereo(&mut l, &mut r, || cc.take());
            assert!(piano.params.duo.foot_is_b());
            assert_eq!(piano.duo.pressed(), 1 << note);
        }
    });
}

#[test]
fn electra_pedals_change_damping_and_electrical_level_at_the_midi_offset() {
    with_circuit_stack(|| {
        let measure = |cc: u8| {
            let mut piano = OpenWurliUi::default();
            let params = Arc::get_mut(&mut piano.params).unwrap();
            params.select_model(Instrument::Electra);
            params.volume.smoothed.reset(OUTPUT_VOLUME_REFERENCE);
            assert_eq!(piano.params.key_range(), 29..=100);
            let mut event = Some(note_on(0, 64));
            let (mut l, mut r) = ([0.; 4096], [0.; 4096]);
            piano.render_stereo(&mut l, &mut r, || event.take());
            let mut events = [
                NoteEvent::MidiCC {
                    timing: 512,
                    channel: 0,
                    cc,
                    value: 1.,
                },
                note_off(2048, 64),
            ]
            .into_iter();
            piano.render_stereo(&mut l, &mut r, || events.next());
            let before = l[..512].to_vec();
            let level = l[1024..1792].iter().map(|x| x * x).sum::<f32>();
            piano.render_stereo(&mut l, &mut r, || None);
            let tail = l[2048..].iter().map(|x| x * x).sum::<f32>();
            assert_eq!(piano.electra.held_notes(), 0);
            (before, level, tail)
        };
        let normal = measure(1);
        let soft = measure(67);
        let sustained = measure(64);
        assert_eq!(normal.0, soft.0);
        assert_eq!(normal.0, sustained.0);
        assert!(soft.1 < 0.99 * normal.1 && soft.1 > 0.);
        assert!(sustained.2 > 2. * normal.2);
    });
}

#[test]
fn e7_can_power_up_during_an_already_held_note() {
    with_circuit_stack(|| {
        let mut p = OpenWurliUi::default();
        let params = Arc::get_mut(&mut p.params).unwrap();
        params.select_model(Instrument::Clavinet(crate::models::ClavinetModel::E7));
        params.clavinet_e7_power = BoolParam::new("E7 Power", false);
        params.volume.smoothed.reset(OUTPUT_VOLUME_REFERENCE);
        let mut key = Some(note_on(0, 64));
        let (mut l, mut r) = ([0.; 4096], [0.; 4096]);
        p.render_stereo(&mut l, &mut r, || key.take());
        assert!(l.iter().all(|v| *v == 0.));
        assert_eq!(p.d6.held_notes(), 1 << 64);
        Arc::get_mut(&mut p.params).unwrap().clavinet_e7_power = BoolParam::new("E7 Power", true);
        p.render_stereo(&mut l, &mut r, || None);
        assert!(l.iter().any(|v| v.abs() > 0.001));
    });
}

#[test]
fn every_model_renders_extended_keys_and_releases_after_range_changes() {
    with_circuit_stack(|| {
        let mut plugin = OpenWurliUi::default();
        for model in PianoType::ALL
            .into_iter()
            .map(Instrument::Piano)
            .chain(Instrument::extra_models())
        {
            let p = Arc::get_mut(&mut plugin.params).unwrap();
            p.select_model(model);
            if let Instrument::Piano(piano) = model {
                p.piano = EnumParam::new("Piano", piano);
            }
            p.volume.smoothed.reset(OUTPUT_VOLUME_REFERENCE);
            let (mut l, mut r) = ([0.; 8192], [0.; 8192]);
            for n in [0, 1, 126, 127] {
                plugin.reset();
                Arc::get_mut(&mut plugin.params).unwrap().extended_notes =
                    BoolParam::new("Extended Notes", true);
                let mut down = Some(note_on(32, n));
                plugin.render_stereo(&mut l[..256], &mut r[..256], || down.take());
                assert_eq!(plugin.held_notes(), 1 << n, "{model:?} MIDI {n} ownership");
                plugin.render_stereo(&mut l[256..], &mut r[256..], || None);
                assert!(
                    l.iter().all(|x| x.is_finite()) && l.iter().any(|x| x.abs() > 1e-10),
                    "{model:?} MIDI {n} output"
                );
                Arc::get_mut(&mut plugin.params).unwrap().extended_notes =
                    BoolParam::new("Extended Notes", false);
                let mut off = Some(NoteEvent::NoteOn {
                    timing: 0,
                    note: n,
                    velocity: 0.,
                    channel: 0,
                    voice_id: None,
                });
                plugin.render_stereo(&mut l, &mut r, || off.take());
                assert_eq!(plugin.held_notes(), 0, "{model:?} MIDI {n} release");
                let mut blocked = Some(note_on(0, n));
                plugin.render_stereo(&mut l, &mut r, || blocked.take());
                assert_eq!(plugin.held_notes(), 0, "{model:?} MIDI {n} native limit");
            }
        }
    });
}

#[test]
fn wurli_distance_opens_bass_and_treble_comb_faces() {
    with_circuit_stack(|| {
        for note in [40, 81] {
            let strike = |gap| {
                let mut p = OpenWurliUi::default();
                Arc::get_mut(&mut p.params).unwrap().regulation.wurli_gap = FloatParam::new(
                    "OpenWurli Comb Distance",
                    gap,
                    FloatRange::Linear {
                        min: -0.08,
                        max: 0.30,
                    },
                );
                let mut events = Some(note_on(0, note));
                let mut l = [0.; 8192];
                p.render_block(&mut l, || events.take());
                l.iter().map(|v| v * v).sum::<f32>()
            };
            assert!(strike(-0.08) > strike(0.30), "OpenWurli MIDI {note}");
        }
    });
}

#[test]
fn hohner_models_route_key_edges_and_ignore_sustain() {
    with_circuit_stack(|| {
        let mut p = OpenWurliUi::default();
        p.params.volume.smoothed.reset(OUTPUT_VOLUME_REFERENCE);
        let mut signatures = Vec::new();
        for model in Instrument::HOHNER
            .into_iter()
            .filter(|m| !matches!(m, Instrument::Electra | Instrument::Duo))
        {
            let params = Arc::get_mut(&mut p.params).unwrap();
            params.select_model(model);
            params.extended_notes = BoolParam::new("Extended", false);
            let (low, high) = p.params.key_range().into_inner();
            let (mut l, mut r) = ([0.; 4096], [0.; 4096]);
            let mut keys = [
                note_on(0, 28),
                note_on(0, high + 1),
                pedal(1.),
                note_on(384, 64),
            ]
            .into_iter();
            p.render_stereo(&mut l, &mut r, || keys.next());
            assert_eq!(l, r);
            assert!(l[128..384].iter().all(|v| v.abs() < 1e-7));
            assert!(
                l[384..].iter().all(|v| v.is_finite()) && l[384..].iter().any(|v| v.abs() > 1e-4),
                "{model:?}"
            );
            signatures.push(l);
            let pressed = |p: &OpenWurliUi| {
                p.reeds
                    .iter()
                    .fold(p.d6.held_notes(), |notes, engine| notes | engine.pressed())
            };
            assert_eq!(pressed(&p), 1 << 64);
            let mut lift = Some(note_off(0, 64));
            p.render_stereo(&mut l, &mut r, || lift.take());
            assert_eq!(pressed(&p), 0);
            for n in [low, high] {
                p.params.keyboard.audition(Some(n));
                p.render_stereo(&mut l, &mut r, || None);
                assert_eq!(pressed(&p), 1 << n);
            }
            p.params.keyboard.audition(None);
            p.render_stereo(&mut l, &mut r, || None);
            assert_eq!(pressed(&p), 0);
        }
        assert!(signatures.windows(2).all(|pair| pair[0] != pair[1]));
    });
}

#[test]
fn sustain_mod_keeps_duo_registration_and_saved_models_independent() {
    with_circuit_stack(|| {
        use pianet_n::Model;
        let mut p = OpenWurliUi::default();
        let select = |p: &mut OpenWurliUi, m| Arc::get_mut(&mut p.params).unwrap().select_model(m);
        let tick = |p: &mut OpenWurliUi, event: Option<NoteEvent<()>>| {
            let mut event = event;
            let (mut l, mut r) = ([0.; 128], [0.; 128]);
            p.render_stereo(&mut l, &mut r, || event.take());
        };
        select(&mut p, Instrument::Duo);
        tick(&mut p, Some(pedal(1.)));
        assert!(p.params.duo.foot_is_b());
        Arc::get_mut(&mut p.params).unwrap().sustain_mod.duo.enabled =
            BoolParam::new("Duo Pianet Sustain Mod", true);
        tick(&mut p, None);
        assert!(!p.params.duo.foot_is_b());
        assert_eq!(
            p.params.sustain_mod.display(Instrument::Duo),
            Some((true, true))
        );
        select(&mut p, Instrument::Reed(Model::N));
        tick(&mut p, None);
        assert_eq!(
            p.params.sustain_mod.display(Instrument::Reed(Model::N)),
            Some((false, false))
        );
        select(&mut p, Instrument::Duo);
        tick(&mut p, None);
        assert!(!p.params.duo.foot_is_b());
        Arc::get_mut(&mut p.params).unwrap().sustain_mod.duo.enabled =
            BoolParam::new("Duo Pianet Sustain Mod", false);
        tick(&mut p, None);
        assert!(p.params.duo.foot_is_b());
        tick(&mut p, Some(pedal(0.)));
        assert!(!p.params.duo.foot_is_b());
        for model in [Model::Cembalet, Model::CembaletN] {
            assert!(p
                .params
                .sustain_mod
                .for_model(Instrument::Reed(model))
                .is_none());
        }
    });
}
