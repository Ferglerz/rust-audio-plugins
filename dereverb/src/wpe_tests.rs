use crate::{
    dsp::Engine,
    params::{EngineKind, Quality},
    weighting::{LevelMode, Weighting, LOUDER},
    wpe::OnlineWpe,
};
use rustfft::num_complex::Complex64 as C;

#[test]
fn matches_upstream_online_wpe_reference() {
    let bytes = include_bytes!("../testdata/wpe-reference.f64");
    let mut wpe = OnlineWpe::new(4);
    for (frame, data) in bytes.chunks_exact(4 * 4 * 8).enumerate() {
        let values: Vec<f64> = data
            .chunks_exact(8)
            .map(|b| f64::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let mut spectrum: [C; 4] =
            std::array::from_fn(|b| C::new(values[b * 4], values[b * 4 + 1]));
        assert!(wpe.process(&mut spectrum));
        for (b, actual) in spectrum.iter().enumerate() {
            let expected = C::new(values[b * 4 + 2], values[b * 4 + 3]);
            assert!(
                (*actual - expected).norm() < 1e-10,
                "frame {frame}, bin {b}: {actual} != {expected}"
            );
        }
    }
}

#[test]
fn recursive_prediction_reduces_delayed_room_energy() {
    let mut state = 591_u32;
    let mut random = || {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        state as f64 / u32::MAX as f64 - 0.5
    };
    let mut wpe = OnlineWpe::new(1);
    let mut room = [C::ZERO; 4];
    let (mut before, mut after, mut direct, mut retained) = (0.0, 0.0, 0.0, 0.0);
    for t in 0..6000 {
        let dry = C::new(random(), random());
        let wet = dry + room[t % 4] * 0.65;
        room[t % 4] = wet;
        let mut spectrum = [wet];
        assert!(wpe.process(&mut spectrum));
        if t > 1000 {
            before += (wet - dry).norm_sqr();
            after += (spectrum[0] - dry).norm_sqr();
            direct += dry.norm_sqr();
            retained += (spectrum[0] * dry.conj()).re;
        }
    }
    assert!(
        after < before * 0.3,
        "prediction error: {after} vs reverberant {before}"
    );
    assert!(
        (retained / direct - 1.0).abs() < 0.03,
        "direct excitation changed"
    );
}

#[test]
fn wpe_postfilter_rejects_tail_energy_from_an_overprediction() {
    use rustfft::num_complex::Complex32;

    for gain in [1e-25, 1.0, 1e25] {
        let observed = Complex32::new(gain, 0.0);
        let overprediction = -2.0 * observed;
        let output = crate::dsp::suppress_wpe_tail(observed, overprediction);
        assert!((output.re / gain + 0.2).abs() < 1e-6);
        assert_eq!(crate::dsp::suppress_wpe_tail(observed, observed), observed);
        assert_eq!(
            crate::dsp::suppress_wpe_tail(Complex32::ZERO, overprediction),
            Complex32::ZERO
        );
    }
}

#[test]
fn silence_cannot_inflate_covariance_or_leak_after_reset() {
    let mut wpe = OnlineWpe::new(1);
    for _ in 0..100_000 {
        let mut spectrum = [C::ZERO];
        assert!(wpe.process(&mut spectrum));
        assert_eq!(spectrum[0], C::ZERO);
    }
    let mut fresh = OnlineWpe::new(1);
    for t in 0..100 {
        let value = C::new((t as f64).sin(), (t as f64 * 0.7).cos());
        let (mut a, mut b) = ([value], [value]);
        assert!(wpe.process(&mut a));
        assert!(fresh.process(&mut b));
        assert!((a[0] - b[0]).norm() < 1e-12);
    }
    wpe.reset();
    let mut zero = [C::ZERO];
    assert!(wpe.process(&mut zero));
    assert_eq!(zero, [C::ZERO]);
}

#[test]
fn stationary_and_near_empty_bins_remain_stable_over_long_sessions() {
    let mut wpe = OnlineWpe::with_hop(3, 240, 0.5);
    for t in 0..100_000 {
        let x = C::from_polar(1.0, t as f64 * 0.71);
        let mut spectrum = [x, x * 1e-14, C::ZERO];
        assert!(
            wpe.process(&mut spectrum),
            "numerical recovery at frame {t}"
        );
        assert!(spectrum
            .iter()
            .all(|x| x.re.is_finite() && x.im.is_finite() && x.norm() < 2.0));
    }
}

#[test]
fn learned_frequency_filters_remove_room_echo_after_the_source_changes() {
    let mut wpe = OnlineWpe::with_hop(2, 240, 0.5);
    let mut history = [[C::ZERO; 8]; 2]; // 40 ms, independent room responses
    let mut random = 591_u32;
    let mut error = [0.0; 2];
    let mut reverb = [0.0; 2];
    for frame in 0..4000 {
        let dry: [C; 2] = std::array::from_fn(|_| {
            random = random.wrapping_mul(1664525).wrapping_add(1013904223);
            C::from_polar(0.5, random as f64 / u32::MAX as f64 * std::f64::consts::TAU)
                * if frame > 2000 { 0.25 } else { 1.0 }
        });
        let mut wet: [C; 2] = std::array::from_fn(|bin| {
            dry[bin] + history[bin][frame % 8] * C::from_polar(0.65, bin as f64 * 1.4)
        });
        let observed = wet;
        for bin in 0..2 {
            history[bin][frame % 8] = wet[bin];
        }
        assert!(wpe.process(&mut wet));
        // A new source level must still benefit from the previously learned room.
        if frame > 2200 {
            for bin in 0..2 {
                error[bin] += (wet[bin] - dry[bin]).norm_sqr();
                reverb[bin] += (observed[bin] - dry[bin]).norm_sqr();
            }
        }
    }
    for bin in 0..2 {
        assert!(error[bin] < reverb[bin] * 0.2);
    }
    wpe.reset();
    let mut onset = [C::new(0.5, 0.2); 2];
    let dry = onset;
    assert!(wpe.process(&mut onset));
    assert_eq!(onset, dry, "reset retained an old room filter");
}

fn engine(rate: f64, channels: usize) -> Engine {
    Engine::prepare(rate, channels, EngineKind::NaraWpe, Quality::Live).unwrap()
}

fn signal(i: usize, rate: f64) -> f32 {
    let t = i as f64 / rate;
    if t < 0.03 || (0.28..0.38).contains(&t) || t > 0.6 {
        return 0.0;
    }
    ((t * 1843.0).sin() * 0.2 + (t * 3737.0 + (t * 93.0).cos()).sin() * 0.1) as f32
}

#[test]
fn wpe_preserves_gain_scaling_relative_weighting_and_reset() {
    for rate in [48000.0, 44100.0] {
        let mut engine = engine(rate, 1);
        let total = (rate * 0.75) as usize;
        for nodes in [[1.0; 5], LOUDER] {
            engine.set_weighting(Weighting {
                mode: LevelMode::Relative,
                nodes,
            });
            engine.reset();
            let baseline: Vec<_> = (0..total)
                .map(|i| engine.tick(0, signal(i, rate), 0.8).1)
                .collect();
            let energy: f64 = baseline.iter().map(|&v| (v as f64).powi(2)).sum();
            assert!(energy > 1e-5);
            for gain in [0.001, 4.0] {
                engine.reset();
                let error: f64 = baseline
                    .iter()
                    .enumerate()
                    .map(|(i, &expected)| {
                        let actual = engine.tick(0, signal(i, rate) * gain, 0.8).1 / gain;
                        assert!(actual.is_finite());
                        ((actual - expected) as f64).powi(2)
                    })
                    .sum();
                assert!(
                    (error / energy).sqrt() < 1e-4,
                    "{rate} Hz, gain {gain}: {}",
                    (error / energy).sqrt()
                );
            }
        }
    }
}

#[test]
fn wpe_latency_zero_weight_dry_and_channel_isolation() {
    for rate in [8000.0, 44100.0, 48000.0, 96000.0, 384000.0] {
        let mut engine = engine(rate, 2);
        if rate == 48000.0 {
            assert_eq!(engine.latency(), 480);
        }
        engine.set_weighting(Weighting {
            mode: LevelMode::Absolute,
            nodes: [0.0; 5],
        });
        let mut peak = (0, 0.0_f32);
        for i in 0..(engine.latency() as usize + 2048) {
            let input = if i == 0 { 0.8 } else { 0.0 };
            let (aligned, output) = engine.tick(0, input, 1.0);
            if rate == 48000.0 {
                assert!((aligned - output).abs() < 1e-6);
            }
            if output.abs() > peak.1 {
                peak = (i, output.abs());
            }
            assert_eq!(engine.tick(1, 0.0, 1.0).1, 0.0);
        }
        assert_eq!(peak.0, engine.latency() as usize, "{rate} Hz alignment");
        assert!(peak.1 > 0.05);
        engine.reset();
        for input in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(engine.tick(0, input, 1.0).1.is_finite());
        }
    }
}

/// Full signal-chain speech comparison using an optional local fixture.
#[test]
#[ignore = "Requires mono 48 kHz f32 speech in DEREVERB_TEST_AUDIO"]
fn wpe_speech_fixture_retains_level_invariance() {
    let bytes = std::fs::read(std::env::var("DEREVERB_TEST_AUDIO").unwrap()).unwrap();
    let samples: Vec<f32> = bytes
        .chunks_exact(4)
        .take(4 * 48000)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    assert!(samples.len() >= 48000);
    let mut engine = engine(48000.0, 1);
    let baseline: Vec<_> = samples.iter().map(|&x| engine.tick(0, x, 1.0).1).collect();
    let energy: f64 = baseline.iter().map(|&x| (x as f64).powi(2)).sum();
    assert!(energy > 1e-4);
    for gain in [0.001, 0.1, 4.0] {
        engine.reset();
        let error: f64 = samples
            .iter()
            .zip(&baseline)
            .map(|(&x, &y)| {
                let out = engine.tick(0, x * gain, 1.0).1 / gain;
                ((out - y) as f64).powi(2)
            })
            .sum();
        eprintln!(
            "WPE gain {gain}: compensated speech RMS error {}",
            (error / energy).sqrt()
        );
        assert!((error / energy).sqrt() < 1e-4);
    }
}

/// Render the actual sample-by-sample path for the reproducible window study.
#[test]
#[ignore = "Requires mono 48 kHz f32 inputs in DEREVERB_WINDOW_STUDY"]
fn render_wpe_window_study() {
    let dir = std::path::PathBuf::from(std::env::var("DEREVERB_WINDOW_STUDY").unwrap());
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap();
        if !name.ends_with("-input.f32") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let samples: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|x| f32::from_le_bytes(x.try_into().unwrap()))
            .collect();
        for (fft, memory) in [
            (960, 0.0),
            (720, 0.0),
            (480, 0.0),
            (360, 0.0),
            (240, 0.0),
            (120, 0.0),
            (480, 0.5),
            (240, 0.5),
            (120, 0.5),
        ] {
            let mut engine = Engine::wpe_window(48000.0, 1, fft, memory).unwrap();
            let latency = engine.latency() as usize;
            let start = std::time::Instant::now();
            let output: Vec<f32> = samples
                .iter()
                .copied()
                .chain(std::iter::repeat_n(0.0, latency))
                .map(|x| engine.tick(0, x, 1.0).1)
                .skip(latency)
                .collect();
            assert!(output.iter().all(|x| x.is_finite()));
            let rtf = start.elapsed().as_secs_f64() / (samples.len() as f64 / 48000.0);
            eprintln!("{name}: {fft} samples, memory {memory}, RTF {rtf:.4}");
            let bytes: Vec<u8> = output.iter().flat_map(|x| x.to_le_bytes()).collect();
            std::fs::write(
                dir.join(format!(
                    "{}-rust-{fft}-{memory}.f32",
                    name.trim_end_matches("-input.f32")
                )),
                bytes,
            )
            .unwrap();
        }
    }
}
