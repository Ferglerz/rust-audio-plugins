//! An asymmetric tuning fork driven by a graduated neoprene hammer.
use pleasant_dsp::beam::{beam_shape, BeamMode};
use std::f64::consts::{PI, TAU};
pub const LOW: u8 = 33;
pub const HIGH: u8 = 96;
/// Nominal spring position inferred from the maker performance’s isolated B4 attack.
pub const SPRING_AT: f64 = 0.939;
/// Factory replacement chart Rev C, notes A13–C76. Millimetres from the block.
const LENGTH_MM: [f64; 64] = [
    110.728, 108.347, 105.172, 103.188, 100.806, 98.425, 95.250, 92.867, 89.692, 87.312, 84.930,
    81.755, 79.771, 77.787, 76.200, 74.612, 71.438, 70.246, 67.864, 66.276, 65.397, 63.500, 61.912,
    59.926, 57.942, 56.355, 53.975, 53.180, 53.180, 50.800, 50.005, 48.417, 46.434, 45.242, 44.051,
    42.093, 41.153, 40.480, 39.688, 38.100, 37.305, 36.114, 34.925, 34.465, 34.130, 33.239, 32.146,
    30.955, 29.764, 29.367, 28.575, 27.780, 26.987, 26.987, 25.796, 25.001, 24.605, 24.209, 23.812,
    23.018, 23.019, 22.622, 21.828, 21.431,
];
pub fn length_mm(note: u8) -> f64 {
    LENGTH_MM[usize::from(note.clamp(LOW, HIGH) - LOW)]
}

pub struct Action {
    modes: [BeamMode; 4],
    hammer: Option<(f64, f64)>,
    hammer_mass: f64,
    stiffness: f64,
    dt: f64,
    held: bool,
    sustain: bool,
    damper: f64,
    damper_step: f64,
    damper_loss: f64,
    gap_m: f64,
    height: f64,
    last_flux: f64,
}
#[derive(Clone, Copy, Default)]
pub struct Motion {
    pub tine_m: f64,
    pub tonebar_m: f64,
    pub contact_n: f64,
    pub pickup_v: f64,
}

impl Action {
    pub fn new(note: u8, rate: f64, host_rate: f64) -> Self {
        Self::with_tine_length_mm(note, rate, host_rate, length_mm(note))
    }

    /// Bench substitution of the free shaft length. All other key regulation remains at stock.
    pub fn with_tine_length_mm(note: u8, rate: f64, host_rate: f64, length_mm: f64) -> Self {
        assert!(length_mm.is_finite() && length_mm > 0. && rate >= host_rate && host_rate >= 8000.);
        let nearest = note.clamp(LOW, HIGH);
        let reg = f64::from(nearest - LOW) / 63.;
        let length = length_mm * 0.001;
        // Uniform equivalent flexural diameter, with spring mass referred to the free end.
        let wire = 0.00185;
        let area = PI * 0.25 * wire * wire;
        let shaft_mass = 7850. * area * length;
        let beam_mass = 0.25 * shaft_mass;
        let bare_hz =
            1.875104_f64.powi(2) * wire * (2.0e11_f64 / 7850.).sqrt() / (4. * TAU * length.powi(2));
        let stock_hz = 440. * 2f64.powf((f64::from(nearest) - 69.) / 12.);
        let mass_ratio = ((bare_hz / stock_hz).powi(2) - 1.).max(0.03);
        let mass = beam_mass * (1. + mass_ratio);
        let hz = 440. * 2f64.powf((f64::from(note) - 69.) / 12.);
        let hit = 0.22 + reg * 0.19;
        let hit1 = beam_shape(1.875104, hit);
        // The common head couples the tine and heavier tonebar. Normalizing the
        // lower eigenfrequency retunes every real or extended key to its note.
        let mbar = mass * (12. + 7. * reg);
        let coupling = 0.037;
        let ret = 1.030 + 0.014 * reg;
        let a = 1. + coupling;
        let d = ret * ret + coupling * mass / mbar;
        let b = -coupling * (mass / mbar).sqrt();
        let radius = (a - d).hypot(2. * b);
        let lo = (a + d - radius) * 0.5;
        let hi = (a + d + radius) * 0.5;
        let norm = b.hypot(lo - a);
        let (u, v) = (b / norm, (lo - a) / norm);
        let t60 = 9.2 * 0.25_f64.powf(reg);
        let mode = |f: f64, t: f64, contact: f64, sensor: f64, ret: f64| {
            let pass = if f < 0.46 * host_rate { 1. } else { 0. };
            BeamMode::new(f, rate, t, contact * pass, sensor, ret)
        };
        let high_mode = |beta: f64, loss: f64| {
            let tip_ratio = beam_shape(beta, SPRING_AT) / beam_shape(1.875104, SPRING_AT);
            let loaded_mass = beam_mass * (1. + mass_ratio * tip_ratio * tip_ratio);
            let f = hz * (beta / 1.875104).powi(2) * (mass / loaded_mass / lo).sqrt();
            // The chart specifies a cut length, not the tapered shaft's modal
            // masses. A positive residue fitted on B4 enters both drive and
            // readout to preserve reciprocity of the hammer and pickup sites.
            let projection = 0.40 / loaded_mass.sqrt();
            mode(
                f,
                t60 * loss,
                beam_shape(beta, hit) * projection,
                projection,
                0.,
            )
        };
        let modes = [
            mode(
                hz,
                t60,
                hit1 * u / mass.sqrt(),
                u / mass.sqrt(),
                v / mbar.sqrt(),
            ),
            mode(
                hz * (hi / lo).sqrt(),
                t60 * 0.38,
                -hit1 * v / mass.sqrt(),
                -v / mass.sqrt(),
                u / mbar.sqrt(),
            ),
            high_mode(4.694091, 0.065),
            high_mode(7.854757, 0.029),
        ];
        Self {
            modes,
            hammer: None,
            hammer_mass: 0.0058 * 0.50_f64.powf(reg),
            stiffness: 3.2e6 * 7_f64.powf(reg),
            dt: 1. / rate,
            held: false,
            sustain: false,
            damper: 1.,
            damper_step: 1. / (0.005 * rate),
            damper_loss: 1. - 10_f64.powf(-3. / (rate * (0.070 * 0.30_f64.powf(reg)))),
            gap_m: 0.00155 - 0.00033 * reg,
            height: 0.00060 + 0.00050 * reg,
            last_flux: 0.,
        }
    }
    pub fn strike(&mut self, speed: f64) {
        if !speed.is_finite() || speed <= 0. {
            return;
        }
        self.held = true;
        self.damper = 0.;
        self.hammer = Some((
            self.modes.iter().map(BeamMode::hit_displacement).sum(),
            speed.min(4.),
        ));
    }
    pub fn release(&mut self) {
        self.held = false;
    }
    pub fn sustain(&mut self, down: bool) {
        self.sustain = down;
    }
    pub fn clear(&mut self) {
        for m in &mut self.modes {
            m.clear();
        }
        self.held = false;
        self.sustain = false;
        self.damper = 1.;
        self.hammer = None;
        self.last_flux = 0.;
    }
    pub fn quiet(&self) -> bool {
        self.hammer.is_none() && self.modes.iter().all(|m| m.speed_bound() < 1e-8)
    }
    fn linkage(&self, tip: f64, gap: f64) -> f64 {
        // Finite-width end-pole approximation; radial offset and longitudinal
        // setback independently affect the field and its first/second derivatives.
        let gz = self.gap_m + gap;
        let r = self.height + tip;
        let pole = 0.0012;
        let s = pole * pole + gz * gz;
        3.2e-4 * pole * pole * ((s + r * r).recip() - (s + self.height * self.height).recip())
    }
    pub(crate) fn reposition(&mut self, gap: f64) {
        let tip = self.modes.iter().map(BeamMode::sensed_displacement).sum();
        self.last_flux = self.linkage(tip, gap);
    }
    pub fn step(&mut self, gap_m: f64) -> Motion {
        let mut force = 0.;
        if let Some((x, v)) = self.hammer {
            let compression = x - self
                .modes
                .iter()
                .map(BeamMode::hit_displacement)
                .sum::<f64>();
            if compression < 0. {
                self.hammer = None;
            } else {
                force = self.stiffness * compression * compression.sqrt();
                let v = v - self.dt * force / self.hammer_mass;
                self.hammer = Some((x + self.dt * v, v));
            }
        }
        self.damper = if self.held || self.sustain {
            (self.damper - self.damper_step).max(0.)
        } else {
            (self.damper + self.damper_step).min(1.)
        };
        for m in &mut self.modes {
            m.step(force * self.dt, 1. - self.damper * self.damper_loss);
        }
        let tip = self.modes.iter().map(BeamMode::sensed_displacement).sum();
        let flux = self.linkage(tip, gap_m);
        let emf = -(flux - self.last_flux) / self.dt;
        self.last_flux = flux;
        Motion {
            tine_m: tip,
            tonebar_m: self.modes.iter().map(BeamMode::return_displacement).sum(),
            contact_n: force,
            pickup_v: emf,
        }
    }
}
