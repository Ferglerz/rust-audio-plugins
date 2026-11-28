//! Isolated loaded register leads and the electrical output for recording comparisons.
use baldwin_electropiano::{
    action::{HIGH, LOW},
    Engine,
};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{env, error::Error, fs, path::Path, time::Instant};

fn main() -> Result<(), Box<dyn Error>> {
    let target = env::args().nth(1).ok_or("expected output directory")?;
    let root = Path::new(&target);
    fs::create_dir_all(root)?;
    let spec = WavSpec {
        channels: 3,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut engine = match env::args().nth(2).as_deref() {
        None => Engine::new(48000.),
        Some("--unvoiced") => Engine::with_regulation(48000., 1., 1., 1., 3.2e6, 10.),
        Some(_) => return Err("expected --unvoiced or no option".into()),
    };
    engine.set_extended(true);
    for n in [0, 1, 126, 127].into_iter().chain(LOW..=HIGH) {
        engine.reset();
        let mut wav = WavWriter::create(root.join(format!("{n}.wav")), spec)?;
        for i in 0..52800 {
            if i == 2400 {
                engine.note_on(n, 0.7);
            }
            if i == 38400 {
                engine.note_off(n);
            }
            let o = engine.sample_with_taps();
            for v in [o.bass_v, o.treble_v, o.jack_v] {
                assert!(v.is_finite(), "nonfinite MIDI {n}, frame {i}");
                wav.write_sample(v as f32)?;
            }
        }
        wav.finalize()?;
    }
    for notes in [&[54][..], &[36, 43, 50, 54, 57, 62, 69, 74]] {
        engine.reset();
        for &n in notes {
            engine.note_on(n, 0.75);
        }
        let start = Instant::now();
        let mut peak = 0.0_f64;
        for _ in 0..48000 {
            peak = peak.max(engine.sample_with_taps().jack_v.abs());
        }
        println!(
            "{} keys at 48 kHz: {:.1}% of one realtime core; peak {peak:.4} V",
            notes.len(),
            100. * start.elapsed().as_secs_f64()
        );
    }
    Ok(())
}
