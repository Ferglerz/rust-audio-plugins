use super::super::*;
use super::support::*;

#[test]
fn last_ignored_key_restarts_with_its_original_quality() {
    for release_order in [[0, 1], [1, 0]] {
        let mut e = Engine::default();
        send(&mut e, Command::KeyDown(0, 60, 0));
        send(&mut e, Command::KeyDown(1, 62, 0));
        send(&mut e, Command::KeyDown(12, 65, 1));
        assert_eq!(e.snapshot().ignored, 1 << 12);
        send(&mut e, Command::KeyUp(release_order[0]));
        let events = send(&mut e, Command::KeyUp(release_order[1]));
        assert_eq!(notes(&events), vec![65, 68, 72]);
        assert_eq!(e.snapshot().ignored, 0);
        assert_eq!(e.snapshot().accepted, 1 << 12);
        send(&mut e, Command::KeyUp(12));
        assert!(e.voices.iter().all(Option::is_none));
    }
}
#[test]
fn ignored_release_can_leave_a_single_pending_midi_root() {
    let mut e = Engine::default();
    on(&mut e, 60, 1);
    on(&mut e, 62, 2);
    on(&mut e, 65, 3);
    e.midi_note(true, 4, 69, 0.4, &mut |_| {});
    off(&mut e, 60, 1);
    off(&mut e, 62, 2);
    assert!(e.root.is_none());
    e.pressure(4, None, 0.7, &mut |_| {});
    let events = off(&mut e, 65, 3);
    assert_eq!(notes(&events), vec![69, 73, 76]);
    assert_eq!(e.root.unwrap().velocity, 0.4);
    assert_eq!(e.expression.pressure, 0.7);
    off(&mut e, 69, 4);
    assert!(e.voices.iter().all(Option::is_none));
}
#[test]
fn last_string_tracks_first_inversion_chord_roots() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: MANUAL,
            inversion: 1,
            ..Config::default()
        },
    );
    for root in [48, 50, 53, 55, 59, 60] {
        on(&mut e, root, 1);
        let events = send(&mut e, Command::BeginGesture(1.0, 0.8));
        assert_eq!(notes(&events), vec![root + 31]);
        off(&mut e, root, 1);
    }
}
#[test]
fn overlapping_first_inversion_chords_update_the_last_string() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: MANUAL,
            inversion: 1,
            ..Config::default()
        },
    );
    send(&mut e, Command::KeyDown(0, 60, 0));
    assert_eq!(
        notes(&send(&mut e, Command::BeginGesture(1.0, 0.8))),
        vec![91]
    );
    send(&mut e, Command::KeyDown(5, 65, 0));
    send(&mut e, Command::KeyUp(0));
    assert_eq!(
        notes(&send(&mut e, Command::BeginGesture(1.0, 0.8))),
        vec![96]
    );
}
#[test]
fn pending_last_key_restarts_each_playback_mode() {
    for mode in [CHORD, AUTO, MANUAL, ARP] {
        let mut e = Engine::default();
        configure(
            &mut e,
            Config {
                mode,
                ..Config::default()
            },
        );
        on(&mut e, 60, 1);
        on(&mut e, 62, 2);
        on(&mut e, 65, 3);
        off(&mut e, 60, 1);
        let mut events = off(&mut e, 62, 2);
        assert_eq!(e.notes.as_slice(), &[65, 69, 72]);
        if mode == MANUAL {
            events.extend(send(&mut e, Command::BeginGesture(0.0, 0.8)));
        } else {
            events.extend(tick(&mut e, 1));
        }
        assert!(notes(&events).contains(&65), "mode {mode}");
        off(&mut e, 65, 3);
        tick(&mut e, 100_000);
        assert!(e.voices.iter().all(Option::is_none));
        assert!(e.scheduled.iter().all(Option::is_none));
    }
}
#[test]
fn full_strings_emit_the_displayed_d_sharp_major_pitches() {
    for mpe in [false, true] {
        for inversion in 0..3 {
            for spread in 0..3 {
                let mut e = Engine::default();
                configure(
                    &mut e,
                    Config {
                        mode: MANUAL,
                        strings: 12,
                        mpe,
                        inversion,
                        spread,
                        ..Config::default()
                    },
                );
                on(&mut e, 63, 1);
                let displayed = e.snapshot().notes;
                for i in 0..12 {
                    let events = send(&mut e, Command::BeginGesture(i as f32 / 11.0, 0.8));
                    assert_eq!(
                        notes(&events),
                        vec![displayed.string(i).unwrap()],
                        "string {i}, inversion {inversion}, spread {spread}, mpe {mpe}"
                    );
                    assert!(notes(&events)
                        .iter()
                        .all(|n| [3, 7, 10].contains(&(n % 12))));
                }
            }
        }
    }
}
#[test]
fn control_intervals_select_transposed_chords_and_survive_revoicing() {
    let expected: [&[u8]; 12] = [
        &[0, 4, 7],
        &[0, 4, 7, 13],
        &[0, 2, 7],
        &[0, 3, 7],
        &[0, 4, 7],
        &[0, 5, 7],
        &[0, 3, 6],
        &[0, 7],
        &[0, 4, 8],
        &[0, 4, 7, 9],
        &[0, 4, 7, 10],
        &[0, 4, 7, 11],
    ];
    for (interval, tones) in expected.iter().enumerate() {
        let mut e = Engine::default();
        configure(
            &mut e,
            Config {
                control_base: 36,
                ..Config::default()
            },
        );
        on(&mut e, 36 + interval as u8, 1);
        on(&mut e, 63, 2);
        assert_eq!(
            e.notes.as_slice(),
            tones.iter().map(|n| n + 63).collect::<Vec<_>>()
        );
        e.rebuild(&mut |_| {});
        assert_eq!(
            e.notes.as_slice(),
            tones.iter().map(|n| n + 63).collect::<Vec<_>>()
        );
        assert!(e.second.is_none());
    }
}
#[test]
fn chord_changes_preserve_ringing_notes_until_their_deadlines() {
    for mode in [CHORD, AUTO, MANUAL, ARP] {
        let mut e = Engine::default();
        configure(
            &mut e,
            Config {
                mode,
                latch: true,
                ..Config::default()
            },
        );
        on(&mut e, 60, 1);
        if mode == MANUAL {
            send(&mut e, Command::BeginGesture(0.0, 0.8));
        }
        tick(&mut e, 1);
        let voice = *e.voices.iter().flatten().find(|v| v.note == 60).unwrap();
        off(&mut e, 60, 1);
        let events = on(&mut e, 61, 1);
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Out::Off(_, 60, _))),
            "mode {mode}"
        );
        let tail = *e.voices.iter().flatten().find(|v| v.note == 60).unwrap();
        assert_eq!(
            tail.off,
            if mode == CHORD {
                e.now + e.duration()
            } else {
                voice.off
            }
        );
        e.now = tail.off;
        let events = tick(&mut e, 1);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Out::Off(_, 60, _))),
            "mode {mode}"
        );
    }
}
#[test]
fn auto_full_strings_match_display_in_both_directions() {
    for direction in 0..2 {
        for spread in 0..3 {
            let mut e = Engine::default();
            configure(
                &mut e,
                Config {
                    mode: AUTO,
                    strings: 12,
                    strum_ms: 0.0,
                    strum_sync: false,
                    spread,
                    arp_pattern: direction,
                    inversion: 1,
                    ..Config::default()
                },
            );
            on(&mut e, 63, 1);
            let mut expected = (0..12)
                .filter_map(|i| e.snapshot().notes.string(i))
                .collect::<Vec<_>>();
            assert_eq!(expected.len(), 12);
            assert!(expected.iter().all(|n| [3, 7, 10].contains(&(n % 12))));
            if direction == 1 {
                expected.reverse();
            }
            assert_eq!(notes(&tick(&mut e, 1)), expected);
        }
    }
}
#[test]
fn released_strums_ring_across_the_next_chord_without_hold() {
    for mode in [AUTO, MANUAL, ARP] {
        let mut e = Engine::default();
        configure(
            &mut e,
            Config {
                mode,
                ..Config::default()
            },
        );
        on(&mut e, 63, 1);
        if mode == MANUAL {
            send(&mut e, Command::BeginGesture(0.0, 0.8));
        }
        tick(&mut e, 1);
        let deadline = e
            .voices
            .iter()
            .flatten()
            .find(|v| v.note == 63)
            .unwrap()
            .off;
        assert!(!off(&mut e, 63, 1).iter().any(|e| matches!(e, Out::Off(..))));
        configure(
            &mut e,
            Config {
                mode,
                ..Config::default()
            },
        );
        on(&mut e, 65, 1);
        assert_eq!(
            e.voices
                .iter()
                .flatten()
                .find(|v| v.note == 63)
                .unwrap()
                .off,
            deadline
        );
    }
}
#[test]
fn auto_strings_played_limits_sweeps_without_changing_the_layout() {
    for total in [3, 8, 12] {
        for played in 1..=12 {
            for direction in 0..2 {
                let mut e = Engine::default();
                configure(
                    &mut e,
                    Config {
                        mode: AUTO,
                        strings: total,
                        strings_played: played,
                        arp_pattern: direction,
                        strum_ms: 0.0,
                        strum_sync: false,
                        ..Config::default()
                    },
                );
                on(&mut e, 63, 1);
                let layout = e.snapshot().notes;
                for sweep in 0..2 {
                    if sweep == 1 {
                        e.rebuild(&mut |_| {});
                    }
                    let reverse = direction == 1;
                    let expected = (0..played.min(total) as usize)
                        .map(|i| {
                            layout
                                .string(if reverse { total as usize - 1 - i } else { i })
                                .unwrap()
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(notes(&tick(&mut e, 1)), expected);
                    assert_eq!(e.snapshot().notes, layout);
                }
            }
        }
    }
}
#[test]
fn smart_voice_leading_nearest_off_and_reset() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            voice_leading: 0,
            ..Config::default()
        },
    );
    send(&mut e, Command::KeyDown(0, 60, 0));
    send(&mut e, Command::KeyUp(0));
    send(&mut e, Command::KeyDown(1, 67, 0));
    assert_eq!(e.full_notes.as_slice(), &[59, 62, 67]);
    let resolved = e.full_notes;
    configure(
        &mut e,
        Config {
            voice_leading: 0,
            filter: 1,
            ..Config::default()
        },
    );
    assert_eq!(e.full_notes, resolved);
    assert_eq!(e.notes.as_slice(), &[59]);
    configure(&mut e, Config::default());
    assert_eq!(e.full_notes.as_slice(), &[67, 71, 74]);
    configure(
        &mut e,
        Config {
            voice_leading: 0,
            ..Config::default()
        },
    );
    send(&mut e, Command::Panic);
    send(&mut e, Command::KeyDown(2, 60, 0));
    assert_eq!(e.full_notes.as_slice(), &[60, 64, 67]);
}
#[test]
fn smart_voice_leading_furthest_only_on_dominant_resolution() {
    let play = |mode, root, quality| {
        let mut e = Engine::default();
        configure(
            &mut e,
            Config {
                voice_leading: mode,
                ..Config::default()
            },
        );
        send(&mut e, Command::KeyDown(0, 67, 2));
        send(&mut e, Command::KeyUp(0));
        send(&mut e, Command::KeyDown(1, root, quality));
        e.full_notes
    };
    let nearest = play(0, 60, 0);
    let furthest = play(1, 60, 0);
    assert_ne!(nearest, furthest);
    assert_eq!(play(0, 62, 1), play(1, 62, 1));
    for n in furthest.as_slice() {
        assert!([0, 4, 7].contains(&(n % 12)));
    }
}
#[test]
fn smart_voice_leading_repeated_chord_is_stable() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            voice_leading: 1,
            ..Config::default()
        },
    );
    send(&mut e, Command::KeyDown(0, 67, 2));
    send(&mut e, Command::KeyUp(0));
    send(&mut e, Command::KeyDown(1, 60, 0));
    let resolved = e.full_notes;
    for _ in 0..10 {
        send(&mut e, Command::KeyUp(1));
        send(&mut e, Command::KeyDown(1, 60, 0));
        assert_eq!(e.full_notes, resolved);
    }
}
#[test]
fn learned_control_octave_consumes_following_notes_immediately() {
    let mut e = Engine::default();
    send(&mut e, Command::Learn(4));
    assert!(notes(&on(&mut e, 36, 0)).is_empty());
    for channel in 0..16 {
        let events = on(&mut e, 39, channel);
        assert!(
            notes(&events).is_empty(),
            "control key emitted musical notes"
        );
        assert!(e.root.is_none(), "control key became a performance root");
        assert_eq!(e.quality, 1);
        off(&mut e, 39, channel);
    }
}
#[test]
fn control_octave_never_triggers_playback_in_any_mode_or_channel() {
    for mode in [CHORD, AUTO, MANUAL, ARP] {
        for channel in 0..16 {
            let mut e = Engine::default();
            configure(
                &mut e,
                Config {
                    mode,
                    control_base: 36,
                    latch: true,
                    root_on_select: true,
                    strum_ms: 0.0,
                    strum_sync: false,
                    ..Config::default()
                },
            );
            on(&mut e, 60, 0);
            tick(&mut e, 1);
            off(&mut e, 60, 0);
            for note in 36..48 {
                let events = on(&mut e, note, channel);
                assert!(
                    notes(&events).is_empty(),
                    "control note {note}, mode {mode}, channel {channel}"
                );
                assert!(
                    e.scheduled.iter().all(Option::is_none),
                    "control scheduled playback, mode {mode}"
                );
                assert!(!e.down[channel as usize * 128 + note as usize]);
                assert!(off(&mut e, note, channel).is_empty());
                let mut events = Vec::new();
                e.midi_note(true, channel, note, 0.0, &mut |v| events.push(v));
                assert!(events.is_empty());
            }
            assert_eq!(e.memory.unwrap().root, 60);
        }
    }
}
#[test]
fn voice_leading_snapshot_records_each_transition_once() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: MANUAL,
            voice_leading: 0,
            ..Config::default()
        },
    );
    on(&mut e, 60, 0);
    let previous = e.full_notes;
    assert_eq!(e.snapshot().leading_serial, 0);
    off(&mut e, 60, 0);
    on(&mut e, 67, 0);
    let snapshot = e.snapshot();
    assert_eq!(snapshot.leading_from, previous);
    assert_eq!(snapshot.leading_to, e.full_notes);
    assert_eq!(snapshot.leading_serial, 1);
    e.rebuild(&mut |_| {});
    assert_eq!(e.snapshot().leading_serial, 1);
    off(&mut e, 67, 0);
    on(&mut e, 60, 0);
    assert_eq!(e.snapshot().leading_serial, 2);
    e.panic(&mut |_| {});
    assert_eq!(e.snapshot().leading_from.len, 0);
    assert_eq!(e.snapshot().leading_to.len, 0);
}
#[test]
fn control_octave_per_note_pressure_is_consumed() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            control_base: 36,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    let mut events = Vec::new();
    e.pressure(1, Some(39), 0.9, &mut |v| events.push(v));
    assert!(events.is_empty());
    assert_eq!(e.sources[4], 0.0);
}
