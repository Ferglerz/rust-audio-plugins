use fundament::dsp::*;

const SR: f32 = 48_000.0;
const BLOCK: usize = 512;

struct Stereo {
    left: Vec<f32>,
    right: Vec<f32>,
}

fn prepare(settings: EngineSettings) -> FundamentEngine {
    let mut engine = FundamentEngine::new();
    engine.set_settings(settings);
    engine.prepare(SR, BLOCK, 2);
    engine
}

fn process(engine: &mut FundamentEngine, input: &Stereo) -> Stereo {
    let mut output = Stereo {
        left: vec![0.0; input.left.len()],
        right: vec![0.0; input.right.len()],
    };
    let mut offset = 0;
    while offset < input.left.len() {
        let n = (input.left.len() - offset).min(BLOCK);
        let mut left = input.left[offset..offset + n].to_vec();
        let mut right = input.right[offset..offset + n].to_vec();
        engine.process_block(&mut [&mut left[..], &mut right[..]]);
        output.left[offset..offset + n].copy_from_slice(&left);
        output.right[offset..offset + n].copy_from_slice(&right);
        offset += n;
    }
    output
}

fn silence(frames: usize) -> Stereo {
    Stereo {
        left: vec![0.0; frames],
        right: vec![0.0; frames],
    }
}

fn sine(freq_hz: f32, amp: f32, frames: usize) -> Stereo {
    let mut tone = Stereo {
        left: vec![0.0; frames],
        right: vec![0.0; frames],
    };
    for i in 0..frames {
        let y = amp * (std::f32::consts::TAU * freq_hz * i as f32 / SR).sin();
        tone.left[i] = y;
        tone.right[i] = y;
    }
    tone
}

fn add_partials(into: &mut [f32], f0_hz: f32, harmonics: usize, amp: f32) {
    for harmonic in 1..=harmonics {
        let freq = f0_hz * harmonic as f32;
        let scale = amp / harmonic as f32;
        for (i, sample) in into.iter_mut().enumerate() {
            *sample += scale * (std::f32::consts::TAU * freq * i as f32 / SR).sin();
        }
    }
}

fn stereo_from_mono(mono: Vec<f32>) -> Stereo {
    Stereo {
        left: mono.clone(),
        right: mono,
    }
}

fn goertzel_amp(samples: &[f32], freq_hz: f32) -> f32 {
    let omega = std::f32::consts::TAU * freq_hz / SR;
    let coeff = 2.0 * omega.cos();
    let mut s1 = 0.0f32;
    let mut s2 = 0.0f32;
    for &sample in samples {
        let s0 = sample + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    let real = s1 - s2 * omega.cos();
    let imag = s2 * omega.sin();
    2.0 * (real * real + imag * imag).sqrt() / samples.len() as f32
}

fn level_db(samples: &[f32], freq_hz: f32) -> f32 {
    20.0 * goertzel_amp(samples, freq_hz).max(1.0e-12).log10()
}

fn tail<'a>(samples: &'a [f32], seconds: f32) -> &'a [f32] {
    let n = (seconds * SR) as usize;
    &samples[samples.len() - n..]
}

fn active(engine: &FundamentEngine) -> Vec<VoiceState> {
    engine
        .voices()
        .iter()
        .copied()
        .filter(|voice| voice.active)
        .collect()
}

#[test]
fn silence_stays_silent_with_no_voices() {
    let mut engine = prepare(EngineSettings::default());
    let output = process(&mut engine, &silence(SR as usize));
    for sample in output.left.iter().chain(output.right.iter()) {
        assert!(sample.abs() < 1.0e-6, "{sample}");
    }
    assert!(active(&engine).is_empty());
}

#[test]
fn dry_mix_delays_input_by_stft_latency() {
    let mut settings = EngineSettings::default();
    settings.mix = 0.0;
    let mut engine = prepare(settings);
    let latency = engine.latency_samples();
    assert_eq!(latency, 2048);

    let frames = 6_144;
    let mut input = silence(frames);
    for i in 0..frames {
        input.left[i] = (i as f32 * 0.01).sin();
        input.right[i] = (i as f32 * 0.017).sin() * 0.5;
    }
    let output = process(&mut engine, &input);
    let latency = latency as usize;
    for i in 0..frames {
        for (out, inp) in [(&output.left, &input.left), (&output.right, &input.right)] {
            let expected = if i >= latency { inp[i - latency] } else { 0.0 };
            assert!((out[i] - expected).abs() < 1.0e-6, "frame {i}");
        }
    }
}

#[test]
fn sine_tracks_one_fundamental() {
    let mut engine = prepare(EngineSettings::default());
    let _ = process(&mut engine, &sine(110.0, 0.5, (2.0 * SR) as usize));
    let voices = active(&engine);
    assert_eq!(voices.len(), 1, "{voices:?}");
    assert!(
        (voices[0].f0_hz - 110.0).abs() <= 1.0,
        "{}",
        voices[0].f0_hz
    );
}

#[test]
fn cut_drops_sine_at_least_10_db() {
    let mut settings = EngineSettings::default();
    settings.cut_depth_db = -24.0;
    settings.synth_level_db = -120.0;
    settings.mix = 1.0;
    let mut engine = prepare(settings);
    let input = sine(110.0, 0.5, (2.0 * SR) as usize);
    let output = process(&mut engine, &input);
    let input_db = level_db(tail(&input.left, 0.5), 110.0);
    let output_db = level_db(tail(&output.left, 0.5), 110.0);
    assert!(
        output_db <= input_db - 10.0,
        "out {output_db} in {input_db}"
    );
}

#[test]
fn synth_replaces_cut_fundamental() {
    let mut settings = EngineSettings::default();
    settings.cut_depth_db = -36.0;
    settings.synth_level_db = 0.0;
    settings.tone = 0.0;
    settings.mix = 1.0;
    let mut engine = prepare(settings);
    let input = sine(110.0, 0.5, (2.0 * SR) as usize);
    let output = process(&mut engine, &input);
    let input_db = level_db(tail(&input.left, 0.5), 110.0);
    let output_db = level_db(tail(&output.left, 0.5), 110.0);
    assert!(
        (output_db - input_db).abs() <= 4.0,
        "out {output_db} in {input_db}"
    );
}

#[test]
fn saw_tracks_fundamental_not_octave() {
    let mut settings = EngineSettings::default();
    settings.harmonics = 12;
    let mut engine = prepare(settings);
    let mut mono = vec![0.0; (2.0 * SR) as usize];
    add_partials(&mut mono, 82.41, 12, 0.4);
    let _ = process(&mut engine, &stereo_from_mono(mono));
    let voices = active(&engine);
    assert_eq!(voices.len(), 1, "{voices:?}");
    assert!(
        (voices[0].f0_hz - 82.41).abs() <= 1.0,
        "{}",
        voices[0].f0_hz
    );
}

#[test]
fn dyad_tracks_both_fundamentals() {
    let mut settings = EngineSettings::default();
    settings.max_voices = 2;
    settings.harmonics = 6;
    let mut engine = prepare(settings);
    let mut mono = vec![0.0; (2.0 * SR) as usize];
    add_partials(&mut mono, 110.0, 6, 0.3);
    add_partials(&mut mono, 138.6, 6, 0.3);
    let _ = process(&mut engine, &stereo_from_mono(mono));
    let voices = active(&engine);
    let has = |hz: f32| voices.iter().any(|voice| (voice.f0_hz - hz).abs() <= 1.5);
    assert!(has(110.0) && has(138.6), "{voices:?}");
}

#[test]
fn glide_keeps_at_most_two_ids_and_lands_on_target() {
    let mut engine = prepare(EngineSettings::default());
    let glide = (1.5 * SR) as usize;
    let frames = glide + (0.5 * SR) as usize;
    let mut input = silence(frames);
    let mut phase = 0.0f64;
    for i in 0..frames {
        let hz = if i < glide {
            110.0 + 20.0 * (i as f64 / glide as f64)
        } else {
            130.0
        };
        let y = (0.5 * (std::f64::consts::TAU * phase).sin()) as f32;
        phase += hz / f64::from(SR);
        input.left[i] = y;
        input.right[i] = y;
    }

    let mut ids = [0u32; 8];
    let mut id_count = 0usize;
    let mut offset = 0;
    while offset < frames {
        let n = (frames - offset).min(BLOCK);
        let mut left = input.left[offset..offset + n].to_vec();
        let mut right = input.right[offset..offset + n].to_vec();
        engine.process_block(&mut [&mut left[..], &mut right[..]]);
        for voice in engine.voices().iter().filter(|voice| voice.active) {
            if voice.id != 0 && !ids[..id_count].contains(&voice.id) && id_count < ids.len() {
                ids[id_count] = voice.id;
                id_count += 1;
            }
        }
        offset += n;
    }

    assert!(id_count <= 2, "ids {}", id_count);
    let voices = active(&engine);
    assert!(!voices.is_empty(), "no voice at end");
    for voice in &voices {
        assert!((voice.f0_hz - 130.0).abs() <= 2.0, "{}", voice.f0_hz);
    }
}

#[test]
fn noise_stays_finite_under_full_polyphony() {
    let mut settings = EngineSettings::default();
    settings.max_voices = MAX_VOICES;
    settings.harmonics = MAX_HARMONICS;
    let mut engine = prepare(settings);

    let frames = (2.0 * SR) as usize;
    let mut input = silence(frames);
    let mut seed = 1u32;
    for i in 0..frames {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        input.left[i] = (f64::from(seed) / f64::from(u32::MAX) * 2.0 - 1.0) as f32;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        input.right[i] = (f64::from(seed) / f64::from(u32::MAX) * 2.0 - 1.0) as f32;
    }
    let output = process(&mut engine, &input);
    let mut peak = 0.0f32;
    for sample in output.left.iter().chain(output.right.iter()) {
        assert!(sample.is_finite(), "{sample}");
        peak = peak.max(sample.abs());
    }
    assert!(peak < 4.0, "{peak}");
}
