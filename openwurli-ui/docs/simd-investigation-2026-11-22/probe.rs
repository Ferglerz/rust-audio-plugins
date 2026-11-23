use revised as simd;
use scalar as baseline;
use scalar as reuse;
use std::hint::black_box;
use std::time::Instant;

const FRAMES: usize = 24_064; // 94 blocks, just over half a second at 48 kHz
const BLOCK: usize = 256;

macro_rules! engine {
    ($dsp:ident, $heavy:expr, $count:expr, $release:expr, $rate:expr, $capture:expr) => {{
        let mode = if $heavy {
            $dsp::CircuitMode::Heavy
        } else {
            $dsp::CircuitMode::Fast
        };
        let mut engine = $dsp::WurliEngine::new_with_circuit_mode($rate, mode);
        engine.set_mlp_enabled(true);
        engine.set_noise_enabled(false);
        engine.set_speaker_character(0.65);
        engine.set_tremolo_depth(0.5);
        engine.ensure_buffer_capacity(1024);
        engine.warm_up();
        for i in 0..$count {
            engine.note_on_extended(33 + (i * 13 % 64) as u8, 0.8);
        }
        let mut block = [0.0; BLOCK];
        for _ in 0..16 {
            engine.render(&mut block);
        }
        if $release {
            for i in 0..$count {
                engine.note_off_extended(33 + (i * 13 % 64) as u8);
            }
        }
        let mut audio = if $capture {
            Vec::with_capacity(FRAMES)
        } else {
            Vec::new()
        };
        let start = Instant::now();
        for _ in 0..FRAMES / BLOCK {
            engine.render(&mut block);
            if $capture {
                audio.extend_from_slice(&block);
            } else {
                black_box(&block);
            }
        }
        (start.elapsed().as_nanos(), audio, engine.nan_guard_fires())
    }};
}

fn render_engine(
    variant: &str,
    heavy: bool,
    count: usize,
    release: bool,
    rate: f64,
    capture: bool,
) -> (u128, Vec<f32>, u64) {
    match variant {
        "baseline" => engine!(baseline, heavy, count, release, rate, capture),
        "reuse" => engine!(reuse, heavy, count, release, rate, capture),
        "simd" => engine!(simd, heavy, count, release, rate, capture),
        _ => unreachable!(),
    }
}

macro_rules! reeds {
    ($dsp:ident, $count:expr, $release:expr) => {{
        let cfg = $dsp::tables::CalibrationConfig::default();
        let mut reeds: Vec<_> = (0..$count)
            .map(|i| {
                $dsp::voice::Voice::build_reed(
                    33 + (i * 13 % 64) as u8,
                    0.8,
                    48_000.0,
                    true,
                    &cfg,
                    None,
                )
                .0
            })
            .collect();
        let mut block = [0.0f64; BLOCK];
        for _ in 0..16 {
            block.fill(0.0);
            for reed in &mut reeds {
                reed.render(&mut block);
            }
        }
        if $release {
            for (i, reed) in reeds.iter_mut().enumerate() {
                reed.start_damper(33 + (i * 13 % 64) as u8, 48_000.0, 1.0);
            }
        }
        let start = Instant::now();
        for _ in 0..FRAMES / BLOCK {
            block.fill(0.0);
            for reed in &mut reeds {
                reed.render(&mut block);
            }
            black_box(&block);
        }
        start.elapsed().as_nanos()
    }};
}

fn render_reeds(variant: &str, count: usize, release: bool) -> u128 {
    match variant {
        "reuse" => reeds!(reuse, count, release),
        "simd" => reeds!(simd, count, release),
        _ => unreachable!(),
    }
}

fn main() {
    let command = std::env::args()
        .nth(1)
        .expect("verify, bench-reed or bench-engine");
    if command == "verify" {
        let mut compared = 0;
        for rate in [44_100.0, 48_000.0, 96_000.0] {
            for heavy in [false, true] {
                for count in [1, 6, 32, 64] {
                    for release in [false, true] {
                        let (_, expected, guards) =
                            render_engine("baseline", heavy, count, release, rate, true);
                        assert_eq!(guards, 0);
                        for variant in ["simd"] {
                            let (_, actual, guards) =
                                render_engine(variant, heavy, count, release, rate, true);
                            assert_eq!(guards, 0);
                            for (i, (a, b)) in actual.iter().zip(&expected).enumerate() {
                                assert!(a.is_finite() && b.is_finite());
                                assert_eq!(
                                    a.to_bits(),
                                    b.to_bits(),
                                    "{variant} rate={rate}, heavy={heavy}, count={count}, release={release}, sample={i}"
                                );
                            }
                            compared += expected.len();
                        }
                    }
                }
            }
        }
        println!(
            "{{\"engine_samples_compared\":{compared},\"mismatches\":0,\"nonfinite\":0,\"nan_guards\":0}}"
        );
    } else if command == "bench-reed" || command == "bench-engine" {
        println!("kind,mode,voices,release,trial,variant,ns,frames");
        for count in [1, 6, 32, 64] {
            for release in [false, true] {
                for heavy in [false, true] {
                    if command == "bench-reed" && heavy {
                        continue;
                    }
                    for trial in 0..5 {
                        let order = if trial % 2 == 0 {
                            ["reuse", "simd"]
                        } else {
                            ["simd", "reuse"]
                        };
                        for variant in order {
                            if command == "bench-reed" {
                                let ns = render_reeds(variant, count, release);
                                println!(
                                    "reed,none,{count},{release},{trial},{variant},{ns},{FRAMES}"
                                );
                            } else {
                                let (ns, _, guards) =
                                    render_engine(variant, heavy, count, release, 48_000.0, false);
                                assert_eq!(guards, 0);
                                let mode = if heavy { "heavy" } else { "fast" };
                                println!(
                                    "engine,{mode},{count},{release},{trial},{variant},{ns},{FRAMES}"
                                );
                            }
                        }
                    }
                }
            }
        }
    } else {
        panic!("verify, bench-reed or bench-engine");
    }
}
