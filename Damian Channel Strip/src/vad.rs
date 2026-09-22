//! Off-audio-thread Silero VAD sidechain.
//!
//! Audio thread: LPF + resample to 16 kHz, push into a lock-free ring, read
//! smoothed `speech_env`. Worker thread owns the ONNX session.

use std::cell::UnsafeCell;
use std::f64::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use atomic_float::AtomicF32;

/// Official Silero VAD ONNX (v6.2.1).
/// SHA-256: 1a153a22f4509e292a94e67d6f9b85e8deb25b4988682b7e174c65279d8788e3
const SILERO_ONNX: &[u8] = include_bytes!("../models/silero_vad.onnx");

pub const VAD_SAMPLE_RATE: f64 = 16_000.0;
const CHUNK_SAMPLES: usize = 512;
const CONTEXT_SAMPLES: usize = 64;
const STATE_LEN: usize = 2 * 128;
const RING_CAP: usize = 4096; // ~256 ms at 16 kHz
const LPF_HZ: f64 = 7500.0;
const ATTACK_SECS: f32 = 0.005;
const RELEASE_SECS: f32 = 0.100;

#[derive(Clone, Copy, Debug)]
pub struct VadBiquad {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
    s1: f64,
    s2: f64,
}

impl Default for VadBiquad {
    fn default() -> Self {
        Self::new()
    }
}

impl VadBiquad {
    pub const fn new() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            s1: 0.0,
            s2: 0.0,
        }
    }

    #[inline(always)]
    pub fn process_sample(&mut self, sample: f64) -> f64 {
        let out = self.b0 * sample + self.s1;
        self.s1 = self.b1 * sample - self.a1 * out + self.s2;
        self.s2 = self.b2 * sample - self.a2 * out;
        out
    }

    pub fn set_lowpass(&mut self, freq: f64, q: f64, sample_rate: f64) {
        let w0 = 2.0 * PI * freq / sample_rate;
        let alpha = w0.sin() / (2.0 * q.max(0.1));
        let cos_w0 = w0.cos();

        let a0_inv = 1.0 / (1.0 + alpha);
        let b_scale = ((1.0 - cos_w0) * 0.5) * a0_inv;
        self.b0 = b_scale;
        self.b1 = (1.0 - cos_w0) * a0_inv;
        self.b2 = b_scale;
        self.a1 = (-2.0 * cos_w0) * a0_inv;
        self.a2 = (1.0 - alpha) * a0_inv;
    }

    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }
}

struct VadShared {
    ring: Box<[UnsafeCell<f32>]>,
    write: AtomicUsize,
    read: AtomicUsize,
    raw_prob: AtomicF32,
    running: AtomicBool,
    reset: AtomicBool,
}

unsafe impl Send for VadShared {}
unsafe impl Sync for VadShared {}

impl VadShared {
    fn new() -> Self {
        Self {
            ring: (0..RING_CAP)
                .map(|_| UnsafeCell::new(0.0f32))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
            raw_prob: AtomicF32::new(0.0),
            running: AtomicBool::new(true),
            reset: AtomicBool::new(false),
        }
    }

    fn available(&self) -> usize {
        let w = self.write.load(Ordering::Acquire);
        let r = self.read.load(Ordering::Relaxed);
        w.wrapping_sub(r)
    }

    fn push(&self, sample: f32) {
        let w = self.write.load(Ordering::Relaxed);
        let r = self.read.load(Ordering::Acquire);
        if w.wrapping_sub(r) >= RING_CAP - 1 {
            // Drop oldest sample so the audio thread never blocks.
            self.read.store(r.wrapping_add(1), Ordering::Release);
        }
        let idx = w % RING_CAP;
        unsafe {
            *self.ring[idx].get() = sample;
        }
        self.write.store(w.wrapping_add(1), Ordering::Release);
    }

    fn pop_chunk(&self, dest: &mut [f32]) -> bool {
        if self.available() < dest.len() {
            return false;
        }
        let r = self.read.load(Ordering::Relaxed);
        for (i, slot) in dest.iter_mut().enumerate() {
            *slot = unsafe { *self.ring[(r.wrapping_add(i)) % RING_CAP].get() };
        }
        self.read
            .store(r.wrapping_add(dest.len()), Ordering::Release);
        true
    }
}

pub struct SpeechVad {
    sample_rate: f64,
    lpf: VadBiquad,
    frac: f64,
    prev: f32,
    speech_env: f32,
    inject: Option<f32>,
    shared: Option<Arc<VadShared>>,
    worker: Option<JoinHandle<()>>,
}

impl SpeechVad {
    pub fn new(sample_rate: f64) -> Self {
        let sr = sample_rate.max(8000.0);
        let mut lpf = VadBiquad::new();
        lpf.set_lowpass(LPF_HZ, std::f64::consts::FRAC_1_SQRT_2, sr);
        Self {
            sample_rate: sr,
            lpf,
            frac: 0.0,
            prev: 0.0,
            speech_env: 0.0,
            inject: None,
            shared: None,
            worker: None,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        let sr = sample_rate.max(8000.0);
        if (self.sample_rate - sr).abs() > 0.1 {
            self.sample_rate = sr;
            self.lpf
                .set_lowpass(LPF_HZ, std::f64::consts::FRAC_1_SQRT_2, sr);
            self.lpf.reset();
            self.frac = 0.0;
            self.prev = 0.0;
            if let Some(shared) = &self.shared {
                shared.reset.store(true, Ordering::Release);
            }
        }
    }

    /// Spawn the ONNX worker. Safe to call once from `Plugin::initialize`.
    pub fn start_worker(&mut self) {
        if self.worker.is_some() {
            return;
        }
        let shared = Arc::new(VadShared::new());
        let worker_shared = Arc::clone(&shared);
        let handle = thread::Builder::new()
            .name("damian-silero-vad".into())
            .spawn(move || vad_worker_loop(worker_shared))
            .ok();
        if handle.is_some() {
            self.shared = Some(shared);
            self.worker = handle;
        }
    }

    /// Test hook: skip worker and force speech_env.
    pub fn inject_speech_env(&mut self, env: Option<f32>) {
        self.inject = env.map(|v| v.clamp(0.0, 1.0));
        if let Some(v) = self.inject {
            self.speech_env = v;
        }
    }

    #[inline(always)]
    pub fn speech_env(&self) -> f32 {
        self.speech_env
    }

    pub fn reset(&mut self) {
        self.lpf.reset();
        self.frac = 0.0;
        self.prev = 0.0;
        if self.inject.is_none() {
            self.speech_env = 0.0;
        }
        if let Some(shared) = &self.shared {
            shared.reset.store(true, Ordering::Release);
            shared.raw_prob.store(0.0, Ordering::Relaxed);
        }
    }

    #[inline]
    pub fn push_sample(&mut self, x: f32) {
        if self.inject.is_some() {
            return;
        }
        let Some(shared) = &self.shared else {
            return;
        };
        let step = VAD_SAMPLE_RATE / self.sample_rate;
        let y = self.lpf.process_sample(x as f64) as f32;
        self.frac += step;
        while self.frac >= 1.0 {
            self.frac -= 1.0;
            let t = (1.0 - self.frac) as f32;
            let out = self.prev + (y - self.prev) * t.clamp(0.0, 1.0);
            shared.push(out);
        }
        self.prev = y;
    }

    pub fn push_samples(&mut self, mono: &[f32]) {
        for &x in mono {
            self.push_sample(x);
        }
    }

    pub fn push_block(&mut self, mono: &[f32], block_secs: f32) {
        self.push_samples(mono);
        self.end_block(block_secs);
    }

    pub fn end_block(&mut self, block_secs: f32) {
        if let Some(v) = self.inject {
            self.speech_env = v;
            return;
        }
        if let Some(shared) = &self.shared {
            let raw = shared.raw_prob.load(Ordering::Relaxed).clamp(0.0, 1.0);
            self.speech_env = smooth_env(self.speech_env, raw, block_secs);
        } else {
            self.speech_env = 0.0;
        }
    }
}

impl Drop for SpeechVad {
    fn drop(&mut self) {
        if let Some(shared) = &self.shared {
            shared.running.store(false, Ordering::Release);
        }
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

fn smooth_env(current: f32, target: f32, block_secs: f32) -> f32 {
    let tau = if target > current {
        ATTACK_SECS
    } else {
        RELEASE_SECS
    };
    let alpha = 1.0 - (-block_secs / tau.max(1.0e-4)).exp();
    current + (target - current) * alpha.clamp(0.0, 1.0)
}

fn vad_worker_loop(shared: Arc<VadShared>) {
    let mut session = match build_session() {
        Ok(s) => s,
        Err(_) => {
            shared.running.store(false, Ordering::Release);
            return;
        }
    };

    let mut state = vec![0.0f32; STATE_LEN];
    let mut context = [0.0f32; CONTEXT_SAMPLES];
    let mut chunk = [0.0f32; CHUNK_SAMPLES];
    let mut input = vec![0.0f32; CONTEXT_SAMPLES + CHUNK_SAMPLES];

    while shared.running.load(Ordering::Relaxed) {
        if shared.reset.swap(false, Ordering::AcqRel) {
            state.fill(0.0);
            context.fill(0.0);
            shared.raw_prob.store(0.0, Ordering::Relaxed);
        }

        if !shared.pop_chunk(&mut chunk) {
            thread::sleep(Duration::from_millis(2));
            continue;
        }

        input[..CONTEXT_SAMPLES].copy_from_slice(&context);
        input[CONTEXT_SAMPLES..].copy_from_slice(&chunk);
        context.copy_from_slice(&input[CHUNK_SAMPLES..]);

        match run_chunk(&mut session, &input, &mut state) {
            Ok(prob) => shared
                .raw_prob
                .store(prob.clamp(0.0, 1.0), Ordering::Relaxed),
            Err(_) => shared.raw_prob.store(0.0, Ordering::Relaxed),
        }
    }
}

fn build_session() -> ort::Result<ort::session::Session> {
    use ort::session::builder::GraphOptimizationLevel;
    use ort::session::Session;

    Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(1)?
        .with_inter_threads(1)?
        .commit_from_memory(SILERO_ONNX)
}

fn run_chunk(
    session: &mut ort::session::Session,
    input: &[f32],
    state: &mut [f32],
) -> ort::Result<f32> {
    use ort::value::Tensor;

    let audio = Tensor::from_array(([1usize, input.len()], input.to_vec()))?;
    let state_t = Tensor::from_array(([2usize, 1, 128], state.to_vec()))?;
    let sr = Tensor::from_array(([1usize], vec![16_000i64]))?;

    let outputs = session.run(ort::inputs![
        "input" => audio,
        "state" => state_t,
        "sr" => sr,
    ])?;

    let prob = extract_f32(&outputs, "output")
        .or_else(|| extract_index(&outputs, 0))
        .unwrap_or(0.0);

    if let Some(next) = extract_state(&outputs) {
        if next.len() == state.len() {
            state.copy_from_slice(&next);
        }
    }

    Ok(prob)
}

fn extract_f32(outputs: &ort::session::SessionOutputs<'_>, name: &str) -> Option<f32> {
    let (_shape, data) = outputs.get(name)?.try_extract_tensor::<f32>().ok()?;
    data.first().copied()
}

fn extract_index(outputs: &ort::session::SessionOutputs<'_>, idx: usize) -> Option<f32> {
    if idx >= outputs.len() {
        return None;
    }
    let (_shape, data) = outputs[idx].try_extract_tensor::<f32>().ok()?;
    data.first().copied()
}

fn extract_state(outputs: &ort::session::SessionOutputs<'_>) -> Option<Vec<f32>> {
    for name in ["stateN", "state", "hn"] {
        if let Some(v) = outputs.get(name) {
            if let Ok((_shape, data)) = v.try_extract_tensor::<f32>() {
                return Some(data.to_vec());
            }
        }
    }
    if outputs.len() > 1 {
        if let Ok((_shape, data)) = outputs[1].try_extract_tensor::<f32>() {
            return Some(data.to_vec());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vad_biquad_lowpass_attenuates_high_frequencies() {
        let sr = 48000.0;
        let mut filter = VadBiquad::new();
        filter.set_lowpass(7500.0, std::f64::consts::FRAC_1_SQRT_2, sr);

        let mut peak_low = 0.0_f64;
        for i in 0..480 {
            let s = (2.0 * PI * 1000.0 * i as f64 / sr).sin();
            let y = filter.process_sample(s);
            if i > 200 {
                peak_low = peak_low.max(y.abs());
            }
        }
        assert!(peak_low > 0.9, "1 kHz should pass, got {peak_low}");

        filter.reset();
        let mut peak_high = 0.0_f64;
        for i in 0..480 {
            let s = (2.0 * PI * 18000.0 * i as f64 / sr).sin();
            let y = filter.process_sample(s);
            if i > 200 {
                peak_high = peak_high.max(y.abs());
            }
        }
        assert!(peak_high < 0.25, "18 kHz should be cut, got {peak_high}");
    }

    #[test]
    fn silero_onnx_inference_evaluates_silence_and_probe() {
        let mut vad = SpeechVad::new(16_000.0);
        vad.start_worker();

        let silence = vec![0.0f32; 16_000];
        vad.push_block(&silence, 1.0);
        std::thread::sleep(Duration::from_millis(250));
        vad.push_block(&silence, 0.032);
        std::thread::sleep(Duration::from_millis(100));
        assert!(
            vad.speech_env() < 0.35,
            "Silence should score low, got {}",
            vad.speech_env()
        );
    }
}
