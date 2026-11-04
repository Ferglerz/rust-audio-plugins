//! Regression coverage for global harmony, fixed clock slots, and Custom playback.
use super::*;
use crate::arpeggiator::{ArpMode, Fill, Settings, StepState, TrackerRow};

fn chord(root: u8, quality: u8) -> harmony::SavedChord {
    harmony::SavedChord {
        root,
        quality,
        second: None,
        additions: 0,
        inversion: 0,
        spread: 0,
        transpose: 0,
    }
}

fn settings() -> Settings {
    Settings {
        mode: ArpMode::SyncLoop,
        sync: false,
        free_ms: 100.0,
        lock_length: true,
        steps: 4,
        ..Settings::default()
    }
}

fn row(id: u64, chord: harmony::SavedChord, settings: Settings) -> TrackerRow {
    let mut row = TrackerRow {
        id,
        chord: Some(chord),
        duration: u32::MAX,
        settings,
        ..TrackerRow::default()
    };
    row.page.ensure_loops(settings.octaves as usize);
    row
}

fn tracker(rows: Vec<TrackerRow>, config: Config) -> Engine {
    let mut engine = Engine {
        sample_rate: 1000.0,
        ..Engine::default()
    };
    engine.arp.data.rows = rows;
    engine.configure(
        Config {
            mode: MANUAL,
            arp_enabled: true,
            tracker_enabled: true,
            bass_enabled: false,
            velocity: 0.5,
            routes: [routing::Route::default(); routing::ROUTE_COUNT],
            ..config
        },
        &mut |_| {},
    );
    engine
}

fn advance(engine: &mut Engine, samples: usize) -> Vec<(u64, Out)> {
    let start = engine.now;
    let mut result = Vec::new();
    engine.advance(samples, &mut |offset, event| {
        result.push((start + offset as u64, event));
    });
    result
}

fn attacks(events: &[(u64, Out)]) -> Vec<(u64, u8, f32)> {
    events
        .iter()
        .filter_map(|&(at, event)| match event {
            Out::On(_, note, velocity) => Some((at, note, velocity)),
            _ => None,
        })
        .collect()
}

fn pitches(events: &[(u64, Out)]) -> Vec<u8> {
    attacks(events).iter().map(|(_, note, _)| *note).collect()
}

#[test]
fn globals_resolve_live_and_tracker_chords_identically_without_baking_offsets() {
    let config = Config {
        mode: MANUAL,
        bass_enabled: false,
        base_inversion: 1,
        complexity: 3,
        transpose: 2,
        voicing_override: 1,
        routes: [routing::Route::default(); routing::ROUTE_COUNT],
        ..Config::default()
    };
    let saved = chord(60, 0);
    let mut live = Engine::default();
    live.configure(config, &mut |_| {});
    live.note_on(0, 60, 0, 0.8, Some(0), &mut |_| {});
    let mut arp = tracker(vec![row(1, saved, settings())], config);
    assert_eq!(live.notes.as_slice(), &[66, 69, 73, 74, 76]);
    assert_eq!(live.notes, arp.arp_cycle_chord());
    let before = arp.arp.data.rows[0].clone();
    for _ in 0..4 {
        arp.configure(arp.host_config, &mut |_| {});
        live.configure(live.host_config, &mut |_| {});
    }
    assert_eq!(live.notes, arp.arp_cycle_chord());
    assert_eq!(arp.arp.data.rows[0], before);
    let original = Config {
        base_inversion: 0,
        complexity: 0,
        transpose: 0,
        voicing_override: 0,
        ..arp.host_config
    };
    arp.configure(original, &mut |_| {});
    assert_eq!(arp.arp_cycle_chord().as_slice(), &[60, 64, 67]);
}

#[test]
fn saved_inversion_then_global_then_loop_inversion_and_octave_are_additive() {
    let saved = harmony::SavedChord {
        inversion: 1,
        transpose: 12,
        ..chord(60, 0)
    };
    let mut first = row(1, saved, settings());
    first.page.loops[0].inversion = 1;
    first.page.loops[0].octave = -1;
    let mut engine = tracker(
        vec![first],
        Config {
            base_inversion: -1,
            transpose: -2,
            ..Config::default()
        },
    );
    assert_eq!(pitches(&advance(&mut engine, 301)), [62, 65, 70, 62]);
    assert_eq!(engine.arp.data.rows[0].chord, Some(saved));
}

#[test]
fn fixed_length_keeps_three_four_and_five_tone_rows_on_identical_boundaries() {
    let shapes = [
        chord(60, 0),
        chord(60, 3),
        harmony::SavedChord {
            additions: 1 << 2,
            ..chord(60, 3)
        },
    ];
    for saved in shapes {
        let mut first = row(1, saved, settings());
        first.duration = 2;
        let next = row(2, chord(48, 0), settings());
        let mut engine = tracker(vec![first, next], Config::default());
        let played = attacks(&advance(&mut engine, 801));
        assert_eq!(
            played.iter().map(|(at, _, _)| *at).collect::<Vec<_>>(),
            (0..=8).map(|i| i * 100).collect::<Vec<_>>()
        );
        assert_eq!(
            played[8].1, 48,
            "next row must begin at the same clock boundary"
        );
    }
}

#[test]
fn repeat_restarts_each_loop_and_evolve_carries_across_groups_then_resets_by_row() {
    for (fill, expected) in [
        (
            Fill::Repeat,
            vec![60, 64, 67, 71, 60, 64, 67, 71, 60, 64, 67, 71],
        ),
        (
            Fill::Evolve,
            vec![60, 64, 67, 71, 74, 60, 64, 67, 71, 74, 60, 64],
        ),
    ] {
        let settings = Settings {
            octaves: 2,
            fill,
            ..settings()
        };
        let mut first = row(
            1,
            harmony::SavedChord {
                additions: 1 << 2,
                ..chord(60, 3)
            },
            settings,
        );
        first.duration = 2;
        let next = row(2, chord(48, 0), settings);
        let mut engine = tracker(vec![first, next], Config::default());
        let events = advance(&mut engine, 1601);
        let played = pitches(&events);
        assert_eq!(&played[..12], expected.as_slice());
        assert_eq!(played[16], 48, "new row restarts Evolve at the first tone");
    }
}

#[test]
fn locked_skips_remove_time_while_mutes_and_probability_misses_keep_slots() {
    let mut first = row(1, chord(60, 0), settings());
    first.duration = 1;
    first.page.loops[0].steps[0].state = StepState::Skip;
    first.page.loops[0].steps[1].state = StepState::Mute;
    first.page.loops[0].steps[2].probability = 0;
    let mut engine = tracker(
        vec![first, row(2, chord(48, 0), settings())],
        Config::default(),
    );
    assert_eq!(
        attacks(&advance(&mut engine, 401)),
        [(200, 60, 0.5), (300, 48, 0.5), (400, 52, 0.5)]
    );
}

#[test]
fn custom_plays_chord_relative_scale_degrees_independent_of_triad_complexity() {
    let settings = Settings {
        pattern: 7,
        lock_length: false,
        ..settings()
    };
    let mut engine = tracker(
        vec![row(1, chord(62, 1), settings)],
        Config {
            key: 0,
            scale: 0,
            complexity: 1,
            ..Config::default()
        },
    );
    assert_eq!(
        pitches(&advance(&mut engine, 601)),
        [62, 64, 65, 67, 69, 71, 72]
    );
}

#[test]
fn custom_modifiers_compose_with_loop_cells_and_global_gate() {
    let mut settings = Settings {
        pattern: 7,
        lock_length: false,
        ..settings()
    };
    settings.custom[0].velocity = 80;
    settings.custom[0].gate = 50;
    settings.custom[1].rest = true;
    settings.custom[2].octave = 1;
    let mut first = row(1, chord(60, 0), settings);
    first.page.loops[0].steps[0].velocity = 50;
    first.page.loops[0].steps[0].gate = 80;
    let mut engine = tracker(
        vec![first],
        Config {
            gate_scale: 2.0,
            ..Config::default()
        },
    );
    assert!((engine.snapshot().arp_gates[0] - 0.8).abs() < 0.00001);
    let events = advance(&mut engine, 301);
    let played = attacks(&events);
    assert_eq!(
        played
            .iter()
            .map(|(at, note, _)| (*at, *note))
            .collect::<Vec<_>>(),
        [(0, 60), (200, 76), (300, 65)]
    );
    assert!((played[0].2 - 0.2).abs() < 0.00001);
    assert!(events.contains(&(80, Out::Off(0, 60, 0.0))));
}

#[test]
fn custom_fixed_repeat_and_evolve_traverse_seven_steps_without_pitch_class_rotation() {
    for (fill, expected) in [
        (Fill::Repeat, vec![72, 74, 64, 65, 72, 74, 64, 65]),
        (Fill::Evolve, vec![72, 74, 64, 65, 67, 69, 71, 72]),
    ] {
        let settings = Settings {
            pattern: 7,
            fill,
            ..settings()
        };
        let mut first = row(1, chord(60, 0), settings);
        first.page.loops[0].inversion = 1;
        let mut engine = tracker(vec![first], Config::default());
        assert_eq!(pitches(&advance(&mut engine, 701)), expected);
    }
}

#[test]
fn global_gate_scales_existing_step_gate_and_clamps_to_a_full_step() {
    for (scale, expected_off) in [(0.5, 40), (1.0, 80), (2.0, 100)] {
        let mut first = row(1, chord(60, 0), settings());
        first.page.loops[0].steps[0].gate = 80;
        let mut engine = tracker(
            vec![first],
            Config {
                gate_scale: scale,
                ..Config::default()
            },
        );
        let events = advance(&mut engine, 101);
        assert!(
            events.contains(&(expected_off, Out::Off(0, 60, 0.0))),
            "gate multiplier {scale}"
        );
    }
}

#[test]
fn complexity_changes_do_not_restart_a_locked_loop() {
    let mut engine = tracker(vec![row(1, chord(60, 0), settings())], Config::default());
    let mut events = advance(&mut engine, 150);
    engine.configure(
        Config {
            complexity: 3,
            ..engine.host_config
        },
        &mut |_| {},
    );
    events.extend(advance(&mut engine, 351));
    let played = attacks(&events);
    assert_eq!(
        played.iter().map(|(at, _, _)| *at).collect::<Vec<_>>(),
        [0, 100, 200, 300, 400, 500]
    );
    assert_eq!(played[3].1, 71);
    assert_eq!(played[4].1, 60);
}

#[test]
fn fixed_length_changes_are_latched_until_the_next_loop_boundary() {
    let mut engine = tracker(vec![row(1, chord(60, 0), settings())], Config::default());
    let mut events = advance(&mut engine, 150);
    engine.arp.data.rows[0].settings.steps = 2;
    engine.arp.data.generation += 1;
    engine.accept_arp_generation(&mut |_| {});
    events.extend(advance(&mut engine, 551));
    assert_eq!(pitches(&events), [60, 64, 67, 60, 60, 64, 60, 64]);
}

#[test]
fn custom_evolving_clock_is_invariant_to_audio_callback_partitioning() {
    fn fixture() -> Engine {
        let settings = Settings {
            pattern: 7,
            fill: Fill::Evolve,
            octaves: 2,
            swing: 0.3,
            humanize: 0.4,
            free_ms: 13.25,
            ..settings()
        };
        let mut first = row(1, chord(62, 1), settings);
        first.page.loops[0].steps[1].micro = -20;
        first.page.loops[1].inversion = -1;
        first.page.loops[1].steps[2].probability = 50;
        tracker(
            vec![first],
            Config {
                gate_scale: 1.25,
                ..Config::default()
            },
        )
    }
    let mut whole = fixture();
    let expected = advance(&mut whole, 2000);
    for chunk in [1, 7, 64, 127, 511] {
        let mut split = fixture();
        let mut observed = Vec::new();
        while split.now < 2000 {
            let count = (2000 - split.now).min(chunk) as usize;
            observed.extend(advance(&mut split, count));
        }
        assert_eq!(observed, expected, "callback size {chunk}");
        assert_eq!(split.snapshot(), whole.snapshot());
    }
}

#[test]
fn early_final_note_does_not_drop_a_late_preceding_note_or_move_the_row_boundary() {
    let timing = Settings {
        steps: 3,
        swing: 0.75,
        ..settings()
    };
    let mut first = row(1, chord(60, 0), timing);
    first.duration = 1;
    first.page.loops[0].steps[1].micro = 49;
    first.page.loops[0].steps[2].micro = -49;
    let next = row(2, chord(48, 0), timing);
    let mut engine = tracker(vec![first, next], Config::default());
    let events = advance(&mut engine, 376);
    assert_eq!(
        attacks(&events)
            .iter()
            .map(|(at, note, _)| (*at, *note))
            .collect::<Vec<_>>(),
        [(0, 60), (175, 67), (187, 64), (375, 48)]
    );
    assert_eq!(engine.arp.row_index, 1);
}

#[test]
fn invalid_custom_pitch_cannot_become_a_low_note_after_octave_offsets() {
    for (loop_octave, cell_octave) in [(1, 0), (0, 1)] {
        let mut timing = Settings {
            pattern: 7,
            ..settings()
        };
        timing.custom[0].degree = 2;
        let mut first = row(1, chord(127, 0), timing);
        first.page.loops[0].octave = loop_octave;
        first.page.loops[0].steps[0].octave = cell_octave;
        let mut engine = tracker(vec![first], Config::default());
        assert!(
            attacks(&advance(&mut engine, 1)).is_empty(),
            "an out-of-range degree must not become a sentinel-derived low MIDI note"
        );
    }
}

#[test]
fn custom_degree_above_midi_range_can_return_through_a_negative_octave() {
    for (custom_octave, cell_octave) in [(-1, 0), (0, -1)] {
        let mut timing = Settings {
            pattern: 7,
            ..settings()
        };
        timing.custom[0].degree = 2;
        timing.custom[0].octave = custom_octave;
        let mut first = row(1, chord(126, 0), timing);
        first.page.loops[0].steps[0].octave = cell_octave;
        let mut engine = tracker(
            vec![first],
            Config {
                key: 6,
                ..Config::default()
            },
        );
        assert_eq!(pitches(&advance(&mut engine, 1)), [116]);
    }
}

#[test]
fn changing_odd_to_even_loop_length_keeps_the_swing_clock_parity() {
    let timing = Settings {
        steps: 3,
        swing: 0.5,
        ..settings()
    };
    let mut engine = tracker(vec![row(1, chord(60, 0), timing)], Config::default());
    let mut events = advance(&mut engine, 201);
    engine.arp.data.rows[0].settings.steps = 4;
    engine.arp.data.generation += 1;
    engine.accept_arp_generation(&mut |_| {});
    events.extend(advance(&mut engine, 400));
    assert_eq!(
        attacks(&events)
            .iter()
            .map(|(at, _, _)| *at)
            .collect::<Vec<_>>(),
        [0, 150, 200, 350, 400, 550, 600]
    );
    assert_eq!(pitches(&events), [60, 64, 67, 60, 64, 67, 60]);
}

#[test]
fn legacy_host_pattern_changes_are_not_hidden_by_latched_settings() {
    let mut engine = Engine {
        sample_rate: 1000.0,
        ..Engine::default()
    };
    engine.configure(
        Config {
            mode: MANUAL,
            arp_enabled: true,
            bass_enabled: false,
            arp_settings: [settings(); 8],
            routes: [routing::Route::default(); routing::ROUTE_COUNT],
            ..Config::default()
        },
        &mut |_| {},
    );
    engine.note_on(0, 60, 0, 0.8, Some(0), &mut |_| {});
    let mut events = advance(&mut engine, 150);
    let mut requested = engine.host_config;
    requested.arp_settings[0].pattern = 1;
    engine.configure(requested, &mut |_| {});
    events.extend(advance(&mut engine, 551));
    assert_eq!(pitches(&events), [60, 64, 67, 60, 67, 64, 60, 67]);
}

#[test]
fn snapshot_custom_gates_follow_evolve_sequence_position() {
    let mut timing = Settings {
        pattern: 7,
        fill: Fill::Evolve,
        ..settings()
    };
    timing.custom[0].gate = 50;
    timing.custom[4].gate = 25;
    let first = row(1, chord(60, 0), timing);
    let mut engine = tracker(vec![first], Config::default());
    assert!((engine.snapshot().arp_gates[0] - 0.325).abs() < 0.00001);
    advance(&mut engine, 401);
    assert!((engine.snapshot().arp_gates[0] - 0.1625).abs() < 0.00001);
}

#[test]
fn inaudible_locked_voicing_still_advances_at_the_row_boundary() {
    let mut first = row(1, chord(127, 0), settings());
    first.duration = 1;
    let second = row(2, chord(48, 0), settings());
    let mut engine = tracker(
        vec![first, second],
        Config {
            base_inversion: 12,
            ..Config::default()
        },
    );
    let played = attacks(&advance(&mut engine, 401));
    assert_eq!(played.len(), 1);
    assert_eq!(played[0].0, 400);
    assert_eq!(engine.arp.row_index, 1);
}

#[test]
fn velocity_multiplier_boosts_cell_and_custom_dynamics_without_changing_clock() {
    for (scale, expected) in [(0.0, 0.0), (0.5, 0.1), (1.0, 0.2), (2.0, 0.4)] {
        let mut program = settings();
        program.pattern = 7;
        program.custom[0].velocity = 80;
        let mut first = row(1, chord(60, 0), program);
        first.page.loops[0].steps[0].velocity = 50;
        let mut engine = tracker(
            vec![first],
            Config {
                velocity_scale: scale,
                ..Config::default()
            },
        );
        let played = attacks(&advance(&mut engine, 401));
        if scale == 0.0 {
            assert!(played.is_empty());
            let mut config = engine.host_config;
            config.velocity_scale = 2.0;
            engine.configure(config, &mut |_| {});
            assert_eq!(attacks(&advance(&mut engine, 100))[0].0, 500);
        } else {
            assert!((played[0].2 - expected).abs() < 0.00001);
            assert_eq!(played[4].0, 400);
            assert!(played.iter().all(|&(_, _, velocity)| velocity <= 1.0));
        }
    }
}

#[test]
fn velocity_multiplier_clamps_only_after_composing_step_gains() {
    let mut program = settings();
    program.pattern = 7;
    program.custom[0].velocity = 200;
    let mut first = row(1, chord(60, 0), program);
    first.page.loops[0].steps[0].velocity = 200;
    let mut engine = tracker(
        vec![first],
        Config {
            velocity_scale: 2.0,
            ..Config::default()
        },
    );
    assert_eq!(attacks(&advance(&mut engine, 1))[0].2, 1.0);
}

#[test]
fn fresh_plugin_plays_selected_chord_and_global_velocity_scales_live_attacks() {
    let params = crate::params::ChordboardParams::default();
    assert!(params.config().always_chord);
    for scale in [0.0, 0.5, 1.0, 2.0] {
        let mut engine = Engine::default();
        let mut config = params.config();
        config.velocity_scale = scale;
        config.routes = [routing::Route::default(); routing::ROUTE_COUNT];
        engine.configure(config, &mut |_| {});
        let mut played = Vec::new();
        engine.command(Command::KeyDown(0, 60, 0), &mut |event| {
            if let Out::On(_, _, velocity) = event {
                played.push(velocity);
            }
        });
        if scale == 0.0 {
            assert!(played.is_empty());
        } else {
            assert!(played.len() >= 3);
            assert!(played
                .iter()
                .all(|&velocity| (velocity - (0.8 * scale).min(1.0)).abs() < 0.00001));
        }
    }
}

#[test]
fn daw_alignment_is_global_across_rows_without_changing_their_repeat_setting() {
    let synced = Settings { sync: true, ..settings() };
    let mut engine = tracker(vec![row(1, chord(60, 0), synced), row(2, chord(48, 0), synced)], Config { daw_alignment: true, ..Config::default() });
    engine.transport(true, 120.0, false, &mut |_| {});
    engine.set_host_position(Some(0.3), true);
    assert!(engine.arp_alignment_available());
    assert_eq!(engine.config.arp_mode, ArpMode::AlignedLoop);
    engine.arp.row_index = 1;
    engine.configure(engine.host_config, &mut |_| {});
    assert!(engine.arp_alignment_available());
    assert_eq!(engine.arp.data.rows[1].settings.mode, ArpMode::SyncLoop);
    let mut config = engine.host_config;
    config.daw_alignment = false;
    engine.configure(config, &mut |_| {});
    assert!(!engine.arp_alignment_available());
    assert_eq!(engine.config.arp_mode, ArpMode::SyncLoop);
}
