use tape_stop::dsp::hermite::hermite_interpolate;
use tape_stop::dsp::ring_buffer::StereoRingBuffer;
use tape_stop::dsp::TapeStopEngine;

#[test]
fn test_hermite_interpolation() {
    let y0 = 0.0;
    let y1 = 1.0;
    let y2 = 2.0;
    let y3 = 3.0;

    let mid = hermite_interpolate(y0, y1, y2, y3, 0.5);
    assert!((mid - 1.5).abs() < 1e-5);
}

#[test]
fn test_ring_buffer_push_and_read() {
    let mut buf = StereoRingBuffer::new();
    for i in 0..1000 {
        buf.push(i as f32, -(i as f32));
    }

    assert_eq!(buf.write_head(), 1000);
    assert_eq!(buf.read_left(500.0), 500.0);
    assert_eq!(buf.read_right(500.0), -500.0);
}

#[test]
fn test_tape_stop_braking_and_crossfade() {
    let mut engine = TapeStopEngine::new();
    engine.set_sample_rate(44100.0);

    // Feed 1000 live samples at normal speed
    for _ in 0..1000 {
        let (out_l, out_r) =
            engine.process_sample(0.5, 0.5, 0.1, 10.0, 0.05, 1.0, 0.0, false, -18.0, true);
        assert!((out_l - 0.5).abs() < 1e-4);
        assert!((out_r - 0.5).abs() < 1e-4);
    }
    assert_eq!(engine.speed_left(), 1.0);

    // Trigger Note On (Engage Tape Brake)
    engine.note_on(60, 1.0);
    assert!(engine.is_braking());

    // Process through full brake deceleration (0.1s drop time ≈ 78,573 samples)
    for _ in 0..100000 {
        engine.process_sample(0.5, 0.5, 0.1, 10.0, 0.05, 1.0, 0.0, false, -18.0, true);
    }

    // Speed should have dropped to 0.0
    assert_eq!(engine.speed_left(), 0.0);
    assert_eq!(engine.speed_right(), 0.0);

    // Trigger Note Off (Release Tape Brake -> catch up, then crossfade)
    engine.note_off(60);
    assert!(!engine.is_braking());
    assert!(engine.is_returning());

    for _ in 0..200_000 {
        engine.process_sample(0.5, 0.5, 0.1, 10.0, 0.05, 1.0, 0.0, false, -18.0, true);
        if !engine.is_returning() {
            break;
        }
        assert!(engine.speed_left() <= 8.01);
    }
    assert!(!engine.is_returning());
    assert_eq!(engine.speed_left(), 1.0);

    // DC in and a held sample match, so the 10ms knob is the whole fade.
    for _ in 0..1000 {
        engine.process_sample(0.5, 0.5, 0.1, 10.0, 0.05, 1.0, 0.0, false, -18.0, true);
    }

    assert!(!engine.is_crossfading());
    assert_eq!(engine.speed_left(), 1.0);
}

#[test]
fn test_stereo_divergence() {
    let mut engine = TapeStopEngine::new();
    engine.set_sample_rate(44100.0);

    engine.note_on(60, 1.0);

    // Run with 50% stereo divergence
    for _ in 0..1000 {
        engine.process_sample(0.5, 0.5, 0.5, 10.0, 0.05, 1.0, 50.0, false, -18.0, true);
    }

    // Left and Right channels should have diverged in deceleration speed
    assert_ne!(engine.speed_left(), engine.speed_right());
    assert!(engine.speed_left() < engine.speed_right());
}

#[test]
fn test_14bit_midi_cc_override() {
    let mut engine = TapeStopEngine::new();
    engine.set_sample_rate(44100.0);

    // Send CC #3 MSB and LSB
    engine.handle_midi_cc(3, 64, 3);
    engine.handle_midi_cc(35, 0, 3);

    engine.note_on(60, 1.0);

    for _ in 0..1000 {
        engine.process_sample(0.5, 0.5, 0.5, 10.0, 0.05, 1.0, 0.0, false, -18.0, true);
    }

    assert!(engine.speed_left() < 1.0);
}

#[test]
fn test_high_midi_cc_override() {
    let mut engine = TapeStopEngine::new();
    engine.set_sample_rate(44100.0);
    engine.note_on(60, 1.0);
    engine.handle_midi_cc(74, 127, 74);
    for _ in 0..1000 {
        engine.process_sample(0.5, 0.5, 16.0, 10.0, 0.05, 1.0, 0.0, false, -18.0, true);
    }
    assert!(engine.speed_left() < 0.1);

    engine.handle_midi_cc(74, 0, 74);
    for _ in 0..1000 {
        engine.process_sample(0.5, 0.5, 16.0, 10.0, 0.05, 1.0, 0.0, false, -18.0, true);
    }
    assert!(engine.speed_left() > 0.9);
}

#[test]
fn test_auto_restart_transient_detection() {
    let mut engine = TapeStopEngine::new();
    engine.set_sample_rate(44100.0);

    // Feed silence initially
    for _ in 0..1000 {
        engine.process_sample(0.0, 0.0, 0.5, 10.0, 0.05, 1.0, 0.0, true, -18.0, true);
    }

    // Engage tape brake
    engine.note_on(60, 1.0);
    assert!(engine.is_braking());

    // Process past the 60ms lockout period with quiet audio (-40 dB ≈ 0.01)
    for _ in 0..4000 {
        engine.process_sample(0.005, 0.005, 0.5, 10.0, 0.05, 1.0, 0.0, true, -18.0, true);
    }
    assert!(engine.is_braking());

    // Send a loud transient spike (0.8 > -18 dB)
    engine.process_sample(0.8, 0.8, 0.5, 10.0, 0.05, 1.0, 0.0, true, -18.0, true);

    // Should automatically release the brake and start the catch-up.
    assert!(!engine.is_braking());
    assert!(engine.is_returning());
    assert!(engine.transient_flash() > 0.5);
}

#[test]
fn test_s_curve_deceleration_matches_ui() {
    use tape_stop::dsp::s_curve;

    let mut engine = TapeStopEngine::new();
    engine.set_sample_rate(44100.0);
    engine.note_on(60, 1.0);

    let drop_time = 0.5;
    let curve_exp = 2.5;

    // Process partway through
    for _ in 0..5000 {
        engine.process_sample(
            0.0, 0.0, drop_time, 10.0, 0.05, curve_exp, 0.0, false, -18.0, true,
        );
    }

    let prog_l = engine.brake_progress_l();
    let speed_l = engine.speed_left();
    let expected_speed = 1.0 - s_curve(prog_l, curve_exp);
    assert!((speed_l - expected_speed).abs() < 1e-5);
}

#[test]
fn test_inv_s_curve_inverts_s_curve() {
    use tape_stop::dsp::{inv_s_curve, s_curve};
    for exp in [0.5_f32, 1.0, 2.5, 4.0] {
        for t in [0.0_f32, 0.1, 0.5, 0.73, 1.0] {
            let y = s_curve(t, exp);
            let back = inv_s_curve(y, exp);
            assert!((back - t).abs() < 1e-5, "exp={exp} t={t} y={y} back={back}");
        }
    }
}

#[test]
fn test_live_playhead_stays_on_drop_curve() {
    use tape_stop::dsp::s_curve;
    // Mirrors ui::TapeStopView::live_playhead: y from speed, x from inverse curve.
    let exp = 2.5_f32;
    for t in [0.0_f32, 0.25, 0.5, 0.8, 1.0] {
        let speed = 1.0 - s_curve(t, exp);
        let y_n = 1.0 - speed;
        let x_n = tape_stop::dsp::inv_s_curve(y_n, exp);
        assert!((x_n - t).abs() < 1e-5);
        assert!((y_n - s_curve(x_n, exp)).abs() < 1e-5);
    }
    let overspeed = 1.4_f32;
    let y_n = 1.0 - overspeed.clamp(0.0, 1.0);
    let x_n = tape_stop::dsp::inv_s_curve(y_n, exp);
    assert!(x_n.abs() < 1e-5);
    assert!(y_n.abs() < 1e-5);
}

#[test]
fn test_clear_held_notes_releases_brake() {
    let mut engine = TapeStopEngine::new();
    engine.set_sample_rate(44100.0);
    engine.note_on(60, 1.0);
    assert!(engine.is_braking());

    engine.clear_held_notes();
    assert!(!engine.is_braking());
    assert!(engine.is_returning());
}

fn tone(i: usize) -> f32 {
    (i as f32 * 0.13).sin() * 0.6
}

#[test]
fn test_crossfade_length_follows_correlation() {
    let mut matched = TapeStopEngine::new();
    matched.set_sample_rate(44100.0);
    matched.note_on(60, 1.0);
    for i in 0..80 {
        let s = tone(i);
        matched.process_sample(s, s, 16.0, 50.0, 0.5, 1.0, 0.0, false, -18.0, true);
    }
    matched.note_off(60);
    let s = tone(80);
    matched.process_sample(s, s, 16.0, 50.0, 0.5, 1.0, 0.0, false, -18.0, true);
    let matched_ms = matched.latched_xfade_ms();
    assert!(
        matched_ms > 40.0,
        "matched audio should keep most of the 50ms knob, got {matched_ms}"
    );

    let mut mismatched = TapeStopEngine::new();
    mismatched.set_sample_rate(44100.0);
    mismatched.note_on(60, 1.0);
    for i in 0..80_000 {
        let s = tone(i);
        mismatched.process_sample(s, s, 0.1, 50.0, 0.5, 1.0, 0.0, false, -18.0, true);
    }
    assert!(mismatched.speed_left() < 0.05);
    mismatched.note_off(60);
    let s = tone(80_000);
    mismatched.process_sample(s, -s, 0.1, 50.0, 0.5, 1.0, 0.0, false, -18.0, true);
    let mismatched_ms = mismatched.latched_xfade_ms();
    assert!(
        mismatched_ms < 8.0,
        "stopped tape vs live audio should use the short fade, got {mismatched_ms}"
    );
}

#[test]
fn test_return_meets_write_head_at_unity() {
    let mut engine = TapeStopEngine::new();
    engine.set_sample_rate(44100.0);
    engine.note_on(60, 1.0);
    for i in 0..20_000 {
        let s = tone(i);
        engine.process_sample(s, s, 0.2, 50.0, 0.2, 1.0, 0.0, false, -18.0, true);
    }
    assert!(engine.speed_left() < 0.95);
    engine.note_off(60);

    let mut peak = 0.0f32;
    for i in 0..200_000 {
        let s = tone(20_000 + i);
        engine.process_sample(s, s, 0.2, 50.0, 0.2, 1.0, 0.0, false, -18.0, true);
        if !engine.is_returning() {
            break;
        }
        peak = peak.max(engine.speed_left()).max(engine.speed_right());
        assert!(engine.speed_left() <= 8.01);
        assert!(engine.speed_right() <= 8.01);
    }

    assert!(!engine.is_returning());
    assert!(engine.is_crossfading() || engine.speed_left() == 1.0);
    assert!((engine.speed_left() - 1.0).abs() < 1e-4);
    assert!((engine.speed_right() - 1.0).abs() < 1e-4);
    assert!(
        engine.play_lag_left() < 2.0,
        "lag {}",
        engine.play_lag_left()
    );
    assert!(
        engine.play_lag_right() < 2.0,
        "lag {}",
        engine.play_lag_right()
    );
    assert!(peak > 1.0, "catch-up should exceed realtime, peak {peak}");
}
