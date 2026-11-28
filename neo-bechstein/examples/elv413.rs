//! Impulse transfer and full-drive sine probes through the actual host-rate signal path.
use neo_bechstein::amp::{Amplifier, Controls};
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};
fn main() -> Result<(), Box<dyn Error>> {
    let dir: PathBuf = std::env::args_os()
        .nth(1)
        .ok_or("expected output directory")?
        .into();
    std::fs::create_dir_all(&dir)?;
    if std::env::args().nth(2).as_deref() == Some("stress") {
        return stress(&dir);
    }
    for rate in [44100, 48000, 96000] {
        for (i, (master, swell)) in [(9, 1.), (9, 0.), (3, 0.5), (12, 1.)]
            .into_iter()
            .enumerate()
        {
            let mut amp = Amplifier::new(f64::from(rate), Controls { master, swell });
            let mut out = BufWriter::new(File::create(dir.join(format!("ir-{rate}-{i}.bin")))?);
            for j in 0..65536 {
                out.write_all(&amp.linear(f64::from(j == 0)).to_le_bytes())?;
            }
        }
    }
    for vrms in [0.005_f64, 0.025, 0.05, 0.1, 0.2, 0.35, 0.5, 1., 2.] {
        let mut amp = Amplifier::new(
            48000.,
            Controls {
                master: 9,
                swell: 1.,
            },
        );
        let mut out = BufWriter::new(File::create(dir.join(format!("sine-{vrms:.3}.bin")))?);
        for j in 0..14400 {
            let y =
                amp.tick(vrms * 2_f64.sqrt() * (std::f64::consts::TAU * (j as f64) / 48.).sin());
            out.write_all(&y.to_le_bytes())?;
        }
    }
    Ok(())
}

// Repeat the two chords from the stock host headroom regression at both board settings.
// Extra input gain also reproduces the superseded high-sensitivity pickup experiment.
fn stress(dir: &std::path::Path) -> Result<(), Box<dyn Error>> {
    use neo_bechstein::keyboard::Keyboard;
    for rate in [44100_u32, 48000, 96000] {
        let mut keys = Keyboard::new(rate.into());
        for root in [75, 76] {
            keys.reset();
            for delta in [0, 7, 12, 16, 19, 23, 26, 28] {
                keys.note_on(root + delta, 1.);
            }
            let source: Vec<_> = (0..rate as usize * 18 / 100)
                .map(|_| keys.sample().input_v)
                .collect();
            for gain in [1., 6.58] {
                for (master, swell) in [(9, 0.), (12, 1.)] {
                    let mut amp = Amplifier::new(rate.into(), Controls { master, swell });
                    let mut file = BufWriter::new(File::create(
                        dir.join(format!("chord-{rate}-{root}-{gain:.2}-{master}.bin")),
                    )?);
                    let (mut pk_in, mut pk_out) = (0_f64, 0_f64);
                    for x in &source {
                        let x = x * gain;
                        let y = amp.tick(x);
                        assert!(y.is_finite());
                        pk_in = pk_in.max(x.abs());
                        pk_out = pk_out.max(y.abs());
                        file.write_all(&x.to_le_bytes())?;
                        file.write_all(&y.to_le_bytes())?;
                    }
                    println!("{rate} root {root} gain {gain:.2} master {master}: {pk_in:.3} V input, {pk_out:.3} V speaker");
                }
            }
        }
    }
    Ok(())
}
