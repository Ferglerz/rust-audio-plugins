use crate::dsp::voice::Voice;
use scd_core::{KitPieceId, ScdPack, StrikeEntry};

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

    pub fn trigger_strike(
        &mut self,
        strike: &StrikeEntry,
        velocity: f32,
        pitch_semi: f32,
        punch: f32,
        sample_rate: f32,
    ) {
        // Find inactive voice, or steal oldest/quietest
        let voice_idx = self
            .voices
            .iter()
            .position(|v| !v.active)
            .unwrap_or_else(|| {
                // Steal voice with greatest playhead frame
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

        self.voices[voice_idx].trigger(strike, velocity, pitch_semi, punch, sample_rate);
    }

    pub fn choke_kitpiece(&mut self, kit_piece: KitPieceId, fade_ms: f32, sample_rate: f32) {
        for v in self.voices.iter_mut() {
            if v.active && v.kit_piece == kit_piece {
                v.start_choke(fade_ms, sample_rate);
            }
        }
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
        pack: &ScdPack,
        mic_accum: &mut [f32; 6],
    ) {
        for v in self.voices.iter_mut() {
            if v.active {
                v.process_sample(pack, mic_accum);
            }
        }
    }
}
