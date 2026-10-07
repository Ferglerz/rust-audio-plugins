use pleasant_dsp::filters::{BiquadCoefficients, BiquadKind, TransposedDirectForm2};

use super::types::{VoiceState, MAX_CHANNELS, MAX_HARMONICS, MAX_VOICES};

const SLOTS: usize = MAX_VOICES * MAX_HARMONICS;
const UPDATE_PERIOD: u32 = 32;
const SMOOTH_TIME_S: f64 = 0.010;
const RESET_FREQ_HZ: f64 = 1_000.0;
const MIN_SMOOTH_FREQ_HZ: f64 = 1.0;
const ACTIVE_GAIN_DB: f64 = 0.05;
const JUMP_GAIN_DB: f64 = 0.1;
const MIN_PARTIAL_HZ: f64 = 20.0;

#[derive(Clone, Copy)]
struct Slot {
    target_freq: f64,
    target_gain_db: f64,
    current_freq: f64,
    current_gain_db: f64,
    voice_id: u32,
    active: bool,
    coefficients: BiquadCoefficients,
}

impl Slot {
    const fn init() -> Self {
        Self {
            target_freq: RESET_FREQ_HZ,
            target_gain_db: 0.0,
            current_freq: RESET_FREQ_HZ,
            current_gain_db: 0.0,
            voice_id: 0,
            active: false,
            coefficients: BiquadCoefficients::IDENTITY,
        }
    }
}

pub struct CutBank {
    sample_rate: f64,
    channels: usize,
    q: f64,
    smooth: f64,
    frame_counter: u32,
    slots: [Slot; SLOTS],
    states: [[TransposedDirectForm2; SLOTS]; MAX_CHANNELS],
}

impl Default for CutBank {
    fn default() -> Self {
        Self::new()
    }
}

impl CutBank {
    pub fn new() -> Self {
        let sample_rate = 48_000.0;
        Self {
            sample_rate,
            channels: MAX_CHANNELS,
            q: 1.0,
            smooth: smoothing_coeff(sample_rate),
            frame_counter: 0,
            slots: [Slot::init(); SLOTS],
            states: [[TransposedDirectForm2::default(); SLOTS]; MAX_CHANNELS],
        }
    }

    pub fn prepare(&mut self, sample_rate: f64, channels: usize) {
        self.sample_rate = sample_rate.max(1.0);
        self.channels = channels.clamp(1, MAX_CHANNELS);
        self.smooth = smoothing_coeff(self.sample_rate);
        self.reset();
    }

    pub fn reset(&mut self) {
        self.frame_counter = 0;
        self.slots = [Slot::init(); SLOTS];
        self.states = [[TransposedDirectForm2::default(); SLOTS]; MAX_CHANNELS];
    }

    pub fn set_targets(
        &mut self,
        voices: &[VoiceState; MAX_VOICES],
        depth_db: f32,
        q: f32,
        harmonics: usize,
    ) {
        self.q = f64::from(q).clamp(0.5, 18.0);
        let depth_db = f64::from(depth_db.min(0.0));
        let nyquist_limit = 0.45 * self.sample_rate;

        for (voice_index, voice) in voices.iter().enumerate() {
            let presence = f64::from(voice.presence.clamp(0.0, 1.0));
            for harmonic_index in 0..MAX_HARMONICS {
                let slot = &mut self.slots[voice_index * MAX_HARMONICS + harmonic_index];
                let freq_hz = f64::from(voice.partials[harmonic_index].freq_hz);
                if voice.active
                    && harmonic_index < harmonics
                    && freq_hz > MIN_PARTIAL_HZ
                    && freq_hz < nyquist_limit
                {
                    slot.target_freq = freq_hz;
                    slot.target_gain_db = depth_db * presence;
                } else {
                    slot.target_gain_db = 0.0;
                }
                if voice.id != slot.voice_id {
                    slot.voice_id = voice.id;
                    if slot.current_gain_db.abs() < JUMP_GAIN_DB {
                        slot.current_freq = slot.target_freq;
                    }
                }
            }
        }
    }

    pub fn process_frame(&mut self, frame: &mut [f64]) {
        if self.frame_counter == 0 {
            self.update_slots();
        }

        let used = frame.len().min(self.channels).min(MAX_CHANNELS);
        for channel in 0..used {
            let mut sample = frame[channel];
            for slot in 0..SLOTS {
                if self.slots[slot].active {
                    sample =
                        self.states[channel][slot].process(sample, self.slots[slot].coefficients);
                }
            }
            frame[channel] = sample;
        }

        self.frame_counter += 1;
        if self.frame_counter >= UPDATE_PERIOD {
            self.frame_counter = 0;
        }
    }

    fn update_slots(&mut self) {
        let smooth = self.smooth;
        let one_minus = 1.0 - smooth;
        let q = self.q;
        let sample_rate = self.sample_rate;

        for slot_index in 0..SLOTS {
            let slot = &mut self.slots[slot_index];
            let ln_current = slot.current_freq.max(MIN_SMOOTH_FREQ_HZ).ln();
            let ln_target = slot.target_freq.max(MIN_SMOOTH_FREQ_HZ).ln();
            slot.current_freq = (smooth * ln_current + one_minus * ln_target).exp();
            slot.current_gain_db = smooth * slot.current_gain_db + one_minus * slot.target_gain_db;

            let was_active = slot.active;
            slot.active = slot.current_gain_db.abs() >= ACTIVE_GAIN_DB;
            let became_inactive = was_active && !slot.active;
            let active = slot.active;
            let freq = slot.current_freq;
            let gain = slot.current_gain_db;
            if active {
                slot.coefficients =
                    BiquadCoefficients::design(BiquadKind::Bell, freq, gain, q, sample_rate);
            }
            if became_inactive {
                for channel in 0..MAX_CHANNELS {
                    self.states[channel][slot_index].reset();
                }
            }
        }
    }
}

fn smoothing_coeff(sample_rate: f64) -> f64 {
    (-32.0 / (SMOOTH_TIME_S * sample_rate.max(1.0))).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::types::Partial;

    const SAMPLE_RATE: f64 = 48_000.0;
    const CHANNELS: usize = 2;

    fn prepared_bank() -> CutBank {
        let mut bank = CutBank::new();
        bank.prepare(SAMPLE_RATE, CHANNELS);
        bank
    }

    fn cut_voice() -> [VoiceState; MAX_VOICES] {
        let mut voices = [VoiceState::default(); MAX_VOICES];
        let mut partials = [Partial::default(); MAX_HARMONICS];
        partials[0] = Partial {
            freq_hz: 200.0,
            mag: 1.0,
        };
        voices[0] = VoiceState {
            active: true,
            id: 1,
            f0_hz: 200.0,
            presence: 1.0,
            partials,
        };
        voices
    }

    fn sine(index: usize, freq_hz: f64) -> f64 {
        (2.0 * std::f64::consts::PI * freq_hz * index as f64 / SAMPLE_RATE).sin()
    }

    fn process_sine_tail(
        bank: &mut CutBank,
        freq_hz: f64,
        frames: usize,
        tail: usize,
    ) -> (f64, f64) {
        let tail_start = frames.saturating_sub(tail);
        let mut out_sum_sq = 0.0;
        let mut in_sum_sq = 0.0;
        for index in 0..frames {
            let input = sine(index, freq_hz);
            let mut frame = [input, input];
            bank.process_frame(&mut frame);
            if index >= tail_start {
                out_sum_sq += frame[0] * frame[0] + frame[1] * frame[1];
                in_sum_sq += 2.0 * input * input;
            }
        }
        (out_sum_sq, in_sum_sq)
    }

    fn rms_ratio_db(out_sum_sq: f64, in_sum_sq: f64) -> f64 {
        10.0 * (out_sum_sq / in_sum_sq.max(1.0e-30)).log10()
    }

    #[test]
    fn no_targets_is_passthrough() {
        let mut bank = prepared_bank();
        for index in 0..256 {
            let left = sine(index, 440.0);
            let right = sine(index, 660.0) * 0.5;
            let mut frame = [left, right];
            bank.process_frame(&mut frame);
            assert_eq!(frame[0], left);
            assert_eq!(frame[1], right);
        }
    }

    #[test]
    fn bell_cut_matches_center_and_passes_far() {
        let mut bank = prepared_bank();
        bank.set_targets(&cut_voice(), -12.0, 8.0, 1);
        let (out_sum_sq, in_sum_sq) = process_sine_tail(&mut bank, 200.0, 48_000, 4_800);
        let center_db = rms_ratio_db(out_sum_sq, in_sum_sq);
        assert!(
            (center_db - (-12.0)).abs() < 1.5,
            "center ratio {center_db} dB"
        );

        let mut far_bank = prepared_bank();
        far_bank.set_targets(&cut_voice(), -12.0, 8.0, 1);
        let (out_sum_sq, in_sum_sq) = process_sine_tail(&mut far_bank, 1_000.0, 48_000, 4_800);
        let far_db = rms_ratio_db(out_sum_sq, in_sum_sq);
        assert!(far_db.abs() < 0.5, "far ratio {far_db} dB");
    }

    #[test]
    fn presence_zero_fully_releases() {
        let mut bank = prepared_bank();
        let mut voices = cut_voice();
        bank.set_targets(&voices, -12.0, 8.0, 1);
        let _ = process_sine_tail(&mut bank, 200.0, 48_000, 4_800);

        voices[0].presence = 0.0;
        bank.set_targets(&voices, -12.0, 8.0, 1);

        let frames = (0.3 * SAMPLE_RATE) as usize;
        let tail = 4_800;
        let tail_start = frames.saturating_sub(tail);
        for index in 0..frames {
            let input = sine(index, 200.0);
            let mut frame = [input, input];
            bank.process_frame(&mut frame);
            if index >= tail_start {
                assert!((frame[0] - input).abs() < 1e-3);
                assert!((frame[1] - input).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn fully_loaded_stays_finite() {
        let mut bank = prepared_bank();
        let mut voices = [VoiceState::default(); MAX_VOICES];
        for voice_index in 0..MAX_VOICES {
            let f0 = 80.0 * (voice_index as f32 + 1.0);
            let mut partials = [Partial::default(); MAX_HARMONICS];
            for harmonic_index in 0..MAX_HARMONICS {
                partials[harmonic_index] = Partial {
                    freq_hz: f0 * (harmonic_index as f32 + 1.0),
                    mag: 1.0,
                };
            }
            voices[voice_index] = VoiceState {
                active: true,
                id: voice_index as u32 + 1,
                f0_hz: f0,
                presence: 1.0,
                partials,
            };
        }
        bank.set_targets(&voices, -12.0, 8.0, MAX_HARMONICS);

        let mut seed = 1u32;
        for _ in 0..2_048 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let left = (seed as f64 / f64::from(u32::MAX)) * 2.0 - 1.0;
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let right = (seed as f64 / f64::from(u32::MAX)) * 2.0 - 1.0;
            let mut frame = [left, right];
            bank.process_frame(&mut frame);
            assert!(frame[0].is_finite());
            assert!(frame[1].is_finite());
        }
    }
}
