//! Offline probe. Channel units, order and event timings are in README.md.
use electra_piano::{
    circuit::{group, Controls, Panel},
    fork::{Fork, KEYS},
};
use std::{
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

const RATE: u32 = 48000;
fn riff(path: &Path, note: u8, speed: f64, sequence: &str) -> Result<(), Box<dyn Error>> {
    let spec = KEYS
        .iter()
        .find(|s| s.note == note)
        .ok_or("expected MIDI 48, 65 or 80")?;
    let mut key = Fork::new(*spec, f64::from(RATE));
    let mut panel = Panel::new(f64::from(RATE), Controls::default());
    let mut wav = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 5,
            sample_rate: RATE,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    key.strike(speed);
    for i in 0..RATE * 2 {
        match (sequence, i) {
            ("release", 24000) => key.release(),
            ("pedals", 17000) => key.sustain(true),
            ("pedals", 24000) => key.release(),
            ("pedals", 30000) => panel.set_controls(Controls {
                soft_pedal: true,
                ..Default::default()
            }),
            ("pedals", 44500) => key.sustain(false),
            ("restrike", 17000) => key.release(),
            ("restrike", 17900) => key.strike(speed),
            _ => {}
        }
        let m = key.tick();
        let mut group_emf = [0.; 7];
        group_emf[group(note).unwrap()] = m.pickup_v;
        let out = panel.tick(group_emf);
        for x in [
            m.sensed_m,
            m.return_m,
            m.pickup_v,
            out.auxiliary_v,
            out.speaker_v,
        ] {
            wav.write_sample(x as f32)?;
        }
    }
    wav.finalize()?;
    Ok(())
}

fn audition(dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut wav = hound::WavWriter::create(
        dir.join("three-key-audition.wav"),
        hound::WavSpec {
            channels: 1,
            sample_rate: RATE,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        },
    )?;
    // One common transfer of 64 digital units per auxiliary volt across keys.
    for spec in KEYS {
        let mut input = hound::WavReader::open(dir.join(format!("{}-0.85-pedals.wav", spec.note)))?;
        for sample in input.samples::<f32>().skip(3).step_by(5) {
            let digital = sample? * 64.;
            assert!(digital.abs() < 1.);
            wav.write_sample((digital * 8_388_607.).round() as i32)?;
        }
    }
    wav.finalize()?;
    Ok(())
}

fn panel_impulses(dir: &Path) -> Result<(), Box<dyn Error>> {
    for volume in [0.25, 0.5, 1.] {
        for pedal in [false, true] {
            for group in [0, 3, 6] {
                let p = Controls {
                    volume,
                    soft_pedal: pedal,
                    ..Default::default()
                };
                let mut panel = Panel::new(96000., p);
                let mut w = BufWriter::new(File::create(
                    dir.join(format!("ir-{volume}-{pedal}-{group}.bin")),
                )?);
                for i in 0..131072 {
                    let mut input = [0.; 7];
                    if i == 0 {
                        input[group] = 1.;
                    }
                    let out = panel.tick(input);
                    w.write_all(&out.auxiliary_v.to_le_bytes())?;
                    w.write_all(&out.speaker_v.to_le_bytes())?;
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: electra-piano notes|impulses OUTPUT_DIRECTORY".into());
    }
    let dir = PathBuf::from(&args[1]);
    fs::create_dir_all(&dir)?;
    if args[0] == "impulses" {
        return panel_impulses(&dir);
    }
    if args[0] != "notes" {
        return Err("unknown probe".into());
    }
    for spec in KEYS {
        for speed in [0.35, 0.85, 1.7] {
            for seq in ["hold", "release", "pedals", "restrike"] {
                riff(
                    &dir.join(format!("{}-{speed}-{seq}.wav", spec.note)),
                    spec.note,
                    speed,
                    seq,
                )?;
            }
        }
    }
    audition(&dir)
}
