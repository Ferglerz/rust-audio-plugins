use hound::{SampleFormat, WavSpec, WavWriter};
use roadmaster::{Probe, PILOT};
use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "target/roadmaster-proof".into()),
    );
    fs::create_dir_all(&out)?;
    let rate = 48_000;
    for (i, note) in PILOT.into_iter().enumerate() {
        for (name, mobile, sustain, gap) in [
            ("stock", true, false, 0.),
            ("pedal", true, true, 0.),
            ("fixed", false, true, 0.),
            ("far", true, true, 1.),
        ] {
            let mut p = Probe::new(rate.into(), 4, mobile);
            p.sustain(sustain);
            p.set_gap_mm(gap);
            let spec = WavSpec {
                channels: 4,
                sample_rate: rate,
                bits_per_sample: 32,
                sample_format: SampleFormat::Float,
            };
            let mut out = WavWriter::create(out.join(format!("{note}-{name}.wav")), spec)?;
            for t in 0..rate {
                if t == rate / 20 {
                    p.strike(i, 0.70);
                }
                if t == rate * 35 / 100 {
                    p.release(i);
                }
                let o = p.sample();
                for x in o.sensor_v.into_iter().chain([o.surface_m_s]) {
                    assert!(x.is_finite());
                    out.write_sample(x as f32)?;
                }
            }
            out.finalize()?;
        }
    }
    // Slide the bass pickup across a fourth-mode node without changing the strike.
    for percent in [18, 20, 22, 24, 25, 26, 28, 32] {
        let mut p = Probe::new(rate.into(), 4, true);
        p.position_sensor(0, f64::from(percent) / 100.);
        p.sustain(true);
        p.strike(0, 0.70);
        let spec = WavSpec {
            channels: 1,
            sample_rate: rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut wav = WavWriter::create(out.join(format!("bass-position-{percent}.wav")), spec)?;
        for _ in 0..rate / 3 {
            wav.write_sample(p.sample().sensor_v[0] as f32)?;
        }
        wav.finalize()?;
    }
    Ok(())
}
