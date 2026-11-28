//! Measure the first six bearing-force modes for candidate hammer regulations.
use baldwin_electropiano::action::{frequency, Action};
use pleasant_dsp::struck_keys::StruckAction;
use std::f64::consts::TAU;
fn main() {
    const RATE: f64 = 48_000.;
    const COUNT: usize = 7680;
    println!("midi,felt,mass,line,p1_N,p2_N,p3_N,p4_N,p5_N,p6_N");
    for n in [38, 43, 53, 54, 64, 67, 69, 74, 76] {
        for felt in [0.035, 0.075, 0.15, 0.30, 0.6, 1., 1.6, 3., 5., 8.] {
            for mass in [0.45, 0.70, 1., 1.4] {
                for strike in [0.75, 1., 1.22] {
                    let mut a = Action::with_regulation(n, RATE, felt, mass, strike);
                    let b = a.inharmonicity();
                    let mut amplitudes = [[0.; 2]; 6];
                    let frequency = std::array::from_fn::<_, 6, _>(|i| {
                        let order = (i + 1) as f64;
                        frequency(n) * order * ((1. + b * order * order) / (1. + b)).sqrt()
                    });
                    let lines = frequency.map(|f| (TAU * f / RATE).sin_cos());
                    let mut osc = [[0., 1.]; 6];
                    a.strike_key(0.7);
                    for _ in 0..6240 {
                        a.step_output(0.);
                    }
                    for i in 0..COUNT {
                        let w = 0.5 - 0.5 * (TAU * i as f64 / COUNT as f64).cos();
                        let v = a.step_output(0.) * w;
                        for j in 0..6 {
                            let [s, c] = osc[j];
                            amplitudes[j][0] += v * s;
                            amplitudes[j][1] += v * c;
                            let (sn, cs) = lines[j];
                            osc[j] = [s * cs + c * sn, c * cs - s * sn];
                        }
                    }
                    print!("{n},{felt},{mass},{strike}");
                    for a in amplitudes {
                        print!(",{:.8}", a[0].hypot(a[1]) * 4. / COUNT as f64);
                    }
                    println!();
                }
            }
        }
    }
}
