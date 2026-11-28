//! Synchronous native-sensor note releases. Each file has 0/50/100% return sound.
use hound::{SampleFormat, WavSpec, WavWriter};
use pianet_n::{claviset, Engine, Model, Reed};
use std::{error::Error, fs, path::Path};
fn main() -> Result<(), Box<dyn Error>> {
    const RATE: u32 = 96000;
    let dir = std::env::args().nth(1).ok_or("expected output directory")?;
    let dir = Path::new(&dir);
    fs::create_dir_all(dir)?;
    for model in [Model::N, Model::T] {
        for n in [45, 64, 81] {
            let mut reeds = std::array::from_fn::<_, 3, _>(|i| {
                let mut r = Reed::at_rate(model, n, RATE.into()).expect("native reed");
                r.set_pad_release(i as f32 * 0.5);
                r
            });
            let fmt = WavSpec {
                channels: 3,
                sample_rate: RATE,
                bits_per_sample: 32,
                sample_format: SampleFormat::Float,
            };
            let mut w = WavWriter::create(dir.join(format!("return-{model:?}-{n}.wav")), fmt)?;
            let mut prev = reeds.each_mut().map(|r| {
                if model == Model::T {
                    r.current_flux()
                } else {
                    r.current_capacitance()
                }
            });
            let mut seated = false;
            for i in 0..RATE * 3 / 4 {
                for (j, r) in reeds.iter_mut().enumerate() {
                    if i == RATE / 20 {
                        r.press(80);
                    }
                    if i == RATE / 2 {
                        r.release();
                    }
                    let o = r.step();
                    if j == 0 && !seated && i > RATE / 2 && o.bonded {
                        println!(
                            "{model:?} MIDI {n} pad seats {:.3} ms after key-up",
                            1000. * f64::from(i - RATE / 2) / f64::from(RATE)
                        );
                        seated = true;
                    }
                    let y = if model == Model::T {
                        o.flux_wb_turns
                    } else {
                        o.capacitance
                    };
                    let signal = (y - prev[j]) * f64::from(RATE);
                    prev[j] = y;
                    // Raw voltage or field charge derivative. A common gain for
                    // all three channels keeps their original relative scale.
                    w.write_sample((signal * if model == Model::T { 1. } else { 3e9 }) as f32)?;
                }
            }
            w.finalize()?;
        }
    }
    jack_audition(dir)
}

const HOST_RATE: usize = 48000;
const KEY_UP: usize = HOST_RATE * 45 / 100;

#[derive(Clone, Copy)]
enum Keyboard {
    Hohner(Model),
    Cam(claviset::Instrument),
}

impl Keyboard {
    fn record(self, settings: Option<(f32, f32)>) -> Vec<f32> {
        let mut frames = vec![0.; KEY_UP + HOST_RATE / 5];
        let (held, released) = frames.split_at_mut(KEY_UP);
        match self {
            Self::Hohner(model) => {
                let mut engine = Engine::new(model, HOST_RATE as f64);
                if let Some((gap, pad)) = settings {
                    engine.set_extended(true);
                    engine.set_pickup_distance_mm(gap);
                    engine.set_pad_release(pad);
                }
                engine.note_on(64, 80. / 127.);
                engine.render(held);
                engine.note_off(64);
                engine.render(released);
            }
            Self::Cam(model) => {
                let mut engine = claviset::Engine::new(model, HOST_RATE as f64);
                if let Some((gap, _)) = settings {
                    engine.set_extended(true);
                    engine.set_pickup_distance_mm(gap);
                }
                engine.note_on(64, 80. / 127.);
                engine.render(held);
                engine.note_off(64);
                engine.render(released);
            }
        }
        frames
    }
}

fn rms(input: &[f32]) -> f64 {
    (input.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>() / input.len() as f64).sqrt()
}

fn peak(input: &[f32]) -> f32 {
    input.iter().fold(0f32, |p, x| p.max(x.abs()))
}

fn append_group(dst: &mut Vec<f32>, takes: &[Vec<f32>], start: usize) {
    // One gain across this instrument's settings. Fade only the edit boundaries.
    let gain = 0.85 / takes.iter().map(|t| peak(&t[start..])).fold(0f32, f32::max);
    for take in takes {
        let len = take.len() - start;
        for (i, x) in take[start..].iter().enumerate() {
            let fade = if start == 0 {
                len - i
            } else {
                (len - i).min(i)
            };
            let fade = (fade as f32 / (HOST_RATE as f32 * 0.003)).min(1.);
            dst.push(x * gain * fade);
        }
        dst.resize(dst.len() + HOST_RATE / 7, 0.);
    }
    dst.resize(dst.len() + HOST_RATE / 3, 0.);
}

fn partial(input: &[f32], hz: f64) -> f64 {
    let mut a = 0.;
    let mut b = 0.;
    for (i, &x) in input.iter().enumerate() {
        let phase = std::f64::consts::TAU * i as f64;
        let weight = (0.5 - 0.5 * (phase / (input.len() - 1) as f64).cos()) * f64::from(x);
        let (sn, cs) = (phase * hz / HOST_RATE as f64).sin_cos();
        a += weight * sn;
        b += weight * cs;
    }
    a.hypot(b)
}

fn jack_audition(dir: &Path) -> Result<(), Box<dyn Error>> {
    use std::io::Write;
    use Keyboard::{Cam, Hohner};
    let mut csv = fs::File::create(dir.join("pickup-distance.csv"))?;
    writeln!(
        csv,
        "instrument,near_dB,stock_rms_V,far_dB,stock_peak_V,stock_max_difference_V"
    )?;
    let mut spectra = fs::File::create(dir.join("variant-terminals.csv"))?;
    writeln!(spectra, "instrument,H2_dBc,H3_dBc,post_keyup_dB,held_rms_V")?;
    let mut native = Vec::new();
    let mut pickup = Vec::new();
    let mut release = Vec::new();
    for (label, instrument) in [
        ("Pianet N", Hohner(Model::N)),
        ("Pianet C", Hohner(Model::C)),
        ("Pianet L", Hohner(Model::L)),
        ("Pianet T", Hohner(Model::T)),
        ("Cembalet", Hohner(Model::Cembalet)),
        ("Cembalet N", Hohner(Model::CembaletN)),
        ("Claviset 200", Cam(claviset::Instrument::Claviset200)),
        ("Pianotron 8701", Cam(claviset::Instrument::Pianotron)),
    ] {
        let reference = instrument.record(None);
        let distances = [-0.15, 0., 1.].map(|gap| instrument.record(Some((gap, 1.))));
        let levels = distances
            .each_ref()
            .map(|t| rms(&t[HOST_RATE / 20..HOST_RATE / 3]));
        let ratio_db = |i: usize| 20. * (levels[i] / levels[1]).log10();
        let difference = reference
            .iter()
            .zip(&distances[1])
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        assert_eq!(difference, 0., "{label} stock waveform");
        writeln!(
            csv,
            "{label},{:.3},{:.9},{:.3},{:.9},{:.9}",
            ratio_db(0),
            levels[1],
            ratio_db(2),
            peak(&reference),
            difference
        )?;
        append_group(&mut pickup, &distances, 0);
        let window = &reference[HOST_RATE / 10..HOST_RATE / 4];
        let h1 = partial(window, 329.627557);
        let h = |order: f64| 20. * (partial(window, order * 329.627557) / h1).log10();
        let tail = rms(&reference[KEY_UP + HOST_RATE / 20..KEY_UP + HOST_RATE / 10]);
        let held = rms(&reference[KEY_UP - HOST_RATE / 20..KEY_UP]);
        writeln!(
            spectra,
            "{label},{:.2},{:.2},{:.2},{:.9}",
            h(2.),
            h(3.),
            20. * (tail / held).log10(),
            held
        )?;
        native.push((label, reference.clone()));
        if matches!(
            instrument,
            Hohner(Model::N | Model::C | Model::L | Model::T)
        ) {
            let amounts = [0., 0.5, 1.].map(|amount| instrument.record(Some((0., amount))));
            assert_eq!(
                amounts[0][..KEY_UP],
                reference[..KEY_UP],
                "{label} held waveform"
            );
            append_group(&mut release, &amounts, KEY_UP - HOST_RATE * 7 / 100);
        }
    }
    let mut pairs = fs::File::create(dir.join("variant-pairs.csv"))?;
    writeln!(pairs, "reference,comparison,aligned_residual_dB")?;
    for (i, (label, wave)) in native.iter().enumerate() {
        for (other, compare) in &native[i + 1..] {
            let a = &wave[..KEY_UP];
            let b = &compare[..KEY_UP];
            let dot = |x: &[f32], y: &[f32]| {
                x.iter()
                    .zip(y)
                    .map(|(&a, &b)| f64::from(a) * f64::from(b))
                    .sum::<f64>()
            };
            let normalized = dot(a, b) / (dot(a, a) * dot(b, b)).sqrt();
            let remaining = (1. - normalized.powi(2)).max(1e-24);
            writeln!(pairs, "{label},{other},{:.2}", 10. * remaining.log10())?;
        }
    }
    let spec = WavSpec {
        channels: 1,
        sample_rate: HOST_RATE as u32,
        bits_per_sample: 24,
        sample_format: SampleFormat::Int,
    };
    for (name, samples) in [
        ("pad-release-0-50-100.wav", release),
        ("pickup-distance-near-stock-far.wav", pickup),
    ] {
        let mut wav = WavWriter::create(dir.join(name), spec)?;
        for x in samples {
            wav.write_sample((x * 8_388_607.).round() as i32)?;
        }
        wav.finalize()?;
    }
    Ok(())
}
