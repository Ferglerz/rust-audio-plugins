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
            strum_ms: 100.0,
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
            strum_ms: 100.0,
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
fn split_channels_do_not_override_mpe_allocation() {
    for mpe in [false, true] {
        let mut e = Engine::default();
        configure(
            &mut e,
            Config {
                mpe,
                split_channels: true,
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
        assert_eq!(channels, if mpe { vec![1, 2, 3] } else { vec![5, 6, 6] });
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
fn synced_sweep_uses_beats_while_free_sweep_keeps_milliseconds() {
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
                strum_beats: 0.5,
                strum_ms: 120.0,
                ..Config::default()
            },
        );
        on(&mut e, 60, 0);
        let last = e.scheduled.iter().flatten().map(|note| note.at).max();
        assert_eq!(last, Some(expected), "tempo {tempo}, sync {sync}");
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
    assert_eq!(e.config.spread, 2);
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
                    strings: 12,
                    strum_ms: 0.0,
                    spread,
                    direction,
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
            for direction in 0..=2 {
                let mut e = Engine::default();
                configure(
                    &mut e,
                    Config {
                        mode: AUTO,
                        strings: total,
                        strings_played: played,
                        direction,
                        strum_ms: 0.0,
                        ..Config::default()
                    },
                );
                on(&mut e, 63, 1);
                let layout = e.snapshot().notes;
                for sweep in 0..2 {
                    if sweep == 1 {
                        e.rebuild(&mut |_| {});
                    }
                    let reverse = direction == 1 || (direction == 2 && sweep == 1);
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
