use super::super::*;
use super::support::*;

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
