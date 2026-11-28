//! Source isolation for voicing: unloaded hammer/string bearing and rail impulse at the same key.
use baldwin_electropiano::{
    action::Action,
    rail::{BearingForces, Rail},
};
use hound::{SampleFormat, WavSpec, WavWriter};
use pleasant_dsp::struck_keys::StruckAction;
use std::{env, error::Error, fs, path::Path};
fn main() -> Result<(), Box<dyn Error>> {
    let root = env::args().nth(1).ok_or("expected output directory")?;
    let dir = Path::new(&root);
    fs::create_dir_all(dir)?;
    let rate = 48000;
    let spec = WavSpec {
        channels: 3,
        sample_rate: rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    for n in [38, 43, 53, 54, 64, 67, 69, 74, 76, 126, 127] {
        let mut source = Action::new(n, rate.into());
        let mut rail = Rail::new(rate.into());
        let mut w = WavWriter::create(dir.join(format!("{n}.wav")), spec)?;
        for i in 0..rate {
            if i == 2400 {
                source.strike_key(0.7);
            }
            let mut inputs = BearingForces::default();
            if i == 0 {
                inputs.add(n, 1.);
            }
            let [bass, treble] = rail.sample(inputs, 1u128 << n, false);
            for x in [source.step_output(0.), bass, treble] {
                w.write_sample(x as f32)?;
            }
        }
        w.finalize()?;
    }
    Ok(())
}
