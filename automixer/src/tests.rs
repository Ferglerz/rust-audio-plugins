use super::*;
use crate::{
    dsp::{Activity, Chorus, Engine, Settings},
    network::{Network, Observer},
    params::{AutomixerParams, SharedParams},
};
use std::collections::VecDeque;

fn default_settings() -> Settings {
    Settings::from_params(&SharedParams::default())
}

#[test]
fn gain_sharing_matches_normalized_weighted_rms() {
    let cap = 60.0;
    let gain = |own, total| 10.0_f32.powf(-dsp::share_reduction(own, total, cap) / 20.0);
    assert!((gain(1.0, 2.0) - 0.5).abs() < 1e-6);
    assert!((gain(3.0, 4.0) - 0.75).abs() < 1e-6);
    assert_eq!(gain(1.0, 1.0), 1.0);
    assert_eq!(gain(0.0, 0.0), 1.0);
    assert!((gain(1e-12, 1.0) - 0.001).abs() < 1e-6);
}

#[test]
fn activity_has_onset_hysteresis_hold_and_optional_six_db_assist() {
    let mut detector = Activity::default();
    for _ in 0..3 {
        detector.update(-39.0, -40.0, 5.0, 100.0);
    }
    assert!(!detector.active, "short bursts must not count as voices");
    detector.update(-39.0, -40.0, 5.0, 100.0);
    assert!(detector.active);
    for _ in 0..100 {
        detector.update(-42.0, -40.0, 5.0, 100.0);
    }
    assert!(
        detector.active,
        "hysteresis retains a voice just below the open threshold"
    );
    for _ in 0..19 {
        detector.update(-44.0, -40.0, 5.0, 100.0);
    }
    assert!(detector.active);
    detector.update(-44.0, -40.0, 5.0, 100.0);
    assert!(!detector.active);
    assert_eq!(dsp::assisted_threshold(-40.0, 1.0, 1.0), -46.0);
    assert_eq!(dsp::assisted_threshold(-40.0, 0.5, 1.0), -43.0);
    assert_eq!(dsp::assisted_threshold(-40.0, 0.0, 1.0), -40.0);
    assert_eq!(dsp::assisted_threshold(-40.0, 1.0, 0.0), -40.0);
}

#[test]
fn chorus_is_strictly_more_than_n_and_holds_before_return() {
    let mut chorus = Chorus::default();
    chorus.update(3, 3, 5.0, 100.0);
    assert!(!chorus.active);
    chorus.update(4, 3, 5.0, 100.0);
    assert!(chorus.active);
    for _ in 0..19 {
        chorus.update(3, 3, 5.0, 100.0);
    }
    assert!(chorus.active);
    chorus.update(3, 3, 5.0, 100.0);
    assert!(!chorus.active);
    chorus.update(64, 0, 5.0, 100.0);
    assert!(!chorus.active, "zero disables chorus protection");
}

fn run_engines(
    engines: &mut [Engine],
    registrations: &[network::Registration],
    levels: &[f32],
    settings: Settings,
    samples: usize,
) -> Vec<f32> {
    let mut gains = vec![1.0; engines.len()];
    for _ in 0..samples {
        for (i, engine) in engines.iter_mut().enumerate() {
            let voice = true;
            if engine.analyze((levels[i] as f64).powi(2)) {
                let registration = &registrations[i];
                engine.control(
                    &registration.network,
                    registration.slot().unwrap(),
                    registration.initial_mic,
                    0.0,
                    false,
                    voice,
                    settings,
                );
            }
            gains[i] = engine.gain(settings, voice, false);
            assert!(gains[i].is_finite() && gains[i] <= 1.0);
            assert!(
                engine.reduction <= settings.voice_cap + 1e-5,
                "cap breached: {}",
                engine.reduction
            );
        }
    }
    gains
}

#[test]
fn chorus_opens_active_mics_but_keeps_quiet_mics_capped_then_restores_sharing() {
    let network = Arc::new(Network::default());
    let registrations: Vec<_> = (0..4).map(|_| network.register()).collect();
    let mut engines: Vec<_> = (0..4).map(|_| Engine::default()).collect();
    let settings = Settings {
        voices: 2,
        fade_ms: 100.0,
        hold_ms: 100.0,
        ..default_settings()
    };
    let gains = run_engines(
        &mut engines,
        &registrations,
        &[0.1, 0.1, 0.1, 0.0001],
        settings,
        48_000,
    );
    for &gain in &gains[..3] {
        assert!(
            (gain - 1.0).abs() < 1e-4,
            "active chorus mic must open: {gain}"
        );
    }
    assert!(
        (gains[3] - 10.0_f32.powf(-18.0 / 20.0)).abs() < 1e-5,
        "quiet chorus mic must remain capped: {}",
        gains[3]
    );
    let gains = run_engines(
        &mut engines,
        &registrations,
        &[0.1, 0.1, 0.0001, 0.0001],
        settings,
        48_000,
    );
    assert!(
        (gains[0] - 0.5).abs() < 0.01,
        "sharing must return after chorus ends: {}",
        gains[0]
    );
    assert!(!engines[0].chorus.active);
    let dry = Settings {
        amount: 0.0,
        ..settings
    };
    let gains = run_engines(
        &mut engines,
        &registrations,
        &[0.1, 0.1, 0.1, 0.0001],
        dry,
        9600,
    );
    assert!(
        gains.iter().all(|&g| (g - 1.0).abs() < 1e-6),
        "amount zero must bypass ALL attenuation, including quiet mics"
    );
}

#[test]
fn reduction_cap_holds_during_cap_changes_and_fades_do_not_jump() {
    let network = Arc::new(Network::default());
    let registrations: Vec<_> = (0..2).map(|_| network.register()).collect();
    let mut engines = vec![Engine::default(), Engine::default()];
    let mut settings = Settings {
        voices: 0,
        voice_cap: 30.0,
        ..default_settings()
    };
    run_engines(&mut engines, &registrations, &[1e-8, 0.1], settings, 48_000);
    settings.voice_cap = 6.0;
    let gains = run_engines(&mut engines, &registrations, &[1e-8, 0.1], settings, 48_000);
    assert!((gains[0] - 10.0_f32.powf(-6.0 / 20.0)).abs() < 0.001);
    let mut ramp = dsp::Ramp::default();
    for i in 1..=1000 {
        assert!((ramp.tick(1.0, 1000.0) - i as f32 / 1000.0).abs() < 0.00002);
    }
    assert!(ramp.tick(0.0, 1000.0) > 0.998);
}

fn run_grouped(
    engines: &mut [Engine],
    registrations: &[network::Registration],
    levels: &[f32],
    voice: &[bool],
    speech: &[f32],
    settings: Settings,
    samples: usize,
) -> Vec<f32> {
    let mut gains = vec![1.0; engines.len()];
    for _ in 0..samples {
        for (i, engine) in engines.iter_mut().enumerate() {
            if engine.analyze((levels[i] as f64).powi(2)) {
                let registration = &registrations[i];
                engine.control(
                    &registration.network,
                    registration.slot().unwrap(),
                    registration.initial_mic,
                    speech[i],
                    false,
                    voice[i],
                    settings,
                );
            }
            gains[i] = engine.gain(settings, voice[i], false);
        }
    }
    gains
}

#[test]
fn groups_share_separately_and_keep_their_own_caps() {
    let network = Arc::new(Network::default());
    let registrations: Vec<_> = (0..4).map(|_| network.register()).collect();
    let mut engines: Vec<_> = (0..4).map(|_| Engine::default()).collect();
    let settings = Settings {
        voices: 0,
        voice_cap: 12.0,
        other_cap: 6.0,
        ..default_settings()
    };
    let gains = run_grouped(
        &mut engines,
        &registrations,
        &[0.1, 1e-8, 0.1, 1e-8],
        &[true, true, false, false],
        &[0.0; 4],
        settings,
        48_000,
    );
    assert!(
        (gains[0] - 1.0).abs() < 0.02,
        "loud voice stays near unity: {}",
        gains[0]
    );
    assert!(
        (gains[1] - 10.0_f32.powf(-12.0 / 20.0)).abs() < 0.02,
        "quiet voice uses the voice cap: {}",
        gains[1]
    );
    assert!(
        (gains[2] - 1.0).abs() < 0.02,
        "loud other stays near unity: {}",
        gains[2]
    );
    assert!(
        (gains[3] - 10.0_f32.powf(-6.0 / 20.0)).abs() < 0.02,
        "quiet other uses the other cap: {}",
        gains[3]
    );
}

#[test]
fn chorus_and_voice_assist_apply_only_to_the_voice_group() {
    let network = Arc::new(Network::default());
    let registrations: Vec<_> = (0..4).map(|_| network.register()).collect();
    let mut engines: Vec<_> = (0..4).map(|_| Engine::default()).collect();
    let settings = Settings {
        voices: 1,
        fade_ms: 80.0,
        hold_ms: 80.0,
        voice_cap: 60.0,
        other_cap: 60.0,
        ..default_settings()
    };
    let gains = run_grouped(
        &mut engines,
        &registrations,
        &[0.1; 4],
        &[true, false, false, false],
        &[0.0; 4],
        settings,
        48_000,
    );
    assert!(
        !engines[0].chorus.active,
        "other mics must not be counted toward chorus"
    );
    assert!((gains[0] - 1.0).abs() < 0.02, "solo voice: {}", gains[0]);
    for gain in &gains[1..] {
        assert!(
            (gain - 1.0 / 3.0).abs() < 0.02,
            "other mics keep sharing: {gain}"
        );
    }

    let network = Arc::new(Network::default());
    let registrations: Vec<_> = (0..4).map(|_| network.register()).collect();
    let mut engines: Vec<_> = (0..4).map(|_| Engine::default()).collect();
    let settings = Settings {
        voices: 1,
        fade_ms: 80.0,
        hold_ms: 80.0,
        ..default_settings()
    };
    let gains = run_grouped(
        &mut engines,
        &registrations,
        &[0.1; 4],
        &[true, true, false, false],
        &[0.0; 4],
        settings,
        48_000,
    );
    assert!(engines[0].chorus.active);
    assert!(!engines[2].chorus.active);
    assert!(
        (gains[0] - 1.0).abs() < 1e-3 && (gains[1] - 1.0).abs() < 1e-3,
        "active voice mics open in chorus: {gains:?}"
    );
    assert!(
        (gains[2] - 0.5).abs() < 0.02 && (gains[3] - 0.5).abs() < 0.02,
        "other mics stay shared during chorus: {gains:?}"
    );

    let network = Arc::new(Network::default());
    let registrations: Vec<_> = (0..2).map(|_| network.register()).collect();
    let mut engines = vec![Engine::default(), Engine::default()];
    let settings = Settings {
        voices: 0,
        assist: 1.0,
        threshold: -40.0,
        ..default_settings()
    };
    let level = 10.0_f32.powf(-43.0 / 20.0);
    run_grouped(
        &mut engines,
        &registrations,
        &[level, level],
        &[true, false],
        &[1.0, 1.0],
        settings,
        48_000,
    );
    assert!(
        engines[0].activity.active,
        "voice assist must lower the voice threshold"
    );
    assert!(
        !engines[1].activity.active,
        "other mics must keep the plain threshold"
    );
}

#[test]
fn inactive_dropped_stalled_and_duplicate_instances_do_not_steal_gain() {
    let network = Arc::new(Network::default());
    let a = network.register();
    let b = network.register();
    let mut observer = Observer::default();
    a.slot().unwrap().publish(0.2, true, true);
    b.slot().unwrap().publish(0.1, true, true);
    let mix = observer.read(&network, 240, 480);
    assert_eq!((mix.members, mix.active), (2, 2));
    b.slot().unwrap().withdraw();
    assert_eq!(observer.read(&network, 240, 480).active, 1);
    b.slot().unwrap().publish(0.1, true, true);
    observer.read(&network, 240, 480);
    for _ in 0..3 {
        a.slot().unwrap().publish(0.2, true, true);
        observer.read(&network, 240, 480);
    }
    a.slot().unwrap().publish(0.2, true, true);
    assert_eq!(
        observer.read(&network, 240, 480).members,
        1,
        "stalled peer must expire"
    );
    b.slot().unwrap().publish(0.1, true, true);
    assert_eq!(
        observer.read(&network, 240, 480).members,
        2,
        "returning peer must rejoin"
    );
    b.slot()
        .unwrap()
        .mic
        .store(a.initial_mic, Ordering::Release);
    assert_eq!(
        observer.read(&network, 240, 480).members,
        0,
        "duplicate identities must be excluded"
    );
    drop(b);
    a.slot().unwrap().publish(0.2, true, true);
    assert_eq!(observer.read(&network, 240, 480).members, 1);
}

#[test]
fn midi_addresses_remote_weights_and_state_persists() {
    let network = Arc::new(Network::default());
    let a = network.register();
    let b = network.register();
    let pa = AutomixerParams::new(network.params.clone(), a.slot().cloned(), a.token);
    let pb = AutomixerParams::new(network.params.clone(), b.slot().cloned(), b.token);
    assert!(Arc::ptr_eq(&pa.shared, &pb.shared));
    assert_eq!(
        pa.shared.mics[1].weight.as_ptr(),
        pb.shared.mics[1].weight.as_ptr()
    );
    pa.shared.midi(0, 21, 1.0);
    assert_eq!(pb.shared.mics[1].effective_weight(), 12.0);
    assert_eq!(pa.shared.mics[0].effective_weight(), 0.0);
    pa.shared.midi(1, 21, 0.0); // Wrong channel.
    pa.shared.midi(0, 21, f32::NAN);
    assert_eq!(pb.shared.mics[1].effective_weight(), 12.0);
    pa.shared.midi(0, 21, 64.0 / 127.0);
    assert_eq!(pb.shared.mics[1].effective_weight(), 0.0);
    pa.shared.midi(0, 21, 0.0);
    assert_eq!(pb.shared.mics[1].effective_weight(), -12.0);
    let restored = SharedParams::default();
    restored.deserialize_fields(&pa.shared.serialize_fields());
    assert_eq!(restored.mics[1].effective_weight(), -12.0);
    restored.mics[1].manual_edit();
    assert_eq!(restored.mics[1].effective_weight(), 0.0);
    let ids: std::collections::HashSet<_> = pa.param_map().into_iter().map(|p| p.0).collect();
    assert_eq!(
        ids.len(),
        pa.param_map().len(),
        "all shared host parameters need unique stable IDs"
    );
}

struct Context {
    latency: std::cell::Cell<u32>,
    input: VecDeque<NoteEvent<()>>,
    output: Vec<NoteEvent<()>>,
}
impl ProcessContext<Automixer> for Context {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Clap
    }
    fn execute_background(&self, _: ()) {}
    fn execute_gui(&self, _: ()) {}
    fn transport(&self) -> &Transport {
        panic!("automixing must not depend on a running transport")
    }
    fn next_event(&mut self) -> Option<NoteEvent<()>> {
        self.input.pop_front()
    }
    fn send_event(&mut self, event: NoteEvent<()>) {
        self.output.push(event);
    }
    fn set_latency_samples(&self, samples: u32) {
        self.latency.set(samples);
    }
    fn set_current_voice_capacity(&self, _: u32) {}
}

fn plugin(network: &Arc<Network>) -> Automixer {
    Automixer::new(network)
}

#[test]
fn strip_bypass_follows_the_instance_and_rejects_a_stale_slot_owner() {
    let network = Arc::new(Network::default());
    let local = plugin(&network);
    let remote = plugin(&network);
    let slot = remote.registration.slot().unwrap().clone();
    let owner = remote.registration.token;

    slot.editor_params(owner).unwrap().toggle_strip_bypass();
    assert!(
        remote.params.bypassed(),
        "remote strip can be bypassed before audio"
    );
    assert!(
        !local.params.bypassed(),
        "the editor's own mic stays engaged"
    );
    assert!(
        !remote.params.bypass.value(),
        "the DAW parameter is independent"
    );

    slot.mic.store(5, Ordering::Release);
    assert_eq!(remote.params.mic_index(), 5);
    assert!(
        remote.params.bypassed(),
        "bypass follows the track when renumbered"
    );
    let saved = remote.params.serialize_fields();
    drop(remote);

    let restored = plugin(&network);
    assert!(Arc::ptr_eq(restored.registration.slot().unwrap(), &slot));
    assert!(!restored.params.bypassed(), "a new owner starts engaged");
    assert!(
        slot.editor_params(owner).is_none(),
        "a stale click cannot hit the new owner"
    );
    restored.params.deserialize_fields(&saved);
    assert!(
        restored.params.bypassed(),
        "project state restores strip bypass"
    );
}

#[test]
fn strip_bypass_ramps_and_withdraws_only_its_mic_while_meters_and_midi_continue() {
    const FRAMES: usize = 240;
    const LEVEL: f32 = 0.1;

    fn process(plugin: &mut Automixer, context: &mut Context) -> [f32; FRAMES] {
        let mut samples = [LEVEL; FRAMES];
        let mut buffer = Buffer::default();
        // SAFETY: the slice covers every frame and outlives the buffer.
        unsafe { buffer.set_slices(FRAMES, |slices| slices.push(&mut samples)) };
        plugin.process(
            &mut buffer,
            &mut AuxiliaryBuffers {
                inputs: &mut [],
                outputs: &mut [],
            },
            context,
        );
        samples
    }

    let network = Arc::new(Network::default());
    let mut a = plugin(&network);
    let mut b = plugin(&network);
    let b_slot = b.registration.slot().unwrap().clone();
    let mut context = Context {
        latency: std::cell::Cell::new(0),
        input: VecDeque::new(),
        output: Vec::new(),
    };
    for _ in 0..140 {
        process(&mut a, &mut context);
        process(&mut b, &mut context);
    }
    let steady = process(&mut b, &mut context)[FRAMES - 1];
    assert!(
        (steady - 0.05).abs() < 1e-4,
        "equal mics share gain before bypass"
    );

    b_slot
        .editor_params(b.registration.token)
        .unwrap()
        .toggle_strip_bypass();
    let midi = NoteEvent::MidiCC {
        timing: 21,
        channel: 0,
        cc: 119,
        value: 0.5,
    };
    context.input.push_back(midi);
    let transition = process(&mut b, &mut context);
    assert!(
        (transition[0] - steady).abs() < 1e-3,
        "first bypass sample is continuous"
    );
    assert!(transition[FRAMES - 1] > transition[0] && transition[FRAMES - 1] < LEVEL);
    assert_eq!(context.output, vec![midi]);
    process(&mut a, &mut context);
    let mix = network.monitor(network.now_ms());
    assert_eq!((mix.members, mix.active), (1, 1));
    assert_eq!((a.engine.members, a.engine.active_count), (1, 1));

    for _ in 0..3 {
        process(&mut b, &mut context);
    }
    assert!((process(&mut b, &mut context)[0] - LEVEL).abs() < 1e-6);
    assert!((b_slot.input_db.load(Ordering::Relaxed) + 20.0).abs() < 1e-3);
    assert!((b_slot.output_db.load(Ordering::Relaxed) + 20.0).abs() < 1e-3);
    assert!(b_slot.reduction_db.load(Ordering::Relaxed) < 1e-3);

    b.params.toggle_strip_bypass();
    let transition = process(&mut b, &mut context);
    assert!(
        (transition[0] - LEVEL).abs() < 1e-3,
        "re-entry starts at unity"
    );
    assert!(transition[FRAMES - 1] < transition[0]);
    let mix = network.monitor(network.now_ms());
    assert_eq!((mix.members, mix.active), (2, 2));

    let mut params =
        AutomixerParams::new(network.params.clone(), Some(b_slot), b.registration.token);
    params.bypass = BoolParam::new("Bypass this microphone", true).make_bypass();
    b.params = Arc::new(params);
    b.params.toggle_strip_bypass();
    b.params.toggle_strip_bypass();
    assert!(
        !b.params.strip_bypassed() && b.params.bypassed(),
        "the DAW still holds bypass"
    );
    process(&mut b, &mut context);
    assert_eq!(network.monitor(network.now_ms()).members, 1);
}

#[test]
fn process_keeps_stereo_linked_passes_midi_and_sanitizes_nonfinite_audio() {
    let network = Arc::new(Network::default());
    let mut plugin = plugin(&network);
    let event = NoteEvent::MidiCC {
        timing: 37,
        channel: 0,
        cc: 21,
        value: 1.0,
    };
    let mut context = Context {
        latency: std::cell::Cell::new(0),
        input: VecDeque::from([event]),
        output: Vec::new(),
    };
    let mut left = [0.1; 128];
    let mut right = [-0.05; 128];
    left[15] = f32::NAN;
    right[17] = f32::INFINITY;
    {
        let mut buffer = Buffer::default();
        // SAFETY: both slices remain alive and have the declared equal length.
        unsafe {
            buffer.set_slices(128, |slices| {
                slices.push(&mut left);
                slices.push(&mut right);
            });
        }
        plugin.process(
            &mut buffer,
            &mut AuxiliaryBuffers {
                inputs: &mut [],
                outputs: &mut [],
            },
            &mut context,
        );
    }
    assert_eq!(context.output.len(), 1);
    assert_eq!(context.output[0].timing(), 37);
    assert_eq!(network.params.mics[1].effective_weight(), 12.0);
    assert!(left.iter().chain(&right).all(|s| s.is_finite()));
    for i in 0..128 {
        if i != 15 && i != 17 {
            assert!((left[i] + right[i] * 2.0).abs() < 1e-7);
        }
    }
    plugin.deactivate();
    assert_eq!(
        plugin
            .registration
            .slot()
            .unwrap()
            .signal
            .load(Ordering::Acquire),
        0
    );
}

#[test]
fn identities_are_reused_without_retaining_a_stale_meter() {
    let network = Arc::new(Network::default());
    let mut registrations: Vec<_> = (0..network::CAPACITY).map(|_| network.register()).collect();
    assert!(network.register().index.is_none());
    let removed = registrations.remove(12);
    let mic = removed.initial_mic;
    removed.slot().unwrap().publish(1.0, true, true);
    drop(removed);
    let replacement = network.register();
    assert_eq!(replacement.initial_mic, mic);
    assert_eq!(
        replacement.slot().unwrap().signal.load(Ordering::Acquire),
        0
    );
}

#[test]
fn monitor_follows_remote_instances_even_when_local_audio_stops() {
    let network = Arc::new(Network::default());
    let local = network.register();
    let remote = network.register();
    let slot = remote.slot().unwrap();
    slot.publish(0.1, true, true);
    slot.active.store(true, Ordering::Relaxed);
    slot.chorus_mix.store(0.6, Ordering::Relaxed);
    slot.updated_ms.store(1000, Ordering::Relaxed);
    local.slot().unwrap().withdraw();
    let monitor = network.monitor(1001);
    assert_eq!((monitor.members, monitor.active), (1, 1));
    assert_eq!(monitor.chorus, 0.6);
    assert_eq!(
        network.monitor(1800).active,
        0,
        "stale audio must not remain displayed as active"
    );
}

#[test]
fn host_input_assigns_a_unique_mic_and_renames_keep_cue_identity() {
    use nih_plug::context::track::{HardwareInput, HostTrackUpdate};

    let network = Arc::new(Network::default());
    let first = plugin(&network);
    let second = plugin(&network);
    let hardware = |first_channel| {
        HostTrackUpdate::HardwareInput(Some(HardwareInput {
            first_channel,
            label: format!("Input {}", first_channel + 1),
        }))
    };
    let a = first.host_track_handler().unwrap();
    let b = second.host_track_handler().unwrap();

    // The second instance initially holds provisional Mic 2.
    a(hardware(1));
    a(HostTrackUpdate::Prepare);
    assert_eq!(first.params.mic_index(), 1);
    assert_eq!(second.params.mic_index(), 0);

    b(hardware(1));
    b(HostTrackUpdate::Prepare);
    assert_eq!(
        second.params.mic_index(),
        0,
        "a duplicate hardware input must fall back"
    );
    a(HostTrackUpdate::Name(Some(" Lead vocal ".into())));
    let slot = first.registration.slot().unwrap();
    assert_eq!(
        slot.labels.lock().unwrap().name.as_deref(),
        Some("Lead vocal")
    );
    assert_eq!(first.params.mic_index(), 1);
    network.params.midi(0, 21, 1.0);
    assert_eq!(
        network.params.mics[first.params.mic_index()].effective_weight(),
        12.0
    );

    a(hardware(7));
    assert_eq!(
        first.params.mic_index(),
        1,
        "metadata must not reassign a running mic"
    );
    a(HostTrackUpdate::Prepare);
    assert_eq!(first.params.mic_index(), 7);
    assert_eq!(network.conflict_mask(), 0);
}

#[test]
fn automatic_assignment_restores_a_saved_fallback_when_hardware_is_unchanged() {
    use nih_plug::context::track::{HardwareInput, HostTrackUpdate};

    let network = Arc::new(Network::default());
    let first = plugin(&network);
    let handler = first.host_track_handler().unwrap();
    // Saved Mic 10 was previously chosen for a shared hardware Input 3.
    first
        .params
        .auto_mic
        .store((3 << 7) | 10, Ordering::Relaxed);
    handler(HostTrackUpdate::HardwareInput(Some(HardwareInput {
        first_channel: 2,
        label: "Input 3".into(),
    })));
    handler(HostTrackUpdate::Prepare);
    assert_eq!(first.params.mic_index(), 9);
}

#[test]
fn spectral_plugin_routes_midi_reports_latency_and_resets_cleanly() {
    let mut raw_network = Network::default();
    Arc::get_mut(&mut raw_network.params).unwrap().mode =
        EnumParam::new("Mix mode", params::MixMode::SpectralSpace);
    let network = Arc::new(raw_network);
    let mut instance = plugin(&network);
    let mut context = Context {
        latency: std::cell::Cell::new(0),
        input: VecDeque::from([NoteEvent::MidiCC {
            timing: 0,
            channel: 0,
            cc: 20,
            value: 1.0,
        }]),
        output: Vec::new(),
    };
    let mut rendered = Vec::new();
    for block in 0..24 {
        let mut left = [0.0; 128];
        let mut right = [0.0; 128];
        if block == 0 {
            left[0] = 1.0;
            right[0] = -0.5;
        }
        let mut buffer = Buffer::default();
        // SAFETY: both slices outlive processing and have the declared length.
        unsafe {
            buffer.set_slices(128, |slices| {
                slices.push(&mut left);
                slices.push(&mut right);
            });
        }
        instance.process(
            &mut buffer,
            &mut AuxiliaryBuffers {
                inputs: &mut [],
                outputs: &mut [],
            },
            &mut context,
        );
        for (left, right) in left.into_iter().zip(right) {
            assert!((right + left * 0.5).abs() < 1e-6);
            rendered.push(left);
        }
    }
    assert_eq!(context.latency.get(), spectral::FFT_SIZE as u32);
    assert_eq!(context.output.len(), 1);
    assert_eq!(network.params.mics[0].effective_weight(), 12.0);
    assert!((rendered[spectral::FFT_SIZE] - 1.0).abs() < 1e-5);
    assert!(rendered
        .iter()
        .enumerate()
        .all(|(i, x)| i == spectral::FFT_SIZE || x.abs() < 1e-5));
    instance.reset();
    assert!(instance
        .registration
        .slot()
        .unwrap()
        .spectrum
        .read()
        .is_none());
    instance.deactivate();
    assert_eq!(
        instance
            .registration
            .slot()
            .unwrap()
            .signal
            .load(Ordering::Acquire),
        0
    );
}

#[test]
fn drums_influence_other_tracks_faster_without_entering_voice_pool() {
    fn render(drums: bool) -> (f32, f32) {
        let mut network = Network::default();
        let params = Arc::get_mut(&mut network.params).unwrap();
        params.mics[0].group = EnumParam::new(
            "Group",
            if drums {
                params::MicGroup::Drums
            } else {
                params::MicGroup::Other
            },
        );
        params.mics[1].group = EnumParam::new("Group", params::MicGroup::Other);
        let network = Arc::new(network);
        let source = network.register();
        let receiver = network.register();
        let sa = source.slot().unwrap();
        let sb = receiver.slot().unwrap();
        let mut a = Engine::default();
        let mut b = Engine::default();
        a.set_drums(drums);
        let config = default_settings();
        let mut attack = 0.0;
        let mut release = 0.0;
        for n in 0..40000 {
            let hit = (24000..24960).contains(&n);
            if a.analyze(if hit { 0.25 } else { 0.0 }) {
                a.control(&network, sa, 0, 0.0, false, false, config);
            }
            a.gain(config, false, false);
            if b.analyze(0.01) {
                b.control(&network, sb, 1, 0.0, false, false, config);
            }
            b.gain(config, false, false);
            if n == 24480 {
                attack = b.reduction;
            }
            if n == 27840 {
                release = b.reduction;
            }
        }
        let snapshot = Observer::default().read(&network, 240, 48000);
        assert_eq!(snapshot.voice_total, 0.0);
        assert_eq!(snapshot.voice_active, 0);
        assert!(!a.chorus.active && !b.chorus.active);
        (attack, release)
    }
    let normal = render(false);
    let drums = render(true);
    assert!(
        drums.0 > normal.0 + 3.0,
        "drum influence must engage faster: drums={drums:?}, other={normal:?}"
    );
    assert!(
        drums.1 < normal.1 * 0.5,
        "drum influence must recover faster: drums={drums:?}, other={normal:?}"
    );
}

#[test]
fn combined_processing_applies_both_stages_without_changing_spectral_detection() {
    fn render(gain_amount: f32) -> (f32, f32, [f32; spectral::BANDS]) {
        let mut raw = Network::default();
        let p = Arc::get_mut(&mut raw.params).unwrap();
        p.mode = EnumParam::new("Mix mode", params::MixMode::SpectralSpace);
        p.amount = params::float("Gain amount", gain_amount, 0.0, 100.0, " %");
        p.mics[0].spectral_weight = params::float("Spectral weight", 12.0, -12.0, 12.0, " dB");
        let network = Arc::new(raw);
        let mut a = plugin(&network);
        let mut b = plugin(&network);
        let mut context = Context {
            latency: std::cell::Cell::new(0),
            input: VecDeque::new(),
            output: Vec::new(),
        };
        let mut energy = [0.0_f64; 2];
        for block in 0..600 {
            for (track, plugin) in [&mut a, &mut b].into_iter().enumerate() {
                let mut audio: [f32; 128] = std::array::from_fn(|i| {
                    (std::f32::consts::TAU * 43.0 * (block * 128 + i) as f32
                        / spectral::FFT_SIZE as f32)
                        .sin()
                        * 0.1
                });
                let mut buffer = Buffer::default();
                // SAFETY: the complete mono slice outlives this processing call.
                unsafe {
                    buffer.set_slices(128, |slices| slices.push(&mut audio));
                }
                plugin.process(
                    &mut buffer,
                    &mut AuxiliaryBuffers {
                        inputs: &mut [],
                        outputs: &mut [],
                    },
                    &mut context,
                );
                if block >= 500 {
                    energy[track] += audio.iter().map(|x| (*x as f64).powi(2)).sum::<f64>();
                }
            }
        }
        assert_eq!(context.latency.get(), spectral::FFT_SIZE as u32);
        assert!(
            b.spectral.cut.iter().any(|cut| *cut > 1.0),
            "lower spectral weight must yield"
        );
        assert!(
            a.spectral.cut.iter().all(|cut| *cut < 0.001),
            "higher spectral weight must stay clear"
        );
        let slot = b.registration.slot().unwrap();
        let (_, power, _) = slot.spectrum.read().unwrap();
        let expected_out = 10.0 * (energy[1] / (100.0 * 128.0)).log10();
        assert!(
            (slot.output_db.load(Ordering::Relaxed) as f64 - expected_out).abs() < 0.2,
            "OUT must include both stages"
        );
        (energy[0] as f32, energy[1] as f32, power)
    }
    let spectral_only = render(0.0);
    let combined = render(100.0);
    assert!(
        (combined.0 / spectral_only.0 - 0.25).abs() < 0.02,
        "equal gain weights must share even with unequal spectral weights"
    );
    assert!(
        (combined.1 / spectral_only.1 - 0.25).abs() < 0.02,
        "gain sharing must compound with spectral ducking"
    );
    for (combined, original) in combined.2.into_iter().zip(spectral_only.2) {
        assert!(
            (combined - original).abs() < 1e-8,
            "sharing must not feed back into spectral analysis"
        );
    }
}

#[test]
fn per_track_spectral_controls_are_independent_of_gain_weight_and_midi() {
    let mut p = SharedParams::default();
    p.mics[0].eq_blend = params::float("EQ blend", -75.0, -100.0, 100.0, " %");
    p.mics[1].eq_blend = params::float("EQ blend", 50.0, -100.0, 100.0, " %");
    p.mics[0].spectral_weight = params::float("Spectral weight", 6.0, -12.0, 12.0, " dB");
    p.midi(0, 20, 0.0);
    assert_eq!(p.mics[0].effective_weight(), -12.0);
    assert_eq!(p.mics[0].spectral_weight.value(), 6.0);
    assert_eq!(p.mics[1].spectral_weight.value(), 0.0);
    assert_eq!(spectral::Settings::from_params(&p, 0).blend, -0.75);
    assert_eq!(spectral::Settings::from_params(&p, 1).blend, 0.5);
    let ids: std::collections::HashSet<_> = p.param_map().into_iter().map(|x| x.0).collect();
    assert!(ids.contains("sweight_1") && ids.contains("eqblend_64") && ids.contains("samount"));
}
