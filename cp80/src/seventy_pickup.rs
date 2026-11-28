//! Trapezoidal solution of the 73 capacitive sources, two series mixing
//! resistors, C101 input coupling and R102 gate leak. Force inputs are summed
//! per electrical branch before scaling by the open-circuit piezo sensitivity.
#[derive(Clone)]
pub struct PickupBank {
    transition: [[f64; 4]; 4],
    drive: [[f64; 3]; 4],
    voltage: [f64; 4],
    last_force: [f64; 3],
}

impl PickupBank {
    pub fn new(rate: f64) -> Self {
        let mut capacitance = [[0.; 4]; 4];
        let ceramic = 1e-9; // Assumed 1 nF per identical pickup, including silent keys.
        for (i, count) in [63., 2., 8.].into_iter().enumerate() {
            capacitance[i][i] = count * ceramic;
        }
        capacitance[2][2] += 100e-9;
        capacitance[2][3] = -100e-9;
        capacitance[3][2] = -100e-9;
        capacitance[3][3] = 100e-9;
        let mix = 1. / 4700.;
        let conductance = [
            [mix, -mix, 0., 0.],
            [-mix, 2. * mix, -mix, 0.],
            [0., -mix, mix, 0.],
            [0., 0., 0., 1. / 470e3],
        ];
        // (2 fs C + G) V[n] = (2 fs C - G) V[n-1] + 2 fs ΔQ.
        // Eliminate the four unknown node voltages once at prepare time.
        let mut rows = [[0.; 11]; 4];
        for (i, row) in rows.iter_mut().enumerate() {
            for j in 0..4 {
                let y = 2. * rate * capacitance[i][j];
                row[j] = y + conductance[i][j];
                row[4 + j] = y - conductance[i][j];
            }
            if i < 3 {
                row[8 + i] = 2. * rate * ceramic;
            }
        }
        for i in 0..4 {
            let div = rows[i][i];
            for x in &mut rows[i][i..] {
                *x /= div;
            }
            let pivot = rows[i];
            for (j, row) in rows.iter_mut().enumerate() {
                if j == i {
                    continue;
                }
                let factor = row[i];
                for k in i..11 {
                    row[k] -= factor * pivot[k];
                }
            }
        }
        Self {
            transition: rows.map(|r| std::array::from_fn(|i| r[i + 4])),
            drive: rows.map(|r| std::array::from_fn(|i| r[i + 8])),
            voltage: [0.; 4],
            last_force: [0.; 3],
        }
    }
    pub fn reset(&mut self) {
        self.voltage.fill(0.);
        self.last_force.fill(0.);
    }
    pub fn tick(&mut self, force: [f64; 3]) -> f64 {
        let delta: [f64; 3] = std::array::from_fn(|i| force[i] - self.last_force[i]);
        self.last_force = force;
        self.voltage = std::array::from_fn(|i| {
            self.transition[i]
                .iter()
                .zip(self.voltage)
                .map(|(a, b)| a * b)
                .sum::<f64>()
                + self.drive[i]
                    .iter()
                    .zip(delta)
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
        });
        self.voltage[3]
    }
}

/// Working equivalent of the original's individually wired 73 sensors, summed at
/// the input boards. Ceramic and harness capacitance stay present for silent keys.
/// Board-side per-key equalization has not been traced. See research/cp70.
pub struct OriginalPickup(pleasant_dsp::piezo::PiezoInput);

impl OriginalPickup {
    const ELEMENT_F: f64 = 1e-9;
    const LEAD_F: f64 = 40e-12;
    const COUPLING_F: f64 = 100e-9;
    const GATE_OHM: f64 = 470e3;

    pub fn new(rate: f64) -> Self {
        let harness = 73. * (Self::ELEMENT_F + Self::LEAD_F);
        Self(pleasant_dsp::piezo::PiezoInput::new(
            rate,
            Self::ELEMENT_F,
            harness,
            Self::COUPLING_F,
            Self::GATE_OHM,
        ))
    }

    pub fn reset(&mut self) {
        self.0.reset();
    }

    /// Bearing forces sum exactly before this filter for equal branch impedances.
    pub fn tick(&mut self, summed_force: f64) -> f64 {
        self.0.tick(summed_force)
    }
}

#[cfg(test)]
mod original_tests {
    use super::*;

    #[test]
    fn star_loading_matches_continuous_capacitor_network() {
        let rate = 192_000.;
        for frequency in [20., 100., 1000., 10_000.] {
            let mut pickup = OriginalPickup::new(rate);
            let omega = std::f64::consts::TAU * frequency;
            let mut amplitude = (0., 0.);
            for i in 0..192_000 {
                let (sin, cos) = (omega * f64::from(i) / rate).sin_cos();
                let out = pickup.tick(sin);
                if i >= 96_000 {
                    amplitude.0 += sin * out;
                    amplitude.1 += cos * out;
                }
            }
            let harness = 73. * (OriginalPickup::ELEMENT_F + OriginalPickup::LEAD_F);
            let tau = OriginalPickup::GATE_OHM / (1. / harness + 1. / OriginalPickup::COUPLING_F);
            let expected =
                OriginalPickup::ELEMENT_F / harness * omega * tau / (omega * tau).hypot(1.);
            let measured = amplitude.0.hypot(amplitude.1) / 48_000.;
            assert!((20. * (measured / expected).log10()).abs() < 0.001);
            pickup.reset();
            assert_eq!(pickup.tick(0.), 0.);
        }
    }
}
