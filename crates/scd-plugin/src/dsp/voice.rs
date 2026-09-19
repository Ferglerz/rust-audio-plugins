use scd_core::{KitPieceId, SampleSlice, StrikeEntry};

pub struct Voice {
    pub active: bool,
    pub kit_piece: KitPieceId,
    pub is_hihat: bool,
    pub is_choking: bool,
    pub choke_coeff: f32,
    pub choke_gain: f32,
    pub velocity_gain: f32,
    pub punch_gain: f32,
    pub pitch_ratio: f64,
    pub playhead_frame: f64,
    pub mic_slices: [Option<SampleSlice>; 6],
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            active: false,
            kit_piece: KitPieceId::Kick,
            is_hihat: false,
            is_choking: false,
            choke_coeff: 1.0,
            choke_gain: 1.0,
            velocity_gain: 1.0,
            punch_gain: 1.0,
            pitch_ratio: 1.0,
            playhead_frame: 0.0,
            mic_slices: [None; 6],
        }
    }
}

impl Voice {
    pub fn trigger(
        &mut self,
        strike: &StrikeEntry,
        velocity: f32,
        pitch_semi: f32,
        punch: f32,
        sample_rate: f32,
    ) {
        self.active = true;
        self.kit_piece = strike.kit_piece;
        self.is_hihat = strike.kit_piece == KitPieceId::Hihat;
        self.is_choking = false;
        self.choke_coeff = 1.0;
        self.choke_gain = 1.0;
        self.playhead_frame = 0.0;
        self.mic_slices = strike.mic_slices;

        // Velocity gain curve (linear to power)
        self.velocity_gain = velocity * velocity;

        // Pitch ratio
        let semi_ratio = 2.0_f64.powf(pitch_semi as f64 / 12.0);
        let sr_ratio = 44100.0 / sample_rate as f64; // Default source SR is 44100
        self.pitch_ratio = semi_ratio * sr_ratio;

        // Punch: -1 to +1 adjusts attack punch
        self.punch_gain = 1.0 + punch * 0.5;
    }

    pub fn start_choke(&mut self, fade_ms: f32, sample_rate: f32) {
        self.is_choking = true;
        self.choke_coeff = (-1.0 / (fade_ms * 0.001 * sample_rate)).exp();
    }

    #[inline(always)]
    pub fn process_sample(
        &mut self,
        pack: &scd_core::ScdPack,
        mic_accum: &mut [f32; 6],
    ) -> bool {
        if !self.active {
            return false;
        }

        let frame_idx = self.playhead_frame as usize;
        let frac = (self.playhead_frame - frame_idx as f64) as f32;
        let mut any_slice_alive = false;

        let gain = self.velocity_gain * self.punch_gain * self.choke_gain;

        for (mic_idx, slice_opt) in self.mic_slices.iter().enumerate() {
            if let Some(slice) = slice_opt {
                if frame_idx + 1 < slice.length_frames as usize {
                    any_slice_alive = true;
                    if let Some(data) = pack.get_sample_slice(slice) {
                        // Linear interpolation
                        let s0 = data[frame_idx];
                        let s1 = data[frame_idx + 1];
                        let interpolated = s0 + frac * (s1 - s0);
                        mic_accum[mic_idx] += interpolated * gain;
                    }
                }
            }
        }

        if self.is_choking {
            self.choke_gain *= self.choke_coeff;
            if self.choke_gain < 0.0001 {
                self.active = false;
                return false;
            }
        }

        self.playhead_frame += self.pitch_ratio;
        self.active = any_slice_alive;
        self.active
    }
}
