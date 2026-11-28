//! Armature, jack and through-bar forces for the C 1004 reconstruction.
use hound::{SampleFormat, WavSpec, WavWriter};
use pianet_n::vivi::{Controls, Engine, Panel};
use std::{error::Error, fs, io::Write, path::Path};
const RATE: u32 = 44100;
fn main() -> Result<(), Box<dyn Error>> {
    let p = std::env::args().nth(1).ok_or("expected output directory")?;
    let root = Path::new(&p);
    fs::create_dir_all(root)?;
    let fmt = WavSpec {
        channels: 4,
        sample_rate: RATE,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut e = Engine::new(f64::from(RATE));
    e.set_extended(true);
    let mut stat = fs::File::create(root.join("action.csv"))?;
    writeln!(
        stat,
        "note,velocity,first_pick_ms,retract_ms,front_mm,rear_mm,peak_coil_V,peak_jack_V"
    )?;
    for n in (24..=96).chain([0, 127]) {
        let velocities: &[u8] = if [24, 48, 64, 84, 96].contains(&n) {
            &[32, 80, 115]
        } else {
            &[80]
        };
        for &v in velocities {
            e.reset();
            let mut writer = WavWriter::create(root.join(format!("note-{n}-{v}.wav")), fmt)?;
            let (mut first, mut retract) = (None, None);
            let mut peak = [0f64; 4];
            for i in 0..RATE * 2 {
                if i == RATE / 20 {
                    e.note_on(n, f32::from(v) / 127.);
                }
                if i == RATE * 15 / 10 {
                    e.note_off(n);
                }
                let y = e.sample();
                let o = e.observation(n).unwrap();
                if o.pick_force_n > 0. {
                    first.get_or_insert(i);
                }
                if !o.armed {
                    retract.get_or_insert(i);
                }
                for (j, x) in [f64::from(y), o.pickup_v, o.front_m, o.armature_m]
                    .into_iter()
                    .enumerate()
                {
                    peak[j] = peak[j].max(x.abs());
                    writer.write_sample(x as f32)?;
                }
            }
            writer.finalize()?;
            let ms = |i: Option<u32>| {
                i.map_or(0., |i| f64::from(i - RATE / 20) * 1000. / f64::from(RATE))
            };
            writeln!(
                stat,
                "{n},{v},{:.4},{:.4},{:.6},{:.6},{:.8},{:.8}",
                ms(first),
                ms(retract),
                1000. * peak[2],
                1000. * peak[3],
                peak[1],
                peak[0]
            )?;
        }
    }
    for load in [100_000, 1_000_000] {
        for volume in [0.5, 1.] {
            for expr in [0.5, 1.] {
                let mut b = Panel::new(88200., f64::from(load));
                b.set(Controls {
                    volume,
                    expression: expr,
                });
                let mut w = WavWriter::create(
                    root.join(format!("terminal-{load}-{volume}-{expr}.wav")),
                    WavSpec {
                        channels: 1,
                        sample_rate: 88200,
                        ..fmt
                    },
                )?;
                for i in 0..22050 {
                    w.write_sample(b.step(if i == 0 { 1. } else { 0. }) as f32)?;
                }
                w.finalize()?;
            }
        }
    }
    let mut st = fs::File::create(root.join("regulation.csv"))?;
    writeln!(st, "note,offset_mm,bass,full,held_rms_V,released_rms_V")?;
    let mut listen = WavWriter::create(
        root.join("pedals-and-pickup.wav"),
        WavSpec { channels: 1, ..fmt },
    )?;
    for (n, g, b, f) in [
        (47, 0., false, false),
        (47, 0., true, false),
        (48, 0., true, false),
        (48, 0., false, true),
        (64, -0.15, false, false),
        (64, 0., false, false),
        (64, 1., false, false),
    ] {
        e.reset();
        e.set_pickup_distance_mm(g);
        e.set_bass_sustain(b);
        e.set_sustain(f);
        e.note_on(n, 80. / 127.);
        let mut ss = [0.; 2];
        for i in 0..RATE {
            if i == RATE * 3 / 10 {
                e.note_off(n);
            }
            if i == RATE * 8 / 10 {
                e.set_sustain(false);
                e.set_bass_sustain(false);
            }
            let y = e.sample();
            if (RATE / 10..RATE / 5).contains(&i) {
                ss[0] += f64::from(y).powi(2);
            }
            if (RATE * 4 / 10..RATE / 2).contains(&i) {
                ss[1] += f64::from(y).powi(2);
            }
            listen.write_sample(140. * y)?;
        }
        let rms = |v: f64| (v / f64::from(RATE / 10)).sqrt();
        writeln!(st, "{n},{g},{b},{f},{:.9},{:.9}", rms(ss[0]), rms(ss[1]))?;
    }
    listen.finalize()?;
    Ok(())
}
