use super::super::*;
use super::support::*;

#[test]
fn first_two_retain_roles_and_later_inputs_add_without_becoming_second() {
    let mut e = Engine::default();
    on(&mut e, 60, 1);
    on(&mut e, 66, 2);
    on(&mut e, 69, 3);
    assert_eq!(e.notes.as_slice(), &[60, 64, 66, 81]);
    off(&mut e, 66, 2);
    assert_eq!(e.notes.as_slice(), &[60, 64, 67, 81]);
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
fn latch_repress_rearticulates_chords_from_midi_keyboard_and_pointer() {
    for token in [None, Some(0), Some(POINTER_KEY_OFFSET)] {
        let mut e = Engine::default();
        configure(&mut e, Config { latch: true, ..Config::default() });
        let press = |e: &mut Engine| match token {
            Some(token) => send(e, Command::KeyDown(token, 60, 0)),
            None => on(e, 60, 1),
        };
        assert_eq!(notes(&press(&mut e)), vec![60, 64, 67]);
        for _ in 0..3 {
            assert!(press(&mut e).is_empty(), "held key repeats must be ignored");
            let released = match token {
                Some(token) => send(&mut e, Command::KeyUp(token)),
                None => off(&mut e, 60, 1),
            };
            assert!(released.is_empty());
            let events = press(&mut e);
            assert_eq!(notes(&events), vec![60, 64, 67], "input {token:?}");
            for note in [60, 64, 67] {
                let attack = events.iter().position(|v| matches!(v, Out::On(_, n, _) if *n == note)).unwrap();
                assert!(events[..attack].iter().any(|v| matches!(v, Out::Off(_, n, _) if *n == note)));
            }
            assert_eq!(e.voices.iter().flatten().count(), 3);
        }
    }
}

#[test]
fn latch_repress_rearticulates_alterations_and_added_notes() {
    let mut e = Engine::default();
    configure(&mut e, Config { latch: true, ..Config::default() });
    on(&mut e, 60, 1);
    on(&mut e, 62, 2);
    assert!(off(&mut e, 62, 2).is_empty());
    assert_eq!(notes(&on(&mut e, 62, 2)), vec![60, 62, 67]);
    on(&mut e, 65, 3);
    assert!(off(&mut e, 65, 3).is_empty());
    assert_eq!(notes(&on(&mut e, 65, 3)), vec![60, 62, 67, 77]);
}

#[test]
fn latch_repress_retriggers_selection_root_without_strumming() {
    let mut e = Engine::default();
    configure(&mut e, Config { latch: true, mode: MANUAL, root_on_select: true, ..Config::default() });
    assert_eq!(notes(&on(&mut e, 60, 1)), vec![60]);
    assert!(off(&mut e, 60, 1).is_empty());
    assert_eq!(notes(&on(&mut e, 60, 1)), vec![60]);
    assert_eq!(e.voices.iter().flatten().count(), 1);
}

#[test]
fn latch_configuration_and_control_selection_do_not_retrigger_chords() {
    let mut e = Engine::default();
    let config = Config { latch: true, control_base: 36, ..Config::default() };
    configure(&mut e, config);
    on(&mut e, 60, 1);
    off(&mut e, 60, 1);
    let mut events = Vec::new();
    e.configure(config, &mut |v| events.push(v));
    assert!(events.is_empty());
    assert!(on(&mut e, 36, 1).is_empty());
    assert!(notes(&send(&mut e, Command::SetQuality(0))).is_empty());
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
            octaves: 3,
            ..Config::default()
        },
    );
    assert!(notes(&on(&mut e, 60, 1)).is_empty());
    let first = send(&mut e, Command::BeginGesture(0.0, 0.8));
    assert_eq!(notes(&first), vec![60]);
    let up = send(&mut e, Command::X(1.0));
    assert_eq!(notes(&up), vec![64, 67, 72, 76, 79, 84, 88, 91]);
    assert!(notes(&send(&mut e, Command::X(1.0))).is_empty());
    let down = send(&mut e, Command::X(0.0));
    assert_eq!(notes(&down), vec![88, 84, 79, 76, 72, 67, 64, 60]);
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
            octaves: 3,
            ..Config::default()
        },
    );
    e.control(0, 1, 0.0, &mut |_| {});
    e.control(0, 1, 1.0, &mut |v| out.push(v));
    assert_eq!(notes(&out).len(), 8);
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
fn held_chord_addition_release_keeps_latched_recipe() {
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
        assert_eq!(e.notes.as_slice(), &[60, 62, 67, 77]);
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

#[test]
fn flat_ninth_replaces_sus2_and_release_restores_it() {
    for keyboard in [false, true] {
        let mut e = Engine::default();
        if keyboard {
            send(&mut e, Command::KeyDown(0, 60, 0));
            send(&mut e, Command::KeyDown(1, 62, 0));
            send(&mut e, Command::KeyDown(2, 61, 0));
        } else {
            on(&mut e, 60, 1);
            on(&mut e, 62, 2);
            on(&mut e, 61, 3);
        }
        assert_eq!(e.notes.as_slice(), &[60, 67, 73]);
        assert_eq!(harmony::chord_name(e.memory.unwrap()), "C5(addb9)");
        send(&mut e, Command::Capture(0));
        let word = e.saved.unwrap().1;
        if keyboard {
            send(&mut e, Command::KeyUp(2));
        } else {
            off(&mut e, 61, 3);
        }
        assert_eq!(e.notes.as_slice(), &[60, 62, 67]);
        assert_eq!(harmony::chord_name(e.memory.unwrap()), "Csus2");
        send(&mut e, Command::Recall(word));
        assert_eq!(e.notes.as_slice(), &[60, 67, 73]);
        assert_eq!(harmony::chord_name(e.memory.unwrap()), "C5(addb9)");
    }
}

#[test]
fn addition_identity_dedup_release_and_capture_preserve_recipe_and_clock() {
    let mut e = Engine::default();
    configure(&mut e, Config { mode: ARP, ..Config::default() });
    on(&mut e, 60, 1); on(&mut e, 70, 2);
    tick(&mut e, 15);
    let clock = (e.arp.next, e.arp.phase, e.arp.step);
    on(&mut e, 61, 3); on(&mut e, 74, 4); on(&mut e, 62, 5);
    assert_eq!(e.notes.as_slice(), &[60,64,67,70,73,74]);
    assert_eq!((e.arp.next,e.arp.phase,e.arp.step), clock);
    off(&mut e, 74, 4);
    assert_eq!(e.notes.as_slice(), &[60,64,67,70,73,74]);
    let word = e.memory.unwrap().encode();
    send(&mut e, Command::Capture(0));
    assert_eq!(e.saved.unwrap().1, word);
    off(&mut e, 62, 5);
    assert_eq!(e.notes.as_slice(), &[60,64,67,70,73]);
    assert_eq!((e.arp.next,e.arp.phase,e.arp.step), clock);
    send(&mut e, Command::Recall(word));
    assert_eq!(e.notes.as_slice(), &[60,64,67,70,73,74]);
}

#[test]
fn additions_preserve_root_velocity_and_member_expression() {
    let mut e = Engine::default();
    configure(&mut e, Config { mpe:true, ..Config::default() });
    on(&mut e,60,1); on(&mut e,70,2);
    e.pressure(3,None,0.6,&mut |_|{});
    let mut events = Vec::new();
    e.midi_note(true,3,62,0.4,&mut |v|events.push(v));
    let (channel,velocity) = events.iter().find_map(|v| match v { Out::On(c,74,v) => Some((*c,*v)), _ => None }).unwrap();
    assert!((velocity-0.8).abs()<0.0001);
    assert!(events.contains(&Out::Pressure(channel,0.0)));
    events.clear(); e.pressure(3,Some(62),0.9,&mut |v|events.push(v));
    assert!(events.is_empty());
    assert_eq!(e.expression.pressure,0.0);
    events.clear(); e.pressure(1,None,0.7,&mut |v|events.push(v));
    assert!(events.contains(&Out::Pressure(channel,0.7)));
}

#[test]
fn latch_keeps_additions_and_resets_retained_recipe_without_admitting_old_second() {
    let mut e = Engine::default();
    configure(&mut e,Config { latch:true, ..Config::default() });
    on(&mut e,60,1); on(&mut e,62,2); on(&mut e,65,3);
    off(&mut e,65,3); off(&mut e,60,1); off(&mut e,62,2);
    assert_eq!(e.memory.unwrap().additions,1<<5);
    send(&mut e,Command::SetQuality(1));
    assert_eq!(e.memory.unwrap().additions,0);
    on(&mut e,60,1); on(&mut e,62,2); on(&mut e,65,3);
    off(&mut e,60,1); on(&mut e,60,1);
    assert!(e.second.is_none());
    assert_eq!(e.memory.unwrap().additions,1<<5);
    assert_eq!(e.notes.as_slice(), &[60,63,67,77]);
}

#[test]
fn several_additions_keep_root_recipe_until_sole_survivor_promotes_in_either_release_order() {
    for root_first in [false,true] {
        let mut e=Engine::default();
        on(&mut e,60,1); on(&mut e,62,2); on(&mut e,63,3); on(&mut e,65,4);
        let order=if root_first {[(60,1),(62,2)]} else {[(62,2),(60,1)]};
        for (note,channel) in order { off(&mut e,note,channel); }
        assert_eq!(e.root.map(|s|s.note),Some(60));
        assert!(e.second.is_none());
        assert_eq!(e.notes.as_slice(),&[60,64,67,75,77]);
        off(&mut e,63,3);
        assert_eq!(e.root.map(|s|s.note),Some(65));
        assert_eq!(e.notes.as_slice(),&[65,69,72]);
        off(&mut e,65,4);
        assert!(e.root.is_none());
        assert!(e.voices.iter().all(Option::is_none));
    }
}

#[test]
fn split_releases_additions_without_latch_and_latch_disable_keeps_held_contributors() {
    let mut e=Engine::default();
    configure(&mut e,Config { key_split:true, split_note:100, bass_enabled:false, ..Config::default() });
    on(&mut e,60,1); on(&mut e,70,2); on(&mut e,62,3);
    assert_eq!(e.additions,1<<2);
    off(&mut e,62,3);
    assert_eq!(e.additions,0);
    assert_eq!(e.notes.as_slice(),&[60,64,67,70]);
    let mut e=Engine::default();
    configure(&mut e,Config { latch:true, ..Config::default() });
    on(&mut e,60,1); on(&mut e,62,2); on(&mut e,63,3); on(&mut e,65,4);
    off(&mut e,60,1); off(&mut e,62,2);
    configure(&mut e,Config::default());
    assert_eq!(e.notes.as_slice(),&[60,64,67,75,77]);
    off(&mut e,63,3);
    assert_eq!(e.root.map(|s|s.note),Some(65));
    assert_eq!(e.notes.as_slice(),&[65,69,72]);
}

#[test]
fn addition_release_remaps_pending_strum_register_without_restarting_due_time() {
    let mut e=Engine::default();
    configure(&mut e,Config { mode:AUTO, ..Config::default() });
    on(&mut e,36,1); on(&mut e,42,2);
    for (n,c) in [(43,3),(46,4),(47,5),(50,6)] { on(&mut e,n,c); }
    send(&mut e,Command::SetInversion(4));
    e.scheduled.fill(None);
    let previous = *e.notes.as_slice().iter().find(|n| **n % 12 == 10).unwrap();
    e.scheduled[0]=Some(Scheduled { at:100, note:previous, velocity:0.8, duration:100, gate_step:None });
    off(&mut e,43,3);
    let current = *e.notes.as_slice().iter().find(|n| **n % 12 == 10).unwrap();
    assert_ne!(previous,current);
    assert_eq!(e.scheduled[0].map(|s|(s.note,s.at)),Some((current,100)));
}

#[test]
fn manual_chord_on_select_sounds_added_tone_and_release_preserves_timed_tail() {
    let mut e=Engine::default();
    configure(&mut e,Config { mode:MANUAL, always_chord:true, ..Config::default() });
    on(&mut e,60,1); on(&mut e,70,2);
    assert!(notes(&on(&mut e,62,3)).contains(&74));
    assert!(e.voices.iter().flatten().any(|v|v.note==74&&v.layer));
    e.strike(74,0.8,100,&mut |_|{});
    assert!(off(&mut e,62,3).iter().all(|v|!matches!(v,Out::Off(_,74,_))));
    assert!(e.voices.iter().flatten().any(|v|v.note==74&&!v.layer&&v.off==e.now+100));
    tick(&mut e,101);
    assert!(!e.voices.iter().flatten().any(|v|v.note==74));
}

#[test]
fn midi_degrees_follow_actual_scale_roots_in_every_key_and_register() {
    for key in 0..12 {
        for scale in 0..8 {
            let mut e = Engine::default();
            e.configure(Config { key, scale, scale_layout: 1, bass_enabled: false, ..Config::default() }, &mut |_| {});
            for octave in [12, 60, 108] {
                for &relative in harmony::scale_tones(scale) {
                    let note = octave + key + relative;
                    if note > 127 { continue; }
                    e.midi_note(true, 0, note, 0.7, &mut |_| {});
                    assert_eq!(e.root.unwrap().note, note);
                    assert_eq!(e.quality, harmony::scale_quality(scale, relative).unwrap(), "key {key}, scale {scale}, note {note}");
                    e.midi_note(false, 0, note, 0.0, &mut |_| {});
                }
            }
        }
    }
}

#[test]
fn midi_degrees_preserve_chromatic_mode_and_out_of_scale_roots() {
    for (layout, note) in [(0, 62), (1, 61)] {
        let mut e = Engine::default();
        e.configure(Config { scale_layout: layout, quality: 6, bass_enabled: false, ..Config::default() }, &mut |_| {});
        e.midi_note(true, 0, note, 0.7, &mut |_| {});
        assert_eq!(e.quality, 6);
        e.midi_note(false, 0, note, 0.0, &mut |_| {});
        assert!(e.root.is_none());
    }
}

#[test]
fn midi_keyboard_feedback_includes_control_bass_melody_and_all_channels() {
    let mut e = Engine::default();
    e.configure(Config { control_base: 36, bass_split: 36, key_split: true, split_note: 60, ..Config::default() }, &mut |_| {});
    for note in [24, 36, 50, 72] {
        e.midi_note(true, 0, note, 0.7, &mut |_| {});
        e.midi_note(true, 2, note, 0.7, &mut |_| {});
        assert!(e.snapshot().held_notes[note as usize], "note {note}");
        e.midi_note(false, 0, note, 0.0, &mut |_| {});
        assert!(e.snapshot().held_notes[note as usize]);
        e.midi_note(true, 2, note, 0.0, &mut |_| {});
        assert!(!e.snapshot().held_notes[note as usize]);
    }
    e.midi_note(true, 0, 36, 0.7, &mut |_| {});
    e.panic(&mut |_| {});
    assert!(!e.snapshot().held_notes.iter().any(|&n| n));
}
