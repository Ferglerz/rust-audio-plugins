use super::*;
use crate::{
    dsp::{Activity, Chorus, Engine, Settings},
    network::{Network, Observer},
    params::{AutomixerParams, SharedParams},
};
use std::collections::VecDeque;

fn settings() -> Settings {
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
            if engine.analyze((levels[i] as f64).powi(2)) {
                let registration = &registrations[i];
                engine.control(
                    &registration.network,
                    registration.slot().unwrap(),
                    registration.initial_mic,
                    0.0,
                    false,
                    settings,
                );
            }
            gains[i] = engine.gain(settings, false);
            assert!(gains[i].is_finite() && gains[i] <= 1.0);
            assert!(
                engine.reduction <= settings.cap + 1e-5,
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
        ..settings()
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
        cap: 30.0,
        ..settings()
    };
    run_engines(&mut engines, &registrations, &[1e-8, 0.1], settings, 48_000);
    settings.cap = 6.0;
    let gains = run_engines(&mut engines, &registrations, &[1e-8, 0.1], settings, 48_000);
    assert!((gains[0] - 10.0_f32.powf(-6.0 / 20.0)).abs() < 0.001);
    let mut ramp = dsp::Ramp::default();
    for i in 1..=1000 {
        assert!((ramp.tick(1.0, 1000.0) - i as f32 / 1000.0).abs() < 0.00002);
    }
    assert!(ramp.tick(0.0, 1000.0) > 0.998);
}

#[test]
fn inactive_dropped_stalled_and_duplicate_instances_do_not_steal_gain() {
    let network = Arc::new(Network::default());
    let a = network.register();
    let b = network.register();
    let mut observer = Observer::default();
    a.slot().unwrap().publish(0.2, true);
    b.slot().unwrap().publish(0.1, true);
    let mix = observer.read(&network, 240, 480);
    assert_eq!((mix.members, mix.active), (2, 2));
    b.slot().unwrap().withdraw();
    assert_eq!(observer.read(&network, 240, 480).active, 1);
    b.slot().unwrap().publish(0.1, true);
    observer.read(&network, 240, 480);
    for _ in 0..3 {
        a.slot().unwrap().publish(0.2, true);
        observer.read(&network, 240, 480);
    }
    a.slot().unwrap().publish(0.2, true);
    assert_eq!(
        observer.read(&network, 240, 480).members,
        1,
        "stalled peer must expire"
    );
    b.slot().unwrap().publish(0.1, true);
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
    a.slot().unwrap().publish(0.2, true);
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
        assert_eq!(samples, 0);
    }
    fn set_current_voice_capacity(&self, _: u32) {}
}

fn plugin(network: &Arc<Network>) -> Automixer {
    let registration = network.register();
    let params = Arc::new(AutomixerParams::new(
        network.params.clone(),
        registration.slot().cloned(),
        registration.token,
    ));
    Automixer {
        current_mic: params.mic_index(),
        params,
        registration,
        engine: Engine::default(),
        vad: pleasant_voice::SpeechVad::new(48_000.0),
        sample_rate: 48_000.0,
    }
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
    removed.slot().unwrap().publish(1.0, true);
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
    slot.publish(0.1, true);
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
