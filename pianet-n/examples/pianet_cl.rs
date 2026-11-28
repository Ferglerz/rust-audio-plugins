//! Raw 1963 Pianet C line and headphones, and E71 Pianet L external and loudspeaker taps.
use hound::{SampleFormat, WavSpec, WavWriter};
use pianet_n::{
    early::{CPanel, LPanel},
    Engine, Model, Reed,
};
use std::{
    error::Error,
    fs,
    io::{BufWriter, Write},
    path::Path,
};
const HOST: u32 = 48000;
const RATE: f64 = 192000.;
fn mono(path: &Path, x: &[f32]) -> Result<(), Box<dyn Error>> {
    let mut w = WavWriter::create(
        path,
        WavSpec {
            channels: 1,
            sample_rate: HOST,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        },
    )?;
    for &sample in x {
        w.write_sample(sample)?;
    }
    w.finalize()?;
    Ok(())
}
fn profile() {
    for (model, monitor) in [
        (Model::C, pianet_n::early::Monitor::Phones),
        (Model::L, pianet_n::early::Monitor::Speakers),
    ] {
        for voices in [8, 61] {
            let mut engine = Engine::new(model, 48000.);
            engine.set_early_monitor(monitor);
            engine.set_n_controls(0.85, true);
            for key in model.key_range().take(voices) {
                engine.note_on(key, 0.75);
            }
            let mut samples = [0.; 256];
            engine.render(&mut samples);
            let mut times = Vec::new();
            for _ in 0..280 {
                let start = std::time::Instant::now();
                engine.render(&mut samples);
                std::hint::black_box(&samples);
                times.push(start.elapsed().as_secs_f64() * 1000.);
            }
            times.sort_by(f64::total_cmp);
            println!(
                "{model:?} {voices} keys, 256 frames at 48 kHz, median {:.3} ms, p99 {:.3} ms",
                times[140], times[277]
            );
        }
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let dir = std::env::args()
        .nth(1)
        .ok_or("usage: pianet_cl OUTPUT_DIR")?;
    if dir == "--bench" {
        profile();
        return Ok(());
    }
    let out = Path::new(&dir);
    fs::create_dir_all(out)?;
    let mut c = CPanel::new(RATE, 100e3, 200.);
    let mut l = LPanel::new(RATE, 100e3);
    let mut w = BufWriter::new(fs::File::create(out.join("boards.csv"))?);
    writeln!(w, "c_line,c_phones,l_line,l_speaker,l_motion")?;
    for i in 0..192000 {
        let stimulus = if i == 0 { 1e-9 } else { 0. };
        let a = c.tick(stimulus);
        let b = l.tick(stimulus);
        writeln!(
            w,
            "{:.10e},{:.10e},{:.10e},{:.10e},{:.10e}",
            a.line, a.headphones, b.line, b.speaker, b.motion
        )?;
    }
    for (model, label, monitor) in [
        (Model::C, "c", pianet_n::early::Monitor::External),
        (Model::L, "l", pianet_n::early::Monitor::Speakers),
    ] {
        let mut engine = Engine::new(model, f64::from(HOST));
        engine.set_early_monitor(monitor);
        let mut audition = Vec::new();
        for note in [45, 52, 55, 57, 60, 62, 64, 67, 69, 72, 81] {
            engine.reset();
            engine.note_on(note, 75. / 127.);
            let mut audio = vec![0.; 72000];
            engine.render(&mut audio[..57600]);
            engine.note_off(note);
            engine.render(&mut audio[57600..]);
            if [45, 64, 81].contains(&note) {
                audition.extend_from_slice(&audio);
            }
            mono(&out.join(format!("{label}-held-{note}.wav")), &audio)?;
        }
        mono(&out.join(format!("pianet-{label}-audition.wav")), &audition)?;
    }
    // Measured C/L pad dimensions and individual reed losses are not yet available.
    // Use the common adhesive reed as the same source into both traced boards.
    for note in [29u8, 45, 64, 81, 89] {
        let mut f = WavWriter::create(
            out.join(format!("key-{note}.wav")),
            WavSpec {
                channels: 5,
                sample_rate: HOST,
                bits_per_sample: 32,
                sample_format: SampleFormat::Float,
            },
        )?;
        let mut reed = Reed::at_rate(Model::N, note, RATE).unwrap();
        let mut old = reed.step().capacitance;
        c.set(1., false);
        c.reset();
        l.set(1.);
        l.reset();
        for i in 0..288000 {
            match i {
                9600 => reed.press(75),
                168000 => reed.release(),
                172224 => reed.press(75),
                211200 => reed.release(),
                _ => (),
            }
            let now = reed.step().capacitance;
            let current = 300. * RATE * (now - old);
            old = now;
            let a = c.tick(current);
            let b = l.tick(current);
            if i % 4 == 0 {
                for x in [a.line, a.headphones, b.line, b.speaker, b.motion] {
                    f.write_sample(x as f32)?;
                }
            }
        }
        f.finalize()?;
    }
    let mut weights = BufWriter::new(fs::File::create(out.join("controls.csv"))?);
    writeln!(
        weights,
        "knee,vibrato,frame,c_line,c_headphones,l_line,l_speaker"
    )?;
    for &(s, on) in &[(0., false), (0.5, false), (1., false), (1., true)] {
        let sr = 48000.;
        let mut c = CPanel::new(sr, 100e3, 200.);
        c.set(s, on);
        c.reset();
        let mut l = LPanel::new(sr, 100e3);
        l.set(s);
        l.reset();
        for i in 0..(sr as usize * 2) {
            let input = 1e-8 * (2. * std::f64::consts::PI * 440. * i as f64 / sr).sin();
            let a = c.tick(input);
            let b = l.tick(input);
            if i >= sr as usize {
                writeln!(
                    weights,
                    "{s},{},{i},{:.8e},{:.8e},{:.8e},{:.8e}",
                    u8::from(on),
                    a.line,
                    a.headphones,
                    b.line,
                    b.speaker
                )?;
            }
        }
    }
    Ok(())
}
