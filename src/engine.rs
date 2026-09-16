use crate::{
    band::Band,
    dsp::{gain_db, BandRuntime, CompSettings, VocalComp},
    processing::{Config, Delay, EqPath, ProcessingMode},
};
use atomic_float::AtomicF32;
use crossbeam_queue::ArrayQueue;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

pub struct Bank {
    pub bands: Vec<BandRuntime>,
    pub sr: f64,
    config: Config,
    path: EqPath,
    dry: Delay,
}
impl Bank {
    fn new(snapshot: &[Band], sr: f64, config: Config) -> Self {
        Self {
            bands: snapshot
                .iter()
                .cloned()
                .map(|b| BandRuntime::new(b, config.mode.rate(sr)))
                .collect(),
            sr,
            config,
            path: EqPath::new(snapshot, sr, config),
            dry: Delay::new(config.latency(sr)),
        }
    }
    fn tick(&mut self, input: [f64; 2]) -> ([f64; 2], [f64; 2]) {
        (
            self.path.tick(input, &mut self.bands, self.sr),
            self.dry.tick(input),
        )
    }
    fn reset(&mut self) {
        for b in &mut self.bands {
            b.reset();
        }
        self.path.reset();
        self.dry.reset();
    }
}
pub struct Shared {
    bands: Arc<Mutex<Vec<Band>>>,
    pub requested_config: AtomicU32,
    pub active_config: AtomicU32,
    pub latency: AtomicU32,
    pub input: AtomicF32,
    pub output: AtomicF32,
    pub gr: AtomicF32,
    pub sample_rate: AtomicF32,
    pub spectrum: [AtomicF32; 128],
    pub selected_id: std::sync::atomic::AtomicU64,
    pub solo_id: std::sync::atomic::AtomicU64,
    pub band_gr: AtomicF32,
    pub pending: ArrayQueue<Box<Bank>>,
    pub retired: ArrayQueue<Box<Bank>>,
    stop: Arc<AtomicBool>,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}
impl Shared {
    pub fn new(bands: Arc<Mutex<Vec<Band>>>) -> Arc<Self> {
        let shared = Arc::new(Self {
            bands: bands.clone(),
            requested_config: AtomicU32::new(Config::default().encode()),
            active_config: AtomicU32::new(Config::default().encode()),
            latency: AtomicU32::new(0),
            input: AtomicF32::new(-90.0),
            output: AtomicF32::new(-90.0),
            gr: AtomicF32::new(0.0),
            sample_rate: AtomicF32::new(44100.0),
            spectrum: std::array::from_fn(|_| AtomicF32::new(-90.0)),
            selected_id: std::sync::atomic::AtomicU64::new(0),
            solo_id: std::sync::atomic::AtomicU64::new(0),
            band_gr: AtomicF32::new(0.0),
            pending: ArrayQueue::new(1),
            retired: ArrayQueue::new(2),
            stop: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
        });
        let weak = Arc::downgrade(&shared);
        let stop = shared.stop.clone();
        let handle = std::thread::spawn(move || {
            let mut last: Option<(Vec<Band>, f64, Config)> = None;
            while !stop.load(Ordering::Relaxed) {
                if let Some(s) = weak.upgrade() {
                    while s.retired.pop().is_some() {}
                    if !s.pending.is_full() {
                        let sr = s.sample_rate.load(Ordering::Relaxed) as f64;
                        let config = Config::decode(s.requested_config.load(Ordering::Relaxed));
                        let snapshot = bands.lock().unwrap_or_else(|e| e.into_inner()).clone();
                        if last.as_ref().is_none_or(|(old, rate, old_config)| {
                            *old != snapshot || *rate != sr || *old_config != config
                        }) {
                            let bank = Box::new(Bank::new(&snapshot, sr, config));
                            if s.pending.push(bank).is_ok() {
                                last = Some((snapshot, sr, config));
                            }
                        }
                    }
                } else {
                    break;
                }
                std::thread::sleep(Duration::from_millis(8));
            }
        });
        *shared.worker.lock().unwrap() = Some(handle);
        shared
    }
}
impl Drop for Shared {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker.get_mut().unwrap().take() {
            if handle.thread().id() != std::thread::current().id() {
                let _ = handle.join();
            }
        }
    }
}
pub struct Engine {
    shared: Arc<Shared>,
    bank: Box<Bank>,
    previous: Option<Box<Bank>>,
    transition: usize,
    sr: f64,
    comp: VocalComp,
    fft: Arc<dyn Fft<f32>>,
    samples: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    window: [f32; 2048],
    position: usize,
    in_peak: f64,
    out_peak: f64,
    eq_mix: f64,
    comp_mix: f64,
    bypass_mix: f64,
    solo_filters: [crate::dsp::Cascade; 2],
    solo_mix: f64,
    solo_topology: Option<(u64, crate::band::Shape, u8)>,
}
impl Engine {
    pub fn new(shared: Arc<Shared>, sr: f64) -> Self {
        shared.sample_rate.store(sr as f32, Ordering::Relaxed);
        let fft = FftPlanner::new().plan_fft_forward(2048);
        let scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
        let config = Config::decode(shared.requested_config.load(Ordering::Relaxed));
        let bank = Box::new(Bank::new(
            &shared.bands.lock().unwrap_or_else(|e| e.into_inner()),
            sr,
            config,
        ));
        shared
            .active_config
            .store(config.encode(), Ordering::Relaxed);
        shared
            .latency
            .store(config.latency(sr) as u32, Ordering::Relaxed);
        Self {
            shared,
            bank,
            previous: None,
            transition: 0,
            sr,
            comp: VocalComp::new(),
            fft,
            samples: vec![Complex::default(); 2048],
            scratch,
            window: std::array::from_fn(|i| {
                0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / 2048.0).cos()
            }),
            position: 0,
            in_peak: 0.0,
            out_peak: 0.0,
            eq_mix: 1.0,
            comp_mix: 1.0,
            bypass_mix: 0.0,
            solo_filters: [crate::dsp::Cascade::default(); 2],
            solo_mix: 0.0,
            solo_topology: None,
        }
    }
    pub fn reset(&mut self) {
        self.bank.reset();
        if let Some(previous) = &mut self.previous {
            previous.reset();
        }
        self.transition = 0;
        self.comp = VocalComp::new();
        self.position = 0;
        self.samples.fill(Complex::default());
        self.in_peak = 0.0;
        self.out_peak = 0.0;
        self.solo_filters = [crate::dsp::Cascade::default(); 2];
        self.solo_mix = 0.0;
        self.solo_topology = None;
    }
    pub fn latency(&self) -> u32 {
        self.bank.config.latency(self.sr) as u32
    }
    pub fn sync(&mut self) {
        if self.transition == 0 && !self.shared.retired.is_full() {
            if let Some(old) = self.previous.take() {
                let result = self.shared.retired.push(old);
                debug_assert!(result.is_ok());
            }
        }
        if self.previous.is_none() && !self.shared.retired.is_full() {
            if let Some(mut next) = self.shared.pending.pop() {
                if next.sr == self.sr
                    && next.config.encode() == self.shared.requested_config.load(Ordering::Relaxed)
                {
                    let mut j = 0;
                    for b in &mut next.bands {
                        while j < self.bank.bands.len() && self.bank.bands[j].band.id < b.band.id {
                            j += 1;
                        }
                        if let Some(old) = self.bank.bands.get(j) {
                            if old.band.id == b.band.id && next.config == self.bank.config {
                                b.inherit(old);
                            }
                        }
                    }
                    std::mem::swap(&mut self.bank, &mut next);
                    self.previous = Some(next);
                    // Warm the new FIR / resampler before crossfading an EQ edit.
                    self.transition =
                        (self.sr * 0.010) as usize + self.bank.config.latency(self.sr) * 2;
                    self.shared
                        .active_config
                        .store(self.bank.config.encode(), Ordering::Relaxed);
                    self.shared.latency.store(self.latency(), Ordering::Relaxed);
                } else {
                    let result = self.shared.retired.push(next);
                    debug_assert!(result.is_ok());
                }
            }
        }
    }
    pub fn tick(
        &mut self,
        input: [f64; 2],
        settings: CompSettings,
        eq_on: bool,
        comp_on: bool,
        bypass: bool,
    ) -> [f64; 2] {
        let k = 1.0 - (-1.0 / (self.sr * 0.010)).exp();
        self.eq_mix += k * (f64::from(eq_on) - self.eq_mix);
        self.comp_mix += k * (f64::from(comp_on) - self.comp_mix);
        self.bypass_mix += k * (f64::from(bypass) - self.bypass_mix);
        let (mut x, mut dry) = self.bank.tick(input);
        let selected = self.shared.selected_id.load(Ordering::Relaxed);
        let band_gr = if self.bank.config.mode == ProcessingMode::LinearPhase {
            0.0
        } else {
            self.bank
                .bands
                .iter()
                .find(|b| b.band.id == selected)
                .map_or(0.0, |b| b.reduction)
        };
        if self.transition > 0 {
            if let Some(previous) = &mut self.previous {
                let (old, old_dry) = previous.tick(input);
                if previous.config == self.bank.config {
                    dry = old_dry;
                }
                let wet =
                    (1.0 - self.transition as f64 / (self.sr * 0.010).floor()).clamp(0.0, 1.0);
                // Different latencies cannot be crossfaded coherently. Use the
                // new aligned dry path during a mode change's warm-up instead.
                let source = if previous.config == self.bank.config {
                    old
                } else {
                    dry
                };
                for i in 0..2 {
                    x[i] = source[i] + wet * (x[i] - source[i]);
                }
            }
            self.transition -= 1;
        }
        for i in 0..2 {
            x[i] = dry[i] + self.eq_mix * (x[i] - dry[i]);
        }
        let solo_id = self.shared.solo_id.load(Ordering::Relaxed);
        let is_solo = solo_id != 0;
        let k_solo = 1.0 - (-1.0 / (self.sr * 0.008)).exp();
        self.solo_mix += k_solo * (f64::from(is_solo) - self.solo_mix);

        let solo_coeff = if is_solo {
            self.bank
                .bands
                .iter()
                .find(|b| b.band.id == solo_id)
                .map(|b| {
                    let mut audition = b.band.clone();
                    if !audition.shape.is_cut() {
                        audition.shape = crate::band::Shape::BandPass;
                        audition.q = audition.q.max(0.35);
                    }
                    let topology = (audition.id, audition.shape, audition.order);
                    if self.solo_topology != Some(topology) {
                        self.solo_filters = [crate::dsp::Cascade::default(); 2];
                        self.solo_topology = Some(topology);
                    }
                    crate::dsp::BandCoeffs::make(&audition, self.sr)
                })
        } else {
            None
        };

        let solo_sample = solo_coeff.map(|c| {
            [
                self.solo_filters[0].tick(dry[0], c) * 1.5,
                self.solo_filters[1].tick(dry[1], c) * 1.5,
            ]
        });

        let (compressed, gr) = self.comp.tick(x, settings, self.sr);
        let effective_comp_mix = self.comp_mix * (1.0 - self.solo_mix);
        for i in 0..2 {
            let processed = x[i] + effective_comp_mix * (compressed[i] - x[i]);
            x[i] = processed + self.bypass_mix * (dry[i] - processed);
            if let Some(s) = solo_sample {
                x[i] = (1.0 - self.solo_mix) * x[i] + self.solo_mix * s[i];
            }
            if !x[i].is_finite() {
                x[i] = 0.0;
            }
        }
        if !is_solo && self.solo_mix < 0.0001 {
            self.solo_filters = [crate::dsp::Cascade::default(); 2];
        }
        self.in_peak = self.in_peak.max(input[0].abs()).max(input[1].abs());
        self.out_peak = self.out_peak.max(x[0].abs()).max(x[1].abs());
        // Input spectrum. Pick the louder channel to avoid mono cancellation.
        let sample = if input[0].abs() > input[1].abs() {
            input[0]
        } else {
            input[1]
        };
        self.samples[self.position] = Complex::new(sample as f32 * self.window[self.position], 0.0);
        self.position += 1;
        if self.position == 2048 {
            self.position = 0;
            self.fft
                .process_with_scratch(&mut self.samples, &mut self.scratch);
            for i in 0..128 {
                let lo = 20.0_f64 * 1000.0_f64.powf(i as f64 / 128.0);
                let hi = 20.0_f64 * 1000.0_f64.powf((i + 1) as f64 / 128.0);
                let start = (lo / self.sr * 2048.0).floor() as usize;
                let end = ((hi / self.sr * 2048.0).ceil() as usize).min(1023);
                let mut mag = 0.0_f32;
                if start <= end {
                    for bin in start.max(1)..=end {
                        mag = mag.max(self.samples[bin].norm() / 512.0);
                    }
                }
                let db = (20.0 * mag.max(0.0000316).log10()).max(-90.0);
                let old = self.shared.spectrum[i].load(Ordering::Relaxed);
                self.shared.spectrum[i].store(db.max(old - 3.0), Ordering::Relaxed);
            }
            self.shared
                .input
                .store(gain_db(self.in_peak) as f32, Ordering::Relaxed);
            self.shared
                .output
                .store(gain_db(self.out_peak) as f32, Ordering::Relaxed);
            self.shared.gr.store(
                (gr * self.comp_mix * (1.0 - self.bypass_mix)) as f32,
                Ordering::Relaxed,
            );
            self.shared.band_gr.store(band_gr as f32, Ordering::Relaxed);
            self.in_peak = 0.0;
            self.out_peak = 0.0;
        }
        x
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn isolated_shared() -> Arc<Shared> {
        let shared = Shared::new(Arc::new(Mutex::new(vec![])));
        shared.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = shared.worker.lock().unwrap().take() {
            worker.join().unwrap();
        }
        while shared.pending.pop().is_some() {}
        shared
    }
    #[test]
    fn bypass_paths_stay_latency_aligned_during_band_updates() {
        use crate::processing::{Resolution, MODES};
        for mode in MODES {
            for global_bypass in [false, true] {
                let shared = isolated_shared();
                let config = Config {
                    mode,
                    resolution: Resolution::Low,
                };
                shared
                    .requested_config
                    .store(config.encode(), Ordering::Relaxed);
                let mut engine = Engine::new(shared.clone(), 44100.0);
                engine.eq_mix = if global_bypass { 1.0 } else { 0.0 };
                engine.bypass_mix = if global_bypass { 1.0 } else { 0.0 };
                let latency = engine.latency() as usize;
                let settings = CompSettings::default();
                for i in 0..20000 {
                    if i == 8000 {
                        let next = Box::new(Bank::new(
                            &[Band {
                                gain: 18.0,
                                ..Band::default()
                            }],
                            44100.0,
                            config,
                        ));
                        assert!(shared.pending.push(next).is_ok());
                        engine.sync();
                    }
                    let x = (i as f64 * 0.1).sin() * 0.3;
                    let y =
                        engine.tick([x, -x], settings, global_bypass, false, global_bypass);
                    let expected = if i >= latency {
                        ((i - latency) as f64 * 0.1).sin() * 0.3
                    } else {
                        0.0
                    };
                    assert!(
                        (y[0] - expected).abs() < 1e-9,
                        "{mode:?} at {i}: {} != {expected}",
                        y[0]
                    );
                    assert!((y[1] + expected).abs() < 1e-9);
                }
            }
        }
    }
    #[test]
    fn mode_switch_publishes_actual_latency_and_rejects_stale_banks() {
        use crate::processing::Resolution;
        let shared = isolated_shared();
        let mut engine = Engine::new(shared.clone(), 48000.0);
        let config = Config {
            mode: ProcessingMode::LinearPhase,
            resolution: Resolution::Low,
        };
        shared
            .requested_config
            .store(config.encode(), Ordering::Relaxed);
        assert!(shared
            .pending
            .push(Box::new(Bank::new(&[], 48000.0, Config::default())))
            .is_ok());
        engine.sync();
        assert_eq!(engine.latency(), 0);
        while shared.retired.pop().is_some() {}
        assert!(shared
            .pending
            .push(Box::new(Bank::new(&[], 48000.0, config)))
            .is_ok());
        engine.sync();
        assert_eq!(engine.latency() as usize, config.latency(48000.0));
        assert_eq!(shared.latency.load(Ordering::Relaxed), engine.latency());
        assert_eq!(
            shared.active_config.load(Ordering::Relaxed),
            config.encode()
        );
        engine.reset();
        for _ in 0..10000 {
            assert_eq!(
                engine.tick([0.0; 2], CompSettings::default(), true, true, false),
                [0.0; 2]
            );
        }
    }
    #[test]
    fn unlimited_band_state_and_neutral_audio() {
        let bands = Arc::new(Mutex::new(
            (1..=200)
                .map(|id| Band {
                    id,
                    ..Band::default()
                })
                .collect(),
        ));
        let shared = Shared::new(bands);
        let mut engine = Engine::new(shared, 48000.0);
        assert_eq!(engine.bank.bands.len(), 200);
        let settings = CompSettings {
            threshold: 0.0,
            gate: -80.0,
            hpf: 80.0,
            dry: 0.0,
            wet: 1.0,
            ..CompSettings::default()
        };
        for i in 0..4096 {
            let x = (i as f64 * 0.1).sin() * 0.5;
            let out = engine.tick([x, x], settings, true, true, false);
            assert!((out[0] - x).abs() < 1e-9);
        }
    }
    #[test]
    fn bypass_preserves_input_after_ramp() {
        let shared = Shared::new(Arc::new(Mutex::new(vec![Band {
            gain: 18.0,
            ..Band::default()
        }])));
        let mut engine = Engine::new(shared, 48000.0);
        let settings = CompSettings {
            threshold: -30.0,
            gate: -80.0,
            hpf: 80.0,
            dry: 0.0,
            wet: 1.0,
            ..CompSettings::default()
        };
        for i in 0..24000 {
            let x = (i as f64 * 0.13).sin() * 0.1;
            let out = engine.tick([x, x], settings, true, true, true);
            if i > 23000 {
                assert!((out[0] - x).abs() < 1e-10);
            }
        }
    }
    #[test]
    fn solo_audition_attenuates_distant_frequencies() {
        let band = Band {
            id: 1,
            freq: 1000.0,
            gain: 0.0,
            q: 2.0,
            ..Band::default()
        };
        let shared = Shared::new(Arc::new(Mutex::new(vec![band])));
        let mut engine = Engine::new(shared.clone(), 48000.0);
        let settings = CompSettings {
            threshold: 0.0,
            gate: -80.0,
            hpf: 80.0,
            dry: 0.0,
            wet: 1.0,
            ..CompSettings::default()
        };
        // Activate solo
        shared.solo_id.store(1, Ordering::Relaxed);
        // Feed distant frequency (e.g. 100 Hz vs 1000 Hz band)
        let mut distant_energy = 0.0;
        for i in 0..4800 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 100.0 * t).sin();
            let out = engine.tick([x, x], settings, true, true, false);
            if i > 2400 {
                distant_energy += out[0].powi(2);
            }
        }
        // At 100 Hz with 1000 Hz bandpass (Q=2), energy must be heavily attenuated (< 0.1)
        assert!(distant_energy / 2400.0 < 0.05);
    }
}
