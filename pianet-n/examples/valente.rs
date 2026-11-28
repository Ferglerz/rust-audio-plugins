//! Valente physical action, passive jack and terminal-network impulses.
use hound::{SampleFormat, WavSpec, WavWriter};
use pianet_n::valente::{Controls, Engine, Panel};
use std::{error::Error, fs, io::Write, path::Path};
const RATE: u32 = 44100;
fn main() -> Result<(), Box<dyn Error>> {
    let dir = std::env::args().nth(1).ok_or("expected output directory")?;
    let dir = Path::new(&dir);
    fs::create_dir_all(dir)?;
    let fmt = WavSpec {
        channels: 3,
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
        0, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 48, 49, 51, 53, 55, 58, 60, 63, 64, 65, 67,
        69, 70, 72, 74, 76, 84, 96, 127,
    ] {
        let velocities: &[u8] = if [0, 36, 64, 84, 96, 127].contains(&n) {
            &[8, 40, 80, 115][..]
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
                w.write_sample(o.reed_tip_m as f32)?;
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
    // All excerpts have the same gain. The left channel is the loaded jack,
    // the right channel is the primary reed's EMF before support transfer.
    let stereo = WavSpec { channels: 2, ..fmt };
    let mut clips = WavWriter::create(dir.join("regulation-and-pedal.wav"), stereo)?;
    let mut settings = fs::File::create(dir.join("regulation.csv"))?;
    writeln!(
        settings,
        "clearance_offset_mm,pedal,held_key_rms_V,released_100_200ms_rms_V"
    )?;
    for (offset, pedal) in [(-0.15, false), (0., false), (1., false), (0., true)] {
        e.reset();
        e.set_pickup_distance_mm(offset);
        e.set_sustain(pedal);
        let mut energy = [0.; 2];
        e.note_on(64, 80. / 127.);
        for i in 0..RATE {
            if i == RATE * 4 / 10 {
                e.note_off(64);
            }
            if i == RATE * 8 / 10 {
                e.set_sustain(false);
            }
            let y = e.sample();
            if (RATE / 10..RATE * 2 / 10).contains(&i) {
                energy[0] += f64::from(y).powi(2);
            }
            if (RATE * 5 / 10..RATE * 6 / 10).contains(&i) {
                energy[1] += f64::from(y).powi(2);
            }
            clips.write_sample(y)?;
            clips.write_sample(e.observation(64).unwrap().pickup_v as f32)?;
        }
        let rms = |v: f64| (v / f64::from(RATE / 10)).sqrt();
        writeln!(
            settings,
            "{offset},{pedal},{:.7},{:.7}",
            rms(energy[0]),
            rms(energy[1])
        )?;
        for _ in 0..RATE / 8 {
            clips.write_sample(0f32)?;
            clips.write_sample(0f32)?;
        }
    }
    clips.finalize()?;
    for load in [100000, 1000000] {
        for tone in [0, 50, 100] {
            for vol in [50, 100] {
                let mut e = Panel::new(88200., f64::from(load));
                e.set(Controls {
                    brilliance: f64::from(tone) / 100.,
                    volume: f64::from(vol) / 100.,
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
