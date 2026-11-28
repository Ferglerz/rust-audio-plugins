//! Render aligned pickup/jack comparisons and time real-time blocks at each host rate.
use clavinet_d6::{
    live::{Engine, HIGH, LOW},
    panel::ALL_TONES,
    Switch, Wiring,
};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{env, error::Error, fs, path::Path, time::Instant};

fn probes(dir: &Path, rate: u32) -> Result<(), Box<dyn Error>> {
    let d = dir.join(format!("{rate}"));
    fs::create_dir_all(&d)?;
    for n in [LOW, 34, 36, 39, 41, 43, 46, 51, 52, 64, 78, HIGH] {
        let mut e = Engine::new(rate as f64);
        if [36, 39, 41, 43].contains(&n) {
            e.set_controls(Switch::CB, Wiring::DaAdds, ALL_TONES, 1.0, 0.0);
        }
        let spec = WavSpec {
            channels: 3,
            sample_rate: rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut w = WavWriter::create(d.join(format!("{n}.wav")), spec)?;
        for i in 0..(rate as f64 * 1.30).round() as usize {
            if i == (rate as f64 * 0.05).round() as usize {
                e.note_on(n, 80. / 127.);
            }
            if i == (rate as f64 * 0.80).round() as usize {
                e.note_off(n);
            }
            let (p, j) = e.sample_with_pickups();
            w.write_sample((0.1 * p.centre) as f32)?;
            w.write_sample((0.1 * p.bridge) as f32)?;
            w.write_sample(j)?;
        }
        w.finalize()?;
    }
    Ok(())
}

fn bench(rate: u32, notes: &[u8]) -> (f64, f64, usize, f64) {
    let mut e = Engine::new(rate as f64);
    for &n in notes {
        e.note_on(n, 0.65);
    }
    let mut block = [0.0_f32; 128];
    for _ in 0..8 {
        e.render(&mut block);
    }
    e.reset();
    for &n in notes {
        e.note_on(n, 0.65);
    }
    let mut times = Vec::new();
    let mut peak = 0.0_f32;
    for _ in 0..rate / 128 {
        let t = Instant::now();
        e.render(&mut block);
        times.push(t.elapsed().as_secs_f64());
        for &x in &block {
            assert!(x.is_finite());
            peak = peak.max(x.abs());
        }
    }
    times.sort_by(f64::total_cmp);
    let deadline = 128. / rate as f64;
    let mean = times.iter().sum::<f64>() / times.len() as f64 / deadline;
    let p99 = times[times.len() * 99 / 100] / deadline;
    let misses = times.iter().filter(|&&t| t > deadline).count();
    (mean, p99, misses, peak.into())
}

fn performance(dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::new(48_000.);
    let spec = WavSpec {
        channels: 1,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut w = WavWriter::create(dir.join("d6-performance.wav"), spec)?;
    let phrase = [
        (46, 0.8),
        (64, 0.67),
        (46, 0.90),
        (78, 0.84),
        (51, 0.72),
        (52, 0.72),
        (52, 0.8),
        (64, 0.85),
    ];
    for (i, &(note, vel)) in phrase.iter().enumerate() {
        let mode = [Switch::CA, Switch::CB, Switch::DA, Switch::DB][i / 2];
        let mute = if i == 6 { 0.75 } else { 0.0 };
        engine.set_controls(mode, Wiring::DaAdds, ALL_TONES, 1.0, mute);
        engine.note_on(note, vel);
        for _ in 0..5200 {
            w.write_sample(engine.sample())?;
        }
        engine.note_off(note);
        for _ in 0..1100 {
            w.write_sample(engine.sample())?;
        }
    }
    for _ in 0..16000 {
        w.write_sample(engine.sample())?;
    }
    w.finalize()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = env::args().nth(1).ok_or("expected output directory")?;
    let dir = Path::new(&root);
    fs::create_dir_all(dir)?;
    let all: Vec<u8> = (LOW..=HIGH).collect();
    for rate in [44_100, 48_000, 96_000] {
        probes(dir, rate)?;
        let mut loud = Engine::new(f64::from(rate));
        for &n in &all {
            loud.note_on(n, 1.0);
        }
        let fortissimo_peak = (0..rate / 4)
            .map(|_| loud.sample().abs())
            .fold(0.0_f32, f32::max);
        println!("{rate} Hz full-keyboard fortissimo first 250 ms: {fortissimo_peak:.4} V peak");
        for notes in [
            &[64][..],
            &[39, 46, 51, 52, 60, 64, 70, 78],
            &all[12..28],
            &all[10..42],
            &all,
        ] {
            let (mean, p99, misses, peak) = bench(rate, notes);
            println!(
                "{rate} Hz {} notes: mean {:.1}%, p99 {:.1}% of a 128-frame deadline, {misses} late blocks, peak {:.5}",
                notes.len(),
                100. * mean,
                100. * p99,
                peak
            );
        }
    }
    performance(dir)
}
