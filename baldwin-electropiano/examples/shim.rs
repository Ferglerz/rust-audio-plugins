//! Separate the load of a preloaded shim and rail from a fixed hammer strike.
use baldwin_electropiano::{
    action::{frequency, Action},
    rail::{BearingForces, Rail},
};
use pleasant_dsp::struck_keys::StruckAction;
use std::f64::consts::TAU;
fn main() {
    const RATE: f64 = 48000.;
    println!("midi,stiffness_N_m,damping_Ns_m,p1_N,p2_N,p3_N,p4_N,p5_N,p6_N");
    for n in [38, 43, 53, 54, 64, 67, 69, 74, 76] {
        let mut a = Action::with_regulation(n, RATE, 1., 1., 1.);
        let b = a.inharmonicity();
        a.strike_key(0.7);
        let source: Vec<_> = (0..13920).map(|_| a.step_output(0.)).collect();
        let coeff = std::array::from_fn::<_, 6, _>(|i| {
            let h = (i + 1) as f64;
            (TAU * frequency(n) * h * ((1. + b * h * h) / (1. + b)).sqrt() / RATE).sin_cos()
        });
        for k in [0.25e6, 0.8e6, 1.6e6, 3.2e6, 6.4e6] {
            for c in [10., 45., 100., 180.] {
                let mut rail = Rail::with_shim(RATE, k, c);
                let mut osc = [[0., 1.]; 6];
                let mut sums = [[0.; 2]; 6];
                for (i, &force) in source.iter().enumerate() {
                    let mut p = BearingForces::default();
                    p.add(n, force);
                    let [lo, hi] = rail.sample(p, 1u128 << n, false);
                    if i < 6240 {
                        continue;
                    }
                    let w = 0.5 - 0.5 * (TAU * (i - 6240) as f64 / 7680.).cos();
                    let v = (lo + hi) * w;
                    for j in 0..6 {
                        let [s, c] = osc[j];
                        sums[j][0] += v * s;
                        sums[j][1] += v * c;
                        let (sn, cs) = coeff[j];
                        osc[j] = [s * cs + c * sn, c * cs - s * sn];
                    }
                }
                print!("{n},{k},{c}");
                for v in sums {
                    print!(",{:.8}", v[0].hypot(v[1]) * 4. / 7680.);
                }
                println!();
            }
        }
    }
}
