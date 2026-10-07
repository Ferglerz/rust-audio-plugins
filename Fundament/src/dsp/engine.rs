use super::cut_bank::CutBank;
use super::delay::DelayLine;
use super::peaks::pick_peaks;
use super::salience::pick_f0s;
use super::stft::StftAnalyzer;
use super::synth::AdditiveSynth;
use super::tracker::VoiceTracker;
use super::types::{
    EngineSettings, F0Candidate, Peak, VoiceState, MAX_CHANNELS, MAX_PEAKS, MAX_VOICES,
};

const SMOOTH_SECONDS: f32 = 0.010;

pub struct FundamentEngine {
    stft: StftAnalyzer,
    tracker: VoiceTracker,
    cut: CutBank,
    synth: AdditiveSynth,
    delay: DelayLine,
    settings: EngineSettings,
    peaks: [Peak; MAX_PEAKS],
    peak_count: usize,
    candidates: [F0Candidate; MAX_VOICES],
    candidate_count: usize,
    sample_rate: f32,
    channels: usize,
    mix: f32,
    output_gain: f32,
    smooth_coeff: f32,
    analysis_frames: u64,
}

impl Default for FundamentEngine {
    fn default() -> Self {
        let settings = EngineSettings::default();
        Self {
            stft: StftAnalyzer::new(),
            tracker: VoiceTracker::new(),
            cut: CutBank::new(),
            synth: AdditiveSynth::new(),
            delay: DelayLine::new(),
            settings,
            peaks: [Peak::default(); MAX_PEAKS],
            peak_count: 0,
            candidates: [F0Candidate::default(); MAX_VOICES],
            candidate_count: 0,
            sample_rate: 0.0,
            channels: 0,
            mix: settings.mix.clamp(0.0, 1.0),
            output_gain: db_to_gain(settings.output_db),
            smooth_coeff: 0.0,
            analysis_frames: 0,
        }
    }
}

impl FundamentEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prepare(&mut self, sample_rate: f32, max_block: usize, channels: usize) {
        let _ = max_block;
        self.stft.prepare(sample_rate);
        self.delay.prepare(self.stft.latency(), channels);
        self.cut.prepare(sample_rate as f64, channels);
        self.synth.prepare(sample_rate);
        self.sample_rate = sample_rate;
        self.channels = channels;
        self.smooth_coeff = one_pole_coeff(sample_rate, SMOOTH_SECONDS);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.stft.reset();
        self.delay.reset();
        self.cut.reset();
        self.synth.reset();
        self.tracker.reset();
        self.peaks = [Peak::default(); MAX_PEAKS];
        self.peak_count = 0;
        self.candidates = [F0Candidate::default(); MAX_VOICES];
        self.candidate_count = 0;
        self.analysis_frames = 0;
        self.snap_smoothers();
    }

    pub fn set_settings(&mut self, settings: EngineSettings) {
        self.settings = settings;
    }

    pub fn process_block(&mut self, channels: &mut [&mut [f32]]) {
        let n_ch = active_channels(channels.len(), self.channels);
        let n_frames = frame_count(channels, n_ch);
        if n_ch == 0 || n_frames == 0 {
            return;
        }

        let target_mix = self.settings.mix.clamp(0.0, 1.0);
        let target_gain = db_to_gain(self.settings.output_db);
        let pole = self.smooth_coeff;
        let mut mix = self.mix;
        let mut gain = self.output_gain;
        let inv_ch = 1.0 / n_ch as f32;

        for frame in 0..n_frames {
            let mut sum = 0.0f32;
            for channel in channels.iter().take(n_ch) {
                sum += channel[frame];
            }
            if self.stft.push(sum * inv_ch) {
                self.analyze();
            }

            let mut delayed = [0.0f32; MAX_CHANNELS];
            for (channel, sample) in delayed.iter_mut().zip(channels.iter()).take(n_ch) {
                *channel = sample[frame];
            }
            self.delay.process_frame(&mut delayed[..n_ch]);

            let mut cut = [0.0f64; MAX_CHANNELS];
            for (sample, dry) in cut.iter_mut().zip(delayed.iter()).take(n_ch) {
                *sample = f64::from(*dry);
            }
            self.cut.process_frame(&mut cut[..n_ch]);

            let synth = self.synth.next_sample();
            mix = target_mix + (mix - target_mix) * pole;
            gain = target_gain + (gain - target_gain) * pole;
            let dry_mix = 1.0 - mix;
            for channel in 0..n_ch {
                let wet = cut[channel] as f32 + synth;
                channels[channel][frame] = (delayed[channel] * dry_mix + wet * mix) * gain;
            }
        }

        self.mix = mix;
        self.output_gain = gain;
    }

    /// Delay only. Tracker and synth envelopes stay where they are.
    pub fn process_bypass(&mut self, channels: &mut [&mut [f32]]) {
        let n_ch = active_channels(channels.len(), self.channels);
        let n_frames = frame_count(channels, n_ch);
        if n_ch == 0 || n_frames == 0 {
            return;
        }

        for frame in 0..n_frames {
            let mut delayed = [0.0f32; MAX_CHANNELS];
            for (channel, sample) in delayed.iter_mut().zip(channels.iter()).take(n_ch) {
                *channel = sample[frame];
            }
            self.delay.process_frame(&mut delayed[..n_ch]);
            for channel in 0..n_ch {
                channels[channel][frame] = delayed[channel];
            }
        }
    }

    pub fn latency_samples(&self) -> u32 {
        self.stft.latency() as u32
    }

    pub fn voices(&self) -> &[VoiceState; MAX_VOICES] {
        self.tracker.voices()
    }

    pub fn analysis_magnitudes(&self) -> &[f32] {
        self.stft.magnitudes()
    }

    pub fn bin_hz(&self) -> f32 {
        self.stft.bin_hz()
    }

    pub fn analysis_frames(&self) -> u64 {
        self.analysis_frames
    }

    fn analyze(&mut self) {
        let settings = self.settings;
        let bin_hz = self.stft.bin_hz();
        let hop = self.stft.hop();
        self.peak_count = pick_peaks(
            self.stft.magnitudes(),
            self.stft.prev_phases(),
            self.stft.phases(),
            bin_hz,
            hop,
            self.sample_rate,
            settings.sensitivity_db,
            &mut self.peaks,
        );
        self.candidate_count = pick_f0s(
            &self.peaks[..self.peak_count],
            settings.harmonics,
            settings.low_hz,
            settings.high_hz,
            settings.max_voices,
            &mut self.candidates,
        );
        let frame_dt = if self.sample_rate > 0.0 {
            hop as f32 / self.sample_rate
        } else {
            0.0
        };
        self.tracker.update(
            &self.candidates[..self.candidate_count],
            &self.peaks[..self.peak_count],
            &settings,
            frame_dt,
        );
        self.cut.set_targets(
            self.tracker.voices(),
            settings.cut_depth_db,
            settings.cut_q,
            settings.harmonics,
        );
        self.synth.set_targets(self.tracker.voices(), &settings);
        self.analysis_frames = self.analysis_frames.wrapping_add(1);
    }

    fn snap_smoothers(&mut self) {
        self.mix = self.settings.mix.clamp(0.0, 1.0);
        self.output_gain = db_to_gain(self.settings.output_db);
    }
}

fn active_channels(buffer_channels: usize, prepared: usize) -> usize {
    buffer_channels.min(prepared).min(MAX_CHANNELS)
}

fn frame_count(channels: &[&mut [f32]], n_ch: usize) -> usize {
    let mut frames = usize::MAX;
    for channel in channels.iter().take(n_ch) {
        frames = frames.min(channel.len());
    }
    if frames == usize::MAX {
        0
    } else {
        frames
    }
}

fn one_pole_coeff(sample_rate: f32, seconds: f32) -> f32 {
    if sample_rate <= 0.0 || seconds <= 0.0 {
        0.0
    } else {
        (-1.0 / (seconds * sample_rate)).exp()
    }
}

fn db_to_gain(db: f32) -> f32 {
    let gain = 10.0f32.powf(db * 0.05);
    if gain.is_finite() {
        gain
    } else {
        0.0
    }
}
