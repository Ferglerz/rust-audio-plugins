#[path = "support/engine_golden.rs"]
mod engine_golden;
#[path = "support/engine_scenario.rs"]
mod engine_scenario;
#[path = "support/legacy_leveling.rs"]
mod legacy_leveling;
#[path = "support/legacy_strength.rs"]
mod legacy_strength;

use flattery::{
    dsp::{leveling::LevelingProcessor, Engine, Shared},
    params::FlatteryParams,
    strength::{PreparedStrengthCurve, StrengthNode},
};
use std::{hint::black_box, sync::Arc, time::Instant};

fn node(id: u64, freq: f64, weight: f64, radius: usize) -> StrengthNode {
    let mut node = StrengthNode::new(id, freq, weight);
    node.radius = radius;
    node
}

#[test]
fn full_engine_matches_pre_optimization_audio_fixture() {
    for (stage, (actual, expected)) in engine_scenario::run()
        .iter()
        .zip(engine_golden::EXPECTED)
        .enumerate()
    {
        for (index, (&actual, expected)) in actual.iter().zip(expected).enumerate() {
            let tolerance = 1.0e-10 * expected.abs().max(1.0);
            assert!(
                (actual - expected).abs() <= tolerance,
                "stage {stage}, statistic {index}: {actual} differs from baseline {expected}"
            );
        }
    }
}

#[test]
fn prepared_curves_match_legacy_with_unsorted_and_duplicate_frequencies() {
    let curves = [
        vec![],
        vec![node(1, 1000.0, 0.4, 7)],
        vec![node(1, 8000.0, 0.1, 11), node(2, 200.0, 2.4, 2)],
        vec![
            node(1, 1000.0, 0.0, 9),
            node(2, 200.0, 0.3, 2),
            node(3, 1000.0, 8.0, 3),
            node(4, 1000.0001, 0.2, 11),
            node(5, 200.0, 2.0, 7),
            node(6, 12000.0, 4.0, 12),
            node(7, 12000.0, 0.0, 1),
        ],
        vec![node(1, 1000.0, 0.5, 2), node(2, 1000.0, 1.5, 11)],
    ];
    for nodes in curves {
        let prepared = PreparedStrengthCurve::new(Arc::from(nodes.clone()));
        assert_eq!(prepared.nodes(), nodes);
        for default_radius in [1, 6, 12] {
            for freq in [
                10.0, 200.0, 200.001, 999.999, 1000.0, 1000.00005, 12000.0, 22050.0,
            ] {
                assert_eq!(
                    prepared.radius_at(freq, default_radius),
                    legacy_strength::radius_at(&nodes, freq, default_radius)
                );
            }
            for fft_size in [128, 256, 512, 1024, 2048, 4096, 8192] {
                for sample_rate in [44100.0, 48000.0, 96000.0] {
                    let half = fft_size / 2;
                    let bin_hz = sample_rate / fft_size as f64;
                    let mut expected_weights = vec![0.0; half];
                    let mut expected_radii = vec![0; half];
                    let mut weights = vec![0.0; half];
                    let mut radii = vec![0; half];
                    legacy_strength::fill_bin_weights(
                        &nodes,
                        half,
                        bin_hz,
                        10.0,
                        22050.0,
                        &mut expected_weights,
                    );
                    legacy_strength::fill_bin_radii(
                        &nodes,
                        half,
                        bin_hz,
                        default_radius,
                        &mut expected_radii,
                    );
                    prepared.fill_bin_weights(half, bin_hz, &mut weights);
                    prepared.fill_bin_radii(half, bin_hz, default_radius, &mut radii);
                    assert_eq!(weights, expected_weights);
                    assert_eq!(radii, expected_radii);
                }
            }
        }
    }
}

struct Frame {
    left: Vec<f64>,
    right: Vec<f64>,
    boost_radii: Vec<usize>,
    cut_radii: Vec<usize>,
    boost_weights: Vec<f64>,
    cut_weights: Vec<f64>,
    count: usize,
}

impl Frame {
    fn new(count: usize, extra: usize, radius: usize, same_radius: bool) -> Self {
        let left = (0..count + extra)
            .map(|k| match k % 11 {
                0 => 0.0,
                1 => 1.0e-14,
                _ => ((k * 17 % 37) as f64 + 1.0) / 50.0,
            })
            .collect();
        let right = (0..count + extra)
            .map(|k| match k % 13 {
                0 => 0.0,
                1 => 1.0e-9,
                _ => ((k * 19 % 31) as f64 + 1.0) / 40.0,
            })
            .collect();
        Self {
            left,
            right,
            boost_radii: vec![radius; count],
            cut_radii: (0..count)
                .map(|k| if same_radius { radius } else { k % 12 + 1 })
                .collect(),
            boost_weights: (0..count).map(|k| (k % 7) as f64 * 0.8).collect(),
            cut_weights: (0..count).map(|k| (k % 5) as f64 * 0.6).collect(),
            count,
        }
    }
}

macro_rules! process_frame {
    ($processor:expr, $frame:expr, $link:expr, $amplify:expr, $narrow:expr) => {{
        let frame = &$frame;
        $processor.process(
            black_box(&frame.left),
            &frame.right,
            frame.count,
            &frame.boost_radii,
            &frame.cut_radii,
            $amplify,
            $link,
            83.0,
            67.0,
            12.0,
            17.0,
            if $narrow { frame.count / 3 } else { 0 },
            if $narrow {
                frame.count / 2
            } else {
                frame.count - 1
            },
            1.0e-8,
            0.75,
            13.0,
            107.0,
            256.0 / 48000.0,
            &frame.boost_weights,
            &frame.cut_weights,
        );
    }};
}

#[test]
fn cached_db_neighborhoods_preserve_successive_legacy_gain_arrays() {
    for (count, extra) in [(8, 0), (8, 19), (64, 0), (4096, 27)] {
        let mut optimized = LevelingProcessor::new();
        let mut legacy = legacy_leveling::LevelingProcessor::new();
        for radius in [1, 6, 12] {
            for same_radius in [true, false] {
                let mut frame = Frame::new(count, extra, radius, same_radius);
                for (iteration, (link, amplify, narrow)) in [
                    (0.0, false, false),
                    (50.0, false, true),
                    (100.0, true, false),
                    (50.0, true, true),
                    (100.0, false, false),
                    (0.0, true, false),
                ]
                .into_iter()
                .enumerate()
                {
                    if iteration == 4 {
                        frame.left.fill(0.0);
                        frame.right.fill(0.0);
                    }
                    process_frame!(optimized, frame, link, amplify, narrow);
                    process_frame!(legacy, frame, link, amplify, narrow);
                    assert_eq!(optimized.smoothed_gain_db_l, legacy.smoothed_gain_db_l);
                    assert_eq!(optimized.smoothed_gain_db_r, legacy.smoothed_gain_db_r);
                    assert_eq!(
                        optimized.smoothed_gain_db_link,
                        legacy.smoothed_gain_db_link
                    );
                }
                optimized.reset();
                legacy.reset();
            }
        }
    }
}

#[test]
fn display_visibility_preserves_audio_and_reopening_discards_old_peaks() {
    let shared = Arc::new(Shared::new());
    let mut visible = Engine::new(Arc::clone(&shared), 44100.0);
    let mut hidden = Engine::new(Arc::new(Shared::new()), 44100.0);
    hidden.set_display_enabled(false);
    let settings = FlatteryParams::default().process_settings();
    visible.set_display_enabled(true);
    for i in 0..4096 {
        let sample = 0.4 * (std::f64::consts::TAU * i as f64 / 17.0).sin();
        assert_eq!(
            visible.tick(sample, sample * 0.3, &settings),
            hidden.tick(sample, sample * 0.3, &settings)
        );
    }
    visible.set_display_enabled(false);
    shared.spectrum_mags_db.write().unwrap().fill(20.0);
    let stale_filters = shared.filter_display.read().unwrap().clone();
    for _ in 0..2048 {
        assert_eq!(
            visible.tick(0.0, 0.0, &settings),
            hidden.tick(0.0, 0.0, &settings)
        );
    }
    assert!(shared
        .spectrum_mags_db
        .read()
        .unwrap()
        .iter()
        .all(|&db| db == 20.0));
    assert_eq!(*shared.filter_display.read().unwrap(), stale_filters);
    // A busy UI must defer reset, not lose it.
    let spectrum_reader = shared.spectrum_mags_db.read().unwrap();
    visible.set_display_enabled(true);
    for _ in 0..settings.fft_size {
        assert_eq!(
            visible.tick(0.0, 0.0, &settings),
            hidden.tick(0.0, 0.0, &settings)
        );
    }
    assert!(spectrum_reader.iter().all(|&db| db == 20.0));
    drop(spectrum_reader);
    for _ in 0..settings.fft_size {
        assert_eq!(
            visible.tick(0.0, 0.0, &settings),
            hidden.tick(0.0, 0.0, &settings)
        );
    }
    assert!(shared
        .spectrum_mags_db
        .read()
        .unwrap()
        .iter()
        .all(|&db| db < 0.0));
}

#[test]
fn reopening_waits_for_a_complete_fresh_fft_window() {
    let shared = Arc::new(Shared::new());
    let mut engine = Engine::new(Arc::clone(&shared), 44100.0);
    let mut settings = FlatteryParams::default().process_settings();
    settings.input_rms_ms = 0.0;
    for i in 0..2048 {
        let loud = (std::f64::consts::TAU * i as f64 / 17.0).sin();
        engine.tick(loud, loud, &settings);
    }
    engine.set_display_enabled(false);
    for i in 0..2048 {
        let loud = (std::f64::consts::TAU * i as f64 / 17.0).sin();
        engine.tick(loud, loud, &settings);
    }
    engine.set_display_enabled(true);
    assert!(shared
        .spectrum_mags_db
        .read()
        .unwrap()
        .iter()
        .all(|&db| db == -120.0));
    for _ in 0..settings.fft_size - 1 {
        engine.tick(0.0, 0.0, &settings);
    }
    assert!(shared
        .spectrum_mags_db
        .read()
        .unwrap()
        .iter()
        .all(|&db| db == -120.0));
    engine.tick(0.0, 0.0, &settings);
    assert!(shared
        .spectrum_mags_db
        .read()
        .unwrap()
        .iter()
        .all(|&db| db < -120.0));
}

#[test]
#[ignore = "release timing harness; run explicitly with --release --ignored --nocapture"]
fn benchmark_legacy_and_cached_spectral_hops() {
    assert!(!cfg!(debug_assertions), "benchmark requires --release");
    for count in [256, 4096] {
        let nodes = vec![node(1, 6000.0, 0.4, 12), node(2, 200.0, 1.8, 2)];
        let prepared = PreparedStrengthCurve::new(Arc::from(nodes.clone()));
        let mut legacy_frame = Frame::new(count, 0, 12, false);
        let mut cached_frame = Frame::new(count, 0, 12, false);
        let bin_hz = 48000.0 / (count * 2) as f64;
        let mut legacy = legacy_leveling::LevelingProcessor::new();
        let mut optimized = LevelingProcessor::new();
        let repetitions = if count == 256 { 2000 } else { 150 };
        for frame in [&mut legacy_frame, &mut cached_frame] {
            prepared.fill_bin_weights(count, bin_hz, &mut frame.boost_weights);
            prepared.fill_bin_weights(count, bin_hz, &mut frame.cut_weights);
            prepared.fill_bin_radii(count, bin_hz, 3, &mut frame.boost_radii);
            prepared.fill_bin_radii(count, bin_hz, 3, &mut frame.cut_radii);
        }
        let start = Instant::now();
        for _ in 0..repetitions {
            legacy_strength::fill_bin_weights(
                black_box(&nodes),
                count,
                bin_hz,
                10.0,
                22050.0,
                &mut legacy_frame.boost_weights,
            );
            legacy_strength::fill_bin_weights(
                &nodes,
                count,
                bin_hz,
                10.0,
                22050.0,
                &mut legacy_frame.cut_weights,
            );
            legacy_strength::fill_bin_radii(
                &nodes,
                count,
                bin_hz,
                3,
                &mut legacy_frame.boost_radii,
            );
            legacy_strength::fill_bin_radii(&nodes, count, bin_hz, 3, &mut legacy_frame.cut_radii);
            process_frame!(legacy, legacy_frame, 50.0, false, false);
            black_box(&legacy.smoothed_gain_db_l);
        }
        let legacy_elapsed = start.elapsed();
        let start = Instant::now();
        for _ in 0..repetitions {
            process_frame!(optimized, cached_frame, 50.0, false, false);
            black_box(&optimized.smoothed_gain_db_l);
        }
        let cached_elapsed = start.elapsed();
        assert_eq!(optimized.smoothed_gain_db_l, legacy.smoothed_gain_db_l);
        assert_eq!(optimized.smoothed_gain_db_r, legacy.smoothed_gain_db_r);
        println!(
            "FFT {} stable spectral hop: legacy {:.1} us, cached {:.1} us, {:.2}x faster ({} hops)",
            count * 2,
            legacy_elapsed.as_secs_f64() * 1e6 / repetitions as f64,
            cached_elapsed.as_secs_f64() * 1e6 / repetitions as f64,
            legacy_elapsed.as_secs_f64() / cached_elapsed.as_secs_f64(),
            repetitions
        );
    }
}
