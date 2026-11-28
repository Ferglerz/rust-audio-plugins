//! Reproduce real-time load, electrical headroom and the held-out key renders.
use electra_piano::{
    circuit::Controls,
    live::{Engine, HIGH, LOW},
};
use std::{error::Error, hint::black_box, path::Path, time::Instant};

fn register(n: usize) -> Vec<u8> {
    // Cluster in the middle; use the entire compass for dense chords.
    let low = usize::from(LOW) + (72 - n) / 2;
    (low..(low + n)).map(|x| x as u8).collect()
}

fn bench() {
    println!("rate,notes,p50_deadline_pct,p99_pct,max_pct,misses,aux_peak_v,speaker_peak_v");
    for rate in [44100., 48000., 96000.] {
        for count in [1, 8, 16, 32, 72] {
            let mut engine = Engine::new(rate);
            let mut maxima = [0_f64; 2];
            let frames = 128;
            let blocks = (rate as usize).div_ceil(frames);
            let mut times = Vec::with_capacity(blocks * 3);
            let notes = register(count);
            for _ in 0..3 {
                engine.reset();
                for block in 0..blocks {
                    let start = Instant::now();
                    if block == 0 {
                        for &n in &notes {
                            engine.note_on(n, 1.);
                        }
                    }
                    for _ in 0..frames {
                        let out = black_box(engine.next_outputs());
                        maxima[0] = maxima[0].max(out.auxiliary_v.abs());
                        maxima[1] = maxima[1].max(out.speaker_v.abs());
                    }
                    times.push(100. * start.elapsed().as_secs_f64() * rate / (frames as f64));
                }
            }
            times.sort_unstable_by(f64::total_cmp);
            let misses = times.iter().filter(|&&t| t > 100.).count();
            println!(
                "{rate},{count},{:.2},{:.2},{:.2},{misses},{:.7},{:.5}",
                times[times.len() / 2],
                times[times.len() * 99 / 100],
                times[times.len() - 1],
                maxima[0],
                maxima[1]
            );
        }
    }
    let mut e = Engine::new(48000.);
    let mut c = Controls::default();
    let start = Instant::now();
    for i in 0..2000 {
        c.volume = 0.40 + f64::from(i % 200) * 0.0025;
        e.set_controls(c);
    }
    println!(
        "Changed controls, mean compile {:.2} us",
        start.elapsed().as_secs_f64() * 1e6 / 2000.
    );
    e.set_controls(Controls::default());
    for note in [LOW, 48, 65, 80, HIGH] {
        e.reset();
        e.note_on(note, 0.5);
        let mut peak = 0_f64;
        let y: Vec<_> = (0..4800)
            .map(|_| {
                let v = e.next_outputs().auxiliary_v;
                peak = peak.max(v.abs());
                v
            })
            .collect();
        let n = y.iter().position(|v| v.abs() > peak * 0.01).unwrap_or(0);
        println!("1 percent of 100ms peak, MIDI {note}: {n} samples");
    }
    for count in [1, 8, 16, 32, 72] {
        e.reset();
        e.set_controls(Controls {
            volume: 1.,
            basic_volume: 1.,
            ..Default::default()
        });
        for n in register(count) {
            e.note_on(n, 1.);
        }
        let mut peak = 0_f64;
        for _ in 0..48000 {
            peak = peak.max(e.next_outputs().auxiliary_v.abs());
        }
        println!("Full P9 and player volume, {count} voices: {peak:.6} V");
    }
}

fn captures(dir: &Path) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(dir)?;
    let rate = 48000.;
    let mut engine = Engine::new(rate);
    for note in LOW..=HIGH {
        engine.reset();
        engine.note_on(note, 0.5);
        let mut w = hound::WavWriter::create(
            dir.join(format!("key-{note}.wav")),
            hound::WavSpec {
                channels: 2,
                sample_rate: rate as u32,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )?;
        for _ in 0..64000 {
            let out = engine.next_outputs();
            w.write_sample(out.auxiliary_v as f32)?;
            w.write_sample(out.speaker_v as f32)?;
        }
        w.finalize()?;
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [command] if command == "bench" => {
            bench();
            Ok(())
        }
        [command, dir] if command == "captures" => captures(Path::new(dir)),
        _ => Err("expected bench or captures OUTPUT_DIRECTORY".into()),
    }
}
