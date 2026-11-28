//! Three stereo performance and fixed-input panning checks for the serviced Rev 4.1.
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{env, error::Error, fs, path::Path};
use vintage_vibe_64::{
    live::StereoEngine,
    stereo::{Controls, Input, Panel},
};
fn main() -> Result<(), Box<dyn Error>> {
    let arg = env::args().nth(1).ok_or("expected output directory")?;
    let root = Path::new(&arg);
    fs::create_dir_all(root)?;
    let sample_rate = 48_000;
    let spec = WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut sweep = String::from("input,frequency_hz,harp_send_db,dry_jack_db\n");
    for mode in [Input::Classic, Input::Fet, Input::Parallel] {
        let c = Controls {
            input: mode,
            ..Controls::default()
        };
        let mut p = Panel::new(sample_rate.into(), c);
        for hz in [40., 140., 440., 1800., 5000., 10000.] {
            p.reset();
            let mut powers = [0.; 3];
            for frame in 0..sample_rate {
                let x = 0.001
                    * (std::f64::consts::TAU * hz * f64::from(frame) / f64::from(sample_rate))
                        .sin();
                let y = p.tick(x);
                if frame >= sample_rate / 2 {
                    for (acc, v) in powers.iter_mut().zip([x, y.send_v, y.left_v]) {
                        *acc += v * v;
                    }
                }
            }
            use std::fmt::Write;
            writeln!(
                sweep,
                "{mode:?},{hz},{:.7},{:.7}",
                10. * (powers[1] / powers[0]).log10(),
                10. * (powers[2] / powers[0]).log10()
            )?;
        }
    }
    fs::write(root.join("input-sweep.csv"), sweep)?;
    // The first setting is chosen from the maker's Ginty console performance.
    // The other settings cover an independently recorded Classic and the maker's
    // older StereoVibe retrofit. Set depth from the observed channel contrast.
    for (name, hz, depth, half) in [
        ("ginty", 1.835, 1., false),
        ("intro-fast", 5.517, 0.373, false),
        ("intro-slow", 1.065, 0.476, true),
        ("sigma-fast", 6.426, 0.53, false),
        ("sigma-slow", 1.563, 0.848, false),
        ("sigma-half", 0.911, 0.872, true),
    ] {
        let full = if half { hz * 2. } else { hz };
        let c = Controls {
            rate: 1. - (9. / full - 1.) / 5.,
            depth,
            tremolo: true,
            half_speed: half,
            ..Controls::default()
        };
        let mut engine = StereoEngine::active(sample_rate.into());
        engine.set_controls(c);
        engine.reset();
        let mut panel = Panel::new(sample_rate.into(), c);
        let mut strike = WavWriter::create(root.join(format!("{name}-chord.wav")), spec)?;
        let mut sine = WavWriter::create(root.join(format!("{name}-pilot.wav")), spec)?;
        for frame in 0..sample_rate * 10 {
            if frame % (2 * sample_rate) == 0 {
                for n in [50, 57, 64, 69] {
                    engine.note_on(n, 0.7);
                }
            }
            if frame % (2 * sample_rate) == 86_400 {
                for n in [50, 57, 64, 69] {
                    engine.note_off(n);
                }
            }
            let y = engine.next_outputs();
            let z = panel.tick(
                0.05 * (f64::from(frame) * std::f64::consts::TAU * 440. / f64::from(sample_rate))
                    .sin(),
            );
            for v in [y.left_v, y.right_v] {
                strike.write_sample(v as f32)?;
            }
            for v in [z.left_v, z.right_v] {
                sine.write_sample(v as f32)?;
            }
        }
        sine.finalize()?;
        strike.finalize()?;
    }
    Ok(())
}
