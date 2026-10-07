use super::super::*;
use super::support::*;

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
            strings: 12,
            strings_played: 12,
            octaves: 4,
            arp_pattern: 3,
            ..Config::default()
        },
    );
    on(&mut e, 24, 1);
    assert_eq!(e.scheduled.iter().flatten().count(), 96);
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
