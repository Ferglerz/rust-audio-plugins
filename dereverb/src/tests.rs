use super::*;
use dsp::Engine;
use params::{EngineKind, Quality};
use rustfft::num_complex::Complex32;
use std::{cell::Cell, f32::consts::TAU};

fn signal(i: usize, rate: f64) -> f32 {
    let phase = std::f64::consts::TAU * 145.0 * i as f64 / rate;
    if i % (rate as usize / 4) < (rate * 0.177) as usize {
        (0.14 * phase.sin() + 0.05 * (2.03 * phase).sin() + 0.025 * (3.97 * phase).sin()) as f32
    } else {
        0.0
    }
}

#[test]
fn spectral_strength_preserves_endpoints_magnitude_and_real_bins() {
    let original = Complex32::new(1.0, 0.0);
    let enhanced = Complex32::new(-0.5, 0.0);
    for strength in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let result = dsp::spectral_strength(original, enhanced, strength, false);
        assert!((result.norm() - (1.0 - 0.5 * strength)).abs() < 1e-6);
        assert_eq!(
            dsp::spectral_strength(original, enhanced, strength, true).im,
            0.0
        );
    }
    assert_eq!(
        dsp::spectral_strength(original, enhanced, 0.0, false),
        original
    );
    assert_eq!(
        dsp::spectral_strength(original, enhanced, 1.0, false),
        enhanced
    );
    let a = Complex32::from_polar(1.0, 179.0_f32.to_radians());
    let b = Complex32::from_polar(1.0, -179.0_f32.to_radians());
    assert!(dsp::spectral_strength(a, b, 0.5, false).re < -0.999);
}

#[test]
fn zero_strength_native_reconstructs_hot_input_at_reported_latency() {
    for quality in [Quality::Live, Quality::HighQuality] {
        let mut engine = Engine::new(48000.0, 1, quality).unwrap();
        assert_eq!(engine.latency(), quality.model_latency() as u32);
        let source: Vec<_> = (0..12000)
            .map(|i| 1.8 * (TAU * 220.0 * i as f32 / 48000.0).sin())
            .collect();
        let mut worst = 0.0_f32;
        for (i, &x) in source.iter().enumerate() {
            let (aligned, out) = engine.tick(0, x, 0.0);
            if i >= quality.model_latency() {
                assert_eq!(aligned, source[i - quality.model_latency()]);
                worst = worst.max((aligned - out).abs());
            }
        }
        assert!(worst < 3e-6, "native reconstruction error: {worst}");
    }
}

#[test]
fn host_rates_reconstruct_in_band_and_match_integer_latency() {
    for quality in [Quality::Live, Quality::HighQuality] {
        for rate in [
            8000.0, 16000.0, 22050.0, 32000.0, 44100.0, 88200.0, 96000.0, 176400.0, 192000.0,
            384000.0,
        ] {
            let mut engine = Engine::new(rate, 1, quality).unwrap();
            let latency = engine.latency() as usize;
            let count = (rate * 0.16) as usize + latency;
            let mut error = 0.0_f64;
            let mut energy = 0.0_f64;
            for i in 0..count {
                let x = 0.4 * (std::f64::consts::TAU * 997.0 * i as f64 / rate).sin() as f32;
                let (aligned, out) = engine.tick(0, x, 0.0);
                if i > latency + 512 {
                    error += ((out - aligned) as f64).powi(2);
                    energy += (aligned as f64).powi(2);
                }
            }
            let relative = (error / energy).sqrt();
            assert!(relative < 0.001, "{rate} Hz aligned RMS error = {relative}");
            engine.reset();
            let mut peak = (0, 0.0_f32);
            for i in 0..latency + 1024 {
                let (_, out) = engine.tick(0, if i == 128 { 1.0 } else { 0.0 }, 0.0);
                if out.abs() > peak.1 {
                    peak = (i, out.abs());
                }
            }
            assert_eq!(peak.0, latency + 128, "{rate} Hz impulse alignment");
        }
    }
}

#[test]
fn resampler_rejects_ultrasonic_aliases_and_preserves_speech_band() {
    fn gain(input_rate: f64, output_rate: f64, frequency: f64) -> f64 {
        let mut resampler =
            resample::Resampler::new(resample::Plan::new(input_rate, output_rate, 0.0));
        let mut energy = 0.0;
        let mut count = 0;
        for i in 0..12000 {
            resampler
                .push((std::f64::consts::TAU * frequency * i as f64 / input_rate).sin() as f32);
            while let Some(value) = resampler.pop() {
                if i > 1024 {
                    energy += (value as f64).powi(2);
                    count += 1;
                }
            }
        }
        (energy * 2.0 / count as f64).sqrt()
    }
    for frequency in [80.0, 1000.0, 6000.0, 16000.0, 20000.0] {
        let measured = gain(96000.0, 48000.0, frequency);
        assert!(
            (measured - 1.0).abs() < 0.01,
            "{frequency} Hz gain: {measured}"
        );
    }
    for frequency in [28000.0, 32000.0, 40000.0] {
        let measured = gain(96000.0, 48000.0, frequency);
        assert!(measured < 0.001, "{frequency} Hz alias gain: {measured}");
    }
    let speech = gain(384000.0, dtln::RATE, 6000.0);
    assert!(speech > 0.98, "DTLN speech-band gain: {speech}");
    let alias = gain(384000.0, dtln::RATE, 10000.0);
    assert!(alias < 0.001, "DTLN alias gain: {alias}");
}

#[test]
fn model_reduces_room_tail_and_retains_direct_onset() {
    for quality in [Quality::Live, Quality::HighQuality] {
        let mut engine = Engine::new(48000.0, 1, quality).unwrap();
        let mut source = vec![0.0; 60000];
        source[0] = 0.8;
        for (i, x) in source.iter_mut().enumerate().skip(960) {
            let t = (i - 960) as f64 / 48000.0;
            *x = (0.3
                * (-t / 0.36).exp()
                * (0.62 * (std::f64::consts::TAU * 311.0 * t).sin()
                    + 0.38 * (std::f64::consts::TAU * 587.0 * t).sin())) as f32;
        }
        let output: Vec<_> = source.iter().map(|&x| engine.tick(0, x, 0.7).1).collect();
        let before: f32 = source[9600..33600].iter().map(|x| x * x).sum();
        let after: f32 = output[9600 + quality.model_latency()..33600 + quality.model_latency()]
            .iter()
            .map(|x| x * x)
            .sum();
        assert!(
            output[quality.model_latency()] / source[0] > 0.25,
            "direct onset lost"
        );
        assert!(
            10.0 * (after / before).log10() < -2.0,
            "late tail was not reduced"
        );
        assert!(output.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
    }
}

#[test]
fn reset_clears_model_resamplers_and_overlap_for_repeatable_renders() {
    for quality in [Quality::Live, Quality::HighQuality] {
        let mut engine = Engine::new(44100.0, 2, quality).unwrap();
        let render = |engine: &mut Engine| -> Vec<[f32; 2]> {
            (0..16000)
                .map(|i| {
                    [
                        engine.tick(0, signal(i, 44100.0), 0.3).1,
                        engine.tick(1, -signal(i, 44100.0), 0.3).1,
                    ]
                })
                .collect()
        };
        let first = render(&mut engine);
        engine.reset();
        let second = render(&mut engine);
        assert_eq!(first, second);
    }
}

#[test]
fn failed_inference_recovers_after_the_active_models_delay() {
    for quality in [Quality::Live, Quality::HighQuality] {
        let mut engine = Engine::new(48000.0, 1, quality).unwrap();
        engine.fail_next_frame(0);
        for i in 0..480 {
            engine.tick(0, signal(i, 48000.0), 1.0);
        }
        assert!(engine.recovering());
        for frame in 0..quality.delay_frames() {
            for i in 0..480 {
                let (_, x) = engine.tick(0, signal(i + (frame + 1) * 480, 48000.0), 1.0);
                assert!(x.is_finite());
            }
            assert!(engine.recovering());
        }
        for _ in 0..480 {
            engine.tick(0, 0.0, 1.0);
        }
        assert!(!engine.recovering());
    }
}

#[test]
fn nonfinite_input_is_sanitized_before_model_and_metering() {
    for quality in [Quality::Live, Quality::HighQuality] {
        let mut engine = Engine::new(44100.0, 1, quality).unwrap();
        let source: Vec<_> = (0..10000)
            .map(|i| if i == 1000 { 0.0 } else { signal(i, 44100.0) })
            .collect();
        let clean: Vec<_> = source.iter().map(|&x| engine.tick(0, x, 0.3)).collect();
        engine.reset();
        for (i, &x) in source.iter().enumerate() {
            assert_eq!(
                engine.tick(0, if i == 1000 { f32::NAN } else { x }, 0.3),
                clean[i]
            );
        }
    }
}

#[test]
fn surround_channels_are_independent_and_centre_channel_is_metered() {
    for quality in [Quality::Live, Quality::HighQuality] {
        let mut engine = Engine::new(48000.0, 6, quality).unwrap();
        let mut energy = [0.0_f64; 6];
        for i in 0..10000 {
            for (channel, sum) in energy.iter_mut().enumerate() {
                let input = if channel == 2 {
                    signal(i, 48000.0)
                } else {
                    0.0
                };
                let (aligned, output) = engine.tick(channel, input, 0.0);
                *sum += (output as f64).powi(2);
                assert!((output - aligned).abs() < 1e-6);
            }
        }
        assert!(energy[2] > 1.0);
        for channel in [0, 1, 3, 4, 5] {
            assert_eq!(energy[channel], 0.0);
        }
    }
}

#[test]
fn fresh_parameter_surface_and_supported_layouts() {
    let params = params::DereverbParams::default();
    let map = params.param_map();
    assert!(map.iter().any(|entry| entry.0 == "strength"));
    assert!(map.iter().any(|entry| entry.0 == "quality"));
    assert!(map.iter().any(|entry| entry.0 == "transient_guard"));
    assert_eq!(params.transient_guard.value(), 100.0);
    assert_eq!(params.strength.value(), 100.0);
    assert_eq!(params.quality.value(), Quality::Live);
    assert!(params.quality.flags().contains(ParamFlags::NON_AUTOMATABLE));
    assert_eq!(Dereverb::AUDIO_IO_LAYOUTS.len(), 16);
    for layout in Dereverb::AUDIO_IO_LAYOUTS {
        assert_eq!(layout.main_input_channels, layout.main_output_channels);
    }
    assert!(Engine::new(f64::NAN, 1, Quality::HighQuality).is_err());
    assert!(Engine::new(48000.0, 17, Quality::HighQuality).is_err());
}

struct Init {
    latency: Cell<u32>,
}
impl InitContext<Dereverb> for Init {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Vst3
    }
    fn execute(&self, _: ()) {}
    fn set_latency_samples(&self, value: u32) {
        self.latency.set(value);
    }
    fn set_current_voice_capacity(&self, _: u32) {}
}

#[test]
fn reinitialization_updates_host_latency_and_status() {
    let mut plugin = Dereverb::default();
    let mut context = Init {
        latency: Cell::new(0),
    };
    for sample_rate in [48000.0, 44100.0, 96000.0] {
        assert!(plugin.initialize(
            &layout(2),
            &BufferConfig {
                sample_rate,
                min_buffer_size: None,
                max_buffer_size: 512,
                process_mode: ProcessMode::Realtime
            },
            &mut context
        ));
        assert_eq!(
            plugin.shared.status.load(Ordering::Relaxed),
            telemetry::ACTIVE
        );
        assert_eq!(
            context.latency.get(),
            plugin.engines[plugin.active_quality.index()]
                .as_ref()
                .unwrap()
                .latency()
        );
    }
}

thread_local! { static ALLOCS: Cell<Option<usize>> = const { Cell::new(None) }; }
struct Allocator;
// SAFETY: allocation/deallocation are delegated unchanged to the system allocator.
unsafe impl std::alloc::GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        ALLOCS.with(|n| {
            if let Some(count) = n.get() {
                n.set(Some(count + 1));
            }
        });
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

#[test]
fn prepared_processing_and_reset_do_not_allocate_in_rust() {
    for (kind, quality) in EngineKind::MODES {
        let mut engine = Engine::prepare(44100.0, 2, kind, quality).unwrap();
        ALLOCS.with(|n| n.set(Some(0)));
        for i in 0..2000 {
            for channel in 0..2 {
                engine.tick(channel, signal(i, 44100.0), 0.3);
            }
        }
        engine.reset();
        let allocations = ALLOCS.with(|n| n.replace(None).unwrap());
        assert_eq!(allocations, 0, "audio path allocated in Rust");
    }
}

#[test]
fn live_switch_preserves_latency_audio_and_history_without_allocating() {
    let mut plugin = Dereverb::default();
    let mut context = Init {
        latency: Cell::new(0),
    };
    assert!(plugin.initialize(
        &layout(2),
        &BufferConfig {
            sample_rate: 48000.0,
            min_buffer_size: None,
            max_buffer_size: 512,
            process_mode: ProcessMode::Realtime,
        },
        &mut context
    ));
    let latency = context.latency.get() as usize;
    assert_eq!(latency, 2880);

    let mut position = 0;
    let mut worst_error = 0.0_f32;
    let quiet_signal = |i: usize| 0.00025 * (TAU * 997.0 * i as f32 / 48000.0).sin();
    for (engine, quality) in [
        (EngineKind::DpdfNet, Quality::Live),
        (EngineKind::NaraWpe, Quality::Live),
        (EngineKind::NaraWpe, Quality::HighQuality),
        (EngineKind::Dtln, Quality::HighQuality),
        (EngineKind::DpdfNet, Quality::HighQuality),
        (EngineKind::DpdfNet, Quality::Live),
    ] {
        let params = Arc::get_mut(&mut plugin.params).unwrap();
        params.engine = EnumParam::new("Engine", engine).non_automatable();
        params.quality = EnumParam::new("Quality", quality).non_automatable();

        ALLOCS.with(|n| n.set(Some(0)));
        plugin.apply_selection();
        for _ in 0..(latency + 480) {
            let mut frame = [0.0, quiet_signal(position)];
            plugin.process_frame(frame.iter_mut(), 0.0);
            let expected = position.checked_sub(latency).map_or(0.0, quiet_signal);
            worst_error = worst_error
                .max(frame[0].abs())
                .max((frame[1] - expected).abs());
            position += 1;
        }
        let allocations = ALLOCS.with(|n| n.replace(None).unwrap());
        assert_eq!(allocations, 0, "mode switching allocated");
        assert_eq!(plugin.active_engine, engine);
        assert_eq!(plugin.active_quality, quality);
        assert_eq!(
            plugin.shared.status.load(Ordering::Relaxed),
            telemetry::ACTIVE
        );
        if engine == EngineKind::Dtln {
            Arc::get_mut(&mut plugin.params).unwrap().quality =
                EnumParam::new("Quality", Quality::Live).non_automatable();
            plugin.apply_selection();
            assert!(
                plugin.transition.is_none(),
                "DTLN restarted for a Quality change"
            );
        }
        assert!(plugin.transition.is_none());
        assert_eq!(plugin.shared.latency.load(Ordering::Relaxed), 2880);
    }
    assert!(
        worst_error < 1e-8,
        "discontinuity in aligned dry signal: {worst_error}"
    );

    let (input, output) = plugin.shared.snapshot();
    // A quiet signal on just one channel still appears on both meters. History
    // must survive each switch instead of filling again from an empty ring.
    let recorded = (position - latency) / plugin.meter_window;
    for (&input, &output) in input.iter().zip(&output).skip(telemetry::POINTS - recorded) {
        assert!(
            (0.00012..0.00013).contains(&input),
            "wrong stereo RMS: {input}"
        );
        assert!((input - output).abs() < 1e-8);
    }

    plugin.reset();
    assert_eq!(
        plugin.shared.snapshot(),
        ([0.0; telemetry::POINTS], [0.0; telemetry::POINTS])
    );
    for _ in 0..(latency + 480) {
        let mut frame = [0.0, 0.0];
        plugin.process_frame(frame.iter_mut(), 1.0);
        assert_eq!(frame, [0.0, 0.0], "reset leaked old audio");
    }
}

#[test]
fn dtln_reduces_noise_without_crosstalk_and_preserves_full_band_dry() {
    for rate in [8000.0, 16000.0, 48000.0, 384000.0] {
        let mut engine = Engine::prepare(rate, 2, EngineKind::Dtln, Quality::Live).unwrap();
        let latency = engine.latency() as usize;
        let mut wet_power = 0.0;
        let mut dry_power = 0.0;
        let mut noise_state = 7u32;
        let mut original = Vec::new();
        for i in 0..(rate as usize / 4 + latency) {
            noise_state = noise_state.wrapping_mul(1664525).wrapping_add(1013904223);
            let noise = (noise_state as i32 as f32) * (0.08 / i32::MAX as f32);
            original.push(noise);
            let (dry, wet) = engine.tick(0, noise, 1.0);
            assert_eq!(
                engine.tick(1, 0.0, 1.0).1,
                0.0,
                "DTLN leaked between channels"
            );
            assert!(wet.is_finite());
            if i >= latency * 2 {
                wet_power += wet * wet;
                dry_power += dry * dry;
            }
        }
        assert!(!engine.recovering(), "DTLN inference failed at {rate} Hz");
        assert!(
            wet_power > 0.0 && wet_power < dry_power * 0.25,
            "{rate} Hz: {wet_power} / {dry_power}"
        );

        engine.reset();
        let replay: Vec<_> = original
            .iter()
            .take(latency + 4096)
            .map(|&sample| engine.tick(0, sample, 1.0).1)
            .collect();
        engine.reset();
        for (&sample, expected) in original.iter().zip(replay) {
            assert_eq!(
                engine.tick(0, sample, 1.0).1,
                expected,
                "{rate} Hz: reset changed DTLN output"
            );
        }

        engine.set_weighting(weighting::Weighting {
            nodes: [0.0; 5],
            ..Default::default()
        });
        engine.reset();
        for (i, &noise) in original.iter().enumerate() {
            let expected = i.checked_sub(latency).map_or(0.0, |i| original[i]);
            assert_eq!(
                engine.tick(0, noise, 1.0).1,
                expected,
                "{rate} Hz: zero curve"
            );
        }
    }
}

#[test]
fn transient_guard_preserves_consonant_energy_across_all_engine_paths() {
    let rate = 48000.0;
    let mut plugin = Dereverb::default();
    let mut context = Init {
        latency: Cell::new(0),
    };
    assert!(plugin.initialize(
        &layout(1),
        &BufferConfig {
            sample_rate: rate,
            min_buffer_size: None,
            max_buffer_size: 512,
            process_mode: ProcessMode::Realtime,
        },
        &mut context,
    ));
    let latency = context.latency.get() as usize;
    assert_eq!(latency, 2880);

    // Low-frequency voiced sound, then a quieter high-band consonant. At 48 kHz
    // this includes energy outside the DTLN path, which the guard can restore.
    let onset = 11_520;
    let mut seed = 11u32;
    let mut low = 0.0;
    let source: Vec<_> = (0..(onset + 8640))
        .map(|i| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let noise = (seed as i32) as f32 / i32::MAX as f32;
            low += 0.42 * (noise - low);
            if i < onset {
                0.19 * (TAU * 170.0 * i as f32 / rate).sin() + 0.002 * noise
            } else if i < onset + 5760 {
                0.085 * (noise - low)
            } else {
                0.0
            }
        })
        .collect();
    let mut render = |engine, quality, percent| {
        let params = Arc::get_mut(&mut plugin.params).unwrap();
        params.engine = EnumParam::new("Engine", engine).non_automatable();
        params.quality = EnumParam::new("Quality", quality).non_automatable();
        params.transient_guard = FloatParam::new(
            "Transient Guard",
            percent,
            FloatRange::Linear {
                min: 0.0,
                max: 100.0,
            },
        )
        .non_automatable();
        plugin.reset();
        assert_eq!(
            plugin.shared.status.load(Ordering::Relaxed),
            telemetry::ACTIVE
        );
        source
            .iter()
            .copied()
            .chain(std::iter::repeat_n(0.0, latency))
            .map(|input| {
                let mut frame = [input];
                plugin.process_frame(frame.iter_mut(), 1.0);
                frame[0]
            })
            .skip(latency)
            .collect::<Vec<_>>()
    };

    for (engine, quality) in EngineKind::MODES {
        let unprotected = render(engine, quality, 0.0);
        let halfway = render(engine, quality, 50.0);
        let protected = render(engine, quality, 100.0);
        for range in [onset..onset + 144, onset + 1920..onset + 3840] {
            let error = |output: &[f32]| {
                range
                    .clone()
                    .map(|i| (output[i] - source[i]).powi(2))
                    .sum::<f32>()
            };
            let before = error(&unprotected);
            let after = error(&protected);
            for i in range.clone() {
                let midpoint = (unprotected[i] + protected[i]) * 0.5;
                assert!(
                    (halfway[i] - midpoint).abs() < 2e-6,
                    "{engine:?} {quality:?}: guard did not scale linearly"
                );
            }
            assert!(
                before > 1e-5 && after < 0.25 * before,
                "{engine:?} {quality:?} {range:?}: {after} / {before}"
            );
        }
        // The guard has already released on the unchanged vowel.
        assert_eq!(
            &protected[8000..onset - 256],
            &unprotected[8000..onset - 256]
        );
    }
}
