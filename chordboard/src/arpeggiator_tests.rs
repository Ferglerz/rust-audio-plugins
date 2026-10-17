use super::*;

#[test]
fn arp_state_round_trip_preserves_eight_pages_and_every_tap_field() {
    let mut state = State::default();
    for (page_index, page) in state.pages.iter_mut().enumerate() {
        for (index, step) in page.steps.iter_mut().enumerate() {
            *step = Step {
                state: [StepState::Play, StepState::Mute, StepState::Skip][index % 3],
                velocity: (index * 2) as u8,
                gate: (index + 1) as u8,
                probability: (100 - index) as u8,
                octave: (index % 9) as i8 - 4,
                micro: index as i16 - 48,
                mods: [index as i16, -(index as i16)],
            };
        }
        page.delay = DelaySettings {
            enabled: page_index % 2 == 0,
            sync: page_index % 2 == 1,
            ping_pong: page_index % 3 == 0,
            sources: page_index as u8 + 1,
            count: page_index as u8 + 1,
            taps: std::array::from_fn(|tap| DelayTap {
                enabled: tap % 2 == 0,
                time: (tap + 1) as f32 * 0.25,
                velocity: 0.25 + tap as f32 * 0.1,
                feedback: tap as f32 * 0.1,
                pitch: tap as i8 * 3 - 12,
                fine: tap as f32 * 7.5 - 25.0,
                pan: tap as f32 * 0.25 - 1.0,
                channel: tap as u8 + 1,
            }),
        };
    }
    let json = serde_json::to_string(&state).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["pages"].as_array().unwrap().len(), PAGES);
    for page in value["pages"].as_array().unwrap() {
        assert_eq!(page["steps"].as_array().unwrap().len(), STEPS);
        assert_eq!(page["delay"]["taps"].as_array().unwrap().len(), TAPS);
    }
    assert_eq!(serde_json::from_str::<State>(&json).unwrap(), state);
}

#[test]
fn arp_state_uses_fresh_defaults_and_rejects_incomplete_occurrence_arrays() {
    assert_eq!(
        serde_json::from_str::<State>("{}").unwrap(),
        State::default()
    );
    assert_eq!(serde_json::from_str::<Page>("{}").unwrap(), Page::default());
    assert_eq!(serde_json::from_str::<Step>("{}").unwrap(), Step::default());
    let mut json = serde_json::to_value(State::default()).unwrap();
    json["pages"][0]["steps"].as_array_mut().unwrap().pop();
    let error = serde_json::from_value::<State>(json).unwrap_err();
    assert!(error.to_string().contains("96 arp steps"));
}

#[test]
fn arp_store_sanitizes_control_state_before_publishing() {
    let store = Store::new();
    let mut current = store.initial_prepared();
    store.edit(|state| {
        state.pages[7].steps[95] = Step {
            state: StepState::Skip,
            velocity: 255,
            gate: 0,
            probability: 255,
            octave: -120,
            micro: 999,
            mods: [-999, 999],
        };
        let delay = &mut state.pages[7].delay;
        delay.sources = 255;
        delay.count = 255;
        delay.taps[7] = DelayTap {
            enabled: true,
            time: f32::NAN,
            velocity: f32::INFINITY,
            feedback: 1.0,
            pitch: 127,
            fine: -1000.0,
            pan: 100.0,
            channel: 255,
        };
    });
    let snapshot = store.snapshot();
    assert_eq!(
        snapshot.pages[7].steps[95],
        Step {
            state: StepState::Skip,
            velocity: 200,
            gate: 1,
            probability: 100,
            octave: -4,
            micro: 49,
            mods: [-100, 100]
        }
    );
    let delay = snapshot.pages[7].delay;
    assert_eq!((delay.sources, delay.count), (31, 8));
    assert_eq!(
        delay.taps[7],
        DelayTap {
            enabled: true,
            time: 0.5,
            velocity: 0.75,
            feedback: 0.95,
            pitch: 24,
            fine: -100.0,
            pan: 1.0,
            channel: 16
        }
    );
    assert!(store.apply_pending(&mut current));
    assert_eq!(current.pages[7].steps, snapshot.pages[7].steps);
    assert_eq!(current.pages[7].delay, snapshot.pages[7].delay);
    assert_eq!((current.generation, current.restore_epoch), (1, 0));
}

#[test]
fn arp_delay_free_and_synced_domains_are_sanitized_without_nonfinite_values() {
    for sync in [false, true] {
        let mut settings = DelaySettings {
            sync,
            count: 0,
            ..DelaySettings::default()
        };
        settings.taps[0].time = -100.0;
        settings.taps[0].velocity = -100.0;
        settings.taps[0].feedback = -100.0;
        settings.taps[0].pitch = -100;
        settings.taps[0].fine = f32::NAN;
        settings.taps[0].pan = f32::NEG_INFINITY;
        settings.taps[1].time = f32::INFINITY;
        settings.taps[2].time = f32::MAX;
        settings.sanitize();
        assert_eq!(settings.count, 1);
        assert_eq!(settings.taps[0].time, if sync { 1.0 / 64.0 } else { 1.0 });
        assert_eq!(settings.taps[1].time, if sync { 0.5 } else { 250.0 });
        assert_eq!(settings.taps[2].time, if sync { 16.0 } else { 8000.0 });
        assert_eq!(
            (
                settings.taps[0].velocity,
                settings.taps[0].feedback,
                settings.taps[0].pitch,
                settings.taps[0].fine,
                settings.taps[0].pan
            ),
            (0.0, 0.0, -24, 0.0, 0.0)
        );
    }
}

#[test]
fn arp_preset_restoration_advances_epoch_but_ordinary_edits_do_not() {
    let store = Store::new();
    let mut current = store.initial_prepared();
    store.edit(|state| state.pages[1].steps[3].velocity = 25);
    assert!(store.apply_pending(&mut current));
    assert_eq!((current.generation, current.restore_epoch), (1, 0));
    let mut restored = State::default();
    restored.pages[1].steps[3].velocity = 175;
    PersistentField::set(&store, restored.clone());
    assert!(store.apply_pending(&mut current));
    assert_eq!((current.generation, current.restore_epoch), (2, 1));
    assert_eq!(current.pages[1].steps[3].velocity, 175);
    store.edit(|state| state.pages[2].delay.enabled = true);
    assert!(store.apply_pending(&mut current));
    assert_eq!((current.generation, current.restore_epoch), (3, 1));
    PersistentField::set(&store, restored);
    assert!(store.apply_pending(&mut current));
    assert_eq!((current.generation, current.restore_epoch), (4, 2));
}

#[test]
fn arp_publication_coalesces_control_updates_and_retires_previous_boxes() {
    let store = Store::new();
    let mut current = store.initial_prepared();
    let initial_pointer: *const Prepared = &*current;
    for velocity in 1..=10 {
        store.edit(|state| state.pages[0].steps[0].velocity = velocity);
    }
    assert_eq!(store.updates.len(), 2);
    assert!(store.apply_pending(&mut current));
    assert_eq!(current.generation, 9);
    assert_eq!(current.pages[0].steps[0].velocity, 9);
    assert_eq!(store.retired.len(), 1);
    let retired = store.retired.pop().unwrap();
    assert_eq!(&*retired as *const Prepared, initial_pointer);
    store.retired.push(retired).unwrap();
    assert!(store.apply_pending(&mut current));
    assert_eq!(current.generation, 10);
    assert_eq!(store.retired.len(), 2);
    assert!(!store.apply_pending(&mut current));
    // Control-side editing, rather than callback consumption, reclaims both boxes.
    store.edit(|_| {});
    assert_eq!(store.retired.len(), 0);
    assert_eq!(current.generation, 10);
}

#[test]
fn arp_full_retirement_queue_preserves_current_and_pending_generations() {
    let store = Store::new();
    let mut current = store.initial_prepared();
    store.edit(|state| state.pages[0].steps[0].velocity = 42);
    for _ in 0..4 {
        store.retired.push(store.initial_prepared()).unwrap();
    }
    let pointer: *const Prepared = &*current;
    assert!(!store.apply_pending(&mut current));
    assert_eq!(&*current as *const Prepared, pointer);
    assert_eq!(current.generation, 0);
    assert_eq!((store.retired.len(), store.updates.len()), (4, 1));
    // Reclamation on the control/test thread makes room for the exact same update.
    drop(store.retired.pop());
    assert!(store.apply_pending(&mut current));
    assert_eq!(
        (current.generation, current.pages[0].steps[0].velocity),
        (1, 42)
    );
    assert_eq!(store.retired.len(), 4);
}

#[test]
fn arp_concurrent_consumer_guard_never_consumes_or_waits() {
    let store = Store::new();
    let mut current = store.initial_prepared();
    store.edit(|state| state.pages[0].steps[0].velocity = 42);
    store.consuming.store(true, Ordering::Release);
    assert!(!store.apply_pending(&mut current));
    assert_eq!(current.generation, 0);
    assert_eq!((store.updates.len(), store.retired.len()), (1, 0));
    store.consuming.store(false, Ordering::Release);
    assert!(store.apply_pending(&mut current));
    assert_eq!(current.generation, 1);
    assert!(!store.consuming.load(Ordering::Acquire));
}

#[test]
fn arp_step_states_and_playback_modes_have_explicit_stable_cycles() {
    assert_eq!(StepState::Play.next(), StepState::Mute);
    assert_eq!(StepState::Mute.next(), StepState::Skip);
    assert_eq!(StepState::Skip.next(), StepState::Play);
    assert_eq!(Settings::default().mode, ArpMode::SyncLoop);
    for (index, mode) in [
        ArpMode::Once,
        ArpMode::Loop,
        ArpMode::SyncLoop,
        ArpMode::AlignedLoop,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(ArpMode::from_index(index as u8), mode);
    }
    assert_eq!(ArpMode::from_index(255), ArpMode::AlignedLoop);
}
