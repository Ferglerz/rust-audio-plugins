use super::*;
fn send(e: &mut Engine, c: Command) -> Vec<Out> {
    let mut out = Vec::new();
    e.command(c, &mut |v| out.push(v));
    out
}
fn on(e: &mut Engine, n: u8, ch: u8) -> Vec<Out> {
    let mut out = Vec::new();
    e.midi_note(true, ch, n, 0.8, &mut |v| out.push(v));
    out
}
fn off(e: &mut Engine, n: u8, ch: u8) -> Vec<Out> {
    let mut out = Vec::new();
    e.midi_note(false, ch, n, 0.3, &mut |v| out.push(v));
    out
}
fn configure(e: &mut Engine, c: Config) {
    e.configure(c, &mut |_| {});
}
fn tick(e: &mut Engine, count: usize) -> Vec<Out> {
    let mut out = Vec::new();
    for _ in 0..count {
        e.tick(&mut |v| out.push(v));
    }
    out
}
fn notes(events: &[Out]) -> Vec<u8> {
    events
        .iter()
        .filter_map(|e| {
            if let Out::On(_, n, _) = e {
                Some(*n)
            } else {
                None
            }
        })
        .collect()
}
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
#[test]
fn transport_and_panic_cancel_pending_notes() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: AUTO,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    e.transport(true, 120.0, false, &mut |_| {});
    e.transport(false, 120.0, false, &mut |_| {});
    assert!(notes(&tick(&mut e, 50000)).is_empty());
    assert!(e.voices.iter().all(Option::is_none));
}
#[test]
fn arp_is_deterministic_and_gate_ends_notes() {
    let mut a = Engine::default();
    let mut b = Engine::default();
    for e in [&mut a, &mut b] {
        e.sample_rate = 1000.0;
        configure(
            e,
            Config {
                mode: ARP,
                arp_pattern: 4,
                humanize: 0.5,
                ..Config::default()
            },
        );
        on(e, 60, 1);
    }
    assert_eq!(tick(&mut a, 1000), tick(&mut b, 1000));
    off(&mut a, 60, 1);
    assert!(a.voices.iter().all(Option::is_none));
}
#[test]
fn channel_capacity_steals_cleanly() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mpe: true,
            members: 2,
            ..Config::default()
        },
    );
    let events = on(&mut e, 60, 1);
    assert_eq!(e.voices.iter().flatten().count(), 2);
    assert!(events.iter().any(|e| matches!(e, Out::Off(_, 60, _))));
}
#[test]
fn memory_captures_and_recalls_full_recipe() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            spread: 1,
            transpose: 12,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    on(&mut e, 66, 2);
    send(&mut e, Command::Inversion(1));
    let expected = e.notes;
    send(&mut e, Command::Capture(2));
    let (slot, word) = e.saved.take().unwrap();
    assert_eq!(slot, 2);
    for index in [8, 9] {
        send(&mut e, Command::Capture(index));
        assert_eq!(e.saved.take(), Some((index, word)));
    }
    send(&mut e, Command::Panic);
    send(&mut e, Command::Recall(word));
    assert_eq!(e.notes, expected);
    e.configure(e.host_config, &mut |_| {});
    assert_eq!(e.notes, expected);
    on(&mut e, 65, 1);
    assert_eq!(e.root.map(|s| s.note), Some(65));
    assert!(e.second.is_none());
}
#[test]
fn filters_apply_after_voicing_and_preserve_source_chord() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            filter: 3,
            inversion: 1,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    assert_eq!(e.full_notes.as_slice(), &[64, 67, 72]);
    assert_eq!(e.notes.as_slice(), &[64, 72]);
}
#[test]
fn chord_channels_follow_output_or_mpe_allocation() {
    for mpe in [false, true] {
        let mut e = Engine::default();
        configure(
            &mut e,
            Config {
                mpe,
                bass_channel: 5,
                upper_channel: 6,
                ..Config::default()
            },
        );
        let events = on(&mut e, 60, 1);
        let channels: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let Out::On(ch, _, _) = e {
                    Some(*ch)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(channels, if mpe { vec![1, 2, 3] } else { vec![0, 0, 0] });
    }
}
#[test]
fn scale_highlighting_does_not_block_outside_chords() {
    assert!(harmony::in_key(0, 0, 0, 0));
    assert!(!harmony::in_key(1, 0, 0, 0));
    let mut e = Engine::default();
    assert_eq!(notes(&on(&mut e, 61, 1)), vec![61, 65, 68]);
    assert_eq!(harmony::roman(7, 2, 0), "V7");
}

#[test]
fn keyboard_focus_loss_does_not_clear_midi_latch() {
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
    send(&mut e, Command::ReleaseKeyboard);
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);
    send(&mut e, Command::KeyDown(0, 65, 0));
    send(&mut e, Command::ReleaseKeyboard);
    assert!(e.notes.as_slice().is_empty());
}
#[test]
fn pointer_and_physical_key_have_independent_ownership() {
    let mut e = Engine::default();
    send(&mut e, Command::KeyDown(0, 60, 0));
    send(&mut e, Command::KeyDown(POINTER_KEY_OFFSET, 60, 0));
    send(&mut e, Command::KeyUp(POINTER_KEY_OFFSET));
    assert!(e.root.is_some_and(|s| s.held));
    assert!(e.voices.iter().any(Option::is_some));
    send(&mut e, Command::KeyUp(0));
    assert!(e.voices.iter().all(Option::is_none));
}
#[test]
fn standard_midi_bend_keeps_master_range() {
    let mut e = Engine::default();
    on(&mut e, 60, 0);
    e.bend(0, 0.75, &mut |_| {});
    assert_eq!(e.expression.bend, 0.75);
}

#[test]
fn extended_keyboard_tracks_last_key_and_releases_every_pointer_token() {
    let mut e = Engine::default();
    let last = harmony::KEY_COUNT as u8 - 1;
    send(&mut e, Command::KeyDown(last, 71, 2));
    assert_eq!(e.snapshot().accepted, 1_u64 << last);
    send(&mut e, Command::KeyDown(last + POINTER_KEY_OFFSET, 71, 2));
    send(&mut e, Command::KeyUp(last + POINTER_KEY_OFFSET));
    assert!(e.root.is_some_and(|s| s.held));
    send(&mut e, Command::ReleaseKeyboard);
    assert_eq!(e.snapshot().accepted, 0);
    assert!(e.down[2048..].iter().all(|&down| !down));
    assert!(e.voices.iter().all(Option::is_none));
    send(&mut e, Command::KeyDown(last + POINTER_KEY_OFFSET, 71, 2));
    assert_eq!(e.snapshot().accepted, 1_u64 << last);
    send(&mut e, Command::ReleaseKeyboard);
    assert!(e.down[2048..].iter().all(|&down| !down));
}

#[test]
fn extended_ignored_keys_have_distinct_snapshot_bits() {
    let mut e = Engine::default();
    send(&mut e, Command::KeyDown(0, 60, 0));
    send(&mut e, Command::KeyDown(1, 61, 0));
    send(&mut e, Command::KeyDown(35, 71, 2));
    assert_eq!(e.snapshot().ignored, 1_u64 << 35);
    send(&mut e, Command::ReleaseKeyboard);
    assert_eq!(e.snapshot().ignored, 0);
}

#[test]
fn signed_swing_reverses_step_durations_without_changing_pair_length() {
    for (swing, expected) in [
        (-0.5, [0, 125, 500]),
        (0.0, [0, 250, 500]),
        (0.5, [0, 375, 500]),
    ] {
        let mut e = Engine {
            sample_rate: 1000.0,
            ..Engine::default()
        };
        configure(
            &mut e,
            Config {
                mode: 3,
                rate: 0.5,
                swing,
                humanize: 0.0,
                ..Config::default()
            },
        );
        on(&mut e, 60, 0);
        let mut strikes = Vec::new();
        for sample in 0..=500 {
            e.tick(&mut |event| {
                if matches!(event, Out::On(_, _, _)) {
                    strikes.push(sample);
                }
            });
        }
        assert_eq!(strikes, expected, "swing {swing}");
    }
}

#[test]
fn once_rate_uses_beats_or_milliseconds_per_note() {
    for (tempo, sync, expected) in [(120.0, true, 250), (60.0, true, 500), (60.0, false, 120)] {
        let mut e = Engine {
            sample_rate: 1000.0,
            ..Engine::default()
        };
        e.transport(false, tempo, false, &mut |_| {});
        configure(
            &mut e,
            Config {
                mode: 1,
                strum_sync: sync,
                rate: 0.5,
                strum_ms: 120.0,
                ..Config::default()
            },
        );
        on(&mut e, 60, 0);
        let last = e.scheduled.iter().flatten().map(|note| note.at).max();
        assert_eq!(last, Some(expected * 2), "tempo {tempo}, sync {sync}");
    }
}

#[test]
fn playback_spread_leaves_chord_mode_close_and_revoices_on_mode_change() {
    let mut e = Engine::default();
    on(&mut e, 60, 1);
    let close = e.notes;
    let c = Config {
        spread: 2,
        ..e.config
    };
    configure(&mut e, c);
    assert_eq!(e.notes.as_slice(), close.as_slice());
    configure(&mut e, Config { mode: MANUAL, ..c });
    assert_ne!(e.notes.as_slice(), close.as_slice());
    configure(&mut e, c);
    assert_eq!(e.notes.as_slice(), close.as_slice());
}

#[test]
fn strum_range_automation_keeps_a_usable_span() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: MANUAL,
            x_min: 0.99,
            x_max: 0.01,
            ..Config::default()
        },
    );
    send(&mut e, Command::X(0.875));
    assert!((e.x - 0.5).abs() < 0.00001);
}

#[test]
fn xy_cc_mappings_accept_all_channels_and_learning_locks_the_channel() {
    for axis in 0..2 {
        for kind in [1, 2] {
            let mut e = Engine::default();
            let mut config = Config::default();
            config.mappings = [Mapping::default(); 2];
            config.mappings[axis] = Mapping {
                kind,
                number: 7,
                channel: 16,
            };
            configure(&mut e, config);
            for channel in 0..16 {
                if axis == 0 {
                    e.x = 0.0;
                } else {
                    e.y = 0.0;
                }
                e.control(channel, 7, 64.0 / 127.0, &mut |_| {});
                if kind == 2 {
                    e.control(channel, 39, 1.0 / 127.0, &mut |_| {});
                }
                let value = if axis == 0 { e.x } else { e.y };
                let expected = if kind == 2 {
                    8193.0 / 16383.0
                } else {
                    64.0 / 127.0
                };
                assert!(
                    (value - expected).abs() < 0.0001,
                    "axis {axis} ignored CC on channel {channel}"
                );
            }
            send(&mut e, Command::Learn(axis as u8 + 1));
            e.control(5, 11, 0.75, &mut |_| {});
            let (learned_axis, mapping) = e.learned.take().unwrap();
            assert_eq!(learned_axis, axis);
            assert_eq!(mapping, Mapping::cc(11, 5));
            config.mappings[axis] = mapping;
            configure(&mut e, config);
            if axis == 0 {
                e.x = 0.0;
            } else {
                e.y = 0.0;
            }
            e.control(4, 11, 0.75, &mut |_| {});
            assert_eq!(if axis == 0 { e.x } else { e.y }, 0.0);
            e.control(5, 11, 0.75, &mut |_| {});
            assert_eq!(if axis == 0 { e.x } else { e.y }, 0.75);
        }
    }
}

#[test]
fn removed_gate_learn_target_cannot_capture_a_controller() {
    let mut e = Engine::default();
    send(&mut e, Command::Learn(3));
    assert_eq!(e.learn, 0);
    e.control(7, 12, 0.75, &mut |_| {});
    assert!(e.learned.is_none());
}

#[test]
fn manual_xy_cc74_assignment_accepts_member_channels() {
    for axis in 0..2 {
        let mut e = Engine::default();
        let mut config = Config::default();
        config.mappings = [Mapping::default(); 2];
        config.mappings[axis] = Mapping::cc(74, 16);
        configure(&mut e, config);
        for channel in 0..16 {
            if axis == 0 {
                e.x = 0.0;
            } else {
                e.y = 0.0;
            }
            e.control(channel, 74, 0.75, &mut |_| {});
            assert_eq!(if axis == 0 { e.x } else { e.y }, 0.75);
        }
    }
}

#[test]
fn recalled_base_voicing_returns_after_disabling_a_route() {
    use super::routing::{Route, TARGETS};
    let mut e = Engine::default();
    let mut c = Config {
        mode: MANUAL,
        ..Config::default()
    };
    c.routes[0] = Route {
        source: 2,
        target: TARGETS.iter().position(|t| t.id == "spread").unwrap() as u8,
        ..Route::default()
    };
    e.configure(c, &mut |_| {});
    e.command(
        Command::Recall(
            harmony::SavedChord {
                root: 60,
                second: None,
                quality: 0,
                inversion: 0,
                spread: 1,
                transpose: 0,
            }
            .encode(),
        ),
        &mut |_| {},
    );
    e.control(0, 1, 1.0, &mut |_| {});
    assert_eq!(e.config.spread, 4);
    c.routes[0].enabled = false;
    e.configure(c, &mut |_| {});
    assert_eq!(e.config.spread, 1);
}

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
                    octaves: 4,
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
fn once_octaves_limits_sweeps_without_changing_the_layout() {
    for octaves in 1..=4 {
        for direction in 0..2 {
            let mut e = Engine::default();
            configure(
                &mut e,
                Config {
                    mode: AUTO,
                    octaves,
                    arp_pattern: direction,
                    strum_ms: 0.0,
                    strum_sync: false,
                    ..Config::default()
                },
            );
            on(&mut e, 63, 1);
            let layout = e.snapshot().notes;
            let total = layout.len * octaves as usize;
            for sweep in 0..2 {
                if sweep == 1 {
                    e.rebuild(&mut |_| {});
                }
                let reverse = direction == 1;
                let expected = (0..total)
                    .map(|i| {
                        layout
                            .string(if reverse { total - 1 - i } else { i })
                            .unwrap()
                    })
                    .collect::<Vec<_>>();
                assert_eq!(notes(&tick(&mut e, 1)), expected);
                assert_eq!(e.snapshot().notes, layout);
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

fn split_config() -> Config {
    Config {
        key_split: true,
        split_note: 60,
        mode: MANUAL,
        ..Config::default()
    }
}
fn layer_notes(e: &Engine) -> Vec<u8> {
    let mut notes: Vec<_> = e
        .voices
        .iter()
        .flatten()
        .filter(|v| v.layer)
        .map(|v| v.note)
        .collect();
    notes.sort_unstable();
    notes
}

#[test]
fn split_keeps_left_selection_and_passes_right_notes_on_original_channels() {
    let mut e = Engine::default();
    configure(&mut e, split_config());
    assert!(notes(&on(&mut e, 48, 2)).is_empty());
    off(&mut e, 48, 2);
    assert_eq!(e.notes.as_slice(), &[48, 52, 55]);
    let events = on(&mut e, 60, 7);
    assert!(events.contains(&Out::On(7, 60, 0.8)));
    assert_eq!(e.memory.unwrap().root, 48);
    assert!(e.second.is_none());
    assert!(e.snapshot().held_notes[60]);
    assert!(e.snapshot().sounding_notes[60]);
    assert!(off(&mut e, 60, 7).contains(&Out::Off(7, 60, 0.3)));
    assert!(!e.snapshot().held_notes[60]);
    assert!(!e.snapshot().sounding_notes[60]);
}

#[test]
fn split_layer_gate_counts_each_right_hand_key_and_channel() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            always_bass: true,
            ..split_config()
        },
    );
    on(&mut e, 48, 0);
    assert!(layer_notes(&e).is_empty());
    on(&mut e, 72, 2);
    on(&mut e, 72, 3);
    assert_eq!(layer_notes(&e), vec![48]);
    off(&mut e, 48, 0);
    assert_eq!(layer_notes(&e), vec![48]);
    assert!(!off(&mut e, 72, 2)
        .iter()
        .any(|v| matches!(v, Out::Off(_, 48, _))));
    assert_eq!(layer_notes(&e), vec![48]);
    assert!(off(&mut e, 72, 3)
        .iter()
        .any(|v| matches!(v, Out::Off(_, 48, _))));
    assert!(layer_notes(&e).is_empty());
}

#[test]
fn split_right_hand_can_open_gate_before_a_chord_is_selected() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            always_chord: true,
            ..split_config()
        },
    );
    on(&mut e, 72, 2);
    assert!(layer_notes(&e).is_empty());
    on(&mut e, 48, 0);
    assert_eq!(layer_notes(&e), vec![48, 52, 55]);
}

#[test]
fn split_layer_toggles_are_independent_and_full_chord_ignores_filter() {
    let mut e = Engine::default();
    let config = Config {
        always_chord: true,
        filter: 1,
        inversion: 1,
        ..split_config()
    };
    configure(&mut e, config);
    on(&mut e, 48, 0);
    on(&mut e, 72, 2);
    assert_eq!(layer_notes(&e), e.full_notes.as_slice());
    assert!(!layer_notes(&e).contains(&48));
    configure(
        &mut e,
        Config {
            always_bass: true,
            ..config
        },
    );
    assert!(layer_notes(&e).contains(&48));
    configure(
        &mut e,
        Config {
            always_bass: true,
            always_chord: false,
            ..config
        },
    );
    assert_eq!(layer_notes(&e), vec![48]);
    configure(
        &mut e,
        Config {
            always_chord: false,
            ..config
        },
    );
    assert!(layer_notes(&e).is_empty());
    assert!(e.melody_note_held(2, 72));
}

#[test]
fn split_shared_melody_and_layer_pitch_has_one_attack_and_one_final_release() {
    for release_layer_first in [false, true] {
        let mut e = Engine::default();
        let config = Config {
            always_chord: true,
            ..split_config()
        };
        configure(&mut e, config);
        on(&mut e, 55, 1);
        let events = on(&mut e, 62, 0);
        assert_eq!(
            events
                .iter()
                .filter(|v| matches!(v, Out::On(0, 62, _)))
                .count(),
            1
        );
        if release_layer_first {
            let mut events = Vec::new();
            e.configure(
                Config {
                    always_chord: false,
                    ..config
                },
                &mut |v| events.push(v),
            );
            assert!(!events.iter().any(|v| matches!(v, Out::Off(0, 62, _))));
            assert_eq!(
                off(&mut e, 62, 0)
                    .iter()
                    .filter(|v| matches!(v, Out::Off(0, 62, _)))
                    .count(),
                1
            );
        } else {
            on(&mut e, 64, 2);
            assert!(!off(&mut e, 62, 0)
                .iter()
                .any(|v| matches!(v, Out::Off(0, 62, _))));
            assert_eq!(
                off(&mut e, 64, 2)
                    .iter()
                    .filter(|v| matches!(v, Out::Off(0, 62, _)))
                    .count(),
                1
            );
        }
    }
}

#[test]
fn split_layer_release_preserves_overlapping_strum_deadline() {
    let mut e = Engine::default();
    e.sample_rate = 1000.0;
    configure(
        &mut e,
        Config {
            always_bass: true,
            length_ms: 100.0,
            ..split_config()
        },
    );
    on(&mut e, 48, 0);
    on(&mut e, 72, 2);
    e.strike(48, 0.8, 100, &mut |_| {});
    assert!(!off(&mut e, 72, 2)
        .iter()
        .any(|v| matches!(v, Out::Off(_, 48, _))));
    assert!(e.snapshot().sounding_notes[48]);
    assert!(!tick(&mut e, 100)
        .iter()
        .any(|v| matches!(v, Out::Off(_, 48, _))));
    assert!(tick(&mut e, 1)
        .iter()
        .any(|v| matches!(v, Out::Off(_, 48, _))));
}

#[test]
fn split_latch_holds_layers_but_releases_melody_and_replaces_harmony() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            latch: true,
            always_chord: true,
            ..split_config()
        },
    );
    on(&mut e, 48, 0);
    on(&mut e, 72, 2);
    off(&mut e, 48, 0);
    assert!(off(&mut e, 72, 2).contains(&Out::Off(2, 72, 0.3)));
    assert_eq!(layer_notes(&e), vec![48, 52, 55]);
    let events = on(&mut e, 50, 0);
    assert!(events.iter().any(|v| matches!(v, Out::Off(_, 48, _))));
    assert_eq!(layer_notes(&e), vec![50, 54, 57]);
    assert!(!e.snapshot().sounding_notes[72]);
}

#[test]
fn split_mpe_melody_reserves_member_and_keeps_initial_expression() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mpe: true,
            members: 4,
            always_chord: true,
            ..split_config()
        },
    );
    on(&mut e, 48, 0);
    e.strike(48, 0.8, 1000, &mut |_| {});
    assert!(e.voices.iter().flatten().any(|v| v.channel == 1));
    e.bend(1, 0.7, &mut |_| {});
    e.pressure(1, None, 0.4, &mut |_| {});
    e.control(1, 74, 0.8, &mut |_| {});
    let events = on(&mut e, 72, 1);
    let attack = events
        .iter()
        .position(|v| matches!(v, Out::On(1, 72, _)))
        .unwrap();
    assert!(events[..attack].contains(&Out::Bend(1, 0.7)));
    assert!(events[..attack].contains(&Out::Pressure(1, 0.4)));
    assert!(events[..attack].contains(&Out::Cc(1, 74, 0.8)));
    assert!(e.voices.iter().flatten().all(|v| v.channel != 1));
    assert_eq!(layer_notes(&e), vec![48, 52, 55]);
    let mut events = Vec::new();
    e.strike(55, 0.8, 20, &mut |v| events.push(v));
    tick(&mut e, 30);
    assert!(e.melody_note_held(1, 72));
    assert!(!events.iter().any(|v| matches!(v, Out::Off(1, 72, _))));
    assert!(off(&mut e, 72, 1).contains(&Out::Off(1, 72, 0.3)));
}

#[test]
fn split_panic_boundaries_and_control_learning_release_all_owned_outputs() {
    for change in 0..5 {
        let mut e = Engine::default();
        let config = Config {
            always_chord: true,
            ..split_config()
        };
        configure(&mut e, config);
        on(&mut e, 48, 0);
        on(&mut e, 72, 4);
        let mut events = Vec::new();
        e.control(4, 64, 1.0, &mut |v| events.push(v));
        assert!(events.contains(&Out::Cc(4, 64, 1.0)));
        events.clear();
        match change {
            0 => e.panic(&mut |v| events.push(v)),
            1 => e.configure(
                Config {
                    split_note: 61,
                    ..config
                },
                &mut |v| events.push(v),
            ),
            2 => e.configure(
                Config {
                    key_split: false,
                    ..config
                },
                &mut |v| events.push(v),
            ),
            3 => e.configure(
                Config {
                    control_base: 72,
                    ..config
                },
                &mut |v| events.push(v),
            ),
            _ => {
                e.command(Command::Learn(4), &mut |_| {});
                e.midi_note(true, 4, 72, 0.8, &mut |v| events.push(v));
            }
        }
        assert_eq!(
            events
                .iter()
                .filter(|v| matches!(v, Out::Off(4, 72, _)))
                .count(),
            1
        );
        assert!(events.contains(&Out::Cc(4, 64, 0.0)));
        assert!(!e.melody_note_held(4, 72));
        assert!(!e.owned_output_notes().iter().flatten().any(|&v| v));
    }
}

#[test]
fn split_output_route_changes_preserve_transparent_melody_and_its_gate() {
    let mut e = Engine::default();
    let config = Config {
        always_chord: true,
        ..split_config()
    };
    configure(&mut e, config);
    on(&mut e, 48, 0);
    on(&mut e, 72, 1);
    on(&mut e, 76, 2);
    let mut events = Vec::new();
    e.configure(
        Config {
            mpe: true,
            members: 6,
            ..config
        },
        &mut |v| events.push(v),
    );
    assert!(!events
        .iter()
        .any(|v| matches!(v, Out::Off(1, 72, _) | Out::Off(2, 76, _))));
    assert!(e.melody_note_held(1, 72));
    assert!(e.melody_note_held(2, 76));
    assert_eq!(layer_notes(&e), vec![48, 52, 55]);
    assert!(e.voices.iter().flatten().all(|v| v.channel > 2));
    assert!(off(&mut e, 72, 1).contains(&Out::Off(1, 72, 0.3)));
    assert_eq!(layer_notes(&e), vec![48, 52, 55]);
    assert!(off(&mut e, 76, 2).contains(&Out::Off(2, 76, 0.3)));
    assert!(layer_notes(&e).is_empty());
}

#[test]
fn split_control_octave_precedes_melody_gate_and_never_triggers_layers() {
    let mut e = Engine::default();
    let config = Config {
        control_base: 72,
        always_chord: true,
        ..split_config()
    };
    configure(&mut e, config);
    on(&mut e, 48, 0);
    assert!(notes(&on(&mut e, 75, 3)).is_empty());
    assert!(layer_notes(&e).is_empty());
    assert!(!e.melody_note_held(3, 75));
    on(&mut e, 67, 1);
    let held = layer_notes(&e);
    let events = on(&mut e, 72, 3);
    assert!(events
        .iter()
        .all(|v| !matches!(v, Out::On(..) | Out::Off(..))));
    configure(&mut e, config);
    assert_eq!(layer_notes(&e), held);
    assert!(off(&mut e, 72, 3).is_empty());
}

#[test]
fn split_recall_preserves_held_melody_and_updates_layer() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            always_chord: true,
            ..split_config()
        },
    );
    on(&mut e, 48, 0);
    on(&mut e, 72, 2);
    let chord = harmony::SavedChord {
        root: 50,
        second: None,
        quality: 0,
        inversion: 0,
        spread: 0,
        transpose: 0,
    };
    let events = send(&mut e, Command::Recall(chord.encode()));
    assert!(!events.iter().any(|v| matches!(v, Out::Off(2, 72, _))));
    assert!(e.snapshot().held_notes[72]);
    assert_eq!(layer_notes(&e), vec![50, 54, 57]);
    assert!(off(&mut e, 72, 2).contains(&Out::Off(2, 72, 0.3)));
    assert!(layer_notes(&e).is_empty());
}

#[test]
fn sustained_layers_without_split_gate_on_performance_keys() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            always_bass: true,
            mode: MANUAL,
            ..Config::default()
        },
    );
    on(&mut e, 48, 0);
    assert_eq!(layer_notes(&e), vec![48]);
    off(&mut e, 48, 0);
    assert!(layer_notes(&e).is_empty());
    assert!(!e.snapshot().sounding_notes[48]);
}

#[test]
fn split_reset_controllers_releases_forwarded_sustain() {
    let mut e = Engine::default();
    configure(&mut e, split_config());
    e.control(4, 64, 1.0, &mut |_| {});
    on(&mut e, 72, 4);
    off(&mut e, 72, 4);
    let mut events = Vec::new();
    e.control(4, 121, 0.0, &mut |v| events.push(v));
    assert!(events.contains(&Out::Cc(4, 64, 0.0)));
    assert!(!e.melody_sustain[4]);
}

#[test]
fn split_melody_on_mpe_master_uses_master_bend_range() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mpe: true,
            ..split_config()
        },
    );
    e.bend(0, 0.75, &mut |_| {});
    let events = on(&mut e, 72, 0);
    assert!(events.contains(&Out::Bend(0, 0.75)));
    let mut events = Vec::new();
    e.bend(0, 0.8, &mut |v| events.push(v));
    assert!(events
        .iter()
        .filter(|v| matches!(v, Out::Bend(0, _)))
        .all(|v| *v == Out::Bend(0, 0.8)));
}

#[test]
fn chord_hand_seventh_and_fifth_reach_sustained_output() {
    for (second, expected, label) in [
        (58, vec![48, 52, 55, 58], "C7"),
        (46, vec![48, 52, 55, 58], "C7"),
        (55, vec![48, 55], "C5"),
    ] {
        for channel in [0, 1] {
            let mut e = Engine::default();
            configure(
                &mut e,
                Config {
                    always_chord: true,
                    ..split_config()
                },
            );
            on(&mut e, 48, 0);
            on(&mut e, 72, 4);
            on(&mut e, second, channel);
            assert_eq!(
                e.full_notes.as_slice(),
                expected.as_slice(),
                "second {second}"
            );
            assert_eq!(layer_notes(&e), expected, "second {second}");
            assert_eq!(harmony::chord_name(e.memory.unwrap()), label);
            off(&mut e, second, channel);
            off(&mut e, 48, 0);
            assert_eq!(layer_notes(&e), expected);
        }
    }
}

#[test]
fn expression_is_unassigned_by_default_but_field_y_controls_velocity() {
    let mut e = Engine::default();
    let original = e.strike_velocity();
    for channel in 0..16 {
        e.control(channel, 11, 0.1, &mut |_| {});
        e.control(channel, 11, 0.9, &mut |_| {});
    }
    assert!(!e.y_active);
    assert_eq!(e.strike_velocity(), original);
    e.position(0.2, 1, &mut |_| {});
    assert!(e.y_active);
    let low = e.strike_velocity();
    e.position(0.9, 1, &mut |_| {});
    assert!(e.strike_velocity() > low);
}

#[test]
fn quality_changes_preserve_arp_step_and_deadline_for_held_and_latched_chords() {
    for latched in [false, true] {
        for source in 0..4 {
            let mut e = Engine::default();
            let mut config = Config {
                mode: ARP,
                latch: latched,
                control_base: 36,
                ..Config::default()
            };
            if source == 3 {
                config.routes[0] = routing::Route {
                    source: 2,
                    target: 2,
                    ..routing::Route::default()
                };
            }
            configure(&mut e, config);
            on(&mut e, 60, 0);
            tick(&mut e, 7000);
            if latched {
                off(&mut e, 60, 0);
            }
            let position = (e.arp_step, e.arp_next);
            let previous = e.notes;
            let mut events = Vec::new();
            match source {
                0 => e.command(Command::SetQuality(1), &mut |v| events.push(v)),
                1 => e.command(Command::SetControlChord(3), &mut |v| events.push(v)),
                2 => e.midi_note(true, 0, 39, 0.8, &mut |v| events.push(v)),
                _ => e.control(0, 1, 1.0 / 11.0, &mut |v| events.push(v)),
            }
            assert_eq!(
                (e.arp_step, e.arp_next),
                position,
                "source {source}, latched {latched}"
            );
            assert_ne!(e.notes, previous);
            assert!(
                notes(&events).is_empty(),
                "quality selection must not strike immediately"
            );
            let remaining = (e.arp_next - e.now) as usize;
            assert!(notes(&tick(&mut e, remaining)).is_empty());
            assert_eq!(notes(&tick(&mut e, 1)).len(), 1);
            assert_eq!(e.arp_step, position.0 + 1);
        }
    }
}

#[test]
fn quality_changes_with_only_control_notes_do_not_start_an_arp() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: ARP,
            control_base: 36,
            ..Config::default()
        },
    );
    on(&mut e, 60, 0);
    tick(&mut e, 100);
    off(&mut e, 60, 0);
    let position = (e.arp_step, e.arp_next);
    assert!(notes(&on(&mut e, 39, 0)).is_empty());
    assert!(notes(&send(&mut e, Command::SetQuality(4))).is_empty());
    assert!(notes(&tick(&mut e, 12000)).is_empty());
    assert_eq!((e.arp_step, e.arp_next), position);
}

#[test]
fn low_zone_routes_only_to_bass_and_never_changes_harmony() {
    for mode in [CHORD, AUTO, MANUAL, ARP] {
        let mut e = Engine::default();
        configure(
            &mut e,
            Config {
                mode,
                control_base: 36,
                bass_channel: 4,
                key_split: true,
                split_note: 60,
                ..Config::default()
            },
        );
        let events = on(&mut e, 35, 2);
        assert!(matches!(events.as_slice(), [Out::On(4, 35, _)]));
        assert!(e.root.is_none());
        assert_eq!(e.notes.len, 0);
        assert!(e.owned_output_notes()[4][35]);
        assert!(notes(&on(&mut e, 36, 2)).is_empty());
        assert!(matches!(
            off(&mut e, 35, 2).as_slice(),
            [Out::Off(4, 35, _)]
        ));
        assert!(!e.owned_output_notes()[4][35]);
    }
}

#[test]
fn bass_zone_sustain_duplicate_owners_and_rerouting_release_safely() {
    let mut e = Engine::default();
    let config = Config {
        control_base: 36,
        bass_channel: 4,
        ..Config::default()
    };
    configure(&mut e, config);
    on(&mut e, 24, 0);
    assert!(on(&mut e, 24, 1).is_empty());
    assert!(off(&mut e, 24, 0).is_empty());
    e.control(1, 64, 1.0, &mut |_| {});
    assert!(off(&mut e, 24, 1).is_empty());
    assert!(e.owned_output_notes()[4][24]);
    let mut events = Vec::new();
    e.control(1, 64, 0.0, &mut |v| events.push(v));
    assert!(events.iter().any(|v| matches!(v, Out::Off(4, 24, _))));
    on(&mut e, 24, 0);
    events.clear();
    e.configure(
        Config {
            bass_channel: 5,
            ..config
        },
        &mut |v| events.push(v),
    );
    assert!(events.iter().any(|v| matches!(v, Out::Off(4, 24, _))));
    assert!(!e.owned_output_notes()[4][24]);
    assert!(matches!(on(&mut e, 24, 0).as_slice(), [Out::On(5, 24, _)]));
    events.clear();
    e.panic(&mut |v| events.push(v));
    assert!(events.iter().any(|v| matches!(v, Out::Off(5, 24, _))));
}

#[test]
fn bass_split_can_cross_the_control_octave_without_consuming_control_keys() {
    for boundary in [24, 48, 60] {
        let mut e = Engine::default();
        configure(
            &mut e,
            Config {
                control_base: 36,
                bass_split: boundary,
                bass_channel: 4,
                ..Config::default()
            },
        );
        for note in 0..boundary as u8 {
            let events = on(&mut e, note, 0);
            if (36..48).contains(&note) {
                assert!(
                    notes(&events).is_empty(),
                    "control key {note} must not play bass"
                );
            } else {
                assert!(matches!(events.as_slice(), [Out::On(4, n, _)] if *n == note));
            }
            off(&mut e, note, 0);
        }
        assert!(e.root.is_none());
        on(&mut e, boundary as u8, 0);
        assert_eq!(e.root.map(|root| root.note), Some(boundary as u8));
    }
}

#[test]
fn moving_bass_split_releases_notes_from_the_old_zone() {
    let mut e = Engine::default();
    let config = Config {
        control_base: 36,
        bass_split: 60,
        bass_channel: 4,
        ..Config::default()
    };
    configure(&mut e, config);
    on(&mut e, 55, 0);
    let mut events = Vec::new();
    e.configure(
        Config {
            bass_split: 24,
            ..config
        },
        &mut |v| events.push(v),
    );
    assert!(events.iter().any(|v| matches!(v, Out::Off(4, 55, _))));
    assert!(e.owned_output_notes().iter().flatten().all(|held| !held));
}

#[test]
fn auto_strum_orders_notes_in_ascending_and_descending_thirds() {
    let mut e = Engine::default();
    let config_up_3rds = Config {
        mode: AUTO,
        arp_pattern: 3, // Up in 3rds
        octaves: 2,
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
        octaves: 2,
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

#[test]
fn learned_splits_swap_crossings_and_consume_the_learning_key_release() {
    for (target, note, expected) in [
        (5, 80, (60, 80)),
        (6, 20, (20, 36)),
        (5, 60, (60, 61)),
        (6, 36, (36, 37)),
    ] {
        let mut e = Engine::default();
        configure(
            &mut e,
            Config {
                mode: MANUAL,
                bass_split: 36,
                split_note: 60,
                key_split: true,
                ..Config::default()
            },
        );
        send(&mut e, Command::Learn(target));
        assert!(notes(&on(&mut e, note, 0)).is_empty());
        assert_eq!((e.config.bass_split as u8, e.config.split_note), expected);
        assert_eq!(e.learn, 0);
        assert_eq!(
            LearnedSplit::decode(e.learned_split.unwrap().encode()),
            e.learned_split
        );
        assert!(off(&mut e, note, 0).is_empty());
        assert!(e.root.is_none());
    }
}

#[test]
fn bass_bypass_releases_owned_bass_and_returns_low_notes_to_performance() {
    let mut e = Engine::default();
    let config = Config {
        mode: MANUAL,
        bass_split: 36,
        always_bass: true,
        ..Config::default()
    };
    configure(&mut e, config);
    assert_eq!(notes(&on(&mut e, 24, 0)), vec![24]);
    let mut released = Vec::new();
    e.configure(
        Config {
            bass_enabled: false,
            ..config
        },
        &mut |event| released.push(event),
    );
    assert!(released
        .iter()
        .any(|event| matches!(event, Out::Off(_, 24, _))));
    assert!(e.owned_output_notes().iter().flatten().all(|on| !on));
    assert!(notes(&on(&mut e, 24, 0)).is_empty());
    assert_eq!(e.root.unwrap().note, 24);
}

#[test]
fn split_order_always_leaves_one_chord_key() {
    for bass in 0..128 {
        for melody in 0..128 {
            let (low, high) = ordered_splits(bass, melody);
            assert!(low < high && high < 128);
        }
    }
}

#[test]
fn arp_controls_do_not_restart_the_clock() {
    for control in 0..8 {
        let mut e = Engine {
            sample_rate: 1000.0,
            ..Engine::default()
        };
        configure(
            &mut e,
            Config {
                mode: ARP,
                rate: 0.1,
                ..Config::default()
            },
        );
        on(&mut e, 60, 1);
        tick(&mut e, 72);
        let position = (e.arp_step, e.arp_next);
        let mut c = e.config;
        match control {
            0 => c.output_channel = 2,
            1 => c.mpe = true,
            2 => c.mode = MANUAL,
            3 => c.inversion = 1,
            4 => c.octaves = 3,
            5 => c.arp_pattern = 2,
            6 => {
                send(&mut e, Command::Inversion(1));
            }
            _ => e.transport(true, 120.0, false, &mut |_| {}),
        }
        if control < 6 {
            configure(&mut e, c);
        }
        assert_eq!((e.arp_step, e.arp_next), position, "control {control}");
        if control == 2 {
            tick(&mut e, 100);
            c.mode = ARP;
            configure(&mut e, c);
            assert_eq!((e.arp_step, e.arp_next), (4, 200));
        }
    }
}

#[test]
fn arp_octave_edits_wait_for_the_current_cycle_limit() {
    for (before, after) in [(1, 3), (3, 1)] {
        let mut e = Engine {
            sample_rate: 80.0,
            ..Engine::default()
        };
        configure(
            &mut e,
            Config {
                mode: ARP,
                rate: 0.25,
                octaves: before,
                ..Config::default()
            },
        );
        on(&mut e, 60, 1);
        let original = e.notes;
        let mut heard = notes(&tick(&mut e, 11));
        let mut c = e.config;
        c.octaves = after;
        configure(&mut e, c);
        let old_len = original.len * before as usize;
        heard.extend(notes(&tick(&mut e, old_len * 10 - 11)));
        assert_eq!(
            heard,
            (0..old_len)
                .map(|i| original.string(i).unwrap())
                .collect::<Vec<_>>()
        );
        let next = notes(&tick(&mut e, original.len * after as usize * 10));
        assert_eq!(
            next,
            (0..original.len * after as usize)
                .map(|i| original.string(i).unwrap())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn arp_swing_edit_preserves_pair_duration() {
    let mut e = Engine {
        sample_rate: 1000.0,
        ..Engine::default()
    };
    configure(
        &mut e,
        Config {
            mode: ARP,
            rate: 0.1,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    tick(&mut e, 20);
    let mut c = e.config;
    c.swing = 0.5;
    configure(&mut e, c);
    tick(&mut e, 31);
    assert_eq!(e.arp_next, 100);
    tick(&mut e, 50);
    assert_eq!(e.arp_next, 175);
    tick(&mut e, 75);
    assert_eq!(e.arp_next, 200);
}

#[test]
fn arp_fractional_intervals_do_not_accumulate_rounding_drift() {
    let mut e = Engine {
        sample_rate: 1000.0,
        tempo: 137.0,
        ..Engine::default()
    };
    configure(
        &mut e,
        Config {
            mode: ARP,
            rate: 0.25,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    tick(&mut e, 100_000);
    let expected = e.arp_step as f64 * 1000.0 * 60.0 / 137.0 * 0.25;
    assert!(
        (e.arp_next as f64 - expected).abs() < 1.0,
        "deadline {} vs {expected}",
        e.arp_next
    );
}

#[test]
fn xy_cc_auto_resolution_is_channel_local_and_resets() {
    let mut e = Engine::default();
    let mut config = Config::default();
    config.mappings = [Mapping::cc(7, 16); 2];
    configure(&mut e, config);
    e.control(0, 7, 1.0, &mut |_| {});
    assert_eq!((e.x, e.y), (1.0, 1.0));
    e.control(0, 39, 0.0, &mut |_| {});
    let expected = 16256.0 / 16383.0;
    assert!((e.x - expected).abs() < 0.00001);
    assert!((e.y - expected).abs() < 0.00001);
    e.control(0, 39, 1.0, &mut |_| {});
    assert_eq!((e.x, e.y), (1.0, 1.0));
    e.control(1, 7, 64.0 / 127.0, &mut |_| {});
    assert!((e.x - 64.0 / 127.0).abs() < 0.00001);
    e.control(0, 121, 0.0, &mut |_| {});
    e.control(0, 7, 1.0, &mut |_| {});
    assert_eq!((e.x, e.y), (1.0, 1.0));
}

#[test]
fn arp_pattern_edits_finish_the_previous_cycle_without_moving_beats() {
    for pattern in 0..7 {
        let mut e = Engine {
            sample_rate: 80.0,
            ..Engine::default()
        };
        configure(
            &mut e,
            Config {
                mode: ARP,
                rate: 0.25,
                arp_pattern: pattern,
                ..Config::default()
            },
        );
        on(&mut e, 60, 1);
        tick(&mut e, 1);
        let cycle = [3, 3, 4, 6, 6, 3, 3][pattern as usize];
        let mut c = e.config;
        c.arp_pattern = (pattern + 1) % 7;
        c.octaves = 2;
        configure(&mut e, c);
        tick(&mut e, cycle * 10 - 1);
        assert_eq!(e.arp_pattern, pattern);
        assert_eq!(e.arp_octaves, 1);
        assert_eq!(e.arp_step, cycle);
        assert_eq!(e.arp_next, (cycle * 10) as u64);
        tick(&mut e, 1);
        assert_eq!(e.arp_pattern, c.arp_pattern);
        assert_eq!(e.arp_octaves, 2);
        assert_eq!(e.arp_step, cycle + 1);
    }
}

#[test]
fn arp_rate_edit_finishes_the_swing_pair_before_changing_division() {
    let mut e = Engine {
        sample_rate: 800.0,
        ..Engine::default()
    };
    configure(
        &mut e,
        Config {
            mode: ARP,
            rate: 0.25,
            swing: 0.5,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    tick(&mut e, 20);
    let mut c = e.config;
    c.rate = 0.5;
    configure(&mut e, c);
    tick(&mut e, 131);
    assert_eq!(e.arp_next, 200);
    tick(&mut e, 50);
    assert_eq!(e.arp_next, 500);
}

#[test]
fn arp_new_chord_restarts_but_revoicing_controls_do_not() {
    let mut e = Engine {
        sample_rate: 80.0,
        ..Engine::default()
    };
    configure(
        &mut e,
        Config {
            mode: ARP,
            rate: 0.25,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    tick(&mut e, 22);
    for change in 0..9 {
        let mut c = e.config;
        match change {
            0 => c.quality = 2,
            1 => c.filter = 1,
            2 => c.spread = 1,
            3 => c.transpose = 12,
            4 => c.voice_leading = 0,
            5 => c.gate = 0.9,
            6 => c.humanize = 0.5,
            7 => c.contour = 1.0,
            _ => c.root_on_select = true,
        }
        configure(&mut e, c);
        assert_eq!((e.arp_step, e.arp_next), (3, 30), "change {change}");
    }
    off(&mut e, 60, 1);
    on(&mut e, 62, 1);
    assert_eq!((e.arp_step, e.arp_next), (0, 22));
}

#[test]
fn arp_releasing_a_chord_tone_does_not_restart_the_clock() {
    let mut e = Engine {
        sample_rate: 80.0,
        ..Engine::default()
    };
    configure(
        &mut e,
        Config {
            mode: ARP,
            rate: 0.25,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    tick(&mut e, 12);
    on(&mut e, 64, 1);
    assert_eq!((e.arp_step, e.arp_next), (2, 20));
    tick(&mut e, 10);
    off(&mut e, 60, 1);
    assert_eq!(e.root.unwrap().note, 64);
    assert_eq!((e.arp_step, e.arp_next), (3, 30));
}

#[test]
fn final_contoured_velocity_drives_same_note_length_before_midi_output() {
    for mode in [AUTO, MANUAL, ARP] {
        let mut e = Engine {
            sample_rate: 1000.0,
            ..Engine::default()
        };
        let mut c = Config {
            mode,
            contour: 1.0,
            humanize: 0.0,
            strings: 3,
            strings_played: 3,
            velocity: 0.4,
            ..Config::default()
        };
        let target = routing::TARGETS
            .iter()
            .position(|t| t.id == "length_ms")
            .unwrap();
        c.routes[1] = routing::Route {
            source: 7,
            target: target as u8,
            ..routing::Route::default()
        };
        configure(&mut e, c);
        on(&mut e, 60, 1);
        let output = if mode == MANUAL {
            let mut events = Vec::new();
            e.pluck_string(0, &mut |v| events.push(v));
            events
        } else {
            tick(&mut e, 1)
        };
        let velocity = output
            .iter()
            .find_map(|v| {
                if let Out::On(_, _, velocity) = v {
                    Some(*velocity)
                } else {
                    None
                }
            })
            .unwrap();
        assert!((velocity - 0.2).abs() < 0.0001, "mode {mode}");
        assert_eq!(e.snapshot().sources[6], velocity);
        let voice = e.voices.iter().flatten().find(|v| !v.layer).unwrap();
        let expected = (20.0 + velocity * 2980.0) as u64;
        assert_eq!(voice.off - voice.started, expected, "mode {mode}");
        if mode == AUTO {
            assert!(e.scheduled.iter().flatten().count() >= 2);
        }
    }
}

#[test]
fn each_arp_note_routes_its_final_velocity_to_gate_without_rounding_the_step_twice() {
    let mut e = Engine {
        sample_rate: 1000.0,
        ..Engine::default()
    };
    let mut config = Config {
        mode: ARP,
        rate: 0.25,
        contour: 1.0,
        velocity: 0.4,
        ..Config::default()
    };
    let gate = routing::TARGETS
        .iter()
        .position(|t| t.id == "gate")
        .unwrap();
    config.routes[1] = routing::Route {
        source: 7,
        target: gate as u8,
        ..routing::Route::default()
    };
    configure(&mut e, config);
    on(&mut e, 60, 1);
    for _ in 0..3 {
        let mut emitted = None;
        while emitted.is_none() {
            e.tick(&mut |event| {
                if let Out::On(_, note, velocity) = event {
                    emitted = Some((note, velocity));
                }
            });
        }
        let (note, velocity) = emitted.unwrap();
        let voice = e.voices.iter().flatten().find(|v| v.note == note).unwrap();
        let expected_gate = 0.05 + 0.95 * velocity;
        assert_eq!(
            voice.off - voice.started,
            (125.0 * expected_gate).max(1.0) as u64
        );
        assert_eq!(e.snapshot().sources[6], velocity);
    }
}

#[test]
fn strumfield_plays_without_a_self_route_and_still_drives_modulation() {
    let mut e = Engine::default();
    let mut c = Config {
        mode: MANUAL,
        routes: [routing::Route::default(); routing::ROUTE_COUNT],
        ..Config::default()
    };
    c.routes[0] = routing::Route {
        source: 8,
        target: routing::TARGETS
            .iter()
            .position(|t| t.id == "length_ms")
            .unwrap() as u8,
        ..routing::Route::default()
    };
    configure(&mut e, c);
    on(&mut e, 60, 1);
    assert_eq!(
        notes(&send(&mut e, Command::BeginGesture(0.0, 0.8))),
        vec![60]
    );
    assert_eq!(
        notes(&send(&mut e, Command::X(1.0))),
        vec![64, 67, 72, 76, 79, 84, 88]
    );
    assert_eq!(e.sources[7], 1.0);
    assert_eq!(e.x, 1.0);
    assert_eq!(e.config.length_ms, 3000.0);
    assert!(notes(&send(&mut e, Command::X(1.0))).is_empty());
}

#[test]
fn once_sweep_octaves_feel_and_duration_preserve_scheduled_cycle() {
    let make = |swing, humanize| {
        let mut e = Engine::default();
        e.sample_rate = 1000.0;
        configure(
            &mut e,
            Config {
                mode: AUTO,
                strings: 3,
                strings_played: 3,
                octaves: 2,
                strum_ms: 200.0,
                strum_sync: false,
                swing,
                humanize,
                ..Config::default()
            },
        );
        on(&mut e, 60, 1);
        let strikes: Vec<_> = e.scheduled.iter().flatten().copied().collect();
        (e, strikes)
    };
    let (_, plain) = make(0.0, 0.0);
    let (mut e, felt) = make(0.4, 1.0);
    assert_eq!(
        plain.iter().map(|s| s.note).collect::<Vec<_>>(),
        vec![60, 64, 67, 72, 76, 79]
    );
    assert!(felt.first().unwrap().at <= 28);
    assert!(felt.last().unwrap().at > plain.last().unwrap().at);
    assert!(felt.last().unwrap().at < 1200);
    assert!(felt[1].at > plain[1].at);
    assert!(felt[1].velocity < plain[1].velocity);
    assert!(felt.iter().all(|s| s.gate_step.is_none()));
    let previous = felt.iter().map(|s| (s.note, s.at)).collect::<Vec<_>>();
    let mut c = e.host_config;
    c.octaves = 1;
    c.swing = 0.0;
    configure(&mut e, c);
    assert_eq!(
        e.scheduled
            .iter()
            .flatten()
            .map(|s| (s.note, s.at))
            .collect::<Vec<_>>(),
        previous
    );
    assert_eq!(notes(&tick(&mut e, 1200)), vec![60, 64, 67, 72, 76, 79]);
    assert!(notes(&tick(&mut e, 1200)).is_empty());
}

#[test]
fn once_and_loop_use_identical_patterns_and_per_note_rates() {
    for pattern in 0..7 {
        for (sync, tempo, interval) in [
            (true, 120.0, 125),
            (true, 60.0, 250),
            (false, 120.0, 80),
            (false, 60.0, 80),
        ] {
            let make = |mode| {
                let mut e = Engine {
                    sample_rate: 1000.0,
                    ..Engine::default()
                };
                e.transport(false, tempo, false, &mut |_| {});
                configure(
                    &mut e,
                    Config {
                        mode,
                        strings: 3,
                        strings_played: 3,
                        arp_pattern: pattern,
                        strum_sync: sync,
                        strum_ms: 80.0,
                        rate: 0.25,
                        humanize: 0.0,
                        ..Config::default()
                    },
                );
                on(&mut e, 60, 1);
                e
            };
            let mut once = make(AUTO);
            let mut looped = make(ARP);
            let count = Engine::pattern_length(pattern, 3);
            let mut finite = Vec::new();
            let mut repeating = Vec::new();
            for time in 0..count * interval {
                once.tick(&mut |out| {
                    if let Out::On(_, note, _) = out {
                        finite.push((time, note));
                    }
                });
                looped.tick(&mut |out| {
                    if let Out::On(_, note, _) = out {
                        repeating.push((time, note));
                    }
                });
            }
            assert_eq!(
                finite, repeating,
                "pattern {pattern}, sync {sync}, tempo {tempo}"
            );
            assert_eq!(finite.len(), count);
            assert_eq!(
                finite.iter().map(|(t, _)| *t).collect::<Vec<_>>(),
                (0..count).map(|step| step * interval).collect::<Vec<_>>()
            );
            assert!(notes(&tick(&mut once, interval + 1)).is_empty());
            assert!(!notes(&tick(&mut looped, interval + 1)).is_empty());
        }
    }
}

#[test]
fn once_long_patterns_fit_scheduler_and_played_order_stays_in_range() {
    let mut e = Engine {
        sample_rate: 1000.0,
        ..Engine::default()
    };
    configure(
        &mut e,
        Config {
            mode: AUTO,
            octaves: 4,
            arp_pattern: 3,
            ..Config::default()
        },
    );
    on(&mut e, 24, 1);
    assert_eq!(e.scheduled.iter().flatten().count(), 24);
    for count in 1..=12 {
        for step in 0..count {
            assert!(e.arp_index(5, step, count) < count);
        }
    }
}

#[test]
fn note_feedback_records_octave_strikes_even_after_short_notes_end() {
    let mut e = Engine {
        sample_rate: 1000.0,
        ..Engine::default()
    };
    configure(
        &mut e,
        Config {
            mode: ARP,
            octaves: 4,
            rate: 0.0625,
            gate: 0.05,
            ..Config::default()
        },
    );
    on(&mut e, 60, 1);
    tick(&mut e, 285);
    let snapshot = e.snapshot();
    assert!(snapshot.note_strikes[96] > 0);
    assert!(!snapshot.sounding_notes[96]);
}

#[test]
fn looped_chord_can_have_9_notes_of_length() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: ARP,
            octaves: 3,
            arp_pattern: 0, // Up
            humanize: 0.0,
            ..Config::default()
        },
    );
    // Chord 60 is C major triad: C4 (60), E4 (64), G4 (67).
    // With octaves: 3, total length is 3 notes * 3 octaves = 9 notes.
    on(&mut e, 60, 0);
    let mut played = Vec::new();
    while played.len() < 10 {
        e.tick(&mut |event| {
            if let Out::On(_, note, _) = event {
                played.push(note);
            }
        });
    }
    assert_eq!(played.len(), 10);
    assert_eq!(
        played,
        vec![60, 64, 67, 72, 76, 79, 84, 88, 91, 60]
    );
}

#[test]
fn four_and_five_note_chords_can_loop_at_9_notes() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: ARP,
            octaves: 3,
            loop_start: 1,
            loop_end: 9,
            arp_pattern: 0, // Up
            humanize: 0.0,
            ..Config::default()
        },
    );
    e.command(Command::KeyDown(0, 60, 3), &mut |_| {});
    let mut played_4 = Vec::new();
    while played_4.len() < 10 {
        e.tick(&mut |event| {
            if let Out::On(_, note, _) = event {
                played_4.push(note);
            }
        });
    }
    assert_eq!(played_4.len(), 10);
    assert_eq!(
        played_4,
        vec![60, 64, 67, 71, 72, 76, 79, 83, 84, 60]
    );

    let mut e5 = Engine::default();
    configure(
        &mut e5,
        Config {
            mode: ARP,
            octaves: 2,
            loop_start: 1,
            loop_end: 9,
            arp_pattern: 0,
            humanize: 0.0,
            ..Config::default()
        },
    );
    e5.notes = harmony::Notes {
        values: [60, 62, 64, 67, 71, 0, 0, 0, 0, 0, 0, 0],
        len: 5,
    };
    e5.restart_arp();
    let mut played_5 = Vec::new();
    while played_5.len() < 10 {
        e5.tick(&mut |event| {
            if let Out::On(_, note, _) = event {
                played_5.push(note);
            }
        });
    }
    assert_eq!(played_5.len(), 10);
    assert_eq!(
        played_5,
        vec![60, 62, 64, 67, 71, 72, 74, 76, 79, 60]
    );
}

#[test]
fn loop_start_and_end_range_plays_custom_subsegment() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: ARP,
            octaves: 3,
            loop_start: 2,
            loop_end: 5,
            arp_pattern: 0, // Up
            humanize: 0.0,
            ..Config::default()
        },
    );
    e.command(Command::KeyDown(0, 60, 3), &mut |_| {});
    let mut played = Vec::new();
    while played.len() < 5 {
        e.tick(&mut |event| {
            if let Out::On(_, note, _) = event {
                played.push(note);
            }
        });
    }
    // String 2 (E3 = 64), String 3 (G3 = 67), String 4 (B3 = 71), String 5 (C4 = 72), wrapping to 64
    assert_eq!(played, vec![64, 67, 71, 72, 64]);
}

#[test]
fn arp_per_note_volume_scales_velocity() {
    let mut e = Engine::default();
    let mut arp_volume = [100u8; 32];
    arp_volume[0] = 100; // 100% -> vel * 1.0
    arp_volume[1] = 50;  // 50%  -> vel * 0.5
    arp_volume[2] = 20;  // 20%  -> vel * 0.2
    arp_volume[3] = 80;  // 80%  -> vel * 0.8
    configure(
        &mut e,
        Config {
            mode: ARP,
            octaves: 1,
            arp_pattern: 0,
            arp_volume,
            humanize: 0.0,
            ..Config::default()
        },
    );
    e.command(Command::KeyDown(0, 60, 3), &mut |_| {});
    let mut played = Vec::new();
    while played.len() < 4 {
        e.tick(&mut |event| {
            if let Out::On(_, note, vel) = event {
                played.push((note, vel));
            }
        });
    }
    assert_eq!(played.len(), 4);
    let base_vel = 0.8;
    assert!((played[0].1 - base_vel * 1.0).abs() < 1e-3);
    assert!((played[1].1 - base_vel * 0.5).abs() < 1e-3);
    assert!((played[2].1 - base_vel * 0.2).abs() < 1e-3);
    assert!((played[3].1 - base_vel * 0.8).abs() < 1e-3);
}

#[test]
fn arp_per_note_mute_acts_as_musical_rest() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: ARP,
            octaves: 1,
            arp_pattern: 0,
            arp_mute: 1 << 1, // mute string 1 (index 1)
            humanize: 0.0,
            ..Config::default()
        },
    );
    e.command(Command::KeyDown(0, 60, 3), &mut |_| {});
    let mut played = Vec::new();
    let mut ticks = 0;
    while played.len() < 3 && ticks < 200_000 {
        e.tick(&mut |event| {
            if let Out::On(_, note, _) = event {
                played.push(note);
            }
        });
        ticks += 1;
    }
    // String 0 is 60, string 1 is muted (rest), string 2 is 67, string 3 is 71
    assert_eq!(played, vec![60, 67, 71]);
}

#[test]
fn arp_per_note_skip_bypasses_step() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: ARP,
            octaves: 1,
            arp_pattern: 0,
            arp_skip: 1 << 1, // skip string 1 (index 1)
            humanize: 0.0,
            ..Config::default()
        },
    );
    e.command(Command::KeyDown(0, 60, 3), &mut |_| {});
    let mut played = Vec::new();
    while played.len() < 4 {
        e.tick(&mut |event| {
            if let Out::On(_, note, _) = event {
                played.push(note);
            }
        });
    }
    // Sequence length is 3 unskipped notes: [60, 67, 71], so cycle wraps immediately to 60!
    assert_eq!(played, vec![60, 67, 71, 60]);
}

#[test]
fn arp_all_strings_skipped_graceful_no_panic() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            mode: ARP,
            octaves: 1,
            arp_pattern: 0,
            arp_skip: 0xFFFF_FFFF, // all strings skipped
            humanize: 0.0,
            ..Config::default()
        },
    );
    e.command(Command::KeyDown(0, 60, 3), &mut |_| {});
    let mut played = Vec::new();
    for _ in 0..10_000 {
        e.tick(&mut |event| {
            if let Out::On(_, note, _) = event {
                played.push(note);
            }
        });
    }
    assert!(played.is_empty());
}

#[test]
fn test_48_voice_capacity_and_stealing() {
    let mut e = Engine::default();
    assert_eq!(e.voices.len(), 48);

    // Allocate 1 Bass voice and 4 Comp voices
    e.allocate_voice(36, 0.9, u64::MAX, true, KIND_BASS, &mut |_| {});
    for note in [60, 64, 67, 71] {
        e.allocate_voice(note, 0.8, u64::MAX, true, KIND_COMP, &mut |_| {});
    }

    // Allocate 43 Arp voices with finite duration
    for i in 0..43 {
        e.allocate_voice(80 + (i % 12) as u8, 0.7, 1000 + i as u64, false, KIND_ARP, &mut |_| {});
    }

    // Total voices is now 1 + 4 + 43 = 48 (at full capacity)
    assert_eq!(e.voices.iter().flatten().count(), 48);

    // Allocating one more voice must steal an Arp voice, NEVER the Bass or Comp voices!
    e.allocate_voice(99, 0.7, 2000, false, KIND_ARP, &mut |_| {});
    assert_eq!(e.voices.iter().flatten().count(), 48);

    assert!(e.voices.iter().flatten().any(|v| v.note == 36 && v.kind == KIND_BASS));
    for note in [60, 64, 67, 71] {
        assert!(e.voices.iter().flatten().any(|v| v.note == note && v.kind == KIND_COMP));
    }
}

#[test]
fn test_adaptive_accents_and_interlock() {
    let mut e = Engine::default();
    let mut config = Config {
        mode: ARP,
        octaves: 1,
        arp_pattern: 0,
        humanize: 0.0,
        ..Config::default()
    };
    // Step 0: Normal (0)
    // Step 1: Accent (1) -> velocity boosted
    // Step 2: Ghost (2) -> velocity reduced
    config.arp_accents[0] = 0;
    config.arp_accents[1] = 1;
    config.arp_accents[2] = 2;
    configure(&mut e, config);

    e.command(Command::KeyDown(0, 60, 3), &mut |_| {});
    let mut vels = Vec::new();
    for _ in 0..100_000 {
        e.tick(&mut |event| {
            if let Out::On(_, _, v) = event {
                vels.push(v);
            }
        });
        if vels.len() >= 3 {
            break;
        }
    }
    assert_eq!(vels.len(), 3);
    assert!(vels[1] > vels[0], "vels[1] = {}, vels[0] = {}", vels[1], vels[0]);
    assert!(vels[2] < vels[0], "vels[2] = {}, vels[0] = {}", vels[2], vels[0]);
}

#[test]
fn test_comp_euclidean_and_dilla_lag() {
    let config = Config {
        comp_euclidean_steps: 8,
        comp_euclidean_hits: 3,
        ..Config::default()
    };
    // 3 hits in 8 steps: steps 0, 3, 6
    assert!(config.comp_is_hit(0));
    assert!(!config.comp_is_hit(1));
    assert!(!config.comp_is_hit(2));
    assert!(config.comp_is_hit(3));
    assert!(!config.comp_is_hit(4));
    assert!(!config.comp_is_hit(5));
    assert!(config.comp_is_hit(6));
    assert!(!config.comp_is_hit(7));
}

#[test]
fn test_single_track_shared_channel_note_off_safety() {
    let mut e = Engine::default();
    configure(&mut e, Config {
        output_channel: 0,
        comp_channel: 0,
        bass_channel: 0,
        ..Config::default()
    });

    // Comp layer holds note 60 on Channel 0
    e.allocate_voice(60, 0.8, u64::MAX, true, KIND_COMP, &mut |_| {});

    // Arp note 60 strikes on Channel 0 with duration 100
    e.strike(60, 0.8, 100, &mut |_| {});

    let mut offs = Vec::new();
    for _ in 0..200 {
        e.tick(&mut |event| {
            if let Out::Off(ch, note, _) = event {
                offs.push((ch, note));
            }
        });
    }

    assert!(!offs.contains(&(0, 60)), "Premature note off emitted on shared channel!");
}

#[test]
fn test_lane_tap_records_distinct_roles() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            output_channel: 0,
            comp_channel: 1,
            bass_channel: 2,
            key_split: true,
            split_note: 70,
            bass_split: 40,
            bass_enabled: true,
            ..Config::default()
        },
    );

    // 1. Bass note (below split)
    e.midi_note(true, 0, 36, 0.8, &mut |_| {});
    // 2. Comp voice
    e.allocate_voice(60, 0.8, u64::MAX, true, KIND_COMP, &mut |_| {});
    // 3. Arp strike
    e.strike(64, 0.8, 100, &mut |_| {});
    // 4. Melody note (above split)
    e.midi_note(true, 0, 72, 0.8, &mut |_| {});

    let on_lanes: Vec<_> = e.tap.events[..e.tap.len]
        .iter()
        .flatten()
        .filter_map(|t| match t.event {
            LaneEvent::On { lane, note, .. } => Some((lane, note)),
            _ => None,
        })
        .collect();

    assert!(on_lanes.contains(&(Lane::Bass, 36)), "Missing Bass On tap");
    assert!(on_lanes.contains(&(Lane::Comp, 60)), "Missing Comp On tap");
    assert!(on_lanes.contains(&(Lane::Arp, 64)), "Missing Arp On tap");
    assert!(on_lanes.contains(&(Lane::Lead, 72)), "Missing Lead On tap");

    // Release bass and melody
    e.midi_note(false, 0, 36, 0.0, &mut |_| {});
    e.midi_note(false, 0, 72, 0.0, &mut |_| {});

    let off_lanes: Vec<_> = e.tap.events[..e.tap.len]
        .iter()
        .flatten()
        .filter_map(|t| match t.event {
            LaneEvent::Off { lane, note, .. } => Some((lane, note)),
            _ => None,
        })
        .collect();

    assert!(off_lanes.contains(&(Lane::Bass, 36)), "Missing Bass Off tap");
    assert!(off_lanes.contains(&(Lane::Lead, 72)), "Missing Lead Off tap");
}

#[test]
fn test_lane_tap_preserves_lanes_on_same_channel() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            output_channel: 0,
            comp_channel: 0,
            bass_channel: 0,
            ..Config::default()
        },
    );

    e.allocate_voice(60, 0.8, u64::MAX, true, KIND_COMP, &mut |_| {});
    e.strike(60, 0.8, 100, &mut |_| {});

    let on_events: Vec<_> = e.tap.events[..e.tap.len]
        .iter()
        .flatten()
        .filter_map(|t| match t.event {
            LaneEvent::On { lane, note, channel, .. } => Some((lane, note, channel)),
            _ => None,
        })
        .collect();

    assert!(on_events.contains(&(Lane::Comp, 60, 0)), "Comp lane On expected");
    assert!(on_events.contains(&(Lane::Arp, 60, 0)), "Arp lane On expected");
}

#[test]
fn nopia_static_mode_harmonizes_and_voices_chords() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            harmonization_mode: 1, // NopiaStatic
            chromatic_flavor: 0,   // SecondaryDominants
            key: 0,                // C
            scale: 0,              // Major
            voice_leading: 0,      // Nearest
            spread: 0,             // Close
            ..Config::default()
        },
    );

    // Play C4 (60) -> C Major triad [60, 64, 67]
    on(&mut e, 60, 1);
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);
    assert_eq!(e.selected_root(), Some(60));
    assert_eq!(e.quality, 0); // Major
    off(&mut e, 60, 1);

    // Play C#4 (61) -> A7 (V7/ii) [57, 61, 64, 67] with root A3 (57)
    on(&mut e, 61, 2);
    assert_eq!(e.quality, 2); // 7th
    assert_eq!(e.selected_root(), Some(57));
    assert!(e.notes.as_slice().contains(&61));
    off(&mut e, 61, 2);

    // Play D4 (62) -> Dm with root D4 (62)
    on(&mut e, 62, 3);
    assert_eq!(e.quality, 1); // Minor
    assert_eq!(e.selected_root(), Some(62));
    off(&mut e, 62, 3);
}

#[test]
fn nopia_modal_interchange_flavor_in_engine() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            harmonization_mode: 1, // NopiaStatic
            chromatic_flavor: 1,   // ModalInterchange
            key: 0,                // C
            scale: 0,              // Major
            ..Config::default()
        },
    );

    // Play Bb3/Bb4 (70) -> Bbmaj7 (bVIImaj7)
    on(&mut e, 70, 1);
    assert_eq!(e.quality, 3); // Maj7
    assert_eq!(e.selected_root().map(|r| r % 12), Some(10)); // Bb
}

#[test]
fn classic_dual_touch_mode_remains_unaffected() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            harmonization_mode: 0, // ClassicDualTouch
            quality: 0,            // Major
            ..Config::default()
        },
    );

    // Single note C4
    on(&mut e, 60, 1);
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);
    assert_eq!(e.selected_root(), Some(60));

    // Dual-touch with D4 (62) alters chord to sus2
    on(&mut e, 62, 2);
    assert_eq!(e.notes.as_slice(), &[60, 62, 67]);
}

#[test]
fn nopia_real_mode_in_engine() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            harmonization_mode: 2, // NopiaReal
            chromatic_flavor: 0,   // SecondaryDominants
            key: 7,                // G
            scale: 0,              // Major
            ..Config::default()
        },
    );

    // Diatonic tonic G4 (67) -> G Major triad [67, 71, 74]
    on(&mut e, 67, 1);
    assert_eq!(e.notes.as_slice(), &[67, 71, 74]);
    assert_eq!(e.selected_root(), Some(67));
    assert_eq!(e.quality, 0); // Major
    off(&mut e, 67, 1);

    // Diatonic supertonic A4 (69) -> Am triad [69, 72, 76]
    on(&mut e, 69, 2);
    assert_eq!(e.notes.as_slice(), &[69, 72, 76]);
    assert_eq!(e.selected_root(), Some(69));
    assert_eq!(e.quality, 1); // Minor
    off(&mut e, 69, 2);

    // Chromatic note C#4 (61) in G Major is #4 -> V7/V (A7) with root A3 (57)
    on(&mut e, 61, 3);
    assert_eq!(e.quality, 2); // 7th
    assert_eq!(e.selected_root().map(|r| r % 12), Some(9)); // A
    assert!(e.notes.as_slice().contains(&61)); // contains C#
    off(&mut e, 61, 3);
}

#[test]
fn nopia_voice_leading_progression_in_engine() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            harmonization_mode: 1, // NopiaStatic
            chromatic_flavor: 0,   // SecondaryDominants
            key: 0,                // C
            scale: 0,              // Major
            voice_leading: 0,      // Nearest resolution
            spread: 0,             // Close
            ..Config::default()
        },
    );

    // Progression: C#4 (yielding A7) -> D4 (yielding Dm)
    on(&mut e, 61, 1);
    let a7_notes = e.notes;
    assert_eq!(e.quality, 2); // 7th
    off(&mut e, 61, 1);

    on(&mut e, 62, 1);
    let dm_notes = e.notes;
    assert_eq!(e.quality, 1); // Minor
    off(&mut e, 62, 1);

    // Nearest voice leading should ensure smooth voice movement
    let base_dm = harmony::voice(62, 1, None, 0, 0, 0);
    let prev_chord = harmony::SavedChord {
        root: 57,
        second: None,
        quality: 2,
        inversion: 0,
        spread: 0,
        transpose: 0,
    };
    let curr_chord = harmony::SavedChord {
        root: 62,
        second: None,
        quality: 1,
        inversion: 0,
        spread: 0,
        transpose: 0,
    };
    let expected_led = harmony::lead(curr_chord, 0, prev_chord, a7_notes, base_dm, false);
    assert_eq!(dm_notes, expected_led);
}

#[test]
fn harmonization_mode_runtime_switch_revoices_held_chord() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            harmonization_mode: 0, // Classic
            quality: 0,            // Major
            ..Config::default()
        },
    );

    // Hold C#4 (61) in Classic mode -> C# Major
    on(&mut e, 61, 1);
    assert_eq!(e.quality, 0);
    assert_eq!(e.notes.as_slice(), &[61, 65, 68]);

    // Switch to NopiaStatic mode at runtime while key is held
    configure(
        &mut e,
        Config {
            harmonization_mode: 1, // NopiaStatic
            chromatic_flavor: 0,   // SecondaryDominants
            key: 0,                // C Major
            scale: 0,
            ..Config::default()
        },
    );

    // Chord should dynamically revoice to A7 (V7/ii) containing C#4 (61)
    assert_eq!(e.quality, 2); // 7th
    assert_eq!(e.selected_root().map(|r| r % 12), Some(9)); // A
    assert!(e.notes.as_slice().contains(&61));
    off(&mut e, 61, 1);
}

#[test]
fn nopia_static_dual_touch_with_non_zero_key() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            harmonization_mode: 1, // NopiaStatic
            chromatic_flavor: 0,   // SecondaryDominants
            key: 7,                // G Major
            scale: 0,              // Major
            voice_leading: 2,      // Off for direct voicing test
            spread: 0,             // Close
            ..Config::default()
        },
    );

    // Play C4 (60) -> G Major triad [67, 71, 74]
    on(&mut e, 60, 1);
    assert_eq!(e.notes.as_slice(), &[67, 71, 74]);
    assert_eq!(e.selected_root(), Some(67));

    // Dual-touch D4 (62) which is +2 semitones from C4 (sus2 gesture)
    // Musician expects Gsus2: [G4 (67), A4 (69), D5 (74)]
    // NOT G5 power chord [67, 74]
    on(&mut e, 62, 2);
    assert_eq!(e.notes.as_slice(), &[67, 69, 74]);
    off(&mut e, 62, 2);

    // After release, reverts back to G Major triad
    assert_eq!(e.notes.as_slice(), &[67, 71, 74]);

    // Dual-touch F4 (65) which is +5 semitones from C4 (sus4 gesture)
    // Musician expects Gsus4: [G4 (67), C5 (72), D5 (74)]
    on(&mut e, 65, 2);
    assert_eq!(e.notes.as_slice(), &[67, 72, 74]);
    off(&mut e, 65, 2);

    // Dual-touch A#4 (70) which is +10 semitones from C4 (dominant 7th gesture)
    // Musician expects G7: [G4 (67), B4 (71), D5 (74), F5 (77)]
    on(&mut e, 70, 2);
    assert_eq!(e.notes.as_slice(), &[67, 71, 74, 77]);
    off(&mut e, 70, 2);

    off(&mut e, 60, 1);
}

#[test]
fn nopia_static_dual_touch_in_key_of_f() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            harmonization_mode: 1, // NopiaStatic
            chromatic_flavor: 0,   // SecondaryDominants
            key: 5,                // F Major
            scale: 0,              // Major
            voice_leading: 2,      // Off
            spread: 0,             // Close
            ..Config::default()
        },
    );

    // C4 (60) -> F Major triad [65, 69, 72]
    on(&mut e, 60, 1);
    assert_eq!(e.notes.as_slice(), &[65, 69, 72]);

    // Dual-touch D4 (62, +2 semitones) -> Fsus2 [65, 67, 72]
    on(&mut e, 62, 2);
    assert_eq!(e.notes.as_slice(), &[65, 67, 72]);
    off(&mut e, 62, 2);

    off(&mut e, 60, 1);
}

#[test]
fn nopia_static_latched_dual_touch() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            harmonization_mode: 1, // NopiaStatic
            chromatic_flavor: 0,   // SecondaryDominants
            key: 7,                // G Major
            scale: 0,              // Major
            latch: true,           // Latch enabled
            spread: 0,             // Close
            voice_leading: 2,
            ..Config::default()
        },
    );

    // Press C4 (60) + D4 (62) for Gsus2
    on(&mut e, 60, 1);
    on(&mut e, 62, 2);
    assert_eq!(e.notes.as_slice(), &[67, 69, 74]);

    // Release both notes
    off(&mut e, 62, 2);
    off(&mut e, 60, 1);

    // Under Latch, Gsus2 remains held and properly voiced
    assert_eq!(e.notes.as_slice(), &[67, 69, 74]);
    assert_eq!(e.selected_root(), Some(67));
}

#[test]
fn nopia_static_boundary_clamping_at_extreme_midi_notes() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            harmonization_mode: 1, // NopiaStatic
            chromatic_flavor: 0,   // SecondaryDominants
            key: 11,               // B Major
            scale: 0,              // Major
            ..Config::default()
        },
    );

    // Play high note 127 (G9) -> White key degree 4 (V = F#)
    on(&mut e, 127, 1);
    assert!(e.selected_root().is_some());
    // Ensure all voiced notes are within valid MIDI range [0..=127]
    for &n in e.notes.as_slice() {
        assert!(n <= 127);
    }
    // Dual-touch with extreme note should not panic or overflow
    on(&mut e, 126, 2);
    for &n in e.notes.as_slice() {
        assert!(n <= 127);
    }
    off(&mut e, 126, 2);
    off(&mut e, 127, 1);
}

#[test]
fn test_comp_guide_tone_isolation() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            comp_mode: 1, // Pad mode
            comp_guide_tone: true,
            comp_channel: 2,
            scale: 0, // Major
            key: 0,   // C
            ..Config::default()
        },
    );

    // Play C4 (60)
    on(&mut e, 60, 1);
    // In C major, degree 0 is C Major triad [60, 64, 67]
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);

    // Check comp layer voices
    let comp_voices: Vec<u8> = e
        .voices
        .iter()
        .flatten()
        .filter(|v| v.kind == KIND_COMP)
        .map(|v| v.note)
        .collect();

    // extract_guide_tone for C Major [60, 64, 67] with root 60 should be the 3rd: 64 (E4)
    assert_eq!(comp_voices, vec![64]);

    // Now switch comp_guide_tone to false
    configure(
        &mut e,
        Config {
            comp_mode: 1,
            comp_guide_tone: false,
            comp_channel: 2,
            scale: 0,
            key: 0,
            ..Config::default()
        },
    );

    // With comp_guide_tone false, comp pad voices should contain all notes of the chord [60, 64, 67]
    let mut comp_voices_all: Vec<u8> = e
        .voices
        .iter()
        .flatten()
        .filter(|v| v.kind == KIND_COMP)
        .map(|v| v.note)
        .collect();
    comp_voices_all.sort();
    assert_eq!(comp_voices_all, vec![60, 64, 67]);
}

#[test]
fn test_engine_extensions_macro() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            latch: true,
            ..Config::default()
        },
    );
    // Default extensions is 0.5 (Triad / 7th)
    on(&mut e, 60, 1);
    assert_eq!(e.notes.as_slice(), &[60, 64, 67]);

    // Set extensions < 0.20 (Root only)
    configure(
        &mut e,
        Config {
            latch: true,
            extensions: 0.1,
            ..Config::default()
        },
    );
    assert_eq!(e.notes.as_slice(), &[60]);

    // Set extensions to 0.30 (Root + 5th)
    configure(
        &mut e,
        Config {
            latch: true,
            extensions: 0.3,
            ..Config::default()
        },
    );
    assert_eq!(e.notes.as_slice(), &[60, 67]);
}

#[test]
fn test_deterministic_major_harmonic_matrix() {
    use crate::harmony::MAJOR_HARMONIC_INTERVALS;
    // Section 1.1 verification:
    assert_eq!(MAJOR_HARMONIC_INTERVALS[0], [0, 4, 7, 11, 14, 17, 21]); // I Maj7
    assert_eq!(MAJOR_HARMONIC_INTERVALS[1], [0, 4, 7, 11, 13, 17, 20]); // bII Neapolitan
    assert_eq!(MAJOR_HARMONIC_INTERVALS[2], [0, 3, 7, 10, 14, 17, 21]); // ii m7
    assert_eq!(MAJOR_HARMONIC_INTERVALS[3], [0, 4, 7, 11, 14, 17, 20]); // bIII Modal Borrow
    assert_eq!(MAJOR_HARMONIC_INTERVALS[4], [0, 3, 7, 10, 13, 17, 20]); // iii m7
    assert_eq!(MAJOR_HARMONIC_INTERVALS[5], [0, 4, 7, 11, 14, 18, 21]); // IV Lydian
    assert_eq!(MAJOR_HARMONIC_INTERVALS[6], [0, 3, 6, 10, 13, 17, 20]); // #IV / bV Half-Dim
    assert_eq!(MAJOR_HARMONIC_INTERVALS[7], [0, 4, 7, 10, 14, 17, 21]); // V Dominant 7th
    assert_eq!(MAJOR_HARMONIC_INTERVALS[8], [0, 4, 7, 11, 14, 18, 20]); // bVI Modal Borrow
    assert_eq!(MAJOR_HARMONIC_INTERVALS[9], [0, 3, 7, 10, 14, 17, 20]); // vi m7
    assert_eq!(MAJOR_HARMONIC_INTERVALS[10], [0, 4, 7, 10, 14, 17, 21]); // bVII Subtonic Dominant
    assert_eq!(MAJOR_HARMONIC_INTERVALS[11], [0, 3, 6, 10, 13, 17, 20]); // vii° Half-Dim
}

#[test]
fn test_deterministic_minor_harmonic_matrix() {
    use crate::harmony::MINOR_HARMONIC_INTERVALS;
    // Section 1.2 verification:
    assert_eq!(MINOR_HARMONIC_INTERVALS[0], [0, 3, 7, 10, 14, 17, 20]); // i m7
    assert_eq!(MINOR_HARMONIC_INTERVALS[1], [0, 4, 7, 11, 13, 17, 20]); // bII Phrygian Major
    assert_eq!(MINOR_HARMONIC_INTERVALS[2], [0, 3, 6, 10, 13, 17, 20]); // ii° Half-Dim
    assert_eq!(MINOR_HARMONIC_INTERVALS[3], [0, 4, 7, 11, 14, 17, 21]); // bIII Relative Major
    assert_eq!(MINOR_HARMONIC_INTERVALS[4], [0, 4, 8, 10, 13, 17, 20]); // III Altered Dominant
    assert_eq!(MINOR_HARMONIC_INTERVALS[5], [0, 3, 7, 10, 14, 17, 21]); // iv m7
    assert_eq!(MINOR_HARMONIC_INTERVALS[6], [0, 3, 6, 9, 13, 16, 20]);  // #IV / bV Dim Substitution
    assert_eq!(MINOR_HARMONIC_INTERVALS[7], [0, 4, 7, 10, 13, 17, 20]); // V Harmonic Dominant
    assert_eq!(MINOR_HARMONIC_INTERVALS[8], [0, 4, 7, 11, 14, 18, 21]); // bVI Major 7th
    assert_eq!(MINOR_HARMONIC_INTERVALS[9], [0, 3, 7, 10, 14, 17, 21]); // VI Dorian Subdominant
    assert_eq!(MINOR_HARMONIC_INTERVALS[10], [0, 4, 7, 10, 14, 17, 21]); // bVII Subtonic Dominant
    assert_eq!(MINOR_HARMONIC_INTERVALS[11], [0, 3, 6, 9, 13, 16, 20]);  // vii° Fully Diminished
}

#[test]
fn test_realtime_bitmask_transition_plan() {
    use crate::engine::{plan_legato_transition, NoteMask};
    // Current chord: C Major {60, 64, 67}
    let current_mask: NoteMask = (1 << 60) | (1 << 64) | (1 << 67);
    // Target chord: A Minor {57, 60, 64} -> 60 and 64 are common tones!
    let target_mask: NoteMask = (1 << 57) | (1 << 60) | (1 << 64);

    let plan = plan_legato_transition(current_mask, target_mask);

    // Common tones (sustain without retrigger): 60, 64
    assert_eq!(plan.sustain_count, 2);
    assert!(plan.notes_to_sustain[..plan.sustain_count].contains(&60));
    assert!(plan.notes_to_sustain[..plan.sustain_count].contains(&64));

    // Released tone: 67
    assert_eq!(plan.release_count, 1);
    assert_eq!(plan.notes_to_release[0], 67);

    // Attacked tone: 57
    assert_eq!(plan.attack_count, 1);
    assert_eq!(plan.notes_to_attack[0], 57);
}

#[test]
fn test_extension_hysteresis_tracker_anti_flamming() {
    use crate::engine::ExtensionHysteresisTracker;
    let mut tracker = ExtensionHysteresisTracker::new(44100.0);
    for _ in 0..2000 {
        tracker.process_parameter(0.5);
    }
    assert_eq!(tracker.current_tier, 0);

    // Push near boundary 1.0 but within hysteresis (1.02 < 1.0 + 0.05)
    for _ in 0..2000 {
        tracker.process_parameter(1.02);
    }
    assert_eq!(tracker.current_tier, 0); // No premature switch!

    // Push beyond hysteresis 1.08 > 1.0 + 0.05
    for _ in 0..2000 {
        tracker.process_parameter(1.08);
    }
    assert_eq!(tracker.current_tier, 1); // Switched cleanly to Tier 1

    // Drop back down slightly (1.02 > 1.0 - 0.05)
    for _ in 0..2000 {
        tracker.process_parameter(1.02);
    }
    assert_eq!(tracker.current_tier, 1); // Stays in Tier 1 without flamming!
}

#[test]
fn test_lookahead_buffer_timing_and_flush() {
    use crate::engine::{LookaheadBuffer, PendingNote};
    let mut buf = LookaheadBuffer::new(1000.0); // 1000 Hz sample rate -> 35ms = 35 samples
    buf.ingest_zone_b(0, 65, 100, 100);
    buf.ingest_zone_b(1, 67, 110, 105);

    // At t = 120 (20 ms elapsed), should NOT flush yet
    let mut out = [PendingNote::default(); 8];
    assert_eq!(buf.drain_expired(120, &mut out), 0);
    assert_eq!(buf.count, 2);

    // Test fast staccato slap cancellation before timer expires:
    assert!(buf.cancel_note(1, 67));
    assert_eq!(buf.count, 1);

    // At t = 140 (40 ms elapsed), 35ms window expired -> note 65 flushes!
    let n = buf.drain_expired(140, &mut out);
    assert_eq!(n, 1);
    assert_eq!(out[0].note, 65);
    assert_eq!(out[0].channel, 0);
    assert_eq!(out[0].velocity, 100);
    assert_eq!(buf.count, 0);
}

#[test]
fn test_muscle_memory_modulo_spread_mapping() {
    use crate::harmony::{map_zone_b_white_key, MAJOR_HARMONIC_INTERVALS};
    let tuple = MAJOR_HARMONIC_INTERVALS[0]; // C Maj7: [0, 4, 7, 11, 14, 17, 21]
    let root = 60; // C4

    // Test structural anchors at low extensions (alpha = 2.0 / Triad), octave_offset = 0
    assert_eq!(map_zone_b_white_key(0, 0, root, &tuple, 2.0), 60); // Key 0 (C) -> Root
    assert_eq!(map_zone_b_white_key(2, 0, root, &tuple, 2.0), 64); // Key 2 (E) -> 3rd (E)
    assert_eq!(map_zone_b_white_key(4, 0, root, &tuple, 2.0), 67); // Key 4 (G) -> 5th (G)
    assert_eq!(map_zone_b_white_key(6, 0, root, &tuple, 2.0), 71); // Key 6 (B) -> 7th (B)

    // Inactive extensions double lower chord tones:
    assert_eq!(map_zone_b_white_key(1, 0, root, &tuple, 2.0), 72); // Key 1 (D) -> Root + 12
    assert_eq!(map_zone_b_white_key(3, 0, root, &tuple, 2.0), 67); // Key 3 (F) -> 5th
    assert_eq!(map_zone_b_white_key(5, 0, root, &tuple, 2.0), 76); // Key 5 (A) -> Octave 3rd

    // Test passing extensions when active (alpha = 3.8 / Upper Colors):
    assert_eq!(map_zone_b_white_key(1, 0, root, &tuple, 3.8), 74); // 9th (D5 = 60 + 14)
    assert_eq!(map_zone_b_white_key(3, 0, root, &tuple, 3.8), 77); // 11th (F5 = 60 + 17)
    assert_eq!(map_zone_b_white_key(5, 0, root, &tuple, 4.0), 81); // 13th (A5 = 60 + 21)

    // Test octave indexing (octave_offset = 1 -> transposed up 12):
    assert_eq!(map_zone_b_white_key(0, 1, root, &tuple, 2.0), 72); // C5
    assert_eq!(map_zone_b_white_key(2, 1, root, &tuple, 2.0), 76); // E5
}

#[test]
fn test_register_boundaries_and_voicing() {
    use crate::harmony::{voice_keys_module, voice_bass_module, KEYS_REGISTER_FLOOR, BASS_REGISTER_CEILING};
    // 4-note chord: Cmaj7 [60, 64, 67, 71]
    let raw = [60, 64, 67, 71];
    let voiced = voice_keys_module(&raw, 0, 0);
    // Keys floor must be >= 52
    for &note in voiced.iter().take(4) {
        assert!(note >= KEYS_REGISTER_FLOOR, "Note {} must be >= {}", note, KEYS_REGISTER_FLOOR);
    }
    // Drop-2 voicing lowers 2nd-from-top (67 -> 55)
    assert!(voiced.contains(&55));

    // Low 4-note chord: F3 [53, 57, 60, 65]
    // Second-from-top is 60. 60 - 12 = 48 (< 52). Pre-scaling transposes chord up by 12 to [65, 69, 72, 77]
    // Then Drop-2 drops 72 -> 60. Final voiced chord: [60, 65, 69, 77], all >= 52 and Drop-2 preserved!
    let low_raw = [53, 57, 60, 65];
    let low_voiced = voice_keys_module(&low_raw, 0, 0);
    for &note in low_voiced.iter().take(4) {
        assert!(note >= KEYS_REGISTER_FLOOR, "Low chord note {} must be >= {}", note, KEYS_REGISTER_FLOOR);
    }
    assert!(low_voiced.contains(&60), "Pre-scaled Drop-2 note 60 must be present");

    // Bass module must be <= 48
    let bass_root = voice_bass_module(0, 4, 7, 11, 0); // Root inversion
    assert!(bass_root <= BASS_REGISTER_CEILING, "Bass note {} must be <= {}", bass_root, BASS_REGISTER_CEILING);
    assert_eq!(bass_root % 12, 0); // Root pitch class C

    let bass_first_inv = voice_bass_module(0, 4, 7, 11, 1); // 1st inversion (3rd in bass)
    assert!(bass_first_inv <= BASS_REGISTER_CEILING);
    assert_eq!(bass_first_inv % 12, 4); // 3rd pitch class E
}

#[test]
fn test_pad_voice_allocation_and_dynamic_hpf() {
    use crate::harmony::{resolve_pad_voices, pad_hpf_cutoff_hz};
    let root = 60;
    // Tier 4: alpha = 3.6 (3rd + extension)
    let p4 = resolve_pad_voices(root, Some(4), Some(7), Some(11), Some(14), 3.6);
    assert_eq!(p4.note_a, 64);
    assert_eq!(p4.note_b, Some(74));

    // Tier 3: alpha = 3.2 (3rd + 7th)
    let p3 = resolve_pad_voices(root, Some(4), Some(7), Some(11), Some(14), 3.2);
    assert_eq!(p3.note_a, 64);
    assert_eq!(p3.note_b, Some(71));

    // Tier 0: alpha = 0.5 (Root +12, Root +24)
    let p0 = resolve_pad_voices(root, None, None, None, None, 0.5);
    assert_eq!(p0.note_a, 72);
    assert_eq!(p0.note_b, Some(84));

    // Dynamic HPF cutoff decreases smoothly as alpha increases
    let hpf_low = pad_hpf_cutoff_hz(0.0);
    let hpf_high = pad_hpf_cutoff_hz(4.0);
    assert!(hpf_low > 350.0);
    assert!(hpf_high < 150.0);
}

#[test]
fn test_mouse_free_root_capture_and_zone_b_dual_core() {
    let mut e = Engine::default();
    configure(
        &mut e,
        Config {
            harmonization_mode: 1, // NopiaStatic
            key_split: true,
            split_note: 60,
            key: 0, // C
            scale: 0, // Major
            extensions: 0.5, // Tier 2 (Triad)
            upper_channel: 1,
            ..Config::default()
        },
    );

    // 1. Dual-Core Zone B with Zone A held
    // Play Zone A root chord on note 48 (C3)
    let mut out_events = Vec::new();
    e.midi_note(true, 0, 48, 0.8, &mut |ev| out_events.push(ev));
    assert!(e.root.is_some());

    // Strike Zone B note 62 (D4 = white key 1 in octave 0). Under C triad, maps to Root + 12 (72).
    out_events.clear();
    e.midi_note(true, 0, 62, 0.8, &mut |ev| out_events.push(ev));
    assert!(out_events.contains(&Out::On(0, 72, 0.8)));
    assert_eq!(e.zone_b_mapped[0][62], Some(72));

    // Strike Zone B note 74 (D5 = white key 1 in octave 1). Maps to 72 + 12 = 84!
    out_events.clear();
    e.midi_note(true, 0, 74, 0.8, &mut |ev| out_events.push(ev));
    assert!(out_events.contains(&Out::On(0, 84, 0.8)));
    assert_eq!(e.zone_b_mapped[0][74], Some(84));

    // Release notes 62 and 74 -> terminates cleanly
    out_events.clear();
    e.midi_note(false, 0, 62, 0.0, &mut |ev| out_events.push(ev));
    e.midi_note(false, 0, 74, 0.0, &mut |ev| out_events.push(ev));
    assert!(out_events.contains(&Out::Off(0, 72, 0.0)));
    assert!(out_events.contains(&Out::Off(0, 84, 0.0)));
    assert_eq!(e.zone_b_mapped[0][62], None);
    assert_eq!(e.zone_b_mapped[0][74], None);

    // Release Zone A chord
    e.midi_note(false, 0, 48, 0.0, &mut |_| {});
    assert!(e.root.is_none());

    // 2. Fast Staccato Slap Edge Case in Lookahead Buffer
    out_events.clear();
    e.midi_note(true, 0, 65, 0.8, &mut |ev| out_events.push(ev));
    assert_eq!(e.lookahead_buffer.count, 1);
    // Released immediately before 35ms expires or Zone A arrives -> cancelled without sounding
    e.midi_note(false, 0, 65, 0.0, &mut |ev| out_events.push(ev));
    assert_eq!(e.lookahead_buffer.count, 0);
    assert!(!out_events.iter().any(|ev| matches!(ev, Out::On(..))));

    // 3. Multi-note Lookahead Strumming Buffer when Zone A is NOT held
    out_events.clear();
    // Glissando / strum across 2 notes before chord arrives: note 60 (C4) and note 62 (D4)
    e.midi_note(true, 0, 60, 0.8, &mut |ev| out_events.push(ev));
    e.midi_note(true, 0, 62, 0.8, &mut |ev| out_events.push(ev));
    assert_eq!(e.lookahead_buffer.count, 2);
    assert!(!out_events.iter().any(|ev| matches!(ev, Out::On(..))));

    // Now strike Zone A chord 48 within 35ms -> both buffered notes flush and retarget in order!
    e.midi_note(true, 0, 48, 0.8, &mut |ev| out_events.push(ev));
    assert_eq!(e.lookahead_buffer.count, 0);
    assert!(out_events.iter().any(|ev| matches!(ev, Out::On(0, 60, _))));
    assert!(out_events.iter().any(|ev| matches!(ev, Out::On(0, 72, _))));

    // Release Zone B notes
    out_events.clear();
    e.midi_note(false, 0, 60, 0.0, &mut |ev| out_events.push(ev));
    e.midi_note(false, 0, 62, 0.0, &mut |ev| out_events.push(ev));
    assert!(out_events.contains(&Out::Off(0, 60, 0.0)));
    assert!(out_events.contains(&Out::Off(0, 72, 0.0)));

    // Release Zone A chord 48
    e.midi_note(false, 0, 48, 0.0, &mut |_| {});

    // 4. Pedal double-tap root capture
    // Reset key to C (0). Hold chord 53 (F3).
    e.config.key = 0;
    e.midi_note(true, 0, 53, 0.8, &mut |_| {});
    assert_eq!(e.selected_root(), Some(53)); // F (5)
    // First pedal press
    e.control(0, 64, 1.0, &mut |_| {});
    // Release pedal
    e.control(0, 64, 0.0, &mut |_| {});
    // Second pedal press within 400ms -> double tap captures F (5)
    e.control(0, 64, 1.0, &mut |_| {});
    assert_eq!(e.config.key, 5); // F captured as scale root!
}
