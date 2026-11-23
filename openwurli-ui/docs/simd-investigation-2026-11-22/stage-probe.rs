use std::hint::black_box;
use std::time::Instant;
pub mod tables {
    pub const NUM_MODES: usize = 7;
}
#[path = "__WRAPPER__/tests/support/revised_simd_reed_v091.rs"]
mod coherent;
#[path = "__WRAPPER__/tests/support/scalar_reed_v091.rs"]
mod scalar;
#[path = "__WRAPPER__/tests/support/simd_reed_v091.rs"]
mod simd;

#[unsafe(no_mangle)]
#[inline(never)]
pub fn scalar_render(reed: &mut scalar::ModalReed, output: &mut [f64]) {
    reed.render(output);
}
#[unsafe(no_mangle)]
#[inline(never)]
pub fn simd_render(reed: &mut simd::ModalReed, output: &mut [f64]) {
    reed.render(output);
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn coherent_render(reed: &mut coherent::ModalReed, output: &mut [f64]) {
    reed.render(output);
}

macro_rules! bench {
    ($module:ident, $render:ident, $stage:expr) => {{
        let ratios = [1.0, 6.267, 17.547, 34.386, 56.842, 85.1, 119.3];
        let amps = [1.0, 0.005, 0.0035, 0.0018, 0.0011, 0.0007, 0.0005];
        let decay = [2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0];
        let mut reed = $module::ModalReed::new(440.0, &ratios, &amps, &decay, 0.003, 0.8, 48_000.0);
        reed.set_air_drag(1.6, 1.2, 48_000.0);
        let mut block = [0.0; 256];
        for _ in 0..16 {
            $render(&mut reed, &mut block);
        }
        if $stage != "held" {
            reed.start_damper(60, 48_000.0, 1.0);
        }
        if $stage == "post" {
            for _ in 0..5 {
                $render(&mut reed, &mut block);
            }
        }
        let frames = if $stage == "ramp" { 1024 } else { 24064 };
        let start = Instant::now();
        for _ in 0..frames / 256 {
            block.fill(0.0);
            $render(&mut reed, &mut block);
            black_box(&block);
        }
        (start.elapsed().as_nanos(), frames)
    }};
}
fn main() {
    println!("stage,trial,variant,ns,frames");
    for stage in ["held", "ramp", "post", "release"] {
        for trial in 0..21 {
            for variant in if trial % 2 == 0 {
                ["scalar", "simd", "coherent"]
            } else {
                ["coherent", "simd", "scalar"]
            } {
                let (ns, frames) = match variant {
                    "scalar" => bench!(scalar, scalar_render, stage),
                    "simd" => bench!(simd, simd_render, stage),
                    _ => bench!(coherent, coherent_render, stage),
                };
                println!("{stage},{trial},{variant},{ns},{frames}");
            }
        }
    }
}
