//! Hold geometry, pickup and amplifier fixed while changing the compressive felt law.
use hound::{SampleFormat, WavSpec, WavWriter};
use neo_bechstein::{
    action::{scale, Action},
    amp::{Amplifier, Controls},
    pickup::{head, Pickups, HEADS},
};
use std::{env, error::Error, fs, path::Path};
fn main() -> Result<(), Box<dyn Error>> {
    let folder = env::args().nth(1).ok_or("supply output path")?;
    let rate = 44100.;
    for q in [1., 0.3, 0.1, 0.04, 0.015, 0.006, 0.002] {
        for l in [1., 0.35, 0.1] {
            let dir = Path::new(&folder).join(format!("quadratic-{q}-linear-{l}"));
            fs::create_dir_all(&dir)?;
            for notes in [[65, 68], [66, 69]] {
                let mut actions = notes.map(|n| {
                    let mut s = scale(n).unwrap();
                    // Sweep the original pre-fit law, independent of future voicing changes.
                    let reg = f64::from(n - neo_bechstein::action::LOW) / 87.;
                    s.contact_k = 9.2e6 * 3.5_f64.powf(reg) * q;
                    s.contact_linear = 230. * l;
                    Action::from_spec(s, rate).unwrap()
                });
                // Common low drive keeps the electrical board out of overload in
                // all candidates, including the original much stiffer contact.
                let mut pickups = Pickups::with_linkage(rate, 0.001524);
                let mut amp = Amplifier::new(rate, Controls::default());
                let mut w = WavWriter::create(
                    dir.join(format!("midi-{}+{}-v80.wav", notes[0], notes[1])),
                    WavSpec {
                        channels: 3,
                        sample_rate: rate as u32,
                        bits_per_sample: 32,
                        sample_format: SampleFormat::Float,
                    },
                )?;
                for i in 0..(rate * 0.245) as usize {
                    let mut motion = [0.; HEADS];
                    for (a, n) in actions.iter_mut().zip(notes) {
                        if i == (rate / 50.) as usize {
                            a.strike(80. / 127.);
                        }
                        motion[head(n)] += pickups.motion(a.sample(), a.strings);
                    }
                    let (emfs, v) = pickups.tick(motion);
                    for tap in [emfs[head(notes[0])], v, amp.tick(v)] {
                        w.write_sample(tap as f32)?;
                    }
                }
                w.finalize()?;
            }
        }
    }
    Ok(())
}
