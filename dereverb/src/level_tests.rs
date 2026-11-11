use crate::{
    dsp::Engine,
    params::Quality,
    weighting::{LevelMode, Weighting, LOUDER},
};

fn source(i: usize, rate: f64) -> f32 {
    let t = i as f64 / rate;
    if t < 0.1 || (0.7..0.95).contains(&t) || t > 1.4 {
        return 0.0;
    }
    let phase = std::f64::consts::TAU * (145.0 * t + 0.7 * (3.1 * t).sin());
    let envelope = 0.02 + 0.4 * (std::f64::consts::TAU * 2.7 * t).sin().max(0.0).powi(2);
    (envelope * (phase.sin() + 0.35 * (phase * 2.0).sin() + 0.15 * (phase * 4.0).sin())) as f32
}

#[test]
fn normalized_model_and_relative_curve_are_gain_equivariant_in_both_qualities() {
    for quality in [Quality::Live, Quality::HighQuality] {
        for rate in [48000.0, 44100.0] {
            let mut engine = Engine::new(rate, 1, quality).unwrap();
            let total = (rate * 1.7) as usize + engine.latency() as usize;
            let input_energy: f64 = (0..total).map(|i| (source(i, rate) as f64).powi(2)).sum();
            for (nodes, strength) in [([1.0; 5], 0.8), ([1.0; 5], 1.0), (LOUDER, 0.8)] {
                engine.set_weighting(Weighting {
                    mode: LevelMode::Relative,
                    nodes,
                });
                engine.reset();
                let baseline: Vec<_> = (0..total)
                    .map(|i| engine.tick(0, source(i, rate), strength).1)
                    .collect();
                for gain in [0.001_f32, 0.1, 4.0] {
                    engine.reset();
                    let mut error = 0.0;
                    for (i, &expected) in baseline.iter().enumerate() {
                        let actual = engine.tick(0, gain * source(i, rate), strength).1 / gain;
                        assert!(actual.is_finite());
                        error += ((actual - expected) as f64).powi(2);
                    }
                    // Reference input energy: the neural model may intentionally
                    // suppress this tonal fixture almost completely at 100%.
                    let relative_error = (error / input_energy).sqrt();
                    assert!(relative_error < 0.001, "{quality:?}, {rate} Hz, strength {strength}, gain {gain}: compensated error/input RMS {relative_error}");
                }
            }
        }
    }
}

#[test]
fn suppressed_wet_bins_cannot_rotate_the_audible_dry_remainder() {
    use rustfft::num_complex::Complex32;
    for scale in [0.001, 1.0, 4.0] {
        for phase in [-3.0, 0.0, 3.0] {
            let dry = Complex32::new(scale, 0.0);
            let wet = Complex32::from_polar(scale * 1e-8, phase);
            let mixed = crate::dsp::spectral_strength(dry, wet, 0.8, false) / scale;
            assert!((mixed.re - 0.2).abs() < 1e-6);
            assert!(mixed.im.abs() < 1e-6);
        }
    }
}

#[test]
fn zero_curve_is_aligned_dry_and_activity_uses_the_applied_detector() {
    for quality in [Quality::Live, Quality::HighQuality] {
        let mut engine = Engine::new(48000.0, 2, quality).unwrap();
        engine.set_weighting(Weighting {
            mode: LevelMode::Absolute,
            nodes: [0.0; 5],
        });
        for i in 0..16000 {
            let x = source(i, 48000.0);
            let (aligned, out) = engine.tick(0, x, 1.0);
            engine.tick(1, x * 0.01, 1.0);
            assert!(
                (out - aligned).abs() < 3e-6,
                "zero weighting must retain dry reconstruction"
            );
        }
        let (_, level, weight) = engine.activity();
        assert!(level.active);
        assert_eq!(weight, 0.0);
        let curve = Weighting {
            mode: LevelMode::Absolute,
            nodes: LOUDER,
        };
        engine.set_weighting(curve);
        for i in 16000..18000 {
            engine.tick(0, source(i, 48000.0), 1.0);
            engine.tick(1, 0.01 * source(i, 48000.0), 1.0);
        }
        let (_, level, weight) = engine.activity();
        assert_eq!(weight, curve.amount(level));
        engine.reset();
        assert!(!engine.activity().1.active);
    }
}

/// Optional real-audio check without committing a third-party speech recording.
/// Supply mono 48 kHz little-endian f32 samples in DEREVERB_TEST_AUDIO.
#[test]
#[ignore = "Requires a mono 48 kHz f32 speech fixture in DEREVERB_TEST_AUDIO"]
fn speech_fixture_retains_level_invariance() {
    let bytes =
        std::fs::read(std::env::var("DEREVERB_TEST_AUDIO").expect("speech fixture path")).unwrap();
    let mut samples: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .take(4 * 48000)
        .collect();
    assert!(samples.len() >= 48000 && samples.iter().all(|x| x.is_finite()));
    let peak = samples.iter().map(|x| x.abs()).fold(0.0_f32, f32::max);
    assert!(peak > 0.0);
    for sample in &mut samples {
        *sample *= 0.5 / peak;
    }
    samples.resize(samples.len() + 4000, 0.0);
    for quality in [Quality::Live, Quality::HighQuality] {
        let mut engine = Engine::new(48000.0, 1, quality).unwrap();
        let baseline: Vec<_> = samples.iter().map(|&x| engine.tick(0, x, 1.0).1).collect();
        let energy: f64 = baseline.iter().map(|&x| (x as f64).powi(2)).sum();
        assert!(energy > 1e-4, "speech disappeared in {quality:?}");
        let mut worst = 0.0_f64;
        for gain in [0.001, 0.1, 4.0] {
            engine.reset();
            let mut peak_error = (0, 0.0_f32, 0.0_f32);
            let error: f64 = samples
                .iter()
                .zip(&baseline)
                .enumerate()
                .map(|(i, (&x, &expected))| {
                    let actual = engine.tick(0, x * gain, 1.0).1 / gain;
                    if (actual - expected).abs() > (peak_error.1 - peak_error.2).abs() {
                        peak_error = (i, actual, expected);
                    }
                    ((actual - expected) as f64).powi(2)
                })
                .sum();
            eprintln!(
                "{quality:?} gain {gain}: error {}, peak {peak_error:?}",
                (error / energy).sqrt()
            );
            worst = worst.max((error / energy).sqrt());
        }
        eprintln!(
            "{quality:?}: worst gain-compensated speech difference {:.6}% of output RMS",
            worst * 100.0
        );
        assert!(worst < 0.001, "{quality:?}: speech gain dependence {worst}");
    }
}
