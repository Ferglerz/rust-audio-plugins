use super::super::*;
use super::support::*;

#[test]
fn input_velocity_drives_length_independently_of_contoured_output() {
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
        assert!((velocity - if mode==ARP {0.4} else {0.2}).abs() < 0.0001, "mode {mode}");
        assert_eq!(e.snapshot().sources[6], 0.8);
        let voice = e.voices.iter().flatten().find(|v| !v.layer).unwrap();
        let expected = if mode == ARP {
            // Arp owns its gate; the manual length route does not override it.
            (125.0 * c.gate) as u64
        } else {
            (20.0 + 0.8 * 2980.0) as u64
        };
        assert_eq!(voice.off - voice.started, expected, "mode {mode}");
        if mode == AUTO {
            assert!(e.scheduled.iter().flatten().count() >= 2);
        }
    }
}
#[test]
fn strumfield_rearticulates_every_string_in_all_selection_modes() {
    for mpe in [false, true] {
        for selection in 0..3 {
            let mut e = Engine { sample_rate: 1000.0, ..Engine::default() };
            configure(&mut e, Config {
                mode: MANUAL,
                mpe,
                key_split: false,
                root_on_select: selection == 1,
                always_chord: selection == 2,
                length_ms: 100.0,
                ..Config::default()
            });
            on(&mut e, 60, 0);
            for _ in 0..2 {
                let mut events = send(&mut e, Command::BeginGesture(0.0, 0.8));
                events.extend(send(&mut e, Command::X(1.0)));
                assert_eq!(notes(&events), [60, 64, 67], "selection {selection}, MPE {mpe}");
                assert_eq!(e.voices.iter().flatten().filter(|v| v.layer).count(), if selection == 2 { 3 } else { 0 });
            }
            off(&mut e, 60, 0);
            tick(&mut e, 101);
            assert!(e.voices.iter().all(Option::is_none), "selection {selection}, MPE {mpe}");
        }
    }
}

#[test]
fn strumfield_plays_without_a_self_route_and_still_drives_modulation() {
    let mut e = Engine::default();
    let mut c = Config {
        mode: MANUAL,
        octaves: 3,
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
        vec![64, 67, 72, 76, 79, 84, 88, 91]
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
                strings_played: 6,
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
    assert!(felt.first().unwrap().at <= 70);
    assert!(felt.last().unwrap().at > plain.last().unwrap().at);
    assert!(felt.last().unwrap().at < 1200);
    assert!(felt[1].at > plain[1].at);
    assert!(felt
        .iter()
        .zip(&plain)
        .any(|(f, p)| (f.velocity - p.velocity).abs() > 0.02));
    assert!(felt
        .iter()
        .zip(&plain)
        .all(|(f, p)| f.velocity >= p.velocity * 0.69 && f.velocity <= (p.velocity * 1.31).min(1.0)));
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
fn once_long_patterns_fit_scheduler_and_played_order_stays_in_range() {
    let mut e = Engine {
        sample_rate: 1000.0,
        ..Engine::default()
    };
    configure(
        &mut e,
        Config {
            mode: AUTO,
            strings: 12,
            strings_played: 12,
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
    for step in &mut e.arp.data.pages[0].steps {step.gate=5;}
    on(&mut e, 60, 1);
    tick(&mut e, 285);
    let snapshot = e.snapshot();
    assert!(snapshot.note_strikes[96] > 0);
    assert!(!snapshot.sounding_notes[96]);
}

#[test]
fn input_velocity_survives_strums_and_tracks_new_input_extensions() {
    let mut e = Engine::default();
    configure(&mut e, Config {
        mode: MANUAL,
        contour: 1.0,
        velocity: 0.4,
        ..Config::default()
    });
    for (note, velocity) in [(60, 0.35), (64, 0.6), (67, 0.9)] {
        e.midi_note(true, 0, note, velocity, &mut |_| {});
        assert_eq!(e.snapshot().sources[6], velocity);
        let mut output = Vec::new();
        for string in 0..3 {
            e.pluck_string(string, &mut |v| output.push(v));
        }
        assert!(output.iter().any(|v| matches!(v, Out::On(..))));
        assert_eq!(e.snapshot().sources[6], velocity);
    }
    e.midi_note(false, 0, 67, 0.1, &mut |_| {});
    assert_eq!(e.snapshot().sources[6], 0.9);
}
