use crate::{
    band::Band,
    dsp::{gain_db, BandRuntime, CompSettings, VocalComp},
    lift::{LiftBand, LiftProcessor, LIFT_ID_BASE, LIFT_LATENCY},
    processing::{Config, Delay, EqPath, ProcessingMode},
    vad::SpeechVad,
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
    pub eq2_bands: Vec<BandRuntime>,
    pub sr: f64,
    config: Config,
    path: EqPath,
    dry: Delay,
}
impl Bank {
    fn new(snapshot: &[Band], snapshot2: &[Band], sr: f64, config: Config) -> Self {
        let mut combined = Vec::with_capacity(snapshot.len() + snapshot2.len());
        combined.extend_from_slice(snapshot);
        combined.extend_from_slice(snapshot2);
        Self {
            bands: snapshot
                .iter()
                .cloned()
                .map(|b| BandRuntime::new(b, config.mode.rate(sr)))
                .collect(),
            eq2_bands: snapshot2
                .iter()
                .cloned()
                .map(|b| BandRuntime::new(b, config.mode.rate(sr)))
                .collect(),
            sr,
            config,
            path: EqPath::new(&combined, sr, config),
            dry: Delay::new(config.latency(sr)),
        }
    }
    fn tick(&mut self, input: [f64; 2], eq1_mix: f64, eq2_mix: f64) -> ([f64; 2], [f64; 2]) {
        let dry = self.dry.tick(input);
        let total_eq_mix = eq1_mix.max(eq2_mix);
        if total_eq_mix == 0.0 {
            (dry, dry)
        } else {
            (
                self.path.tick_dual(
                    input,
                    &mut self.bands,
                    &mut self.eq2_bands,
                    self.sr,
                    eq1_mix,
                    eq2_mix,
                ),
                dry,
            )
        }
    }
    fn reset(&mut self) {
        for b in &mut self.bands {
            b.reset();
        }
        for b in &mut self.eq2_bands {
            b.reset();
        }
        self.path.reset();
        self.dry.reset();
    }
}
pub struct Shared {
    bands: Arc<Mutex<Vec<Band>>>,
    pub eq2_bands: Arc<Mutex<Vec<Band>>>,
    pub sc_eq_bands: Arc<Mutex<Vec<Band>>>,
    pub lift_bands: Arc<Mutex<Vec<LiftBand>>>,
    pub requested_config: AtomicU32,
    pub active_config: AtomicU32,
    pub latency: AtomicU32,
    pub input: AtomicF32,
    pub output: AtomicF32,
    pub gr: AtomicF32,
    pub gr_uncapped: AtomicF32,
    pub pse_gr: AtomicF32,
    pub sc_level: AtomicF32,
    pub speech_env: AtomicF32,
    pub sample_rate: AtomicF32,
    pub spectrum: [AtomicF32; 128],
    pub lift_band_gr: [AtomicF32; 8],
    pub lift_bin_gr: [AtomicF32; 256],
    pub lift_bin_gr_uncapped: [AtomicF32; 256],
    pub selected_id: std::sync::atomic::AtomicU64,
    pub solo_id: std::sync::atomic::AtomicU64,
    pub band_gr: AtomicF32,
    pub dyn_gr_uncapped: Mutex<Vec<(u64, f32)>>,
    pub pending: ArrayQueue<Box<Bank>>,
    pub retired: ArrayQueue<Box<Bank>>,
    stop: Arc<AtomicBool>,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}
impl Shared {
    pub fn new(
        bands: Arc<Mutex<Vec<Band>>>,
        eq2_bands: Arc<Mutex<Vec<Band>>>,
        sc_eq_bands: Arc<Mutex<Vec<Band>>>,
        lift_bands: Arc<Mutex<Vec<LiftBand>>>,
    ) -> Arc<Self> {
        let shared = Arc::new(Self {
            bands: bands.clone(),
            eq2_bands: eq2_bands.clone(),
            sc_eq_bands: sc_eq_bands.clone(),
            lift_bands: lift_bands.clone(),
            requested_config: AtomicU32::new(Config::default().encode()),
            active_config: AtomicU32::new(Config::default().encode()),
            latency: AtomicU32::new(0),
            input: AtomicF32::new(-90.0),
            output: AtomicF32::new(-90.0),
            gr: AtomicF32::new(0.0),
            gr_uncapped: AtomicF32::new(0.0),
            pse_gr: AtomicF32::new(0.0),
            sc_level: AtomicF32::new(-90.0),
            speech_env: AtomicF32::new(0.0),
            sample_rate: AtomicF32::new(44100.0),
            spectrum: std::array::from_fn(|_| AtomicF32::new(-90.0)),
            lift_band_gr: std::array::from_fn(|_| AtomicF32::new(0.0)),
            lift_bin_gr: std::array::from_fn(|_| AtomicF32::new(0.0)),
            lift_bin_gr_uncapped: std::array::from_fn(|_| AtomicF32::new(0.0)),
            selected_id: std::sync::atomic::AtomicU64::new(0),
            solo_id: std::sync::atomic::AtomicU64::new(0),
            band_gr: AtomicF32::new(0.0),
            dyn_gr_uncapped: Mutex::new(Vec::new()),
            pending: ArrayQueue::new(1),
            retired: ArrayQueue::new(2),
            stop: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
        });
        let weak = Arc::downgrade(&shared);
        let stop = shared.stop.clone();
        let handle = std::thread::spawn(move || {
            let mut last: Option<(Vec<Band>, Vec<Band>, f64, Config)> = None;
            while !stop.load(Ordering::Relaxed) {
                if let Some(s) = weak.upgrade() {
                    while s.retired.pop().is_some() {}
                    if !s.pending.is_full() {
                        let sr = s.sample_rate.load(Ordering::Relaxed) as f64;
                        let config = Config::decode(s.requested_config.load(Ordering::Relaxed));
                        let snapshot = bands.lock().unwrap_or_else(|e| e.into_inner()).clone();
                        let snapshot2 = eq2_bands.lock().unwrap_or_else(|e| e.into_inner()).clone();
                        if last.as_ref().is_none_or(|(old1, old2, rate, old_config)| {
                            *old1 != snapshot
                                || *old2 != snapshot2
                                || *rate != sr
                                || *old_config != config
                        }) {
                            let bank = Box::new(Bank::new(&snapshot, &snapshot2, sr, config));
                            if s.pending.push(bank).is_ok() {
                                last = Some((snapshot, snapshot2, sr, config));
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
    eq1_mix: f64,
    eq2_mix: f64,
    comp_mix: f64,
    comp_pre_mix: f64,
    bypass_mix: f64,
    solo_filters: [crate::dsp::Cascade; 2],
    solo_mix: f64,
    solo_topology: Option<(u64, crate::band::Shape, u8)>,
    lift: LiftProcessor,
    lift_bands: Vec<LiftBand>,
    pub sc_eq_bands: Vec<BandRuntime>,
    sc_eq_mix: f64,
    lift_mix: f64,
    vad: SpeechVad,
    speech_env: f32,
    dyn_gr_scratch: Vec<(u64, f32)>,
}
impl Engine {
    pub fn new(shared: Arc<Shared>, sr: f64) -> Self {
        shared.sample_rate.store(sr as f32, Ordering::Relaxed);
        let fft = FftPlanner::new().plan_fft_forward(2048);
        let scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
        let config = Config::decode(shared.requested_config.load(Ordering::Relaxed));
        let bank = Box::new(Bank::new(
            &shared.bands.lock().unwrap_or_else(|e| e.into_inner()),
            &shared.eq2_bands.lock().unwrap_or_else(|e| e.into_inner()),
            sr,
            config,
        ));
        let lift_bands = shared
            .lift_bands
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let sc_eq_bands = shared
            .sc_eq_bands
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .into_iter()
            .map(|b| BandRuntime::new(b, sr))
            .collect();
        let lift_latency = if lift_bands.iter().any(|b| b.enabled) {
            LIFT_LATENCY as u32
        } else {
            0
        };
        shared
            .active_config
            .store(config.encode(), Ordering::Relaxed);
        shared.latency.store(
            (config.latency(sr) as u32) + lift_latency,
            Ordering::Relaxed,
        );
        let mut vad = SpeechVad::new(sr);
        vad.start_worker();
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
            eq1_mix: 1.0,
            eq2_mix: 1.0,
            comp_mix: 1.0,
            comp_pre_mix: 0.0,
            bypass_mix: 0.0,
            solo_filters: [crate::dsp::Cascade::default(); 2],
            solo_mix: 0.0,
            solo_topology: None,
            lift: LiftProcessor::new(),
            lift_bands,
            sc_eq_bands,
            sc_eq_mix: 1.0,
            lift_mix: 1.0,
            vad,
            speech_env: 0.0,
            dyn_gr_scratch: Vec::new(),
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
        self.lift.reset();
        for b in &mut self.sc_eq_bands {
            b.reset();
        }
        self.vad.reset();
        self.speech_env = 0.0;
        self.shared.speech_env.store(0.0, Ordering::Relaxed);
    }
    pub fn start_voice_detector(&mut self) {
        self.vad.start_worker();
    }
    pub fn speech_env(&self) -> f32 {
        self.speech_env
    }
    pub fn sample_rate(&self) -> f64 {
        self.sr
    }
    #[allow(dead_code)]
    pub fn inject_speech_env(&mut self, env: Option<f32>) {
        self.vad.inject_speech_env(env);
        if let Some(v) = env {
            self.speech_env = v;
            self.shared.speech_env.store(v, Ordering::Relaxed);
        }
    }
    pub fn end_block(&mut self, block_secs: f32) {
        self.vad.end_block(block_secs);
        self.speech_env = self.vad.speech_env();
        self.shared
            .speech_env
            .store(self.speech_env, Ordering::Relaxed);
    }
    pub fn latency(&self) -> u32 {
        let lift_latency = if self.lift_bands.iter().any(|b| b.enabled) {
            LIFT_LATENCY as u32
        } else {
            0
        };
        self.bank.config.latency(self.sr) as u32 + lift_latency
    }
    pub fn sync(&mut self) {
        if let Ok(guard) = self.shared.lift_bands.try_lock() {
            self.lift_bands = guard.clone();
            self.shared.latency.store(self.latency(), Ordering::Relaxed);
        }
        if let Ok(guard) = self.shared.sc_eq_bands.try_lock() {
            if self.sc_eq_bands.len() != guard.len()
                || self
                    .sc_eq_bands
                    .iter()
                    .zip(guard.iter())
                    .any(|(r, b)| r.band != *b)
            {
                let mut new_runtimes = Vec::with_capacity(guard.len());
                for b in guard.iter() {
                    let mut runtime = BandRuntime::new(b.clone(), self.sr);
                    if let Some(old) = self.sc_eq_bands.iter().find(|old| old.band.id == b.id) {
                        runtime.inherit(old);
                    }
                    new_runtimes.push(runtime);
                }
                self.sc_eq_bands = new_runtimes;
            }
        }
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
                    let mut j2 = 0;
                    for b in &mut next.eq2_bands {
                        while j2 < self.bank.eq2_bands.len()
                            && self.bank.eq2_bands[j2].band.id < b.band.id
                        {
                            j2 += 1;
                        }
                        if let Some(old) = self.bank.eq2_bands.get(j2) {
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
    fn sc_detector(&mut self, tap: [f64; 2]) -> [f64; 2] {
        if self.sc_eq_mix <= 0.0001 {
            return tap;
        }
        let mut sc_filtered = tap;
        for b in &mut self.sc_eq_bands {
            sc_filtered = b.tick(sc_filtered, self.sr);
        }
        [
            tap[0] + self.sc_eq_mix * (sc_filtered[0] - tap[0]),
            tap[1] + self.sc_eq_mix * (sc_filtered[1] - tap[1]),
        ]
    }
    pub fn tick(
        &mut self,
        input: [f64; 2],
        settings: CompSettings,
        eq_on: bool,
        eq2_on: bool,
        sc_eq_on: bool,
        lift_on: bool,
        comp_on: bool,
        comp_pre: bool,
        bypass: bool,
    ) -> [f64; 2] {
        let k = 1.0 - (-1.0 / (self.sr * 0.010)).exp();
        self.eq1_mix += k * (f64::from(eq_on) - self.eq1_mix);
        self.eq2_mix += k * (f64::from(eq2_on) - self.eq2_mix);
        self.sc_eq_mix += k * (f64::from(sc_eq_on) - self.sc_eq_mix);
        self.lift_mix += k * (f64::from(lift_on) - self.lift_mix);
        self.comp_mix += k * (f64::from(comp_on) - self.comp_mix);
        self.comp_pre_mix += k * (f64::from(comp_pre) - self.comp_pre_mix);
        self.bypass_mix += k * (f64::from(bypass) - self.bypass_mix);

        let mono = ((input[0] + input[1]) * 0.5) as f32;
        self.vad.push_sample(mono);

        let effective_comp_mix = self.comp_mix * (1.0 - self.solo_mix);
        let is_pre = self.comp_pre_mix > 0.5;

        // PSE always sits first on the audible path. PRE/POST only moves the
        // compressor (and its SC EQ detector tap).
        let (staged, pse_gr) = self.comp.tick_pse(input, settings, self.sr);

        let (bank_in, pre_gr, pre_sc) = if is_pre {
            let sc_detector = self.sc_detector(staged);
            let (compressed, gr, sc) =
                self.comp.tick_comp(staged, sc_detector, settings, self.sr);
            let mix = if settings.pse.listen {
                1.0
            } else {
                effective_comp_mix
            };
            let comp_in = [
                staged[0] + mix * (compressed[0] - staged[0]),
                staged[1] + mix * (compressed[1] - staged[1]),
            ];
            (comp_in, gr, sc)
        } else {
            (staged, 0.0, -90.0)
        };

        let (mut x, mut dry) = self.bank.tick(bank_in, self.eq1_mix, self.eq2_mix);
        let selected = self.shared.selected_id.load(Ordering::Relaxed);
        let band_gr = if self.bank.config.mode == ProcessingMode::LinearPhase {
            0.0
        } else {
            self.bank
                .bands
                .iter()
                .chain(self.bank.eq2_bands.iter())
                .find(|b| b.band.id == selected)
                .map_or(0.0, |b| b.reduction)
        };
        if self.transition > 0 {
            if let Some(previous) = &mut self.previous {
                let (old, old_dry) = previous.tick(bank_in, self.eq1_mix, self.eq2_mix);
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
        let lift_active = self.lift_bands.iter().any(|b| b.enabled);
        let (lift_out, eq_delayed, dry_delayed) = self.lift.tick(dry, x, &self.lift_bands, self.sr);
        for i in 0..8 {
            self.shared.lift_band_gr[i].store(self.lift.band_gr[i], Ordering::Relaxed);
        }
        for i in 0..256 {
            self.shared.lift_bin_gr[i].store(self.lift.bin_gr[i], Ordering::Relaxed);
            self.shared.lift_bin_gr_uncapped[i]
                .store(self.lift.bin_gr_uncapped[i], Ordering::Relaxed);
        }
        if lift_active {
            for i in 0..2 {
                x[i] = eq_delayed[i] + self.lift_mix * lift_out[i];
            }
            dry = dry_delayed;
        }
        let solo_id = self.shared.solo_id.load(Ordering::Relaxed);
        let is_solo = solo_id != 0;
        let k_solo = 1.0 - (-1.0 / (self.sr * 0.008)).exp();
        self.solo_mix += k_solo * (f64::from(is_solo) - self.solo_mix);

        let is_lift_solo = is_solo && solo_id >= LIFT_ID_BASE;
        let solo_sample = if is_lift_solo {
            Some([lift_out[0] * 1.5, lift_out[1] * 1.5])
        } else if is_solo {
            self.bank
                .bands
                .iter()
                .chain(self.bank.eq2_bands.iter())
                .chain(self.sc_eq_bands.iter())
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
                .map(|c| {
                    [
                        self.solo_filters[0].tick(dry[0], c) * 1.5,
                        self.solo_filters[1].tick(dry[1], c) * 1.5,
                    ]
                })
        } else {
            None
        };

        let (gr, sc_level) = if !is_pre {
            let sc_detector = self.sc_detector(x);
            let (compressed, gr, sc) = self.comp.tick_comp(x, sc_detector, settings, self.sr);
            let mix = if settings.pse.listen {
                1.0
            } else {
                effective_comp_mix
            };
            for i in 0..2 {
                x[i] += mix * (compressed[i] - x[i]);
            }
            (gr, sc)
        } else {
            (pre_gr, pre_sc)
        };

        for i in 0..2 {
            x[i] += self.bypass_mix * (dry[i] - x[i]);
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
        if self.position.is_multiple_of(256) {
            self.shared.sc_level.store(
                if comp_on && !bypass {
                    (sc_level as f32).max(-90.0)
                } else {
                    -90.0
                },
                Ordering::Relaxed,
            );
            self.shared.gr.store(
                (gr * self.comp_mix * (1.0 - self.bypass_mix)) as f32,
                Ordering::Relaxed,
            );
            self.shared.gr_uncapped.store(
                (self.comp.uncapped_gr() * self.comp_mix * (1.0 - self.bypass_mix)) as f32,
                Ordering::Relaxed,
            );
            self.shared
                .pse_gr
                .store((pse_gr * (1.0 - self.bypass_mix)) as f32, Ordering::Relaxed);
        }
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
            self.shared.gr_uncapped.store(
                (self.comp.uncapped_gr() * self.comp_mix * (1.0 - self.bypass_mix)) as f32,
                Ordering::Relaxed,
            );
            self.shared.sc_level.store(
                if comp_on && !bypass {
                    (sc_level as f32).max(-90.0)
                } else {
                    -90.0
                },
                Ordering::Relaxed,
            );
            self.shared.band_gr.store(band_gr as f32, Ordering::Relaxed);
            self.dyn_gr_scratch.clear();
            if self.bank.config.mode != ProcessingMode::LinearPhase {
                for b in self.bank.bands.iter().chain(self.bank.eq2_bands.iter()) {
                    if b.band.dynamic && b.band.shape.has_gain() {
                        self.dyn_gr_scratch
                            .push((b.band.id, b.reduction_uncapped as f32));
                    }
                }
            }
            if let Ok(mut slot) = self.shared.dyn_gr_uncapped.try_lock() {
                std::mem::swap(&mut *slot, &mut self.dyn_gr_scratch);
            }
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
        let shared = Shared::new(
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
        );
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
                engine.eq1_mix = if global_bypass { 1.0 } else { 0.0 };
                engine.eq2_mix = if global_bypass { 1.0 } else { 0.0 };
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
                            &[],
                            44100.0,
                            config,
                        ));
                        assert!(shared.pending.push(next).is_ok());
                        engine.sync();
                    }
                    let x = (i as f64 * 0.1).sin() * 0.3;
                    let y = engine.tick(
                        [x, -x],
                        settings,
                        global_bypass,
                        global_bypass,
                        false,
                        true,
                        false,
                        false,
                        global_bypass,
                    );
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
            .push(Box::new(Bank::new(&[], &[], 48000.0, Config::default())))
            .is_ok());
        engine.sync();
        assert_eq!(engine.latency(), 0);
        while shared.retired.pop().is_some() {}
        assert!(shared
            .pending
            .push(Box::new(Bank::new(&[], &[], 48000.0, config)))
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
                engine.tick(
                    [0.0; 2],
                    CompSettings::default(),
                    true,
                    true,
                    true,
                    true,
                    true,
                    false,
                    false
                ),
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
        let shared = Shared::new(
            bands,
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
        );
        let mut engine = Engine::new(shared, 48000.0);
        assert_eq!(engine.bank.bands.len(), 200);
        let settings = CompSettings {
            threshold: 0.0,
            gate: -80.0,
            dry: 0.0,
            wet: 1.0,
            ..CompSettings::default()
        };
        for i in 0..4096 {
            let x = (i as f64 * 0.1).sin() * 0.5;
            let out = engine.tick([x, x], settings, true, true, false, true, true, false, false);
            assert!((out[0] - x).abs() < 1e-9);
        }
    }
    #[test]
    fn bypass_preserves_input_after_ramp() {
        let shared = Shared::new(
            Arc::new(Mutex::new(vec![Band {
                gain: 18.0,
                ..Band::default()
            }])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
        );
        let mut engine = Engine::new(shared, 48000.0);
        let settings = CompSettings {
            threshold: -30.0,
            gate: -80.0,
            dry: 0.0,
            wet: 1.0,
            ..CompSettings::default()
        };
        for i in 0..24000 {
            let x = (i as f64 * 0.13).sin() * 0.1;
            let out = engine.tick([x, x], settings, true, true, false, true, true, false, true);
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
        let shared = Shared::new(
            Arc::new(Mutex::new(vec![band])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
        );
        let mut engine = Engine::new(shared.clone(), 48000.0);
        let settings = CompSettings {
            threshold: 0.0,
            gate: -80.0,
            dry: 0.0,
            wet: 1.0,
            ..CompSettings::default()
        };
        shared.solo_id.store(1, Ordering::Relaxed);
        let mut distant_energy = 0.0;
        for i in 0..4800 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 100.0 * t).sin();
            let out = engine.tick([x, x], settings, true, true, false, true, true, false, false);
            if i > 2400 {
                distant_energy += out[0].powi(2);
            }
        }
        assert!(distant_energy / 2400.0 < 0.05);
    }
    #[test]
    fn lift_band_parallel_processing_and_solo() {
        let lift = LiftBand {
            id: LIFT_ID_BASE + 1,
            shape: crate::band::Shape::HighShelf,
            freq: 10000.0,
            gain: 0.0,
            threshold: -30.0,
            ratio: 4.0,
            attack: 1.0,
            release: 50.0,
            range: 12.0,
            enabled: true,
            ..LiftBand::default()
        };
        let shared = Shared::new(
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![lift])),
        );
        let mut engine = Engine::new(shared.clone(), 48000.0);
        assert_eq!(engine.latency(), LIFT_LATENCY as u32);
        let settings = CompSettings::default();

        // Feed 10 kHz tone
        for i in 0..8192 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 10000.0 * t).sin() * 0.5;
            let out = engine.tick([x, x], settings, true, true, false, true, false, false, false);
            if i > LIFT_LATENCY + 2048 {
                assert!(out[0].is_finite() && out[1].is_finite());
            }
        }

        // Test soloing the Lift band
        shared.solo_id.store(LIFT_ID_BASE + 1, Ordering::Relaxed);
        let mut solo_energy = 0.0;
        for i in 0..4800 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 10000.0 * t).sin() * 0.5;
            let out = engine.tick([x, x], settings, true, true, false, true, false, false, false);
            if i > 4000 {
                solo_energy += out[0].powi(2);
            }
        }
        assert!(solo_energy > 0.01);
    }
    #[test]
    fn dual_eq_series_processing_both_apply_and_bypass_independently() {
        let b1 = Band {
            id: 1,
            freq: 1000.0,
            gain: 6.0,
            q: 1.0,
            ..Band::default()
        };
        let b2 = Band {
            id: 10_001,
            freq: 1000.0,
            gain: 6.0,
            q: 1.0,
            ..Band::default()
        };
        let shared = Shared::new(
            Arc::new(Mutex::new(vec![b1])),
            Arc::new(Mutex::new(vec![b2])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
        );
        let mut engine = Engine::new(shared, 48000.0);
        let settings = CompSettings {
            threshold: 0.0,
            gate: -80.0,
            dry: 0.0,
            wet: 1.0,
            ..CompSettings::default()
        };

        // Let ramps settle
        for _ in 0..1000 {
            engine.tick([0.0; 2], settings, true, true, false, true, false, false, false);
        }

        // Both EQs active: 6 dB + 6 dB = +12 dB gain at 1 kHz (approx 4.0x linear gain amplitude)
        let mut max_both: f64 = 0.0;
        for i in 0..4800 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * 0.1;
            let out = engine.tick([x, x], settings, true, true, false, true, false, false, false);
            if i > 2400 {
                max_both = max_both.max(out[0].abs());
            }
        }
        assert!(
            (max_both - 0.4).abs() < 0.05,
            "expected ~0.4, got {max_both}"
        );

        // Bypass EQ 2: only EQ 1 applies (+6 dB = ~2.0x linear gain amplitude)
        for _ in 0..2000 {
            engine.tick([0.0; 2], settings, true, false, false, true, false, false, false);
        }
        let mut max_eq1_only: f64 = 0.0;
        for i in 0..4800 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * 0.1;
            let out = engine.tick([x, x], settings, true, false, false, true, false, false, false);
            if i > 2400 {
                max_eq1_only = max_eq1_only.max(out[0].abs());
            }
        }
        assert!(
            (max_eq1_only - 0.2).abs() < 0.05,
            "expected ~0.2, got {max_eq1_only}"
        );
    }

    #[test]
    fn dynamics_pre_and_post_routing_order() {
        let b1 = Band {
            id: 1,
            freq: 1000.0,
            gain: 12.0,
            q: 2.0,
            ..Band::default()
        };
        let shared = Shared::new(
            Arc::new(Mutex::new(vec![b1])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
        );
        let mut engine = Engine::new(shared, 48000.0);
        let settings = CompSettings {
            threshold: -20.0,
            ratio: 4.0,
            attack: 0.5,
            release: 50.0,
            auto_makeup: false,
            gate: -80.0,
            dry: 0.0,
            wet: 1.0,
            ..CompSettings::default()
        };

        // Pre-EQ mode: input is -25 dB (~0.056 amplitude).
        // It enters compressor before EQ boost. Threshold is -20 dB, so little to no GR occurs.
        for _ in 0..1000 {
            engine.tick([0.0; 2], settings, true, false, false, true, true, true, false);
        }
        let mut max_pre = 0.0_f64;
        for i in 0..4800 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * 0.056;
            let out = engine.tick([x, x], settings, true, false, false, true, true, true, false);
            if i > 2400 {
                max_pre = max_pre.max(out[0].abs());
            }
        }

        // Post-EQ mode: input is boosted by +12 dB (4x) first to ~0.224 (-13 dB),
        // entering compressor well above -20 dB threshold, triggering strong gain reduction.
        for _ in 0..2000 {
            engine.tick([0.0; 2], settings, true, false, false, true, true, false, false);
        }
        let mut max_post = 0.0_f64;
        for i in 0..4800 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * 0.056;
            let out = engine.tick([x, x], settings, true, false, false, true, true, false, false);
            if i > 2400 {
                max_post = max_post.max(out[0].abs());
            }
        }

        // Pre output has full +12 dB boost preserved after compressor;
        // Post output was compressed because the EQ boosted it over threshold.
        assert!(
            max_pre > max_post * 1.25,
            "max_pre={max_pre}, max_post={max_post}"
        );
    }

    #[test]
    fn pse_runs_before_eq_in_pre_and_post() {
        let band = Band {
            id: 1,
            freq: 1000.0,
            gain: 18.0,
            q: 2.0,
            ..Band::default()
        };
        let shared = Shared::new(
            Arc::new(Mutex::new(vec![band])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
        );
        let mut pre = Engine::new(shared, 48000.0);
        let post_shared = Shared::new(
            Arc::new(Mutex::new(vec![Band {
                id: 1,
                freq: 1000.0,
                gain: 18.0,
                q: 2.0,
                ..Band::default()
            }])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
        );
        let mut post = Engine::new(post_shared, 48000.0);
        let settings = CompSettings {
            threshold: 0.0,
            auto_makeup: false,
            gate: -40.0,
            dry: 0.0,
            wet: 1.0,
            ..CompSettings::default()
        };
        let amp = 0.003;
        for _ in 0..96000 {
            pre.tick([0.0; 2], settings, true, false, false, true, true, true, false);
            post.tick([0.0; 2], settings, true, false, false, true, true, false, false);
        }
        let mut max_pre = 0.0_f64;
        let mut max_post = 0.0_f64;
        for i in 0..48000 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * amp;
            let pre_out = pre.tick([x, x], settings, true, false, false, true, true, true, false);
            let post_out = post.tick([x, x], settings, true, false, false, true, true, false, false);
            if i > 24000 {
                max_pre = max_pre.max(pre_out[0].abs());
                max_post = max_post.max(post_out[0].abs());
            }
        }
        // +18 dB of EQ on 0.003 would be ~0.024 if PSE ran after the boost.
        // With PSE first (~10 dB down) the boosted peak stays well below that.
        assert!(max_pre < 0.014, "max_pre={max_pre}");
        assert!(max_post < 0.014, "max_post={max_post}");
    }

    #[test]
    fn sidechain_eq_filters_detector_and_listen_path() {
        let sc_band = Band {
            id: 20_001,
            shape: crate::band::Shape::LowCut,
            order: 3,
            freq: 500.0,
            gain: 0.0,
            q: std::f64::consts::FRAC_1_SQRT_2,
            enabled: true,
            dynamic: false,
            ..Band::default()
        };
        let shared = Shared::new(
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![])),
            Arc::new(Mutex::new(vec![sc_band])),
            Arc::new(Mutex::new(vec![])),
        );
        let mut engine = Engine::new(shared.clone(), 48000.0);
        let settings = CompSettings {
            threshold: -20.0,
            ratio: 4.0,
            attack: 0.5,
            release: 50.0,
            auto_makeup: false,
            gate: -80.0,
            dry: 0.0,
            wet: 1.0,
            ..CompSettings::default()
        };

        // Warm up ramps
        for _ in 0..2000 {
            engine.tick([0.0; 2], settings, false, false, true, true, true, false, false);
        }

        // 1. Feed a 100 Hz tone at 0.5 amplitude (-6 dB).
        // With SC EQ LowCut at 500 Hz (3rd order 18 dB/oct), 100 Hz is attenuated ~40 dB,
        // so sidechain detector sees <-45 dB (well below -20 dB threshold) -> no compression!
        for i in 0..4800 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 100.0 * t).sin() * 0.5;
            let out = engine.tick([x, x], settings, false, false, true, true, true, false, false);
            if i > 3000 {
                // Should pass through essentially uncompressed (matches input x)
                assert!((out[0] - x).abs() < 0.01, "out={}, x={}", out[0], x);
            }
        }
        let gr_attenuated = shared.gr.load(Ordering::Relaxed);
        assert!(
            gr_attenuated < 1.0,
            "expected minimal GR, got {gr_attenuated}"
        );

        // 2. Feed a 1000 Hz tone at 0.5 amplitude (-6 dB).
        // 1000 Hz is in the passband (> 500 Hz), so it easily exceeds -20 dB threshold -> strong compression!
        for i in 0..4800 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * 0.5;
            engine.tick([x, x], settings, false, false, true, true, true, false, false);
        }
        let gr_passed = shared.gr.load(Ordering::Relaxed);
        assert!(gr_passed > 5.0, "expected significant GR, got {gr_passed}");

        // 3. Test Listen SC path: output should be the filtered sidechain tone
        let mut listen_settings = settings;
        listen_settings.pse.listen = true;
        let mut max_sc_listen = 0.0_f64;
        for i in 0..4800 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() * 0.5;
            let out = engine.tick(
                [x, x],
                listen_settings,
                false,
                false,
                true,
                true,
                true,
                false,
                false,
            );
            if i > 2400 {
                max_sc_listen = max_sc_listen.max(out[0].abs());
            }
        }
        assert!(
            (max_sc_listen - 0.5).abs() < 0.05,
            "expected passband signal on listen, got {max_sc_listen}"
        );

        // Listen SC stays available when the compressor section is bypassed.
        // 100 Hz is rejected by the 500 Hz SC low-cut, so output must be quiet.
        let mut max_sc_listen_comp_off = 0.0_f64;
        for i in 0..4800 {
            let t = i as f64 / 48000.0;
            let x = (2.0 * std::f64::consts::PI * 100.0 * t).sin() * 0.5;
            let out = engine.tick(
                [x, x],
                listen_settings,
                false,
                false,
                true,
                true,
                false,
                false,
                false,
            );
            if i > 2400 {
                max_sc_listen_comp_off = max_sc_listen_comp_off.max(out[0].abs());
            }
        }
        assert!(
            max_sc_listen_comp_off < 0.05,
            "expected filtered sidechain on listen with compressor off, got {max_sc_listen_comp_off}"
        );
    }
}
