//! Capture all three jack voltages with a shared origin and common gain.
use hohner_duo::{Controls, Engine, Split, HIGH, LAST_BASS, LOW};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{env, error::Error, fs, path::Path, time::Instant};

fn benchmark() {
    for rate in [44_100, 48_000, 96_000] {
        let mut engine = Engine::new(f64::from(rate));
        let mut block = [0.0; 128];
        for count in [1, 8, 16, 32, 60] {
            engine.reset();
            for i in 0..count {
                // Span the instrument, including the long bass and short treble grids.
                let key = if count == 1 {
                    64
                } else {
                    LOW + (i * u16::from(HIGH - LOW) / (count - 1)) as u8
                };
                engine.note_on(key, 0.7);
            }
            engine.render(&mut block);
            let deadline = block.len() as f64 / f64::from(rate);
            let mut times = Vec::new();
            let mut peak = 0.0_f32;
            for _ in 0..rate / block.len() as u32 {
                let start = Instant::now();
                engine.render(&mut block);
                times.push(start.elapsed().as_secs_f64() / deadline);
                for &sample in &block {
                    assert!(sample.is_finite());
                    peak = peak.max(sample.abs());
                }
            }
            times.sort_by(f64::total_cmp);
            let mean = times.iter().sum::<f64>() / times.len() as f64;
            let p99 = times[times.len() * 99 / 100];
            let late = times.iter().filter(|&&t| t > 1.).count();
            println!("{rate} Hz, {count:2} notes, mean {:.1}% and p99 {:.1}% of deadline, {late} late blocks, {peak:.3} V peak", 100. * mean, 100. * p99);
        }
    }
}

fn capture(
    dir: &Path,
    name: &str,
    notes: &[(f64, u8, f32)],
    changes: &[(f64, Controls)],
    rate: u32,
) -> Result<(), Box<dyn Error>> {
    let mut e = Engine::new(rate.into());
    let wav = WavSpec {
        channels: 3,
        sample_rate: rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut w = WavWriter::create(dir.join(format!("{name}.wav")), wav)?;
    let mut notes = notes.iter().copied().peekable();
    let mut changes = changes.iter().copied().peekable();
    for i in 0..rate * 2 {
        while let Some((_, c)) = changes.next_if(|(t, _)| (t * f64::from(rate)).round() as u32 == i)
        {
            e.set_controls(c);
        }
        while let Some((_, n, v)) =
            notes.next_if(|(t, _, _)| (t * f64::from(rate)).round() as u32 == i)
        {
            e.note_on(n, v);
        }
        for x in e.jacks() {
            w.write_sample(x as f32)?;
        }
    }
    w.finalize()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let arg = env::args()
        .nth(1)
        .ok_or("expected output directory or --bench")?;
    if arg == "--bench" {
        benchmark();
        return Ok(());
    }
    let dir = Path::new(&arg);
    fs::create_dir_all(dir)?;
    let stock = Controls::default();
    for n in [
        LOW,
        31,
        33,
        36,
        37,
        39,
        40,
        41,
        43,
        44,
        45,
        46,
        LAST_BASS,
        LAST_BASS + 1,
        51,
        52,
        55,
        58,
        59,
        60,
        61,
        62,
        64,
        66,
        68,
        70,
        72,
        76,
        78,
        HIGH,
    ] {
        for (suffix, vel) in [("soft", 32.), ("medium", 80.), ("hard", 120.)] {
            capture(
                dir,
                &format!("{n}-{suffix}"),
                &[(0.05, n, vel / 127.), (0.80, n, 0.)],
                &[],
                44100,
            )?;
        }
    }
    let notes = [
        (0.05, 48, 0.7),
        (0.05, 49, 0.7),
        (0.50, 48, 0.),
        (0.9, 48, 0.85),
        (1.4, 49, 0.),
        (1.6, 48, 0.),
    ];
    capture(
        dir,
        "held-split",
        &notes,
        &[
            (
                0.,
                Controls {
                    split: Split::ClavinetTreble,
                    ..stock
                },
            ),
            (
                0.25,
                Controls {
                    split: Split::PianetTreble,
                    ..stock
                },
            ),
            (0.75, stock),
            (
                1.1,
                Controls {
                    mute: true,
                    ..stock
                },
            ),
        ],
        44100,
    )?;
    capture(
        dir,
        "foot",
        &[(0.05, 60, 0.7), (0.9, 60, 0.)],
        &[
            (
                0.,
                Controls {
                    tab_two: false,
                    tab_three: false,
                    ..stock
                },
            ),
            (
                0.35,
                Controls {
                    tab_two: false,
                    tab_three: false,
                    foot: true,
                    ..stock
                },
            ),
            (0.65, stock),
        ],
        44100,
    )?;
    Ok(())
}
