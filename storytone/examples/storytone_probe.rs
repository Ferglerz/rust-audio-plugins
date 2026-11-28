//! Render terminal observations or AC impulses for the 1940 service circuit.
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};
use storytone::{
    amp::{Amplifier, Controls},
    engine::{Engine, HIGH, LOW},
};

fn wave(dir: &Path) -> Result<(), Box<dyn Error>> {
    let sr = 48000.;
    let mut e = Engine::new(sr);
    for n in [21, 35, 50, 55, 57, 59, 64, 69, 74, 79, 81, 83, 86, 96, 108] {
        for vel in [0.30, 0.60, 0.90] {
            let mut w = hound::WavWriter::create(
                dir.join(format!("{n}-{vel:.2}.wav")),
                hound::WavSpec {
                    channels: 4,
                    sample_rate: 48000,
                    bits_per_sample: 32,
                    sample_format: hound::SampleFormat::Float,
                },
            )?;
            e.reset();
            e.note_on(n, vel);
            for i in 0..72000 {
                if i == 60000 {
                    e.note_off(n);
                }
                let x = e.next_outputs();
                for a in [
                    x.pickups_v[0] + x.pickups_v[1],
                    x.amplifier.grid_v,
                    x.amplifier.speaker_v,
                    x.amplifier.auxiliary_v,
                ] {
                    assert!(a.is_finite());
                    w.write_sample(a as f32)?;
                }
            }
            w.finalize()?;
        }
    }
    let mut w = hound::WavWriter::create(
        dir.join("storytone-audition.wav"),
        hound::WavSpec {
            channels: 1,
            sample_rate: 48000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    e.reset();
    for (j, notes) in [[50, 57, 64], [55, 59, 67], [59, 66, 74], [62, 69, 81]]
        .iter()
        .enumerate()
    {
        e.set_sustain(true);
        e.set_soft(j == 2);
        for &n in notes {
            e.note_on(n, 0.60);
        }
        for i in 0..48000 {
            if i == 22000 {
                for &n in notes {
                    e.note_off(n);
                }
            }
            if i == 26000 {
                e.set_controls(Controls {
                    swell: 0.5,
                    ..Controls::default()
                });
            }
            if i == 34500 {
                e.set_sustain(false);
            }
            let x = e.next_outputs();
            w.write_sample(x.amplifier.auxiliary_v as f32)?;
        }
    }
    w.finalize()?;
    Ok(())
}
fn impulses(dir: &Path) -> Result<(), Box<dyn Error>> {
    for (i, (master, swell, opt)) in [
        (0., 0., 0.7),
        (0., 1., 0.7),
        (0.5, 0., 0.7),
        (0.5, 0.5, 0.7),
        (0.5, 1., 0.7),
        (1., 0., 0.7),
        (1., 1., 1.),
    ]
    .into_iter()
    .enumerate()
    {
        let mut a = Amplifier::new(
            96000.,
            Controls {
                optimum: opt,
                manual: master,
                swell,
            },
        );
        let mut w = BufWriter::new(File::create(dir.join(format!("transfer-{i}.bin")))?);
        for j in 0..131072 {
            let o = a.linear([f64::from(j == 0), 0.]);
            w.write_all(&o.speaker_v.to_le_bytes())?;
        }
    }
    let mut a = Amplifier::new(
        96000.,
        Controls {
            optimum: 1.,
            manual: 1.,
            swell: 1.,
        },
    );
    for (i, amp) in [5e-5, 0.0002, 0.0005, 0.001, 0.002, 0.004, 0.00628, 0.008]
        .into_iter()
        .enumerate()
    {
        let mut w = BufWriter::new(File::create(dir.join(format!("sine-{i}.bin")))?);
        a.reset();
        for j in 0..24000 {
            let x = amp * (std::f64::consts::TAU * j as f64 / 240.).sin();
            let o = a.tick([x, 0.]);
            w.write_all(&o.speaker_v.to_le_bytes())?;
        }
    }
    Ok(())
}
fn bench() {
    for rate in [44100., 48000., 96000.] {
        for count in [1, 8, 32, 88] {
            let mut e = Engine::new(rate);
            let first = LOW + (HIGH - LOW + 1 - count) / 2;
            let mut times = Vec::new();
            let mut peak = 0_f64;
            for i in 0..(rate as usize / 128) {
                let started = std::time::Instant::now();
                if i == 0 {
                    for n in first..first + count {
                        e.note_on(n, 1.);
                    }
                }
                for _ in 0..128 {
                    let a = std::hint::black_box(e.next_outputs());
                    peak = peak.max(a.amplifier.auxiliary_v.abs());
                }
                times.push(started.elapsed().as_secs_f64() * 100. * rate / 128.);
            }
            times.sort_by(f64::total_cmp);
            println!(
                "{rate},{count},{:.2},{:.2},{peak:.5}",
                times[99 * times.len() / 100],
                times[times.len() - 1]
            );
        }
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.as_slice() == ["bench"] {
        bench();
        return Ok(());
    }
    if args.len() != 2 {
        return Err("expected notes|impulses OUTPUT_DIR, or bench".into());
    }
    let d = PathBuf::from(&args[1]);
    std::fs::create_dir_all(&d)?;
    if args[0] == "notes" {
        wave(&d)
    } else if args[0] == "impulses" {
        impulses(&d)
    } else {
        Err("unknown probe".into())
    }
}
