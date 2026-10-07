use super::super::*;
use super::support::*;

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
        assert_eq!(last, Some(expected * 7), "tempo {tempo}, sync {sync}");
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
    use super::super::routing::{Route, TARGETS};
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
