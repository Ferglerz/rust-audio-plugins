use super::super::*;
use super::support::*;

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
