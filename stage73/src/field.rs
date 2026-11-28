//! Flux gradient in the transverse plane, tabulated at prepare time in mm.
//! Integrate a 4-mm-radius chamfered pole face. The radial continuation away
//! from the strike plane approximates the much smaller lateral motion.
use std::sync::OnceLock;
const RADII: usize = 385;
const GAPS: usize = 65;
const DR: f64 = 0.0625;
const DG: f64 = 0.0625;
pub(super) const POLE_RADIUS_MM: f64 = 4.0;
pub(super) const POLE_FLAT_HALF_HEIGHT_MM: f64 = 1.3;
pub(super) const MIN_GAP: f64 = 0.5;
pub(super) const MAX_GAP: f64 = MIN_GAP + (GAPS - 1) as f64 * DG;

pub(super) struct Field(Vec<f32>);

impl Field {
    pub fn shared() -> &'static Self {
        static TABLE: OnceLock<Field> = OnceLock::new();
        TABLE.get_or_init(|| {
            let mut face = Vec::new();
            for i in 0..24 {
                for j in 0..24 {
                    let x = (i as f64 + 0.5) / 3.0 - POLE_RADIUS_MM;
                    let y = (j as f64 + 0.5) / 3.0 - POLE_RADIUS_MM;
                    if x * x + y * y <= POLE_RADIUS_MM * POLE_RADIUS_MM {
                        let bevel = (x.abs() - POLE_FLAT_HALF_HEIGHT_MM).max(0.0);
                        let area = if bevel > 0.0 {
                            2.0_f64.sqrt() / 9.0
                        } else {
                            1.0 / 9.0
                        };
                        face.push((x, y * y, bevel, area));
                    }
                }
            }
            let mut gradient = Vec::with_capacity(RADII * GAPS);
            for g in 0..GAPS {
                let gap = MIN_GAP + g as f64 * DG;
                for r in 0..RADII {
                    let radius = r as f64 * DR;
                    let (mut field, mut slope) = (0.0, 0.0);
                    for &(x, y2, bevel, area) in &face {
                        let dx = radius - x;
                        let distance = gap + bevel;
                        let d2 = dx * dx + y2 + distance * distance;
                        let contribution = area * distance / (d2 * d2.sqrt());
                        field += contribution;
                        slope -= 3.0 * dx * contribution / d2;
                    }
                    // Minus d(field^2)/dr from Faraday's law.
                    gradient.push((-2.0 * field * slope) as f32);
                }
            }
            Field(gradient)
        })
    }

    #[inline]
    pub fn emf(
        &self,
        height: f64,
        lateral: f64,
        vertical_speed: f64,
        lateral_speed: f64,
        gap: f64,
    ) -> f64 {
        let radius = (height * height + lateral * lateral).sqrt();
        if radius < 1e-10 {
            return 0.0;
        }
        let r = (radius / DR).min((RADII - 1) as f64);
        let g = (gap.clamp(MIN_GAP, MAX_GAP) - MIN_GAP) / DG;
        let i = (r as usize).min(RADII - 2);
        let j = (g as usize).min(GAPS - 2);
        let row = |j: usize| {
            let a = self.0[j * RADII + i] as f64;
            a + (r - i as f64) * (self.0[j * RADII + i + 1] as f64 - a)
        };
        let a = row(j);
        let gradient = a + (g - j as f64) * (row(j + 1) - a);
        gradient * (height * vertical_speed + lateral * lateral_speed) / radius
    }
}
