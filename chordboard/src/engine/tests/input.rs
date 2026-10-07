use super::super::*;
use super::support::*;

#[test]
fn first_two_win_and_ignored_third_requires_repress() {
    let mut e = Engine::default();
    on(&mut e, 60, 1);
    on(&mut e, 66, 2);
    on(&mut e, 69, 3);
    assert_eq!(e.notes.as_slice(), &[60, 64, 66]);
    off(&mut e, 66, 2);
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);
    on(&mut e, 69, 3);
    assert!(e.second.is_none());
    off(&mut e, 69, 3);
    on(&mut e, 69, 3);
    assert_eq!(e.second.map(|s| s.note), Some(69));
}
#[test]
fn last_held_second_becomes_root() {
    let mut e = Engine::default();
    on(&mut e, 60, 1);
    on(&mut e, 66, 2);
    off(&mut e, 60, 1);
    assert_eq!(e.root.map(|s| s.note), Some(66));
    assert_eq!(e.notes.as_slice(), &[66, 70, 73]);
    off(&mut e, 66, 2);
    assert!(e.root.is_none());
    assert!(e.voices.iter().all(Option::is_none));
}
#[test]
fn control_octave_is_consumed_and_changes_live_quality() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            control_base: 36,
            ..Config::default()
        },
    );
    assert!(notes(&on(&mut e, 39, 1)).is_empty());
    assert!(e.root.is_none());
    on(&mut e, 60, 2);
    assert_eq!(e.notes.as_slice(), &[60, 63, 67]);
    on(&mut e, 36, 3);
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);
    assert!(e.second.is_none());
}
#[test]
fn common_notes_are_not_retriggered() {
    let mut e = Engine::default();
    on(&mut e, 60, 1);
    let events = on(&mut e, 62, 2);
    assert_eq!(notes(&events), vec![62]);
    assert!(!events.iter().any(|e| matches!(e, Out::Off(..))));
    let tail = e.voices.iter().flatten().find(|v| v.note == 64).unwrap();
    assert_eq!(tail.off, e.now + e.duration());
}
#[test]
fn release_velocity_and_zero_velocity_note_on() {
    let mut e = Engine::default();
    on(&mut e, 60, 1);
    let events = off(&mut e, 60, 1);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e,Out::Off(_,_,v) if *v==0.3))
            .count(),
        3
    );
    on(&mut e, 60, 1);
    e.midi_note(true, 1, 60, 0.0, &mut |_| {});
    assert!(e.root.is_none());
}
#[test]
fn inversion_arrows_cycle_both_directions() {
    let mut e = Engine::default();
    on(&mut e, 60, 1);
    send(&mut e, Command::Inversion(-1));
    assert_eq!(e.notes.as_slice(), &[67, 72, 76]);
    send(&mut e, Command::Inversion(1));
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);
    for _ in 0..30 {
        send(&mut e, Command::Inversion(1));
    }
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);
}
#[test]
fn cardinality_change_normalizes_inversion() {
    let mut e = Engine::default();
    on(&mut e, 60, 1);
    on(&mut e, 70, 2);
    send(&mut e, Command::Inversion(-1));
    assert_eq!(e.inversion, 3);
    off(&mut e, 70, 2);
    assert_eq!(e.inversion, 0);
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);
}
#[test]
fn manual_sweeps_cross_every_string_and_do_not_repeat_at_rest() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: MANUAL,
            ..Config::default()
        },
    );
    assert!(notes(&on(&mut e, 60, 1)).is_empty());
    let first = send(&mut e, Command::BeginGesture(0.0, 0.8));
    assert_eq!(notes(&first), vec![60]);
    let up = send(&mut e, Command::X(1.0));
    assert_eq!(notes(&up), vec![64, 67, 72, 76, 79, 84, 88]);
    assert!(notes(&send(&mut e, Command::X(1.0))).is_empty());
    let down = send(&mut e, Command::X(0.0));
    assert_eq!(notes(&down), vec![84, 79, 76, 72, 67, 64, 60]);
}
#[test]
fn modwheel_is_premapped_only_manual_and_does_not_change_mode() {
    let mut e = Engine::default();
    on(&mut e, 60, 1);
    let mut out = Vec::new();
    e.control(0, 1, 1.0, &mut |v| out.push(v));
    assert!(notes(&out).is_empty());
    assert_eq!(e.config.mode, CHORD);
    configure(
        &mut e,
        Config {
            mode: MANUAL,
            ..Config::default()
        },
    );
    e.control(0, 1, 0.0, &mut |_| {});
    e.control(0, 1, 1.0, &mut |v| out.push(v));
    assert_eq!(notes(&out).len(), 7);
}
#[test]
fn jitter_does_not_create_phantom_strikes() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: MANUAL,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    send(&mut e, Command::X(0.0));
    send(&mut e, Command::X(0.08));
    for x in [0.07, 0.075, 0.08, 0.073] {
        assert!(notes(&send(&mut e, Command::X(x))).is_empty());
    }
}
#[test]
fn auto_strum_uses_sample_clock_and_releases_notes() {
    let mut e = Engine::default();
    e.sample_rate = 1000.0;
    configure(
        &mut e,
        Config {
            mode: AUTO,
            strings: 3,
            strum_ms: 50.0,
            strum_sync: false,
            length_ms: 20.0,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    assert_eq!(notes(&tick(&mut e, 1)), vec![60]);
    assert!(notes(&tick(&mut e, 49)).is_empty());
    assert_eq!(notes(&tick(&mut e, 1)), vec![64]);
    assert_eq!(notes(&tick(&mut e, 50)), vec![67]);
    tick(&mut e, 21);
    assert!(e.voices.iter().all(Option::is_none));
}
#[test]
fn mpe_initializes_every_channel_before_note_on() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mpe: true,
            ..Config::default()
        },
    );
    e.pressure(1, None, 0.7, &mut |_| {});
    e.control(1, 74, 0.6, &mut |_| {});
    e.bend(1, 0.55, &mut |_| {});
    let events = on(&mut e, 60, 1);
    for (i, event) in events.iter().enumerate() {
        if let Out::On(ch, _, _) = event {
            assert!(events[..i].contains(&Out::Pressure(*ch, 0.7)));
            assert!(events[..i].contains(&Out::Cc(*ch, 74, 0.6)));
            assert!(events[..i].contains(&Out::Bend(*ch, 0.55)));
        }
    }
    assert_eq!(notes(&events).len(), 3);
}
#[test]
fn only_first_note_expression_controls_all_voices() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mpe: true,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    on(&mut e, 62, 2);
    let mut out = Vec::new();
    e.pressure(2, None, 0.9, &mut |v| out.push(v));
    assert!(out.is_empty());
    e.pressure(1, None, 0.8, &mut |v| out.push(v));
    assert_eq!(
        out.iter()
            .filter(|e| matches!(e,Out::Pressure(_,p) if *p==0.8))
            .count(),
        4
    );
}
#[test]
fn promoted_root_owns_expression_instead_of_reused_channel() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mpe: true,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    on(&mut e, 62, 2);
    e.pressure(1, None, 0.6, &mut |_| {});
    off(&mut e, 60, 1);
    on(&mut e, 70, 1);
    e.pressure(1, None, 0.9, &mut |_| {});
    assert_eq!(e.root.map(|s| s.note), Some(62));
    assert_eq!(e.expression.pressure, 0.0);
}
#[test]
fn delayed_strum_inherits_latest_expression() {
    let mut e = Engine::default();
    e.sample_rate = 1000.0;
    configure(
        &mut e,
        Config {
            mpe: true,
            mode: AUTO,
            strings: 3,
            strum_ms: 50.0,
            strum_sync: false,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    tick(&mut e, 1);
    e.pressure(1, None, 0.85, &mut |_| {});
    let events = tick(&mut e, 50);
    let channel = events
        .iter()
        .find_map(|e| {
            if let Out::On(ch, 64, _) = e {
                Some(*ch)
            } else {
                None
            }
        })
        .unwrap();
    assert!(events.contains(&Out::Pressure(channel, 0.85)));
}
#[test]
fn master_bend_is_not_fanned_twice() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mpe: true,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    let mut out = Vec::new();
    e.bend(0, 0.75, &mut |v| out.push(v));
    assert_eq!(out, vec![Out::Bend(0, 0.75)]);
    assert_eq!(e.expression.bend, 0.5);
}
#[test]
fn member_bend_cannot_be_learned_as_fader() {
    let mut e = Engine::default();
    send(&mut e, Command::Learn(1));
    e.bend(2, 0.7, &mut |_| {});
    assert!(e.learned.is_none());
    assert_eq!(e.learn, 1);
    e.bend(0, 0.7, &mut |_| {});
    assert!(e.learned.is_some());
}
#[test]
fn rpn_updates_input_bend_and_zone() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mpe: true,
            ..Config::default()
        },
    );
    for (cc, value) in [(101, 0), (100, 0), (6, 12)] {
        e.control(1, cc, value as f32 / 127.0, &mut |_| {});
    }
    on(&mut e, 60, 1);
    e.bend(1, 1.0, &mut |_| {});
    assert_eq!(e.expression.bend, 0.625);
    for (cc, value) in [(101, 0), (100, 6), (6, 7)] {
        e.control(15, cc, value as f32 / 127.0, &mut |_| {});
    }
    assert!(e.input_upper);
    assert_eq!(e.input_members, 7);
}
#[test]
fn paired_cc_and_calibration_work() {
    let mut e = Engine::default();
    let mut c = Config::default();
    c.mappings[0] = Mapping {
        kind: 2,
        number: 1,
        channel: 0,
    };
    configure(&mut e, c);
    e.control(0, 1, 64.0 / 127.0, &mut |_| {});
    e.control(0, 33, 1.0 / 127.0, &mut |_| {});
    assert!((e.x - 8193.0 / 16383.0).abs() < 0.0001);
    c.x_min = 0.25;
    c.x_max = 0.75;
    c.x_reverse = true;
    configure(&mut e, c);
    send(&mut e, Command::X(0.25));
    assert_eq!(e.x, 1.0);
}
#[test]
fn latch_and_sustain_free_input_slots() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            latch: true,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    off(&mut e, 60, 1);
    assert!(e.root.is_none());
    assert_eq!(e.notes.len, 3);
    on(&mut e, 65, 1);
    assert_eq!(e.root.map(|s| s.note), Some(65));
    configure(&mut e, Config::default());
    e.control(0, 64, 1.0, &mut |_| {});
    off(&mut e, 65, 1);
    assert_eq!(e.notes.len, 3);
    e.control(0, 64, 0.0, &mut |_| {});
    assert_eq!(e.notes.len, 0);
}
#[test]
fn held_chord_keeps_alteration_in_either_release_order() {
    for mode in [CHORD, AUTO, MANUAL, ARP] {
        for root_first in [false, true] {
            for gap in [0, 1, 20] {
                let mut e = Engine {
                    sample_rate: 1000.0,
                    ..Engine::default()
                };
                configure(
                    &mut e,
                    Config {
                        latch: true,
                        mode,
                        ..Config::default()
                    },
                );
                on(&mut e, 60, 1);
                on(&mut e, 62, 2);
                let releases = if root_first {
                    [(60, 1), (62, 2)]
                } else {
                    [(62, 2), (60, 1)]
                };
                for (note, channel) in releases {
                    assert!(
                        off(&mut e, note, channel).is_empty(),
                        "release changed playback: mode={mode}, root_first={root_first}, gap={gap}"
                    );
                    assert_eq!(e.notes.as_slice(), &[60, 62, 67]);
                    tick(&mut e, gap);
                }
                assert!(e.root.is_none() && e.second.is_none());
                assert_eq!(e.memory.unwrap().second, Some(62));
                on(&mut e, 60, 1);
                assert_eq!(e.notes.as_slice(), &[60, 64, 67]);
                assert_eq!(e.memory.unwrap().second, None);
            }
        }
    }
}
#[test]
fn held_chord_keeps_released_alteration_through_revoicing_and_replaces_it_on_press() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            latch: true,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    on(&mut e, 62, 2);
    off(&mut e, 62, 2);
    send(&mut e, Command::SetInversion(1));
    assert_eq!(e.notes.as_slice(), &[62, 67, 72]);
    on(&mut e, 65, 2);
    assert_eq!(e.notes.as_slice(), &[65, 67, 72]);
    assert_eq!(e.memory.unwrap().second, Some(65));
}
#[test]
fn held_chord_root_repress_resets_while_alteration_is_still_down() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            latch: true,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    on(&mut e, 62, 2);
    off(&mut e, 60, 1);
    on(&mut e, 60, 1);
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);
    assert!(off(&mut e, 62, 2).is_empty());
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);
}
#[test]
fn held_chord_ignored_note_release_never_changes_the_harmony() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            latch: true,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    on(&mut e, 62, 2);
    on(&mut e, 65, 3);
    for (note, channel) in [(60, 1), (62, 2), (65, 3)] {
        assert!(off(&mut e, note, channel).is_empty());
        assert_eq!(e.notes.as_slice(), &[60, 62, 67]);
    }
}
#[test]
fn held_chord_disabling_hold_returns_to_the_physically_held_input() {
    for root_first in [false, true] {
        let mut e = Engine::default();
        configure(
            &mut e,
            Config {
                latch: true,
                ..Config::default()
            },
        );
        on(&mut e, 60, 1);
        on(&mut e, 62, 2);
        if root_first {
            off(&mut e, 60, 1);
        } else {
            off(&mut e, 62, 2);
        }
        configure(&mut e, Config::default());
        let (remaining, channel, expected) = if root_first {
            (62, 2, [62, 66, 69])
        } else {
            (60, 1, [60, 64, 67])
        };
        assert_eq!(e.notes.as_slice(), &expected);
        off(&mut e, remaining, channel);
        assert!(e.voices.iter().all(Option::is_none));
    }
}
#[test]
fn held_chord_keyboard_releases_keep_sus2_until_root_is_repressed() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            latch: true,
            ..Config::default()
        },
    );
    send(&mut e, Command::KeyDown(0, 60, 0));
    send(&mut e, Command::KeyDown(1, 62, 0));
    assert!(send(&mut e, Command::KeyUp(1)).is_empty());
    assert!(send(&mut e, Command::KeyUp(0)).is_empty());
    assert_eq!(e.notes.as_slice(), &[60, 62, 67]);
    send(&mut e, Command::KeyDown(0, 60, 0));
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);
}
