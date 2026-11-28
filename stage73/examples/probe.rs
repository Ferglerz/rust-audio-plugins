//! Offline instrument renders and synchronous sine measurements at the panel.
use stage73::{
    circuit::Panel,
    engine::{Controls, Stage73Engine, HIGH, LOW},
    era::Era,
    period_panel::{EarlyPanel, Jordan},
};
use std::{env, error::Error, f64::consts::TAU, path::Path, time::Instant};

fn wav(path: &Path, rate: u32, audio: &[f32]) -> Result<(), Box<dyn Error>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &s in audio {
        writer.write_sample(s)?;
    }
    writer.finalize()?;
    Ok(())
}

fn sweep(rate: f64) {
    println!("volume,bass,hz,db");
    for volume in [0.1, 0.5, 1.0] {
        for bass in [0.0, 0.5, 1.0] {
            for i in 0..=56 {
                let hz = (4.0 * 250.0f64.powf(i as f64 / 56.0)).round() * 10.0;
                let mut panel = Panel::new(rate * 4.0);
                panel.set(volume, bass);
                // Integrate an integer number of sine cycles after the settling interval.
                let samples = (rate * 0.4) as usize;
                let mut re = 0.0;
                let mut im = 0.0;
                for j in 0..2 * samples {
                    let (sin, cos) = (TAU * hz * j as f64 / (rate * 4.0)).sin_cos();
                    let y = panel.tick(sin);
                    if j >= samples {
                        re += y * sin;
                        im += y * cos;
                    }
                }
                let db = 20.0 * (2.0 * re.hypot(im) / samples as f64).log10();
                println!("{volume},{bass},{hz},{db:.8}");
            }
        }
    }
}

fn demo() -> Vec<f32> {
    let mut engine = Stage73Engine::new(48000.0);
    let mut song = Vec::new();
    // Two bars with per-note velocities, overlapping pedal and a dry final chord.
    for (bass, notes) in [
        (40, [55, 59, 62, 66]),
        (45, [55, 59, 61, 64]),
        (38, [54, 57, 60, 64]),
        (43, [53, 57, 59, 64]),
    ] {
        engine.sustain(true);
        engine.note_on(bass, 0.82);
        for (i, n) in notes.into_iter().enumerate() {
            engine.note_on(n, 0.48 + i as f32 * 0.12);
            let start = song.len();
            song.resize(start + 4000, 0.0);
            engine.render(&mut song[start..]);
            engine.note_off(n);
        }
        let start = song.len();
        song.resize(start + 24000, 0.0);
        engine.render(&mut song[start..]);
        engine.note_off(bass);
        engine.sustain(false);
        let start = song.len();
        song.resize(start + 8000, 0.0);
        engine.render(&mut song[start..]);
    }
    song
}

fn polyphony() {
    for rate in [44100, 48000, 96000] {
        let mut engine = Stage73Engine::new(rate as f64);
        for note in LOW..=HIGH {
            engine.note_on(note, 0.8);
        }
        let mut block = [0.0; 128];
        let start = Instant::now();
        let mut peak = 0.0f32;
        for _ in 0..rate / 128 {
            engine.render(&mut block);
            for sample in block {
                peak = peak.max(sample.abs());
            }
        }
        println!(
            "{rate} Hz: {:.3} s per audio second, peak {peak:.3}",
            start.elapsed().as_secs_f64()
        );
    }
}

fn era(name: &str) -> Result<Era, Box<dyn Error>> {
    Era::ALL
        .into_iter()
        .find(|e| e.slug() == name)
        .ok_or_else(|| format!("Unknown era {name}").into())
}

// An isolated pickup with 1 V induced EMF. The deck independently builds
// every pickup and drives the matching branch. Record before decimation.
fn panel_impulse(args: &[String]) -> Result<(), Box<dyn Error>> {
    let era = era(&args[2])?;
    let fs = args[3].parse::<u32>()? * 4;
    let volume = args[4].parse::<f64>()?;
    let tone = args[5].parse::<f64>()?;
    let treble = args[6].parse::<f64>()?;
    let key = args[7].parse::<usize>()?;
    let cable_pf = args
        .get(8)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(stage73::circuit::DEFAULT_CABLE_PF);
    let mut stage = Panel::new(fs as f64);
    stage.set_loaded(volume, tone, cable_pf);
    let mut early = EarlyPanel::new(fs as f64);
    early.set_loaded(volume, tone, cable_pf);
    let mut jordan = Jordan::new(fs as f64);
    jordan.set(volume, tone, treble);
    let group = if era == Era::Sparkletop {
        if key < 7 {
            7.
        } else {
            6.
        }
    } else if key < 4 {
        4.
    } else {
        3.
    };
    let mut output = vec![0.; fs as usize / 2];
    for (i, y) in output.iter_mut().enumerate() {
        let x = if i == 0 { 1. / group } else { 0. };
        *y = match era {
            Era::Sparkletop => jordan.tick(x),
            Era::EarlyI => early.tick(x),
            _ => stage.tick(x),
        } as f32;
    }
    wav(Path::new(&args[1]), fs, &output)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("geometry") if args.len() == 3 => {
            let (height, gap) = era(&args[1])?
                .pickup_geometry(args[2].parse()?)
                .ok_or("MIDI note outside E1–E7")?;
            println!("{height} {gap}");
        }
        Some("tremolo") if args.len() == 5 => {
            let rate: u32 = args[2].parse()?;
            let mut jordan = Jordan::new(rate as f64);
            jordan.tremolo(args[3].parse()?, args[4].parse()?);
            let samples = (0..rate * 2)
                .map(|i| jordan.tick(0.01 * (TAU * 1000. * i as f64 / rate as f64).sin()) as f32)
                .collect::<Vec<_>>();
            wav(Path::new(&args[1]), rate, &samples)?;
        }
        Some("panel") if (8..=9).contains(&args.len()) => panel_impulse(&args)?,
        Some("sweep") if args.len() == 2 => sweep(args[1].parse()?),
        Some("demo") if args.len() == 2 => wav(Path::new(&args[1]), 48000, &demo())?,
        Some("polyphony") if args.len() == 1 => polyphony(),
        Some("note") if (6..=10).contains(&args.len()) => {
            let rate: u32 = args[2].parse()?;
            let mut engine = Stage73Engine::new(rate as f64);
            let note = args[3].parse()?;
            let velocity = args[4].parse()?;
            let held: f64 = args[5].parse()?;
            let mut controls = Controls::default();
            if let Some(h) = args.get(6) {
                controls.height_mm = h.parse()?;
            }
            if let Some(g) = args.get(7) {
                controls.gap_mm = g.parse()?;
            }
            if let Some(name) = args.get(8) {
                controls.era = era(name)?;
            }
            if let Some(tap) = args.get(9) {
                if tap != "harp" {
                    return Err("Expected harp tap".into());
                }
                engine.set_harp_tap(true);
            }
            engine.set_controls(controls);
            engine.reset();
            engine.note_on(note, velocity);
            let frames = (rate as f64 * held).round() as usize;
            let mut audio = vec![0.0; frames + rate as usize / 2];
            engine.render(&mut audio[..frames]);
            engine.note_off(note);
            engine.render(&mut audio[frames..]);
            wav(Path::new(&args[1]), rate, &audio)?;
        }
        _ => {
            return Err(concat!(
                "usage: probe geometry ERA MIDI | sweep RATE | polyphony | demo FILE | ",
                "note FILE RATE MIDI VELOCITY HELD_SECONDS [HEIGHT_MM GAP_MM ERA [harp]] | panel FILE ERA RATE VOL TONE TREBLE KEY_INDEX [CABLE_PF] | tremolo FILE RATE HZ DEPTH"
            )
            .into());
        }
    }
    Ok(())
}
