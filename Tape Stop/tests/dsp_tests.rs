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
fn prepared_block_processing_matches_compatibility_api_across_updates() {
    let mut prepared_engine = TapeStopEngine::new();
    let mut compatibility_engine = TapeStopEngine::new();
    prepared_engine.set_sample_rate(44100.0);
    compatibility_engine.set_sample_rate(44100.0);
    prepared_engine.note_on(60, 1.0);
    compatibility_engine.note_on(60, 1.0);

    let blocks = [
        (44100.0, 0.12, 0.0, -18.0),
        (44100.0, 0.08, 35.0, -24.0),
        (96000.0, 0.2, -25.0, -12.0),
    ];
    let mut current_sample_rate = 44100.0;
    for (sample_rate, drop_time, stereo_div, threshold) in blocks {
        if sample_rate != current_sample_rate {
            prepared_engine.set_sample_rate(sample_rate);
            compatibility_engine.set_sample_rate(sample_rate);
            current_sample_rate = sample_rate;
        }
        let settings = prepared_engine
            .prepare_block(drop_time, stereo_div, threshold)
            .with_processing_settings(12.0, 0.05, 2.0, true, true);
        for frame in 0..512 {
            let input = (frame as f32 * 0.037).sin() * 0.8;
            let prepared = prepared_engine.process_sample_prepared(
                [input, -input],
                settings,
            );
            let compatibility = compatibility_engine.process_sample(
                input,
                -input,
                drop_time,
                12.0,
                0.05,
                2.0,
                stereo_div,
                true,
                threshold,
                true,
            );
            assert_eq!(prepared, compatibility);
            assert_eq!(prepared_engine.speed_left(), compatibility_engine.speed_left());
            assert_eq!(prepared_engine.speed_right(), compatibility_engine.speed_right());
            assert_eq!(prepared_engine.brake_progress(), compatibility_engine.brake_progress());
            assert_eq!(prepared_engine.is_returning(), compatibility_engine.is_returning());
        }
    }
}

#[test]
fn prepared_processing_matches_legacy_golden_mid_return_and_power_off() {
    // Golden landmarks captured by running this same input sequence through
    // the pre-optimization engine at commit 3009591.
    let mut engine = TapeStopEngine::new();
    engine.set_sample_rate(1000.0);
    let settings_a = engine
        .prepare_block(0.2, 30.0, -18.0)
        .with_processing_settings(40.0, 0.08, 1.7, false, true);
    for i in 0..128 {
        let x = (i as f32 * 0.11).sin() * 0.4;
        engine.process_sample_prepared([x, -x], settings_a);
    }

    engine.note_on(60, 1.0);
    let mut output = (0.0, 0.0);
    for i in 0..80 {
        let x = (i as f32 * 0.17).sin() * 0.7;
        output = engine.process_sample_prepared([x, -x], settings_a);
    }
    assert!((output.0 - 0.5270776).abs() < 1e-6);
    assert!((output.1 + 0.5292928).abs() < 1e-6);
    assert!((engine.brake_progress() - 0.019521978).abs() < 1e-7);
    assert!((engine.brake_progress_l() - 0.026412088).abs() < 1e-7);
    assert!((engine.brake_progress_r() - 0.019521978).abs() < 1e-7);

    engine.note_off(60);
    for i in 0..5 {
        let x = (i as f32 * 0.17).sin() * 0.7;
        output = engine.process_sample_prepared([x, -x], settings_a);
    }
    assert!((output.0 - 0.43330872).abs() < 1e-6);
    assert!((output.1 + 0.4363323).abs() < 1e-6);
    assert!((engine.latched_xfade_ms() - 39.009823).abs() < 1e-5);

    let settings_b = engine
        .prepare_block(0.05, -60.0, -6.0)
        .with_processing_settings(400.0, 1.0, 4.0, false, true);
    for i in 0..16 {
        let x = (i as f32 * 0.13).cos() * 0.6;
        output = engine.process_sample_prepared([x, -x], settings_b);
    }
    assert!((output.0 + 0.21652651).abs() < 1e-6);
    assert!((output.1 - 0.21900803).abs() < 1e-6);
    assert!(engine.is_returning());
    assert!((engine.latched_xfade_ms() - 39.009823).abs() < 1e-5);

    let frozen_progress = (
        engine.brake_progress(),
        engine.brake_progress_l(),
        engine.brake_progress_r(),
    );
    let powered_off = engine
        .prepare_block(20.0, 80.0, -3.0)
        .with_processing_settings(5.0, 0.01, 0.5, true, false);
    assert_eq!(
        engine.process_sample_prepared([0.2, -0.3], powered_off),
        (0.2, -0.3)
    );
    assert_eq!(
        frozen_progress,
        (
            engine.brake_progress(),
            engine.brake_progress_l(),
            engine.brake_progress_r(),
        )
    );
    assert!(engine.is_returning());
    assert_eq!(engine.speed_left(), 1.0);
    assert_eq!(engine.speed_right(), 1.0);
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

#[test]
#[ignore = "manual release timing benchmark; run with --release --ignored --nocapture"]
fn tape_stop_prepared_release_benchmark() {
    use std::hint::black_box;
    use std::time::Instant;

    const SAMPLES: usize = 1_000_000;
    let mut prepared_engine = TapeStopEngine::new();
    let mut compatibility_engine = TapeStopEngine::new();
    prepared_engine.set_sample_rate(48_000.0);
    compatibility_engine.set_sample_rate(48_000.0);
    prepared_engine.note_on(60, 1.0);
    compatibility_engine.note_on(60, 1.0);

    let settings = prepared_engine
        .prepare_block(10.0, 25.0, -18.0)
        .with_processing_settings(12.0, 0.05, 2.0, false, true);
    for _ in 0..4_096 {
        let prepared = prepared_engine.process_sample_prepared([0.31, -0.27], settings);
        let compatibility = compatibility_engine.process_sample(
            0.31, -0.27, 10.0, 12.0, 0.05, 2.0, 25.0, false, -18.0, true,
        );
        assert_eq!(prepared, compatibility);
    }

    let mut prepared_checksum = [0.0_f64; 2];
    let prepared_start = Instant::now();
    for _ in 0..SAMPLES {
        let input = black_box([0.31, -0.27]);
        let output = prepared_engine.process_sample_prepared(input, black_box(settings));
        prepared_checksum[0] += black_box(output.0) as f64;
        prepared_checksum[1] += black_box(output.1) as f64;
    }
    let prepared_elapsed = prepared_start.elapsed();

    let mut compatibility_checksum = [0.0_f64; 2];
    let compatibility_start = Instant::now();
    for _ in 0..SAMPLES {
        let input = black_box([0.31, -0.27]);
        let output = compatibility_engine.process_sample(
            input[0],
            input[1],
            black_box(10.0),
            black_box(12.0),
            black_box(0.05),
            black_box(2.0),
            black_box(25.0),
            black_box(false),
            black_box(-18.0),
            black_box(true),
        );
        compatibility_checksum[0] += black_box(output.0) as f64;
        compatibility_checksum[1] += black_box(output.1) as f64;
    }
    let compatibility_elapsed = compatibility_start.elapsed();

    assert_eq!(prepared_checksum, compatibility_checksum);
    assert_eq!(prepared_engine.speed_left(), compatibility_engine.speed_left());
    assert_eq!(prepared_engine.speed_right(), compatibility_engine.speed_right());
    println!(
        "Tape Stop samples={SAMPLES}: prepared={prepared_elapsed:?}, compatibility={compatibility_elapsed:?}"
    );
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

#[test]
fn test_return_and_crossfade_follow_live_audio() {
    let mut engine = TapeStopEngine::new();
    engine.set_sample_rate(1000.0);
    for i in 0..100 {
        let s = 0.4 + 0.001 * i as f32;
        engine.process_sample(s, -s, 0.01, 50.0, 0.05, 1.0, 0.0, false, -18.0, true);
    }
    engine.note_on(60, 1.0);
    for i in 100..200 {
        let s = 0.4 + 0.001 * i as f32;
        engine.process_sample(s, -s, 0.01, 50.0, 0.05, 1.0, 0.0, false, -18.0, true);
    }
    engine.note_off(60);
    let mut joined = false;
    let mut saw_crossfade = false;
    for i in 200..1000 {
        let s = 0.4 + 0.001 * i as f32;
        let out = engine.process_sample(s, -s, 0.01, 50.0, 0.05, 1.0, 0.0, false, -18.0, true);
        if !engine.is_returning() {
            joined = true;
            saw_crossfade |= engine.is_crossfading();
            assert!(
                (out.0 - s).abs() < 1e-6,
                "Live join L mismatch at {i}: {} versus {s}",
                out.0
            );
            assert!(
                (out.1 + s).abs() < 1e-6,
                "Live join R mismatch at {i}: {} versus {}",
                out.1,
                -s
            );
        }
    }
    assert!(joined && saw_crossfade);
}

#[test]
fn test_hermite_live_edge_uses_recorded_samples() {
    use tape_stop::dsp::ring_buffer::StereoRingBuffer;
    let mut ring = StereoRingBuffer::new();
    for _ in 0..8 {
        ring.push(0.75, -0.25);
    }
    for offset in [0.0, 0.25, 0.5, 0.75] {
        let pos = ring.live_head() as f64 - offset;
        assert!((ring.read_left(pos) - 0.75).abs() < 1e-6);
        assert!((ring.read_right(pos) + 0.25).abs() < 1e-6);
    }
}
