//! Render direct mono T jack and pickup/pressure observations with original note numbers.
use hound::{SampleFormat, WavSpec, WavWriter};
use pianet_n::{Engine, Model};
use std::{error::Error, fs, io::Write, path::Path};
const RATE: u32 = 48000;

fn write_wav(path: &Path, signal: &[f32]) -> Result<(), Box<dyn Error>> {
    let mut w = WavWriter::create(
        path,
        WavSpec {
            channels: 1,
            sample_rate: RATE,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        },
    )?;
    for &x in signal {
        w.write_sample(x)?;
    }
    w.finalize()?;
    Ok(())
}
fn profile() {
    for model in [Model::N, Model::T] {
        for polyphony in [8, 60] {
            let mut e = Engine::new(model, 48_000.);
            if model == Model::N {
                e.set_n_controls(0.8, true);
            }
            for midi in model.key_range().take(polyphony) {
                e.note_on(midi, 0.8);
            }
            let mut block = [0.0; 256];
            e.render(&mut block);
            let mut times = Vec::with_capacity(240);
            for _ in 0..240 {
                let start = std::time::Instant::now();
                e.render(&mut block);
                std::hint::black_box(&block);
                times.push(start.elapsed().as_secs_f64() * 1000.);
            }
            times.sort_by(f64::total_cmp);
            println!("{model:?} {polyphony} keys, 256 frames/48 kHz: median {:.3} ms, p99 {:.3} ms (5.33 ms budget)",
                times[120],times[237]);
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let directory = std::env::args()
        .nth(1)
        .ok_or("usage: pianet_t OUTPUT_DIR")?;
    if directory == "--bench" {
        profile();
        return Ok(());
    }
    let out = Path::new(&directory);
    fs::create_dir_all(out)?;
    let mut audition = Vec::new();
    for (label, velocity) in [("P", 40.0 / 127.0), ("FF", 112.0 / 127.0)] {
        let mut engine = Engine::new(Model::T, f64::from(RATE));
        for note in [
            29, 33, 37, 41, 45, 47, 49, 52, 53, 57, 61, 65, 69, 73, 77, 81, 85, 88,
        ] {
            engine.reset();
            engine.note_on(note, velocity);
            let mut y = vec![0.0; (RATE * 2) as usize];
            engine.render(&mut y[..72000]);
            engine.note_off(note);
            engine.render(&mut y[72000..]);
            write_wav(&out.join(format!("{note}-{label}.wav")), &y)?;
            if [45, 65, 81].contains(&note) {
                audition.extend(&y[..28000]);
                audition.extend([0.0; 9600]);
            }
        }
    }
    write_wav(&out.join("pianet-t-audition.wav"), &audition)?;
    for load in [10e3, 100e3, 1e6] {
        let mut panel = pianet_n::circuit::magnetic_jack(f64::from(RATE), load);
        let impulse: Vec<_> = (0..RATE)
            .map(|i| panel.tick(if i == 0 { 1.0 } else { 0.0 }) as f32)
            .collect();
        write_wav(&out.join(format!("bank-{}k.wav", load / 1000.)), &impulse)?;
    }
    let mut engine = Engine::new(Model::T, f64::from(RATE));
    let mut trace = std::io::BufWriter::new(fs::File::create(out.join("suction-repeats.csv"))?);
    writeln!(
        trace,
        "time_s,midi,pad_m,tip_m,force_N,suction_Pa,seating,bonded,flux_Wbturn"
    )?;
    let mut phrase = Vec::new();
    for frame in 0..RATE {
        for midi in [45, 65, 81] {
            match frame {
                2400 | 11584 | 20160 => engine.note_on(midi, 0.62),
                11440 | 19200 | 32640 => engine.note_off(midi),
                _ => (),
            }
        }
        let mut s = [0.];
        engine.render(&mut s);
        phrase.push(s[0]);
        if frame % 8 == 0 {
            for midi in [45, 65, 81] {
                let o = engine.observation(midi).unwrap();
                writeln!(
                    trace,
                    "{:.8},{},{:.8e},{:.8e},{:.8e},{:.4},{:.6},{},{:.8e}",
                    (frame + 1) as f64 / f64::from(RATE),
                    midi,
                    o.pad_height,
                    o.reed_tip,
                    o.pad_force,
                    o.suction_pa,
                    o.seating,
                    u8::from(o.bonded),
                    o.flux_wb_turns
                )?;
            }
        }
    }
    write_wav(&out.join("suction-repeats.wav"), &phrase)?;
    trace.flush()?;
    Ok(())
}
