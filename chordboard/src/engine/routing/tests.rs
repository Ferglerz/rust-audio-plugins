use super::*;
use crate::engine::{Command, ARP, AUTO, MANUAL};

fn target(id: &str) -> u8 {
    TARGETS.iter().position(|t| t.id == id).unwrap() as u8
}
fn route(source: u8, id: &str) -> Route {
    Route {
        source,
        target: target(id),
        ..Route::default()
    }
}
fn ons(events: &[Out]) -> Vec<u8> {
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
fn swing_and_humanize_are_routable_on_both_arpeggiators() {
    let mut e = Engine::default();
    let mut config = Config {
        tracker_enabled: true,
        arp_enabled: true,
        ..Config::default()
    };
    config.routes[0] = route(2, "swing");
    config.routes[1] = route(2, "humanize");
    e.configure(config, &mut |_| {});
    e.source(1, 1.0, &mut |_| {});
    assert_eq!((e.config.swing, e.config.humanize), (0.75, 1.0));
    e.with_performance_arp(|e| {
        assert_eq!((e.config.swing, e.config.humanize), (0.75, 1.0));
    });
}

#[test]
fn bipolar_offsets_preserve_bases_sum_before_clamping_and_never_accumulate() {
    let mut e = Engine::default();
    let mut c = Config {
        transpose: 3,
        velocity: 0.6,
        ..Config::default()
    };
    c.routes[0] = route(10, "transpose");
    c.routes[1] = route(11, "transpose");
    c.routes[2] = route(10, "velocity");
    e.configure(c, &mut |_| {});
    assert_eq!(e.config.transpose, 3);
    assert!((e.config.velocity - 0.6).abs() < 1e-6);
    e.source(9, -0.25, &mut |_| {});
    e.source(10, 0.125, &mut |_| {});
    assert_eq!(e.config.transpose, -3);
    let expected_velocity = TARGETS[target("velocity") as usize]
        .plain(TARGETS[target("velocity") as usize].normalized(&c) - 0.25);
    assert!((e.config.velocity - expected_velocity).abs() < 1e-6);
    for _ in 0..20 {
        e.configure(c, &mut |_| {});
    }
    assert_eq!(e.config.transpose, -3);
    assert_eq!(e.host_config.transpose, 3);
    e.source(9, 0.0, &mut |_| {});
    e.source(10, 0.0, &mut |_| {});
    assert_eq!(e.config.transpose, 3);
    assert!((e.config.velocity - 0.6).abs() < 1e-6);
}

#[test]
fn additive_modulators_offset_live_controllers_and_signed_modulators() {
    let mut e = Engine::default();
    let mut c = Config::default();
    c.routes[0] = route(10, "source_0");
    c.routes[1] = route(10, "source_10");
    e.sources[0] = 0.5;
    e.sources[10] = -0.1;
    e.configure(c, &mut |_| {});
    e.source(9, -0.25, &mut |_| {});
    assert_eq!(e.effective_sources()[0], 0.25);
    assert!((e.effective_sources()[10] + 0.35).abs() < 1e-6);
    assert_eq!(e.sources[0], 0.5);
    assert_eq!(e.sources[10], -0.1);
    e.source(0, 0.6, &mut |_| {});
    assert!((e.source_overrides[0].unwrap() - 0.35).abs() < 1e-6);
    c.routes[0].enabled = false;
    e.configure(c, &mut |_| {});
    assert_eq!(e.effective_sources()[0], 0.6);
}

#[test]
fn smoothing_ramps_in_milliseconds_and_advance_matches_ticks() {
    for source in 1..=SOURCE_COUNT {
        let mut c = Config::default();
        c.routes[0] = Route {
            smoothing_ms: 10.0,
            ..route(source as u8, "velocity")
        };
        let mut e = Engine {
            sample_rate: 1000.0,
            ..Engine::default()
        };
        e.configure(c, &mut |_| {});
        let initial = e.config.velocity;
        let value = if bipolar_source(source - 1) {
            -0.2
        } else {
            0.75
        };
        e.source(source - 1, value, &mut |_| {});
        assert_eq!(e.config.velocity, initial);
        e.advance(5, &mut |_, _| {});
        let midpoint = e.config.velocity;
        let mut reference = Engine {
            sample_rate: 1000.0,
            ..Engine::default()
        };
        reference.configure(c, &mut |_| {});
        reference.source(source - 1, value, &mut |_| {});
        for _ in 0..5 {
            reference.tick(&mut |_| {});
        }
        assert_eq!(midpoint, reference.config.velocity);
        e.advance(5, &mut |_, _| {});
        assert!(!e.smoothing_active());
        let destination = &TARGETS[target("velocity") as usize];
        let norm = if bipolar_source(source - 1) {
            destination.normalized(&c) + value
        } else {
            value
        };
        let expected = destination.plain(norm);
        assert!(
            (e.config.velocity - expected).abs() < 1e-6,
            "source {source}"
        );
        assert!(
            (midpoint - (initial + expected) * 0.5).abs() < 1e-5,
            "source {source}"
        );
        c.routes[0].enabled = false;
        e.configure(c, &mut |_| {});
        assert!(!e.smoothing_active());
        assert_eq!(e.config.velocity, c.velocity);
    }
}

#[test]
fn smoothing_controller_output_is_bounded_and_reaches_the_final_value() {
    let mut e = Engine {
        sample_rate: 48000.0,
        ..Engine::default()
    };
    let mut c = Config {
        mode: MANUAL,
        mpe: false,
        ..Config::default()
    };
    c.routes[0] = Route {
        smoothing_ms: 10.0,
        ..route(10, "source_4")
    };
    e.configure(c, &mut |_| {});
    e.midi_note(true, 0, 60, 0.8, &mut |_| {});
    e.strike(60, 0.8, 1000, &mut |_| {});
    e.source(9, 1.0, &mut |_| {});
    let mut pressure = Vec::new();
    e.advance(480, &mut |_, out| {
        if let Out::Pressure(_, value) = out {
            pressure.push(value);
        }
    });
    assert!(
        pressure.len() > 2 && pressure.len() <= 12,
        "{} pressure events",
        pressure.len()
    );
    assert_eq!(pressure.last(), Some(&1.0));
    assert!(pressure.windows(2).all(|pair| pair[1] >= pair[0]));
}

#[test]
fn routed_velocity_and_axis_sources_affect_performance_without_default_x_route() {
    let mut e = Engine::default();
    let mut c = Config {
        mode: MANUAL,
        ..Config::default()
    };
    c.routes[0] = route(10, "source_6");
    c.routes[1] = route(10, "source_7");
    c.routes[2] = route(10, "source_8");
    e.configure(c, &mut |_| {});
    e.sources[9] = 0.25;
    e.configure(c, &mut |_| {});
    assert!((e.base_strike_velocity() - 1.0).abs() < 0.0001);
    assert_eq!(e.x, 0.25);
    assert_eq!(e.y, 1.0);
    assert_eq!(e.sources[6], 0.8);
}

#[test]
fn routed_midi_modulators_reach_owned_notes_and_restore_live_baselines() {
    let mut e = Engine::default();
    let mut c = Config {
        mode: MANUAL,
        mpe: false,
        ..Config::default()
    };
    c.routes[0] = route(10, "source_4");
    c.routes[1] = route(10, "source_0");
    c.routes[2] = route(10, "source_1");
    e.configure(c, &mut |_| {});
    e.midi_note(true, 0, 60, 0.8, &mut |_| {});
    e.sources[9] = 0.75;
    e.configure(c, &mut |_| {});
    let mut events = Vec::new();
    e.strike(60, 0.8, 1000, &mut |event| events.push(event));
    assert!(events
        .iter()
        .any(|event| matches!(event, Out::Pressure(_, value) if *value == 0.75)));
    assert!(events
        .iter()
        .any(|event| matches!(event, Out::Bend(_, value) if *value == 1.0)));
    assert!(events
        .iter()
        .any(|event| matches!(event, Out::Cc(_, 1, value) if *value == 0.75)));
    assert_eq!(e.expression.pressure, 0.0);
    events.clear();
    c.routes = [Route::default(); ROUTE_COUNT];
    e.configure(c, &mut |event| events.push(event));
    assert!(events
        .iter()
        .any(|event| matches!(event, Out::Pressure(_, value) if *value == 0.0)));
    assert!(events
        .iter()
        .any(|event| matches!(event, Out::Bend(_, value) if *value == 0.5)));
    assert!(events
        .iter()
        .any(|event| matches!(event, Out::Cc(_, 1, value) if *value == 0.0)));
}

#[test]
fn modulator_chains_resolve_independent_of_route_order_and_preserve_inputs() {
    let mut e = Engine::default();
    let mut c = Config::default();
    c.routes = [Route::default(); ROUTE_COUNT];
    c.routes[0] = route(11, "velocity");
    c.routes[1] = route(10, "source_10");
    c.routes[2] = route(2, "source_9");
    e.configure(c, &mut |_| {});
    e.source(1, 0.7, &mut |_| {});
    assert!(e.effective_sources()[9..11]
        .iter()
        .all(|v| (*v - 0.4).abs() < 1e-6));
    assert!((e.config.velocity - 1.0).abs() < 0.0001);
    assert_eq!(e.sources[9..11], [0.0, 0.0]);
    assert!((e.snapshot().sources[10] - 0.4).abs() < 1e-6);
    c.routes[2].enabled = false;
    e.configure(c, &mut |_| {});
    assert_eq!(e.effective_sources()[9..11], [0.0, 0.0]);
}

#[test]
fn circular_modulator_routes_are_bounded_and_do_not_accumulate() {
    let mut e = Engine::default();
    let mut c = Config::default();
    c.routes[0] = route(10, "source_10");
    c.routes[1] = route(11, "source_9");
    e.sources[9] = 0.25;
    e.sources[10] = 0.75;
    e.configure(c, &mut |_| {});
    let expected = e.effective_sources();
    for _ in 0..100 {
        e.configure(c, &mut |_| {});
        assert_eq!(e.effective_sources(), expected);
    }
    assert_eq!(e.sources[9..11], [0.25, 0.75]);
    assert!(!route(10, "source_9").active());
}

#[test]
fn root_on_select_plays_immediately_in_manual_and_releases() {
    let mut e = Engine::default();
    let mut events = Vec::new();
    e.configure(
        Config {
            mode: MANUAL,
            root_on_select: true,
            transpose: 12,
            inversion: 1,
            ..Config::default()
        },
        &mut |v| events.push(v),
    );
    e.midi_note(true, 0, 60, 0.8, &mut |v| events.push(v));
    assert_eq!(ons(&events), vec![72]);
    e.midi_note(false, 0, 60, 0.0, &mut |v| events.push(v));
    assert!(!events.iter().any(|v| matches!(v, Out::Off(_, 72, _))));
    e.now += e.duration();
    e.tick(&mut |v| events.push(v));
    assert!(events.iter().any(|v| matches!(v, Out::Off(_, 72, _))));
    assert!(e.voices.iter().all(Option::is_none));
}

#[test]
fn root_is_not_doubled_by_auto_or_first_arp_step() {
    for mode in [AUTO, ARP] {
        let mut e = Engine::default();
        e.configure(
            Config {
                mode,
                direction: 1,
                root_on_select: true,
                ..Config::default()
            },
            &mut |_| {},
        );
        let mut events = Vec::new();
        e.midi_note(true, 0, 60, 0.8, &mut |v| events.push(v));
        assert_eq!(ons(&events), vec![60]);
        for _ in 0..6000 {
            e.tick(&mut |v| events.push(v));
        }
        assert_eq!(ons(&events).iter().filter(|&&n| n == 60).count(), 1);
    }
}

#[test]
fn disabled_root_preserves_manual_silence() {
    let mut e = Engine::default();
    e.configure(
        Config {
            mode: MANUAL,
            ..Config::default()
        },
        &mut |_| {},
    );
    let mut events = Vec::new();
    e.midi_note(true, 0, 60, 0.8, &mut |v| events.push(v));
    assert!(ons(&events).is_empty());
}

#[test]
fn midi_and_field_sources_are_captured_even_with_legacy_axis_assignments() {
    let mut e = Engine::default();
    e.configure(
        Config {
            mode: MANUAL,
            ..Config::default()
        },
        &mut |_| {},
    );
    e.bend(0, 0.9, &mut |_| {});
    e.control(0, 1, 0.2, &mut |_| {});
    e.control(0, 11, 0.3, &mut |_| {});
    e.control(0, 2, 0.4, &mut |_| {});
    e.pressure(0, None, 0.6, &mut |_| {});
    e.control(0, 74, 0.7, &mut |_| {});
    e.midi_note(true, 0, 60, 0.8, &mut |_| {});
    e.position(0.1, 0, &mut |_| {});
    e.position(0.5, 1, &mut |_| {});
    assert_eq!(
        e.snapshot().sources,
        [0.9, 0.2, 0.3, 0.4, 0.6, 0.7, 0.8, 0.1, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0]
    );
}

#[test]
fn one_controller_drives_octaves_and_spread_and_disable_restores_base() {
    let mut e = Engine::default();
    let mut c = Config {
        mode: MANUAL,
        octaves: 2,
        spread: 1,
        ..Config::default()
    };
    c.routes[0] = route(2, "octaves");
    c.routes[1] = route(2, "spread");
    e.configure(c, &mut |_| {});
    e.midi_note(true, 0, 60, 0.8, &mut |_| {});
    e.control(0, 1, 1.0, &mut |_| {});
    assert_eq!((e.config.octaves, e.config.spread), (4, 4));
    assert_eq!((e.host_config.octaves, e.host_config.spread), (2, 1));
    c.routes[0].enabled = false;
    c.routes[1].enabled = false;
    e.configure(c, &mut |_| {});
    assert_eq!((e.config.octaves, e.config.spread), (2, 1));
}

#[test]
fn legacy_strings_routes_stay_inert() {
    let mut e = Engine::default();
    let mut c = Config {
        strings: 8,
        ..Config::default()
    };
    c.routes[0] = route(2, "strings");
    assert!(!c.routes[0].active());
    assert!(available_targets().all(|(_, t)| t.id != "strings"));
    e.configure(c, &mut |_| {});
    e.control(0, 1, 1.0, &mut |_| {});
    assert_eq!(e.config.strings, 8);
}

#[test]
fn reversed_and_curved_ranges_keep_endpoints() {
    let r = Route {
        min: 0.8,
        max: 0.2,
        curve: 1.0,
        ..route(1, "octaves")
    };
    assert!((r.value(0.0) - 0.8).abs() < 1e-6);
    assert!((r.value(1.0) - 0.2).abs() < 1e-6);
    assert!(r.value(0.5) > 0.7);
    assert!((Route { curve: 0.0, ..r }.value(0.5) - 0.5).abs() < 1e-6);
}

#[test]
fn later_routes_win_and_never_feed_destination_values_back_into_sources() {
    let mut e = Engine::default();
    let mut c = Config::default();
    c.routes[0] = route(2, "octaves");
    c.routes[1] = Route {
        min: 0.0,
        max: 0.0,
        ..route(3, "octaves")
    };
    e.configure(c, &mut |_| {});
    e.control(0, 1, 1.0, &mut |_| {});
    assert_eq!(e.config.octaves, 1);
    assert_eq!(e.sources[1], 1.0);
    assert_eq!(e.sources[2], 0.0);
}

#[test]
fn routing_cannot_resurrect_a_released_chord() {
    let mut e = Engine::default();
    let mut c = Config {
        mode: AUTO,
        ..Config::default()
    };
    c.routes[0] = route(2, "transpose");
    e.configure(c, &mut |_| {});
    e.midi_note(true, 0, 60, 0.8, &mut |_| {});
    e.midi_note(false, 0, 60, 0.0, &mut |_| {});
    let mut events = Vec::new();
    e.control(0, 1, 1.0, &mut |v| events.push(v));
    for _ in 0..100 {
        e.tick(&mut |v| events.push(v));
    }
    assert!(ons(&events).is_empty());
}

#[test]
fn explicit_velocity_route_does_not_square_incoming_velocity() {
    let mut e = Engine::default();
    let mut c = Config {
        mode: MANUAL,
        root_on_select: true,
        ..Config::default()
    };
    c.routes[0] = route(7, "velocity");
    e.configure(c, &mut |_| {});
    let mut events = Vec::new();
    e.midi_note(true, 0, 60, 0.5, &mut |v| events.push(v));
    assert!(events
        .iter()
        .any(|e| matches!(e,Out::On(_,60,v) if (*v-0.505).abs()<1e-5)));
    e.command(Command::Panic, &mut |_| {});
    assert!(e.voices.iter().all(Option::is_none));
}

#[test]
fn modwheel_controls_strings_played_without_changing_total() {
    let mut e = Engine::default();
    let mut c = Config {
        mode: AUTO,
        octaves: 4,
        strum_ms: 0.0,
        strum_sync: false,
        ..Config::default()
    };
    c.routes[0] = route(2, "strings_played");
    e.configure(c, &mut |_| {});
    for (value, count) in [(0.0, 1), (0.5, 7), (1.0, 12)] {
        e.control(0, 1, value, &mut |_| {});
        assert_eq!(e.config.octaves, 4);
        assert_eq!(e.config.strings_played, count);
        e.midi_note(true, 0, 63, 0.8, &mut |_| {});
        let mut events = Vec::new();
        e.tick(&mut |v| events.push(v));
        assert_eq!(ons(&events).len(), count as usize);
        e.midi_note(false, 0, 63, 0.0, &mut |_| {});
    }
}

#[test]
fn routed_x_sweeps_once_and_overrides_legacy_mapping_without_feedback() {
    for source in [2, 4, 8] {
        let mut e = Engine::default();
        let mut c = Config {
            mode: MANUAL,
            octaves: 3,
            ..Config::default()
        };
        c.routes[0] = route(source, "x");
        e.configure(c, &mut |_| {});
        e.midi_note(true, 0, 63, 0.8, &mut |_| {});
        e.position(0.0, 0, &mut |_| {});
        let mut events = Vec::new();
        match source {
            2 => e.control(0, 1, 1.0, &mut |v| events.push(v)),
            4 => e.control(0, 2, 1.0, &mut |v| events.push(v)),
            _ => e.position(1.0, 0, &mut |v| events.push(v)),
        }
        assert_eq!(ons(&events), vec![67, 70, 75, 79, 82, 87, 91, 94]);
        events.clear();
        e.configure(c, &mut |v| events.push(v));
        assert!(ons(&events).is_empty());
        assert_eq!(e.x, 1.0);
        if source == 4 {
            e.control(0, 1, 0.0, &mut |v| events.push(v));
            assert_eq!(e.x, 1.0);
            assert!(ons(&events).is_empty());
        }
    }
}

#[test]
fn strumfield_connection_is_editable_but_direct_input_survives_removal() {
    let mut e = Engine::default();
    let mut c = Config {
        mode: MANUAL,
        ..Config::default()
    };
    c.routes[0].max = 0.5;
    c.routes[0].curve = 1.0;
    e.configure(c, &mut |_| {});
    e.position(0.5, 0, &mut |_| {});
    assert!((e.x - 0.5 * 0.5_f32.powi(8)).abs() < 1e-6);
    assert_eq!(e.sources[7], 0.5);
    e.position(1.0, 0, &mut |_| {});
    assert_eq!(e.x, 0.5);
    c.routes[0].enabled = false;
    e.configure(c, &mut |_| {});
    e.position(0.0, 0, &mut |_| {});
    assert_eq!(e.x, 0.0);
    c.routes[0].source = 0;
    c.routes[0].enabled = true;
    e.configure(c, &mut |_| {});
    e.position(1.0, 0, &mut |_| {});
    assert_eq!(e.x, 1.0);
}

#[test]
fn controllers_cannot_modulate_their_own_input_setup() {
    let mut e = Engine::default();
    let mut c = Config::default();
    for (slot, source, id) in [
        (1, 8, "x_reverse"),
        (2, 9, "y_reverse"),
        (3, 9, "y_target"),
        (4, 9, "y_cc"),
    ] {
        c.routes[slot] = route(source, id);
        assert!(!c.routes[slot].active());
    }
    e.configure(c, &mut |_| {});
    e.position(1.0, 0, &mut |_| {});
    e.position(1.0, 1, &mut |_| {});
    assert_eq!(
        (
            e.config.x_reverse,
            e.config.y_reverse,
            e.config.y_target,
            e.config.y_cc
        ),
        (c.x_reverse, c.y_reverse, c.y_target, c.y_cc)
    );
    assert!(route(8, "x").active());
    assert!(route(2, "x_reverse").active());
}

#[test]
fn touch_bounds_are_not_modulation_destinations() {
    let mut e = Engine::default();
    let mut c = Config::default();
    for (i, id) in ["x_min", "x_max", "y_min", "y_max"].iter().enumerate() {
        c.routes[i] = route(2, id);
        assert!(!c.routes[i].active());
        assert!(!available_targets().any(|(_, t)| t.id == *id));
    }
    e.configure(c, &mut |_| {});
    e.control(0, 1, 1.0, &mut |_| {});
    assert_eq!(
        (
            e.config.x_min,
            e.config.x_max,
            e.config.y_min,
            e.config.y_max
        ),
        (c.x_min, c.x_max, c.y_min, c.y_max)
    );
    assert!(route(2, "x").active());
}

#[test]
fn every_target_round_trips_apply_normalized_and_formats_labels() {
    for target in TARGETS {
        for norm in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
            let label = target.label(norm);
            assert!(!label.is_empty(), "empty label for {}", target.id);
            let mut config = Config::default();
            target.apply(&mut config, norm);
            if target.source_index().is_some() {
                assert_eq!(target.normalized(&config), 0.0);
                continue;
            }
            let expected = ((target.plain(norm) - target.min) / (target.max - target.min))
                .clamp(0.0, 1.0);
            let got = target.normalized(&config);
            assert!(
                (got - expected).abs() < 1.0e-5,
                "round-trip mismatch for {} at norm={norm}: got {got}, expected {expected}",
                target.id
            );
            if target.id == "strum_beats" {
                assert_eq!(config.rate, target.plain(norm));
            }
            if target.id == "direction" {
                assert_eq!(config.arp_pattern, target.plain(norm) as u8);
            }
        }
    }
}
