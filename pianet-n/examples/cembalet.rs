//! Render original Cembalet 1.75 MHz receiver and Cembalet N type 650720 separately.
use hound::{SampleFormat, WavSpec, WavWriter};
use pianet_n::{
    cembalet::{Key, Receiver},
    circuit, Engine, Model,
};
use std::{
    error::Error,
    fs,
    io::{BufWriter, Write},
    path::Path,
};
const RATE: u32 = 48000;
fn wav(path: &Path, audio: &[f32]) -> Result<(), Box<dyn Error>> {
    let mut w = WavWriter::create(
        path,
        WavSpec {
            channels: 1,
            sample_rate: RATE,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        },
    )?;
    for &s in audio {
        w.write_sample(s)?;
    }
    w.finalize()?;
    Ok(())
}
fn rf_regulation() {
    let rate = 96000.;
    for (label, flank) in [("center", 0.), ("flank", std::f64::consts::FRAC_1_SQRT_2)] {
        let mut keys: Vec<_> = (54..64)
            .map(|n| {
                let mut k = Key::new(n, rate).unwrap();
                let rest = k.step().capacitance;
                k.press(80);
                (k, rest, Receiver::at_flank(rate, flank))
            })
            .collect();
        let mut bank = Receiver::at_flank(rate, flank);
        let (mut difference, mut energy) = (0., 0.);
        for _ in 0..48000 {
            let (mut total_c, mut separate) = (0., 0.);
            for (key, rest, receiver) in &mut keys {
                let dc = key.step().capacitance - *rest;
                total_c += dc;
                separate += receiver.tick(dc);
            }
            let together = bank.tick(total_c);
            energy += together * together;
            difference += (together - separate).powi(2);
        }
        println!(
            "ten-key RF {label} superposition residual {:.2} dB",
            10. * (difference / energy).log10()
        );
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    rf_regulation();
    let dir = std::env::args()
        .nth(1)
        .ok_or("usage: cembalet OUTPUT_DIR")?;
    let dir = Path::new(&dir);
    fs::create_dir_all(dir)?;
    for (model, label) in [(Model::Cembalet, "tube"), (Model::CembaletN, "650720")] {
        let mut phrase = Vec::new();
        let mut balance =
            BufWriter::new(fs::File::create(dir.join(format!("{label}-balance.csv")))?);
        writeln!(balance, "midi,rms_V")?;
        let mut bank = Engine::new(model, f64::from(RATE));
        for m in model.key_range() {
            bank.reset();
            bank.note_on(m, 80. / 127.);
            let mut y = [0.; 5760];
            bank.render(&mut y);
            let settled = &y[960..];
            let rms = (settled.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>()
                / settled.len() as f64)
                .sqrt();
            writeln!(balance, "{m},{rms:.9e}")?;
        }

        for note in [36, 60, 84] {
            for velocity in [35u8, 105] {
                let mut e = Engine::new(model, f64::from(RATE));
                let mut y = vec![0.; RATE as usize];
                e.note_on(note, f32::from(velocity) / 127.);
                e.render(&mut y[..18000]);
                e.note_off(note);
                e.render(&mut y[18000..]);
                wav(&dir.join(format!("{label}-{note}-{velocity}.wav")), &y)?;
                phrase.extend_from_slice(&y[..24000]);
            }
        }
        let mut e = Engine::new(model, f64::from(RATE));
        let mut held = vec![0.; 2 * RATE as usize];
        e.note_on(44, 80. / 127.);
        e.render(&mut held);
        wav(&dir.join(format!("{label}-held-44.wav")), &held)?;
        let mut sum = vec![0f64; 24000];
        let mut single = vec![0.; 24000];
        for n in [48, 55, 64] {
            e.reset();
            e.note_on(n, 0.75);
            e.render(&mut single);
            for (total, x) in sum.iter_mut().zip(&single) {
                *total += f64::from(*x);
            }
        }
        e.reset();
        for n in [48, 55, 64] {
            e.note_on(n, 0.75);
        }
        e.render(&mut single);
        let energy = single.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>();
        let difference = single
            .iter()
            .zip(sum)
            .map(|(x, y)| (f64::from(*x) - y).powi(2))
            .sum::<f64>();
        println!(
            "{label} summed-bank superposition residual {:.2} dB",
            10. * (difference / energy).log10()
        );
        e.reset();
        e.set_n_controls(0.8, true);
        for n in [48, 55, 60, 64] {
            e.note_on(n, 0.7);
        }
        let mut chord = vec![0.; RATE as usize];
        e.render(&mut chord[..32000]);
        for n in [48, 55, 60, 64] {
            e.note_off(n);
        }
        e.render(&mut chord[32000..]);
        phrase.extend(chord);
        wav(&dir.join(format!("cembalet-{label}-audition.wav")), &phrase)?;
    }
    let rate = 96_000;
    let mut w = BufWriter::new(fs::File::create(dir.join("mechanism.csv"))?);
    writeln!(
        w,
        "midi,time_s,key_fraction,armed,contact,pick_N,damper_N,tip_m,capacitance_F"
    )?;
    for note in [36, 60, 84] {
        let mut k = Key::new(note, f64::from(rate)).unwrap();
        for i in 0..rate / 2 {
            match i {
                100 => k.move_to(0.4, 80),
                6500 => k.release(),
                11200 | 25800 | 34500 => k.press(80),
                25500 | 32500 | 39800 => k.release(),
                _ => (),
            }
            let o = k.step();
            if i % 3 == 0 {
                writeln!(
                    w,
                    "{note},{:.8},{:.8},{},{},{:.9e},{:.9e},{:.9e},{:.9e}",
                    f64::from(i) / f64::from(rate),
                    o.key_fraction,
                    u8::from(o.pick_armed),
                    u8::from(o.pick_engaged),
                    o.pad_force,
                    o.damper_force,
                    o.reed_tip,
                    o.capacitance
                )?;
            }
        }
    }
    let mut w = BufWriter::new(fs::File::create(dir.join("rf.csv"))?);
    writeln!(w, "tone_hz,cap_mod_pF,frame,detector_V")?;
    for (tone, pf) in [(110., 0.5), (440., 0.7), (1000., 1.0)] {
        let mut r = Receiver::new(f64::from(RATE) * 2.);
        for i in 0..19200 {
            let dc = pf * 1e-12 * (std::f64::consts::TAU * tone * f64::from(i) / 96000.).sin();
            writeln!(w, "{tone},{pf},{i},{:.10e}", r.tick(dc))?;
        }
    }
    let mut triad = BufWriter::new(fs::File::create(dir.join("650720-triad.csv"))?);
    writeln!(triad, "low_V,mid_V,high_V")?;
    let mut keys = [60, 64, 67].map(|n| {
        let mut key = Key::for_model(Model::CembaletN, n, 192000.).unwrap();
        let old = key.step().capacitance;
        key.press(80);
        (key, old)
    });
    let mut panel = circuit::cembalet_n_jacks(192000., 100e3);
    for _ in 0..38400 {
        let mut drive = 0.;
        for (k, c) in &mut keys {
            let now = k.step().capacitance;
            drive += 300. * (now - *c) * 192000.;
            *c = now;
        }
        let [low, mid, high] = panel(drive);
        writeln!(triad, "{low:.10e},{mid:.10e},{high:.10e}")?;
    }
    let mut w = BufWriter::new(fs::File::create(dir.join("650720-impulse.csv"))?);
    writeln!(w, "low_V,mid_V,high_V")?;
    let mut probe = circuit::cembalet_n_jacks(192000., 100e3);
    for i in 0..192000 {
        let [low, mid, high] = probe(if i == 0 { 1e-6 } else { 0. });
        writeln!(w, "{low:.10e},{mid:.10e},{high:.10e}")?;
    }
    Ok(())
}
