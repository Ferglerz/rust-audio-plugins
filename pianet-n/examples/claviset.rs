use hound::{SampleFormat, WavSpec, WavWriter};
use pianet_n::claviset::{Controls, Engine, Instrument, Modulation, Register};
use std::{error::Error, fs, path::Path};
const SR: u32 = 44100;
fn note(
    engine: &mut Engine,
    root: &Path,
    name: &str,
    fmt: WavSpec,
    n: u8,
    c: Controls,
) -> Result<(), Box<dyn Error>> {
    engine.reset();
    engine.set_controls(c);
    let mut wav = WavWriter::create(root.join(format!("{name}.wav")), fmt)?;
    for i in 0..SR * 3 {
        if i == SR / 20 {
            engine.note_on(n, 80. / 127.);
        }
        if i == 2 * SR {
            engine.note_off(n);
        }
        wav.write_sample(engine.sample())?;
    }
    wav.finalize()?;
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let dir = std::env::args().nth(1).ok_or("expected output directory")?;
    let dir = Path::new(&dir);
    fs::create_dir_all(dir)?;
    let fmt = WavSpec {
        channels: 1,
        sample_rate: SR,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    for (instrument, label) in [
        (Instrument::Pianotron, "selmer"),
        (Instrument::Claviset200, "200"),
    ] {
        let mut e = Engine::new(instrument, SR.into());
        for n in [
            36, 40, 43, 44, 46, 48, 51, 52, 55, 56, 57, 58, 60, 62, 64, 67, 69, 71, 72, 73, 74, 76,
            84, 96,
        ] {
            note(
                &mut e,
                dir,
                &format!("{label}-{n}"),
                fmt,
                n,
                Controls::default(),
            )?;
        }
        if instrument == Instrument::Claviset200 {
            for (register, notes) in [
                (Register::Bass, &[36, 48][..]),
                (Register::Celesta, &[55, 64, 73][..]),
                (Register::MusicBox, &[62, 64, 69, 74][..]),
                (Register::Xylophone, &[62, 74][..]),
            ] {
                for &n in notes {
                    note(
                        &mut e,
                        dir,
                        &format!("200-{register:?}-{n}"),
                        fmt,
                        n,
                        Controls {
                            register,
                            ..Default::default()
                        },
                    )?;
                }
            }
        }
        let mut wav = WavWriter::create(dir.join(format!("{label}-registration.wav")), fmt)?;
        for (i, r) in Register::ALL.into_iter().enumerate() {
            e.reset();
            let c = Controls {
                register: r,
                tone: i as f64 / 8.,
                ..Default::default()
            };
            e.set_controls(c);
            for n in [48, 60, 67] {
                e.note_on(n, 0.7);
            }
            for _ in 0..SR * 3 / 8 {
                wav.write_sample(e.sample())?;
            }
            for n in [48, 60, 67] {
                e.note_off(n);
            }
            for _ in 0..SR / 8 {
                wav.write_sample(e.sample())?;
            }
        }
        wav.finalize()?;
        let mut wav = WavWriter::create(dir.join(format!("{label}-modulation.wav")), fmt)?;
        for (i, m) in [
            Modulation::Off,
            Modulation::Slow,
            Modulation::Medium,
            Modulation::Fast,
        ]
        .into_iter()
        .enumerate()
        {
            e.reset();
            e.set_controls(Controls {
                modulation: m,
                depth: if i == 0 { 0. } else { 1. },
                rate: i as f64 / 3.,
                ..Default::default()
            });
            e.note_on(60, 0.6);
            for _ in 0..SR {
                wav.write_sample(e.sample())?;
            }
        }
        wav.finalize()?;
    }
    Ok(())
}
