//! Plain data shared by every DSP stage. No host types live here.

pub const MAX_VOICES: usize = 6;
pub const MAX_HARMONICS: usize = 16;
pub const MAX_PEAKS: usize = 128;
pub const MAX_CHANNELS: usize = 2;

/// Spectral peak. `mag` is linear amplitude: a sine of amplitude `A` reports `mag ≈ A`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Peak {
    pub freq_hz: f32,
    pub mag: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct F0Candidate {
    pub f0_hz: f32,
    pub salience: f32,
}

/// One harmonic of a voice. `mag` is linear amplitude; `0.0` means not found.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Partial {
    pub freq_hz: f32,
    pub mag: f32,
}

/// Tracked voice. `partials[h - 1]` is harmonic `h`; `partials[0]` is the fundamental.
/// `presence` is the 0..=1 fade envelope applied to cuts and synthesis.
/// `id` changes whenever a slot is reused for a new note; `0` is never a live id.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VoiceState {
    pub active: bool,
    pub id: u32,
    pub f0_hz: f32,
    pub presence: f32,
    pub partials: [Partial; MAX_HARMONICS],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Waveform {
    #[default]
    Sine,
    Saw,
    Square,
    Triangle,
}

/// Host-free processing settings. `params.rs` builds this once per block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EngineSettings {
    /// 1..=MAX_VOICES
    pub max_voices: usize,
    /// 1..=MAX_HARMONICS
    pub harmonics: usize,
    /// Fundamental search range, hertz.
    pub low_hz: f32,
    pub high_hz: f32,
    /// Peak threshold, dBFS of linear peak amplitude.
    pub sensitivity_db: f32,
    /// Cut gain at full presence, dB, <= 0.
    pub cut_depth_db: f32,
    /// Bell Q for every cut.
    pub cut_q: f32,
    /// Synth gain relative to measured partial level, dB.
    pub synth_level_db: f32,
    /// 0 = measured partial amplitudes, 1 = `waveform` spectrum.
    pub tone: f32,
    pub waveform: Waveform,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub glide_ms: f32,
    /// 0 = dry only, 1 = cut signal plus synth.
    pub mix: f32,
    pub output_db: f32,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            max_voices: 3,
            harmonics: 8,
            low_hz: 35.0,
            high_hz: 1_200.0,
            sensitivity_db: -50.0,
            cut_depth_db: -12.0,
            cut_q: 8.0,
            synth_level_db: 0.0,
            tone: 0.0,
            waveform: Waveform::Sine,
            attack_ms: 20.0,
            release_ms: 120.0,
            glide_ms: 15.0,
            mix: 1.0,
            output_db: 0.0,
        }
    }
}
