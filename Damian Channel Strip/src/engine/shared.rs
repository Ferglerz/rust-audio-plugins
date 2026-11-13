use crate::{
    band::Band,
    dsp::{BandInputMeters, BandRuntime, DYNAMIC_BAND_METER_RESERVE},
    lift::LiftBand,
    processing::Config,
};
use atomic_float::AtomicF32;
use crossbeam_queue::ArrayQueue;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use super::Bank;

pub(super) const AUX_BAND_RESERVE: usize = 64;

pub(super) struct AuxBuffers {
    pub lift_bands: Vec<LiftBand>,
    pub lift_scratch: Vec<LiftBand>,
    pub sc_eq_bands: Vec<BandRuntime>,
    pub sc_eq_scratch: Vec<BandRuntime>,
    pub dyn_gr_scratch: Vec<(u64, f32)>,
    pub dyn_input_scratch: Vec<(u64, f32)>,
    pub dyn_gr_slot: Vec<(u64, f32)>,
    pub dyn_input_slot: Vec<(u64, f32)>,
    pub band_input: BandInputMeters,
    pub sr: f64,
}

impl AuxBuffers {
    pub(super) fn new(lift: &[LiftBand], sc: &[Band], sr: f64, band_count: usize) -> Self {
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
    pub(super) bands: Arc<Mutex<Vec<Band>>>,
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
    pub(super) pending_aux: ArrayQueue<Box<AuxBuffers>>,
    pub(super) retired_aux: ArrayQueue<Box<AuxBuffers>>,
    pub(super) stop: Arc<AtomicBool>,
    pub(super) worker: Mutex<Option<std::thread::JoinHandle<()>>>,
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
