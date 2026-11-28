use hound::{SampleFormat, WavSpec, WavWriter};
use rhodes_mk8::{
    action::{Action, HIGH, LOW},
    live::{hammer_speed, Engine},
    panel::{Controls, Panel, Wave},
};
use std::{error::Error, path::Path, time::Instant};

fn captures(path: &Path) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(path)?;
    let mut e = Engine::new(48000.);
    let spec = WavSpec {
        channels: 3,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    for note in [
        28, 31, 33, 36, 38, 41, 44, 45, 46, 52, 60, 61, 62, 65, 71, 74, 75, 77, 79, 80, 81, 84, 91,
        100,
    ] {
        for v in [32, 48, 64, 68, 72, 80, 96, 112, 127] {
            e.reset();
            e.note_on(note, v as f32 / 127.);
            let mut w = WavWriter::create(path.join(format!("n{note}-v{v}.wav")), spec)?;
            let mut peak = 0_f64;
            for i in 0..72000 {
                if i == 57600 {
                    e.note_off(note);
                }
                let x = e.next_outputs();
                for val in [x.send_v, x.left_v, x.right_v] {
                    w.write_sample(val as f32)?;
                }
                peak = peak.max(x.send_v.abs());
            }
            w.finalize()?;
            if [28, 36, 60, 84, 100].contains(&note) && [48, 80, 112].contains(&v) {
                let mut a = Action::new(note, 192000., 48000.);
                a.strike(hammer_speed(v as f32 / 127.));
                let mut contact = 0;
                let mut amp = 0_f64;
                for i in 0..19200 {
                    let m = a.step();
                    if m.contact_n > 0. {
                        contact = i;
                    }
                    amp = amp.max(m.tine_mm.abs());
                }
                println!("{note},{v},{peak:.5},{amp:.5},{:.3}", contact as f64 / 192.);
            }
        }
    }
    Ok(())
}
fn panel(path: &Path) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(path)?;
    let spec = WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    for (name, c) in [
        ("flat", Controls::default()),
        (
            "boost",
            Controls {
                low_db: 15.,
                mid_db: 15.,
                high_db: 15.,
                ..Controls::default()
            },
        ),
        (
            "cut",
            Controls {
                low_db: -15.,
                mid_db: -15.,
                high_db: -15.,
                ..Controls::default()
            },
        ),
        (
            "square",
            Controls {
                pan_enabled: true,
                pan_rate_hz: 1.686,
                pan_depth: 0.75,
                ..Controls::default()
            },
        ),
        (
            "ramp",
            Controls {
                pan_enabled: true,
                pan_rate_hz: 2.857,
                pan_depth: 0.69,
                wave: Wave::Ramp,
                ..Controls::default()
            },
        ),
        (
            "audio",
            Controls {
                pan_enabled: true,
                pan_rate_hz: 321.8,
                pan_depth: 0.85,
                wave: Wave::Sine,
                ..Controls::default()
            },
        ),
    ] {
        let mut p = Panel::new(48000., c);
        let mut w = WavWriter::create(path.join(format!("{name}.wav")), spec)?;
        for i in 0..480000 {
            let x = if c.pan_enabled {
                0.1 * (std::f64::consts::TAU * 137. * i as f64 / 48000.).sin()
            } else if i == 0 {
                0.1
            } else {
                0.
            };
            for v in p.process_return(x) {
                w.write_sample(v as f32)?;
            }
        }
        w.finalize()?;
    }
    Ok(())
}
fn bench(fx: bool) {
    for rate in [44100, 48000, 96000] {
        let mut e = Engine::new(f64::from(rate));
        if fx {
            e.set_fx_controls(rhodes_mk8::fx::Controls {
                fitted: true,
                compressor: true,
                chorus: true,
                phaser: true,
                delay: true,
                spread: 0.6,
                jitter: 0.4,
                ..Default::default()
            });
        }
        let mut block = [0_f32; 64];
        for count in [1, 8, 16, 32, 73] {
            e.reset();
            for n in LOW..LOW + count {
                e.note_on(n, 0.8);
            }
            let start = Instant::now();
            for _ in 0..rate / 64 {
                e.render(std::hint::black_box(&mut block));
            }
            println!("{rate},{count},{:.3}", start.elapsed().as_secs_f64());
        }
        assert_eq!(HIGH - LOW + 1, 73);
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("captures") => captures(Path::new(args.get(2).ok_or("missing capture directory")?))?,
        Some("panel") => panel(Path::new(args.get(2).ok_or("missing panel directory")?))?,
        Some("diodes") => {
            let mut p = Panel::new(
                48000.,
                Controls {
                    drive: 1.,
                    ..Controls::default()
                },
            );
            for x in [-1., -0.25, -0.05, 0.005, 0.025, 0.1, 0.5, 1.] {
                println!("{:.5},{:.8}", x * 36., p.process_return(x)[0]);
            }
        }
        Some("bench") => bench(false),
        Some("bench-fx") => bench(true),
        _ => return Err("use captures DIR, panel DIR or bench".into()),
    }
    Ok(())
}
