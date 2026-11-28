//! Isolate extended notes and pickup-gap changes with the native solver running beside them.
use clavinet_d6::live::Engine;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{env, error::Error, fs, path::Path};

fn capture(dir: &Path, rate: u32, note: u8, gap: f64) -> Result<(), Box<dyn Error>> {
    let mut e = Engine::extended(rate.into());
    e.set_pickup_distance_mm(gap, 0.);
    e.note_on(note, 0.75);
    let path = dir.join(format!("{rate}-{note}-{gap}.wav"));
    let mut w = WavWriter::create(
        path,
        WavSpec {
            channels: 3,
            sample_rate: rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        },
    )?;
    for _ in 0..rate * 2 {
        let (p, j) = e.sample_with_pickups();
        for value in [p.centre as f32, p.bridge as f32, j] {
            assert!(value.is_finite(), "{note} {gap} mm");
            w.write_sample(value)?;
        }
    }
    w.finalize()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let folder = env::args().nth(1).ok_or("expected output directory")?;
    let dir = Path::new(&folder);
    fs::create_dir_all(dir)?;
    for rate in [44100, 48000, 96000] {
        for n in [0, 12, 28, 29, 64, 88, 89, 100, 115, 126, 127] {
            capture(dir, rate, n, 0.)?;
        }
    }
    for gap in [-1., 1.] {
        capture(dir, 48000, 64, gap)?;
    }
    Ok(())
}
