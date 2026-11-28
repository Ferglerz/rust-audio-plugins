//! EP-61C passive jack, harp and action observations, plus control-circuit impulses.
use hound::{SampleFormat, WavSpec, WavWriter};
use pianet_n::elepian::{Controls, Engine, Panel};
use std::{error::Error, fs, io::Write, path::Path};
const RATE: u32 = 44100;
fn main() -> Result<(), Box<dyn Error>> {
    let dir = std::env::args().nth(1).ok_or("expected output directory")?;
    let dir = Path::new(&dir);
    fs::create_dir_all(dir)?;
    let fmt = WavSpec {
        channels: 2,
        sample_rate: RATE,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut e = Engine::new(f64::from(RATE));
    e.set_extended(true);
    let mut stats = fs::File::create(dir.join("action.csv"))?;
    writeln!(
        stats,
        "midi,velocity,impact_ms,contact_ms,peak_force_N,peak_tip_mm,peak_harp_V,peak_jack_V"
    )?;
    for n in [
        0, 36, 39, 41, 42, 44, 46, 48, 49, 51, 53, 54, 55, 56, 58, 60, 64, 65, 67, 69, 70, 72, 76,
        84, 96, 127,
    ] {
        let velocities: &[u8] = if [0, 36, 64, 84, 96, 127].contains(&n) {
            &[40, 80, 115][..]
        } else {
            &[80][..]
        };
        for &vel in velocities {
            e.reset();
            let mut w = WavWriter::create(dir.join(format!("note-{n}-{vel}.wav")), fmt)?;
            let mut contact = 0;
            let mut onset = None;
            let (mut force, mut tip, mut harp, mut jack) = (0f64, 0f64, 0f64, 0f32);
            for i in 0..2 * RATE {
                if i == RATE / 20 {
                    e.note_on(n, f32::from(vel) / 127.);
                }
                if i == RATE * 3 / 2 {
                    e.note_off(n);
                }
                let y = e.sample();
                let o = e.observation(n).unwrap();
                if o.hammer_force_n > 0. && i < RATE / 2 {
                    onset.get_or_insert(i);
                    contact += 1;
                }
                force = force.max(o.hammer_force_n);
                tip = tip.max(o.reed_tip_m.abs());
                harp = harp.max(o.pickup_v.abs());
                jack = jack.max(y.abs());
                w.write_sample(y)?;
                w.write_sample(o.pickup_v as f32)?;
            }
            w.finalize()?;
            writeln!(
                stats,
                "{n},{vel},{:.3},{:.3},{:.8},{:.6},{:.6},{:.6}",
                f64::from(onset.unwrap_or(RATE / 20) - RATE / 20) * 1000. / f64::from(RATE),
                f64::from(contact) * 1000. / f64::from(RATE),
                force,
                tip * 1000.,
                harp,
                jack
            )?;
        }
    }
    for (label, alignment) in [("minus", -0.5), ("stock", 0.), ("plus", 0.5)] {
        e.set_controls(Controls {
            alignment_mm: alignment,
            ..Default::default()
        });
        e.reset();
        e.note_on(65, 80. / 127.);
        let mut w = WavWriter::create(dir.join(format!("voicing-{label}.wav")), fmt)?;
        for _ in 0..RATE {
            w.write_sample(e.sample())?;
            w.write_sample(e.observation(65).unwrap().pickup_v as f32)?;
        }
        w.finalize()?;
    }
    let mut sweep = fs::File::create(dir.join("regulation.csv"))?;
    writeln!(sweep, "lateral_offset_mm,F4_H2_dBc,F4_H3_dBc")?;
    for j in -40..=40 {
        let offset = f64::from(j) / 40.;
        e.set_controls(Controls {
            alignment_mm: offset,
            ..Default::default()
        });
        e.reset();
        e.note_on(65, 80. / 127.);
        let frequency = 440. * 2f64.powf(-4. / 12.);
        let mut real = [0.; 3];
        let mut imag = [0.; 3];
        let start = RATE * 13 / 100;
        let len = RATE * 16 / 100;
        for i in 0..start + len {
            e.sample();
            if i >= start {
                let theta = std::f64::consts::TAU * f64::from(i - start);
                let w = 0.5 - 0.5 * (theta / f64::from(len - 1)).cos();
                let y = e.observation(65).unwrap().pickup_v;
                for k in 0..3 {
                    let (a, b) = (theta * frequency * (k + 1) as f64 / f64::from(RATE)).sin_cos();
                    real[k] += w * y * b;
                    imag[k] += w * y * a;
                }
            }
        }
        let amp = |i: usize| real[i].hypot(imag[i]);
        writeln!(
            sweep,
            "{offset:.3},{:.3},{:.3}",
            20. * (amp(1) / amp(0)).log10(),
            20. * (amp(2) / amp(0)).log10()
        )?;
    }
    for load in [100000, 1000000] {
        for tone in [0, 50, 100] {
            for vol in [50, 100] {
                let mut e = Panel::new(88200., f64::from(load));
                e.set(Controls {
                    tone: f64::from(tone) / 100.,
                    volume: f64::from(vol) / 100.,
                    ..Default::default()
                });
                let spec = WavSpec {
                    channels: 1,
                    sample_rate: 88200,
                    ..fmt
                };
                let mut w =
                    WavWriter::create(dir.join(format!("panel-{load}-{tone}-{vol}.wav")), spec)?;
                for i in 0..22050 {
                    w.write_sample(e.step(if i == 0 { 1. } else { 0. }) as f32)?;
                }
                w.finalize()?;
            }
        }
    }
    Ok(())
}
