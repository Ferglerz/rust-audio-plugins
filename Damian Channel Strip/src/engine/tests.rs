use super::*;
fn isolated_shared() -> Arc<Shared> {
    let shared = Shared::new(
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
    );
    shared.stop.store(true, Ordering::Relaxed);
    if let Some(worker) = shared.worker.lock().unwrap().take() {
        worker.join().unwrap();
    }
    while shared.pending.pop().is_some() {}
    while shared.pending_aux.pop().is_some() {}
    shared
}

#[test]
fn hidden_display_preserves_audio_peaks_and_meter_cadence_and_reopens_fresh() {
    let visible_shared = isolated_shared();
    let hidden_shared = isolated_shared();
    let mut visible = Engine::new(visible_shared.clone(), 48000.0);
    let mut hidden = Engine::new(hidden_shared.clone(), 48000.0);
    let settings = CompSettings::default();
    let frozen: [f32; 128] =
        std::array::from_fn(|i| hidden_shared.spectrum[i].load(Ordering::Relaxed));
    hidden.set_display_enabled(false);
    for i in 0..5000 {
        let input = [(i as f64 * 0.13).sin() * 0.7, (i as f64 * 0.17).cos() * 0.5];
        assert_eq!(
            visible.tick(input, settings, true, true, true, false, true, false, false),
            hidden.tick(input, settings, true, true, true, false, true, false, false),
        );
        assert_eq!(visible.position, hidden.position);
        assert_eq!(
            (visible.in_peak, visible.out_peak, visible.wall_peak),
            (hidden.in_peak, hidden.out_peak, hidden.wall_peak)
        );
        for (left, right) in [
            (&visible_shared.input, &hidden_shared.input),
            (&visible_shared.output, &hidden_shared.output),
            (&visible_shared.gr, &hidden_shared.gr),
            (&visible_shared.gr_uncapped, &hidden_shared.gr_uncapped),
            (&visible_shared.pse_gr, &hidden_shared.pse_gr),
            (&visible_shared.sc_level, &hidden_shared.sc_level),
            (&visible_shared.wall_level, &hidden_shared.wall_level),
        ] {
            assert_eq!(
                left.load(Ordering::Relaxed),
                right.load(Ordering::Relaxed),
                "sample {i}"
            );
        }
    }
    assert_eq!(hidden.spectrum_position, 0);
    for (index, expected) in frozen.into_iter().enumerate() {
        assert_eq!(
            hidden_shared.spectrum[index].load(Ordering::Relaxed),
            expected
        );
    }
    // Hide after a partial live window, then reopen: pre-close samples must not
    // enter the new display frame, nor may reopening move the meter clock.
    hidden.set_display_enabled(true);
    for _ in 0..53 {
        hidden.tick(
            [0.9, -0.9],
            settings,
            true,
            true,
            true,
            false,
            true,
            false,
            false,
        );
    }
    hidden.set_display_enabled(false);
    hidden.set_display_enabled(true);
    let meter_position = hidden.position;
    for i in 0..2048 {
        hidden.tick(
            [0.0; 2], settings, true, true, true, false, true, false, false,
        );
        assert_eq!(hidden.position, (meter_position + i + 1) % 2048);
        assert!(hidden_shared
            .spectrum
            .iter()
            .all(|bin| bin.load(Ordering::Relaxed) == -90.0));
    }
    assert_eq!(hidden.spectrum_position, 0);
}

#[test]
fn visible_spectrum_matches_original_window_fft_and_peak_hold() {
    let shared = isolated_shared();
    let mut engine = Engine::new(shared.clone(), 48000.0);
    let mut samples = vec![Complex::default(); 2048];
    let mut expected: [f32; 128] =
        std::array::from_fn(|i| shared.spectrum[i].load(Ordering::Relaxed));
    for frame in 0..3 {
        for (i, sample) in samples.iter_mut().enumerate() {
            let t = (frame * 2048 + i) as f64;
            let input = [(t * 0.13).sin() * 0.7, (t * 0.17).cos() * 0.5];
            let loudest = if input[0].abs() > input[1].abs() {
                input[0]
            } else {
                input[1]
            };
            *sample = Complex::new(loudest as f32 * engine.window[i], 0.0);
            engine.tick(
                input,
                CompSettings::default(),
                true,
                true,
                true,
                false,
                true,
                false,
                false,
            );
            if i < 2047 {
                for (bin, &value) in shared.spectrum.iter().zip(&expected) {
                    assert_eq!(bin.load(Ordering::Relaxed), value);
                }
            }
        }
        engine
            .fft
            .process_with_scratch(&mut samples, &mut engine.scratch);
        for (i, value) in expected.iter_mut().enumerate() {
            let (start, end) = engine.spectrum_bin_ranges[i];
            let mut magnitude = 0.0_f32;
            if start <= end {
                for bin in start.max(1)..=end {
                    magnitude = magnitude.max(samples[bin].norm() / 512.0);
                }
            }
            let db = (20.0 * magnitude.max(0.0000316).log10()).max(-90.0);
            *value = peak_hold(*value, db, SPECTRUM_FALL_DB_PER_FRAME);
            assert_eq!(shared.spectrum[i].load(Ordering::Relaxed), *value);
        }
    }
}

#[test]
fn wall_clips_with_zero_harmonics_in_the_audio_path() {
    let mut engine = Engine::new(isolated_shared(), 48000.0);
    let settings = CompSettings {
        wall: crate::dsp::WallSettings {
            threshold: -12.0,
            ..Default::default()
        },
        ..CompSettings::default()
    };
    let ceiling = 10.0_f64.powf(-12.0 / 20.0);
    for input in [[0.8, -0.4], [-2.0, 2.0], [0.0, 0.0]] {
        let output = engine.tick(
            input, settings, false, false, false, false, false, false, false,
        );
        for channel in 0..2 {
            let expected = ceiling * (input[channel] / ceiling).tanh();
            assert!((output[channel] - expected).abs() < 1.0e-12);
        }
    }
}

#[test]
fn linear_independent_bank_bypasses_match_single_bank_filters() {
    use crate::processing::{EqPath, Resolution};
    let sr = 44100.0;
    let config = Config {
        mode: ProcessingMode::LinearPhase,
        resolution: Resolution::Low,
    };
    let first = Band {
        id: 1,
        gain: 9.0,
        freq: 1000.0,
        dynamic: true,
        ..Band::default()
    };
    let second = Band {
        id: 2,
        gain: -6.0,
        freq: 3000.0,
        ..Band::default()
    };
    for (eq1, eq2) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut bank = Bank::new(
            std::slice::from_ref(&first),
            std::slice::from_ref(&second),
            sr,
            config,
        );
        let mut enabled = Vec::new();
        if eq1 {
            enabled.push(first.clone());
        }
        if eq2 {
            enabled.push(second.clone());
        }
        let mut reference = EqPath::new(&enabled, sr, config);
        for i in 0..12000 {
            let input = [(i as f64 * 0.13).sin(), (i as f64 * 0.07).cos()];
            let (actual, _) = bank.tick(input, f64::from(eq1), f64::from(eq2));
            let expected = reference.tick(input, &mut [], sr);
            for ch in 0..2 {
                assert!(
                    (actual[ch] - expected[ch]).abs() < 1e-10,
                    "EQ1={eq1}, EQ2={eq2}, sample={i}: {actual:?} != {expected:?}"
                );
            }
        }
    }
}

#[test]
fn linear_bank_bypass_ramps_keep_declared_latency() {
    use crate::processing::Resolution;
    let shared = isolated_shared();
    let config = Config {
        mode: ProcessingMode::LinearPhase,
        resolution: Resolution::Low,
    };
    shared
        .requested_config
        .store(config.encode(), Ordering::Relaxed);
    let mut engine = Engine::new(shared.clone(), 44100.0);
    let latency = engine.latency() as usize;
    for i in 0..25000 {
        if i == 8000 {
            assert!(shared
                .pending
                .push(Box::new(Bank::new(
                    &[Band {
                        id: 1,
                        ..Band::default()
                    }],
                    &[Band {
                        id: 2,
                        ..Band::default()
                    }],
                    44100.0,
                    config,
                )))
                .is_ok());
            engine.sync();
        }
        let eq1 = i < 5000;
        let eq2 = i < 3000;
        let input = (i as f64 * 0.17).sin() * 0.1;
        let actual = engine.tick(
            [input, -input],
            CompSettings {
                wall: crate::dsp::WallSettings {
                    on: false,
                    ..Default::default()
                },
                ..CompSettings::default()
            },
            eq1,
            eq2,
            false,
            false,
            false,
            false,
            false,
        );
        let expected = if i >= latency {
            ((i - latency) as f64 * 0.17).sin() * 0.1
        } else {
            0.0
        };
        assert!(
            (actual[0] - expected).abs() < 1e-10,
            "sample {i}: {actual:?}, expected {expected}"
        );
        assert!((actual[1] + expected).abs() < 1e-10);
        assert_eq!(engine.latency() as usize, latency);
    }
    assert_eq!(engine.eq1_mix, 0.0);
    assert_eq!(engine.eq2_mix, 0.0);
}

#[test]
fn recycled_callback_buffers_keep_reserved_capacity() {
    let shared = isolated_shared();
    let mut engine = Engine::new(shared.clone(), 48000.0);
    for count in [1, 0, 32, 1, 64, 0] {
        *shared.lift_bands.lock().unwrap() = (0..count)
            .map(|i| LiftBand {
                id: LIFT_ID_BASE + i as u64,
                ..LiftBand::default()
            })
            .collect();
        *shared.sc_eq_bands.lock().unwrap() = (0..count)
            .map(|i| Band {
                id: i as u64 + 1,
                ..Band::default()
            })
            .collect();
        engine.sync();
        assert_eq!(engine.lift_bands.len(), count);
        assert_eq!(engine.sc_eq_bands.len(), count);
        for capacity in [
            engine.lift_bands.capacity(),
            engine.lift_scratch.capacity(),
            engine.sc_eq_bands.capacity(),
            engine.sc_eq_scratch.capacity(),
        ] {
            assert!(capacity >= AUX_BAND_RESERVE);
        }
    }
    let count = AUX_BAND_RESERVE + 1;
    let lift: Vec<_> = (0..count)
        .map(|i| LiftBand {
            id: LIFT_ID_BASE + i as u64,
            ..LiftBand::default()
        })
        .collect();
    let sc: Vec<_> = (0..count)
        .map(|i| Band {
            id: i as u64 + 1,
            ..Band::default()
        })
        .collect();
    *shared.lift_bands.lock().unwrap() = lift.clone();
    *shared.sc_eq_bands.lock().unwrap() = sc.clone();
    engine.sync();
    assert!(engine.lift_bands.is_empty());
    assert!(engine.sc_eq_bands.is_empty());
    assert!(shared
        .pending_aux
        .push(Box::new(AuxBuffers::new(&lift, &sc, 48000.0, 0)))
        .is_ok());
    engine.sync();
    assert_eq!(engine.lift_bands.len(), count);
    assert_eq!(engine.sc_eq_bands.len(), count);
    assert!(engine.lift_scratch.capacity() >= count);
    assert!(engine.sc_eq_scratch.capacity() >= count);

    engine.bank = Box::new(Bank::new(
        &(1..=300)
            .map(|id| Band {
                id,
                dynamic: true,
                ..Band::default()
            })
            .collect::<Vec<_>>(),
        &[],
        48000.0,
        Config::default(),
    ));
    let capacities = [
        engine.dyn_gr_scratch.capacity(),
        engine.dyn_input_scratch.capacity(),
        shared.dyn_gr_uncapped.lock().unwrap().capacity(),
        shared.dyn_band_input.lock().unwrap().capacity(),
    ];
    for _ in 0..4096 {
        engine.tick(
            [0.0; 2],
            CompSettings::default(),
            true,
            true,
            false,
            false,
            false,
            false,
            false,
        );
    }
    assert_eq!(
        [
            engine.dyn_gr_scratch.capacity(),
            engine.dyn_input_scratch.capacity(),
            shared.dyn_gr_uncapped.lock().unwrap().capacity(),
            shared.dyn_band_input.lock().unwrap().capacity()
        ],
        capacities
    );
    assert_eq!(
        shared.dyn_gr_uncapped.lock().unwrap().len(),
        DYNAMIC_BAND_METER_RESERVE
    );
    assert_eq!(
        shared.dyn_band_input.lock().unwrap().len(),
        DYNAMIC_BAND_METER_RESERVE
    );
    assert!(shared
        .pending_aux
        .push(Box::new(AuxBuffers::new(&lift, &sc, 48000.0, 300)))
        .is_ok());
    let locked_meter = shared.dyn_gr_uncapped.lock().unwrap();
    engine.sync();
    assert!(engine.pending_aux.is_some());
    assert_eq!(engine.dyn_gr_scratch.capacity(), DYNAMIC_BAND_METER_RESERVE);
    drop(locked_meter);
    engine.sync();
    assert!(engine.pending_aux.is_none());
    let grown_capacities = [
        engine.dyn_gr_scratch.capacity(),
        engine.dyn_input_scratch.capacity(),
        shared.dyn_gr_uncapped.lock().unwrap().capacity(),
        shared.dyn_band_input.lock().unwrap().capacity(),
    ];
    assert!(grown_capacities.iter().all(|capacity| *capacity >= 300));
    for _ in 0..4096 {
        engine.tick(
            [0.0; 2],
            CompSettings::default(),
            true,
            true,
            false,
            false,
            false,
            false,
            false,
        );
    }
    assert_eq!(
        [
            engine.dyn_gr_scratch.capacity(),
            engine.dyn_input_scratch.capacity(),
            shared.dyn_gr_uncapped.lock().unwrap().capacity(),
            shared.dyn_band_input.lock().unwrap().capacity()
        ],
        grown_capacities
    );
    assert_eq!(shared.dyn_gr_uncapped.lock().unwrap().len(), 300);
    assert_eq!(shared.dyn_band_input.lock().unwrap().len(), 300);
}
#[test]
fn bypass_paths_stay_latency_aligned_during_band_updates() {
    use crate::processing::{Resolution, MODES};
    for mode in MODES {
        for global_bypass in [false, true] {
            let shared = isolated_shared();
            let config = Config {
                mode,
                resolution: Resolution::Low,
            };
            shared
                .requested_config
                .store(config.encode(), Ordering::Relaxed);
            let mut engine = Engine::new(shared.clone(), 44100.0);
            engine.eq1_mix = if global_bypass { 1.0 } else { 0.0 };
            engine.eq2_mix = if global_bypass { 1.0 } else { 0.0 };
            engine.bypass_mix = if global_bypass { 1.0 } else { 0.0 };
            let latency = engine.latency() as usize;
            let settings = CompSettings {
                wall: crate::dsp::WallSettings {
                    on: false,
                    ..Default::default()
                },
                ..CompSettings::default()
            };
            for i in 0..20000 {
                if i == 8000 {
                    let next = Box::new(Bank::new(
                        &[Band {
                            gain: 18.0,
                            ..Band::default()
                        }],
                        &[],
                        44100.0,
                        config,
                    ));
                    assert!(shared.pending.push(next).is_ok());
                    engine.sync();
                }
                let x = (i as f64 * 0.1).sin() * 0.3;
                let y = engine.tick(
                    [x, -x],
                    settings,
                    global_bypass,
                    global_bypass,
                    false,
                    true,
                    false,
                    false,
                    global_bypass,
                );
                let expected = if i >= latency {
                    ((i - latency) as f64 * 0.1).sin() * 0.3
                } else {
                    0.0
                };
                assert!(
                    (y[0] - expected).abs() < 1e-9,
                    "{mode:?} at {i}: {} != {expected}",
                    y[0]
                );
                assert!((y[1] + expected).abs() < 1e-9);
            }
        }
    }
}
#[test]
fn mode_switch_publishes_actual_latency_and_rejects_stale_banks() {
    use crate::processing::Resolution;
    let shared = isolated_shared();
    let mut engine = Engine::new(shared.clone(), 48000.0);
    let config = Config {
        mode: ProcessingMode::LinearPhase,
        resolution: Resolution::Low,
    };
    shared
        .requested_config
        .store(config.encode(), Ordering::Relaxed);
    assert!(shared
        .pending
        .push(Box::new(Bank::new(&[], &[], 48000.0, Config::default())))
        .is_ok());
    engine.sync();
    assert_eq!(engine.latency(), 0);
    while shared.retired.pop().is_some() {}
    assert!(shared
        .pending
        .push(Box::new(Bank::new(&[], &[], 48000.0, config)))
        .is_ok());
    engine.sync();
    assert_eq!(engine.latency() as usize, config.latency(48000.0));
    assert_eq!(shared.latency.load(Ordering::Relaxed), engine.latency());
    assert_eq!(
        shared.active_config.load(Ordering::Relaxed),
        config.encode()
    );
    engine.reset();
    for _ in 0..10000 {
        assert_eq!(
            engine.tick(
                [0.0; 2],
                CompSettings::default(),
                true,
                true,
                true,
                true,
                true,
                false,
                false
            ),
            [0.0; 2]
        );
    }
}
#[test]
fn unlimited_band_state_and_neutral_audio() {
    let bands = Arc::new(Mutex::new(
        (1..=200)
            .map(|id| Band {
                id,
                ..Band::default()
            })
            .collect(),
    ));
    let shared = Shared::new(
        bands,
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
    );
    let mut engine = Engine::new(shared, 48000.0);
    assert_eq!(engine.bank.bands.len(), 200);
    let settings = CompSettings {
        wall: crate::dsp::WallSettings {
            on: false,
            ..Default::default()
        },
        threshold: 0.0,
        gate: -80.0,
        dry: 0.0,
        wet: 1.0,
        ..CompSettings::default()
    };
    for i in 0..4096 {
        let x = (i as f64 * 0.1).sin() * 0.5;
        let out = engine.tick(
            [x, x],
            settings,
            true,
            true,
            false,
            true,
            true,
            false,
            false,
        );
        assert!((out[0] - x).abs() < 1e-9);
    }
}
#[test]
fn bypass_preserves_input_after_ramp() {
    let shared = Shared::new(
        Arc::new(Mutex::new(vec![Band {
            gain: 18.0,
            ..Band::default()
        }])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
    );
    let mut engine = Engine::new(shared, 48000.0);
    let settings = CompSettings {
        threshold: -30.0,
        gate: -80.0,
        dry: 0.0,
        wet: 1.0,
        ..CompSettings::default()
    };
    for i in 0..24000 {
        let x = (i as f64 * 0.13).sin() * 0.1;
        let out = engine.tick([x, x], settings, true, true, false, true, true, false, true);
        if i > 23000 {
            assert!((out[0] - x).abs() < 1e-10);
        }
    }
}
#[test]
fn solo_audition_attenuates_distant_frequencies() {
    let band = Band {
        id: 1,
        freq: 1000.0,
        gain: 0.0,
        q: 2.0,
        ..Band::default()
    };
    let shared = Shared::new(
        Arc::new(Mutex::new(vec![band])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
    );
    let mut engine = Engine::new(shared.clone(), 48000.0);
    let settings = CompSettings {
        threshold: 0.0,
        gate: -80.0,
        dry: 0.0,
        wet: 1.0,
        ..CompSettings::default()
    };
    shared.solo_id.store(1, Ordering::Relaxed);
    let mut distant_energy = 0.0;
    for i in 0..4800 {
        let t = i as f64 / 48000.0;
        let x = (2.0 * std::f64::consts::PI * 100.0 * t).sin();
        let out = engine.tick(
            [x, x],
            settings,
            true,
            true,
            false,
            true,
            true,
            false,
            false,
        );
        if i > 2400 {
            distant_energy += out[0].powi(2);
        }
    }
    assert!(distant_energy / 2400.0 < 0.05);
}
#[test]
fn lift_band_parallel_processing_and_solo() {
    let lift = LiftBand {
        id: LIFT_ID_BASE + 1,
        shape: crate::band::Shape::HighShelf,
        freq: 10000.0,
        gain: 0.0,
        threshold: -30.0,
        ratio: 4.0,
        attack: 1.0,
        release: 50.0,
        range: 12.0,
        enabled: true,
        ..LiftBand::default()
    };
    let shared = Shared::new(
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![lift])),
    );
    let mut engine = Engine::new(shared.clone(), 48000.0);
    assert_eq!(engine.latency(), LIFT_LATENCY as u32);
    let settings = CompSettings::default();

    // Feed 10 kHz tone
    for i in 0..8192 {
        let t = i as f64 / 48000.0;
        let x = (2.0 * std::f64::consts::PI * 10000.0 * t).sin() * 0.5;
        let out = engine.tick(
            [x, x],
            settings,
            true,
            true,
            false,
            true,
            false,
            false,
            false,
        );
        if i > LIFT_LATENCY + 2048 {
            assert!(out[0].is_finite() && out[1].is_finite());
        }
    }

    // Test soloing the Lift band
    shared.solo_id.store(LIFT_ID_BASE + 1, Ordering::Relaxed);
    let mut solo_energy = 0.0;
    for i in 0..4800 {
        let t = i as f64 / 48000.0;
        let x = (2.0 * std::f64::consts::PI * 10000.0 * t).sin() * 0.5;
        let out = engine.tick(
            [x, x],
            settings,
            true,
            true,
            false,
            true,
            false,
            false,
            false,
        );
        if i > 4000 {
            solo_energy += out[0].powi(2);
        }
    }
    assert!(solo_energy > 0.01);
}
#[test]
fn dual_eq_series_processing_both_apply_and_bypass_independently() {
    let b1 = Band {
        id: 1,
        freq: 1000.0,
        gain: 6.0,
        q: 1.0,
        ..Band::default()
    };
    let b2 = Band {
        id: 10_001,
        freq: 1000.0,
        gain: 6.0,
        q: 1.0,
        ..Band::default()
    };
    let shared = Shared::new(
        Arc::new(Mutex::new(vec![b1])),
        Arc::new(Mutex::new(vec![b2])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
    );
    let mut engine = Engine::new(shared, 48000.0);
    let settings = CompSettings {
        threshold: 0.0,
        gate: -80.0,
        dry: 0.0,
        wet: 1.0,
        ..CompSettings::default()
    };

    // Let ramps settle
    for _ in 0..1000 {
        engine.tick(
            [0.0; 2], settings, true, true, false, true, false, false, false,
        );
    }

    // Both EQs active: 6 dB + 6 dB = +12 dB gain at 1 kHz (approx 4.0x linear gain amplitude)
    let mut max_both: f64 = 0.0;
    for i in 0..4800 {
        let t = i as f64 / 48000.0;
        let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * 0.1;
        let out = engine.tick(
            [x, x],
            settings,
            true,
            true,
            false,
            true,
            false,
            false,
            false,
        );
        if i > 2400 {
            max_both = max_both.max(out[0].abs());
        }
    }
    assert!(
        (max_both - 0.4).abs() < 0.05,
        "expected ~0.4, got {max_both}"
    );

    // Bypass EQ 2: only EQ 1 applies (+6 dB = ~2.0x linear gain amplitude)
    for _ in 0..2000 {
        engine.tick(
            [0.0; 2], settings, true, false, false, true, false, false, false,
        );
    }
    let mut max_eq1_only: f64 = 0.0;
    for i in 0..4800 {
        let t = i as f64 / 48000.0;
        let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * 0.1;
        let out = engine.tick(
            [x, x],
            settings,
            true,
            false,
            false,
            true,
            false,
            false,
            false,
        );
        if i > 2400 {
            max_eq1_only = max_eq1_only.max(out[0].abs());
        }
    }
    assert!(
        (max_eq1_only - 0.2).abs() < 0.05,
        "expected ~0.2, got {max_eq1_only}"
    );
}

#[test]
fn dynamics_pre_and_post_routing_order() {
    let b1 = Band {
        id: 1,
        freq: 1000.0,
        gain: 12.0,
        q: 2.0,
        ..Band::default()
    };
    let shared = Shared::new(
        Arc::new(Mutex::new(vec![b1])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
    );
    let mut engine = Engine::new(shared, 48000.0);
    let settings = CompSettings {
        threshold: -20.0,
        ratio: 4.0,
        attack: 0.5,
        release: 50.0,
        auto_makeup: false,
        gate: -80.0,
        dry: 0.0,
        wet: 1.0,
        ..CompSettings::default()
    };

    // Pre-EQ mode: input is -25 dB (~0.056 amplitude).
    // It enters compressor before EQ boost. Threshold is -20 dB, so little to no GR occurs.
    for _ in 0..1000 {
        engine.tick(
            [0.0; 2], settings, true, false, false, true, true, true, false,
        );
    }
    let mut max_pre = 0.0_f64;
    for i in 0..4800 {
        let t = i as f64 / 48000.0;
        let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * 0.056;
        let out = engine.tick(
            [x, x],
            settings,
            true,
            false,
            false,
            true,
            true,
            true,
            false,
        );
        if i > 2400 {
            max_pre = max_pre.max(out[0].abs());
        }
    }

    // Post-EQ mode: input is boosted by +12 dB (4x) first to ~0.224 (-13 dB),
    // entering compressor well above -20 dB threshold, triggering strong gain reduction.
    for _ in 0..2000 {
        engine.tick(
            [0.0; 2], settings, true, false, false, true, true, false, false,
        );
    }
    let mut max_post = 0.0_f64;
    for i in 0..4800 {
        let t = i as f64 / 48000.0;
        let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * 0.056;
        let out = engine.tick(
            [x, x],
            settings,
            true,
            false,
            false,
            true,
            true,
            false,
            false,
        );
        if i > 2400 {
            max_post = max_post.max(out[0].abs());
        }
    }

    // Pre output has full +12 dB boost preserved after compressor;
    // Post output was compressed because the EQ boosted it over threshold.
    assert!(
        max_pre > max_post * 1.25,
        "max_pre={max_pre}, max_post={max_post}"
    );
}

#[test]
fn pse_runs_before_eq_in_pre_and_post() {
    let band = Band {
        id: 1,
        freq: 1000.0,
        gain: 18.0,
        q: 2.0,
        ..Band::default()
    };
    let shared = Shared::new(
        Arc::new(Mutex::new(vec![band])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
    );
    let mut pre = Engine::new(shared, 48000.0);
    let post_shared = Shared::new(
        Arc::new(Mutex::new(vec![Band {
            id: 1,
            freq: 1000.0,
            gain: 18.0,
            q: 2.0,
            ..Band::default()
        }])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
    );
    let mut post = Engine::new(post_shared, 48000.0);
    let settings = CompSettings {
        threshold: 0.0,
        auto_makeup: false,
        gate: -40.0,
        dry: 0.0,
        wet: 1.0,
        ..CompSettings::default()
    };
    let amp = 0.003;
    for _ in 0..96000 {
        pre.tick(
            [0.0; 2], settings, true, false, false, true, true, true, false,
        );
        post.tick(
            [0.0; 2], settings, true, false, false, true, true, false, false,
        );
    }
    let mut max_pre = 0.0_f64;
    let mut max_post = 0.0_f64;
    for i in 0..48000 {
        let t = i as f64 / 48000.0;
        let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * amp;
        let pre_out = pre.tick(
            [x, x],
            settings,
            true,
            false,
            false,
            true,
            true,
            true,
            false,
        );
        let post_out = post.tick(
            [x, x],
            settings,
            true,
            false,
            false,
            true,
            true,
            false,
            false,
        );
        if i > 24000 {
            max_pre = max_pre.max(pre_out[0].abs());
            max_post = max_post.max(post_out[0].abs());
        }
    }
    // +18 dB of EQ on 0.003 would be ~0.024 if PSE ran after the boost.
    // With PSE first (~10 dB down) the boosted peak stays well below that.
    assert!(max_pre < 0.014, "max_pre={max_pre}");
    assert!(max_post < 0.014, "max_post={max_post}");
}

#[test]
fn sidechain_eq_filters_detector_and_listen_path() {
    let sc_band = Band {
        id: 20_001,
        shape: crate::band::Shape::LowCut,
        order: 3,
        freq: 500.0,
        gain: 0.0,
        q: std::f64::consts::FRAC_1_SQRT_2,
        enabled: true,
        dynamic: false,
        ..Band::default()
    };
    let shared = Shared::new(
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![])),
        Arc::new(Mutex::new(vec![sc_band])),
        Arc::new(Mutex::new(vec![])),
    );
    let mut engine = Engine::new(shared.clone(), 48000.0);
    let settings = CompSettings {
        wall: crate::dsp::WallSettings {
            on: false,
            ..Default::default()
        },
        threshold: -20.0,
        ratio: 4.0,
        attack: 0.5,
        release: 50.0,
        auto_makeup: false,
        gate: -80.0,
        dry: 0.0,
        wet: 1.0,
        ..CompSettings::default()
    };

    // Warm up ramps
    for _ in 0..2000 {
        engine.tick(
            [0.0; 2], settings, false, false, true, true, true, false, false,
        );
    }

    // 1. Feed a 100 Hz tone at 0.5 amplitude (-6 dB).
    // With SC EQ LowCut at 500 Hz (3rd order 18 dB/oct), 100 Hz is attenuated ~40 dB,
    // so sidechain detector sees <-45 dB (well below -20 dB threshold) -> no compression!
    for i in 0..4800 {
        let t = i as f64 / 48000.0;
        let x = (2.0 * std::f64::consts::PI * 100.0 * t).sin() * 0.5;
        let out = engine.tick(
            [x, x],
            settings,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
        );
        if i > 3000 {
            // Should pass through essentially uncompressed (matches input x)
            assert!((out[0] - x).abs() < 0.01, "out={}, x={}", out[0], x);
        }
    }
    let gr_attenuated = shared.gr.load(Ordering::Relaxed);
    assert!(
        gr_attenuated < 1.0,
        "expected minimal GR, got {gr_attenuated}"
    );

    // 2. Feed a 1000 Hz tone at 0.5 amplitude (-6 dB).
    // 1000 Hz is in the passband (> 500 Hz), so it easily exceeds -20 dB threshold -> strong compression!
    for i in 0..4800 {
        let t = i as f64 / 48000.0;
        let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * 0.5;
        engine.tick(
            [x, x],
            settings,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
        );
    }
    let gr_passed = shared.gr.load(Ordering::Relaxed);
    assert!(gr_passed > 5.0, "expected significant GR, got {gr_passed}");

    // 3. Test Listen SC path: output should be the filtered sidechain tone
    let mut listen_settings = settings;
    listen_settings.pse.listen = true;
    let mut max_sc_listen = 0.0_f64;
    for i in 0..4800 {
        let t = i as f64 / 48000.0;
        let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * 0.5;
        let out = engine.tick(
            [x, x],
            listen_settings,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
        );
        if i > 2400 {
            max_sc_listen = max_sc_listen.max(out[0].abs());
        }
    }
    assert!(
        (max_sc_listen - 0.5).abs() < 0.05,
        "expected passband signal on listen, got {max_sc_listen}"
    );

    // Listen SC stays available when the compressor section is bypassed.
    // 100 Hz is rejected by the 500 Hz SC low-cut, so output must be quiet.
    let mut max_sc_listen_comp_off = 0.0_f64;
    for i in 0..4800 {
        let t = i as f64 / 48000.0;
        let x = (2.0 * std::f64::consts::PI * 100.0 * t).sin() * 0.5;
        let out = engine.tick(
            [x, x],
            listen_settings,
            false,
            false,
            true,
            true,
            false,
            false,
            false,
        );
        if i > 2400 {
            max_sc_listen_comp_off = max_sc_listen_comp_off.max(out[0].abs());
        }
    }
    assert!(
        max_sc_listen_comp_off < 0.05,
        "expected filtered sidechain on listen with compressor off, got {max_sc_listen_comp_off}"
    );
}
