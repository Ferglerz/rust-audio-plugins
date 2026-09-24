use crate::dsp::voice::Voice;
use scd_core::{KitPieceId, MicChannel, ScdPack, StrikeEntry};

pub const MAX_VOICES: usize = 48;

pub struct VoicePool {
    voices: [Voice; MAX_VOICES],
    pack: Option<ScdPack>,
}

impl Default for VoicePool {
    fn default() -> Self {
        Self {
            voices: std::array::from_fn(|_| Voice::default()),
            pack: None,
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

    pub fn pack(&self) -> Option<&ScdPack> {
        self.pack.as_ref()
    }

    pub fn set_pack(&mut self, pack: ScdPack) {
        self.reset();
        self.pack = Some(pack);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn trigger_strike(
        &mut self,
        strike: &StrikeEntry,
        velocity: f32,
        pitch_semi: f32,
        punch: f32,
        pan: f32,
        sample_rate: f32,
    ) {
        let Some(pack) = self.pack.as_ref() else {
            return;
        };
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
        let Some(pack) = self.pack.as_ref() else {
            return;
        };
        let audio_samples = pack.audio_samples();
        for v in self.voices.iter_mut() {
            if v.active {
                let kp = v.kit_piece as usize;
                let peak = v.process_sample(audio_samples, mic_accum, &mix_gains[kp]);
                if peak > kit_peaks[kp] {
                    kit_peaks[kp] = peak;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scd_core::{PackIndex, SampleSlice, SCD_MAGIC};
    use std::io::Write;

    #[test]
    fn pool_owns_pack_for_triggered_voice() -> Result<(), Box<dyn std::error::Error>> {
        let mut file = tempfile::NamedTempFile::new()?;
        let strike = StrikeEntry::new(
            36,
            1,
            127,
            1,
            0,
            KitPieceId::Kick,
            [
                Some(SampleSlice {
                    offset_bytes: 0,
                    length_frames: 4,
                    sample_rate: 44_100,
                    channels: 1,
                }),
                None,
                None,
                None,
                None,
                None,
            ],
        );
        let index = rkyv::to_bytes::<_, 256>(&PackIndex {
            strikes: vec![strike.clone()],
        })?;
        file.write_all(SCD_MAGIC)?;
        file.write_all(&(index.len() as u64).to_le_bytes())?;
        file.write_all(&index)?;
        for _ in 0..4 {
            file.write_all(&0.5f32.to_le_bytes())?;
        }
        file.flush()?;

        let pack = ScdPack::open(file.path())?;
        drop(file);
        let mut pool = VoicePool::new();
        pool.set_pack(pack);
        pool.trigger_strike(&strike, 1.0, 0.0, 0.0, 0.0, 44_100.0);

        let mut mic_accum = [[0.0; 2]; MicChannel::COUNT];
        let mix_gains = [[1.0; MicChannel::COUNT]; KitPieceId::COUNT];
        let mut kit_peaks = [0.0; KitPieceId::COUNT];
        pool.process_sample(&mut mic_accum, &mix_gains, &mut kit_peaks);
        assert!(mic_accum[MicChannel::Close as usize][0] > 0.3);
        assert!(kit_peaks[KitPieceId::Kick as usize] > 0.3);

        Ok(())
    }
}
