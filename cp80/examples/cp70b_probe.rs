//! Render the CP-70B construction breaks, pedal catch and pickup branch responses.
use cp80::{
    engine::Engine,
    panel::{Controls, Panel},
    seventy::{course, Revision, Strings, HIGH, LOW},
    seventy_pickup::{OriginalPickup, PickupBank},
};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{error::Error, fs, io::Write, path::Path};

const RATE: u32 = 48_000;

fn mono(rate: u32) -> WavSpec {
    WavSpec {
        channels: 1,
        sample_rate: rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let arg = args
        .next()
        .ok_or("usage: cp70b_probe OUTPUT_DIR [original]")?;
    let original = match args.next().as_deref() {
        None => false,
        Some(s) if s == "original" => true,
        _ => return Err("expected original or no revision argument".into()),
    };
    let prefix = if original { "cp70" } else { "cp70b" };
    let dir = Path::new(&arg);
    fs::create_dir_all(dir)?;
    let mut csv = fs::File::create(dir.join("scale.csv"))?;
    writeln!(csv, "revision,physical,service,midi,cents,hz,string_1,string_2,wire_gauge,hammer,damper,stand,bus")?;
    for (name, rev) in [("CP-70", Revision::Original), ("CP-70B", Revision::B)] {
        for midi in LOW..=HIGH {
            let c = course(rev, midi).unwrap();
            let (left, right, gauge) = match c.strings {
                Strings::Wound { left, right } => (
                    format!("NB{left:06}"),
                    right.map(|id| format!("NB{id:06}")).unwrap_or_default(),
                    String::new(),
                ),
                Strings::Plain {
                    wire_part, gauge, ..
                } => {
                    let part = format!("MV{wire_part:06}");
                    (part.clone(), part, gauge.to_string())
                }
            };
            let damper = c
                .damper_part
                .map(|id| format!("NB{id:06}"))
                .unwrap_or_default();
            let (stand, bus) = c
                .b_wiring
                .map(|(s, b)| (s.to_string(), (b + 1).to_string()))
                .unwrap_or_default();
            writeln!(
                csv,
                "{name},{},{},{},{},{:.4},{left},{right},{gauge},{},{damper},{},{}",
                c.physical,
                c.service,
                midi,
                c.cents,
                c.hz(),
                c.hammer_part,
                stand,
                bus
            )?;
        }
    }
    let mut e = if original {
        Engine::cp70(f64::from(RATE), false)
    } else {
        Engine::cp70b(f64::from(RATE), false)
    };
    let mut wav = WavWriter::create(dir.join(format!("{prefix}-boundaries.wav")), mono(RATE))?;
    // String count, winding, middle-to-high pickup branch and last damper.
    for note in [42, 43, 64, 65, 92, 93, 93, 94] {
        e.reset();
        e.note_on(note, 0.68);
        for i in 0..RATE {
            if i == RATE / 2 {
                e.note_off(note);
            }
            wav.write_sample(e.sample()[0])?;
        }
    }
    wav.finalize()?;
    let mut wav = WavWriter::create(
        dir.join(format!("{prefix}-pedal.wav")),
        WavSpec {
            channels: 2,
            ..mono(RATE)
        },
    )?;
    e.reset();
    for i in 0..2 * RATE {
        match i {
            0 => e.note_on(52, 0.55),
            4200 => {
                e.sustain(true);
                e.note_off(52);
                e.note_on(59, 0.6);
            }
            8400 => {
                e.note_off(59);
                e.note_on(64, 0.65);
            }
            12600 => {
                e.note_off(64);
                e.note_on(67, 0.64);
            }
            16800 => {
                e.note_off(67);
                e.note_on(71, 0.68);
            }
            21000 => e.note_off(71),
            30000 => e.note_on(64, 0.78),
            35000 => e.note_off(64),
            48000 => e.sustain(false),
            _ => {}
        }
        for sample in e.sample() {
            wav.write_sample(sample)?;
        }
    }
    wav.finalize()?;
    let rate = 4 * RATE;
    if original {
        let mut bank = OriginalPickup::new(f64::from(rate));
        let mut wav = WavWriter::create(dir.join("wired-pickup.wav"), mono(rate))?;
        for i in 0..rate {
            wav.write_sample(bank.tick(if i == 0 { 1. } else { 0. }) as f32)?;
        }
        wav.finalize()?;
        return Ok(());
    }
    for bus in 0..3 {
        let mut panel = Panel::cp70b(f64::from(rate));
        panel.set(Controls {
            bass: 1.,
            middle: 1.,
            treble: 1.,
            ..Default::default()
        });
        let mut bank = PickupBank::new(f64::from(rate));
        let mut wav = WavWriter::create(dir.join(format!("bus-{}.wav", bus + 1)), mono(rate))?;
        for i in 0..rate {
            let mut force = [0.; 3];
            if i == 0 {
                force[bus] = 1.;
            }
            wav.write_sample(panel.tick(bank.tick(force), 0.) as f32)?;
        }
        wav.finalize()?;
    }
    Ok(())
}
