const OVERSAMPLING_FACTOR: usize = 4;
const TAPS: usize = 257;

/// Four-times oversampling shell. Expensive filter processing is supplied by caller.
pub struct OversampledEq {
    kernel: [f64; TAPS],
    up: [[f64; 2]; TAPS],
    down: [[f64; 2]; TAPS],
    position: usize,
}

impl OversampledEq {
    pub fn new() -> Self {
        let mut kernel = std::array::from_fn(|index| {
            let time = index as f64 - (TAPS / 2) as f64;
            let cutoff = 0.11875;
            let sinc = if time == 0.0 {
                2.0 * cutoff
            } else {
                (std::f64::consts::TAU * cutoff * time).sin() / (std::f64::consts::PI * time)
            };
            let phase = std::f64::consts::TAU * index as f64 / (TAPS - 1) as f64;
            sinc * (0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos())
        });
        let sum: f64 = kernel.iter().sum();
        for coefficient in &mut kernel {
            *coefficient /= sum;
        }
        Self {
            kernel,
            up: [[0.0; 2]; TAPS],
            down: [[0.0; 2]; TAPS],
            position: 0,
        }
    }

    pub fn process(
        &mut self,
        input: [f64; 2],
        sample_rate: f64,
        mut process_oversampled: impl FnMut([f64; 2], f64) -> [f64; 2],
    ) -> [f64; 2] {
        let mut output = [0.0; 2];
        for phase in 0..OVERSAMPLING_FACTOR {
            self.up[self.position] = if phase == 0 {
                [
                    input[0] * OVERSAMPLING_FACTOR as f64,
                    input[1] * OVERSAMPLING_FACTOR as f64,
                ]
            } else {
                [0.0; 2]
            };
            let mut interpolated = [0.0; 2];
            for index in (phase..TAPS).step_by(OVERSAMPLING_FACTOR) {
                let value = self.up[(self.position + TAPS - index) % TAPS];
                for channel in 0..2 {
                    interpolated[channel] += self.kernel[index] * value[channel];
                }
            }
            self.down[self.position] =
                process_oversampled(interpolated, sample_rate * OVERSAMPLING_FACTOR as f64);
            if phase == 0 {
                for index in 0..TAPS {
                    let value = self.down[(self.position + TAPS - index) % TAPS];
                    for channel in 0..2 {
                        output[channel] += self.kernel[index] * value[channel];
                    }
                }
            }
            self.position = (self.position + 1) % TAPS;
        }
        output
    }

    pub fn reset(&mut self) {
        self.up.fill([0.0; 2]);
        self.down.fill([0.0; 2]);
        self.position = 0;
    }

    pub const fn latency_samples(&self) -> u32 {
        64
    }
}

impl Default for OversampledEq {
    fn default() -> Self {
        Self::new()
    }
}
