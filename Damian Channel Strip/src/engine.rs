use crate::{
    band::Band,
    dsp::{
        gain_db, tick_wall, BandInputMeters, BandRuntime, CompSettings, VocalComp,
        DYNAMIC_BAND_METER_RESERVE,
    },
    lift::{LiftBand, LiftProcessor, LIFT_ID_BASE, LIFT_LATENCY},
    processing::{Config, ProcessingMode},
    vad::SpeechVad,
};
use atomic_float::AtomicF32;
use crossbeam_queue::ArrayQueue;
use pleasant_dsp::spectrum::{peak_hold, SPECTRUM_FALL_DB_PER_FRAME};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

mod bank;

pub use bank::Bank;

const AUX_BAND_RESERVE: usize = 64;

struct AuxBuffers {
    lift_bands: Vec<LiftBand>,
    lift_scratch: Vec<LiftBand>,
    sc_eq_bands: Vec<BandRuntime>,
    sc_eq_scratch: Vec<BandRuntime>,
    dyn_gr_scratch: Vec<(u64, f32)>,
    dyn_input_scratch: Vec<(u64, f32)>,
    dyn_gr_slot: Vec<(u64, f32)>,
    dyn_input_slot: Vec<(u64, f32)>,
    band_input: BandInputMeters,
    sr: f64,
}

impl AuxBuffers {
    fn new(lift: &[LiftBand], sc: &[Band], sr: f64, band_count: usize) -> Self {
        let meter_capacity = band_count.max(DYNAMIC_BAND_METER_RESERVE);
        let mut lift_bands = Vec::with_capacity(lift.len().max(AUX_BAND_RESERVE));
        lift_bands.extend_from_slice(lift);
        let mut sc_eq_bands = Vec::with_capacity(sc.len().max(AUX_BAND_RESERVE));
        sc_eq_bands.extend(sc.iter().cloned().map(|b| BandRuntime::new(b, sr)));
        Self {
            lift_bands,
            lift_scratch: Vec::with_capacity(lift.len().max(AUX_BAND_RESERVE)),
            sc_eq_bands,
            sc_eq_scratch: Vec::with_capacity(sc.len().max(AUX_BAND_RESERVE)),
            dyn_gr_scratch: Vec::with_capacity(meter_capacity),
            dyn_input_scratch: Vec::with_capacity(meter_capacity),
            dyn_gr_slot: Vec::with_capacity(meter_capacity),
            dyn_input_slot: Vec::with_capacity(meter_capacity),
            band_input: BandInputMeters::with_capacity(sr, meter_capacity),
            sr,
        }
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
    pub gr_peak: AtomicF32,
    pub pse_gr: AtomicF32,
    pub pse_gr_peak: AtomicF32,
    pub transport_playing: AtomicBool,
    pub sc_level: AtomicF32,
    pub wall_level: AtomicF32,
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
    pub dyn_band_input: Mutex<Vec<(u64, f32)>>,
    pub pending: ArrayQueue<Box<Bank>>,
    pub retired: ArrayQueue<Box<Bank>>,
    pending_aux: ArrayQueue<Box<AuxBuffers>>,
    retired_aux: ArrayQueue<Box<AuxBuffers>>,
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
            gr_peak: AtomicF32::new(0.0),
            pse_gr: AtomicF32::new(0.0),
            pse_gr_peak: AtomicF32::new(0.0),
            transport_playing: AtomicBool::new(false),
            sc_level: AtomicF32::new(-90.0),
            wall_level: AtomicF32::new(-90.0),
            speech_env: AtomicF32::new(0.0),
            sample_rate: AtomicF32::new(44100.0),
            spectrum: std::array::from_fn(|_| AtomicF32::new(-90.0)),
            lift_band_gr: std::array::from_fn(|_| AtomicF32::new(0.0)),
            lift_bin_gr: std::array::from_fn(|_| AtomicF32::new(0.0)),
            lift_bin_gr_uncapped: std::array::from_fn(|_| AtomicF32::new(0.0)),
            selected_id: std::sync::atomic::AtomicU64::new(0),
            solo_id: std::sync::atomic::AtomicU64::new(0),
            band_gr: AtomicF32::new(0.0),
            dyn_gr_uncapped: Mutex::new(Vec::with_capacity(DYNAMIC_BAND_METER_RESERVE)),
            dyn_band_input: Mutex::new(Vec::with_capacity(DYNAMIC_BAND_METER_RESERVE)),
            pending: ArrayQueue::new(1),
            retired: ArrayQueue::new(2),
            pending_aux: ArrayQueue::new(1),
            retired_aux: ArrayQueue::new(2),
            stop: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
        });
        let weak = Arc::downgrade(&shared);
        let stop = shared.stop.clone();
        let handle = std::thread::spawn(move || {
            let mut last: Option<(Vec<Band>, Vec<Band>, f64, Config)> = None;
            let mut last_aux: Option<(Vec<LiftBand>, Vec<Band>, f64, usize)> = None;
            while !stop.load(Ordering::Relaxed) {
                if let Some(s) = weak.upgrade() {
                    while s.retired.pop().is_some() {}
                    while s.retired_aux.pop().is_some() {}
                    if !s.pending_aux.is_full() {
                        let sr = s.sample_rate.load(Ordering::Relaxed) as f64;
                        let lift = lift_bands.lock().unwrap_or_else(|e| e.into_inner()).clone();
                        let sc = sc_eq_bands
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .clone();
                        let count = bands.lock().unwrap_or_else(|e| e.into_inner()).len()
                            + eq2_bands.lock().unwrap_or_else(|e| e.into_inner()).len();
                        if last_aux
                            .as_ref()
                            .is_none_or(|(old_lift, old_sc, rate, old_count)| {
                                *old_lift != lift
                                    || *old_sc != sc
                                    || *rate != sr
                                    || *old_count != count
                            })
                        {
                            let aux = Box::new(AuxBuffers::new(&lift, &sc, sr, count));
                            if s.pending_aux.push(aux).is_ok() {
                                last_aux = Some((lift, sc, sr, count));
                            }
                        }
                    }
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

    pub fn reset_gr_peaks(&self) {
        self.gr_peak.store(0.0, Ordering::Relaxed);
        self.pse_gr_peak.store(0.0, Ordering::Relaxed);
    }

    /// Returns true on rising edge of host transport play.
    pub fn note_transport_playing(&self, playing: bool) -> bool {
        let was = self.transport_playing.swap(playing, Ordering::Relaxed);
        playing && !was
    }

    pub fn update_gr_peaks_while_playing(&self, playing: bool) {
        if !playing {
            return;
        }
        let gr = self.gr.load(Ordering::Relaxed);
        let pse = self.pse_gr.load(Ordering::Relaxed);
        let gr_peak = self.gr_peak.load(Ordering::Relaxed).max(gr.max(0.0));
        let pse_peak = self.pse_gr_peak.load(Ordering::Relaxed).max(pse.max(0.0));
        self.gr_peak.store(gr_peak, Ordering::Relaxed);
        self.pse_gr_peak.store(pse_peak, Ordering::Relaxed);
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
    pending_aux: Option<Box<AuxBuffers>>,
    sr: f64,
    comp: VocalComp,
    fft: Arc<dyn Fft<f32>>,
    samples: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    window: [f32; 2048],
    spectrum_bin_ranges: [(usize, usize); 128],
    position: usize,
    in_peak: f64,
    out_peak: f64,
    wall_peak: f64,
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
    lift_scratch: Vec<LiftBand>,
    pub sc_eq_bands: Vec<BandRuntime>,
    sc_eq_scratch: Vec<BandRuntime>,
    sc_eq_mix: f64,
    lift_mix: f64,
    wall_mix: f64,
    vad: SpeechVad,
    speech_env: f32,
    dyn_gr_scratch: Vec<(u64, f32)>,
    dyn_input_scratch: Vec<(u64, f32)>,
    band_input: BandInputMeters,
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
        let lift_snapshot = shared
            .lift_bands
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let sc_snapshot = shared
            .sc_eq_bands
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let mut aux = AuxBuffers::new(
            &lift_snapshot,
            &sc_snapshot,
            sr,
            bank.bands.len() + bank.eq2_bands.len(),
        );
        std::mem::swap(
            &mut *shared
                .dyn_gr_uncapped
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
            &mut aux.dyn_gr_slot,
        );
        std::mem::swap(
            &mut *shared
                .dyn_band_input
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
            &mut aux.dyn_input_slot,
        );
        let lift_bands = aux.lift_bands;
        let sc_eq_bands = aux.sc_eq_bands;
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
            pending_aux: None,
            sr,
            comp: VocalComp::new(),
            fft,
            samples: vec![Complex::default(); 2048],
            scratch,
            window: std::array::from_fn(|i| {
                0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / 2048.0).cos()
            }),
            spectrum_bin_ranges: std::array::from_fn(|i| {
                let lo = 20.0_f64 * 1000.0_f64.powf(i as f64 / 128.0);
                let hi = 20.0_f64 * 1000.0_f64.powf((i + 1) as f64 / 128.0);
                (
                    (lo / sr * 2048.0).floor() as usize,
                    ((hi / sr * 2048.0).ceil() as usize).min(1023),
                )
            }),
            position: 0,
            in_peak: 0.0,
            out_peak: 0.0,
            wall_peak: 0.0,
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
            lift_scratch: aux.lift_scratch,
            sc_eq_bands,
            sc_eq_scratch: aux.sc_eq_scratch,
            sc_eq_mix: 1.0,
            lift_mix: 1.0,
            wall_mix: 1.0,
            vad,
            speech_env: 0.0,
            dyn_gr_scratch: aux.dyn_gr_scratch,
            dyn_input_scratch: aux.dyn_input_scratch,
            band_input: aux.band_input,
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
        self.wall_peak = 0.0;
        self.solo_filters = [crate::dsp::Cascade::default(); 2];
        self.solo_mix = 0.0;
        self.solo_topology = None;
        self.lift.reset();
        for b in &mut self.sc_eq_bands {
            b.reset();
        }
        self.vad.reset();
        self.speech_env = 0.0;
        self.band_input.reset();
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
        if !self.shared.retired_aux.is_full() {
            if self.pending_aux.is_none() {
                self.pending_aux = self.shared.pending_aux.pop();
            }
            if let Some(mut aux) = self.pending_aux.take() {
                let grow_meters = aux.sr == self.sr
                    && aux.dyn_gr_scratch.capacity() > self.dyn_gr_scratch.capacity();
                if grow_meters {
                    if let (Ok(mut gr), Ok(mut input)) = (
                        self.shared.dyn_gr_uncapped.try_lock(),
                        self.shared.dyn_band_input.try_lock(),
                    ) {
                        aux.band_input.inherit(&self.band_input);
                        std::mem::swap(&mut self.dyn_gr_scratch, &mut aux.dyn_gr_scratch);
                        std::mem::swap(&mut self.dyn_input_scratch, &mut aux.dyn_input_scratch);
                        std::mem::swap(&mut *gr, &mut aux.dyn_gr_slot);
                        std::mem::swap(&mut *input, &mut aux.dyn_input_slot);
                        std::mem::swap(&mut self.band_input, &mut aux.band_input);
                    } else {
                        self.pending_aux = Some(aux);
                        return;
                    }
                }
                // Only the worker can grow these buffers. Keep replaced storage
                // alive until it can also be destroyed on the worker.
                if aux.sr == self.sr
                    && (aux.lift_bands.len() > self.lift_scratch.capacity()
                        || aux.sc_eq_bands.len() > self.sc_eq_scratch.capacity())
                {
                    for runtime in &mut aux.sc_eq_bands {
                        if let Some(old) = self
                            .sc_eq_bands
                            .iter()
                            .find(|old| old.band.id == runtime.band.id)
                        {
                            runtime.inherit(old);
                        }
                    }
                    std::mem::swap(&mut self.lift_bands, &mut aux.lift_bands);
                    std::mem::swap(&mut self.lift_scratch, &mut aux.lift_scratch);
                    std::mem::swap(&mut self.sc_eq_bands, &mut aux.sc_eq_bands);
                    std::mem::swap(&mut self.sc_eq_scratch, &mut aux.sc_eq_scratch);
                    self.shared.latency.store(self.latency(), Ordering::Relaxed);
                }
                let result = self.shared.retired_aux.push(aux);
                debug_assert!(result.is_ok());
            }
        }
        if let Ok(guard) = self.shared.lift_bands.try_lock() {
            if *guard != self.lift_bands && guard.len() <= self.lift_scratch.capacity() {
                self.lift_scratch.clear();
                self.lift_scratch.extend_from_slice(&guard);
                std::mem::swap(&mut self.lift_bands, &mut self.lift_scratch);
                self.shared.latency.store(self.latency(), Ordering::Relaxed);
            }
        }
        if let Ok(guard) = self.shared.sc_eq_bands.try_lock() {
            if guard.len() <= self.sc_eq_scratch.capacity()
                && (self.sc_eq_bands.len() != guard.len()
                    || self
                        .sc_eq_bands
                        .iter()
                        .zip(guard.iter())
                        .any(|(r, b)| r.band != *b))
            {
                self.sc_eq_scratch.clear();
                for b in guard.iter() {
                    let mut runtime = BandRuntime::new(b.clone(), self.sr);
                    if let Some(old) = self.sc_eq_bands.iter().find(|old| old.band.id == b.id) {
                        runtime.inherit(old);
                    }
                    self.sc_eq_scratch.push(runtime);
                }
                std::mem::swap(&mut self.sc_eq_bands, &mut self.sc_eq_scratch);
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
        smooth_eq_bypass(&mut self.eq1_mix, eq_on, k);
        smooth_eq_bypass(&mut self.eq2_mix, eq2_on, k);
        self.sc_eq_mix += k * (f64::from(sc_eq_on) - self.sc_eq_mix);
        self.lift_mix += k * (f64::from(lift_on) - self.lift_mix);
        self.wall_mix += k * (f64::from(settings.wall.on) - self.wall_mix);
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
            let (compressed, gr, sc) = self.comp.tick_comp(staged, sc_detector, settings, self.sr);
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

        self.band_input.tick(bank_in);
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

        self.wall_peak = self.wall_peak.max(x[0].abs()).max(x[1].abs());
        let clipped = tick_wall(x, settings.wall);
        for i in 0..2 {
            x[i] += self.wall_mix * (clipped[i] - x[i]);
        }

        let out_gain = 10.0_f64.powf(settings.output_gain / 20.0);
        for i in 0..2 {
            x[i] *= out_gain;
        }
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
            self.band_input.sync(
                self.bank
                    .bands
                    .iter()
                    .chain(self.bank.eq2_bands.iter())
                    .map(|b| &b.band),
            );
            self.band_input.write_levels_db(&mut self.dyn_input_scratch);
            if let Ok(mut slot) = self.shared.dyn_band_input.try_lock() {
                std::mem::swap(&mut *slot, &mut self.dyn_input_scratch);
            }
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
            self.shared.wall_level.store(
                if settings.wall.on && !bypass {
                    (gain_db(self.wall_peak) as f32).max(-90.0)
                } else {
                    -90.0
                },
                Ordering::Relaxed,
            );
        }
        if self.position == 2048 {
            self.position = 0;
            self.fft
                .process_with_scratch(&mut self.samples, &mut self.scratch);
            for i in 0..128 {
                let (start, end) = self.spectrum_bin_ranges[i];
                let mut mag = 0.0_f32;
                if start <= end {
                    for bin in start.max(1)..=end {
                        mag = mag.max(self.samples[bin].norm() / 512.0);
                    }
                }
                let db = (20.0 * mag.max(0.0000316).log10()).max(-90.0);
                let old = self.shared.spectrum[i].load(Ordering::Relaxed);
                self.shared.spectrum[i].store(
                    peak_hold(old, db, SPECTRUM_FALL_DB_PER_FRAME),
                    Ordering::Relaxed,
                );
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
                    if b.band.dynamic
                        && b.band.shape.has_gain()
                        && self.dyn_gr_scratch.len() < self.dyn_gr_scratch.capacity()
                    {
                        self.dyn_gr_scratch
                            .push((b.band.id, b.reduction_uncapped as f32));
                    }
                }
            }
            if let Ok(mut slot) = self.shared.dyn_gr_uncapped.try_lock() {
                std::mem::swap(&mut *slot, &mut self.dyn_gr_scratch);
            }
            self.shared.wall_level.store(
                if settings.wall.on && !bypass {
                    (gain_db(self.wall_peak) as f32).max(-90.0)
                } else {
                    -90.0
                },
                Ordering::Relaxed,
            );
            self.in_peak = 0.0;
            self.out_peak = 0.0;
            self.wall_peak = 0.0;
        }
        x
    }
}

fn smooth_eq_bypass(mix: &mut f64, enabled: bool, coefficient: f64) {
    let target = f64::from(enabled);
    *mix += coefficient * (target - *mix);
    if (*mix - target).abs() < 1.0e-10 {
        *mix = target;
    }
}
#[cfg(test)]
mod tests;
