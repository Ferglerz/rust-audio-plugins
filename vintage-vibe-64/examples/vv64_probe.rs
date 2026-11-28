//! Write 48 kHz, 32-bit float WAVs in volts, time the live path, or sweep the assumed passive panel.
use hound::{SampleFormat, WavSpec, WavWriter};
use std::{error::Error, path::Path, time::Instant};
use vintage_vibe_64::{
    action::{Action, HIGH, LOW},
    live::{hammer_speed, Engine},
    panel::{Controls, Panel},
};

fn captures(path: &Path, references: bool) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(path)?;
    let spec = WavSpec {
        channels: 1,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut engine = Engine::new(48000.);
    let notes: Vec<u8> = if references {
        vec![50, 53, 59, 62, 69, 71, 74, 76, 81]
    } else {
        (LOW..=HIGH).collect()
    };
    let velocities: &[u8] = if references {
        &[
            12, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120, 127,
        ]
    } else {
        &[48, 80, 110]
    };
    let hold = if references { 67200 } else { 48000 };
    for note in notes {
        for &velocity in velocities {
            engine.reset();
            engine.note_on(note, velocity as f32 / 127.);
            let mut file = WavWriter::create(path.join(format!("n{note}-v{velocity}.wav")), spec)?;
            let mut power = 0.;
            let mut peak = 0_f64;
            for i in 0..hold + 9600 {
                if i == hold {
                    engine.note_off(note);
                }
                let y = engine.next_outputs().jack_v;
                if i < 4800 {
                    power += y * y;
                }
                peak = peak.max(y.abs());
                file.write_sample(y as f32)?;
            }
            file.finalize()?;
            if [33, 60, 96].contains(&note) {
                println!(
                    "note,{note},{velocity},rms100ms,{:.7},peak,{peak:.7}",
                    (power / 4800_f64).sqrt()
                );
            }
        }
    }
    for n in [33, 60, 96] {
        let mut a = Action::new(n, 192000., 48000.);
        a.strike(hammer_speed(80. / 127.));
        let mut force = 0_f64;
        let mut travel = 0_f64;
        let mut end = 0;
        for i in 0..10000 {
            let x = a.step(0.);
            if x.contact_n > 0. {
                end = i;
            }
            force = force.max(x.contact_n);
            travel = travel.max(x.tine_m.abs());
        }
        println!(
            "mechanism,{n},contact_ms,{:.3},peak_N,{force:.3},tip_mm,{:.3}",
            end as f64 / 192.,
            travel * 1000.
        );
    }
    Ok(())
}
fn comparison(path: &Path) -> Result<(), Box<dyn Error>> {
    use pleasant_dsp::instrument::Decimator;
    use vintage_vibe_64::{action::length_mm, live::LATENCY};
    std::fs::create_dir_all(path)?;
    let spec = WavSpec {
        channels: 4,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    for note in [40, 60, 84] {
        for velocity in [48, 80, 110] {
            let reg = f64::from(note - LOW) / f64::from(HIGH - LOW);
            let smooth = length_mm(LOW) * (length_mm(HIGH) / length_mm(LOW)).powf(reg);
            let mut alternate = Action::with_tine_length_mm(note, 192000., 48000., smooth);
            let mut decimator = Decimator::<1>::new(4, 48000., LATENCY);
            let mut e = Engine::new(48000.);
            e.note_on(note, velocity as f32 / 127.);
            alternate.strike(hammer_speed(velocity as f32 / 127.));
            let mut w = WavWriter::create(path.join(format!("vv-{note}-{velocity}.wav")), spec)?;
            for i in 0..57600 {
                if i == 48000 {
                    e.note_off(note);
                    alternate.release();
                }
                let y = e.next_outputs();
                for _ in 0..4 {
                    decimator.push([alternate.step(0.).pickup_v]);
                }
                let [smoothed] = decimator.read();
                for v in [y.pickup_v, y.harp_v, y.jack_v, smoothed] {
                    w.write_sample(v as f32)?;
                }
            }
            w.finalize()?;
        }
    }
    Ok(())
}

fn bench() {
    for rate in [44100, 48000, 96000] {
        let mut e = Engine::new(f64::from(rate));
        for keys in [1, 8, 16, 32, 64] {
            e.reset();
            for k in 0..keys {
                e.note_on(LOW + k, 0.8);
            }
            let mut buf = [0_f32; 64];
            let start = Instant::now();
            for _ in 0..rate / 64 {
                e.render(std::hint::black_box(&mut buf));
            }
            println!("{rate},{keys},{:.1}", 1000. * start.elapsed().as_secs_f64());
        }
    }
}
fn panel() {
    for (name, c) in [
        ("open", Controls::default()),
        (
            "lowcut",
            Controls {
                bass: 0.,
                ..Controls::default()
            },
        ),
        (
            "highcut",
            Controls {
                treble: 0.,
                ..Controls::default()
            },
        ),
        (
            "both",
            Controls {
                volume: 0.65,
                bass: 0.3,
                treble: 0.4,
            },
        ),
    ] {
        let mut p = Panel::new(48000., c);
        for f in [40., 100., 440., 1000., 4000., 10000.] {
            p.reset();
            let mut ym = 0.;
            let mut xm = 0.;
            let mut phase = 0.;
            // Prewarp the host test frequency to compare to an analog SPICE sweep.
            let hz = 48000. / std::f64::consts::PI * (std::f64::consts::PI * f / 48000.).atan();
            for i in 0..48000 {
                let x = f64::sin(phase);
                phase += std::f64::consts::TAU * hz / 48000.;
                let y = p.tick(x).jack_v;
                if i >= 24000 {
                    ym += y * y;
                    xm += x * x;
                }
            }
            println!("{name},{f:.0},{:.6}", 10. * (ym / xm).log10());
        }
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some(c @ ("captures" | "references")) => captures(
            Path::new(
                args.get(2)
                    .ok_or("captures needs a destination directory")?,
            ),
            c == "references",
        )?,
        Some("comparison") => comparison(Path::new(
            args.get(2).ok_or("comparison needs a directory")?,
        ))?,
        Some("bench") => bench(),
        Some("panel") => panel(),
        _ => return Err("use captures DIRECTORY, references DIRECTORY, bench or panel".into()),
    }
    Ok(())
}
