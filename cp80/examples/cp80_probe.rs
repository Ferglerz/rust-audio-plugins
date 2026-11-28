//! Generate a single offline bearing-force trace at four times the listening rate.
use cp80::{Key, KeySpec};
use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let mut a: Vec<_> = env::args().skip(1).collect();
    let fitting = a.first().is_some_and(|s| s == "fit");
    if fitting {
        a.remove(0);
    }
    match a.first().map(String::as_str) {
        Some("bench") => return bench(a.get(1).is_some_and(|model| model == "cp70b")),
        Some("demo") => return demo(a.get(1).ok_or("missing WAV path")?),
        Some("play") => return play(&a[1..]),
        _ => {}
    }
    if a.first().is_some_and(|v| v == "panel") {
        return panel_impulse(&a[1..]);
    }
    if a.len() < 5 {
        return Err("usage: cp80 MIDI VELOCITY_0_1 HOST_RATE HOLD_SECONDS OUT.wav [contact_k strike_fraction hammer_kg [linear_N_per_m]]".into());
    }
    let mut spec = KeySpec::for_note(a[0].parse()?).ok_or("unsupported prototype pitch")?;
    let velocity: f64 = a[1].parse()?;
    let rate: u32 = a[2].parse()?;
    let hold: f64 = a[3].parse()?;
    if !(22050..=192000).contains(&rate)
        || !(0.0..=60.0).contains(&hold)
        || !(0.0..=1.).contains(&velocity)
    {
        return Err("sample rate, hold time or velocity outside probe range".into());
    }
    if !matches!(a.len(), 5 | 8 | 9) {
        return Err("supply all three mechanical overrides together".into());
    }
    if a.len() >= 8 {
        spec.contact_k = a[5].parse()?;
        spec.strike_fraction = a[6].parse()?;
        spec.hammer_kg = a[7].parse()?;
        if let Some(linear) = a.get(8) {
            spec.contact_linear = linear.parse()?;
        }
        if !(0.0..=1e7).contains(&spec.contact_linear)
            || !(1e5..=1e10).contains(&spec.contact_k)
            || !(0.02..=0.5).contains(&spec.strike_fraction)
            || !(0.001..=0.04).contains(&spec.hammer_kg)
        {
            return Err("invalid mechanical override".into());
        }
    }
    let rate = rate * 4;
    let mut key = Key::new(spec, rate as f64);
    let wav = hound::WavSpec {
        channels: 1,
        sample_rate: rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(&a[4], wav)?;
    key.strike(velocity);
    let release = (hold * rate as f64).round() as usize;
    let tail = if fitting { 0 } else { rate as usize / 3 };
    for i in 0..release + tail {
        if i == release {
            key.release();
        }
        writer.write_sample(key.step() as f32)?;
    }
    writer.finalize()?;
    Ok(())
}

fn panel_impulse(a: &[String]) -> Result<(), Box<dyn Error>> {
    use cp80::panel::{Controls, Panel};
    if a.len() != 8 {
        return Err("panel HOST_RATE BASS MID TREBLE BRILLIANCE VOLUME UPPER OUT.wav".into());
    }
    let rate: u32 = a[0].parse()?;
    let c = Controls {
        bass: a[1].parse()?,
        middle: a[2].parse()?,
        treble: a[3].parse()?,
        brilliance: a[4].parse()?,
        volume: a[5].parse()?,
        ..Default::default()
    };
    let upper = a[6] == "1";
    let mut panel = Panel::new(4. * rate as f64);
    panel.set(c);
    let w = hound::WavSpec {
        channels: 1,
        sample_rate: 4 * rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut file = hound::WavWriter::create(&a[7], w)?;
    for i in 0..4 * rate {
        let act = f64::from(i == 0);
        file.write_sample(
            panel.tick(if upper { 0. } else { act }, if upper { act } else { 0. }) as f32,
        )?;
    }
    file.finalize()?;
    Ok(())
}

fn play(a: &[String]) -> Result<(), Box<dyn Error>> {
    use cp80::engine::Engine;
    if a.len() != 5 {
        return Err("play MIDI VELOCITY RATE HOLD OUT.wav".into());
    }
    let rate: u32 = a[2].parse()?;
    let note: u8 = a[0].parse()?;
    let velocity: f32 = a[1].parse()?;
    let hold = (a[3].parse::<f64>()? * rate as f64).round() as usize;
    let mut engine = Engine::new(rate as f64);
    let mut w = hound::WavWriter::create(
        &a[4],
        hound::WavSpec {
            channels: 2,
            sample_rate: rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    engine.note_on(note, velocity);
    for i in 0..hold + rate as usize / 3 {
        if i == hold {
            engine.note_off(note);
        }
        for y in engine.sample() {
            w.write_sample(y)?;
        }
    }
    w.finalize()?;
    Ok(())
}
fn demo(path: &str) -> Result<(), Box<dyn Error>> {
    use cp80::{engine::Engine, panel::Controls};
    const FS: u32 = 48000;
    let mut e = Engine::new(FS as f64);
    let mut wave = Vec::new();
    for tremolo in [false, true] {
        e.set(Controls {
            tremolo,
            speed: 0.85,
            depth: 0.8,
            ..Default::default()
        });
        for chord in [
            [38, 50, 57, 64, 65, 69],
            [34, 46, 53, 60, 62, 65],
            [41, 48, 55, 57, 60, 64],
            [36, 48, 55, 62, 65, 69],
        ] {
            e.sustain(true);
            for (j, n) in chord.into_iter().enumerate() {
                e.note_on(n, 0.48 + j as f32 * 0.065);
                for _ in 0..FS / 14 {
                    wave.push(e.sample());
                }
                e.note_off(n);
            }
            for _ in 0..FS / 3 {
                wave.push(e.sample());
            }
            e.sustain(false);
            for _ in 0..FS / 6 {
                wave.push(e.sample());
            }
        }
    }
    let peak = wave.iter().flatten().fold(0.0f32, |x, y| x.max(y.abs()));
    let mut w = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate: FS,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        },
    )?;
    for s in wave.into_iter().flatten() {
        w.write_sample((s * (0.9 / peak) * 8388607.).round() as i32)?;
    }
    w.finalize()?;
    Ok(())
}
fn bench(cp70b: bool) -> Result<(), Box<dyn Error>> {
    use cp80::engine::{Engine, HIGH, LOW};
    use std::time::Instant;
    for rate in [44100, 48000, 96000] {
        let (mut e, keys) = if cp70b {
            (
                Engine::cp70b(rate as f64, false),
                cp80::seventy::LOW..=cp80::seventy::HIGH,
            )
        } else {
            (Engine::new(rate as f64), LOW..=HIGH)
        };
        let mut blocks = Vec::new();
        let mut peak = 0f32;
        e.sustain(true);
        for n in keys.clone() {
            e.note_on(n, 0.8);
            e.note_off(n);
        }
        let elapsed = Instant::now();
        for _ in 0..rate / 64 {
            let start = Instant::now();
            for _ in 0..64 {
                let v = e.sample();
                if !v[0].is_finite() {
                    return Err("nonfinite".into());
                }
                peak = peak.max(v[0].abs());
            }
            blocks.push(start.elapsed().as_secs_f64() * 1000.);
        }
        let total = elapsed.elapsed().as_secs_f64();
        blocks.sort_by(f64::total_cmp);
        println!("{rate} Hz, {} keys, {total:.3}s/audio s, 64 frames p99 {:.3} ms, worst {:.3} ms, budget {:.3} ms, peak {peak:.4}",
                 keys.count(),blocks[blocks.len()*99/100],blocks[blocks.len()-1],64000./rate as f64);
        e.sustain(false);
        for _ in 0..rate {
            if !e.sample()[0].is_finite() {
                return Err("nonfinite release".into());
            }
        }
    }
    Ok(())
}
