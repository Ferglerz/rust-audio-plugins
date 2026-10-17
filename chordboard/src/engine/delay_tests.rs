use super::*;
use crate::arpeggiator::{source, DelayTap};

fn free(time: f32, feedback: f32) -> DelaySettings {
    let mut config = DelaySettings {
        enabled: true,
        sync: false,
        ..DelaySettings::default()
    };
    config.taps[0].time = time;
    config.taps[0].velocity = 1.0;
    config.taps[0].feedback = feedback;
    config
}

fn capture(runtime: &mut Runtime, config: DelaySettings, token: u64, off: Option<u64>) {
    runtime.capture(
        config,
        source::ARP,
        token,
        60,
        1.0,
        Expression::default(),
        off,
        0,
        0.0,
        1000.0,
        120.0,
    );
}

fn drain(runtime: &mut Runtime, now: u64, beat: f64) -> Vec<Action> {
    let mut result = Vec::new();
    while let Some(action) = runtime.pop_due(now, beat) {
        result.push(action);
        assert!(result.len() <= MAX_ECHOES * 3);
    }
    result
}

fn on(action: Action) -> (u64, u8, f32) {
    match action {
        Action::On {
            token,
            note,
            velocity,
            ..
        } => (token, note, velocity),
        Action::Off { .. } => panic!("expected an echo attack"),
    }
}

#[test]
fn delay_free_time_and_source_duration_are_exact() {
    let mut runtime = Runtime::default();
    capture(&mut runtime, free(10.0, 0.5), 1, Some(7));
    assert_eq!(runtime.deadline(0, 0.0, 500.0), Some(10));
    assert_eq!(runtime.pop_due(9, 0.018), None);
    let first = on(runtime.pop_due(10, 0.02).unwrap());
    assert_eq!((first.1, first.2), (60, 1.0));
    assert_eq!(runtime.deadline(10, 0.02, 500.0), Some(17));
    assert_eq!(
        runtime.pop_due(17, 0.034),
        Some(Action::Off { token: first.0 })
    );
    let second = on(runtime.pop_due(20, 0.04).unwrap());
    assert_eq!(second.2, 0.5);
    assert_eq!(
        runtime.pop_due(27, 0.054),
        Some(Action::Off { token: second.0 })
    );
}

#[test]
fn delay_synced_attack_and_release_follow_tempo_changes() {
    let mut runtime = Runtime::default();
    let mut config = free(0.5, 0.0);
    config.sync = true;
    runtime.capture(
        config,
        source::ARP,
        1,
        60,
        1.0,
        Expression::default(),
        Some(6000),
        0,
        0.0,
        48_000.0,
        120.0,
    );
    assert_eq!(runtime.deadline(0, 0.0, 24_000.0), Some(12_000));
    // The tempo halves at beat 1/4. Both echo boundaries remain in beats.
    assert_eq!(runtime.deadline(6000, 0.25, 48_000.0), Some(18_000));
    let first = on(runtime.pop_due(18_000, 0.5).unwrap());
    assert_eq!(runtime.deadline(18_000, 0.5, 48_000.0), Some(30_000));
    assert_eq!(
        runtime.pop_due(30_000, 0.75),
        Some(Action::Off { token: first.0 })
    );
    assert_eq!(runtime.deadline(30_000, 0.75, 48_000.0), None);
}

#[test]
fn delay_taps_have_independent_feedback_and_immutable_attack_settings() {
    let mut runtime = Runtime::default();
    let mut config = free(10.0, 0.5);
    config.count = 2;
    config.taps[0].velocity = 0.8;
    config.taps[0].pitch = 12;
    config.taps[0].fine = 37.0;
    config.taps[0].pan = -0.3;
    config.taps[0].channel = 9;
    config.taps[1] = DelayTap {
        time: 15.0,
        velocity: 0.4,
        feedback: 0.0,
        pitch: -12,
        fine: -21.0,
        pan: 0.7,
        channel: 4,
        ..DelayTap::default()
    };
    let expression = Expression {
        pressure: 0.45,
        timbre: 0.8,
        bend: 0.3,
    };
    runtime.capture(
        config,
        source::ARP,
        1,
        60,
        1.0,
        expression,
        Some(2),
        0,
        0.0,
        1000.0,
        120.0,
    );
    config.taps[0].pitch = -24;
    config.taps[0].feedback = 0.0;
    config.enabled = false;
    assert!(!config.enabled);
    let first = runtime.pop_due(10, 0.02).unwrap();
    assert!(
        matches!(first, Action::On { note: 72, expression: e, fine: 37.0, pan: -0.3, channel: 9, .. } if e == expression)
    );
    assert_eq!(on(first).2, 0.8);
    drain(&mut runtime, 12, 0.024);
    let second = runtime.pop_due(15, 0.03).unwrap();
    assert!(
        matches!(second, Action::On { note: 48, expression: e, fine: -21.0, pan: 0.7, channel: 4, .. } if e == expression)
    );
    assert_eq!(on(second).2, 0.4);
    drain(&mut runtime, 17, 0.034);
    let repeat = on(runtime.pop_due(20, 0.04).unwrap());
    assert_eq!((repeat.1, repeat.2), (72, 0.4));
}

#[test]
fn delay_release_before_first_attack_supplies_its_duration() {
    let mut runtime = Runtime::default();
    capture(&mut runtime, free(10.0, 0.0), 3, None);
    runtime.release(3, 7, 0.014);
    let first = on(runtime.pop_due(10, 0.02).unwrap());
    assert_eq!(runtime.deadline(10, 0.02, 500.0), Some(17));
    assert_eq!(
        runtime.pop_due(17, 0.034),
        Some(Action::Off { token: first.0 })
    );
}

#[test]
fn delay_release_after_attack_preserves_overlapping_note_lengths() {
    let mut runtime = Runtime::default();
    capture(&mut runtime, free(10.0, 0.5), 4, None);
    let first = on(runtime.pop_due(10, 0.02).unwrap());
    runtime.release(4, 15, 0.03);
    let second = on(runtime.pop_due(20, 0.04).unwrap());
    assert_eq!(
        runtime.pop_due(25, 0.05),
        Some(Action::Off { token: first.0 })
    );
    let third = on(runtime.pop_due(30, 0.06).unwrap());
    assert_eq!(
        runtime.pop_due(35, 0.07),
        Some(Action::Off { token: second.0 })
    );
    assert_ne!(third.0, second.0);
}

#[test]
fn delay_release_uses_actual_beats_for_held_synced_notes() {
    let mut runtime = Runtime::default();
    let mut config = free(0.5, 0.0);
    config.sync = true;
    runtime.capture(
        config,
        source::ARP,
        8,
        60,
        1.0,
        Expression::default(),
        None,
        0,
        0.0,
        48_000.0,
        120.0,
    );
    let first = on(runtime.pop_due(12_000, 0.5).unwrap());
    runtime.release(8, 18_000, 0.625);
    assert_eq!(runtime.deadline(18_000, 0.625, 48_000.0), Some(42_000));
    assert_eq!(
        runtime.pop_due(42_000, 1.125),
        Some(Action::Off { token: first.0 })
    );
}

#[test]
fn delay_actual_early_release_overrides_planned_off_once() {
    let mut runtime = Runtime::default();
    capture(&mut runtime, free(10.0, 0.5), 7, Some(100));
    runtime.release(7, 5, 0.01);
    runtime.release(7, 50, 0.1);
    let first = on(runtime.pop_due(10, 0.02).unwrap());
    assert_eq!(runtime.deadline(10, 0.02, 500.0), Some(15));
    assert_eq!(
        runtime.pop_due(15, 0.03),
        Some(Action::Off { token: first.0 })
    );
}

#[test]
fn delay_source_filter_and_power_are_independent_of_the_arp() {
    for selected in [
        source::ARP,
        source::STRUM,
        source::CHORDS,
        source::BASS,
        source::MELODY,
    ] {
        let mut runtime = Runtime::default();
        let mut config = free(10.0, 0.0);
        config.sources = selected;
        for candidate in [
            source::ARP,
            source::STRUM,
            source::CHORDS,
            source::BASS,
            source::MELODY,
        ] {
            runtime.capture(
                config,
                candidate,
                u64::from(candidate),
                60,
                1.0,
                Expression::default(),
                Some(2),
                0,
                0.0,
                1000.0,
                120.0,
            );
        }
        assert_eq!(drain(&mut runtime, 10, 0.02).len(), 1);
        runtime.clear();
        config.enabled = false;
        capture(&mut runtime, config, 100, Some(2));
        assert_eq!(runtime.deadline(0, 0.0, 500.0), None);
    }
}

#[test]
fn delay_ping_pong_alternates_each_tap_without_accumulating_pitch() {
    let mut runtime = Runtime::default();
    let mut config = free(10.0, 0.5);
    config.count = 2;
    config.ping_pong = true;
    config.taps[0].pitch = 12;
    config.taps[1] = config.taps[0];
    capture(&mut runtime, config, 1, Some(2));
    let first = drain(&mut runtime, 10, 0.02);
    assert!(matches!(
        first[0],
        Action::On {
            note: 72,
            pan: -1.0,
            ..
        }
    ));
    assert!(matches!(
        first[1],
        Action::On {
            note: 72,
            pan: 1.0,
            ..
        }
    ));
    drain(&mut runtime, 12, 0.024);
    let second = drain(&mut runtime, 20, 0.04);
    assert!(matches!(
        second[0],
        Action::On {
            note: 72,
            pan: 1.0,
            ..
        }
    ));
    assert!(matches!(
        second[1],
        Action::On {
            note: 72,
            pan: -1.0,
            ..
        }
    ));
}

#[test]
fn delay_feedback_threshold_and_generation_cap_produce_finite_tails() {
    let mut runtime = Runtime::default();
    capture(&mut runtime, free(10.0, 0.5), 1, Some(2));
    let mut velocities = Vec::new();
    while let Some(at) = runtime.deadline(0, 0.0, 500.0) {
        for action in drain(&mut runtime, at, at as f64 / 500.0) {
            if let Action::On { velocity, .. } = action {
                velocities.push(velocity);
            }
        }
    }
    assert_eq!(velocities.len(), 7);
    assert!(velocities.iter().all(|velocity| *velocity >= 1.0 / 127.0));
    capture(&mut runtime, free(10.0, 1.0), 2, Some(2));
    let mut attacks = 0;
    let mut releases = 0;
    while let Some(at) = runtime.deadline(0, 0.0, 500.0) {
        for action in drain(&mut runtime, at, at as f64 / 500.0) {
            match action {
                Action::On { .. } => attacks += 1,
                Action::Off { .. } => releases += 1,
            }
        }
    }
    assert_eq!((attacks, releases), (64, 64));
}

#[test]
fn delay_zero_feedback_is_one_echo_and_low_velocity_is_silent() {
    let mut runtime = Runtime::default();
    capture(&mut runtime, free(10.0, 0.0), 1, Some(2));
    let first = on(runtime.pop_due(10, 0.02).unwrap());
    assert_eq!(
        runtime.pop_due(12, 0.024),
        Some(Action::Off { token: first.0 })
    );
    assert_eq!(runtime.deadline(12, 0.024, 500.0), None);
    let mut config = free(10.0, 0.5);
    config.taps[0].velocity = 0.007;
    capture(&mut runtime, config, 2, Some(2));
    assert_eq!(runtime.deadline(0, 0.0, 500.0), None);

    let config = free(10.0, 0.0);
    runtime.capture(
        config,
        source::ARP,
        3,
        60,
        1.0 / 127.0,
        Expression::default(),
        Some(2),
        0,
        0.0,
        1000.0,
        120.0,
    );
    assert_eq!(on(runtime.pop_due(10, 0.02).unwrap()).2, 1.0 / 127.0);
}

#[test]
fn delay_gain_clipping_does_not_change_feedback_gain() {
    let mut runtime = Runtime::default();
    let mut config = free(10.0, 0.5);
    config.taps[0].velocity = 2.0;
    runtime.capture(
        config,
        source::ARP,
        1,
        60,
        0.8,
        Expression::default(),
        Some(2),
        0,
        0.0,
        1000.0,
        120.0,
    );
    assert_eq!(on(runtime.pop_due(10, 0.02).unwrap()).2, 1.0);
    drain(&mut runtime, 12, 0.024);
    assert_eq!(on(runtime.pop_due(20, 0.04).unwrap()).2, 0.8);
}

#[test]
fn delay_zero_length_notes_emit_their_reserved_off() {
    let mut runtime = Runtime::default();
    capture(&mut runtime, free(10.0, 0.0), 1, Some(0));
    let first = on(runtime.pop_due(10, 0.02).unwrap());
    assert_eq!(
        runtime.pop_due(10, 0.02),
        Some(Action::Off { token: first.0 })
    );
    assert_eq!(runtime.pop_due(10, 0.02), None);
}

#[test]
fn delay_note_offs_win_simultaneous_attack_deadlines() {
    let mut runtime = Runtime::default();
    capture(&mut runtime, free(10.0, 0.5), 1, Some(10));
    let first = on(runtime.pop_due(10, 0.02).unwrap());
    assert_eq!(
        runtime.pop_due(20, 0.04),
        Some(Action::Off { token: first.0 })
    );
    assert!(matches!(runtime.pop_due(20, 0.04), Some(Action::On { .. })));
}

#[test]
fn delay_full_capacity_drops_attacks_without_losing_pending_releases() {
    let mut runtime = Runtime::default();
    let config = free(1.0, 0.0);
    for batch in 0..2_u64 {
        for index in 0..MAX_TRAINS {
            runtime.capture(
                config,
                source::ARP,
                batch * MAX_TRAINS as u64 + index as u64,
                60,
                1.0,
                Expression::default(),
                None,
                batch,
                batch as f64 / 500.0,
                1000.0,
                120.0,
            );
        }
        assert_eq!(
            drain(&mut runtime, batch + 1, (batch + 1) as f64 / 500.0).len(),
            MAX_TRAINS
        );
    }
    assert_eq!(runtime.echoes.iter().flatten().count(), MAX_ECHOES);
    for index in 0..MAX_TRAINS {
        runtime.capture(
            config,
            source::ARP,
            10_000 + index as u64,
            60,
            1.0,
            Expression::default(),
            None,
            2,
            0.004,
            1000.0,
            120.0,
        );
    }
    assert_eq!(runtime.pop_due(3, 0.006), None);
    assert_eq!(runtime.trains.iter().flatten().count(), 0);
    for token in 0..MAX_ECHOES as u64 {
        runtime.release(token, 4, 0.008);
    }
    let releases = drain(&mut runtime, 5, 0.01);
    assert_eq!(releases.len(), MAX_ECHOES);
    assert!(releases
        .iter()
        .all(|action| matches!(action, Action::Off { .. })));
    assert_eq!(runtime.deadline(5, 0.01, 500.0), None);
}

#[test]
fn delay_train_capacity_never_overwrites_existing_sources() {
    let mut runtime = Runtime::default();
    let config = free(10.0, 0.0);
    for token in 0..=MAX_TRAINS as u64 {
        capture(&mut runtime, config, token, Some(2));
    }
    assert_eq!(drain(&mut runtime, 10, 0.02).len(), MAX_TRAINS);
    assert_eq!(drain(&mut runtime, 12, 0.024).len(), MAX_TRAINS);
    assert_eq!(runtime.deadline(12, 0.024, 500.0), None);
}

fn rendered(block_size: u64, sync: bool) -> Vec<(u64, Action)> {
    let mut runtime = Runtime::default();
    let mut config = free(if sync { 0.03125 } else { 1.25 }, 0.95);
    config.sync = sync;
    capture(&mut runtime, config, 1, Some(25));
    let mut result = Vec::new();
    let mut block = 0;
    while block < 1200 {
        let end = (block + block_size).min(1200);
        let mut now = block;
        while let Some(at) = runtime.deadline(now, now as f64 / 500.0, 500.0) {
            if at >= end {
                break;
            }
            now = at;
            let actions = drain(&mut runtime, now, now as f64 / 500.0);
            assert!(
                !actions.is_empty(),
                "a deadline must produce or discard an event"
            );
            result.extend(actions.into_iter().map(|action| (now, action)));
        }
        block = end;
    }
    result
}

#[test]
fn delay_fractional_deadlines_are_block_size_independent() {
    for sync in [false, true] {
        let reference = rendered(1, sync);
        assert_eq!(reference.len(), 128);
        for block in [7, 64, 257, 4096] {
            assert_eq!(rendered(block, sync), reference);
        }
        if !sync {
            let attacks: Vec<_> = reference
                .iter()
                .filter_map(|(at, action)| matches!(action, Action::On { .. }).then_some(*at))
                .collect();
            assert_eq!(&attacks[..4], &[2, 3, 4, 5]);
            assert_eq!(attacks.last(), Some(&80));
        }
    }
}

#[test]
fn delay_clear_discards_tails_and_does_not_reuse_voice_tokens() {
    let mut runtime = Runtime::default();
    capture(&mut runtime, free(10.0, 0.5), 1, Some(2));
    let first = on(runtime.pop_due(10, 0.02).unwrap()).0;
    runtime.clear();
    assert_eq!(runtime.deadline(10, 0.02, 500.0), None);
    capture(&mut runtime, free(10.0, 0.0), 2, Some(2));
    assert_ne!(on(runtime.pop_due(10, 0.02).unwrap()).0, first);
}

#[test]
fn delay_malformed_inputs_and_disabled_taps_cannot_strand_notes() {
    let mut runtime = Runtime::default();
    let mut config = free(10.0, 0.0);
    config.taps[0].enabled = false;
    capture(&mut runtime, config, 1, Some(2));
    assert_eq!(runtime.deadline(0, 0.0, 500.0), None);
    config.taps[0].enabled = true;
    config.taps[0].pitch = 24;
    runtime.capture(
        config,
        source::ARP,
        2,
        120,
        1.0,
        Expression::default(),
        Some(2),
        0,
        0.0,
        1000.0,
        120.0,
    );
    assert_eq!(runtime.deadline(0, 0.0, 500.0), None);
    config.sync = true;
    config.taps[0].pitch = 0;
    config.taps[0].time = 0.5;
    runtime.capture(
        config,
        source::ARP,
        3,
        60,
        1.0,
        Expression::default(),
        Some(2),
        0,
        f64::NAN,
        1000.0,
        120.0,
    );
    assert_eq!(runtime.deadline(0, f64::NAN, 500.0), Some(250));
    let first = on(runtime.pop_due(250, f64::NAN).unwrap());
    assert_eq!(
        runtime.pop_due(252, f64::NAN),
        Some(Action::Off { token: first.0 })
    );
}
