use crate::{
    band::Band,
    dsp::{
        gain_db, wall_lookahead_samples, BandInputMeters, BandRuntime, CompSettings, VocalComp,
        Wall,
    },
    lift::{LiftBand, LiftProcessor, LIFT_ID_BASE, LIFT_LATENCY},
    processing::{Config, ProcessingMode},
    vad::SpeechVad,
};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::{atomic::Ordering, Arc};

mod bank;
mod shared;
mod spectrum;
mod tick;

pub use bank::Bank;
pub use shared::Shared;
use shared::AuxBuffers;
#[cfg(test)]
use crate::dsp::DYNAMIC_BAND_METER_RESERVE;
#[cfg(test)]
use pleasant_dsp::spectrum::{peak_hold, SPECTRUM_FALL_DB_PER_FRAME};
#[cfg(test)]
use shared::AUX_BAND_RESERVE;
#[cfg(test)]
use std::sync::Mutex;

/// Fixed stereo delay used to keep dry and solo paths sample-aligned with
/// the WALL lookahead path.
struct StereoDelay {
    buffer: Vec<[f64; 2]>,
    position: usize,
}

impl StereoDelay {
    fn new(samples: usize) -> Self {
        Self {
            buffer: vec![[0.0; 2]; samples.max(1)],
            position: 0,
        }
    }

    fn reset(&mut self) {
        self.buffer.fill([0.0; 2]);
        self.position = 0;
    }

    fn process(&mut self, input: [f64; 2]) -> [f64; 2] {
        let delayed = self.buffer[self.position];
        self.buffer[self.position] = input;
        self.position = (self.position + 1) % self.buffer.len();
        delayed
    }
}

pub struct Engine {
    shared: Arc<Shared>,
    bank: Box<Bank>,
    previous: Option<Box<Bank>>,
    transition: usize,
    pending_aux: Option<Box<AuxBuffers>>,
    sr: f64,
    k_10ms: f64,
    k_8ms: f64,
    last_output_gain_db: f64,
    last_output_gain: f64,
    comp: VocalComp,
    fft: Arc<dyn Fft<f32>>,
    samples: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    window: [f32; 2048],
    spectrum_bin_ranges: [(usize, usize); 128],
    // Meter cadence remains independent of the optional display FFT.
    position: usize,
    spectrum_position: usize,
    display_enabled: bool,
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
    solo_coeffs: Option<(Band, crate::dsp::BandCoeffs)>,
    lift: LiftProcessor,
    lift_bands: Vec<LiftBand>,
    lift_scratch: Vec<LiftBand>,
    pub sc_eq_bands: Vec<BandRuntime>,
    sc_eq_scratch: Vec<BandRuntime>,
    sc_eq_mix: f64,
    lift_mix: f64,
    wall_mix: f64,
    wall: Wall,
    wall_lookahead: bool,
    wall_base_delay: StereoDelay,
    dry_delay: StereoDelay,
    solo_delay: StereoDelay,
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
            k_10ms: 1.0 - (-1.0 / (sr * 0.010)).exp(),
            k_8ms: 1.0 - (-1.0 / (sr * 0.008)).exp(),
            last_output_gain_db: 0.0,
            last_output_gain: 1.0,
            comp: VocalComp::new(),
            fft,
            samples: vec![Complex::default(); 2048],
            scratch,
            window: spectrum::hann_window(),
            spectrum_bin_ranges: spectrum::bin_ranges(sr),
            position: 0,
            spectrum_position: 0,
            display_enabled: true,
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
            solo_coeffs: None,
            lift: LiftProcessor::new(),
            lift_bands,
            lift_scratch: aux.lift_scratch,
            sc_eq_bands,
            sc_eq_scratch: aux.sc_eq_scratch,
            sc_eq_mix: 1.0,
            lift_mix: 1.0,
            wall_mix: 1.0,
            wall: Wall::new(sr),
            wall_lookahead: false,
            wall_base_delay: StereoDelay::new(wall_lookahead_samples(sr)),
            dry_delay: StereoDelay::new(wall_lookahead_samples(sr)),
            solo_delay: StereoDelay::new(wall_lookahead_samples(sr)),
            vad,
            speech_env: 0.0,
            dyn_gr_scratch: aux.dyn_gr_scratch,
            dyn_input_scratch: aux.dyn_input_scratch,
            band_input: aux.band_input,
        }
    }

    /// Called once per host block. Reopening starts a complete fresh FFT window
    /// without moving the always-on peak/meter clock.
    pub fn set_display_enabled(&mut self, enabled: bool) {
        if self.display_enabled != enabled {
            self.display_enabled = enabled;
            self.spectrum_position = 0;
            if enabled {
                for bin in &self.shared.spectrum {
                    bin.store(-90.0, Ordering::Relaxed);
                }
            }
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
        self.spectrum_position = 0;
        self.samples.fill(Complex::default());
        self.in_peak = 0.0;
        self.out_peak = 0.0;
        self.wall_peak = 0.0;
        self.wall.reset();
        self.wall_base_delay.reset();
        self.dry_delay.reset();
        self.solo_delay.reset();
        self.solo_filters = [crate::dsp::Cascade::default(); 2];
        self.solo_mix = 0.0;
        self.solo_topology = None;
        self.solo_coeffs = None;
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
        let wall_latency = if self.wall_lookahead {
            wall_lookahead_samples(self.sr) as u32
        } else {
            0
        };
        self.bank.config.latency(self.sr) as u32 + lift_latency + wall_latency
    }
}

#[cfg(test)]
mod tests;
