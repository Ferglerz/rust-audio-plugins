use crate::{
    dtln,
    model::{Model, Tensors, BINS},
    params::{EngineKind, Quality},
    resample::{OutputQueue, Plan, Resampler},
    weighting::{FrameLevel, LevelTracker, Weighting},
    wpe::OnlineWpe,
};
use rustfft::{
    num_complex::{Complex32, Complex64},
    Fft, FftPlanner,
};
use std::{f32::consts::PI, sync::Arc};

const FFT: usize = 960;
const MAX_MODEL_DELAY: usize = 4;
pub const MODEL_RATE: f64 = 48_000.0;

/// Interpolate magnitude and shortest phase arc before a single resynthesis.
/// Opposing phases must not cancel as they would in a waveform dry/wet blend.
pub(crate) fn spectral_strength(
    original: Complex32,
    enhanced: Complex32,
    strength: f32,
    real: bool,
) -> Complex32 {
    if strength <= 0.0 {
        return original;
    }
    if strength >= 1.0 {
        return enhanced;
    }
    if real {
        return Complex32::new(original.re + strength * (enhanced.re - original.re), 0.0);
    }
    let a = original.norm();
    let b = enhanced.norm();
    if a == 0.0 {
        return enhanced * strength;
    }
    if b == 0.0 {
        return original * (1.0 - strength);
    }
    let phase = original.arg();
    let mut delta = enhanced.arg() - phase;
    if delta > PI {
        delta -= 2.0 * PI;
    }
    if delta < -PI {
        delta += 2.0 * PI;
    }
    let magnitude = a * (1.0 - strength) + b * strength;
    // A heavily suppressed bin has an effectively undefined phase. Weight its
    // phase by its contribution to the result, otherwise FFT roundoff can rotate
    // the audible dry remainder. This ratio also has no absolute level threshold.
    let phase_weight = strength * b / magnitude;
    Complex32::from_polar(magnitude, phase + phase_weight * delta)
}

struct Transform {
    forward: Arc<dyn Fft<f64>>,
    inverse: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    room_seconds: f64,
}

enum Processor {
    Neural(Tensors),
    Wpe(OnlineWpe),
}

struct SpectralChannel {
    processor: Processor,
    input: Vec<f32>,
    output: Vec<f32>,
    spectrum: Vec<Complex32>,
    analysis: Vec<Complex64>,
    analysis_scratch: Vec<Complex64>,
    scratch: Vec<Complex32>,
    history: [[Complex32; BINS]; MAX_MODEL_DELAY],
    delay_frames: usize,
    position: usize,
    hop: usize,
    history_position: usize,
    recovery: usize,
    failed: bool,
    tracker: LevelTracker,
    levels: [FrameLevel; MAX_MODEL_DELAY],
    activity: (FrameLevel, f32),
    #[cfg(test)]
    fail_next_frame: bool,
}

impl SpectralChannel {
    fn new(
        model: &mut Option<Model>,
        transform: &Transform,
        quality: Quality,
    ) -> Result<Self, String> {
        let fft = transform.window.len();
        Ok(Self {
            processor: match model {
                Some(model) => Processor::Neural(model.tensors()?),
                None => Processor::Wpe(OnlineWpe::with_hop(
                    fft / 2 + 1,
                    fft / 2,
                    transform.room_seconds,
                )),
            },
            input: vec![0.0; fft],
            output: vec![0.0; fft],
            spectrum: vec![Complex32::default(); fft],
            analysis: vec![Complex64::default(); fft],
            analysis_scratch: vec![
                Complex64::default();
                transform.forward.get_inplace_scratch_len()
            ],
            scratch: vec![Complex32::default(); transform.inverse.get_inplace_scratch_len()],
            history: [[Complex32::default(); BINS]; MAX_MODEL_DELAY],
            delay_frames: if model.is_some() {
                quality.delay_frames()
            } else {
                0
            },
            position: 0,
            hop: 0,
            history_position: 0,
            recovery: 0,
            failed: false,
            tracker: LevelTracker::new(fft as f64 / (2.0 * MODEL_RATE)),
            levels: [FrameLevel::default(); MAX_MODEL_DELAY],
            activity: (FrameLevel::default(), 1.0),
            #[cfg(test)]
            fail_next_frame: false,
        })
    }
    fn reset(&mut self, model: &Option<Model>) {
        self.input.fill(0.0);
        self.output.fill(0.0);
        self.spectrum.fill(Complex32::default());
        self.analysis.fill(Complex64::default());
        self.history.fill([Complex32::default(); BINS]);
        self.position = 0;
        self.hop = 0;
        self.history_position = 0;
        self.recovery = 0;
        self.failed = false;
        self.tracker.reset();
        self.levels.fill(FrameLevel::default());
        self.activity = (FrameLevel::default(), 1.0);
        match (&mut self.processor, model) {
            (Processor::Neural(tensors), Some(model)) => model.reset(tensors),
            (Processor::Wpe(wpe), _) => wpe.reset(),
            _ => unreachable!(),
        }
    }
    fn tick(
        &mut self,
        input: f32,
        strength: f32,
        weighting: Weighting,
        model: &mut Option<Model>,
        transform: &Transform,
    ) -> f32 {
        let fft = transform.window.len();
        let output = self.output[self.position];
        self.output[self.position] = 0.0;
        self.input[self.position] = input;
        self.position = (self.position + 1) % fft;
        self.hop += 1;
        if self.hop == fft / 2 {
            self.hop = 0;
            self.frame(strength, weighting, model, transform);
        }
        output
    }
    fn frame(
        &mut self,
        strength: f32,
        weighting: Weighting,
        model: &mut Option<Model>,
        transform: &Transform,
    ) {
        let fft = transform.window.len();
        let bins = fft / 2 + 1;
        let power = self.input.iter().map(|&x| (x as f64).powi(2)).sum::<f64>() / fft as f64;
        let level = self.tracker.update(power);
        let normalize_by = if level.restore > 0.0 {
            level.restore
        } else {
            1.0
        };
        // Normalize before a double-precision analysis transform. Single-
        // precision FFT roundoff in near-empty bins is otherwise amplified by
        // the model's log features and can depend on the original input gain.
        for (i, value) in self.analysis.iter_mut().enumerate() {
            *value = Complex64::new(
                self.input[(self.position + i) % fft] as f64 * transform.window[i] as f64
                    / normalize_by,
                0.0,
            );
        }
        transform
            .forward
            .process_with_scratch(&mut self.analysis, &mut self.analysis_scratch);
        for (original, normalized) in self.spectrum.iter_mut().zip(&self.analysis) {
            *original = Complex32::new(
                (normalized.re * normalize_by) as f32,
                (normalized.im * normalize_by) as f32,
            );
        }
        let valid = match (&mut self.processor, model) {
            (Processor::Neural(tensors), Some(model)) => {
                for (value, complex) in tensors
                    .spectrum
                    .extract_tensor_mut()
                    .1
                    .as_chunks_mut::<2>()
                    .0
                    .iter_mut()
                    .zip(&self.analysis)
                {
                    value[0] = complex.re as f32;
                    value[1] = complex.im as f32;
                }
                let valid = model.run(tensors);
                #[cfg(test)]
                let valid = valid && !std::mem::take(&mut self.fail_next_frame);
                if !valid {
                    model.reset(tensors);
                }
                valid
            }
            (Processor::Wpe(wpe), _) => wpe.process(&mut self.analysis[..bins]),
            _ => unreachable!(),
        };
        self.failed = !valid || self.recovery > 0;
        if !valid {
            self.recovery = self.delay_frames;
        }
        let aligned_level = if self.delay_frames == 0 {
            level
        } else {
            let previous = self.levels[self.history_position];
            self.levels[self.history_position] = level;
            previous
        };
        let weight = weighting.amount(aligned_level);
        self.activity = (aligned_level, weight);
        for bin in 0..bins {
            let original = if self.delay_frames == 0 {
                self.spectrum[bin]
            } else {
                self.history[self.history_position][bin]
            };
            let wet = if self.failed {
                original
            } else {
                let enhanced = match &self.processor {
                    Processor::Neural(tensors) => {
                        let output = tensors.enhanced.extract_tensor().1;
                        Complex64::new(output[bin * 2] as f64, output[bin * 2 + 1] as f64)
                    }
                    Processor::Wpe(_) => self.analysis[bin],
                };
                Complex32::new(
                    (enhanced.re * aligned_level.restore) as f32,
                    (enhanced.im * aligned_level.restore) as f32,
                )
            };
            let wet = match &self.processor {
                Processor::Wpe(_) if !self.failed => suppress_wpe_tail(original, wet),
                _ => wet,
            };
            let real = bin == 0 || bin == bins - 1;
            let mut result = spectral_strength(original, wet, strength * weight, real);
            if real {
                result.im = 0.0;
            }
            self.history[self.history_position][bin] = self.spectrum[bin];
            self.spectrum[bin] = result;
        }
        if valid && self.recovery > 0 {
            self.recovery -= 1;
        }
        if self.delay_frames > 0 {
            self.history_position = (self.history_position + 1) % self.delay_frames;
        }
        for bin in bins..fft {
            self.spectrum[bin] = self.spectrum[fft - bin].conj();
        }
        transform
            .inverse
            .process_with_scratch(&mut self.spectrum, &mut self.scratch);
        for (i, sample) in self.spectrum.iter().enumerate() {
            self.output[(self.position + i) % fft] += sample.re * transform.window[i] / fft as f32;
        }
    }
}

pub(crate) fn suppress_wpe_tail(original: Complex32, residual: Complex32) -> Complex32 {
    // A prediction learned from earlier frames can exceed the current signal
    // as speech decays. Bound the residual by the estimated direct-sound power
    // so subtracting a stale prediction cannot synthesize a new tail.
    let original = Complex64::new(original.re.into(), original.im.into());
    let residual64 = Complex64::new(residual.re.into(), residual.im.into());
    let observed_power = original.norm_sqr();
    let late_power = (original - residual64).norm_sqr();
    let direct_power = (observed_power - late_power).max(0.04 * observed_power);
    let residual_power = residual64.norm_sqr();
    if residual_power > direct_power {
        residual * (direct_power / residual_power).sqrt() as f32
    } else {
        residual
    }
}

struct Conversion {
    input: Resampler,
    output: Resampler,
    queue: OutputQueue,
}
enum Backend {
    Spectral {
        model: Option<Model>,
        transform: Transform,
    },
    Dtln(dtln::Model),
}

enum ChannelProcessor {
    Spectral(Box<SpectralChannel>),
    Dtln {
        denoiser: Box<dtln::Channel>,
        mix: dtln::Mix,
    },
}

impl ChannelProcessor {
    fn tick(
        &mut self,
        input: f32,
        strength: f32,
        weighting: Weighting,
        backend: &mut Backend,
    ) -> f32 {
        match (self, backend) {
            (Self::Spectral(channel), Backend::Spectral { model, transform }) => {
                channel.tick(input, strength, weighting, model, transform)
            }
            (Self::Dtln { denoiser, .. }, Backend::Dtln(model)) => denoiser.tick(input, model),
            _ => unreachable!(),
        }
    }

    fn reset(&mut self, backend: &Backend) {
        match (self, backend) {
            (Self::Spectral(channel), Backend::Spectral { model, .. }) => channel.reset(model),
            (Self::Dtln { denoiser, mix }, Backend::Dtln(_)) => {
                denoiser.reset();
                mix.reset();
            }
            _ => unreachable!(),
        }
    }

    fn recovering(&self) -> bool {
        match self {
            Self::Spectral(channel) => channel.failed,
            Self::Dtln { denoiser, .. } => denoiser.recovering(),
        }
    }

    fn activity(&self) -> (FrameLevel, f32) {
        match self {
            Self::Spectral(channel) => channel.activity,
            Self::Dtln { mix, .. } => mix.activity(),
        }
    }
}

struct Channel {
    processor: ChannelProcessor,
    conversion: Option<Conversion>,
    padding: DelayLine,
    #[cfg(test)]
    meter_delay: DelayLine,
}

#[derive(Default)]
pub(crate) struct DelayLine {
    samples: Vec<f32>,
    position: usize,
}

impl DelayLine {
    pub fn new(delay: usize) -> Self {
        Self {
            samples: vec![0.0; delay],
            position: 0,
        }
    }

    pub fn reset(&mut self) {
        self.samples.fill(0.0);
        self.position = 0;
    }

    pub fn tick(&mut self, input: f32) -> f32 {
        if self.samples.is_empty() {
            return input;
        }
        let output = std::mem::replace(&mut self.samples[self.position], input);
        self.position = (self.position + 1) % self.samples.len();
        output
    }
}

pub struct Engine {
    backend: Backend,
    channels: Vec<Channel>,
    latency: usize,
    weighting: Weighting,
}

impl Engine {
    #[cfg(test)]
    pub fn new(sample_rate: f64, channels: usize, quality: Quality) -> Result<Self, String> {
        Self::prepare(sample_rate, channels, EngineKind::DpdfNet, quality)
    }
    pub fn prepare(
        sample_rate: f64,
        channels: usize,
        kind: EngineKind,
        quality: Quality,
    ) -> Result<Self, String> {
        let (fft, room_seconds) = match (kind, quality) {
            (EngineKind::NaraWpe, Quality::Live) => (480, 0.5),
            _ => (FFT, 0.0),
        };
        Self::prepare_config(sample_rate, channels, kind, quality, fft, room_seconds)
    }
    #[cfg(test)]
    pub fn wpe_window(
        sample_rate: f64,
        channels: usize,
        fft: usize,
        room_seconds: f64,
    ) -> Result<Self, String> {
        Self::prepare_config(
            sample_rate,
            channels,
            EngineKind::NaraWpe,
            Quality::Live,
            fft,
            room_seconds,
        )
    }
    fn prepare_config(
        sample_rate: f64,
        channels: usize,
        kind: EngineKind,
        quality: Quality,
        fft: usize,
        room_seconds: f64,
    ) -> Result<Self, String> {
        if !sample_rate.is_finite()
            || !(8000.0..=384000.0).contains(&sample_rate)
            || !(1..=16).contains(&channels)
        {
            return Err("Supported rates: 8–384 kHz; supported channel counts: 1–16".into());
        }
        assert!((120..=FFT).contains(&fft) && fft.is_multiple_of(2));
        let (mut backend, model_rate, model_latency) = match kind {
            EngineKind::Dtln => (
                Backend::Dtln(dtln::Model::new()?),
                dtln::RATE,
                dtln::LATENCY,
            ),
            EngineKind::DpdfNet | EngineKind::NaraWpe => {
                let model = if kind == EngineKind::DpdfNet {
                    Some(Model::new(quality)?)
                } else {
                    None
                };
                let latency = if model.is_some() {
                    quality.model_latency()
                } else {
                    fft
                };
                let transform = Transform {
                    forward: FftPlanner::<f64>::new().plan_fft_forward(fft),
                    inverse: FftPlanner::<f32>::new().plan_fft_inverse(fft),
                    window: (0..fft)
                        .map(|i| {
                            let s = (PI * (i as f32 + 0.5) / fft as f32).sin();
                            (0.5 * PI * s * s).sin()
                        })
                        .collect(),
                    room_seconds,
                };
                (Backend::Spectral { model, transform }, MODEL_RATE, latency)
            }
        };
        let ratio = sample_rate / model_rate;
        let conversion = if sample_rate == model_rate {
            None
        } else {
            let input_delay = Plan::delay(sample_rate, model_rate);
            let output_delay = Plan::delay(model_rate, sample_rate);
            let base = input_delay as f64 + (model_latency + output_delay) as f64 * ratio;
            let padding = ratio.ceil() as usize + 2;
            // An integer host latency needs a fractional model-rate correction.
            let extra_delay = (base.ceil() - base) / ratio;
            Some((
                Plan::new(sample_rate, model_rate, 0.0),
                Plan::new(model_rate, sample_rate, extra_delay),
                padding,
                base.ceil() as usize + padding,
            ))
        };
        let latency = conversion.as_ref().map_or(model_latency, |c| c.3);
        let channels = (0..channels)
            .map(|_| {
                Ok(Channel {
                    processor: match &mut backend {
                        Backend::Spectral { model, transform } => ChannelProcessor::Spectral(
                            Box::new(SpectralChannel::new(model, transform, quality)?),
                        ),
                        Backend::Dtln(model) => ChannelProcessor::Dtln {
                            denoiser: Box::new(model.channel()?),
                            mix: dtln::Mix::new(sample_rate),
                        },
                    },
                    conversion: conversion
                        .as_ref()
                        .map(|(input, output, padding, _)| Conversion {
                            input: Resampler::new(input.clone()),
                            output: Resampler::new(output.clone()),
                            queue: OutputQueue::new(*padding),
                        }),
                    padding: DelayLine::default(),
                    #[cfg(test)]
                    meter_delay: DelayLine::new(latency),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            backend,
            channels,
            latency,
            weighting: Weighting::default(),
        })
    }
    pub fn latency(&self) -> u32 {
        self.latency as u32
    }
    pub fn pad_to_latency(&mut self, latency: u32) {
        for channel in &mut self.channels {
            channel.padding = DelayLine::new(latency as usize - self.latency);
            #[cfg(test)]
            {
                channel.meter_delay = DelayLine::new(latency as usize);
            }
        }
        self.latency = latency as usize;
    }
    pub fn recovering(&self) -> bool {
        self.channels.iter().any(|c| c.processor.recovering())
    }
    pub fn set_weighting(&mut self, weighting: Weighting) {
        self.weighting = weighting;
    }
    /// Show the loudest channel's actual detector and applied curve weight.
    pub fn activity(&self) -> (usize, FrameLevel, f32) {
        self.channels
            .iter()
            .enumerate()
            .map(|(index, c)| {
                let (level, weight) = c.processor.activity();
                (index, level, weight)
            })
            .max_by(|a, b| a.1.absolute_db.total_cmp(&b.1.absolute_db))
            .unwrap_or_default()
    }
    pub fn reset(&mut self) {
        for channel in &mut self.channels {
            channel.processor.reset(&self.backend);
            channel.padding.reset();
            #[cfg(test)]
            channel.meter_delay.reset();
            if let Some(c) = &mut channel.conversion {
                c.input.reset();
                c.output.reset();
                c.queue.reset();
            }
        }
    }
    /// Returns (latency-aligned input, output), both at the host rate.
    #[cfg(test)]
    pub fn tick(&mut self, channel: usize, input: f32, strength: f32) -> (f32, f32) {
        let input = if input.is_finite() { input } else { 0.0 };
        let aligned = self.channels[channel].meter_delay.tick(input);
        (
            aligned,
            self.process_sample(channel, input, aligned, strength),
        )
    }

    pub fn process_sample(&mut self, channel: usize, input: f32, dry: f32, strength: f32) -> f32 {
        let channel = &mut self.channels[channel];
        let output = if let Some(conversion) = &mut channel.conversion {
            conversion.input.push(input);
            while let Some(sample) = conversion.input.pop() {
                conversion.output.push(channel.processor.tick(
                    sample,
                    strength,
                    self.weighting,
                    &mut self.backend,
                ));
                while let Some(sample) = conversion.output.pop() {
                    conversion.queue.push(sample);
                }
            }
            conversion.queue.pop()
        } else {
            channel
                .processor
                .tick(input, strength, self.weighting, &mut self.backend)
        };
        let wet = channel
            .padding
            .tick(if output.is_finite() { output } else { 0.0 });
        match &mut channel.processor {
            ChannelProcessor::Dtln { mix, .. } => mix.tick(dry, wet, strength, self.weighting),
            ChannelProcessor::Spectral(_) => wet,
        }
    }
    #[cfg(test)]
    pub fn fail_next_frame(&mut self, channel: usize) {
        let ChannelProcessor::Spectral(spectral) = &mut self.channels[channel].processor else {
            unreachable!()
        };
        spectral.fail_next_frame = true;
    }
}
