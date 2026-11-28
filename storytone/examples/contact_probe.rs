//! Sweep upper-register hammer mass and stiffness against the owner's treble notes.
use pleasant_dsp::struck_string::{LiveCourse, Readout};
use std::f64::consts::TAU;
use storytone::engine::scale;

fn main() {
    let rate = 48000.;
    println!("note,velocity,mass_ratio,stiffness_ratio,h2_db,h3_db");
    for note in [64, 81, 83, 86] {
        for velocity in [0.45_f64, 0.6, 0.75] {
            for mass in [0.45, 0.7, 1., 1.4, 1.8] {
                for k in [
                    0.015625, 0.03125, 0.0625, 0.125, 0.25, 0.5, 1., 2., 4., 8., 16., 32., 64.,
                ] {
                    let mut s = scale(note).unwrap();
                    // Reconstruct the original unvoiced scale so the fit remains reproducible.
                    let t = f64::from(note - 21) / 87.;
                    s.hammer_kg = 0.010 * 0.25_f64.powf(t);
                    s.contact_k = 8e6 * 9.0_f64.powf(t);
                    s.readout = Readout::Displacement {
                        fraction: 0.12 + 0.07 * t,
                        aperture_m: 0.010_f64.min(s.length * 0.025),
                    };
                    s.hammer_kg *= mass;
                    s.contact_k *= k;
                    let freq: [f64; 3] = std::array::from_fn(|i| {
                        let h = (i + 1) as f64;
                        s.frequency_hz
                            * h
                            * ((1. + s.stiffness * h * h) / (1. + s.stiffness)).sqrt()
                    });
                    let mut string = LiveCourse::at_rates(s, rate * 4., 4);
                    string.strike_speed(0.16 + 3.4 * velocity.powf(1.5));
                    let mut last = 0.;
                    let mut a = [(0., 0.); 3];
                    for i in 0..6000 {
                        let x = string.host_sample();
                        let v = (x - last) * rate;
                        last = x;
                        if i >= 2400 {
                            let w = 0.5 - 0.5 * (TAU * (i - 2400) as f64 / 3600.).cos();
                            for (j, (sin, cos)) in a.iter_mut().enumerate() {
                                let (sn, cs) = (TAU * freq[j] * i as f64 / rate).sin_cos();
                                *sin += v * w * sn;
                                *cos += v * w * cs;
                            }
                        }
                    }
                    let rms = a.map(|(s, c)| s.hypot(c));
                    println!(
                        "{note},{velocity},{mass},{k},{:.3},{:.3}",
                        20. * (rms[1] / rms[0]).log10(),
                        20. * (rms[2] / rms[0]).log10()
                    );
                }
            }
        }
    }
}
