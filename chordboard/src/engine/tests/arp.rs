use super::super::*;
use super::support::*;

#[test]
fn auto_strum_orders_notes_in_ascending_and_descending_thirds() {
    let mut e = Engine::default();
    let config_up_3rds = Config {
        mode: AUTO,
        arp_pattern: 3, // Up in 3rds
        strings: 6,
        strings_played: 6,
        ..Config::default()
    };
    configure(&mut e, config_up_3rds);
    on(&mut e, 60, 0); // C4 chord
    let notes_up: Vec<u8> = e
        .scheduled
        .iter()
        .filter_map(|s| s.map(|n| n.note))
        .collect();
    // Strings 0..6: Up in 3rds should pick index sequence:
    // i=0: 0, i=1: 2, i=2: 1, i=3: 3, i=4: 2, i=5: 4
    let expected_indices_up = [0, 2, 1, 3, 2, 4, 3, 5, 4, 0, 5, 1];
    let expected_notes_up: Vec<u8> = expected_indices_up
        .iter()
        .map(|&idx| e.notes.string(idx).unwrap())
        .collect();
    assert_eq!(notes_up, expected_notes_up);

    off(&mut e, 60, 0);
    tick(&mut e, 50000);

    let config_down_3rds = Config {
        mode: AUTO,
        arp_pattern: 4, // Down in 3rds
        strings: 6,
        strings_played: 6,
        ..Config::default()
    };
    configure(&mut e, config_down_3rds);
    on(&mut e, 60, 0);
    let notes_down: Vec<u8> = e
        .scheduled
        .iter()
        .filter_map(|s| s.map(|n| n.note))
        .collect();
    // Strings 0..6: Down in 3rds should pick descending thirds:
    // i=0: 5, i=1: 3, i=2: 4, i=3: 2, i=4: 3, i=5: 1
    let expected_indices_down = [5, 3, 4, 2, 3, 1, 2, 0, 1, 5, 0, 4];
    let expected_notes_down: Vec<u8> = expected_indices_down
        .iter()
        .map(|&idx| e.notes.string(idx).unwrap())
        .collect();
    assert_eq!(notes_down, expected_notes_down);
}
#[test]
fn arp_patterns_in_thirds_step_correctly() {
    let mut e = Engine::default();
    let config = Config {
        mode: ARP,
        arp_pattern: 3, // Up in 3rds
        octaves: 1,
        ..Config::default()
    };
    configure(&mut e, config);
    on(&mut e, 60, 0);
    let total = e.notes.len;
    assert!(total >= 3);
    // Up in 3rds: pairs (0, 2), (1, 3), (2, 4 % total)...
    let mut played = Vec::new();
    for _ in 0..(total * 2) {
        let events = tick(&mut e, 5000);
        for ev in events {
            if let Out::On(_, note, _) = ev {
                played.push(note);
            }
        }
    }
    assert_eq!(played[0], e.notes.string(0).unwrap());
    assert_eq!(played[1], e.notes.string(2 % total).unwrap());
    assert_eq!(played[2], e.notes.string(1).unwrap());
    assert_eq!(played[3], e.notes.string(3 % total).unwrap());
}
#[test]
fn auto_strum_hold_plays_until_next_chord() {
    let mut e = Engine::default();
    let config = Config {
        mode: AUTO,
        strum_hold: true,
        strings: 4,
        strings_played: 4,
        ..Config::default()
    };
    configure(&mut e, config);
    on(&mut e, 60, 0); // Chord 1: C
                       // All scheduled notes should have u64::MAX duration
    assert!(e.scheduled.iter().flatten().all(|s| s.duration == u64::MAX));

    // Tick past strum
    let events1 = tick(&mut e, 20000);
    assert!(!events1.is_empty());
    // Voices should be active and not off
    assert!(e
        .voices
        .iter()
        .flatten()
        .any(|v| !v.layer && v.off == u64::MAX));

    // Strike next chord (G)
    off(&mut e, 60, 0);
    let mut next_events = Vec::new();
    e.midi_note(true, 0, 67, 0.8, &mut |v| next_events.push(v));

    // Previous chord voices should receive Off events
    assert!(next_events.iter().any(|v| matches!(v, Out::Off(_, _, _))));
}
#[test]
fn melody_affects_chords_without_exact_doubling_and_restores_on_release() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            key_split: true,
            split_note: 52,
            control_base: 36,
            affect_chords: true,
            always_chord: true,
            voice_leading: 2,
            ..Config::default()
        },
    );
    on(&mut e, 48, 0);
    let base = e.full_notes;
    on(&mut e, 52, 1);
    assert!(!e.full_notes.as_slice().contains(&52));
    assert!(e.full_notes.as_slice().iter().any(|n| n % 12 == 4));
    assert!(!e.voices.iter().flatten().any(|v| v.note == 52));
    on(&mut e, 62, 1);
    assert!(e.full_notes.as_slice().iter().any(|n| n % 12 == 2));
    assert!(!e.full_notes.as_slice().contains(&62));
    off(&mut e, 52, 1);
    off(&mut e, 62, 1);
    assert_eq!(e.full_notes, base);
}
#[test]
fn melody_affects_chords_omits_pitch_when_all_register_octaves_are_held() {
    let base = harmony::voice(60, 0, None, 0, 0, 0);
    let mut melody = [[false; 128]; 16];
    for n in [52, 64, 76] {
        melody[1][n] = true;
    }
    let voiced = harmony::melody_harmony(base, &melody);
    assert!(!voiced.as_slice().iter().any(|n| n % 12 == 4));
    assert!(voiced.as_slice().contains(&60));
    assert!(voiced.as_slice().contains(&67));
}
#[test]
fn melody_affects_chords_disabled_preserves_original_harmony() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            key_split: true,
            split_note: 52,
            control_base: 36,
            affect_chords: false,
            voice_leading: 2,
            ..Config::default()
        },
    );
    on(&mut e, 48, 0);
    let base = e.full_notes;
    on(&mut e, 62, 1);
    assert_eq!(e.full_notes, base);
}
#[test]
fn melody_affects_chords_preserves_pending_sweep_timing() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            key_split: true,
            split_note: 52,
            control_base: 36,
            affect_chords: true,
            mode: AUTO,
            strum_ms: 120.0,
            voice_leading: 2,
            ..Config::default()
        },
    );
    on(&mut e, 48, 0);
    let before: Vec<_> = e.scheduled.iter().flatten().map(|s| s.at).collect();
    assert!(!before.is_empty());
    on(&mut e, 52, 1);
    let after: Vec<_> = e.scheduled.iter().flatten().map(|s| s.at).collect();
    assert_eq!(after, before);
    assert!(!e.scheduled.iter().flatten().any(|s| s.note == 52));
}
#[test]
fn enabling_melody_affects_chords_removes_live_exact_collisions() {
    let mut e = Engine::default();
    let config = Config {
        key_split: true,
        split_note: 52,
        control_base: 36,
        always_chord: true,
        voice_leading: 2,
        ..Config::default()
    };
    configure(&mut e, config);
    on(&mut e, 48, 0);
    on(&mut e, 52, 1);
    assert!(e.voices.iter().flatten().any(|v| v.note == 52));
    configure(
        &mut e,
        Config {
            affect_chords: true,
            ..config
        },
    );
    assert!(!e.full_notes.as_slice().contains(&52));
    assert!(!e.voices.iter().flatten().any(|v| v.note == 52));
    assert!(e.melody_note_held(1, 52));
    configure(&mut e, config);
    assert!(e.full_notes.as_slice().contains(&52));
}
#[test]
fn velocity_contour_preserves_linear_defaults_and_bends_monotonically() {
    for tilt in [-1.0, 0.0, 1.0] {
        for curve in [-1.0, 0.0, 1.0] {
            assert_eq!(contour_factor(0.0, tilt, curve), 1.0 - tilt * 0.5);
            assert_eq!(contour_factor(1.0, tilt, curve), 1.0 + tilt * 0.5);
            let mut previous = contour_factor(0.0, tilt, curve);
            for i in 1..=100 {
                let t = i as f32 / 100.0;
                let value = contour_factor(t, tilt, curve);
                assert!((0.5..=1.5).contains(&value));
                assert!((value - previous) * tilt >= -0.00001);
                if curve == 0.0 {
                    assert_eq!(value, 1.0 + tilt * (t - 0.5));
                }
                previous = value;
            }
        }
    }
    assert!(contour_factor(0.5, 1.0, 1.0) > 1.0);
    assert!(contour_factor(0.5, 1.0, -1.0) < 1.0);
}
#[test]
fn manual_plucks_apply_velocity_contour_and_curve() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: MANUAL,
            strings: 3,
            contour: 1.0,
            contour_curve: 1.0,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    let base = e.strike_velocity();
    for index in 0..3 {
        let mut out = Vec::new();
        e.pluck_string(index, &mut |v| out.push(v));
        let actual = out
            .iter()
            .find_map(|v| {
                if let Out::On(_, _, velocity) = v {
                    Some(*velocity)
                } else {
                    None
                }
            })
            .unwrap();
        let expected = (base * contour_factor(index as f32 / 2.0, 1.0, 1.0)).clamp(0.01, 1.0);
        assert!((actual - expected).abs() < 0.00001);
    }
}
#[test]
fn arp_velocity_contour_follows_each_pattern_cycle_and_repeats() {
    for pattern in 0..7 {
        for tilt in [-1.0, 0.0, 1.0] {
            let mut e = Engine::default();
            e.sample_rate = 1000.0;
            configure(
                &mut e,
                Config {
                    mode: ARP,
                    arp_pattern: pattern,
                    octaves: 2,
                    rate: 0.01,
                    humanize: 0.0,
                    velocity: 0.4,
                    contour: tilt,
                    contour_curve: 0.6,
                    ..Config::default()
                },
            );
            on(&mut e, 60, 1);
            let base = e.strike_velocity();
            let total = e.notes.len * 2;
            let cycle = match pattern {
                2 => total * 2 - 2,
                3 | 4 => total * 2,
                _ => total,
            };
            let mut velocities = Vec::new();
            while velocities.len() < cycle * 2 {
                e.tick(&mut |event| {
                    if let Out::On(_, _, v) = event {
                        velocities.push(v);
                    }
                });
                assert!(e.now < 1000, "arpeggiator stopped producing notes");
            }
            for (step, actual) in velocities.iter().enumerate() {
                let t = (step % cycle) as f32 / (cycle - 1) as f32;
                let expected = (base * contour_factor(t, tilt, 0.6)).clamp(0.01, 1.0);
                assert!(
                    (actual - expected).abs() < 0.00001,
                    "pattern {pattern}, step {step}: {actual} != {expected}"
                );
            }
        }
    }
}
#[test]
fn manual_selection_modes_trigger_silence_root_or_chord_for_midi_and_ui() {
    for ui in [false, true] {
        for selected in 0..3 {
            let mut e = Engine::default();
            configure(
                &mut e,
                Config {
                    mode: MANUAL,
                    root_on_select: selected == 1,
                    always_chord: selected == 2,
                    voice_leading: 2,
                    ..Config::default()
                },
            );
            let events = if ui {
                send(&mut e, Command::KeyDown(0, 60, 0))
            } else {
                on(&mut e, 60, 0)
            };
            let mut sounded = notes(&events);
            sounded.sort_unstable();
            let expected = match selected {
                0 => vec![],
                1 => vec![60],
                _ => vec![60, 64, 67],
            };
            assert_eq!(sounded, expected, "selection {selected}, UI {ui}");
            if ui {
                send(&mut e, Command::KeyUp(0));
            } else {
                off(&mut e, 60, 0);
            }
            tick(&mut e, 96000);
            assert!(e.voices.iter().all(Option::is_none));
        }
    }
}
