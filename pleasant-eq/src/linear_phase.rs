use crate::{BandCoefficients, BandSettings};
use pleasant_dsp::units::db_to_linear;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

pub const PARTITION_SIZE: usize = 1024;
const FFT_SIZE: usize = PARTITION_SIZE * 2;

pub struct LinearPhaseEq {
    kernels: Vec<Vec<Complex<f64>>>,
    history: [Vec<Vec<Complex<f64>>>; 2],
    input: [[f64; 2]; PARTITION_SIZE],
    output: [[f64; 2]; PARTITION_SIZE],
    overlap: [[f64; 2]; PARTITION_SIZE],
    work: Vec<Complex<f64>>,
    scratch: Vec<Complex<f64>>,
    forward: Arc<dyn Fft<f64>>,
    inverse: Arc<dyn Fft<f64>>,
    position: usize,
    head: usize,
    latency_samples: u32,
}

impl LinearPhaseEq {
    /// Prepare a linear-phase kernel. `order` is rounded up to whole partitions.
    pub fn prepare(bands: &[BandSettings], sample_rate: f64, order: usize) -> Self {
        let order = order.max(PARTITION_SIZE * 2).div_ceil(PARTITION_SIZE) * PARTITION_SIZE;
        let design_size = (order * 2).next_power_of_two();
        let mut planner = FftPlanner::<f64>::new();
        let design_inverse = planner.plan_fft_inverse(design_size);
        let prepared: Vec<_> = bands
            .iter()
            .filter(|band| band.enabled)
            .map(|band| BandCoefficients::prepare(band, sample_rate))
            .collect();
        let mut response: Vec<_> = (0..design_size)
            .map(|index| {
                let frequency =
                    index.min(design_size - index) as f64 * sample_rate / design_size as f64;
                let decibels: f64 = prepared
                    .iter()
                    .map(|coefficients| coefficients.response_db(frequency, sample_rate))
                    .sum();
                Complex::new(db_to_linear(decibels.clamp(-240.0, 120.0)), 0.0)
            })
            .collect();
        design_inverse.process(&mut response);

        let forward = planner.plan_fft_forward(FFT_SIZE);
        let inverse = planner.plan_fft_inverse(FFT_SIZE);
        let kernel: Vec<_> = (0..=order)
            .map(|index| {
                let source = (index + design_size - order / 2) % design_size;
                let phase = std::f64::consts::TAU * index as f64 / order as f64;
                response[source].re / design_size as f64
                    * (0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos())
            })
            .collect();
        let kernels: Vec<_> = kernel
            .chunks(PARTITION_SIZE)
            .map(|chunk| {
                let mut partition = vec![Complex::default(); FFT_SIZE];
                for (value, coefficient) in partition.iter_mut().zip(chunk) {
                    value.re = *coefficient;
                }
                forward.process(&mut partition);
                partition
            })
            .collect();
        let history =
            std::array::from_fn(|_| vec![vec![Complex::default(); FFT_SIZE]; kernels.len()]);
        let scratch = vec![
            Complex::default();
            forward
                .get_inplace_scratch_len()
                .max(inverse.get_inplace_scratch_len())
        ];
        Self {
            kernels,
            history,
            input: [[0.0; 2]; PARTITION_SIZE],
            output: [[0.0; 2]; PARTITION_SIZE],
            overlap: [[0.0; 2]; PARTITION_SIZE],
            work: vec![Complex::default(); FFT_SIZE],
            scratch,
            forward,
            inverse,
            position: 0,
            head: 0,
            latency_samples: (order / 2 + PARTITION_SIZE) as u32,
        }
    }

    pub fn process_sample(&mut self, input: [f64; 2]) -> [f64; 2] {
        let output = self.output[self.position];
        self.input[self.position] = input;
        self.position += 1;
        if self.position == PARTITION_SIZE {
            self.position = 0;
            for channel in 0..2 {
                let current = &mut self.history[channel][self.head];
                current.fill(Complex::default());
                for (value, input) in current.iter_mut().zip(&self.input) {
                    value.re = input[channel];
                }
                self.forward
                    .process_with_scratch(current, &mut self.scratch);
                self.work.fill(Complex::default());
                for (partition, kernel) in self.kernels.iter().enumerate() {
                    let source = &self.history[channel]
                        [(self.head + self.kernels.len() - partition) % self.kernels.len()];
                    for ((sum, input), coefficient) in self.work.iter_mut().zip(source).zip(kernel)
                    {
                        *sum += input * coefficient;
                    }
                }
                self.inverse
                    .process_with_scratch(&mut self.work, &mut self.scratch);
                for index in 0..PARTITION_SIZE {
                    self.output[index][channel] =
                        self.work[index].re / FFT_SIZE as f64 + self.overlap[index][channel];
                    self.overlap[index][channel] =
                        self.work[index + PARTITION_SIZE].re / FFT_SIZE as f64;
                }
            }
            self.head = (self.head + 1) % self.kernels.len();
        }
        output
    }

    pub fn reset(&mut self) {
        for channel in &mut self.history {
            for partition in channel {
                partition.fill(Complex::default());
            }
        }
        self.input.fill([0.0; 2]);
        self.output.fill([0.0; 2]);
        self.overlap.fill([0.0; 2]);
        self.position = 0;
        self.head = 0;
    }

    pub fn latency_samples(&self) -> u32 {
        self.latency_samples
    }
}
