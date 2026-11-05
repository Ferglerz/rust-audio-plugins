use openwurli_dsp::{CircuitMode, WurliEngine};

fn circuit_test(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn neutral_controls_match_upstream_v090_in_both_circuits() {
    circuit_test(|| {
        let cases: [(CircuitMode, u32, &[u8]); 6] = [
            (
                CircuitMode::Fast,
                44100,
                include_bytes!("fixtures/upstream-0.9.0/fast-44100.f32le"),
            ),
            (
                CircuitMode::Fast,
                48000,
                include_bytes!("fixtures/upstream-0.9.0/fast-48000.f32le"),
            ),
            (
                CircuitMode::Fast,
                96000,
                include_bytes!("fixtures/upstream-0.9.0/fast-96000.f32le"),
            ),
            (
                CircuitMode::Heavy,
                44100,
                include_bytes!("fixtures/upstream-0.9.0/heavy-44100.f32le"),
            ),
            (
                CircuitMode::Heavy,
                48000,
                include_bytes!("fixtures/upstream-0.9.0/heavy-48000.f32le"),
            ),
            (
                CircuitMode::Heavy,
                96000,
                include_bytes!("fixtures/upstream-0.9.0/heavy-96000.f32le"),
            ),
        ];
        for (mode, rate, bytes) in cases {
            let mut engine = WurliEngine::new_with_circuit_mode(rate as f64, mode);
            engine.set_mlp_enabled(true);
            engine.set_speaker_character(0.65);
            engine.set_tremolo_depth(0.7);
            engine.warm_up();
            for note in [36, 60, 64, 79, 96] {
                engine.note_on(note, 0.8);
            }
            let mut actual = Vec::new();
            for block in 0..16 {
                if block == 4 {
                    engine.set_sustain(true);
                    engine.note_off(60);
                }
                if block == 8 {
                    engine.set_sustain(false);
                    engine.note_on(72, 0.5);
                }
                if block == 12 {
                    engine.note_off(36);
                    engine.note_off(96);
                }
                let mut out = [0.0; 256];
                engine.render(&mut out);
                actual.extend(out);
            }
            assert_eq!(actual.len() * 4, bytes.len());
            for (index, (actual, expected)) in actual.iter().zip(bytes.chunks_exact(4)).enumerate()
            {
                let expected = f32::from_le_bytes(expected.try_into().unwrap());
                // Allow cross-platform last-bit differences, not a changed sound model.
                assert!(
                    (actual - expected).abs() <= 1e-7 + expected.abs() * 1e-5,
                    "{mode:?} {rate} Hz sample {index}: {actual} != upstream {expected}"
                );
            }
            assert_eq!(engine.nan_guard_fires(), 0);
        }
    });
}

#[test]
fn live_circuit_switch_preserves_held_notes_at_large_host_block_sizes() {
    circuit_test(|| {
        let mut engine = WurliEngine::new(44100.0);
        engine.ensure_buffer_capacity(16384);
        engine.set_sample_rate(96000.0);
        engine.note_on(60, 0.8);
        for mode in [CircuitMode::Heavy, CircuitMode::Fast] {
            engine.set_circuit_mode(mode);
            let mut audio = vec![0.0; 16384];
            engine.render(&mut audio);
            assert!(audio.iter().all(|s| s.is_finite()));
            assert!(audio.iter().any(|s| s.abs() > 1e-6));
            assert_eq!(engine.held_voice_count(), 1);
            assert_eq!(engine.circuit_mode(), mode);
            assert_eq!(engine.nan_guard_fires(), 0);
        }
        engine.note_off(60);
        assert_eq!(engine.held_voice_count(), 0);
    });
}

#[test]
fn character_controls_reach_the_v090_audio_path() {
    circuit_test(|| {
        for mode in [CircuitMode::Fast, CircuitMode::Heavy] {
            let render = |control| {
                let mut engine = WurliEngine::new_with_circuit_mode(44100.0, mode);
                engine.set_mlp_enabled(true);
                match control {
                    1 => engine.set_hammer_hardness(2.0),
                    2 => engine.set_pickup_drive(0.5),
                    3 => engine.set_tremolo_response(2.0),
                    _ => {}
                }
                engine.note_on(60, 0.8);
                let mut audio = vec![0.0; 8192];
                for block in audio.chunks_mut(256) {
                    engine.render(block);
                }
                assert!(audio.iter().all(|s| s.is_finite()));
                audio
            };
            let neutral = render(0);
            for control in 1..=3 {
                let changed = render(control);
                let difference: f32 = neutral
                    .iter()
                    .zip(changed)
                    .map(|(a, b)| (a - b).abs())
                    .sum();
                assert!(
                    difference > 0.001,
                    "{mode:?} control {control} had no audible effect"
                );
            }
        }
    });
}
