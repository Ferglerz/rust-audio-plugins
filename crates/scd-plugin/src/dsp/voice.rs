use super::{db_to_gain, exp_coeff, semitone_ratio_f64};
use scd_core::{KitPieceId, MicChannel, ScdPack, StrikeEntry};

const ENV_FLOOR: f32 = 0.0001;
const NATIVE_SR: f64 = 44100.0;

#[derive(Clone, Copy)]
struct MicTap {
    data: &'static [f32],
    channels: u8,
    length_frames: u32,
}

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
    mic_taps: [Option<MicTap>; MicChannel::COUNT],
    pub pan_l: f32,
    pub pan_r: f32,
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
            mic_taps: [None; MicChannel::COUNT],
            pan_l: 1.0,
            pan_r: 1.0,
        }
    }
}

impl Voice {
    #[allow(clippy::too_many_arguments)]
    pub fn trigger(
        &mut self,
        strike: &StrikeEntry,
        pack: &ScdPack,
        velocity: f32,
        pitch_semi: f32,
        punch: f32,
        pan: f32, // -100..+100
        sample_rate: f32,
    ) {
        self.active = true;
        self.kit_piece = strike.kit_piece;
        self.is_hihat = strike.kit_piece == KitPieceId::Hihat;
        self.is_choking = false;
        self.choke_coeff = 1.0;
        self.choke_gain = 1.0;
        self.playhead_frame = 0.0;

        let mut pack_sr = NATIVE_SR;
        let mut saw_slice = false;
        for (i, slice_opt) in strike.mic_slices.iter().enumerate() {
            self.mic_taps[i] = slice_opt.and_then(|slice| {
                if !saw_slice {
                    pack_sr = slice.sample_rate as f64;
                    saw_slice = true;
                }
                pack.get_sample_slice(&slice).map(|data| MicTap {
                    data,
                    channels: slice.channels,
                    length_frames: slice.length_frames,
                })
            });
        }

        self.velocity_gain = velocity * velocity * db_to_gain(strike.volume_db as f32);

        let semi_ratio = semitone_ratio_f64(pitch_semi as f64);
        let sr_ratio = pack_sr / sample_rate as f64;
        self.pitch_ratio = semi_ratio * sr_ratio;

        self.punch_gain = 1.0 + punch * 0.5;

        let (pan_l, pan_r) = equal_power_pan(pan);
        self.pan_l = pan_l;
        self.pan_r = pan_r;
    }

    pub fn start_choke(&mut self, fade_ms: f32, sample_rate: f32) {
        self.is_choking = true;
        self.choke_coeff = exp_coeff(fade_ms, sample_rate);
    }

    #[inline(always)]
    pub fn process_sample(
        &mut self,
        mic_accum: &mut [[f32; 2]; MicChannel::COUNT],
        mic_gains: &[f32; MicChannel::COUNT],
    ) -> f32 {
        if !self.active {
            return 0.0;
        }

        let frame_idx = self.playhead_frame as usize;
        let frac = (self.playhead_frame - frame_idx as f64) as f32;
        let mut any_slice_alive = false;
        let mut peak = 0.0f32;

        let gain = self.velocity_gain * self.punch_gain * self.choke_gain;

        for (mic_idx, tap) in self.mic_taps.iter().enumerate() {
            let Some(tap) = tap else { continue };
            if frame_idx + 1 >= tap.length_frames as usize {
                continue;
            }
            any_slice_alive = true;
            let data = tap.data;
            let mic_gain = gain * mic_gains[mic_idx];
            let (out_l, out_r) = if tap.channels == 2 {
                let base = frame_idx * 2;
                if base + 3 >= data.len() {
                    continue;
                }
                let pair = super::simd::stereo_lerp_scale(
                    [data[base], data[base + 1]],
                    [data[base + 2], data[base + 3]],
                    frac,
                    [mic_gain * self.pan_l, mic_gain * self.pan_r],
                );
                (pair[0], pair[1])
            } else if frame_idx + 1 < data.len() {
                let mono = lerp(data[frame_idx], data[frame_idx + 1], frac) * mic_gain;
                (mono * self.pan_l, mono * self.pan_r)
            } else {
                continue;
            };
            super::simd::add_stereo(&mut mic_accum[mic_idx], [out_l, out_r]);
            peak = peak.max(out_l.abs()).max(out_r.abs());
        }

        if self.is_choking {
            self.choke_gain *= self.choke_coeff;
            if self.choke_gain < ENV_FLOOR {
                self.active = false;
                return peak;
            }
        }

        self.playhead_frame += self.pitch_ratio;
        self.active = any_slice_alive;
        peak
    }
}

#[inline]
fn lerp(a: f32, b: f32, frac: f32) -> f32 {
    a + frac * (b - a)
}

/// Equal-power pan for kit-piece pan knobs in the -100..+100 range.
pub fn equal_power_pan(pan: f32) -> (f32, f32) {
    let pan_norm = (pan.clamp(-100.0, 100.0) + 100.0) / 200.0;
    let angle = pan_norm * std::f32::consts::FRAC_PI_2;
    (angle.cos(), angle.sin())
}

#[cfg(test)]
mod tests {
    use super::equal_power_pan;

    #[test]
    fn center_pan_is_equal_power() {
        let (l, r) = equal_power_pan(0.0);
        let expected = std::f32::consts::FRAC_1_SQRT_2;
        assert!((l - expected).abs() < 1e-5);
        assert!((r - expected).abs() < 1e-5);
    }

    #[test]
    fn hard_pan_extremes() {
        let (l, r) = equal_power_pan(-100.0);
        assert!((l - 1.0).abs() < 1e-5);
        assert!(r.abs() < 1e-5);
        let (l, r) = equal_power_pan(100.0);
        assert!(l.abs() < 1e-5);
        assert!((r - 1.0).abs() < 1e-5);
    }
}
