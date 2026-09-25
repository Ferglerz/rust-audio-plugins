//! Release CPU split: reed / pickup / circuit / speaker.
//!
//! ```sh
//! cargo test --manifest-path vendor/openwurli-dsp/Cargo.toml --release \
//!   --test cpu_split_profile -- --ignored --nocapture
//! ```
use openwurli_dsp::dk_preamp::DkPreamp;
use openwurli_dsp::oversampler::Oversampler;
use openwurli_dsp::power_amp::PowerAmp;
use openwurli_dsp::preamp::PreampModel;
use openwurli_dsp::speaker::Speaker;
use openwurli_dsp::tables;
use openwurli_dsp::tremolo::Tremolo;
use openwurli_dsp::voice::Voice;
use openwurli_dsp::{CpuSplit, WurliEngine};
use std::time::Instant;

const SR: f64 = 44_100.0;
const BLOCK: usize = 64;
const AUDIO_SECS: f64 = 2.0;

const CHORD_6: [u8; 6] = [48, 55, 60, 63, 67, 70];
const HOLD_32: [u8; 32] = [
    36, 38, 40, 41, 43, 45, 47, 48, 50, 52, 53, 55, 57, 59, 60, 62, 64, 65, 67, 69, 71, 72, 74, 76,
    77, 79, 81, 83, 84, 86, 88, 89,
];

#[test]
#[ignore = "perf probe; run explicitly in release"]
fn cpu_split_chord_and_pad() {
    println!(
        "cpu-split  sr={SR}  block={BLOCK}  audio={AUDIO_SECS}s  {:?}",
        build_label()
    );
    profile_scenario("6-note ff chord", &CHORD_6, 0.95);
    profile_scenario("32-note hold", &HOLD_32, 0.75);
}

fn profile_scenario(name: &str, notes: &[u8], velocity: f32) {
    let mut engine = WurliEngine::new(SR);
    engine.set_volume(0.5);
    engine.set_tremolo_depth(0.5);
    engine.set_mlp_enabled(false);
    engine.ensure_buffer_capacity(BLOCK);
    engine.warm_up();
    for &note in notes {
        engine.note_on(note, velocity);
    }

    let total = (SR * AUDIO_SECS) as usize;
    let mut buf = vec![0.0f32; BLOCK];
    let mut split = CpuSplit::default();
    let mut rendered = 0usize;
    let wall = Instant::now();
    while rendered < total {
        let take = (total - rendered).min(BLOCK);
        split.add_assign(&engine.profile_render(&mut buf[..take]));
        rendered += take;
    }
    let wall_s = wall.elapsed().as_secs_f64();

    println!();
    println!("=== {name}  voices={} ===", notes.len());
    print_split(&split, AUDIO_SECS, wall_s);
    isolated_circuit_split(name, notes, velocity);
}

fn print_split(split: &CpuSplit, audio_s: f64, wall_s: f64) {
    let rows = [
        ("reed", split.reed_ns),
        ("noise", split.noise_ns),
        ("pickup", split.pickup_ns),
        ("mix", split.mix_ns),
        ("upsample", split.upsample_ns),
        ("circuit (tremolo+preamp+amp)", split.circuit_ns),
        ("downsample", split.downsample_ns),
        ("speaker", split.speaker_ns),
    ];
    let accounted = split.accounted_ns() as f64 / 1e9;
    println!(
        "  wall {wall_s:.3}s for {audio_s:.1}s audio  ({:.1}% realtime)",
        100.0 * wall_s / audio_s
    );
    println!("  accounted {accounted:.3}s  (timers; wall includes Instant overhead)");
    for (label, ns) in rows {
        let s = ns as f64 / 1e9;
        println!(
            "  {label:<32} {s:7.3}s  {:>5.1}% of wall  {:>5.1}% of audio",
            100.0 * s / wall_s,
            100.0 * s / audio_s
        );
    }
}

/// Replay one captured block through each shared stage so preamp vs amp
/// vs tremolo vs speaker can be ranked without per-sample clocks.
fn isolated_circuit_split(name: &str, notes: &[u8], velocity: f32) {
    let capture = 2048usize;
    let mut sum = vec![0.0f64; capture];
    let mut scratch = vec![0.0f64; capture];
    for (i, &note) in notes.iter().enumerate() {
        let seed = (note as u32)
            .wrapping_mul(2654435761)
            .wrapping_add(i as u32);
        let mut voice = Voice::note_on(note, velocity as f64, SR, seed, false);
        voice.render(&mut scratch);
        for (dst, src) in sum.iter_mut().zip(scratch.iter()) {
            *dst += *src;
        }
    }

    let repeats = ((SR * AUDIO_SECS) as usize).div_ceil(capture);
    let audio_s = repeats as f64 * capture as f64 / SR;

    let mut os = Oversampler::new();
    let mut up = vec![0.0f64; capture * 2];
    let mut down = vec![0.0f64; capture];
    os.upsample_2x(&sum, &mut up);
    let captured_up = up.clone();
    let t = Instant::now();
    for _ in 0..repeats {
        os.upsample_2x(&sum, &mut up);
        os.downsample_2x(&up, &mut down);
    }
    let os_s = t.elapsed().as_secs_f64();

    let mut trem = Tremolo::new(0.5, SR * 2.0);
    let t = Instant::now();
    for _ in 0..repeats {
        for _ in 0..capture * 2 {
            let _ = trem.process();
        }
    }
    let trem_s = t.elapsed().as_secs_f64();

    let mut preamp = DkPreamp::new(SR * 2.0);
    let t = Instant::now();
    for _ in 0..repeats {
        for &sample in &captured_up {
            let _ = preamp.process_sample(sample);
        }
    }
    let preamp_s = t.elapsed().as_secs_f64();

    let mut amp = PowerAmp::new_at_sample_rate(SR * 2.0);
    let t = Instant::now();
    for _ in 0..repeats {
        for &sample in &captured_up {
            let _ = amp.process(sample * tables::FIXED_CIRCUIT_DRIVE);
        }
    }
    let amp_s = t.elapsed().as_secs_f64();

    let mut speaker = Speaker::new(SR);
    let t = Instant::now();
    for _ in 0..repeats {
        for &sample in &sum {
            let _ = speaker.process(sample);
        }
    }
    let speaker_s = t.elapsed().as_secs_f64();

    println!("  isolated replay ({audio_s:.1}s equivalent, {name}):");
    for (label, s) in [
        ("oversampler up+down", os_s),
        ("tremolo", trem_s),
        ("preamp (legacy, on OS buffer)", preamp_s),
        ("power amp (behavioral, OS)", amp_s),
        ("speaker", speaker_s),
    ] {
        println!(
            "    {label:<32} {s:7.3}s  {:>5.1}% of audio",
            100.0 * s / audio_s
        );
    }
}

fn build_label() -> &'static str {
    if cfg!(feature = "melange-preamp") {
        "melange-preamp"
    } else {
        "legacy preamp + behavioral amp"
    }
}
