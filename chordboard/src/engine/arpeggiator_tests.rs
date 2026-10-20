use super::*;
use crate::arpeggiator::{source, ArpMode, DelaySettings, Settings, StepState, STEPS};

fn notes(values: &[u8]) -> Notes {
    let mut notes = Notes::default();
    for &value in values {
        notes.push(i16::from(value));
    }
    notes
}

fn settings(mode: ArpMode, pattern: u8) -> Settings {
    Settings {
        mode,
        pattern,
        sync: false,
        free_ms: 100.0,
        ..Settings::default()
    }
}

fn make_engine(mode: ArpMode, pattern: u8) -> Engine {
    let mut engine = Engine {
        sample_rate: 1000.0,
        ..Engine::default()
    };
    engine.configure(
        Config {
            mode: MANUAL,
            arp_enabled: true,
            arp_settings: [settings(mode, pattern); 8],
            velocity: 0.5,
            bass_enabled: false,
            ..Config::default()
        },
        &mut |_| {},
    );
    engine.notes = notes(&[60, 64, 67]);
    engine.full_notes = engine.notes;
    engine.root = Some(Slot {
        id: 2048,
        note: 60,
        channel: 16,
        held: true,
        velocity: 0.5,
    });
    engine.playing = true;
    engine
}

fn advance(engine: &mut Engine, samples: usize) -> Vec<(u64, Out)> {
    let start = engine.now;
    let mut result = Vec::new();
    engine.advance(samples, &mut |offset, event| {
        result.push((start + offset as u64, event))
    });
    result
}

fn attacks(events: &[(u64, Out)]) -> Vec<(u64, u8, u8, f32)> {
    events
        .iter()
        .filter_map(|&(at, event)| match event {
            Out::On(channel, note, velocity) => Some((at, channel, note, velocity)),
            _ => None,
        })
        .collect()
}

fn recall(engine: &mut Engine, root: u8, quality: u8) -> Vec<Out> {
    let chord = harmony::SavedChord {
        root,
        quality,
        second: None,
        additions: 0,
        inversion: 0,
        spread: 0,
        transpose: 0,
    };
    let mut events = Vec::new();
    engine.command(Command::Recall(chord.encode()), &mut |event| {
        events.push(event)
    });
    events
}

#[test]
fn arp_up_down_repeated_notes_have_independent_velocity_and_gate() {
    let mut engine = make_engine(ArpMode::Once, 2);
    engine.arp.data.pages[0].steps[1].velocity = 50;
    engine.arp.data.pages[0].steps[1].gate = 20;
    engine.arp.data.pages[0].steps[3].velocity = 150;
    engine.arp.data.pages[0].steps[3].gate = 80;
    let events = advance(&mut engine, 450);
    assert_eq!(
        attacks(&events),
        vec![
            (0, 0, 60, 0.5),
            (100, 0, 64, 0.25),
            (200, 0, 67, 0.5),
            (300, 0, 64, 0.75)
        ]
    );
    assert!(events.contains(&(120, Out::Off(0, 64, 0.0))));
    assert!(events.contains(&(380, Out::Off(0, 64, 0.0))));
    assert!(engine.arp.done);
}

#[test]
fn arp_orderings_build_the_expected_occurrence_indices() {
    let chord = notes(&[60, 64, 67]);
    let expected: [&[u8]; 6] = [
        &[60, 64, 67],
        &[67, 64, 60],
        &[60, 64, 67, 64],
        &[60, 67, 64, 60, 67, 64],
        &[67, 60, 64, 67, 60, 64],
        &[60, 64, 67],
    ];
    for (pattern, expected) in expected.into_iter().enumerate() {
        let (cycle, len) =
            arpeggiator::cycle_notes(chord, settings(ArpMode::Once, pattern as u8), 17);
        assert_eq!(&cycle[..len], expected, "ordering {pattern}");
    }
    let unsorted = notes(&[67, 60, 64]);
    let (cycle, len) = arpeggiator::cycle_notes(unsorted, settings(ArpMode::Once, 5), 17);
    assert_eq!(&cycle[..len], &[67, 60, 64]);
    let random = settings(ArpMode::Once, 6);
    let first = arpeggiator::cycle_notes(chord, random, 1234);
    assert_eq!(first, arpeggiator::cycle_notes(chord, random, 1234));
    assert_eq!(first.1, 3);
    assert!(first.0[..first.1]
        .iter()
        .all(|note| chord.as_slice().contains(note)));
}

#[test]
fn shared_ordering_preserves_cycles_and_rng_draws() {
    let random_states = [
        0x1ee76a63, 0x86c50576, 0xeb111e23, 0xd2765369,
        0x1b7493e4, 0xa802bb60, 0x12b65e37, 0xf76a836f,
        0x47e12d6c, 0x2bdf150a, 0xe6b80715, 0x2d005338,
    ];
    for total in 1usize..=12 {
        let chord = notes(&(60..60 + total as u8).collect::<Vec<_>>());
        for pattern in [0, 1, 2, 3, 4, 6] {
            let mut engine = Engine { notes: chord, ..Engine::default() };
            let initial_seed = engine.rng;
            let (cycle, len) = arpeggiator::cycle_notes(
                chord, settings(ArpMode::Once, pattern), initial_seed,
            );
            let expected_len = match pattern {
                2 => (total * 2).saturating_sub(2).max(1),
                3 | 4 => total * 2,
                _ => total,
            };
            assert_eq!(len, expected_len);
            for (step, &note) in cycle[..len].iter().enumerate() {
                let index = engine.arp_index(pattern, step, total);
                assert_eq!(note, chord.values[index], "pattern {pattern}, total {total}, step {step}");
                let expected_seed = if pattern == 6 { random_states[step] } else { initial_seed };
                assert_eq!(engine.rng, expected_seed, "pattern {pattern}, step {step}");
            }
            if total == 1 {
                assert!(cycle[..len].iter().all(|&note| note == 60));
            }
        }
    }
}

#[test]
fn played_order_preserves_partial_octaves_and_cycle_clipping() {
    let mut engine = make_engine(ArpMode::Once, 5);
    engine.second = Some(Slot { id: 1, note: 67, channel: 0, held: true, velocity: 0.5 });
    let legacy = (0..5).map(|step| {
        let index = engine.arp_index(5, step, 5);
        engine.notes.string(index).unwrap()
    }).collect::<Vec<_>>();
    assert_eq!(legacy, [60, 67, 64, 72, 76]);
    let mut config = settings(ArpMode::Once, 5);
    config.octaves = 2;
    let (cycle, len) = arpeggiator::cycle_notes(engine.arp_cycle_chord(), config, engine.rng);
    assert_eq!(&cycle[..len], [60, 67, 64, 72, 79, 76]);

    engine.notes = notes(&[116, 120, 124]);
    engine.second.as_mut().unwrap().note = 124;
    config.octaves = 4;
    let (cycle, len) = arpeggiator::cycle_notes(engine.arp_cycle_chord(), config, engine.rng);
    assert_eq!(&cycle[..len], [116, 124, 120]);
}

#[test]
fn fractional_bend_ranges_match_dry_and_delay_rpn() {
    let rpn = |channel, semitones, cents| {
        [(101, 0), (100, 0), (6, semitones), (38, cents), (101, 127), (100, 127)]
            .map(|(cc, value)| Out::Cc(channel, cc, value as f32 / 127.0))
    };
    for mpe in [false, true] {
        let mut engine = delay_engine(source::STRUM);
        let mut config = engine.host_config;
        config.mpe = mpe;
        config.members = 2;
        config.master_range = 2.5;
        config.bend_range = 12.75;
        let mut dry = Vec::new();
        engine.configure(config, &mut |event| dry.push(event));
        engine.allocate_owned_voice(60, 0.8, 100, false, 0, None, &mut |event| dry.push(event));
        if mpe {
            assert!(dry.windows(6).any(|events| events == rpn(0, 2, 50)));
            for channel in [1, 2] {
                assert!(dry.windows(6).any(|events| events == rpn(channel, 12, 75)));
            }
        }
        let delayed = advance(&mut engine, 11);
        let delayed_events = delayed.iter().map(|&(_, event)| event).collect::<Vec<_>>();
        let played = attacks(&delayed);
        assert_eq!(played.len(), 1);
        let channel = played[0].1;
        let expected = if mpe { rpn(channel, 12, 75) } else { rpn(channel, 2, 50) };
        assert!(delayed_events.windows(6).any(|events| events == expected));
    }
}

#[test]
fn arp_full_ninety_six_occurrence_cycle_is_displayed_and_played() {
    let mut engine = make_engine(ArpMode::Once, 3);
    engine.notes = notes(&(24..36).collect::<Vec<_>>());
    engine.full_notes = engine.notes;
    let mut config = engine.host_config;
    config.arp_settings[0].octaves = 4;
    engine.configure(config, &mut |_| {});
    let snapshot = engine.snapshot();
    assert_eq!(usize::from(snapshot.arp_len), STEPS);
    let played = attacks(&advance(&mut engine, 9601));
    assert_eq!(played.len(), STEPS);
    assert_eq!(
        played
            .iter()
            .map(|&(_, _, note, _)| note)
            .collect::<Vec<_>>(),
        snapshot.arp_notes.to_vec()
    );
    assert_eq!(played.last().map(|event| event.0), Some(9500));
}

#[test]
fn arp_once_restarts_on_memory_recall_and_loop_restarts_on_chord_change() {
    for mode in [ArpMode::Once, ArpMode::Loop] {
        let mut engine = make_engine(mode, 0);
        assert_eq!(attacks(&advance(&mut engine, 250)).len(), 3);
        recall(&mut engine, 65, 0);
        assert_eq!(attacks(&advance(&mut engine, 1)), vec![(250, 0, 65, 0.5)]);
        if mode == ArpMode::Once {
            assert_eq!(attacks(&advance(&mut engine, 1000)).len(), 2);
            assert!(engine.arp.done);
            recall(&mut engine, 67, 0);
            assert_eq!(attacks(&advance(&mut engine, 1)), vec![(1251, 0, 67, 0.5)]);
        }
    }
}

#[test]
fn arp_sync_loop_retains_clock_and_wraps_into_changed_chord_lengths() {
    let mut engine = make_engine(ArpMode::SyncLoop, 0);
    recall(&mut engine, 60, 0);
    advance(&mut engine, 250);
    recall(&mut engine, 65, 3); // Four-note Fmaj7; retained ordinal 3 is E.
    assert!(attacks(&advance(&mut engine, 50)).is_empty());
    assert_eq!(attacks(&advance(&mut engine, 1)), vec![(300, 0, 76, 0.5)]);
    assert_eq!(attacks(&advance(&mut engine, 149)), vec![(400, 0, 65, 0.5)]);
    recall(&mut engine, 65, 11); // Two-note power chord; ordinal 5 wraps to C.
    assert!(attacks(&advance(&mut engine, 50)).is_empty());
    assert_eq!(attacks(&advance(&mut engine, 1)), vec![(500, 0, 72, 0.5)]);
}

#[test]
fn arp_aligned_loop_uses_host_zero_and_falls_back_without_transport_data() {
    let mut engine = make_engine(ArpMode::AlignedLoop, 0);
    let mut config = engine.host_config;
    config.arp_settings[0].sync = true;
    config.arp_settings[0].rate = 0.25;
    engine.configure(config, &mut |_| {});
    engine.set_host_position(Some(0.3), true);
    assert!(engine.snapshot().arp_aligned);
    assert!(attacks(&advance(&mut engine, 100)).is_empty());
    assert_eq!(attacks(&advance(&mut engine, 1)), vec![(100, 0, 67, 0.5)]);

    let mut fallback = make_engine(ArpMode::AlignedLoop, 0);
    assert!(!fallback.snapshot().arp_aligned);
    assert_eq!(attacks(&advance(&mut fallback, 1)), vec![(0, 0, 60, 0.5)]);
    advance(&mut fallback, 249);
    recall(&mut fallback, 65, 0);
    assert_eq!(
        attacks(&advance(&mut fallback, 51)),
        vec![(300, 0, 65, 0.5)]
    );
}

#[test]
fn arp_aligned_loop_retains_swing_and_realigns_after_seek() {
    let mut engine = make_engine(ArpMode::AlignedLoop, 0);
    let mut config = engine.host_config;
    config.arp_settings[0].sync = true;
    config.arp_settings[0].rate = 0.25;
    config.arp_settings[0].swing = 0.2;
    engine.configure(config, &mut |_| {});
    engine.set_host_position(Some(0.3), true);
    assert_eq!(attacks(&advance(&mut engine, 1)), vec![(0, 0, 64, 0.5)]);
    assert_eq!(attacks(&advance(&mut engine, 100)), vec![(100, 0, 67, 0.5)]);
    engine.transport(true, 120.0, true, &mut |_| {});
    engine.set_host_position(Some(1.1), true);
    recall(&mut engine, 60, 0);
    // Swing puts phase 5 at beat 1.3, 100 samples after the seek.
    assert!(attacks(&advance(&mut engine, 100)).is_empty());
    assert_eq!(attacks(&advance(&mut engine, 1)), vec![(201, 0, 67, 0.5)]);
}

#[test]
fn arp_position_counts_muted_steps_and_excludes_skipped_occurrences() {
    let mut engine = make_engine(ArpMode::Once, 2);
    engine.arp.data.pages[0].steps[1].state = StepState::Mute;
    engine.arp.data.pages[0].steps[1].mods = [25, -50];
    engine.arp.data.pages[0].steps[2].state = StepState::Skip;
    advance(&mut engine, 1);
    assert_eq!(engine.sources[11], 0.0);
    assert!(attacks(&advance(&mut engine, 100)).is_empty());
    assert_eq!(&engine.sources[9..12], &[0.25, -0.5, 0.5]);
    assert_eq!(attacks(&advance(&mut engine, 100)), vec![(200, 0, 64, 0.5)]);
    assert_eq!(engine.sources[11], 1.0);
}

#[test]
fn arp_chance_failure_updates_bipolar_modulators() {
    let mut engine = make_engine(ArpMode::Once, 0);
    engine.arp.data.pages[0].steps[1].probability = 0;
    engine.arp.data.pages[0].steps[1].mods = [-100, 75];
    advance(&mut engine, 1);
    assert!(attacks(&advance(&mut engine, 100)).is_empty());
    assert_eq!(&engine.sources[9..12], &[-1.0, 0.75, 0.5]);
    assert_eq!(attacks(&advance(&mut engine, 100)), vec![(200, 0, 67, 0.5)]);
}

#[test]
fn arp_all_skipped_is_idle_and_single_step_and_disabled_position_are_zero() {
    let mut skipped = make_engine(ArpMode::SyncLoop, 0);
    for step in &mut skipped.arp.data.pages[0].steps {
        step.state = StepState::Skip;
    }
    assert_eq!(skipped.arp_deadline(), None);
    assert!(advance(&mut skipped, 100_000).is_empty());
    assert_eq!(skipped.now, 100_000);
    assert!(!skipped.arp.running);
    assert_eq!(&skipped.sources[9..12], &[0.0; 3]);
    let mut single = make_engine(ArpMode::SyncLoop, 0);
    single.notes = notes(&[60]);
    advance(&mut single, 201);
    assert_eq!(single.sources[11], 0.0);
    let mut config = single.host_config;
    config.arp_enabled = false;
    single.sources[11] = 1.0;
    single.configure(config, &mut |_| {});
    assert_eq!(single.sources[11], 0.0);
}

#[test]
fn arp_negative_microtiming_uses_lookahead_and_positive_timing_can_follow_boundary() {
    let mut engine = make_engine(ArpMode::Once, 0);
    engine.arp.data.pages[0].steps[0].micro = -49;
    engine.arp.data.pages[0].steps[1].micro = -49;
    engine.arp.data.pages[0].steps[2].micro = 49;
    let played = attacks(&advance(&mut engine, 350));
    assert_eq!(
        played,
        vec![(0, 0, 60, 0.5), (51, 0, 64, 0.5), (249, 0, 67, 0.5)]
    );
}

fn fractional_render(block_size: usize) -> Vec<(u64, Out)> {
    let mut engine = make_engine(ArpMode::SyncLoop, 2);
    engine.sample_rate = 1000.0;
    engine.tempo = 137.0;
    let mut config = engine.host_config;
    config.arp_settings[0].sync = true;
    config.arp_settings[0].rate = 0.125;
    config.arp_settings[0].swing = 0.21;
    config.arp_settings[0].humanize = 0.33;
    engine.configure(config, &mut |_| {});
    engine.arp.data.pages[0].steps[1].micro = -31;
    engine.arp.data.pages[0].steps[2].micro = 23;
    let mut result = Vec::new();
    while engine.now < 5000 {
        let count = block_size.min((5000 - engine.now) as usize);
        result.extend(advance(&mut engine, count));
    }
    result
}

#[test]
fn arp_fractional_swing_humanize_and_microtiming_are_block_size_independent() {
    let reference = fractional_render(1);
    assert!(attacks(&reference).len() > 80);
    for block in [7, 64, 257, 1024] {
        assert_eq!(fractional_render(block), reference, "block size {block}");
    }
}

#[test]
fn arp_page_requests_wait_for_boundary_and_keep_manual_octaves_independent() {
    let mut engine = make_engine(ArpMode::SyncLoop, 0);
    advance(&mut engine, 150);
    let mut config = engine.host_config;
    config.arp_page = 1;
    config.arp_settings[1].free_ms = 200.0;
    config.arp_settings[1].octaves = 4;
    engine.arp.data.pages[1].steps[2].octave = 1;
    engine.configure(config, &mut |_| {});
    assert_eq!(
        (engine.snapshot().arp_page, engine.snapshot().arp_pending),
        (0, 1)
    );
    assert_eq!(engine.config.octaves, 1);
    assert_eq!(engine.string_count(), 3);
    assert!(attacks(&advance(&mut engine, 50)).is_empty());
    assert_eq!(attacks(&advance(&mut engine, 1)), vec![(200, 0, 79, 0.5)]);
    assert_eq!(
        (engine.snapshot().arp_page, engine.snapshot().arp_pending),
        (1, 255)
    );
    assert_eq!(engine.config.arp_octaves, 4);
    assert_eq!(engine.config.octaves, 1);
    assert_eq!(engine.string_count(), 3);
    assert_eq!(
        attacks(&advance(&mut engine, 200))
            .first()
            .map(|event| event.0),
        Some(400)
    );
}

#[test]
fn arp_completed_once_starts_a_requested_page() {
    let mut engine = make_engine(ArpMode::Once, 0);
    advance(&mut engine, 350);
    assert!(engine.arp.done);
    let mut config = engine.host_config;
    config.arp_page = 1;
    engine.arp.data.pages[1].steps[0].octave = 1;
    engine.configure(config, &mut |_| {});
    assert_eq!(attacks(&advance(&mut engine, 1)), vec![(350, 0, 72, 0.5)]);
    assert_eq!(engine.arp.active, 1);
    assert!(!engine.arp.done);
}

#[test]
fn arp_random_snapshot_matches_every_sounding_occurrence_across_cycles() {
    let mut engine = make_engine(ArpMode::SyncLoop, 6);
    for _ in 0..1201 {
        let played = attacks(&advance(&mut engine, 1));
        if let Some(&(_, _, note, _)) = played.first() {
            let snapshot = engine.snapshot();
            assert_eq!(
                note,
                snapshot.arp_notes[snapshot.arp_step as usize],
                "step {} at sample {}",
                snapshot.arp_step,
                engine.now - 1
            );
        }
    }
}

fn delay_engine(selected: u8) -> Engine {
    let mut engine = make_engine(ArpMode::SyncLoop, 0);
    let mut config = engine.host_config;
    config.arp_enabled = false;
    engine.configure(config, &mut |_| {});
    let mut delay = DelaySettings {
        enabled: true,
        sync: false,
        sources: selected,
        ..DelaySettings::default()
    };
    delay.taps[0].time = 10.0;
    delay.taps[0].velocity = 0.5;
    delay.taps[0].feedback = 0.0;
    engine.arp.data.pages[0].delay = delay;
    engine
}

#[test]
fn delay_synced_repeats_advance_while_host_position_is_stopped() {
    for block in [1, 7, 64] {
        let mut e = delay_engine(source::STRUM);
        e.playing = false;
        e.arp.data.pages[0].delay.sync = true;
        e.arp.data.pages[0].delay.taps[0].time = 0.5;
        e.set_host_position(Some(8.0), true);
        e.strike(60, 0.8, 20, &mut |_| {});
        let mut events = Vec::new();
        while e.now < 600 {
            e.set_host_position(Some(8.0), true);
            let count = block.min((600 - e.now) as usize);
            events.extend(advance(&mut e, count));
        }
        let played = attacks(&events);
        assert_eq!(played.len(), 1, "block size {block}");
        assert_eq!((played[0].0, played[0].2), (250, 60));
        assert!(events.contains(&(270, Out::Off(played[0].1, 60, 0.0))));
    }
}

#[test]
fn delay_captures_strums_over_held_chords_with_the_strum_duration() {
    for selected in [source::STRUM, source::CHORDS] {
        let mut e = delay_engine(selected);
        let mut config = e.host_config;
        config.always_chord = true;
        config.key_split = false;
        e.configure(config, &mut |_| {});
        e.allocate_owned_voice(60, 0.8, 0, true, 0, None, &mut |_| {});
        advance(&mut e, 1);
        let mut events = Vec::new();
        e.strike(60, 0.6, 20, &mut |event| events.push(event));
        assert_eq!(events.iter().filter(|e| matches!(e, Out::On(_, 60, _))).count(), 1);
        let events = advance(&mut e, 40);
        let played = attacks(&events);
        assert_eq!(played.len(), 1);
        let echo = played[0];
        if selected == source::STRUM {
            assert_eq!((echo.0, echo.3), (11, 0.3));
            assert!(events.contains(&(31, Out::Off(echo.1, 60, 0.0))));
        } else {
            assert_eq!((echo.0, echo.3), (10, 0.4));
            assert!(!events.iter().any(|(_, event)| *event == Out::Off(echo.1, 60, 0.0)));
        }
        assert!(e.voices.iter().flatten().any(|v| v.layer && v.note == 60));
        e.panic(&mut |_| {});
        assert!(e.voices.iter().all(Option::is_none));
    }
}

#[test]
fn delay_integration_captures_only_selected_dry_owners() {
    for selected in [source::ARP, source::STRUM, source::CHORDS] {
        let mut engine = delay_engine(selected);
        for (note, owner, layer) in [
            (60, arpeggiator::ARP_OWNER, false),
            (64, 0, false),
            (67, 2, true),
        ] {
            engine.allocate_owned_voice(note, 0.8, 5, layer, owner, None, &mut |_| {});
        }
        let played = attacks(&advance(&mut engine, 11));
        let expected = match selected {
            source::ARP => 60,
            source::STRUM => 64,
            _ => 67,
        };
        assert_eq!(played.len(), 1);
        assert_eq!((played[0].0, played[0].2, played[0].3), (10, expected, 0.4));
        assert_ne!(played[0].1, 0);
    }
}

#[test]
fn delay_integration_held_melody_preserves_duration_and_isolates_pitch_pan() {
    let mut engine = delay_engine(source::MELODY);
    engine.channels[3] = Expression {
        pressure: 0.4,
        timbre: 0.8,
        bend: 0.7,
    };
    engine.input_bend_values[3] = 0.7;
    engine.input_bend[3] = engine.config.master_range;
    engine.arp.data.pages[0].delay.taps[0].pitch = 12;
    engine.arp.data.pages[0].delay.taps[0].fine = 25.0;
    engine.arp.data.pages[0].delay.taps[0].pan = -0.5;
    engine.melody_note(true, 3, 84, 0.8, &mut |_| {});
    let events = advance(&mut engine, 11);
    let played = attacks(&events);
    assert_eq!(played, vec![(10, 2, 96, 0.4)]);
    assert!(events.contains(&(10, Out::Cc(2, 10, 0.25))));
    assert!(events.contains(&(10, Out::Bend(2, 0.7625))));
    assert!(!events
        .iter()
        .any(|(_, event)| matches!(event, Out::Bend(3, _) | Out::Cc(3, 10, _))));
    assert_eq!(engine.channels[3].bend, 0.7);
    let mut released = Vec::new();
    engine.melody_note(false, 3, 84, 0.0, &mut |event| released.push(event));
    assert!(released.contains(&Out::Off(3, 84, 0.0)));
    let tail = advance(&mut engine, 11);
    assert!(tail.contains(&(21, Out::Off(2, 96, 0.0))));
    assert!(tail.contains(&(21, Out::Bend(2, 0.5))));
    assert!(tail.contains(&(21, Out::Cc(2, 10, 0.5))));
}

#[test]
fn delay_integration_bass_sources_and_channel_overrides_respect_dry_ownership() {
    let mut engine = delay_engine(source::BASS);
    engine.bass_note(true, 5, 36, 0.8, &mut |_| {});
    let events = attacks(&advance(&mut engine, 11));
    assert_eq!(events.len(), 1);
    assert_eq!((events[0].0, events[0].2), (10, 36));
    assert_ne!(events[0].1, engine.config.bass_channel);
    let mut blocked = delay_engine(source::STRUM);
    blocked.arp.data.pages[0].delay.taps[0].channel = 1;
    blocked.allocate_owned_voice(60, 0.8, 100, false, 0, None, &mut |_| {});
    let events = advance(&mut blocked, 20);
    assert!(attacks(&events).is_empty());
    assert!(!events
        .iter()
        .any(|(_, event)| matches!(event, Out::Bend(0, _) | Out::Cc(0, 10, _))));
    assert!(blocked
        .voices
        .iter()
        .flatten()
        .any(|voice| voice.owner == 0 && voice.note == 60));
}

#[test]
fn delay_integration_reuses_released_channels_and_panic_clears_tails() {
    let mut engine = delay_engine(source::STRUM);
    engine.arp.data.pages[0].delay.taps[0].feedback = 0.5;
    engine.allocate_owned_voice(60, 0.8, 3, false, 0, None, &mut |_| {});
    let played = attacks(&advance(&mut engine, 21));
    assert_eq!(played, vec![(10, 2, 60, 0.4), (20, 2, 60, 0.2)]);
    let mut events = Vec::new();
    engine.panic(&mut |event| events.push(event));
    assert!(events.contains(&Out::Off(2, 60, 0.0)));
    assert!(engine.voices.iter().all(Option::is_none));
    assert!(attacks(&advance(&mut engine, 1000)).is_empty());
    assert_eq!(engine.delay.deadline(engine.now, engine.beat, 500.0), None);
}

#[test]
fn delay_integration_channel_exhaustion_drops_echo_without_stealing_dry_notes() {
    let mut engine = delay_engine(source::STRUM);
    for channel in 0..16 {
        engine.melody[channel][80] = true;
    }
    engine.capture_delay_source(source::STRUM, 123, 60, 0.8, Expression::default(), Some(5));
    assert!(attacks(&advance(&mut engine, 100)).is_empty());
    assert!(engine.melody.iter().all(|notes| notes[80]));
    assert!(engine.voices.iter().all(Option::is_none));
}

#[test]
fn arp_aligned_tempo_change_retimes_remaining_beats_and_host_start_is_immediate() {
    let mut e = make_engine(ArpMode::AlignedLoop, 0);
    let mut c = e.host_config;
    c.arp_settings[0].sync = true;
    c.arp_settings[0].rate = 0.25;
    e.configure(c, &mut |_| {});
    e.transport(true, 120.0, false, &mut |_| {});
    e.set_host_position(Some(0.0), true);
    assert_eq!(attacks(&advance(&mut e, 50))[0].0, 0);
    e.transport(true, 240.0, false, &mut |_| {});
    e.set_host_position(Some(0.1), true);
    let events = attacks(&advance(&mut e, 110));
    assert_eq!(events.iter().map(|e| e.0).collect::<Vec<_>>(), [88, 150]);
    let mut fallback = make_engine(ArpMode::AlignedLoop, 0);
    fallback.playing = false;
    fallback.configure(c, &mut |_| {});
    fallback.transport(false, 120.0, false, &mut |_| {});
    advance(&mut fallback, 50);
    fallback.transport(true, 120.0, false, &mut |_| {});
    fallback.set_host_position(Some(0.0), true);
    assert_eq!(attacks(&advance(&mut fallback, 1))[0].0, 50);
}

#[test]
fn arp_once_keeps_final_step_active_until_completion_then_clears_position() {
    let mut e = make_engine(ArpMode::Once, 0);
    advance(&mut e, 201);
    assert!(e.snapshot().arp_running);
    assert_eq!(e.sources[11], 1.0);
    advance(&mut e, 100);
    assert!(!e.snapshot().arp_running);
    assert_eq!(e.sources[11], 0.0);
}

#[test]
fn arp_continuous_page_modes_preserve_phase_and_early_occurrences_never_double() {
    let mut e = make_engine(ArpMode::SyncLoop, 0);
    e.host_config.arp_settings[1].mode = ArpMode::Loop;
    advance(&mut e, 150);
    let mut c = e.host_config;
    c.arp_page = 1;
    e.configure(c, &mut |_| {});
    assert_eq!(attacks(&advance(&mut e, 51))[0].2, 67);
    let mut e = make_engine(ArpMode::SyncLoop, 0);
    e.arp.data.pages[0].steps[2].micro = -49;
    advance(&mut e, 152); // phase 2 already sounded at151
    let mut c = e.host_config;
    c.arp_page = 1;
    e.configure(c, &mut |_| {});
    assert!(attacks(&advance(&mut e, 49)).is_empty());
}

#[test]
fn delay_held_chord_echo_keeps_duration_and_page_recall_keeps_old_tails() {
    let mut e = delay_engine(source::CHORDS);
    e.allocate_owned_voice(60, 0.8, 0, true, 0, None, &mut |_| {});
    assert_eq!(attacks(&advance(&mut e, 11)), [(10, 2, 60, 0.4)]);
    let dry = e
        .voices
        .iter()
        .position(|v| v.is_some_and(|v| v.owner == 0))
        .unwrap();
    e.end_voice(dry, 0.0, &mut |_| {});
    assert!(advance(&mut e, 11).contains(&(21, Out::Off(2, 60, 0.0))));
    let mut e = delay_engine(source::STRUM);
    e.configure(e.host_config, &mut |_| {});
    e.capture_delay_source(source::STRUM, 123, 60, 0.8, Expression::default(), Some(5));
    let mut c = e.host_config;
    c.arp_page = 1;
    e.configure(c, &mut |_| {});
    assert_eq!(attacks(&advance(&mut e, 11)), [(10, 2, 60, 0.4)]);
}

#[test]
fn delay_mpe_melody_reclaims_echo_channel_before_attack_without_late_off_or_pan() {
    let mut e = delay_engine(source::STRUM);
    let mut c = e.host_config;
    c.mpe = true;
    c.members = 2;
    e.configure(c, &mut |_| {});
    e.arp.data.pages[0].delay.taps[0].pan = -1.0;
    e.capture_delay_source(source::STRUM, 123, 64, 0.8, Expression::default(), Some(5));
    assert_eq!(attacks(&advance(&mut e, 11)), [(10, 1, 64, 0.4)]);
    let mut events = Vec::new();
    e.melody_note(true, 1, 64, 0.8, &mut |event| events.push(event));
    let old_off = events
        .iter()
        .position(|e| *e == Out::Off(1, 64, 0.0))
        .unwrap();
    let centered = events
        .iter()
        .position(|e| *e == Out::Cc(1, 10, 0.5))
        .unwrap();
    let attack = events
        .iter()
        .position(|e| *e == Out::On(1, 64, 0.8))
        .unwrap();
    assert!(old_off < attack && centered < attack);
    assert!(!advance(&mut e, 20)
        .iter()
        .any(|(_, e)| *e == Out::Off(1, 64, 0.0)));
    assert!(e.melody_note_held(1, 64));
}

#[test]
fn arp_adjustment_modulation_changes_the_current_strike_pitch() {
    let mut e = make_engine(ArpMode::SyncLoop, 0);
    e.arp.data.pages[0].steps[0].mods[0] = 50;
    let mut c = e.host_config;
    c.routes[1] = routing::Route {
        source: 10,
        target: routing::TARGETS
            .iter()
            .position(|t| t.id == "transpose")
            .unwrap() as u8,
        max: 0.5,
        ..Default::default()
    };
    e.configure(c, &mut |_| {});
    let played = attacks(&advance(&mut e, 101));
    assert_eq!(played.iter().map(|e| e.2).collect::<Vec<_>>(), [72, 64]);
}
