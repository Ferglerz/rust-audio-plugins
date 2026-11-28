//! Pass a mono float capture or test excitation through the stereo factory board.
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use rhodes_mk8::{
    fx::Controls as Fx,
    panel::{Controls, Panel},
};
use std::error::Error;
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: mk8_fx_probe SETTING mono-input.wav stereo-output.wav".into());
    }
    let mut input = WavReader::open(&args[2])?;
    let spec = input.spec();
    if spec.channels != 1 || spec.sample_format != SampleFormat::Float {
        return Err("expected mono IEEE float WAV".into());
    }
    let mut out = WavWriter::create(
        &args[3],
        WavSpec {
            channels: 2,
            ..spec
        },
    )?;
    let steps = (176400. / f64::from(spec.sample_rate)).ceil() as usize;
    let mut panel = Panel::new(
        f64::from(spec.sample_rate) * steps as f64,
        Controls::default(),
    );
    let mut fx = Fx {
        fitted: true,
        ..Fx::default()
    };
    match args[1].as_str() {
        "bypass" => {}
        "comp" => {
            fx.compressor = true;
            fx.amount = 0.70;
            fx.ratio = 4.;
        }
        "chorus" | "chorus-bonners" => {
            fx.chorus = true;
            let (rate, center, excursion) = if args[1] == "chorus" {
                (3.253, 5.07, 1.085)
            } else {
                (3.228, 6.40, 0.694)
            };
            fx.chorus_rate_hz = rate;
            fx.chorus_ms = center;
            fx.chorus_depth = excursion / (0.35 * center);
        }
        "phaser-slow" | "phaser-medium" | "phaser-fast" => {
            fx.phaser = true;
            fx.phaser_hz = 420.;
            let (rate, octaves) = match args[1].as_str() {
                "phaser-slow" => (0.659, 1.53),
                "phaser-medium" => (1.801, 0.67),
                _ => (4.68, 1.30),
            };
            fx.phaser_rate_hz = rate;
            fx.phaser_depth = octaves / 2.4;
        }
        "delay-long" | "delay-short" => {
            fx.delay = true;
            fx.delay_ms = if args[1] == "delay-long" {
                393.3
            } else {
                177.6
            };
            fx.spread = 0.33;
            fx.feedback = 0.48;
            fx.mix = 0.5;
        }
        _ => return Err("unknown board setting".into()),
    }
    panel.set_fx_controls(fx);
    let mut prior = 0.;
    for mono in input.samples::<f32>() {
        let x = f64::from(mono?);
        let mut stereo = [0.; 2];
        for sub in 1..=steps {
            stereo = panel.process_return(prior + (x - prior) * sub as f64 / steps as f64);
        }
        for v in stereo {
            out.write_sample(v as f32)?;
        }
        prior = x;
    }
    out.finalize()?;
    Ok(())
}
