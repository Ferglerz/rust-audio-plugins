use crate::dsp::{Partial, VoiceState, MAX_HARMONICS, MAX_VOICES};
use atomic_float::AtomicF32;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

pub struct FundamentTelemetry {
    pub voice_active: [AtomicBool; MAX_VOICES],
    pub voice_id: [AtomicU32; MAX_VOICES],
    pub voice_f0: [AtomicF32; MAX_VOICES],
    pub voice_presence: [AtomicF32; MAX_VOICES],
    pub partial_freq: [[AtomicF32; MAX_HARMONICS]; MAX_VOICES],
    pub partial_mag: [[AtomicF32; MAX_HARMONICS]; MAX_VOICES],
    pub spectrum_db: [AtomicF32; Self::SPECTRUM_BANDS],
    pub input_peak: AtomicF32,
    pub output_peak: AtomicF32,
    pub cut_depth_db: AtomicF32,
}

impl FundamentTelemetry {
    pub const SPECTRUM_BANDS: usize = 160;

    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            voice_active: std::array::from_fn(|_| AtomicBool::new(false)),
            voice_id: std::array::from_fn(|_| AtomicU32::new(0)),
            voice_f0: std::array::from_fn(|_| AtomicF32::new(0.0)),
            voice_presence: std::array::from_fn(|_| AtomicF32::new(0.0)),
            partial_freq: std::array::from_fn(|_| std::array::from_fn(|_| AtomicF32::new(0.0))),
            partial_mag: std::array::from_fn(|_| std::array::from_fn(|_| AtomicF32::new(0.0))),
            spectrum_db: std::array::from_fn(|_| AtomicF32::new(-120.0)),
            input_peak: AtomicF32::new(0.0),
            output_peak: AtomicF32::new(0.0),
            cut_depth_db: AtomicF32::new(0.0),
        })
    }

    /// Log-spaced center from 20 Hz through 20 kHz.
    pub fn band_hz(index: usize) -> f32 {
        let last = (Self::SPECTRUM_BANDS - 1) as f32;
        let t = (index.min(Self::SPECTRUM_BANDS - 1) as f32) / last;
        20.0 * 1_000.0f32.powf(t)
    }

    pub fn publish_voices(&self, voices: &[VoiceState; MAX_VOICES]) {
        for (index, voice) in voices.iter().enumerate() {
            self.voice_active[index].store(voice.active, Ordering::Relaxed);
            self.voice_id[index].store(voice.id, Ordering::Relaxed);
            self.voice_f0[index].store(voice.f0_hz, Ordering::Relaxed);
            self.voice_presence[index].store(voice.presence, Ordering::Relaxed);
            for harmonic in 0..MAX_HARMONICS {
                self.partial_freq[index][harmonic]
                    .store(voice.partials[harmonic].freq_hz, Ordering::Relaxed);
                self.partial_mag[index][harmonic]
                    .store(voice.partials[harmonic].mag, Ordering::Relaxed);
            }
        }
    }

    pub fn publish_spectrum(&self, mags: &[f32], bin_hz: f32) {
        if mags.is_empty() || !(bin_hz > 0.0) || !bin_hz.is_finite() {
            return;
        }
        let last_bin = mags.len() - 1;
        for band in 0..Self::SPECTRUM_BANDS {
            let (start, end) = band_bins(band, bin_hz, last_bin);
            let mut peak = 0.0f32;
            for mag in &mags[start..=end] {
                peak = peak.max(*mag);
            }
            let db = pleasant_dsp::units::linear_to_db_with_floor(f64::from(peak), 1.0e-6) as f32;
            let previous = self.spectrum_db[band].load(Ordering::Relaxed);
            self.spectrum_db[band].store(db.max(previous - 1.5), Ordering::Relaxed);
        }
    }

    pub fn publish_levels(&self, in_peak: f32, out_peak: f32) {
        self.input_peak.store(in_peak, Ordering::Relaxed);
        self.output_peak.store(out_peak, Ordering::Relaxed);
    }

    pub fn voice_snapshot(&self, index: usize) -> VoiceState {
        if index >= MAX_VOICES {
            return VoiceState::default();
        }
        let mut partials = [Partial::default(); MAX_HARMONICS];
        for harmonic in 0..MAX_HARMONICS {
            partials[harmonic] = Partial {
                freq_hz: self.partial_freq[index][harmonic].load(Ordering::Relaxed),
                mag: self.partial_mag[index][harmonic].load(Ordering::Relaxed),
            };
        }
        VoiceState {
            active: self.voice_active[index].load(Ordering::Relaxed),
            id: self.voice_id[index].load(Ordering::Relaxed),
            f0_hz: self.voice_f0[index].load(Ordering::Relaxed),
            presence: self.voice_presence[index].load(Ordering::Relaxed),
            partials,
        }
    }

    pub fn spectrum(&self, out: &mut [f32; Self::SPECTRUM_BANDS]) {
        for (band, sample) in out.iter_mut().enumerate() {
            *sample = self.spectrum_db[band].load(Ordering::Relaxed);
        }
    }
}

fn band_bins(band: usize, bin_hz: f32, last_bin: usize) -> (usize, usize) {
    let center = FundamentTelemetry::band_hz(band);
    let nearest = (center / bin_hz).round().clamp(0.0, last_bin as f32) as usize;
    let lower_hz = if band == 0 {
        0.0
    } else {
        (FundamentTelemetry::band_hz(band - 1) * center).sqrt()
    };
    let upper_hz = if band + 1 >= FundamentTelemetry::SPECTRUM_BANDS {
        f32::MAX
    } else {
        (center * FundamentTelemetry::band_hz(band + 1)).sqrt()
    };

    let mut lo = (lower_hz / bin_hz).ceil();
    let mut hi = (upper_hz / bin_hz).floor();
    if !lo.is_finite() {
        lo = nearest as f32;
    }
    if !hi.is_finite() {
        hi = last_bin as f32;
    }
    lo = lo.clamp(0.0, last_bin as f32);
    hi = hi.clamp(0.0, last_bin as f32);
    if lo > hi {
        return (nearest, nearest);
    }
    ((lo as usize).min(nearest), (hi as usize).max(nearest))
}
