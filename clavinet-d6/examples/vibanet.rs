//! Matched strikes for the factory film's four pickup settings and wah passage.
use clavinet_d6::{
    panel::PICKUP_VOLTS,
    vibanet::{Controls, Engine, Selector},
};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{env, error::Error, fs, path::Path};
const RATE: u32 = 48_000;

fn strike(
    dir: &Path,
    label: &str,
    note: u8,
    gate_ms: u32,
    controls: Controls,
) -> Result<(), Box<dyn Error>> {
    let mut engine = Engine::vibanet(f64::from(RATE), false);
    engine.set_vibanet_controls(controls);
    engine.note_on(note, 0.70);
    let spec = WavSpec {
        channels: 4,
        sample_rate: RATE,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut wav = WavWriter::create(dir.join(format!("{label}-{note}.wav")), spec)?;
    for i in 0..RATE * 3 / 4 {
        if i == RATE * gate_ms / 1000 {
            engine.note_off(note);
        }
        let (raw, jack) = engine.sample_with_pickups();
        for value in [
            raw.centre * PICKUP_VOLTS,
            raw.bridge * PICKUP_VOLTS,
            f64::from(jack),
            engine.wah_frequency_hz() / 10000.,
        ] {
            wav.write_sample(value as f32)?;
        }
    }
    wav.finalize()?;
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or("expected output directory")?;
    let dir = Path::new(&path);
    fs::create_dir_all(dir)?;
    for (name, pickup) in [
        ("lead", Selector::Lead),
        ("rhythm", Selector::Rhythm),
        ("fulltone", Selector::Fulltone),
        ("phase", Selector::Phase),
    ] {
        for note in [34, 38, 40, 41, 42, 43, 45, 46, 64, 78] {
            strike(
                dir,
                name,
                note,
                250,
                Controls {
                    pickup,
                    ..Default::default()
                },
            )?;
        }
    }
    // Estimated gate lengths in the dry solo and paired-pickup passages.
    for (pickup, note, gate_ms) in [
        (Selector::Lead, 41, 134),
        (Selector::Rhythm, 41, 134),
        (Selector::Rhythm, 41, 126),
        (Selector::Fulltone, 41, 122),
        (Selector::Rhythm, 46, 114),
        (Selector::Fulltone, 46, 154),
        (Selector::Phase, 43, 118),
        (Selector::Fulltone, 43, 146),
    ] {
        strike(
            dir,
            &format!("release-{pickup:?}-{gate_ms}"),
            note,
            gate_ms,
            Controls {
                pickup,
                ..Default::default()
            },
        )?;
    }
    for note in [40, 42, 43, 45] {
        strike(
            dir,
            "wah",
            note,
            250,
            Controls {
                wah: true,
                sensitivity: 0.8,
                ..Default::default()
            },
        )?;
    }
    Ok(())
}
