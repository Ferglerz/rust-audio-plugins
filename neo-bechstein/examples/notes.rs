//! Write head, loaded piano input and 6-ohm speaker voltage without level matching.
use hound::{SampleFormat, WavSpec, WavWriter};
use neo_bechstein::{pickup::head, Engine};
use std::{env, error::Error, fs, path::Path};
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let folder = args
        .next()
        .ok_or("expected folder [rate] [MIDI notes...]")?;
    let dir = Path::new(&folder);
    fs::create_dir_all(dir)?;
    let rate: u32 = args.next().as_deref().unwrap_or("48000").parse()?;
    let mut groups: Vec<String> = args.collect();
    let mut engine = Engine::new(rate.into());
    engine.keyboard.set_extended(true);
    if groups.is_empty() {
        groups = [0, 21, 44, 45, 60, 87, 88, 100, 101, 104, 105, 108, 127]
            .map(|n| n.to_string())
            .to_vec();
    }
    for name in groups {
        let notes = name
            .split('+')
            .map(str::parse)
            .collect::<Result<Vec<u8>, _>>()?;
        if notes.iter().any(|&n| n > 127) {
            return Err("MIDI note outside 0–127".into());
        }
        engine.reset();
        let mut wav = WavWriter::create(
            dir.join(format!("midi-{name}-v80.wav")),
            WavSpec {
                channels: 3,
                sample_rate: rate,
                bits_per_sample: 32,
                sample_format: SampleFormat::Float,
            },
        )?;
        for i in 0..rate {
            if i == rate / 50 {
                for &n in &notes {
                    engine.keyboard.note_on(n, 80. / 127.);
                }
            }
            if i == rate * 3 / 5 {
                for &n in &notes {
                    engine.keyboard.note_off(n);
                }
            }
            let (taps, speaker) = engine.sample_with_taps();
            for v in [taps.head_emf_v[head(notes[0])], taps.input_v, speaker] {
                assert!(v.is_finite(), "MIDI {name} at {i}");
                wav.write_sample(v as f32)?;
            }
        }
        wav.finalize()?;
    }
    Ok(())
}
