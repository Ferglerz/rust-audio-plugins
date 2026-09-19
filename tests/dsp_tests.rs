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
            engine.process_sample(0.5, 0.5, 0.1, 10.0, 1.0, 0.0, false, -18.0, true);
        assert!((out_l - 0.5).abs() < 1e-4);
        assert!((out_r - 0.5).abs() < 1e-4);
    }
    assert_eq!(engine.speed_left(), 1.0);

    // Trigger Note On (Engage Tape Brake)
    engine.note_on(60, 1.0);
    assert!(engine.is_braking());

    // Process through full brake deceleration (0.1s drop time ≈ 78,573 samples)
    for _ in 0..100000 {
        engine.process_sample(0.5, 0.5, 0.1, 10.0, 1.0, 0.0, false, -18.0, true);
    }

    // Speed should have dropped to 0.0
    assert_eq!(engine.speed_left(), 0.0);
    assert_eq!(engine.speed_right(), 0.0);

    // Trigger Note Off (Release Tape Brake -> Crossfade back)
    engine.note_off(60);
    assert!(!engine.is_braking());
    assert!(engine.is_crossfading());

    // Process through crossfade (10ms = 441 samples)
    for _ in 0..1000 {
        engine.process_sample(0.5, 0.5, 0.1, 10.0, 1.0, 0.0, false, -18.0, true);
    }

    // Crossfade should finish and restore normal speed
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
        engine.process_sample(0.5, 0.5, 0.5, 10.0, 1.0, 50.0, false, -18.0, true);
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
        engine.process_sample(0.5, 0.5, 0.5, 10.0, 1.0, 0.0, false, -18.0, true);
    }

    assert!(engine.speed_left() < 1.0);
}

#[test]
fn test_auto_restart_transient_detection() {
    let mut engine = TapeStopEngine::new();
    engine.set_sample_rate(44100.0);

    // Feed silence initially
    for _ in 0..1000 {
        engine.process_sample(0.0, 0.0, 0.5, 10.0, 1.0, 0.0, true, -18.0, true);
    }

    // Engage tape brake
    engine.note_on(60, 1.0);
    assert!(engine.is_braking());

    // Process past the 60ms lockout period with quiet audio (-40 dB ≈ 0.01)
    for _ in 0..4000 {
        engine.process_sample(0.005, 0.005, 0.5, 10.0, 1.0, 0.0, true, -18.0, true);
    }
    assert!(engine.is_braking());

    // Send a loud transient spike (0.8 > -18 dB)
    engine.process_sample(0.8, 0.8, 0.5, 10.0, 1.0, 0.0, true, -18.0, true);

    // Should automatically release the brake and begin crossfading back!
    assert!(!engine.is_braking());
    assert!(engine.is_crossfading());
    assert!(engine.transient_flash() > 0.5);
}
