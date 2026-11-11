//! Causal windowed-sinc conversion. Plans are shared; rings and clocks are per channel.
//! Both directions run on sample clocks, independent of the host's block size.
use std::{f64::consts::PI, sync::Arc};

const BASE_DELAY: usize = 64;
const PHASES: usize = 1024;
const RING: usize = 512;

pub struct Plan {
    step: f64,
    extra_delay: f64,
    taps: usize,
    coefficients: Vec<f32>,
}

impl Plan {
    pub fn delay(input_rate: f64, output_rate: f64) -> usize {
        let step = input_rate / output_rate;
        // Keep the existing filters up to 8:1. At higher ratios, a longer
        // window keeps the transition band narrow enough to reject aliases.
        if step > 8.0 {
            BASE_DELAY * (step / 6.0).ceil() as usize
        } else {
            BASE_DELAY
        }
    }

    pub fn new(input_rate: f64, output_rate: f64, extra_delay: f64) -> Arc<Self> {
        let cutoff = (output_rate / input_rate).min(1.0) * 0.94;
        let delay = Self::delay(input_rate, output_rate);
        let taps = 2 * delay;
        let mut coefficients = vec![0.0; (PHASES + 1) * taps];
        for (phase, row) in coefficients.chunks_exact_mut(taps).enumerate() {
            let fraction = phase as f64 / PHASES as f64;
            let mut total = 0.0;
            for (k, weight) in row.iter_mut().enumerate() {
                let distance = k as f64 - delay as f64 + fraction;
                let x = distance * cutoff;
                let sinc = if x.abs() < 1e-12 {
                    1.0
                } else {
                    (PI * x).sin() / (PI * x)
                };
                let window = if distance.abs() <= delay as f64 {
                    0.42 + 0.5 * (PI * distance / delay as f64).cos()
                        + 0.08 * (2.0 * PI * distance / delay as f64).cos()
                } else {
                    0.0
                };
                *weight = (sinc * cutoff * window) as f32;
                total += *weight;
            }
            for weight in row {
                *weight /= total;
            }
        }
        Arc::new(Self {
            step: input_rate / output_rate,
            extra_delay,
            taps,
            coefficients,
        })
    }
}

pub struct Resampler {
    plan: Arc<Plan>,
    ring: Vec<f32>,
    input_count: u64,
    output_count: u64,
}

impl Resampler {
    pub fn new(plan: Arc<Plan>) -> Self {
        let ring = (plan.taps + plan.extra_delay.ceil() as usize)
            .next_power_of_two()
            .max(RING);
        Self {
            plan,
            ring: vec![0.0; ring],
            input_count: 0,
            output_count: 0,
        }
    }
    pub fn reset(&mut self) {
        self.ring.fill(0.0);
        self.input_count = 0;
        self.output_count = 0;
    }
    pub fn push(&mut self, sample: f32) {
        let index = self.input_count as usize & (self.ring.len() - 1);
        self.ring[index] = sample;
        self.input_count += 1;
    }
    /// Drain after each push. No allocation, locking, or coefficient generation.
    pub fn pop(&mut self) -> Option<f32> {
        let time = self.output_count as f64 * self.plan.step;
        if self.input_count == 0 || time > (self.input_count - 1) as f64 + 1e-9 {
            return None;
        }
        self.output_count += 1;
        let delayed = time - self.plan.extra_delay;
        let base = delayed.floor() as i64;
        let phase = (delayed - base as f64) * PHASES as f64;
        let index = (phase as usize).min(PHASES - 1);
        let blend = (phase - index as f64) as f32;
        let mut value = 0.0;
        let taps = self.plan.taps;
        let offset = index * taps;
        let a = &self.plan.coefficients[offset..offset + taps];
        let b = &self.plan.coefficients[offset + taps..offset + 2 * taps];
        let mask = self.ring.len() - 1;
        for (k, (&a, &b)) in a.iter().zip(b).enumerate() {
            let source = base - k as i64;
            if source >= 0 {
                value += self.ring[source as usize & mask] * (a + (b - a) * blend);
            }
        }
        Some(value)
    }
}

/// The round trip produces bursts when the host rate exceeds the model rate.
/// Prefill with enough samples to make output available on every host tick.
pub struct OutputQueue {
    samples: [f32; 64],
    read: usize,
    write: usize,
    count: usize,
    padding: usize,
}

impl OutputQueue {
    pub fn new(padding: usize) -> Self {
        Self {
            samples: [0.0; 64],
            read: 0,
            write: padding,
            count: padding,
            padding,
        }
    }
    pub fn reset(&mut self) {
        *self = Self::new(self.padding);
    }
    pub fn push(&mut self, sample: f32) {
        debug_assert!(self.count < self.samples.len());
        self.samples[self.write] = sample;
        self.write = (self.write + 1) % self.samples.len();
        self.count += 1;
    }
    pub fn pop(&mut self) -> f32 {
        debug_assert!(self.count > 0, "sample-rate converter output underrun");
        let sample = self.samples[self.read];
        self.read = (self.read + 1) % self.samples.len();
        self.count -= 1;
        sample
    }
}
