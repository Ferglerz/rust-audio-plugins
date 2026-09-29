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
fn first_release_never_promotes_second() {
    let mut e = Engine::default();
    on(&mut e, 60, 1);
    on(&mut e, 66, 2);
    off(&mut e, 60, 1);
    assert_eq!(e.root.map(|s| s.note), Some(60));
    assert_eq!(e.notes.as_slice(), &[60, 64, 66]);
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
    assert!(notes(&on(&mut e, 37, 1)).is_empty());
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
    assert!(events.contains(&Out::Off(0, 64, 0.0)));
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
        3
    );
}
#[test]
fn released_anchor_cannot_borrow_reused_channel_expression() {
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
    assert_eq!(e.expression.pressure, 0.6);
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
