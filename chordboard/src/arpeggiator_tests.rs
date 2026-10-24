use super::*;

#[test]
fn tracker_row_actions_preserve_selection_payload_and_minimum_row() {
    let mut state = State::default();
    state.rows[0].chord = Some(crate::harmony::parse_chord_symbol("Am7").unwrap());
    state.rows[0].duration = 4;
    state.rows[0].page.loops[0].steps[2].velocity = 73;
    let second = state.add_row_after(0);
    assert_eq!(second, 1);
    assert_eq!(state.rows[1].duration, 1);
    assert_eq!(state.rows[1].chord, state.rows[0].chord);
    assert_eq!(state.rows[1].page.loops[0].steps[2].velocity, 73);
    let copied = state.rows[0].clone();
    state.rows[1].chord = None;
    state.paste_row(1, &copied);
    assert_eq!(state.rows[1].chord, copied.chord);
    let logical_id = state.rows[1].id;
    assert_eq!(state.move_row(1, -1), 0);
    assert_eq!(state.rows[0].id, logical_id);
    assert_eq!(state.delete_row(0), 0);
    assert_eq!(state.rows.len(), 1);
    state.delete_row(0);
    assert_eq!(state.rows.len(), 1);
    assert!(state.rows[0].chord.is_none());
}

#[test]
fn delay_harmony_options_default_off_for_existing_presets() {
    let old = r#"{"enabled":true,"sync":false,"ping_pong":true,"sources":31}"#;
    let delay: DelaySettings = serde_json::from_str(old).unwrap();
    assert!(delay.enabled && delay.ping_pong);
    assert!(!delay.voice_leading && !delay.follow_chords);
}

#[test]
fn tracker_state_round_trip_preserves_dynamic_rows_and_every_tap_field() {
    let mut state = State::default();
    for page_index in 1..PAGES {
        state.rows.push(TrackerRow {
            id: page_index as u64 + 1,
            ..TrackerRow::default()
        });
    }
    for (page_index, row) in state.rows.iter_mut().enumerate() {
        row.duration = page_index as u32 + 1;
        row.settings.pattern = (page_index % 7) as u8;
        let page = &mut row.page;
        for (index, step) in page.loops.iter_mut().flat_map(|l| &mut l.steps).enumerate() {
            *step = Step {
                state: [StepState::Play, StepState::Mute, StepState::Skip][index % 3],
                velocity: (index * 2) as u8,
                gate: (index + 1) as u8,
                probability: (100 - index % 96) as u8,
                octave: (index % 9) as i8 - 4,
                micro: index as i16 - 48,
                mods: [index as i16, -(index as i16)],
            };
        }
        page.delay = DelaySettings {
            enabled: page_index % 2 == 0,
            sync: page_index % 2 == 1,
            ping_pong: page_index % 3 == 0,
            voice_leading: page_index % 2 == 0,
            follow_chords: page_index % 3 == 1,
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
    assert!(value.get("pages").is_none());
    assert_eq!(value["rows"].as_array().unwrap().len(), PAGES);
    for row in value["rows"].as_array().unwrap() {
        let page = &row["page"];
        assert_eq!(page["loops"].as_array().unwrap().len(), LOOPS);
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
    json["rows"][0]["page"]["loops"][0]["steps"]
        .as_array_mut()
        .unwrap()
        .pop();
    let error = serde_json::from_value::<State>(json).unwrap_err();
    assert!(error.to_string().contains("24"));
}

#[test]
fn arp_store_sanitizes_control_state_before_publishing() {
    let store = Store::new();
    let mut current = store.initial_prepared();
    store.edit(|state| {
        state.pages[7].loops[3].steps[23] = Step {
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
        snapshot.pages[7].loops[3].steps[23],
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
    assert_eq!(current.pages[7].loops, snapshot.pages[7].loops);
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
    store.edit(|state| state.pages[1].loops[0].steps[3].velocity = 25);
    assert!(store.apply_pending(&mut current));
    assert_eq!((current.generation, current.restore_epoch), (1, 0));
    let mut restored = State::default();
    restored.pages[1].loops[0].steps[3].velocity = 175;
    PersistentField::set(&store, restored.clone());
    assert!(store.apply_pending(&mut current));
    assert_eq!((current.generation, current.restore_epoch), (2, 1));
    assert_eq!(current.pages[1].loops[0].steps[3].velocity, 175);
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
        store.edit(|state| state.pages[0].loops[0].steps[0].velocity = velocity);
    }
    assert_eq!(store.updates.len(), 2);
    assert!(store.apply_pending(&mut current));
    assert_eq!(current.generation, 9);
    assert_eq!(current.pages[0].loops[0].steps[0].velocity, 9);
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
    store.edit(|state| state.pages[0].loops[0].steps[0].velocity = 42);
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
        (
            current.generation,
            current.pages[0].loops[0].steps[0].velocity
        ),
        (1, 42)
    );
    assert_eq!(store.retired.len(), 4);
}

#[test]
fn arp_concurrent_consumer_guard_never_consumes_or_waits() {
    let store = Store::new();
    let mut current = store.initial_prepared();
    store.edit(|state| state.pages[0].loops[0].steps[0].velocity = 42);
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

#[test]
fn arp_loop_octaves_round_trip_and_publish_sanitized_values() {
    let store = Store::new();
    let mut prepared = store.initial_prepared();
    store.edit(|state| {
        for (l, octave) in state.pages[2].loops.iter_mut().zip([-100, -3, 2, 100]) {
            l.octave = octave;
        }
    });
    let state = store.snapshot();
    assert_eq!(state.pages[2].octaves(), [-4, -3, 2, 4]);
    assert_eq!(
        serde_json::from_str::<State>(&serde_json::to_string(&state).unwrap()).unwrap(),
        state
    );
    assert!(store.apply_pending(&mut prepared));
    assert_eq!(prepared.pages[2].loops, state.pages[2].loops);
}

#[test]
fn arp_new_loops_inherit_all_fields_once_and_preserve_hidden_variations() {
    let mut page = Page::default();
    page.loops[0].octave = -2;
    page.loops[0].steps[23] = Step {
        state: StepState::Skip,
        velocity: 140,
        gate: 20,
        probability: 71,
        octave: 2,
        micro: -19,
        mods: [37, -42],
    };
    assert!(page.ensure_loops(4));
    assert!(page.loops.iter().all(|l| *l == page.loops[0]));
    page.loops[1].steps[23].velocity = 22;
    page.loops[0].octave = 3;
    assert!(!page.ensure_loops(1));
    assert!(!page.ensure_loops(4));
    assert_eq!(page.loops[1].steps[23].velocity, 22);
    assert_eq!(page.loops[2].octave, -2);
    let generation = Store::new();
    let before = generation.initial_prepared().generation;
    generation.ensure_loops([1; PAGES]);
    assert_eq!(generation.initial_prepared().generation, before);
}

#[test]
fn arp_loop_occurrence_identity_survives_pass_length_changes() {
    let mut page = Page::default();
    page.ensure_loops(4);
    page.step_mut(3, 3).velocity = 21; // Loop 2, first occurrence.
    page.step_mut(5, 3).micro = -12; // Loop 2, third occurrence.
    assert_eq!(page.step(4, 4).velocity, 21);
    assert_eq!(page.step(6, 4).micro, -12);
    page.step_mut(7, 4).probability = 17;
    assert_eq!(page.step(3, 3).velocity, 21);
    assert_eq!(page.step(7, 4).probability, 17);
}
