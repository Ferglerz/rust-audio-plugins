use crate::dsp::voice::Voice;
use scd_core::{KitPieceId, MicChannel, ScdPack, StrikeEntry};

pub const MAX_VOICES: usize = 48;

pub struct VoicePool {
    voices: [Voice; MAX_VOICES],
}

impl Default for VoicePool {
    fn default() -> Self {
        Self {
            voices: std::array::from_fn(|_| Voice::default()),
        }
    }
}

impl VoicePool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        for v in self.voices.iter_mut() {
            v.active = false;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn trigger_strike(
        &mut self,
        strike: &StrikeEntry,
        pack: &ScdPack,
        velocity: f32,
        pitch_semi: f32,
        punch: f32,
        pan: f32,
        sample_rate: f32,
    ) {
        let voice_idx = self
            .voices
            .iter()
            .position(|v| !v.active)
            .unwrap_or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| {
                        a.playhead_frame
                            .partial_cmp(&b.playhead_frame)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            });

        self.voices[voice_idx].trigger(strike, pack, velocity, pitch_semi, punch, pan, sample_rate);
    }

    pub fn choke_hihat(&mut self, fade_ms: f32, sample_rate: f32) {
        for v in self.voices.iter_mut() {
            if v.active && v.is_hihat {
                v.start_choke(fade_ms, sample_rate);
            }
        }
    }

    #[inline(always)]
    pub fn process_sample(
        &mut self,
        mic_accum: &mut [[f32; 2]; MicChannel::COUNT],
        mix_gains: &[[f32; MicChannel::COUNT]; KitPieceId::COUNT],
        kit_peaks: &mut [f32; KitPieceId::COUNT],
    ) {
        for v in self.voices.iter_mut() {
            if v.active {
                let kp = v.kit_piece as usize;
                let peak = v.process_sample(mic_accum, &mix_gains[kp]);
                if peak > kit_peaks[kp] {
                    kit_peaks[kp] = peak;
                }
            }
        }
    }
}
