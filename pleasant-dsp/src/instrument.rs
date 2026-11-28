//! Numerical and scheduling primitives shared by the two electric pianos.

/// One quadrature oscillator step. The increments may include modal decay.
#[inline]
pub fn rotate(sine: f64, cosine: f64, sin_inc: f64, cos_inc: f64) -> (f64, f64) {
    (
        sine * cos_inc + cosine * sin_inc,
        cosine * cos_inc - sine * sin_inc,
    )
}

/// Deliver ordered host events at the start of their sample, including events
/// on the end boundary (which affect the following block).
pub fn render_events<S, E>(
    state: &mut S,
    output: &mut [f32],
    mut next: impl FnMut() -> Option<E>,
    timestamp: impl Fn(&E) -> u32,
    mut handle: impl FnMut(&mut S, &E),
    mut render: impl FnMut(&mut S, &mut [f32]),
) {
    let mut start = 0;
    while let Some(event) = next() {
        let end = (timestamp(&event) as usize).clamp(start, output.len());
        if start < end {
            render(state, &mut output[start..end]);
        }
        handle(state, &event);
        start = end;
    }
    if start < output.len() {
        render(state, &mut output[start..]);
    }
}

use std::f64::consts::PI;

/// Blackman-windowed decimator for aligned audio and physical instrument taps.
pub struct Decimator<const C: usize> {
    taps: Vec<f64>,
    history: Vec<[f64; C]>,
    cursor: usize,
}

impl<const C: usize> Decimator<C> {
    pub fn new(substeps: usize, host_rate: f64, latency: usize) -> Self {
        Self::with_cutoff(
            substeps,
            host_rate,
            latency,
            18_500.0_f64.min(0.43 * host_rate),
        )
    }

    pub fn with_cutoff(substeps: usize, host_rate: f64, latency: usize, cutoff_hz: f64) -> Self {
        assert!(substeps > 0 && latency > 0 && host_rate > 0.);
        assert!(cutoff_hz > 0. && cutoff_hz <= 0.5 * host_rate);
        let half = latency * substeps;
        let cutoff = cutoff_hz / (host_rate * substeps as f64);
        let mut taps: Vec<_> = (0..=2 * half)
            .map(|i| {
                let t = i as f64 - half as f64;
                let phi = PI * i as f64 / half as f64;
                let sinc = if i == half {
                    2.0 * cutoff
                } else {
                    (2.0 * PI * cutoff * t).sin() / (PI * t)
                };
                sinc * (0.42 - 0.5 * phi.cos() + 0.08 * (2.0 * phi).cos())
            })
            .collect();
        let sum: f64 = taps.iter().sum();
        for x in &mut taps {
            *x /= sum;
        }
        Self {
            history: vec![[0.0; C]; taps.len()],
            taps,
            cursor: 0,
        }
    }

    pub fn reset(&mut self) {
        self.history.fill([0.; C]);
        self.cursor = 0;
    }

    pub fn push(&mut self, sample: [f64; C]) {
        self.history[self.cursor] = sample;
        self.cursor = (self.cursor + 1) % self.history.len();
    }

    /// Read the first N channels at the current decimation phase.
    pub fn read<const N: usize>(&self) -> [f64; N] {
        assert!(N <= C);
        let mut result = [0.0; N];
        for (h, t) in self.history[self.cursor..]
            .iter()
            .chain(&self.history[..self.cursor])
            .zip(&self.taps)
        {
            for i in 0..N {
                result[i] += h[i] * t;
            }
        }
        result
    }
}
