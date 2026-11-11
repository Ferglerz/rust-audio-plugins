//! The original two-stage DTLN speech denoiser, with streaming state per channel.
use crate::{
    model::{load_session, run_prepared},
    weighting::LevelTracker,
};
use ort::{session::Session, value::Tensor};
use rustfft::{num_complex::Complex32, Fft, FftPlanner};
use std::{ffi::CStr, sync::Arc};

pub const RATE: f64 = 16_000.0;
pub const HOP: usize = 128;
pub const LATENCY: usize = 512;
const BLOCK: usize = LATENCY;
const BINS: usize = BLOCK / 2 + 1;

struct Stage {
    session: Session,
    inputs: [&'static CStr; 2],
    outputs: [&'static CStr; 2],
}

struct Tensors {
    input: Tensor<f32>,
    output: Tensor<f32>,
    state: Tensor<f32>,
    next_state: Tensor<f32>,
}

impl Stage {
    fn tensors(&mut self, width: usize) -> Result<Tensors, String> {
        let mut tensors = (|| -> ort::Result<Tensors> {
            Ok(Tensors {
                input: Tensor::from_array(([1, 1, width], vec![0.0; width]))?,
                output: Tensor::from_array(([1, 1, width], vec![0.0; width]))?,
                state: Tensor::from_array(([1, 2, 128, 2], vec![0.0; 512]))?,
                next_state: Tensor::from_array(([1, 2, 128, 2], vec![0.0; 512]))?,
            })
        })()
        .map_err(|error| error.to_string())?;
        if !self.run(&mut tensors) {
            return Err("DTLN model warm-up failed".into());
        }
        tensors.reset();
        Ok(tensors)
    }

    fn run(&mut self, tensors: &mut Tensors) -> bool {
        let valid = run_prepared(
            &mut self.session,
            self.inputs,
            self.outputs,
            [&tensors.input, &tensors.state],
            [&mut tensors.output, &mut tensors.next_state],
        ) && tensors
            .output
            .extract_tensor()
            .1
            .iter()
            .chain(tensors.next_state.extract_tensor().1)
            .all(|x| x.is_finite());
        if valid {
            std::mem::swap(&mut tensors.state, &mut tensors.next_state);
        }
        valid
    }
}

impl Tensors {
    fn reset(&mut self) {
        self.state.extract_tensor_mut().1.fill(0.0);
        self.next_state.extract_tensor_mut().1.fill(0.0);
    }
}

pub struct Model {
    mask: Stage,
    waveform: Stage,
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
}

impl Model {
    pub fn new() -> Result<Self, String> {
        let mut planner = FftPlanner::new();
        Ok(Self {
            mask: Stage {
                session: load_session(include_bytes!("../assets/dtln_1.onnx"))?,
                inputs: [c"input_2", c"input_3"],
                outputs: [c"activation_2", c"tf_op_layer_stack_2"],
            },
            waveform: Stage {
                session: load_session(include_bytes!("../assets/dtln_2.onnx"))?,
                inputs: [c"input_4", c"input_5"],
                outputs: [c"conv1d_3", c"tf_op_layer_stack_5"],
            },
            forward: planner.plan_fft_forward(BLOCK),
            inverse: planner.plan_fft_inverse(BLOCK),
        })
    }

    pub fn channel(&mut self) -> Result<Channel, String> {
        Ok(Channel {
            mask: self.mask.tensors(BINS)?,
            waveform: self.waveform.tensors(BLOCK)?,
            input: [0.0; BLOCK],
            overlap: [0.0; BLOCK],
            output: [0.0; HOP],
            spectrum: [Complex32::default(); BLOCK],
            scratch: vec![
                Complex32::default();
                self.forward
                    .get_inplace_scratch_len()
                    .max(self.inverse.get_inplace_scratch_len())
            ],
            position: 0,
            recovery: 0,
            tracker: LevelTracker::new(HOP as f64 / RATE),
        })
    }
}

pub struct Channel {
    mask: Tensors,
    waveform: Tensors,
    input: [f32; BLOCK],
    overlap: [f32; BLOCK],
    output: [f32; HOP],
    spectrum: [Complex32; BLOCK],
    scratch: Vec<Complex32>,
    position: usize,
    recovery: usize,
    tracker: LevelTracker,
}

impl Channel {
    pub fn recovering(&self) -> bool {
        self.recovery > 0
    }

    pub fn reset(&mut self) {
        self.mask.reset();
        self.waveform.reset();
        self.input.fill(0.0);
        self.overlap.fill(0.0);
        self.output.fill(0.0);
        self.position = 0;
        self.recovery = 0;
        self.tracker.reset();
    }

    pub fn tick(&mut self, input: f32, model: &mut Model) -> f32 {
        let output = self.output[self.position];
        self.input[BLOCK - HOP + self.position] = input;
        self.position += 1;
        if self.position == HOP {
            self.position = 0;
            self.frame(model);
            self.input.copy_within(HOP.., 0);
        }
        output
    }

    fn frame(&mut self, model: &mut Model) {
        let power = self.input.iter().map(|&x| (x as f64).powi(2)).sum::<f64>() / BLOCK as f64;
        let restore = self.tracker.update(power).restore;
        let normalize_by = if restore > 0.0 { restore } else { 1.0 };
        for (bin, &sample) in self.spectrum.iter_mut().zip(&self.input) {
            *bin = Complex32::new((sample as f64 / normalize_by) as f32, 0.0);
        }
        model
            .forward
            .process_with_scratch(&mut self.spectrum, &mut self.scratch);
        for (mag, bin) in self
            .mask
            .input
            .extract_tensor_mut()
            .1
            .iter_mut()
            .zip(&self.spectrum)
        {
            *mag = bin.norm();
        }
        let valid = model.mask.run(&mut self.mask) && {
            let mask = self.mask.output.extract_tensor().1;
            for (bin, &gain) in self.spectrum.iter_mut().zip(mask) {
                *bin *= gain;
            }
            for i in BINS..BLOCK {
                self.spectrum[i] = self.spectrum[BLOCK - i].conj();
            }
            model
                .inverse
                .process_with_scratch(&mut self.spectrum, &mut self.scratch);
            for (sample, bin) in self
                .waveform
                .input
                .extract_tensor_mut()
                .1
                .iter_mut()
                .zip(&self.spectrum)
            {
                *sample = bin.re / BLOCK as f32;
            }
            model.waveform.run(&mut self.waveform)
        };
        if valid {
            // The learned decoder already produces the overlap-add contribution.
            // Restore each frame's original scale before summing overlapping frames.
            for (sum, &sample) in self
                .overlap
                .iter_mut()
                .zip(self.waveform.output.extract_tensor().1)
            {
                *sum += (sample as f64 * restore) as f32;
            }
        } else {
            self.mask.reset();
            self.waveform.reset();
            self.overlap.fill(0.0);
            self.recovery = BLOCK / HOP;
        }

        if self.recovering() {
            self.output.copy_from_slice(&self.input[..HOP]);
            if valid {
                self.recovery -= 1;
            }
        } else {
            self.output.copy_from_slice(&self.overlap[..HOP]);
        }
        self.overlap.copy_within(HOP.., 0);
        self.overlap[BLOCK - HOP..].fill(0.0);
    }
}

/// Apply the curve to the full-band, latency-aligned host signal. Blending here
/// keeps 0% Strength transparent, even when the wet path is resampled to 16 kHz.
pub struct Mix {
    detector: LevelTracker,
    level: crate::weighting::FrameLevel,
    weight: f32,
    power: f64,
    count: usize,
    window: usize,
}

impl Mix {
    pub fn new(sample_rate: f64) -> Self {
        let window = (sample_rate * HOP as f64 / RATE).round() as usize;
        Self {
            detector: LevelTracker::new(window as f64 / sample_rate),
            level: Default::default(),
            weight: 1.0,
            power: 0.0,
            count: 0,
            window,
        }
    }

    pub fn reset(&mut self) {
        self.detector.reset();
        self.level = Default::default();
        self.weight = 1.0;
        self.power = 0.0;
        self.count = 0;
    }

    pub fn activity(&self) -> (crate::weighting::FrameLevel, f32) {
        (self.level, self.weight)
    }

    pub fn tick(
        &mut self,
        dry: f32,
        wet: f32,
        strength: f32,
        weighting: crate::weighting::Weighting,
    ) -> f32 {
        self.power += (dry as f64).powi(2);
        self.count += 1;
        if self.count == self.window {
            self.level = self.detector.update(self.power / self.window as f64);
            self.power = 0.0;
            self.count = 0;
        }
        self.weight = weighting.amount(self.level);
        let amount = strength * self.weight;
        dry * (1.0 - amount) + wet * amount
    }
}
