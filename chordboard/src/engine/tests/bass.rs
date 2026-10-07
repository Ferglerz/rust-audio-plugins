use super::super::*;
use super::support::*;

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
