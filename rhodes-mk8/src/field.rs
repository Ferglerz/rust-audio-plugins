//! Induced magnetization at the tine end from a uniformly excited flat pole.
//! All coordinates in millimetres. Absolute magnet strength is regulated in the action.
use std::sync::OnceLock;

const RADII: usize = 301;
const GAPS: usize = 81;
const DR: f64 = 0.05;
const DG: f64 = 0.05;
const MIN_GAP: f64 = 0.45;
pub const RADIUS_MM: f64 = 3.0;

pub struct FlatPole {
    energy: Vec<f64>,
}
impl FlatPole {
    pub fn shared() -> &'static Self {
        static POLE: OnceLock<FlatPole> = OnceLock::new();
        POLE.get_or_init(|| {
            // Integrate the normal field of a circular pole face with equal-area
            // annuli. The second magnetic pole is remote compared to tine travel.
            let mut points = Vec::with_capacity(12 * 48);
            for i in 0..12 {
                let r = RADIUS_MM * ((i as f64 + 0.5) / 12.).sqrt();
                for j in 0..48 {
                    let (s, c) = (std::f64::consts::TAU * (j as f64 + 0.5) / 48.).sin_cos();
                    points.push((r * c, (r * s).powi(2)));
                }
            }
            let mut energy = Vec::with_capacity(RADII * GAPS);
            for g in 0..GAPS {
                let z = MIN_GAP + DG * g as f64;
                for i in 0..RADII {
                    let r = DR * i as f64;
                    let bz = points
                        .iter()
                        .map(|&(x, y2)| {
                            let d2 = (r - x).powi(2) + y2 + z * z;
                            z / (d2 * d2.sqrt())
                        })
                        .sum::<f64>()
                        * std::f64::consts::PI
                        * RADIUS_MM.powi(2)
                        / points.len() as f64;
                    energy.push(bz * bz);
                }
            }
            Self { energy }
        })
    }
    pub fn linkage(&self, radial_mm: f64, gap_mm: f64) -> f64 {
        let r = (radial_mm.abs() / DR).min((RADII - 1) as f64);
        let g = ((gap_mm - MIN_GAP) / DG).clamp(0., (GAPS - 1) as f64);
        let i = (r as usize).min(RADII - 2);
        let j = (g as usize).min(GAPS - 2);
        let row = |j: usize| {
            let a = self.energy[j * RADII + i];
            a + (r - i as f64) * (self.energy[j * RADII + i + 1] - a)
        };
        row(j) + (g - j as f64) * (row(j + 1) - row(j))
    }
}
