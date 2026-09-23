use super::*;
use crate::{
    band::{Band, Shape},
    dsp::BandRuntime,
};

#[test]
fn linear_impulse_is_symmetric_and_has_correct_latency_at_every_resolution() {
    for resolution in RESOLUTIONS {
        let config = Config {
            mode: ProcessingMode::LinearPhase,
            resolution,
        };
        let mut path = EqPath::new(&[], 44100.0, config);
        let latency = config.latency(44100.0);
        for i in 0..latency + HOP * 2 {
            let input = if i == 0 { [1.0, -0.5] } else { [0.0; 2] };
            let actual = path.tick(input, &mut [], 44100.0);
            let expected = if i == latency { [1.0, -0.5] } else { [0.0; 2] };
            for ch in 0..2 {
                assert!(
                    (actual[ch] - expected[ch]).abs() < 1e-10,
                    "{resolution:?}, sample {i}: {actual:?}"
                );
            }
        }
    }
}

#[test]
fn linear_bell_matches_gain_and_is_symmetric_across_partition_boundaries() {
    let band = Band {
        freq: 1000.0,
        gain: 9.0,
        q: 2.0,
        ..Band::default()
    };
    let config = Config {
        mode: ProcessingMode::LinearPhase,
        resolution: Resolution::Low,
    };
    let sr = 48000.0;
    let mut path = EqPath::new(std::slice::from_ref(&band), sr, config);
    let latency = config.latency(sr);
    let impulse_at = HOP - 13;
    let center = latency + impulse_at;
    let mut impulse = Vec::new();
    for i in 0..center * 2 + HOP {
        let x = if i == impulse_at {
            [1.0, 0.0]
        } else {
            [0.0; 2]
        };
        let out = path.tick(x, &mut [], sr);
        assert!(out[1].abs() < 1e-12);
        impulse.push(out[0]);
    }
    for i in 1..center {
        assert!(
            (impulse[center - i] - impulse[center + i]).abs() < 1e-10,
            "asymmetric at {i}"
        );
    }
    let gain: f64 = impulse
        .iter()
        .enumerate()
        .map(|(i, x)| {
            x * (std::f64::consts::TAU * band.freq * (i as f64 - center as f64) / sr).cos()
        })
        .sum();
    assert!(
        (20.0 * gain.log10() - band.gain).abs() < 0.1,
        "gain: {gain}"
    );
    path.reset();
    for _ in 0..center * 2 {
        assert_eq!(path.tick([0.0; 2], &mut [], sr), [0.0; 2]);
    }
}

#[test]
fn oversampled_neutral_path_has_unity_gain_and_declared_group_delay() {
    let sr = 48000.0;
    let config = Config {
        mode: ProcessingMode::NaturalPhase,
        ..Config::default()
    };
    let mut path = EqPath::new(&[], sr, config);
    let latency = config.latency(sr);
    let impulse: Vec<_> = (0..256)
        .map(|i| path.tick(if i == 0 { [1.0, -1.0] } else { [0.0; 2] }, &mut [], sr))
        .collect();
    let peak = impulse
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a[0].total_cmp(&b[0]))
        .unwrap()
        .0;
    assert_eq!(peak, latency);
    for i in 0..64 {
        assert!((impulse[latency - i][0] - impulse[latency + i][0]).abs() < 1e-12);
    }
    assert!((impulse.iter().map(|x| x[0]).sum::<f64>() - 1.0).abs() < 1e-5);
    for x in &impulse {
        assert!((x[0] + x[1]).abs() < 1e-12);
    }
    for i in 0..4800 {
        let x = (std::f64::consts::TAU * 1000.0 * i as f64 / sr).sin();
        let out = path.tick([x, x], &mut [], sr);
        if i > 1000 {
            let expected =
                (std::f64::consts::TAU * 1000.0 * (i as f64 - latency as f64) / sr).sin();
            assert!((out[0] - expected).abs() < 1e-4);
        }
    }
    path.reset();
    for _ in 0..256 {
        assert_eq!(path.tick([0.0; 2], &mut [], sr), [0.0; 2]);
    }
}

#[test]
fn oversampled_eq_matches_high_rate_filter_and_keeps_dynamics() {
    let sr = 48000.0;
    let band = Band {
        freq: 8000.0,
        gain: 6.0,
        q: 1.0,
        ..Band::default()
    };
    let config = Config {
        mode: ProcessingMode::NaturalPhase,
        ..Config::default()
    };
    let mut path = EqPath::new(std::slice::from_ref(&band), sr, config);
    let mut bands = vec![BandRuntime::new(band.clone(), sr * 4.0)];
    let mut energy = 0.0;
    for i in 0..9600 {
        let x = (std::f64::consts::TAU * band.freq * i as f64 / sr).sin() * 0.25;
        let out = path.tick([x, x], &mut bands, sr);
        if i >= 4800 {
            energy += out[0] * out[0];
        }
    }
    let gain_db = 10.0 * (energy / 4800.0 / (0.25 * 0.25 / 2.0)).log10();
    assert!((gain_db - 6.0).abs() < 0.05, "gain: {gain_db}");
    bands[0] = BandRuntime::new(
        Band {
            dynamic: true,
            threshold: -36.0,
            ratio: 8.0,
            range: 12.0,
            ..band
        },
        sr * 4.0,
    );
    path.reset();
    for i in 0..24000 {
        let x = (std::f64::consts::TAU * 8000.0 * i as f64 / sr).sin() * 0.25;
        path.tick([x, x], &mut bands, sr);
    }
    assert!(bands[0].reduction > 10.0);
}

#[test]
fn linear_uses_static_gain_of_dynamic_bands_and_ignores_disabled_bands() {
    let dynamic = Band {
        gain: 6.0,
        freq: 1500.0,
        dynamic: true,
        ..Band::default()
    };
    let disabled = Band {
        shape: Shape::LowCut,
        freq: 8000.0,
        enabled: false,
        ..Band::default()
    };
    let config = Config {
        mode: ProcessingMode::LinearPhase,
        resolution: Resolution::Low,
    };
    let mut a = EqPath::new(&[dynamic.clone(), disabled], 44100.0, config);
    let mut b = EqPath::new(
        &[Band {
            dynamic: false,
            ..dynamic
        }],
        44100.0,
        config,
    );
    for i in 0..10000 {
        let x = [(i as f64 * 0.1).sin(); 2];
        assert_eq!(a.tick(x, &mut [], 44100.0), b.tick(x, &mut [], 44100.0));
    }
}

#[test]
fn resolution_scales_with_sample_rate_and_config_round_trips() {
    for mode in MODES {
        for resolution in RESOLUTIONS {
            let config = Config { mode, resolution };
            assert_eq!(Config::decode(config.encode()), config);
            if mode == ProcessingMode::LinearPhase {
                let order = resolution.order(96000.0);
                assert!(order as f64 / 96000.0 >= resolution.order(44100.0) as f64 / 44100.0);
                assert_eq!(order % HOP, 0);
            }
        }
    }
}
